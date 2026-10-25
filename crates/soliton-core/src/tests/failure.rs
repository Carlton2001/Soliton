use super::*;
use anyhow::Context;

#[test]
fn attached_failures_survive_context_and_others_keep_their_message() {
    let typed = Err::<(), _>(Failure::with(Kind::OutputUnavailable, "Speakers"))
        .context("Opening the stream")
        .unwrap_err();
    assert_eq!(
        Failure::of(&typed),
        Failure::with(Kind::OutputUnavailable, "Speakers")
    );
    assert_eq!(
        Failure::wrap(Kind::UnreadableAudio, &typed).kind,
        Kind::OutputUnavailable
    );
    let plain = anyhow::anyhow!("disk full").context("Saving");
    assert_eq!(
        Failure::of(&plain),
        Failure::with(Kind::Other, "Saving: disk full")
    );
    assert_eq!(
        Failure::wrap(Kind::SettingsNotSaved, &plain),
        Failure::with(Kind::SettingsNotSaved, "Saving: disk full")
    );
    assert_eq!(Failure::of(&plain).to_string(), "Saving: disk full");
    assert_eq!(Failure::new(Kind::Busy).to_string(), "Busy");
    assert_eq!(
        Failure::new(Kind::TrackUnavailable).about("a.flac").detail,
        "a.flac"
    );
    assert_eq!(
        Failure::with(Kind::UnreadableAudio, "bad header")
            .about("a.flac")
            .detail,
        "a.flac: bad header"
    );
}
