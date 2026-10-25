use unicode_normalization::UnicodeNormalization;

/// Comparison key ignoring case, accents and whitespace variations between tags.
/// Used for grouping, sorting and filtering; never displayed.
pub fn fold(text: &str) -> String {
    let mut folded = String::with_capacity(text.len());
    // Compatibility decomposition separates accents and expands forms such as `ﬁ`.
    for c in text.nfkd().filter(|c| !diacritic(*c)) {
        match c {
            'œ' | 'Œ' => folded.push_str("oe"),
            'æ' | 'Æ' => folded.push_str("ae"),
            'ß' | 'ẞ' => folded.push_str("ss"),
            'ø' | 'Ø' => folded.push('o'),
            'ł' | 'Ł' => folded.push('l'),
            'đ' | 'Đ' => folded.push('d'),
            c => folded.extend(c.to_lowercase()),
        }
    }
    folded.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Combining diacritical mark blocks. Other combining marks, such as kana voicing
/// marks, change the letter itself and are kept.
fn diacritic(c: char) -> bool {
    matches!(
        c,
        '\u{0300}'..='\u{036F}'
            | '\u{1AB0}'..='\u{1AFF}'
            | '\u{1DC0}'..='\u{1DFF}'
            | '\u{20D0}'..='\u{20FF}'
            | '\u{FE20}'..='\u{FE2F}'
    )
}

#[cfg(test)]
#[path = "tests/text.rs"]
mod tests;
