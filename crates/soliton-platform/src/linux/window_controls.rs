//! Read desktop button placement off the UI thread.

use crate::{CaptionButton, CaptionStyle, DesktopEvent, WindowControls};
use crossbeam_channel::Sender;
use std::{path::Path, process::Command, sync::OnceLock};
use zbus::{
    blocking::{Connection, Proxy},
    zvariant::OwnedValue,
};

const NAMESPACE: &str = "org.gnome.desktop.wm.preferences";
const KEY: &str = "button-layout";
const KDE_GROUP: &str = "org.kde.kdecoration2";
const KDE_LEFT: &str = "ButtonsOnLeft";
const KDE_RIGHT: &str = "ButtonsOnRight";
const KDE_DEFAULT_RIGHT: &str = "IAX";

pub(crate) fn style() -> CaptionStyle {
    static STYLE: OnceLock<CaptionStyle> = OnceLock::new();
    *STYLE.get_or_init(|| {
        let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
        if desktop
            .split(':')
            .any(|name| name.eq_ignore_ascii_case("KDE"))
        {
            CaptionStyle::Breeze
        } else {
            CaptionStyle::Adwaita
        }
    })
}

pub(super) fn watch(events: Sender<DesktopEvent>) {
    if style() == CaptionStyle::Breeze {
        let _ = events.try_send(DesktopEvent::WindowControls(kde_controls()));
        return;
    }
    let _ = std::thread::Builder::new()
        .name("soliton-caption-layout".into())
        .spawn(move || watch_gnome(events));
}

fn watch_gnome(events: Sender<DesktopEvent>) {
    let Ok(connection) = Connection::session() else {
        return;
    };
    let Ok(proxy) = Proxy::new(
        &connection,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Settings",
    ) else {
        return;
    };
    let changes = proxy.receive_signal_with_args("SettingChanged", &[(0, NAMESPACE), (1, KEY)]);
    let publish = |value: OwnedValue| {
        let Some(layout) = String::try_from(value)
            .ok()
            .and_then(|value| gnome_layout(&value))
        else {
            return true;
        };
        !matches!(
            events.try_send(DesktopEvent::WindowControls(layout)),
            Err(crossbeam_channel::TrySendError::Disconnected(_))
        )
    };
    if let Ok(value) = proxy.call("ReadOne", &(NAMESPACE, KEY)) {
        if !publish(value) {
            return;
        }
    } else if let Ok(output) = Command::new("gsettings")
        .args(["get", NAMESPACE, KEY])
        .output()
        && output.status.success()
        && let Ok(value) = std::str::from_utf8(&output.stdout)
        && let Some(layout) = gnome_layout(value.trim().trim_matches('\''))
    {
        let _ = events.try_send(DesktopEvent::WindowControls(layout));
    }
    if let Ok(changes) = changes {
        for change in changes {
            if let Ok((_, _, value)) = change.body().deserialize::<(&str, &str, OwnedValue)>()
                && !publish(value)
            {
                break;
            }
        }
    }
}

fn gnome_layout(value: &str) -> Option<WindowControls> {
    let (left, right) = value.split_once(':')?;
    Some(layout(
        CaptionStyle::Adwaita,
        left.split(',').filter_map(gnome_button),
        right.split(',').filter_map(gnome_button),
    ))
}

fn gnome_button(value: &str) -> Option<CaptionButton> {
    match value.trim() {
        "minimize" => Some(CaptionButton::Minimize),
        "maximize" => Some(CaptionButton::Maximize),
        "close" => Some(CaptionButton::Close),
        _ => None,
    }
}

fn kde_controls() -> WindowControls {
    let read_config = |directory: &Path| std::fs::read_to_string(directory.join("kwinrc")).ok();
    if std::env::var_os("FLATPAK_ID").is_some() {
        // The manifest exposes only this file. XDG_CONFIG_HOME is app-private.
        let config = dirs::home_dir().and_then(|home| read_config(&home.join(".config")));
        return kde_config_layout(config.as_deref());
    }

    for reader in ["kreadconfig6", "kreadconfig5"] {
        if let Some(left) = kde_read_entry(reader, KDE_LEFT, "")
            && let Some(right) = kde_read_entry(reader, KDE_RIGHT, KDE_DEFAULT_RIGHT)
        {
            return kde_layout(&left, &right);
        }
    }

    let system_dirs = std::env::var_os("XDG_CONFIG_DIRS")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "/etc/xdg".into());
    let configs: Vec<_> = std::env::split_paths(&system_dirs)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .chain(dirs::config_dir())
        .filter(|directory| directory.is_absolute())
        .filter_map(|directory| read_config(&directory))
        .collect();
    kde_config_layout(configs.iter().map(String::as_str))
}

fn kde_read_entry(reader: &str, key: &str, default: &str) -> Option<String> {
    let output = Command::new(reader)
        .args([
            "--file",
            "kwinrc",
            "--group",
            KDE_GROUP,
            "--key",
            key,
            "--default",
            default,
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

fn kde_config_layout<'a>(configs: impl IntoIterator<Item = &'a str>) -> WindowControls {
    let mut left = "";
    let mut right = KDE_DEFAULT_RIGHT;
    for config in configs {
        let mut decoration = false;
        for line in config.lines().map(str::trim) {
            if line.starts_with('[') {
                decoration = line
                    .strip_prefix('[')
                    .and_then(|line| line.strip_suffix(']'))
                    == Some(KDE_GROUP);
            } else if decoration && let Some((key, value)) = line.split_once('=') {
                match key.trim() {
                    KDE_LEFT => left = value.trim(),
                    KDE_RIGHT => right = value.trim(),
                    _ => {}
                }
            }
        }
    }
    kde_layout(left, right)
}

fn kde_layout(left: &str, right: &str) -> WindowControls {
    layout(
        CaptionStyle::Breeze,
        left.chars().filter_map(kde_button),
        right.chars().filter_map(kde_button),
    )
}

fn kde_button(value: char) -> Option<CaptionButton> {
    match value {
        'I' => Some(CaptionButton::Minimize),
        'A' => Some(CaptionButton::Maximize),
        'X' => Some(CaptionButton::Close),
        _ => None,
    }
}

fn layout(
    style: CaptionStyle,
    left: impl Iterator<Item = CaptionButton>,
    right: impl Iterator<Item = CaptionButton>,
) -> WindowControls {
    let mut seen = Vec::new();
    let mut unique = |button: &CaptionButton| {
        if seen.contains(button) {
            return false;
        }
        seen.push(*button);
        true
    };
    WindowControls {
        style,
        left: left.filter(&mut unique).collect(),
        right: right.filter(unique).collect(),
    }
}

#[cfg(test)]
#[path = "tests/window_controls.rs"]
mod tests;
