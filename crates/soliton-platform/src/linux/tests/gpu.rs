use super::{
    DISABLED, UNFILTERED, card_of_node, gpu_initialization_failed, idle_drivers,
    initialize_display, prefer_display_gpu, read_cards,
};
use std::{fs, os::unix::fs::symlink};

fn sysfs(cards: &[(u16, Option<&str>)]) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    for (index, (vendor, status)) in cards.iter().enumerate() {
        let device = root.path().join(format!("devices/gpu{index}"));
        fs::create_dir_all(device.join("power")).unwrap();
        fs::write(device.join("vendor"), format!("0x{vendor:04x}\n")).unwrap();
        if let Some(status) = status {
            fs::write(device.join("power/runtime_status"), format!("{status}\n")).unwrap();
        }
        for node in [
            format!("drm/card{index}"),
            format!("char/226:{index}"),
            format!("char/226:{}", 128 + index),
        ] {
            let node = root.path().join(node);
            fs::create_dir_all(&node).unwrap();
            symlink(&device, node.join("device")).unwrap();
        }
    }
    root
}

#[test]
fn nvidia_compositor_wins_when_the_screen_is_wired_to_intel() {
    let root = sysfs(&[(0x8086, Some("active")), (0x10de, Some("active"))]);
    let drm = root.path().join("drm");
    let screen = drm.join("card0-eDP-1");
    fs::create_dir(&screen).unwrap();
    fs::write(screen.join("status"), "connected\n").unwrap();

    let cards = read_cards(&drm);
    let dev_char = root.path().join("char");
    for node in ["226:1", "226:129"] {
        let rendering = card_of_node(&cards, &dev_char, node);
        assert_eq!(idle_drivers(&cards, rendering).as_deref(), Some("*intel*"));
    }
    let rendering = card_of_node(&cards, &dev_char, "226:128");
    assert_eq!(
        idle_drivers(&cards, rendering).as_deref(),
        Some("*nvidia*,*nouveau*")
    );
}

#[test]
fn without_compositor_feedback_only_suspended_vendors_are_filtered() {
    let root = sysfs(&[
        (0x8086, Some("active")),
        (0x1002, Some("suspended")),
        (0x10de, None),
    ]);
    let cards = read_cards(&root.path().join("drm"));
    let rendering = card_of_node(&cards, &root.path().join("char"), "226:200");
    assert!(rendering.is_none());
    assert_eq!(
        idle_drivers(&cards, rendering).as_deref(),
        Some("*radeon*,*amd*")
    );
}

#[test]
fn a_vendor_with_an_awake_card_keeps_its_drivers() {
    let root = sysfs(&[
        (0x8086, Some("active")),
        (0x10de, Some("active")),
        (0x10de, Some("suspended")),
    ]);
    let cards = read_cards(&root.path().join("drm"));
    assert_eq!(idle_drivers(&cards, None), None);
}

#[test]
fn without_a_gpu_to_keep_all_drivers_remain_available() {
    assert_eq!(idle_drivers(&[], None), None);
    let root = sysfs(&[(0x8086, Some("suspended")), (0x10de, Some("suspended"))]);
    let cards = read_cards(&root.path().join("drm"));
    assert_eq!(idle_drivers(&cards, None), None);
}

#[test]
fn only_vulkan_initialization_panics_request_an_unfiltered_start() {
    use anyhow::Context as _;

    fn wayland(result: anyhow::Result<()>) {
        result.expect("Unable to init GPU context");
    }

    fn x11(result: anyhow::Result<()>) {
        result.context("Failed to initialize X11 client.").unwrap();
    }

    let failed_starts: [fn(); 2] = [
        || wayland(Err(anyhow::anyhow!("NoSupportedDeviceFound"))),
        || x11(Err(anyhow::anyhow!("NoSupportedDeviceFound")).context("Unable to init GPU context")),
    ];
    for start in failed_starts {
        let panic = std::panic::catch_unwind(start).unwrap_err();
        assert!(gpu_initialization_failed(panic.as_ref()));
    }

    let panic = std::panic::catch_unwind(|| x11(Err(anyhow::anyhow!("X11 connection closed"))))
        .unwrap_err();
    assert!(!gpu_initialization_failed(panic.as_ref()));
}

#[test]
fn a_failed_filtered_start_reexecutes_without_the_filter() {
    const CHILD: &str = "SOLITON_TEST_VULKAN_RESTART";
    if std::env::var_os(CHILD).is_some() {
        if std::env::var_os(UNFILTERED).is_some() {
            assert!(std::env::var_os(DISABLED).is_none());
            assert!(prefer_display_gpu().is_none());
            println!("Vulkan fallback completed");
            return;
        }
        initialize_display(true, || panic!("Unable to init GPU context: test failure")).unwrap();
        unreachable!("The process should have restarted");
    }

    let retry = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "a_failed_filtered_start_reexecutes_without_the_filter",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .env(DISABLED, "*nvidia*")
        .env_remove(UNFILTERED)
        .output()
        .unwrap();
    assert!(retry.status.success(), "{:?}", retry);
    assert!(String::from_utf8_lossy(&retry.stdout).contains("Vulkan fallback completed"));
}
