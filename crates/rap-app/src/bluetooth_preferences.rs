//! Paired Bluetooth devices are separate from currently usable audio endpoints.
use crate::{
    app::{Rap, Section},
    i18n::Text,
    preferences::{hint, notice, section},
    widgets::{TEXT_SMALL, button, ghost, list_box, list_row, radio_row, tinted_icon},
};
use gpui::{prelude::*, *};
use rap_audio::Command;
use rap_platform::{Bluetooth, BluetoothDevice, DeviceWatch};
use std::time::{Duration, Instant};

/// Pause after the last native change before querying: connecting a device
/// raises several events in a row.
const SETTLE: Duration = Duration::from_millis(500);
/// Query interval used only when native notifications are unavailable.
const FALLBACK_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Default)]
pub struct BluetoothState {
    devices: Vec<BluetoothDevice>,
    refreshing: bool,
    opening: bool,
    refreshed: Option<Instant>,
    error: Option<String>,
    /// Native notifications, kept only while the settings page is visible.
    watch: Option<DeviceWatch>,
    watch_starting: bool,
    watch_failed: bool,
    /// Last native change not yet followed by a query.
    changed: Option<Instant>,
}

impl Rap {
    pub fn poll_bluetooth(&mut self, cx: &mut Context<Self>) {
        let visible = Bluetooth::SUPPORTED && self.section == Section::Settings && !self.quitting;
        if !visible {
            // Leaving the page stops the native watchers; reopening starts fresh.
            self.bluetooth.watch = None;
            self.bluetooth.watch_failed = false;
            self.bluetooth.refreshed = None;
            self.bluetooth.changed = None;
            return;
        }
        if self.bluetooth.watch.is_none()
            && !self.bluetooth.watch_starting
            && !self.bluetooth.watch_failed
        {
            self.start_bluetooth_watch(cx);
        }
        if let Some(watch) = &self.bluetooth.watch
            && watch.changes.try_recv().is_ok()
        {
            self.bluetooth.changed = Some(Instant::now());
        }
        let settled = self
            .bluetooth
            .changed
            .is_some_and(|last| last.elapsed() >= SETTLE);
        let fallback = self.bluetooth.watch_failed
            && self
                .bluetooth
                .refreshed
                .is_some_and(|last| last.elapsed() >= FALLBACK_INTERVAL);
        if (self.bluetooth.refreshed.is_none() || settled || fallback) && !self.bluetooth.refreshing
        {
            self.bluetooth.changed = None;
            self.refresh_bluetooth(cx);
        }
    }

    fn start_bluetooth_watch(&mut self, cx: &mut Context<Self>) {
        self.bluetooth.watch_starting = true;
        let start = cx.background_executor().spawn(async { Bluetooth::watch() });
        cx.spawn(async move |this, cx| {
            let result = start.await;
            let _ = this.update(cx, |this, _| {
                this.bluetooth.watch_starting = false;
                match result {
                    // Dropped at the next poll if the page was left meanwhile.
                    Ok(watch) => this.bluetooth.watch = Some(watch),
                    Err(error) => {
                        tracing::warn!("Bluetooth notifications unavailable: {error:#}");
                        this.bluetooth.watch_failed = true;
                    }
                }
            });
        })
        .detach();
    }

    fn refresh_bluetooth(&mut self, cx: &mut Context<Self>) {
        if self.bluetooth.refreshing || self.quitting {
            return;
        }
        self.bluetooth.refreshing = true;
        self.bluetooth.refreshed = Some(Instant::now());
        let query = cx
            .background_executor()
            .spawn(async { Bluetooth::devices() });
        cx.spawn(async move |this, cx| {
            let result = query.await;
            let _ = this.update(cx, |this, cx| {
                let state = &mut this.bluetooth;
                let mut changed = false;
                state.refreshing = false;
                state.refreshed = Some(Instant::now());
                match result {
                    Ok(devices) => {
                        changed |= devices != state.devices || state.error.is_some();
                        state.devices = devices;
                        state.error = None;
                    }
                    Err(error) => {
                        let error = error.to_string();
                        changed |= state.error.as_ref() != Some(&error);
                        state.error = Some(error);
                    }
                }
                // Repaint only when something visible changed.
                if changed {
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub fn bluetooth_preferences(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !Bluetooth::SUPPORTED {
            return None;
        }
        let theme = self.theme();
        let mut rows = Vec::new();
        for device in &self.bluetooth.devices {
            let outputs = self.ready_bluetooth_outputs(device);
            let ready = !outputs.is_empty();
            let status = if ready {
                Text::BluetoothAudioReady
            } else if device.connected {
                Text::BluetoothAudioUnavailable
            } else {
                Text::BluetoothDisconnected
            };
            rows.push(
                list_row()
                    .id(SharedString::from(format!("bluetooth:{}", device.id)))
                    .child(tinted_icon(
                        "bluetooth",
                        if ready { theme.accent } else { theme.muted },
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(div().truncate().child(device.name.clone()))
                            .child(
                                div()
                                    .text_size(TEXT_SMALL)
                                    .text_color(theme.muted)
                                    .child(self.t(status)),
                            ),
                    )
                    .when(!ready, |row| {
                        row.child(
                            button(
                                SharedString::from(format!("connect-bluetooth:{}", device.id)),
                                "bluetooth",
                                self.t(Text::ConnectBluetoothWindows),
                                theme,
                            )
                            .when(self.bluetooth.opening, |button| button.opacity(0.5))
                            .on_click(
                                cx.listener(|this, _, _, cx| this.open_bluetooth_settings(cx)),
                            ),
                        )
                    })
                    .into_any_element(),
            );
            rows.extend(outputs.into_iter().map(|name| {
                let selected = self.playback.network_output.is_none()
                    && self.playback.selected_output.as_ref() == Some(&name);
                radio_row(
                    SharedString::from(format!("bluetooth-output:{}:{name}", device.id)),
                    "volume-2",
                    name.clone(),
                    selected,
                    theme,
                )
                .pl(rems(2.5))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.audio_command(Command::SelectOutput(Some(name.clone())), cx);
                }))
                .into_any_element()
            }));
        }
        if self.bluetooth.refreshed.is_some()
            && !self.bluetooth.refreshing
            && self.bluetooth.devices.is_empty()
            && self.bluetooth.error.is_none()
        {
            rows.push(
                list_row()
                    .text_color(theme.muted)
                    .child(self.t(Text::NoPairedBluetooth))
                    .into_any_element(),
            );
        }
        rows.push(
            div()
                .p(rems(0.375))
                .child(
                    ghost(
                        "add-bluetooth-output",
                        "plus",
                        self.t(if self.bluetooth.opening {
                            Text::OpeningBluetoothSettings
                        } else {
                            Text::AddBluetoothDevice
                        }),
                        theme,
                    )
                    .when(self.bluetooth.opening, |button| button.opacity(0.5))
                    .on_click(cx.listener(|this, _, _, cx| this.open_bluetooth_settings(cx))),
                )
                .into_any_element(),
        );
        let body = div()
            .flex()
            .flex_col()
            .gap(rems(0.5))
            .child(list_box(rows, theme))
            .when_some(self.bluetooth.error.as_ref(), |view, error| {
                view.child(notice(
                    self.labelled(self.t(Text::BluetoothQueryError), error),
                    theme,
                ))
            })
            .children(
                [Text::BluetoothPairingHint, Text::BluetoothQualityHint]
                    .map(|text| hint(self.t(text), theme)),
            );
        Some(section(self.t(Text::BluetoothAudio), None, body))
    }

    /// Names from the native container association still require an unambiguous,
    /// active engine output. Never route by a Bluetooth friendly-name substring.
    fn ready_bluetooth_outputs(&self, device: &BluetoothDevice) -> Vec<String> {
        ready_outputs(device, &self.playback.outputs)
    }

    fn open_bluetooth_settings(&mut self, cx: &mut Context<Self>) {
        if self.bluetooth.opening || self.quitting {
            return;
        }
        self.bluetooth.opening = true;
        cx.notify();
        let launch = cx
            .background_executor()
            .spawn(async { Bluetooth::open_settings() });
        cx.spawn(async move |this, cx| {
            let result = launch.await;
            let _ = this.update(cx, |this, cx| {
                this.bluetooth.opening = false;
                if let Err(error) = result {
                    this.report_error(
                        this.labelled(this.t(Text::AddBluetoothDevice), &error.to_string()),
                    );
                }
                // Pairing happens in Windows; the watchers report its outcome.
                cx.notify();
            });
        })
        .detach();
    }
}

fn ready_outputs(device: &BluetoothDevice, outputs: &[rap_audio::OutputDevice]) -> Vec<String> {
    device
        .outputs
        .iter()
        .filter(|name| {
            outputs
                .iter()
                .any(|output| output.selectable && &output.name == *name)
        })
        .cloned()
        .collect()
}

#[cfg(test)]
#[path = "tests/bluetooth_preferences.rs"]
mod tests;
