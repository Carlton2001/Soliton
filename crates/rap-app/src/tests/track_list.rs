use super::album_track_number;
use rap_core::model::Metadata;

#[test]
fn album_numbers_use_tagged_track_numbers_and_prefix_discs_only_when_needed() {
    let mut metadata = Metadata {
        disc: Some(1),
        track: Some(7),
        ..Default::default()
    };
    assert_eq!(album_track_number(&metadata, 0, true), "1.7");
    assert_eq!(album_track_number(&metadata, 0, false), "7");
    metadata.disc = Some(2);
    metadata.track = Some(1);
    assert_eq!(album_track_number(&metadata, 13, true), "2.1");
    metadata.disc = Some(12);
    metadata.track = Some(107);
    assert_eq!(album_track_number(&metadata, 0, true), "12.107");
}

#[test]
fn missing_or_zero_numbers_fall_back_to_first_disc_and_list_position() {
    let mut metadata = Metadata::default();
    assert_eq!(album_track_number(&metadata, 6, true), "1.7");
    assert_eq!(album_track_number(&metadata, 6, false), "7");
    metadata.disc = Some(0);
    metadata.track = Some(0);
    assert_eq!(album_track_number(&metadata, 6, true), "1.7");
}
