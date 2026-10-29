//! Keep Vulkan to the GPUs that drive a screen. The loader otherwise opens
//! every installed driver: a sleeping discrete GPU is woken at startup, kept
//! awake while Soliton runs and woken again to release it at exit.
use std::{collections::BTreeSet, fs, path::Path};

/// Variables through which a user, the desktop ("Launch using Discrete GPU")
/// or a driver chooses the GPU. Any of them leaves the choice untouched.
const USER_CHOICE: &[&str] = &[
    "VK_LOADER_DRIVERS_SELECT",
    "VK_LOADER_DRIVERS_DISABLE",
    "VK_DRIVER_FILES",
    "VK_ICD_FILENAMES",
    "VK_ADD_DRIVER_FILES",
    "MESA_VK_DEVICE_SELECT",
    "DRI_PRIME",
    "__NV_PRIME_RENDER_OFFLOAD",
];

/// Vulkan driver manifests of each PCI vendor, as loader globs.
fn drivers(vendor: u16) -> &'static [&'static str] {
    match vendor {
        0x10de => &["*nvidia*", "*nouveau*"],
        0x1002 => &["*radeon*", "*amd*"],
        0x8086 => &["*intel*"],
        _ => &[],
    }
}

/// Globs of the drivers of GPU vendors with no connected screen, or `None`
/// when nothing is known to be safe to disable. `gpus` lists each card's
/// vendor and whether one of its connectors is connected.
pub(super) fn idle_drivers(gpus: &[(u16, bool)]) -> Option<String> {
    let shown: BTreeSet<u16> = gpus.iter().filter(|g| g.1).map(|g| g.0).collect();
    // No screen seen (sysfs hidden, headless session): change nothing.
    if shown.is_empty() {
        return None;
    }
    let idle: BTreeSet<u16> = gpus
        .iter()
        .map(|g| g.0)
        .filter(|vendor| !shown.contains(vendor))
        .collect();
    let globs: Vec<&str> = idle.into_iter().flat_map(drivers).copied().collect();
    (!globs.is_empty()).then(|| globs.join(","))
}

fn read_gpus(drm: &Path) -> Vec<(u16, bool)> {
    let Ok(entries) = fs::read_dir(drm) else {
        return Vec::new();
    };
    let names: Vec<String> = entries
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names
        .iter()
        .filter(|name| name.starts_with("card") && !name.contains('-'))
        .filter_map(|card| {
            let vendor = fs::read_to_string(drm.join(card).join("device/vendor")).ok()?;
            let vendor = u16::from_str_radix(vendor.trim().trim_start_matches("0x"), 16).ok()?;
            let prefix = format!("{card}-");
            let connected = names.iter().filter(|n| n.starts_with(&prefix)).any(|n| {
                fs::read_to_string(drm.join(n).join("status"))
                    .is_ok_and(|s| s.trim() == "connected")
            });
            Some((vendor, connected))
        })
        .collect()
}

/// Must run before any other thread exists. Returns the disabled drivers.
pub fn prefer_display_gpu() -> Option<String> {
    if USER_CHOICE
        .iter()
        .any(|name| std::env::var_os(name).is_some())
    {
        return None;
    }
    let globs = idle_drivers(&read_gpus(Path::new("/sys/class/drm")))?;
    // SAFETY: called first in `main`, while the process has a single thread.
    unsafe { std::env::set_var("VK_LOADER_DRIVERS_DISABLE", &globs) };
    Some(globs)
}

#[cfg(test)]
#[path = "tests/gpu.rs"]
mod tests;
