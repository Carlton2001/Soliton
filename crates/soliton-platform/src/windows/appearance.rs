//! Read Windows accent colors on the desktop worker, never on the UI thread.
use crate::DesktopEvent;
use crossbeam_channel::Sender;
use std::time::{Duration, Instant};
use windows::UI::ViewManagement::{UIColorType, UISettings};

pub(super) struct AccentWatcher {
    settings: UISettings,
    last: Option<[u32; 2]>,
    next_poll: Instant,
}

impl AccentWatcher {
    pub fn new() -> Option<Self> {
        Some(Self {
            settings: UISettings::new().ok()?,
            last: None,
            next_poll: Instant::now(),
        })
    }

    pub fn poll(&mut self, events: &Sender<DesktopEvent>) {
        if Instant::now() < self.next_poll {
            return;
        }
        // Bounded polling also covers delayed/missed ColorValuesChanged events.
        self.next_poll = Instant::now() + Duration::from_secs(1);
        let read = |kind| {
            self.settings
                .GetColorValue(kind)
                .ok()
                .map(|c| (u32::from(c.R) << 16) | (u32::from(c.G) << 8) | u32::from(c.B))
        };
        let (Some(light), Some(dark)) = (
            read(UIColorType::AccentDark2),
            read(UIColorType::AccentLight3),
        ) else {
            return;
        };
        let colors = [light, dark];
        if self.last != Some(colors) && events.try_send(DesktopEvent::SystemAccent(colors)).is_ok()
        {
            self.last = Some(colors);
        }
    }
}
