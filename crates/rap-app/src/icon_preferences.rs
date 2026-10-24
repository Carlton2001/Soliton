//! Application icon picker. Its palette is independent of the interface theme.
use crate::{app::Rap, assets::Assets, i18n::Text, preferences::hint, widgets::TEXT_SMALL};
use gpui::{prelude::*, *};
use rap_core::config::AppIcon;

impl Rap {
    fn icon_name(&self, icon: AppIcon) -> &'static str {
        self.t(match icon {
            AppIcon::Blue => Text::ElectricBlue,
            AppIcon::Teal => Text::DeepTeal,
            AppIcon::Violet => Text::Violet,
            AppIcon::Copper => Text::Copper,
            AppIcon::GraphiteMint => Text::GraphiteMint,
            AppIcon::IvoryInk => Text::IvoryInk,
        })
    }

    fn set_app_icon(&mut self, icon: AppIcon, cx: &mut Context<Self>) {
        if self.settings.app_icon != icon {
            self.settings.app_icon = icon;
            self.save();
            cx.notify();
        }
    }

    pub fn icon_preferences(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        div()
            .w_full()
            .px(crate::widgets::PAGE_INSET)
            .py(rems(0.875))
            .flex()
            .flex_col()
            .gap(rems(0.75))
            .child(self.t(Text::AppIcon))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(rems(0.5))
                    .children(AppIcon::ALL.map(|icon| {
                        let selected = self.settings.app_icon == icon;
                        div()
                            .id(("app-icon", icon as usize))
                            .tab_index(0)
                            .capture_any_mouse_down(crate::widgets::keyboard_focus_only)
                            .focus(move |style| style.border_color(theme.accent))
                            .w(rems(7.25))
                            .min_h(rems(6.))
                            .p(rems(0.5))
                            .rounded(rems(0.5))
                            .border_2()
                            .border_color(if selected {
                                theme.accent
                            } else {
                                theme.control_border
                            })
                            .bg(if selected {
                                theme.selected
                            } else {
                                theme.control
                            })
                            .hover(move |style| style.bg(theme.control_hover))
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .gap(rems(0.5))
                            .child(img(Assets::logo_path(icon)).size(rems(2.5)).flex_none())
                            .child(
                                div()
                                    .w_full()
                                    .text_center()
                                    .text_size(TEXT_SMALL)
                                    .child(self.icon_name(icon)),
                            )
                            .on_click(
                                cx.listener(move |this, _, _, cx| this.set_app_icon(icon, cx)),
                            )
                            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                    this.set_app_icon(icon, cx);
                                    cx.stop_propagation();
                                }
                            }))
                    })),
            )
            .child(hint(self.t(Text::AppIconHint), theme))
            .into_any_element()
    }
}
