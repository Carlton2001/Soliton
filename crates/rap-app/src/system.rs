//! GPUI adapter for native commands and the application/window lifecycle.
use crate::{
    app::Rap,
    i18n::Text,
    preferences::{section, setting_row},
    service::Work,
    widgets::{RADIUS_LARGE, TEXT_SUBTITLE, button, checkbox, list_box, segment, segments},
};
use gpui::{prelude::*, *};
use rap_audio::{Command, PlaybackState};
use rap_core::config::CloseBehavior;
use rap_platform::{
    Desktop, DesktopEvent, MediaCommand, MediaState, MediaTrack, PreviewColors, TransportState,
};
use raw_window_handle::HasWindowHandle;
use std::time::Duration;

impl Rap {
    pub fn start_desktop(&mut self, window: &Window) {
        if !Desktop::SUPPORTED {
            return;
        }
        let result = HasWindowHandle::window_handle(window)
            .map_err(|error| anyhow::anyhow!(error.to_string()))
            .and_then(|handle| Desktop::start(handle.as_raw(), self.settings.language));
        match result {
            Ok(desktop) => self.desktop = Some(desktop),
            Err(error) => {
                self.report_warning(self.labelled(self.t(Text::DesktopError), &error.to_string()))
            }
        }
    }

    pub fn poll_desktop(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let events: Vec<_> = self
            .desktop
            .as_ref()
            .map(|desktop| desktop.events.try_iter().collect())
            .unwrap_or_default();
        for event in events {
            match event {
                DesktopEvent::Media(command) if !self.quitting => self.system_media(command, cx),
                DesktopEvent::Media(_) => {}
                DesktopEvent::Show => {
                    if let Some(desktop) = &self.desktop {
                        desktop.show();
                    }
                }
                DesktopEvent::Quit => self.quit_app(cx),
                DesktopEvent::Autostart(result) => {
                    self.startup_pending = false;
                    match result {
                        Ok(enabled) => self.autostart = Some(enabled),
                        Err(error) => self.report_warning(
                            self.labelled(self.t(Text::Autostart), &error.to_string()),
                        ),
                    }
                }
                DesktopEvent::Warning(error) => {
                    self.report_warning(self.labelled(self.t(Text::DesktopError), &error))
                }
            }
            cx.notify();
        }
        if let Some(desktop) = &self.desktop {
            let theme = self.theme();
            let rgb = |color: Rgba| {
                ((color.r * 255.) as u32) << 16
                    | ((color.g * 255.) as u32) << 8
                    | (color.b * 255.) as u32
            };
            let state = MediaState {
                track: self.playback.track.as_ref().map(|track| MediaTrack {
                    title: track.metadata.title.clone(),
                    artist: track.metadata.artist.clone(),
                    album: track.metadata.album.clone(),
                    cover: track.cover.clone(),
                }),
                state: match self.playback.state {
                    PlaybackState::Playing => TransportState::Playing,
                    PlaybackState::Paused => TransportState::Paused,
                    _ => TransportState::Stopped,
                },
                position_ms: self.playback.position_ms,
                duration_ms: self.playback.duration_ms,
                preview: PreviewColors {
                    background: rgb(theme.panel),
                    text: rgb(theme.text),
                    muted: rgb(theme.muted),
                    accent: rgb(theme.accent),
                },
            };
            desktop.publish(state, self.settings.language);
        }
    }

    fn system_media(&mut self, command: MediaCommand, cx: &mut Context<Self>) {
        match command {
            MediaCommand::Toggle => self.play_pause(cx),
            MediaCommand::Play if self.playback.state != PlaybackState::Playing => {
                self.play_pause(cx)
            }
            MediaCommand::Play => {}
            MediaCommand::Pause => self.audio_command(Command::Pause, cx),
            MediaCommand::Stop => self.audio_command(Command::Stop, cx),
            MediaCommand::Previous => self.audio_command(Command::Previous, cx),
            MediaCommand::Next => self.audio_command(Command::Next, cx),
            MediaCommand::Seek(position) => {
                self.audio_command(Command::Seek(position.min(self.playback.duration_ms)), cx)
            }
        }
    }

    pub fn request_close(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.quitting {
            return;
        }
        if !Desktop::SUPPORTED {
            self.quit_app(cx);
            return;
        }
        match self.settings.close_behavior {
            CloseBehavior::Quit => self.quit_app(cx),
            CloseBehavior::Hide => self.hide_to_tray(cx),
            CloseBehavior::Ask => {
                self.confirm_close = true;
                cx.notify();
            }
        }
    }

    fn hide_to_tray(&mut self, cx: &mut Context<Self>) {
        self.confirm_close = false;
        if !self.desktop.as_ref().is_some_and(Desktop::hide) {
            // Keep the visible taskbar window when there is no recovery icon.
            self.report_warning(self.t(Text::CannotHide));
        }
        cx.notify();
    }

    pub fn set_close_behavior(&mut self, behavior: CloseBehavior, cx: &mut Context<Self>) {
        self.settings.close_behavior = behavior;
        self.save();
        cx.notify();
    }

    pub fn toggle_autostart(&mut self, cx: &mut Context<Self>) {
        if self.startup_pending {
            return;
        }
        if let Some(desktop) = &self.desktop {
            match desktop.set_autostart(!self.autostart.unwrap_or(false)) {
                Ok(()) => self.startup_pending = true,
                Err(error) => {
                    self.report_error(self.labelled(self.t(Text::Autostart), &error.to_string()))
                }
            }
        }
        cx.notify();
    }

    /// Cleanup completes before asking GPUI to destroy the borrowed native window.
    pub fn quit_app(&mut self, cx: &mut Context<Self>) {
        if self.quitting {
            return;
        }
        self.quitting = true;
        self.confirm_close = false;
        self.cancellation.cancel();
        if let Some(preparing) = &self.preparing {
            preparing.cancel();
        }
        self.final_queue();
        let audio = self.audio.take();
        let desktop = self.desktop.take();
        let work = self.services.work.clone();
        let settings = self.settings.clone();
        let task = cx.background_executor().spawn(async move {
            if let Some(audio) = audio {
                audio.shutdown();
            }
            if let Some(desktop) = desktop {
                desktop.shutdown();
            }
            let (reply, saved) = crossbeam_channel::bounded(1);
            work.send_timeout(Work::SaveAndFlush(settings, reply), Duration::from_secs(2))
                .map_err(|error| anyhow::anyhow!(error.to_string()))?;
            saved.recv_timeout(Duration::from_secs(3))??;
            anyhow::Ok(())
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                if let Err(error) = result {
                    tracing::warn!("Final settings save failed: {error:#}");
                }
                // Quit still works when storage is unavailable.
                cx.quit();
                this.quitting = true;
            });
        })
        .detach();
        cx.notify();
    }

    pub fn system_preferences(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !Desktop::SUPPORTED {
            return None;
        }
        let theme = self.theme();
        let on_close = segments(
            [
                (CloseBehavior::Quit, Text::QuitApp),
                (CloseBehavior::Hide, Text::HideToTray),
                (CloseBehavior::Ask, Text::AskOnClose),
            ]
            .map(|(behavior, label)| {
                segment(
                    ("close-behavior", behavior as usize),
                    None,
                    self.t(label),
                    self.settings.close_behavior == behavior,
                    theme,
                )
                .on_click(cx.listener(move |this, _, _, cx| this.set_close_behavior(behavior, cx)))
            }),
            theme,
        );
        let autostart = div().px(rems(1.)).py(rems(0.625)).child(
            checkbox(
                "autostart",
                self.autostart == Some(true),
                self.t(Text::Autostart),
                theme,
            )
            .when(self.startup_pending || self.autostart.is_none(), |row| {
                row.opacity(0.5)
            })
            .on_click(cx.listener(|this, _, _, cx| {
                if this.autostart.is_some() {
                    this.toggle_autostart(cx);
                }
            })),
        );
        Some(section(
            self.t(Text::WindowsIntegration),
            None,
            list_box(
                [
                    setting_row(self.t(Text::OnClose), None, on_close, theme),
                    autostart.into_any_element(),
                ],
                theme,
            ),
        ))
    }

    pub fn close_dialog(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.confirm_close {
            return None;
        }
        let theme = self.theme();
        Some(
            div()
                .absolute()
                .inset_0()
                .occlude()
                .flex()
                .items_center()
                .justify_center()
                .bg(theme.scrim)
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .p(rems(1.5))
                        .rounded(RADIUS_LARGE)
                        .border_1()
                        .border_color(theme.control_border)
                        .bg(theme.panel)
                        .flex()
                        .flex_col()
                        .gap(rems(1.25))
                        .child(
                            div()
                                .text_size(TEXT_SUBTITLE)
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(self.t(Text::CloseQuestion)),
                        )
                        .child(
                            div()
                                .flex()
                                .gap(rems(0.5))
                                .child(
                                    button("close-quit", "x", self.t(Text::QuitApp), theme)
                                        .on_click(cx.listener(|this, _, _, cx| this.quit_app(cx))),
                                )
                                .when(
                                    self.desktop.as_ref().is_some_and(Desktop::can_hide),
                                    |view| {
                                        view.child(
                                            button(
                                                "close-hide",
                                                "minimize-2",
                                                self.t(Text::HideToTray),
                                                theme,
                                            )
                                            .on_click(
                                                cx.listener(|this, _, _, cx| this.hide_to_tray(cx)),
                                            ),
                                        )
                                    },
                                )
                                .child(
                                    button(
                                        "close-cancel",
                                        "arrow-left",
                                        self.t(Text::Cancel),
                                        theme,
                                    )
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.confirm_close = false;
                                            cx.notify();
                                        },
                                    )),
                                ),
                        ),
                )
                .into_any_element(),
        )
    }
}
