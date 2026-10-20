use gpui::{AssetSource, SharedString};
use std::borrow::Cow;

pub struct Assets;
impl AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        macro_rules! icons {
            ($($name:literal),*) => { match path {
                $(concat!("icons/", $name, ".svg") => Some(Cow::Borrowed(include_bytes!(concat!("../../../assets/icons/", $name, ".svg")).as_slice())),)*
                _ => None,
            } };
        }
        Ok(icons!(
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
            "search",
            "arrow-left",
            "play",
            "pause",
            "square",
            "skip-back",
            "skip-forward",
            "shuffle",
            "repeat",
            "volume-2",
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
            "triangle-alert"
        ))
    }
    fn list(&self, _: &str) -> anyhow::Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}
