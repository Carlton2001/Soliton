use super::*;

#[test]
fn native_rate_is_unchanged_and_mono_stereo_mapping_is_explicit() {
    let mut converter = Converter::new(44100, 2, 44100, 2).unwrap();
    assert_eq!(
        converter.process(&[0.1, 0.2, 0.3, 0.4], false).unwrap(),
        vec![0.1, 0.2, 0.3, 0.4]
    );
    let mut mono = Converter::new(44100, 1, 44100, 2).unwrap();
    assert_eq!(
        mono.process(&[0.25, -0.5], false).unwrap(),
        vec![0.25, 0.25, -0.5, -0.5]
    );
}

#[test]
fn resampling_preserves_duration_across_arbitrary_packet_boundaries() {
    let mut converter = Converter::new(44100, 2, 48000, 2).unwrap();
    let input = vec![0.25; 44100 * 2];
    let mut output = Vec::new();
    for packet in input.chunks(1152 * 2) {
        output.extend(converter.process(packet, false).unwrap());
    }
    output.extend(converter.process(&[], true).unwrap());
    assert_eq!(output.len(), 48000 * 2);
    assert!(output.iter().all(|v| v.is_finite()));
    assert!((output[4000] - 0.25).abs() < 0.01);
}
