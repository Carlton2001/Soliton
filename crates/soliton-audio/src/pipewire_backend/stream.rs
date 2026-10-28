//! One PipeWire playback stream, run by its own thread and main loop.
use super::{ANSWER, Connection};
use crate::{OutputInfo, output::Callback};
use anyhow::{Result, anyhow};
use pipewire as pw;
use pw::{
    properties::properties,
    spa::{self, pod::Pod, sys as spa_sys},
    stream::{StreamFlags, StreamState},
};
use soliton_core::failure::{Failure, Kind};
use std::{
    cell::Cell,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::JoinHandle,
};

const SAMPLE_BYTES: usize = std::mem::size_of::<f32>();
/// Default maximum quantum; longer requests are filled in several parts.
const SCRATCH_FRAMES: usize = 8192;

/// Name and label of a sink; both empty while unknown.
type Device = (String, String);

pub(crate) struct Stream {
    stop: pw::channel::Sender<()>,
    thread: Option<JoinHandle<()>>,
    /// Sink currently linked to the stream, as reported by the registry.
    linked: Arc<Mutex<Option<Device>>>,
}

impl Stream {
    pub fn device(&self) -> Option<Device> {
        self.linked.lock().ok().and_then(|linked| linked.clone())
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Without a preferred sink, the stream follows the system default, which
/// PipeWire moves when it changes. A chosen sink is never replaced: its
/// removal fails the output instead.
pub(crate) fn open(
    rate: u32,
    channels: usize,
    preferred: Option<&str>,
    callback: Callback,
    failed: Arc<AtomicBool>,
) -> Result<(Stream, OutputInfo)> {
    // PipeWire adapts the stream to the sink's layout and rate.
    let channels = if channels == 1 { 1 } else { 2 };
    let (ready, answer) = mpsc::sync_channel(2);
    let (stop, stopped) = pw::channel::channel();
    let linked = Arc::new(Mutex::new(None));
    let target = preferred.map(str::to_owned);
    let thread = std::thread::Builder::new()
        .name("soliton-pipewire".into())
        .spawn({
            let linked = linked.clone();
            move || {
                let stream = Playback {
                    rate,
                    channels,
                    target,
                    failed: failed.clone(),
                    linked,
                };
                if let Err(error) = stream.run(callback, stopped, &ready) {
                    failed.store(true, Ordering::Relaxed);
                    let _ = ready.send(Err(error));
                }
            }
        })?;
    let stream = Stream {
        stop,
        thread: Some(thread),
        linked,
    };
    let (device_name, device_label) = answer
        .recv_timeout(ANSWER * 2)
        .map_err(|_| anyhow!("PipeWire did not open the stream"))??;
    let info = OutputInfo {
        device_name,
        device_label,
        sample_rate: rate,
        channels: channels as u16,
        sample_format: "F32".into(),
    };
    Ok((stream, info))
}

struct Playback {
    rate: u32,
    channels: usize,
    target: Option<String>,
    failed: Arc<AtomicBool>,
    linked: Arc<Mutex<Option<Device>>>,
}

impl Playback {
    /// Reports the opened stream through `ready`, then plays until stopped.
    fn run(
        self,
        callback: Callback,
        stopped: pw::channel::Receiver<()>,
        ready: &mpsc::SyncSender<Result<Device>>,
    ) -> Result<()> {
        let node = Rc::new(Cell::new(None));
        let target = Rc::new(Cell::new(None));
        let connection = Connection::new({
            let (node, target) = (node.clone(), target.clone());
            let (linked, failed) = (self.linked.clone(), self.failed.clone());
            move |graph| {
                if let Some(node) = node.get()
                    && let Ok(mut linked) = linked.lock()
                {
                    *linked = graph.linked(node);
                }
                if target.get().is_some_and(|sink| graph.sink(sink).is_none()) {
                    failed.store(true, Ordering::Relaxed);
                }
            }
        })?;
        connection.roundtrip()?;
        let device = {
            let graph = connection.graph.borrow();
            match &self.target {
                Some(name) => {
                    let sink = graph.find(name)?;
                    target.set(Some(sink));
                    graph.sink(sink).unwrap_or_default()
                }
                None if graph.has_sinks() => Device::default(),
                None => return Err(Failure::new(Kind::NoOutputDevice).into()),
            }
        };
        let mut props = properties! {
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_CATEGORY => "Playback",
            *pw::keys::MEDIA_ROLE => "Music",
        };
        if let Some(name) = &self.target {
            props.insert(*pw::keys::TARGET_OBJECT, name.as_str());
            // Moving to another sink would silently replace the chosen output.
            props.insert(*pw::keys::NODE_DONT_RECONNECT, "true");
            props.insert("node.dont-fallback", "true");
        }
        let stream = pw::stream::StreamBox::new(&connection.core, "Soliton", props)?;
        let channels = self.channels;
        let frame = SAMPLE_BYTES * channels;
        let _listener = stream
            .add_local_listener_with_user_data(vec![0f32; SCRATCH_FRAMES * channels])
            .state_changed({
                let failed = self.failed.clone();
                move |stream, _, _, state| match state {
                    StreamState::Paused | StreamState::Streaming => {
                        node.set(Some(stream.node_id()))
                    }
                    StreamState::Error(_) | StreamState::Unconnected => {
                        failed.store(true, Ordering::Relaxed)
                    }
                    StreamState::Connecting => {}
                }
            })
            // Real-time thread: same contract as the CPAL callback.
            .process(move |stream, scratch| {
                let Some(mut buffer) = stream.dequeue_buffer() else {
                    return;
                };
                let requested = buffer.requested() as usize;
                let Some(data) = buffer.datas_mut().first_mut() else {
                    return;
                };
                let frames = match data.data() {
                    Some(bytes) => {
                        let mut frames = bytes.len() / frame;
                        if requested > 0 {
                            frames = frames.min(requested);
                        }
                        for part in bytes[..frames * frame].chunks_mut(scratch.len() * SAMPLE_BYTES)
                        {
                            let samples = &mut scratch[..part.len() / SAMPLE_BYTES];
                            callback.fill(samples, channels, |value| value);
                            let (encoded, _) = part.as_chunks_mut::<SAMPLE_BYTES>();
                            for (bytes, value) in encoded.iter_mut().zip(&*samples) {
                                *bytes = value.to_le_bytes();
                            }
                        }
                        frames
                    }
                    None => 0,
                };
                let chunk = data.chunk_mut();
                *chunk.offset_mut() = 0;
                *chunk.stride_mut() = frame as i32;
                *chunk.size_mut() = (frames * frame) as u32;
            })
            .register()?;
        let format = format(self.rate, channels)?;
        let mut params = [Pod::from_bytes(&format).ok_or(anyhow!("Invalid stream format"))?];
        stream.connect(
            spa::utils::Direction::Output,
            None,
            StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS | StreamFlags::RT_PROCESS,
            &mut params,
        )?;
        let _stop = stopped.attach(connection.mainloop.loop_(), {
            let mainloop = connection.mainloop.clone();
            move |()| mainloop.quit()
        });
        let _ = ready.send(Ok(device));
        connection.mainloop.run();
        let _ = stream.disconnect();
        connection.check()
    }
}

/// Interleaved 32-bit floats, mono or front left and right.
fn format(rate: u32, channels: usize) -> Result<Vec<u8>> {
    let mut info = spa::param::audio::AudioInfoRaw::new();
    info.set_format(spa::param::audio::AudioFormat::F32LE);
    info.set_rate(rate);
    info.set_channels(channels as u32);
    let mut position = [0; spa::param::audio::MAX_CHANNELS];
    if channels == 1 {
        position[0] = spa_sys::SPA_AUDIO_CHANNEL_MONO;
    } else {
        position[0] = spa_sys::SPA_AUDIO_CHANNEL_FL;
        position[1] = spa_sys::SPA_AUDIO_CHANNEL_FR;
    }
    info.set_position(position);
    let object = spa::pod::Value::Object(spa::pod::Object {
        type_: spa_sys::SPA_TYPE_OBJECT_Format,
        id: spa_sys::SPA_PARAM_EnumFormat,
        properties: info.into(),
    });
    let (cursor, _) =
        spa::pod::serialize::PodSerializer::serialize(std::io::Cursor::new(Vec::new()), &object)?;
    Ok(cursor.into_inner())
}

#[cfg(test)]
#[path = "tests/stream.rs"]
mod tests;
