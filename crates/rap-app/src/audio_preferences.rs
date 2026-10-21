//! Local/Bluetooth output choices and the actual RAP stream format.
use crate::{
    app::Rap,
    i18n::Text,
    widgets::{button, choice},
};
use gpui::{prelude::*, *};
use rap_audio::Command;
use rap_platform::Bluetooth;

impl Rap {
    pub fn audio_preferences(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let selected = self.playback.selected_output.as_ref();
        let local = self.playback.network_output.is_none();
        let missing = local
            && selected.is_some_and(|name| {
                !self
                    .playback
                    .outputs
                    .iter()
                    .any(|d| &d.name == name && d.selectable)
            });
        div()
            .flex()
            .flex_col()
            .gap_2()
            .min_w_0()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_3()
                    .child(div().text_lg().child(self.t(Text::AudioOutput)))
                    .child(
                        button(
                            "refresh-outputs",
                            "refresh-cw",
                            self.t(Text::RefreshOutputs),
                            theme,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.audio_command(Command::RefreshOutputs, cx)
                        })),
                    ),
            )
            .child(
                choice(
                    "default-output",
                    "volume-2",
                    self.t(Text::SystemOutput),
                    local && selected.is_none(),
                    theme,
                )
                .on_click(
                    cx.listener(|this, _, _, cx| {
                        this.audio_command(Command::SelectOutput(None), cx)
                    }),
                ),
            )
            .children(
                self.playback
                    .outputs
                    .iter()
                    .enumerate()
                    .map(|(index, device)| {
                        let name = device.name.clone();
                        let selectable = device.selectable;
                        let label = if selectable {
                            name.clone()
                        } else {
                            format!("{name} — {}", self.t(Text::AmbiguousOutput))
                        };
                        choice(
                            ("audio-output", index),
                            "volume-2",
                            label,
                            local && selected == Some(&name),
                            theme,
                        )
                        .opacity(if selectable { 1. } else { 0.5 })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if selectable {
                                this.audio_command(Command::SelectOutput(Some(name.clone())), cx);
                            }
                        }))
                    }),
            )
            .when(self.playback.outputs.is_empty(), |view| {
                view.child(
                    div()
                        .text_color(theme.muted)
                        .child(self.t(Text::NoAudioOutputs)),
                )
            })
            .when(missing, |view| {
                view.child(div().text_color(theme.accent).child(format!(
                    "{} — {}",
                    selected.unwrap(),
                    self.t(Text::PreferredOutputUnavailable)
                )))
            })
            .when(Bluetooth::SUPPORTED, |view| {
                view.child(self.bluetooth_preferences(cx))
            })
            .child(self.network_preferences(cx))
            .when(self.playback.output_changing, |view| {
                view.child(
                    div()
                        .text_color(theme.accent)
                        .child(self.t(Text::ConnectingOutput)),
                )
            })
            .child(
                div().text_color(theme.muted).child(
                    self.output_description()
                        .unwrap_or_else(|| self.t(Text::NoActiveOutput).into()),
                ),
            )
            .when(local, |view| {
                view.child(
                    div()
                        .text_sm()
                        .text_color(theme.muted)
                        .child(self.t(Text::OutputStreamHint)),
                )
            })
            .into_any_element()
    }

    pub fn output_description(&self) -> Option<String> {
        if let Some(device) = &self.playback.network_output {
            return Some(format!(
                "{} — {} · {}",
                device.name,
                device.protocol.label(),
                self.t(Text::RemotePlayback)
            ));
        }
        let info = self.playback.output.as_ref()?;
        let format = match info.sample_format.as_str() {
            "F32" => format!("32 {}", self.t(Text::FloatBits)),
            "F64" => format!("64 {}", self.t(Text::FloatBits)),
            "I16" => "16 bits PCM".into(),
            "I32" => "32 bits PCM".into(),
            "U16" => format!("16 {}", self.t(Text::UnsignedBits)),
            other => other.into(),
        };
        Some(format!(
            "{} — {} Hz · {} {} · {format}",
            info.device_name,
            info.sample_rate,
            info.channels,
            self.t(Text::Channels)
        ))
    }

    fn network_preferences(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let selected = self.playback.network_output.as_ref();
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
                    .child(div().text_lg().child(self.t(Text::NetworkOutputs)))
                    .child(
                        button(
                            "discover-network-outputs",
                            "refresh-cw",
                            self.t(if self.playback.network_discovering {
                                Text::SearchingReceivers
                            } else {
                                Text::FindReceivers
                            }),
                            theme,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.audio_command(Command::DiscoverNetworkOutputs, cx)
                        })),
                    ),
            )
            .children(self.playback.network_outputs.iter().map(|device| {
                let id = device.id.clone();
                choice(
                    SharedString::from(format!("network-output:{id}")),
                    "volume-2",
                    format!("{} — {}", device.name, device.protocol.label()),
                    selected.is_some_and(|d| d.id == id),
                    theme,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.audio_command(Command::SelectNetworkOutput(id.clone()), cx)
                }))
            }))
            .when(
                self.playback.network_enabled
                    && !self.playback.network_discovering
                    && self.playback.network_outputs.is_empty(),
                |view| {
                    view.child(
                        div()
                            .text_color(theme.muted)
                            .child(self.t(Text::NoNetworkOutputs)),
                    )
                },
            )
            .when(
                selected.is_some_and(|d| {
                    !self
                        .playback
                        .network_outputs
                        .iter()
                        .any(|found| found.id == d.id)
                }),
                |view| {
                    view.child(
                        div()
                            .text_color(theme.accent)
                            .child(self.t(Text::NetworkOutputMissing)),
                    )
                },
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.muted)
                    .child(self.t(Text::NetworkOutputHint)),
            )
            .into_any_element()
    }
}
