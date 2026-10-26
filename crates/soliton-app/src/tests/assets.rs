use super::*;

#[test]
fn every_palette_loads_the_mirrored_wave_from_bundled_assets() {
    let assets = Assets;
    let mut previous = Vec::new();
    for icon in AppIcon::ALL {
        let path = Assets::logo_path(icon);
        let bytes = assets.load(&path).unwrap().expect("bundled icon");
        let svg = std::str::from_utf8(bytes.as_ref()).unwrap();
        assert!(svg.contains("translate(256 0) scale(-1 1)"));
        assert!(svg.contains("<path"));
        assert!(!previous.iter().any(|other| other == bytes.as_ref()));
        previous.push(bytes.into_owned());
    }
    assert!(assets.load("logos/missing.svg").unwrap().is_none());
}

#[test]
fn icons_load_with_a_lighter_stroke_and_unchanged_paths() {
    let file = include_str!("../../../../assets/icons/play.svg");
    let bytes = Assets
        .load("icons/play.svg")
        .unwrap()
        .expect("bundled icon");
    let svg = std::str::from_utf8(bytes.as_ref()).unwrap();
    assert!(file.contains(r#"stroke-width="2""#));
    assert!(svg.contains(ICON_STROKE_WIDTH));
    assert!(!svg.contains(r#"stroke-width="2""#));
    assert_eq!(svg.replace(ICON_STROKE_WIDTH, r#"stroke-width="2""#), file);
}
