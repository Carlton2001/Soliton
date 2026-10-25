use gpui::{AssetSource, SharedString};
use rap_core::config::AppIcon;
use std::borrow::Cow;

pub struct Assets;
#[cfg(test)]
#[path = "tests/assets.rs"]
mod tests;
impl Assets {
    pub fn logo_path(icon: AppIcon) -> SharedString {
        format!("logos/{}.svg", icon.slug()).into()
    }

    pub fn load_fonts(cx: &gpui::App) -> anyhow::Result<()> {
        cx.text_system().add_fonts(vec![
            Cow::Borrowed(include_bytes!("../../../assets/fonts/Inter-Regular.ttf").as_slice()),
            Cow::Borrowed(include_bytes!("../../../assets/fonts/Inter-Italic.ttf").as_slice()),
        ])
    }
}
impl AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        macro_rules! icons {
            ($($name:literal),*) => { match path {
                $(concat!("icons/", $name, ".svg") => Some(Cow::Borrowed(include_bytes!(concat!("../../../assets/icons/", $name, ".svg")).as_slice())),)*
                _ => None,
            } };
        }
        macro_rules! logos {
            ($($name:literal),*) => { match path {
                $(concat!("logos/", $name, ".svg") => Some(Cow::Borrowed(include_bytes!(concat!("../../../assets/logos/", $name, ".svg")).as_slice())),)*
                _ => None,
            } };
        }
        macro_rules! window_controls {
            ($($name:literal),*) => { match path {
                $(concat!("window-controls/", $name, ".svg") => Some(Cow::Borrowed(include_bytes!(concat!("../../../assets/window-controls/", $name, ".svg")).as_slice())),)*
                _ => None,
            } };
        }
        if let Some(control) = window_controls!("minimize", "maximize", "restore", "close") {
            return Ok(Some(control));
        }
        if let Some(logo) = logos!(
            "blue",
            "teal",
            "violet",
            "copper",
            "graphite-mint",
            "ivory-ink"
        ) {
            return Ok(Some(logo));
        }
        Ok(icons!(
            "house",
            "disc-3",
            "users",
            "tag",
            "calendar",
            "settings",
            "info",
            "folder-plus",
            "folder-minus",
            "refresh-cw",
            "layout-grid",
            "list",
            "list-plus",
            "list-end",
            "search",
            "arrow-left",
            "arrow-up",
            "arrow-down",
            "arrow-down-from-line",
            "eraser",
            "pencil",
            "play",
            "pause",
            "square",
            "skip-back",
            "skip-forward",
            "shuffle",
            "repeat",
            "volume-2",
            "bluetooth",
            "x",
            "maximize-2",
            "minimize-2",
            "chevron-down",
            "volume-x",
            "volume-1",
            "repeat-1",
            "sliders-horizontal",
            "arrow-up-down",
            "check",
            "circle-alert",
            "triangle-alert",
            "minus",
            "plus"
        ))
    }
    fn list(&self, _: &str) -> anyhow::Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}
