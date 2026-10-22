use gpui::{Rgba, rgb};
use rap_core::config::Accent;

#[derive(Clone, Copy)]
pub struct Theme {
    pub background: Rgba,
    pub panel: Rgba,
    pub card: Rgba,
    pub text: Rgba,
    pub muted: Rgba,
    /// Dividers and container outlines.
    pub border: Rgba,
    pub accent: Rgba,
    /// Text and icons drawn on an accent fill.
    pub on_accent: Rgba,
    /// Accent tint marking the selected item of a list or set.
    pub selected: Rgba,
    /// Fill and outline of buttons and other controls at rest.
    pub control: Rgba,
    pub control_border: Rgba,
    pub control_hover: Rgba,
    pub control_pressed: Rgba,
    /// Translucent layers over any surface, for hovered and pressed controls.
    pub hover: Rgba,
    pub pressed: Rgba,
    pub danger: Rgba,
    /// Translucent layer behind overlays and drop targets.
    pub scrim: Rgba,
}

impl Theme {
    pub fn new(dark: bool, accent: Accent) -> Self {
        let accent = rgb(accent_color(accent, dark));
        let selected = Rgba {
            a: if dark { 0.18 } else { 0.14 },
            ..accent
        };
        let layer = |value: f32, a: f32| Rgba {
            r: value,
            g: value,
            b: value,
            a,
        };
        // Palettes of Oxyde Windows Dark and Windows Light.
        if dark {
            Self {
                background: rgb(0x181818),
                panel: rgb(0x1e1e1e),
                card: rgb(0x2a2a2a),
                text: rgb(0xd0d0d0),
                muted: rgb(0x999999),
                border: rgb(0x2e2e2e),
                accent,
                on_accent: rgb(0x111111),
                selected,
                control: rgb(0x2b2b2b),
                control_border: rgb(0x3d3d3d),
                control_hover: rgb(0x333333),
                control_pressed: rgb(0x262626),
                hover: layer(1., 0.07),
                pressed: layer(1., 0.11),
                danger: rgb(0xc42b1c),
                scrim: layer(0.094, 0.82),
            }
        } else {
            Self {
                background: rgb(0xeeeeee),
                panel: rgb(0xffffff),
                card: rgb(0xededed),
                text: rgb(0x2e2e2e),
                muted: rgb(0x666666),
                border: rgb(0xe0e0e0),
                accent,
                on_accent: rgb(0xffffff),
                selected,
                control: rgb(0xfdfdfd),
                control_border: rgb(0xcfcfcf),
                control_hover: rgb(0xf3f3f3),
                control_pressed: rgb(0xe8e8e8),
                hover: layer(0., 0.05),
                pressed: layer(0., 0.09),
                danger: rgb(0xc42b1c),
                scrim: layer(0.933, 0.88),
            }
        }
    }
}

/// One readable variant per theme: accents color text, icons and progress on
/// every surface of the palette.
fn accent_color(accent: Accent, dark: bool) -> u32 {
    let (dark_variant, light_variant) = match accent {
        Accent::Teal => (0x68d4b3, 0x0f6e5a),
        Accent::Blue => (0x74b6ff, 0x1d5fb4),
        Accent::Indigo => (0xa3acff, 0x4249c2),
        Accent::Violet => (0xc6a2ff, 0x6c3dbf),
        Accent::Pink => (0xff94c8, 0xa82a66),
        Accent::Red => (0xff8f86, 0xb02a26),
        Accent::Orange => (0xffb36e, 0x9a4a08),
        Accent::Green => (0x93d97f, 0x2c6e1e),
    };
    if dark { dark_variant } else { light_variant }
}

#[cfg(test)]
#[path = "tests/theme.rs"]
mod tests;
