use gpui::{AssetSource, SharedString};
use std::borrow::Cow;

pub struct Assets;
impl Assets {
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
        if path == "images/logo.svg" {
            return Ok(Some(Cow::Borrowed(include_bytes!(
                "../../../assets/log_rap_small.svg"
            ))));
        }
        Ok(icons!(
            "house",
            "disc-3",
            "users",
            "tag",
            "calendar",
            "settings",
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
            "copy",
            "plus"
        ))
    }
    fn list(&self, _: &str) -> anyhow::Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}
