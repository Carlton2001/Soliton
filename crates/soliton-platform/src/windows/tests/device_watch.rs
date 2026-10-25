use super::*;

#[test]
#[ignore = "Starts native Windows device watchers"]
fn watchers_start_and_stop_without_signalling_existing_devices() {
    let (watch, changes) = watch(&[Source::PairedBluetooth, Source::AudioOutputs]).unwrap();
    // The initial enumeration of already known devices is not a change.
    std::thread::sleep(std::time::Duration::from_secs(2));
    assert!(changes.try_recv().is_err());
    drop(watch);
}
