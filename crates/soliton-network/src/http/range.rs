//! Single byte ranges, including suffix and open-ended requests. Multiple ranges
//! are rejected rather than silently returning an unrelated portion of a file.

/// What to send for a request's `Range` header.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Span {
    /// The whole file: no range, or one in a unit other than bytes, which
    /// RFC 9110 requires ignoring.
    Whole,
    /// Inclusive byte offsets.
    Part(u64, u64),
    Unsatisfiable,
}

pub(super) fn span(value: Option<&str>, length: u64) -> Span {
    match value {
        None => Span::Whole,
        Some(value) if !value.starts_with("bytes=") && value.contains('=') => Span::Whole,
        Some(value) => {
            parse(value, length).map_or(Span::Unsatisfiable, |(start, end)| Span::Part(start, end))
        }
    }
}

fn parse(value: &str, length: u64) -> Option<(u64, u64)> {
    let (first, last) = value.strip_prefix("bytes=")?.split_once('-')?;
    if length == 0
        || value.contains(',')
        || !first.bytes().all(|b| b.is_ascii_digit())
        || !last.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    if first.is_empty() {
        let suffix: u64 = last.parse().ok()?;
        (suffix > 0).then(|| (length.saturating_sub(suffix), length - 1))
    } else {
        let start: u64 = first.parse().ok()?;
        let end: u64 = if last.is_empty() {
            length - 1
        } else {
            last.parse().ok()?
        };
        (start < length && end >= start).then_some((start, end.min(length - 1)))
    }
}

#[cfg(test)]
#[path = "../tests/range.rs"]
mod tests;
