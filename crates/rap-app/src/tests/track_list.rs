use super::track_number;
use rap_core::model::Metadata;

#[test]
fn source_numbers_always_include_disc_and_ignore_list_position() {
    let mut metadata = Metadata {
        disc: Some(1),
        track: Some(7),
        ..Default::default()
    };
    assert_eq!(track_number(&metadata, 0), "1.7");
    assert_eq!(track_number(&metadata, 20), "1.7");
    metadata.disc = Some(2);
    metadata.track = Some(1);
    assert_eq!(track_number(&metadata, 13), "2.1");
    metadata.disc = Some(12);
    metadata.track = Some(107);
    assert_eq!(track_number(&metadata, 0), "12.107");
}

#[test]
fn missing_or_zero_numbers_fall_back_to_first_disc_and_list_position() {
    let mut metadata = Metadata::default();
    assert_eq!(track_number(&metadata, 6), "1.7");
    metadata.track = Some(5);
    assert_eq!(track_number(&metadata, 20), "1.5");
    metadata.disc = Some(0);
    metadata.track = Some(0);
    assert_eq!(track_number(&metadata, 6), "1.7");
}
