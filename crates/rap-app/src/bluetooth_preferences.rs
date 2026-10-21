//! Paired Bluetooth devices are separate from currently usable audio endpoints.
use crate::{
    app::{Rap, Section},
    i18n::Text,
    widgets::{button, choice},
};
use gpui::{prelude::*, *};
use rap_audio::Command;
use rap_platform::{Bluetooth, BluetoothDevice};
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct BluetoothState {
    devices: Vec<BluetoothDevice>,
    refreshing: bool,
    opening: bool,
    refreshed: Option<Instant>,
    error: Option<String>,
}

impl Rap {
    pub fn poll_bluetooth(&mut self, cx: &mut Context<Self>) {
        if Bluetooth::SUPPORTED
            && self.section == Section::Settings
            && !self.now_playing
            && !self.quitting
            && !self.bluetooth.refreshing
            && self
                .bluetooth
                .refreshed
                .is_none_or(|last| last.elapsed() >= Duration::from_secs(5))
        {
            self.refresh_bluetooth(cx);
        }
    }

    fn refresh_bluetooth(&mut self, cx: &mut Context<Self>) {
        if self.bluetooth.refreshing || self.quitting {
            return;
        }
        self.bluetooth.refreshing = true;
        self.bluetooth.refreshed = Some(Instant::now());
        cx.notify();
        let query = cx
            .background_executor()
            .spawn(async { Bluetooth::devices() });
        cx.spawn(async move |this, cx| {
            let result = query.await;
            let _ = this.update(cx, |this, cx| {
                this.bluetooth.refreshing = false;
                this.bluetooth.refreshed = Some(Instant::now());
                match result {
                    Ok(devices) => {
                        this.bluetooth.devices = devices;
                        this.bluetooth.error = None;
                    }
                    Err(error) => this.bluetooth.error = Some(error.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub fn bluetooth_preferences(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        div()
            .flex()
            .flex_col()
            .gap_2()
            .mt_3()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_3()
                    .child(div().text_lg().child(self.t(Text::BluetoothAudio)))
                    .child(
                        button(
                            "refresh-bluetooth",
                            "refresh-cw",
                            self.t(if self.bluetooth.refreshing {
                                Text::SearchingBluetooth
                            } else {
                                Text::RefreshOutputs
                            }),
                            theme,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.refresh_bluetooth(cx))),
                    ),
            )
            .children(self.bluetooth.devices.iter().map(|device| {
                let outputs = self.ready_bluetooth_outputs(device);
                let ready = !outputs.is_empty();
                let status = if ready {
                    Text::BluetoothAudioReady
                } else if device.connected {
                    Text::BluetoothAudioUnavailable
                } else {
                    Text::BluetoothDisconnected
                };
                div()
                    .id(SharedString::from(format!("bluetooth:{}", device.id)))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(theme.card)
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap_2()
                            .child(crate::widgets::icon("bluetooth", theme))
                            .child(device.name.clone())
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted)
                                    .child(self.t(status)),
                            )
                            .when(!ready, |row| {
                                row.child(
                                    button(
                                        SharedString::from(format!(
                                            "connect-bluetooth:{}",
                                            device.id
                                        )),
                                        "bluetooth",
                                        self.t(Text::ConnectBluetoothWindows),
                                        theme,
                                    )
                                    .opacity(if self.bluetooth.opening { 0.5 } else { 1. })
                                    .on_click(cx.listener(
                                        |this, _, _, cx| this.open_bluetooth_settings(cx),
                                    )),
                                )
                            }),
                    )
                    .children(outputs.into_iter().map(|name| {
                        let selected = self.playback.network_output.is_none()
                            && self.playback.selected_output.as_ref() == Some(&name);
                        choice(
                            SharedString::from(format!("bluetooth-output:{}:{name}", device.id)),
                            "volume-2",
                            name.clone(),
                            selected,
                            theme,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.audio_command(Command::SelectOutput(Some(name.clone())), cx);
                        }))
                    }))
            }))
            .when(
                self.bluetooth.refreshed.is_some()
                    && !self.bluetooth.refreshing
                    && self.bluetooth.devices.is_empty()
                    && self.bluetooth.error.is_none(),
                |view| {
                    view.child(
                        div()
                            .text_sm()
                            .text_color(theme.muted)
                            .child(self.t(Text::NoPairedBluetooth)),
                    )
                },
            )
            .when_some(self.bluetooth.error.as_ref(), |view, error| {
                view.child(
                    div()
                        .text_sm()
                        .text_color(theme.accent)
                        .child(self.labelled(self.t(Text::BluetoothQueryError), error)),
                )
            })
            .child(
                div().child(
                    button(
                        "add-bluetooth-output",
                        "bluetooth",
                        self.t(if self.bluetooth.opening {
                            Text::OpeningBluetoothSettings
                        } else {
                            Text::AddBluetoothDevice
                        }),
                        theme,
                    )
                    .opacity(if self.bluetooth.opening { 0.5 } else { 1. })
                    .on_click(cx.listener(|this, _, _, cx| this.open_bluetooth_settings(cx))),
                ),
            )
            .children(
                [
                    Text::BluetoothPairingHint,
                    Text::BluetoothEchoHint,
                    Text::BluetoothQualityHint,
                ]
                .map(|text| div().text_sm().text_color(theme.muted).child(self.t(text))),
            )
            .into_any_element()
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
                } else if !this.quitting {
                    this.audio_command(Command::RefreshOutputs, cx);
                    this.refresh_bluetooth(cx);
                }
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
