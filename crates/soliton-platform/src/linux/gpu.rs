//! Prefer the GPU the desktop renders with. The Vulkan loader otherwise opens
//! every installed driver, which can wake a sleeping discrete GPU.
//! Adapted from Oxyde-GPUI (Carlton2001, GPL-3.0-only).
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

mod compositor;

const DISABLED: &str = "VK_LOADER_DRIVERS_DISABLE";

/// Variables through which a user, the desktop ("Launch using Discrete GPU")
/// or a driver chooses the GPU. Any of them leaves the choice untouched.
const USER_CHOICE: &[&str] = &[
    "VK_LOADER_DRIVERS_SELECT",
    DISABLED,
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

struct Card {
    name: String,
    vendor: u16,
    device: PathBuf,
    suspended: bool,
}

fn read_cards(drm: &Path) -> Vec<Card> {
    let Ok(entries) = fs::read_dir(drm) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            name.strip_prefix("card")?.parse::<u32>().ok()?;
            let device = fs::canonicalize(entry.path().join("device")).ok()?;
            let vendor = fs::read_to_string(device.join("vendor")).ok()?;
            let vendor = u16::from_str_radix(vendor.trim().trim_start_matches("0x"), 16).ok()?;
            let suspended = fs::read_to_string(device.join("power/runtime_status"))
                .is_ok_and(|status| status.trim() == "suspended");
            Some(Card {
                name,
                vendor,
                device,
                suspended,
            })
        })
        .collect()
}

/// A primary DRM node and a render node can point to the same PCI device.
fn card_of_node<'a>(cards: &'a [Card], dev_char: &Path, node: &str) -> Option<&'a Card> {
    let device = fs::canonicalize(dev_char.join(node).join("device")).ok()?;
    cards.iter().find(|card| card.device == device)
}

fn idle_drivers(cards: &[Card], rendering: Option<&Card>) -> Option<String> {
    let kept: BTreeSet<u16> = cards
        .iter()
        .filter(|card| rendering.map_or(!card.suspended, |chosen| card.vendor == chosen.vendor))
        .map(|card| card.vendor)
        .collect();
    // Without a GPU to keep, let Vulkan pick from all available drivers.
    if kept.is_empty() {
        return None;
    }
    let idle: BTreeSet<u16> = cards
        .iter()
        .map(|card| card.vendor)
        .filter(|vendor| !kept.contains(vendor))
        .collect();
    let globs: Vec<&str> = idle.into_iter().flat_map(drivers).copied().collect();
    (!globs.is_empty()).then(|| globs.join(","))
}

/// Must run before any other thread exists. Returns the disabled drivers and why.
pub fn prefer_display_gpu() -> Option<String> {
    if USER_CHOICE
        .iter()
        .any(|name| std::env::var_os(name).is_some())
    {
        return None;
    }
    let cards = read_cards(Path::new("/sys/class/drm"));
    let vendors: BTreeSet<u16> = cards
        .iter()
        .map(|card| card.vendor)
        .filter(|vendor| !drivers(*vendor).is_empty())
        .collect();
    if vendors.len() < 2 {
        return None;
    }
    let rendering = compositor::main_device()
        .and_then(|node| card_of_node(&cards, Path::new("/sys/dev/char"), &node));
    let globs = idle_drivers(&cards, rendering)?;
    // SAFETY: called first in `main`, while the process has a single thread.
    unsafe { std::env::set_var(DISABLED, &globs) };
    Some(match rendering {
        Some(card) => format!("{globs} (Wayland compositor GPU: {})", card.name),
        None => format!("{globs} (GPUs in runtime suspend)"),
    })
}

#[cfg(test)]
#[path = "tests/gpu.rs"]
mod tests;
