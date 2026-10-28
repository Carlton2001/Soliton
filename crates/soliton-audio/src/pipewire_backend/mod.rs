//! PipeWire outputs on Linux: sinks listed live from the registry, and one
//! playback stream per opened output, routed to a chosen sink or following
//! the system default.
mod graph;
mod stream;

use crate::OutputDevice;
use anyhow::{Result, bail, ensure};
use crossbeam_channel::{Receiver, Sender};
use graph::{Global, Graph};
use pipewire as pw;
use pw::types::ObjectType;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
pub(crate) use stream::{Stream, open};

/// Time allowed for the sound server to answer; it normally takes milliseconds.
const ANSWER: Duration = Duration::from_secs(3);
/// Delay before connecting again after the sound server stopped.
const RECONNECT: Duration = Duration::from_secs(5);

/// A connection driven by its own main loop on the calling thread, with the
/// graph kept up to date by the registry.
struct Connection {
    mainloop: pw::main_loop::MainLoopRc,
    core: pw::core::CoreRc,
    graph: Rc<RefCell<Graph>>,
    /// Set once the server reported the existing globals.
    ready: Rc<Cell<bool>>,
    broken: Rc<Cell<bool>>,
    _registry: pw::registry::RegistryRc,
    _listeners: (pw::registry::Listener, pw::core::Listener),
    _context: pw::context::ContextRc,
}

impl Connection {
    /// `changed` runs after each registry event following [`Self::roundtrip`].
    fn new(changed: impl Fn(&Graph) + 'static) -> Result<Self> {
        pw::init();
        let mainloop = pw::main_loop::MainLoopRc::new(None)?;
        let context = pw::context::ContextRc::new(&mainloop, None)?;
        let core = context.connect_rc(None)?;
        let registry = core.get_registry_rc()?;
        let graph = Rc::new(RefCell::new(Graph::default()));
        let ready = Rc::new(Cell::new(false));
        let changed = Rc::new(changed);
        let notify = {
            let (graph, ready) = (graph.clone(), ready.clone());
            move || {
                if ready.get() {
                    changed(&graph.borrow());
                }
            }
        };
        let notify = Rc::new(notify);
        let registry_listener = registry
            .add_listener_local()
            .global({
                let (graph, notify) = (graph.clone(), notify.clone());
                move |global| {
                    let kind = match global.type_ {
                        ObjectType::Node => Global::Node,
                        ObjectType::Device => Global::Device,
                        ObjectType::Link => Global::Link,
                        _ => return,
                    };
                    let props = global.props;
                    graph.borrow_mut().add(global.id, kind, |key| {
                        props.and_then(|props| props.get(key)).map(str::to_owned)
                    });
                    notify();
                }
            })
            .global_remove({
                let (graph, notify) = (graph.clone(), notify.clone());
                move |id| {
                    graph.borrow_mut().remove(id);
                    notify();
                }
            })
            .register();
        let broken = Rc::new(Cell::new(false));
        let core_listener = core
            .add_listener_local()
            .error({
                let (broken, mainloop) = (broken.clone(), mainloop.clone());
                move |id, _, _, _| {
                    if id == pw::core::PW_ID_CORE {
                        broken.set(true);
                        mainloop.quit();
                    }
                }
            })
            .register();
        Ok(Self {
            mainloop,
            core,
            graph,
            ready,
            broken,
            _registry: registry,
            _listeners: (registry_listener, core_listener),
            _context: context,
        })
    }

    /// Waits until the server has announced every existing global.
    fn roundtrip(&self) -> Result<()> {
        let pending = self.core.sync(0)?;
        let done = Rc::new(Cell::new(false));
        let _listener = self
            .core
            .add_listener_local()
            .done({
                let (done, mainloop) = (done.clone(), self.mainloop.clone());
                move |id, seq| {
                    if id == pw::core::PW_ID_CORE && seq == pending {
                        done.set(true);
                        mainloop.quit();
                    }
                }
            })
            .register();
        let timeout = self.mainloop.loop_().add_timer({
            let mainloop = self.mainloop.clone();
            move |_| mainloop.quit()
        });
        timeout.update_timer(Some(ANSWER), None).into_result()?;
        self.mainloop.run();
        self.check()?;
        ensure!(done.get(), "PipeWire did not answer");
        self.ready.set(true);
        Ok(())
    }

    fn check(&self) -> Result<()> {
        if self.broken.get() {
            bail!("Connection to PipeWire lost");
        }
        Ok(())
    }
}

/// The listing thread stops when the engine drops this handle.
pub(crate) struct Outputs {
    stop: Arc<AtomicBool>,
}

impl Drop for Outputs {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// The registry announces every sink change: outputs are never polled, and
/// manual refreshes are unnecessary.
pub(crate) fn watch_outputs(
    discovered: Sender<Result<Vec<OutputDevice>>>,
    _refresh: Receiver<()>,
    _watched: Arc<AtomicBool>,
) -> Result<Outputs> {
    let stop = Arc::new(AtomicBool::new(false));
    std::thread::Builder::new()
        .name("soliton-outputs".into())
        .spawn({
            let stop = stop.clone();
            move || {
                while !stop.load(Ordering::Relaxed) {
                    if let Err(error) = list(&discovered, &stop)
                        && discovered.send(Err(error)).is_err()
                    {
                        return;
                    }
                    // A restarted sound server is joined again.
                    let start = Instant::now();
                    while !stop.load(Ordering::Relaxed) && start.elapsed() < RECONNECT {
                        std::thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        })?;
    Ok(Outputs { stop })
}

/// Sends the outputs after each change until stopped or disconnected.
fn list(discovered: &Sender<Result<Vec<OutputDevice>>>, stop: &Arc<AtomicBool>) -> Result<()> {
    // Links and other nodes change often: only a different listing is sent.
    let sent = Rc::new(RefCell::new(Vec::new()));
    let connection = Connection::new({
        let (discovered, sent) = (discovered.clone(), sent.clone());
        move |graph| {
            let outputs = graph.outputs();
            if *sent.borrow() != outputs {
                sent.replace(outputs.clone());
                let _ = discovered.send(Ok(outputs));
            }
        }
    })?;
    connection.roundtrip()?;
    let outputs = connection.graph.borrow().outputs();
    sent.replace(outputs.clone());
    let _ = discovered.send(Ok(outputs));
    let watch = connection.mainloop.loop_().add_timer({
        let (mainloop, stop) = (connection.mainloop.clone(), stop.clone());
        move |_| {
            if stop.load(Ordering::Relaxed) {
                mainloop.quit();
            }
        }
    });
    let second = Some(Duration::from_secs(1));
    watch.update_timer(second, second).into_result()?;
    connection.mainloop.run();
    connection.check()
}

/// Whether a sink name designates exactly one current output.
pub(crate) fn check(name: &str) -> Result<()> {
    let connection = Connection::new(|_| {})?;
    connection.roundtrip()?;
    connection.graph.borrow().find(name).map(|_| ())
}
