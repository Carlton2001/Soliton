//! Styled menu of the notification area icon, in its own popup window.
use crate::{
    app::Soliton,
    assets::Assets,
    i18n::Text,
    theme::Theme,
    widgets::{RADIUS, TEXT_BODY, TEXT_SMALL, UI_FONT, tinted_icon},
};
use gpui::{prelude::*, *};
use soliton_audio::PlaybackState;
use soliton_platform::{Desktop, MediaCommand, TrayAnchor};

const WIDTH: f32 = 264.;
const HEIGHT: f32 = 202.;
const COVER: Pixels = px(44.);
const ACTION_HEIGHT: Pixels = px(34.);
const TRANSPORT_SIZE: Pixels = px(36.);

impl Soliton {
    /// Replaces an open menu, since the same click may not have closed it yet.
    pub fn open_tray_menu(&mut self, anchor: TrayAnchor, cx: &mut Context<Self>) {
        if self.quitting {
            return;
        }
        self.close_tray_menu(cx);
        let owner = cx.entity();
        // Opening draws at once and the menu reads this entity, so wait for
        // the current update to end.
        cx.defer(move |cx| {
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: point(px(anchor.x as f32), px(anchor.y as f32)),
                    size: size(px(WIDTH), px(HEIGHT)),
                })),
                titlebar: None,
                kind: WindowKind::PopUp,
                is_movable: false,
                is_resizable: false,
                is_minimizable: false,
                ..Default::default()
            };
            let weak = owner.downgrade();
            let opened = cx.open_window(options, |window, cx| {
                cx.new(|cx| TrayMenu::new(weak, window, cx))
            });
            owner.update(cx, |app, cx| app.tray_menu_opened(opened, anchor, cx));
        });
    }

    fn tray_menu_opened(
        &mut self,
        opened: anyhow::Result<WindowHandle<TrayMenu>>,
        anchor: TrayAnchor,
        cx: &mut Context<Self>,
    ) {
        let Some(desktop) = &self.desktop else {
            return;
        };
        let handle = match opened {
            Ok(handle) => handle,
            Err(error) => {
                tracing::warn!("Styled tray menu unavailable: {error:#}");
                desktop.show_native_tray_menu();
                return;
            }
        };
        let _ = handle.update(cx, |_, window, _| {
            if let Err(error) = desktop.place_tray_menu(window, anchor, (WIDTH, HEIGHT)) {
                tracing::warn!("Tray menu placement failed: {error:#}");
            }
            window.activate_window();
        });
        self.tray_menu = Some(handle);
    }

    fn close_tray_menu(&mut self, cx: &mut Context<Self>) {
        if let Some(menu) = self.tray_menu.take() {
            let _ = menu.update(cx, |_, window, _| window.remove_window());
        }
    }

    /// Delivers tray menu requests at once, even while the idle poll is slow.
    pub fn watch_tray_menu(&self, cx: &mut Context<Self>) {
        let Some(requests) = self
            .desktop
            .as_ref()
            .and_then(|desktop| desktop.tray_menu_requests())
        else {
            return;
        };
        cx.spawn(async move |this, cx| {
            loop {
                let requests = requests.clone();
                let request = cx
                    .background_executor()
                    .spawn(async move { requests.recv() })
                    .await;
                let Ok(anchor) = request else { break };
                if this
                    .update(cx, |this, cx| this.open_tray_menu(anchor, cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }
}

pub struct TrayMenu {
    owner: WeakEntity<Soliton>,
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl TrayMenu {
    fn new(owner: WeakEntity<Soliton>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        focus.focus(window);
        let mut subscriptions = vec![cx.observe_window_activation(window, |_, window, _| {
            if !window.is_window_active() {
                window.remove_window();
            } else if let Err(error) = Desktop::raise_tray_menu(window) {
                tracing::warn!("Tray menu raise failed: {error:#}");
            }
        })];
        if let Some(owner) = owner.upgrade() {
            subscriptions.push(cx.observe(&owner, |_, _, cx| cx.notify()));
        }
        Self {
            owner,
            focus,
            _subscriptions: subscriptions,
        }
    }

    fn transport(
        &self,
        id: &'static str,
        name: &'static str,
        command: MediaCommand,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        div()
            .id(id)
            .size(TRANSPORT_SIZE)
            .flex()
            .items_center()
            .justify_center()
            .rounded(RADIUS)
            .hover(move |style| style.bg(theme.control))
            .child(tinted_icon(name, theme.text))
            .on_click(cx.listener(move |this, _, _, cx| {
                // Stays open to chain commands, like a media flyout.
                let _ = this
                    .owner
                    .update(cx, |app, cx| app.system_media(command, cx));
            }))
    }

    fn action(
        &self,
        id: &'static str,
        name: &'static str,
        label: &'static str,
        theme: Theme,
        run: fn(&mut Soliton, &mut Context<Soliton>),
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        div()
            .id(id)
            .h(ACTION_HEIGHT)
            .flex_none()
            .flex()
            .items_center()
            .gap(px(10.))
            .px(px(10.))
            .rounded(RADIUS)
            .text_size(TEXT_BODY)
            .hover(move |style| style.bg(theme.control))
            .child(tinted_icon(name, theme.muted))
            .child(label)
            .on_click(cx.listener(move |this, _, window, cx| {
                window.remove_window();
                let _ = this.owner.update(cx, run);
            }))
    }
}

fn separator(theme: Theme) -> Div {
    div().flex_none().h(px(1.)).mx(px(4.)).bg(theme.border)
}

impl Render for TrayMenu {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(owner) = self.owner.upgrade() else {
            return div();
        };
        let app = owner.read(cx);
        let theme = app.theme();
        let playing = app.playback.state == PlaybackState::Playing;
        let track = app.playback.track.as_ref();
        let title: SharedString = track
            .map(|track| track.metadata.title.clone())
            .unwrap_or_else(|| "Soliton".into())
            .into();
        let artist: Option<SharedString> = track.map(|track| track.metadata.artist.clone().into());
        let cover = track.and_then(|track| track.cover.clone());
        let logo = Assets::logo_path(app.settings.app_icon);
        let show = app.t(Text::ShowSoliton);
        let quit = app.t(Text::QuitApp);
        let previous = self.transport(
            "tray-previous",
            "skip-back",
            MediaCommand::Previous,
            theme,
            cx,
        );
        let toggle = self.transport(
            "tray-toggle",
            if playing { "pause" } else { "play" },
            MediaCommand::Toggle,
            theme,
            cx,
        );
        let next = self.transport("tray-next", "skip-forward", MediaCommand::Next, theme, cx);
        div()
            .track_focus(&self.focus)
            .on_key_down(|event: &KeyDownEvent, window, _| {
                if event.keystroke.key == "escape" {
                    window.remove_window();
                }
            })
            .size_full()
            .flex()
            .flex_col()
            .gap(px(4.))
            .p(px(6.))
            .overflow_hidden()
            .font_family(UI_FONT)
            .bg(theme.panel)
            .border_1()
            .border_color(theme.border)
            .text_color(theme.text)
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .p(px(4.))
                    .child(match cover {
                        Some(cover) => img(cover)
                            .size(COVER)
                            .flex_none()
                            .rounded(RADIUS)
                            .object_fit(ObjectFit::Cover)
                            .into_any_element(),
                        None => img(logo).size(COVER).flex_none().into_any_element(),
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(div().text_size(TEXT_BODY).truncate().child(title))
                            .children(artist.map(|artist| {
                                div()
                                    .text_size(TEXT_SMALL)
                                    .text_color(theme.muted)
                                    .truncate()
                                    .child(artist)
                            })),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .justify_center()
                    .gap(px(8.))
                    .child(previous)
                    .child(toggle)
                    .child(next),
            )
            .child(separator(theme))
            .child(self.action(
                "tray-show",
                "app-window",
                show,
                theme,
                |app, _| app.show_from_tray(),
                cx,
            ))
            .child(self.action(
                "tray-quit",
                "log-out",
                quit,
                theme,
                |app, cx| app.quit_app(cx),
                cx,
            ))
    }
}
