use std::{io::Write, path::Path};

pub fn wav(path: &Path, rate: u32, seconds: u32) {
    let frames = rate * seconds;
    let data_size = frames * 2;
    let mut file = std::fs::File::create(path).unwrap();
    file.write_all(b"RIFF").unwrap();
    file.write_all(&(36 + data_size).to_le_bytes()).unwrap();
    file.write_all(b"WAVEfmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&rate.to_le_bytes()).unwrap();
    file.write_all(&(rate * 2).to_le_bytes()).unwrap();
    file.write_all(&2u16.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();
    for index in 0..frames {
        let sample =
            ((index as f64 * 440. * std::f64::consts::TAU / rate as f64).sin() * 1000.) as i16;
        file.write_all(&sample.to_le_bytes()).unwrap();
    }
}
