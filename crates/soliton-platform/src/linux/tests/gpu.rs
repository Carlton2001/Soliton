use super::idle_drivers;

#[test]
fn hybrid_laptop_disables_the_idle_discrete_gpu() {
    assert_eq!(
        idle_drivers(&[(0x10de, false), (0x8086, true)]).as_deref(),
        Some("*nvidia*,*nouveau*")
    );
    assert_eq!(
        idle_drivers(&[(0x8086, true), (0x1002, false)]).as_deref(),
        Some("*radeon*,*amd*")
    );
}

#[test]
fn every_gpu_with_a_screen_is_kept() {
    // External monitor wired to the discrete GPU.
    assert_eq!(idle_drivers(&[(0x10de, true), (0x8086, true)]), None);
    // NVIDIA alone, or driving the display.
    assert_eq!(idle_drivers(&[(0x10de, true)]), None);
    assert_eq!(
        idle_drivers(&[(0x10de, true), (0x8086, false)]).as_deref(),
        Some("*intel*")
    );
}

#[test]
fn unknown_screens_change_nothing() {
    assert_eq!(idle_drivers(&[]), None);
    assert_eq!(idle_drivers(&[(0x10de, false), (0x8086, false)]), None);
}
