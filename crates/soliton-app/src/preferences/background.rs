//! Live background opacity for the main window.
use super::setting_row;
use crate::{
    app::Soliton,
    i18n::Text,
    slider::{OnChange, keyboard_value},
    theme::Theme,
    widgets::{CONTROL_HEIGHT, TEXT_SMALL},
    window_chrome::background_appearance,
};
use gpui::{prelude::*, *};
use soliton_core::config::Settings;
use std::rc::Rc;

const PERCENT_SCALE: f32 = Settings::MAX_BACKGROUND_OPACITY as f32;

impl Soliton {
    pub(super) fn background_opacity_preference(
        &self,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let percent = self.settings.background_opacity;
        let entity = cx.weak_entity();
        let change: OnChange = Rc::new(move |fraction, released, window, cx| {
            let _ = entity.update(cx, |this, cx| {
                this.set_background_opacity(fraction, released, window, cx);
            });
        });
        let control = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(rems(0.5))
            .child(
                div()
                    .id("background-opacity")
                    .track_focus(&self.background_opacity_focus)
                    .capture_any_mouse_down(cx.listener(
                        |this, event: &MouseDownEvent, window, _| {
                            if event.button == MouseButton::Left {
                                this.background_opacity_focus.focus(window);
                                window.prevent_default();
                            }
                        },
                    ))
                    .border_1()
                    .border_color(transparent_black())
                    .focus(move |style| style.border_color(theme.accent))
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                        if let Some(fraction) = keyboard_value(
                            &event.keystroke.key,
                            f32::from(this.settings.background_opacity) / PERCENT_SCALE,
                            1. / PERCENT_SCALE,
                        ) {
                            this.set_background_opacity(fraction, true, window, cx);
                            cx.stop_propagation();
                        }
                    }))
                    .w(rems(8.))
                    .h(CONTROL_HEIGHT)
                    .px(rems(0.5))
                    .child(
                        self.background_opacity_slider
                            .render(
                                f32::from(percent) / PERCENT_SCALE,
                                true,
                                false,
                                theme,
                                change,
                            )
                            .size_full(),
                    ),
            )
            .child(
                div()
                    .w(rems(3.))
                    .text_right()
                    .text_size(TEXT_SMALL)
                    .child(format!("{percent} %")),
            );
        setting_row(self.t(Text::BackgroundOpacity), None, control, theme)
    }

    fn set_background_opacity(
        &mut self,
        fraction: f32,
        save: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        let before = background_appearance(&self.settings);
        let previous = self.settings.background_opacity;
        let percent = (fraction * PERCENT_SCALE).round() as u8;
        self.settings.background_opacity = percent;
        let after = background_appearance(&self.settings);
        if before != after {
            window.set_background_appearance(after);
        }
        if save {
            self.save();
        }
        if previous != percent {
            cx.notify();
        }
    }
}
