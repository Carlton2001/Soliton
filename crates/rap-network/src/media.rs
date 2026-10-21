use anyhow::{Result, bail};
use rap_core::model::Track;

pub(crate) fn mime(track: &Track) -> Result<&'static str> {
    let extension = track
        .path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "mp3" => Ok("audio/mpeg"),
        "flac" => Ok("audio/flac"),
        "m4a" | "mp4" => Ok("audio/mp4"),
        "aac" => Ok("audio/aac"),
        "wav" | "wave" => Ok("audio/wav"),
        "aif" | "aiff" => Ok("audio/aiff"),
        "ogg" | "oga" => Ok("audio/ogg"),
        _ => bail!("Unsupported network audio format: {extension}"),
    }
}

pub(crate) fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub(crate) fn duration(milliseconds: u64) -> String {
    let seconds = milliseconds / 1000;
    format!(
        "{}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}

pub(crate) fn milliseconds(value: &str) -> Option<u64> {
    let mut parts = value.split(':');
    let hours: u64 = parts.next()?.parse().ok()?;
    let minutes: u64 = parts.next()?.parse().ok()?;
    let seconds: f64 = parts.next()?.parse().ok()?;
    if parts.next().is_some()
        || minutes >= 60
        || !seconds.is_finite()
        || !(0.0..60.0).contains(&seconds)
    {
        return None;
    }
    hours
        .checked_mul(3_600_000)?
        .checked_add(minutes * 60_000)?
        .checked_add((seconds * 1000.) as u64)
}

#[cfg(test)]
#[path = "tests/media.rs"]
mod tests;
