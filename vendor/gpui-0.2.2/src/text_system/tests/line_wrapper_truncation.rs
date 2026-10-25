use super::*;
use crate::{TestAppContext, TestDispatcher, font};
use rand::{SeedableRng, rngs::StdRng};

#[test]
fn repeated_measurements_keep_unicode_run_boundaries() {
    let dispatcher = TestDispatcher::new(StdRng::seed_from_u64(0));
    let cx = TestAppContext::build(dispatcher, None);
    let font = font(".ZedMono");
    let id = cx.text_system().resolve_font(&font);
    let mut wrapper = LineWrapper::new(id, px(16.), cx.text_system().platform_text_system.clone());

    for original in ["l’application", "Été à Montréal", "🎵 l’album 日本語"] {
        let source = vec![TextRun {
            len: original.len(),
            font: font.clone(),
            color: Default::default(),
            background_color: None,
            underline: None,
            strikethrough: None,
        }];
        for suffix in ["…", ""] {
            for width in [px(1.), px(500.), px(40.), px(500.), px(1.)] {
                let (text, runs) =
                    wrapper.truncate_line_with_runs(original.into(), width, suffix, &source);
                assert_eq!(source[0].len, original.len());
                assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), text.len());
                let mut offset = 0;
                for run in runs {
                    offset += run.len;
                    assert!(text.is_char_boundary(offset), "{text:?}: {offset}");
                }
                if width == px(500.) {
                    assert_eq!(text.as_ref(), original);
                }
            }
        }
    }
}
