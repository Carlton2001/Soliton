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

#[test]
fn flush_never_lets_frames_from_before_it_play_or_count_after_it() {
    let samples = Arc::new(ArrayQueue::new(4096));
    let playing = Arc::new(AtomicBool::new(true));
    let played = Arc::new(AtomicU64::new(0));
    let callbacks = Arc::new(AtomicU64::new(0));
    let running = Arc::new(AtomicBool::new(true));
    for _ in 0..4096 {
        samples.push([0.25, 0.25]).unwrap();
    }
    // Simulates the device thread with the production callback sequence.
    let device = {
        let (samples, playing, played, callbacks, running) = (
            samples.clone(),
            playing.clone(),
            played.clone(),
            callbacks.clone(),
            running.clone(),
        );
        std::thread::spawn(move || {
            let mut heard: Vec<f32> = Vec::new();
            let mut buffer = [0f32; 64];
            while running.load(Ordering::SeqCst) {
                let active = playing.load(Ordering::SeqCst);
                let consumed = render(&mut buffer, 2, &samples, active, 1.);
                played.fetch_add(consumed, Ordering::Relaxed);
                callbacks.fetch_add(1, Ordering::SeqCst);
                heard.extend(buffer.iter().step_by(2).take(consumed as usize).copied());
                std::thread::sleep(Duration::from_micros(200));
            }
            heard
        })
    };
    std::thread::sleep(Duration::from_millis(5));
    flush(&samples, &playing, &played, &callbacks);
    assert!(samples.is_empty());
    assert_eq!(played.load(Ordering::SeqCst), 0);
    for _ in 0..256 {
        samples.push([0.5, 0.5]).unwrap();
    }
    playing.store(true, Ordering::SeqCst);
    let start = Instant::now();
    while !samples.is_empty() {
        assert!(start.elapsed() < Duration::from_secs(5));
        std::thread::yield_now();
    }
    running.store(false, Ordering::SeqCst);
    let heard = device.join().unwrap();
    let after = heard.iter().position(|&value| value == 0.5).unwrap();
    assert!(heard[..after].iter().all(|&value| value == 0.25));
    assert!(heard[after..].iter().all(|&value| value == 0.5));
    assert_eq!(heard.len() - after, 256);
    assert_eq!(played.load(Ordering::SeqCst), 256);
}
