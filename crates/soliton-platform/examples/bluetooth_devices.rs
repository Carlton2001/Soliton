//! Read-only native QA. Does not discover unpaired devices or change connections.
fn main() -> anyhow::Result<()> {
    for device in soliton_platform::Bluetooth::devices()? {
        println!(
            "{} | connected={} | audio outputs={:?}",
            device.name, device.connected, device.outputs
        );
    }
    Ok(())
}
