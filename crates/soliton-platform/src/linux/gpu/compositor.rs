//! Reads the Wayland compositor's preferred GPU from linux-dmabuf feedback.
//! Adapted from Oxyde-GPUI (Carlton2001, GPL-3.0-only).

use wayland_client::{
    Connection, Dispatch, QueueHandle, delegate_noop,
    globals::{GlobalListContents, registry_queue_init},
    protocol::wl_registry::{self, WlRegistry},
};
use wayland_protocols::wp::linux_dmabuf::zv1::client::{
    zwp_linux_dmabuf_feedback_v1::{self, ZwpLinuxDmabufFeedbackV1},
    zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1,
};

#[derive(Default)]
struct Feedback {
    main_device: Option<libc::dev_t>,
}

impl Dispatch<WlRegistry, GlobalListContents> for Feedback {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

delegate_noop!(Feedback: ignore ZwpLinuxDmabufV1);

impl Dispatch<ZwpLinuxDmabufFeedbackV1, ()> for Feedback {
    fn event(
        state: &mut Self,
        _: &ZwpLinuxDmabufFeedbackV1,
        event: zwp_linux_dmabuf_feedback_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zwp_linux_dmabuf_feedback_v1::Event::MainDevice { device } = event {
            state.main_device = device.try_into().ok().map(libc::dev_t::from_ne_bytes);
        }
    }
}

/// The compositor's DRM node number as `major:minor`, when available.
pub(super) fn main_device() -> Option<String> {
    // Mirror GPUI's Wayland selection. Leave any inherited socket for GPUI.
    if std::env::var_os("ZED_HEADLESS").is_some()
        || std::env::var_os("WAYLAND_SOCKET").is_some()
        || !std::env::var_os("WAYLAND_DISPLAY").is_some_and(|display| !display.is_empty())
    {
        return None;
    }
    let connection = Connection::connect_to_env().ok()?;
    let (globals, mut queue) = registry_queue_init::<Feedback>(&connection).ok()?;
    let handle = queue.handle();
    let dmabuf: ZwpLinuxDmabufV1 = globals.bind(&handle, 4..=4, ()).ok()?;
    let feedback = dmabuf.get_default_feedback(&handle, ());
    let mut answer = Feedback::default();
    queue.roundtrip(&mut answer).ok()?;
    feedback.destroy();
    dmabuf.destroy();
    let device = answer.main_device?;
    Some(format!("{}:{}", libc::major(device), libc::minor(device)))
}
