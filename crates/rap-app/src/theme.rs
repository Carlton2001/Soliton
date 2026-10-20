use gpui::{Rgba, rgb};
use rap_core::config::Accent;

#[derive(Clone, Copy)]
pub struct Theme {
    pub background: Rgba,
    pub panel: Rgba,
    pub card: Rgba,
    pub text: Rgba,
    pub muted: Rgba,
    pub border: Rgba,
    pub accent: Rgba,
    /// Translucent layer behind overlays and drop targets.
    pub scrim: Rgba,
}

impl Theme {
    pub fn new(dark: bool, accent: Accent) -> Self {
        let accent = rgb(accent_color(accent, dark));
        if dark {
            Self {
                background: rgb(0x14171c),
                panel: rgb(0x191d24),
                card: rgb(0x222831),
                text: rgb(0xeef2f7),
                muted: rgb(0x9ba8b8),
                border: rgb(0x303944),
                accent,
                scrim: Rgba {
                    r: 0.04,
                    g: 0.05,
                    b: 0.06,
                    a: 0.82,
                },
            }
        } else {
            Self {
                background: rgb(0xf5f6f8),
                panel: rgb(0xffffff),
                card: rgb(0xe7edf1),
                text: rgb(0x202b37),
                muted: rgb(0x5b6876),
                border: rgb(0xd6dfe5),
                accent,
                scrim: Rgba {
                    r: 0.96,
                    g: 0.965,
                    b: 0.97,
                    a: 0.88,
                },
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
