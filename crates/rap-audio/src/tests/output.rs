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
