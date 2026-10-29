use super::variants;

fn lightness(rgb: u32) -> f64 {
    let c =
        [rgb >> 16, (rgb >> 8) & 0xff, rgb & 0xff].map(|c| super::to_linear(f64::from(c) / 255.0));
    super::oklab(c)[0]
}

#[test]
fn mid_tone_accent_is_darkened_for_light_and_lightened_for_dark() {
    // GNOME's pink accent as published by the portal.
    let [light, dark] = variants([0.835, 0.380, 0.600]);
    assert!(lightness(light) <= 0.501);
    assert!(lightness(dark) >= 0.849);
    // Hue kept: red stays the dominant component.
    assert!(light >> 16 > light & 0xff && dark >> 16 > dark & 0xff);
}

#[test]
fn accent_already_in_range_is_kept() {
    // OKLab L of #3584e4 (GNOME blue) is about 0.6: only the light variant moves.
    let [_, dark] = variants([
        0x35 as f64 / 255.0,
        0x84 as f64 / 255.0,
        0xe4 as f64 / 255.0,
    ]);
    assert!(lightness(dark) >= 0.849);
    let [light, _] = variants([0.1, 0.2, 0.4]);
    assert_eq!(light, 0x1a3366);
}
