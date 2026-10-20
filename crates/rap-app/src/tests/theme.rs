use super::*;

fn luminance(color: Rgba) -> f32 {
    let channel = |c: f32| {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
}

fn contrast(a: Rgba, b: Rgba) -> f32 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

#[test]
fn every_accent_is_readable_on_every_surface() {
    for dark in [true, false] {
        for accent in Accent::ALL {
            let theme = Theme::new(dark, accent);
            for (name, surface) in [
                ("background", theme.background),
                ("panel", theme.panel),
                ("card", theme.card),
            ] {
                let ratio = contrast(theme.accent, surface);
                assert!(ratio >= 4.5, "{accent:?} dark={dark} on {name}: {ratio:.2}");
            }
        }
    }
}

#[test]
fn text_stays_readable() {
    for dark in [true, false] {
        let theme = Theme::new(dark, Accent::Teal);
        for surface in [theme.background, theme.panel, theme.card] {
            assert!(contrast(theme.text, surface) >= 7.0);
            assert!(contrast(theme.muted, surface) >= 4.5);
        }
    }
}
