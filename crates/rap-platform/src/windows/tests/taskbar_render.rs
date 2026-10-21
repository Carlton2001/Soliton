use super::*;
use crate::MediaTrack;

#[test]
fn duration_progress_and_thumbnail_size_are_bounded() {
    assert_eq!(duration(3_661_000), "1:01:01");
    assert_eq!(progress(50, 100, 592), 296);
    assert_eq!(progress(u64::MAX, u64::MAX, 592), 592);
    assert_eq!(progress(500, 0, 592), 0);
    assert_eq!(super::super::bitmap::fitted(640, 360, 200, 200), (200, 112));
    assert_eq!(super::super::bitmap::fitted(640, 360, 0, 0), (1, 1));
}

#[test]
fn render_clears_artwork_and_uses_the_supplied_palette() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("cover.png");
    image::RgbaImage::from_pixel(32, 32, image::Rgba([250, 25, 10, 255]))
        .save(&path)
        .unwrap();
    let mut renderer = Renderer::default();
    let mut state = MediaState {
        track: Some(MediaTrack {
            title: "L’été & le temps — titre très long ".repeat(10),
            artist: "Artiste".into(),
            album: "Album".into(),
            cover: Some(path.canonicalize().unwrap()),
        }),
        duration_ms: 420_000,
        position_ms: 42_000,
        ..Default::default()
    };
    let frame = renderer.render(&state, Language::French).unwrap();
    let pixel = (80 * frame.width as usize + 80) * 4;
    assert_eq!(&frame.pixels[pixel..pixel + 4], &[10, 25, 250, 255]);
    assert!(frame.pixels.chunks_exact(4).all(|pixel| pixel[3] == 255));
    state.track.as_mut().unwrap().cover = None;
    state.preview.background = 0xf5f6f8;
    let clear = renderer.render(&state, Language::English).unwrap();
    assert_eq!(&clear.pixels[pixel..pixel + 4], &[248, 246, 245, 255]);
}

#[test]
#[ignore = "Exports a Windows GDI preview for visual QA"]
fn export_preview_for_visual_qa() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target");
    let mut renderer = Renderer::default();
    let state = MediaState {
        track: Some(MediaTrack {
            title: "Run Like Hell".into(),
            artist: "Pink Floyd".into(),
            album: "Is There Anybody Out There?".into(),
            cover: None,
        }),
        duration_ms: 425_000,
        position_ms: 90_000,
        ..Default::default()
    };
    let frame = renderer.render(&state, Language::English).unwrap();
    frame
        .bitmap(320, 180)
        .unwrap()
        .save(&root.join("qa-taskbar-preview.png"))
        .unwrap();
}
