use super::{
    Theme, accent_button, button, checkbox, ghost, icon_button, menu_item, nav_icon, nav_item,
    radio_row, segment, toggle_button,
};
use soliton_core::config::Accent;

#[test]
fn shared_controls_build_without_duplicate_interaction_styles() {
    for dark in [false, true] {
        let theme = Theme::new(dark, Accent::Teal);
        let _ = button("button", "play", "Play", theme);
        let _ = accent_button("primary", "play", "Play", theme);
        let _ = super::with_tooltip(
            segment("stepper", Some("minus"), "", false, theme),
            "Reduce".into(),
            theme,
        );
        let _ = ghost("ghost", "play", "Play", theme);
        for selected in [false, true] {
            let _ = toggle_button("toggle", "tag", "Filters", selected, theme);
            let _ = icon_button("icon", "play", "Play", selected, theme);
            let _ = nav_item("nav", "disc-3", "Albums", selected, theme);
            let _ = nav_icon("rail", "disc-3", "Albums", selected, theme);
            let _ = segment("segment", Some("list"), "List", selected, theme);
            let _ = menu_item("menu", "Option", selected, theme);
            let _ = checkbox("checkbox", selected, "Option", theme);
            let _ = radio_row("radio", "volume-2", "Output", selected, theme);
        }
    }
}

#[test]
fn durations_show_hours_from_one_hour() {
    assert_eq!(super::time(0), "0:00");
    assert_eq!(super::time(59_999), "0:59");
    assert_eq!(super::time(3_599_000), "59:59");
    assert_eq!(super::time(3_600_000), "1:00:00");
    assert_eq!(super::time(4_501_000), "1:15:01");
}
