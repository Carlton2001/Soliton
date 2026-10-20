/// Comparison key ignoring case and whitespace variations between tags.
/// Used for grouping, sorting and filtering; never displayed.
pub fn fold(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
