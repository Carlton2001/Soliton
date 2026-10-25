//! Read-only renderer discovery. Does not connect, change volume or start media.
use std::time::{Duration, Instant};

fn main() -> anyhow::Result<()> {
    let mut network = soliton_network::Network::start()?;
    let mut previous = Vec::new();
    let mut error = None;
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(12) {
        if let Some(snapshot) = network.take_update() {
            if snapshot.devices.as_ref() != &previous {
                previous = snapshot.devices.as_ref().clone();
                for device in &previous {
                    println!(
                        "{} | {} | {} | {}",
                        device.id,
                        device.protocol.label(),
                        device.name,
                        device.address
                    );
                }
            }
            if snapshot.discovery_error != error {
                error = snapshot.discovery_error;
                if let Some(error) = &error {
                    eprintln!("{error}");
                }
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    println!("{} receiver(s) discovered", previous.len());
    network.shutdown();
    Ok(())
}
