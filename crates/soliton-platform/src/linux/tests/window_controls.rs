use super::*;
use CaptionButton::{Close, Maximize, Minimize};

#[test]
fn gnome_preserves_order_and_split_groups() {
    let controls = gnome_layout("menu,close,minimize:maximize").unwrap();
    assert_eq!(controls.left, [Close, Minimize]);
    assert_eq!(controls.right, [Maximize]);
}

#[test]
fn gnome_preserves_hidden_buttons() {
    let controls = gnome_layout(":close").unwrap();
    assert!(controls.left.is_empty());
    assert_eq!(controls.right, [Close]);
    assert!(gnome_layout("invalid").is_none());
}

#[test]
fn kde_reads_decoration_section_and_explicitly_empty_side() {
    let controls = kde_config_layout([
        "[org.kde.kdecoration2]\nButtonsOnLeft=M XAI\nButtonsOnRight=\n[Windows]\nButtonsOnRight=IAX",
    ]);
    assert_eq!(controls.left, [Close, Maximize, Minimize]);
    assert!(controls.right.is_empty());
}

#[test]
fn kde_uses_defaults_for_missing_keys_and_ignores_duplicates() {
    let defaults = kde_config_layout([]);
    assert!(defaults.left.is_empty());
    assert_eq!(defaults.right, [Minimize, Maximize, Close]);
    let controls = kde_config_layout(["[org.kde.kdecoration2]\nButtonsOnLeft=X"]);
    assert_eq!(controls.left, [Close]);
    assert_eq!(controls.right, [Minimize, Maximize]);
}

#[test]
fn kde_inherits_each_side_and_respects_user_overrides() {
    let system = "[org.kde.kdecoration2]\nButtonsOnLeft=X\nButtonsOnRight=AI";
    let controls = kde_config_layout([system, "[Windows]\nBorderlessMaximizedWindows=true"]);
    assert_eq!(controls.left, [Close]);
    assert_eq!(controls.right, [Maximize, Minimize]);

    let controls = kde_config_layout([system, "[org.kde.kdecoration2]\nButtonsOnRight="]);
    assert_eq!(controls.left, [Close]);
    assert!(controls.right.is_empty());
}
