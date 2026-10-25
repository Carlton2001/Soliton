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
            assert!(
                contrast(theme.accent, theme.on_accent) >= 4.5,
                "{accent:?} dark={dark}: text on accent fill"
            );
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

#[test]
fn system_accent_follows_palette_and_theme_without_affecting_fixed_accents() {
    let first = [0x003e92, 0x99caff];
    let changed = [0x5c2d91, 0xd8b9ff];
    for dark in [false, true] {
        let theme = Theme::with_system_accent(dark, Accent::System, Some(first));
        assert_eq!(theme.accent, rgb(first[usize::from(dark)]));
        let updated = Theme::with_system_accent(dark, Accent::System, Some(changed));
        assert_eq!(updated.accent, rgb(changed[usize::from(dark)]));
        assert_eq!(updated.selected.r, updated.accent.r);
        assert_eq!(updated.selected.g, updated.accent.g);
        assert_eq!(updated.selected.b, updated.accent.b);
        assert_eq!(
            Theme::with_system_accent(dark, Accent::Teal, Some(changed)).accent,
            Theme::new(dark, Accent::Teal).accent,
        );
        assert_eq!(
            Theme::new(dark, Accent::System).accent,
            Theme::new(dark, Accent::Blue).accent
        );
    }
}
