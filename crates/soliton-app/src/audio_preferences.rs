//! Local output choices, network receivers and the actual Soliton stream format.
use crate::{
    app::Soliton,
    i18n::Text,
    preferences::{hint, notice, section},
    widgets::{button, list_box, list_row, radio_row},
};
use gpui::{prelude::*, *};
use soliton_audio::Command;

impl Soliton {
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
        let mut rows = vec![
            radio_row(
                "default-output",
                "volume-2",
                self.t(Text::SystemOutput),
                local && selected.is_none(),
                theme,
            )
            .on_click(
                cx.listener(|this, _, _, cx| this.audio_command(Command::SelectOutput(None), cx)),
            )
            .into_any_element(),
        ];
        rows.extend(
            self.playback
                .outputs
                .iter()
                .enumerate()
                .map(|(index, device)| {
                    let name = device.name.clone();
                    let selectable = device.selectable;
                    let label = if selectable {
                        device.label.clone()
                    } else {
                        format!("{} — {}", device.label, self.t(Text::AmbiguousOutput))
                    };
                    radio_row(
                        ("audio-output", index),
                        if device.bluetooth {
                            "bluetooth"
                        } else {
                            "volume-2"
                        },
                        label,
                        local && selected == Some(&name),
                        theme,
                    )
                    .when(!selectable, |row| row.opacity(0.5))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if selectable {
                            this.audio_command(Command::SelectOutput(Some(name.clone())), cx);
                        }
                    }))
                    .into_any_element()
                }),
        );
        if self.playback.outputs.is_empty() {
            rows.push(
                list_row()
                    .text_color(theme.muted)
                    .child(self.t(Text::NoAudioOutputs))
                    .into_any_element(),
            );
        }
        let body = div()
            .flex()
            .flex_col()
            .gap(rems(0.5))
            .child(list_box(rows, theme))
            .when(missing, |view| {
                view.child(notice(
                    format!(
                        "{} — {}",
                        selected.unwrap(),
                        self.t(if self.playback.fallback {
                            Text::PreferredOutputFallback
                        } else {
                            Text::PreferredOutputUnavailable
                        })
                    ),
                    theme,
                ))
            })
            .when(self.playback.output_changing, |view| {
                view.child(notice(self.t(Text::ConnectingOutput), theme))
            })
            .child(hint(
                self.labelled(
                    self.t(Text::StreamFormat),
                    &self
                        .output_description()
                        .unwrap_or_else(|| self.t(Text::NoActiveOutput).into()),
                ),
                theme,
            ))
            .when(local, |view| {
                view.child(hint(
                    self.t(if soliton_audio::DEFAULT_FOLLOWED {
                        Text::FollowedOutputStreamHint
                    } else {
                        Text::OutputStreamHint
                    }),
                    theme,
                ))
            });
        section(self.t(Text::AudioOutput), None, body)
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
            self.output_label(info),
            info.sample_rate,
            info.channels,
            self.t(Text::Channels)
        ))
    }

    /// A stream following the system default is unnamed until it is linked.
    pub fn output_label(&self, info: &soliton_audio::OutputInfo) -> String {
        if info.device_label.is_empty() {
            self.t(Text::SystemOutput).into()
        } else {
            info.device_label.clone()
        }
    }

    pub fn network_preferences(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let selected = self.playback.network_output.as_ref();
        let discover = button(
            "discover-network-outputs",
            "refresh-cw",
            self.t(if self.playback.network_discovering {
                Text::SearchingReceivers
            } else {
                Text::FindReceivers
            }),
            theme,
        )
        .when(self.playback.network_discovering, |button| {
            button.opacity(0.5)
        })
        .on_click(
            cx.listener(|this, _, _, cx| this.audio_command(Command::DiscoverNetworkOutputs, cx)),
        );
        let rows: Vec<AnyElement> = self
            .playback
            .network_outputs
            .iter()
            .map(|device| {
                let id = device.id.clone();
                radio_row(
                    SharedString::from(format!("network-output:{id}")),
                    "volume-2",
                    format!("{} — {}", device.name, device.protocol.label()),
                    selected.is_some_and(|d| d.id == id),
                    theme,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.audio_command(Command::SelectNetworkOutput(id.clone()), cx)
                }))
                .into_any_element()
            })
            .collect();
        let empty =
            self.playback.network_enabled && !self.playback.network_discovering && rows.is_empty();
        let body = div()
            .flex()
            .flex_col()
            .gap(rems(0.5))
            .when(!rows.is_empty(), |view| view.child(list_box(rows, theme)))
            .when(empty, |view| {
                view.child(list_box(
                    [list_row()
                        .text_color(theme.muted)
                        .child(self.t(Text::NoNetworkOutputs))
                        .into_any_element()],
                    theme,
                ))
            })
            .when(
                selected.is_some_and(|d| {
                    !self
                        .playback
                        .network_outputs
                        .iter()
                        .any(|found| found.id == d.id)
                }),
                |view| view.child(notice(self.t(Text::NetworkOutputMissing), theme)),
            )
            .child(hint(self.t(Text::NetworkOutputHint), theme));
        section(
            self.t(Text::NetworkOutputs),
            Some(discover.into_any_element()),
            body,
        )
    }
}
