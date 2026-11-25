//! Desktop preferences for client-drawn title bars.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptionButton {
    Minimize,
    Maximize,
    Close,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptionStyle {
    Windows,
    Adwaita,
    Breeze,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowControls {
    pub style: CaptionStyle,
    pub left: Vec<CaptionButton>,
    pub right: Vec<CaptionButton>,
}

impl Default for WindowControls {
    fn default() -> Self {
        #[cfg(target_os = "linux")]
        let style = crate::linux::window_controls::style();
        #[cfg(not(target_os = "linux"))]
        let style = CaptionStyle::Windows;
        Self {
            style,
            left: Vec::new(),
            right: vec![
                CaptionButton::Minimize,
                CaptionButton::Maximize,
                CaptionButton::Close,
            ],
        }
    }
}
