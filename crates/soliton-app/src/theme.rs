use gpui::{Rgba, rgb};
use soliton_core::config::Accent;

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
    #[cfg(test)]
    pub fn new(dark: bool, accent: Accent) -> Self {
        Self::with_system_accent(dark, accent, None)
    }

    pub fn with_system_accent(dark: bool, accent: Accent, system: Option<[u32; 2]>) -> Self {
        let accent = rgb(if accent == Accent::System {
            system
                .map(|colors| colors[usize::from(dark)])
                .unwrap_or_else(|| accent_color(Accent::Blue, dark))
        } else {
            accent_color(accent, dark)
        });
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
        // Windows 11 / WinUI neutral resources, adapted to Soliton's surface roles.
        // Source: microsoft/microsoft-ui-xaml, Common_themeresources_any.xaml.
        // Keep control fills translucent so they adapt to each containing surface;
        // native previews receive opaque RGB surface and text colors.
        if dark {
            Self {
                background: rgb(0x202020), // SolidBackgroundFillColorBase
                panel: rgb(0x282828),      // SolidBackgroundFillColorTertiary
                card: rgb(0x323232),       // CardBackgroundFillColorDefault over panel
                text: rgb(0xffffff),
                muted: rgb(0xc5c5c5),
                border: layer(1., 21. / 255.), // DividerStrokeColorDefault
                accent,
                on_accent: rgb(0x000000),
                selected,
                control: layer(1., 15. / 255.),
                control_border: layer(1., 18. / 255.),
                control_hover: layer(1., 21. / 255.),
                control_pressed: layer(1., 8. / 255.),
                hover: layer(1., 15. / 255.),
                pressed: layer(1., 10. / 255.),
                danger: rgb(0xff99a4),
                scrim: layer(0., 77. / 255.),
            }
        } else {
            Self {
                background: rgb(0xf3f3f3), // SolidBackgroundFillColorBase
                panel: rgb(0xf9f9f9),      // SolidBackgroundFillColorTertiary
                card: rgb(0xffffff),       // CardBackgroundFillColorTertiary
                text: rgb(0x1b1b1b),
                muted: rgb(0x606060),
                border: layer(0., 15. / 255.),
                accent,
                on_accent: rgb(0xffffff),
                selected,
                control: layer(1., 179. / 255.),
                control_border: layer(0., 15. / 255.),
                control_hover: Rgba {
                    a: 128. / 255.,
                    ..rgb(0xf9f9f9)
                },
                control_pressed: Rgba {
                    a: 77. / 255.,
                    ..rgb(0xf9f9f9)
                },
                hover: layer(0., 9. / 255.),
                pressed: layer(0., 6. / 255.),
                danger: rgb(0xc42b1c),
                scrim: layer(0., 77. / 255.),
            }
        }
    }
}

/// Stable swatch colors from the Windows personalization palette.
/// Reference: Microsoft's Universal color palette (Microsoft-Design-Language-1603.pdf).
/// These identify the chosen hue independently of the current light/dark theme.
pub fn accent_swatch(accent: Accent) -> Rgba {
    rgb(accent_palette(accent).0)
}

/// Reference swatch, dark-theme variant, light-theme variant.
/// Variants retain the reference hue while meeting 4.5:1 text contrast on
/// every Soliton surface, including text drawn on accent-filled controls.
fn accent_palette(accent: Accent) -> (u32, u32, u32) {
    match accent {
        Accent::Teal => (0x038387, 0x4ab0b4, 0x037a7e),
        Accent::Blue | Accent::System => (0x0078d4, 0x45a2ff, 0x006ec3),
        Accent::Indigo => (0x6b69d6, 0x8e90ff, 0x6462c7),
        Accent::Violet => (0x8764b8, 0xad8ae1, 0x7f5ead),
        Accent::Pink => (0xe3008c, 0xff5fb2, 0xd30082),
        Accent::Red => (0xe81123, 0xff6b63, 0xda1021),
        Accent::Orange => (0xca5010, 0xf2753f, 0xba4a0f),
        Accent::Green => (0x10893e, 0x4fb76a, 0x0f7e39),
    }
}

fn accent_color(accent: Accent, dark: bool) -> u32 {
    let (_, dark_variant, light_variant) = accent_palette(accent);
    if dark { dark_variant } else { light_variant }
}

#[cfg(test)]
#[path = "tests/theme.rs"]
mod tests;
