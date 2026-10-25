use super::*;

#[test]
fn ignores_case_accents_ligatures_and_spacing() {
    assert_eq!(fold("  Été   à Rome "), "ete a rome");
    assert_eq!(fold("BJÖRK"), fold("bjork"));
    assert_eq!(fold("Œuvres complètes"), "oeuvres completes");
    assert_eq!(fold("Straße"), "strasse");
    assert_eq!(fold("Mötley Crüe"), fold("Motley Crue"));
    assert_eq!(fold("Ｆｕｌｌ ｗｉｄｔｈ"), "full width");
    // Distinct letters stay distinct.
    assert_ne!(fold("Ana"), fold("Ann"));
    assert_eq!(fold("日本語"), "日本語");
    assert_ne!(fold("ガラス"), fold("カラス"));
}
