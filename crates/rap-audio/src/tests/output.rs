use super::*;

#[test]
fn callback_keeps_stereo_frames_aligned_across_underflow_and_pause() {
    let queue = ArrayQueue::new(2);
    queue.push([0.2, -0.4]).unwrap();
    let mut output = [9f32; 4];
    assert_eq!(render(&mut output, 2, &queue, true, 0.5), 1);
    assert_eq!(output, [0.1, -0.2, 0., 0.]);
    queue.push([0.3, -0.7]).unwrap();
    assert_eq!(render(&mut output, 2, &queue, false, 1.), 0);
    assert_eq!(output, [0.; 4]);
    assert_eq!(queue.len(), 1);
    assert_eq!(render(&mut output, 2, &queue, true, 1.), 1);
    assert_eq!(output, [0.3, -0.7, 0., 0.]);
}

#[test]
fn surround_devices_receive_stereo_on_front_channels_and_silence_elsewhere() {
    let queue = ArrayQueue::new(2);
    queue.push([0.5, -0.25]).unwrap();
    queue.push([0.75, 0.125]).unwrap();
    let mut output = [9f32; 12];
    assert_eq!(render(&mut output, 6, &queue, true, 1.), 2);
    assert_eq!(
        output,
        [0.5, -0.25, 0., 0., 0., 0., 0.75, 0.125, 0., 0., 0., 0.]
    );
    // Device-native integer formats get the same channel mapping.
    queue.push([1., -1.]).unwrap();
    let mut output = [7i16; 8];
    assert_eq!(render(&mut output, 8, &queue, true, 1.), 1);
    assert_eq!(output, [i16::MAX, i16::MIN, 0, 0, 0, 0, 0, 0]);
}

#[test]
fn mono_devices_take_the_first_frame_channel() {
    let queue = ArrayQueue::new(1);
    queue.push([0.5, 0.]).unwrap();
    let mut output = [9f32; 1];
    assert_eq!(render(&mut output, 1, &queue, true, 1.), 1);
    assert_eq!(output, [0.5]);
}
