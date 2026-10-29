//! Read the desktop accent color from the XDG Settings portal, never on the UI
//! thread: once at start, then on each `SettingChanged` signal for it. GNOME
//! 47+ and KDE Plasma publish it; the portal is also the only source reachable
//! from the Flatpak sandbox.
use crate::DesktopEvent;
use crossbeam_channel::Sender;
use zbus::{
    blocking::{Connection, Proxy},
    zvariant::OwnedValue,
};

const PORTAL: &str = "org.freedesktop.portal.Desktop";
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
const SETTINGS: &str = "org.freedesktop.portal.Settings";
const NAMESPACE: &str = "org.freedesktop.appearance";
const KEY: &str = "accent-color";

/// Sends the current accent, then each change, from a thread of its own that
/// sleeps on the bus. It ends with the application, or once nobody listens.
pub(super) fn watch_accent(events: Sender<DesktopEvent>) {
    let Ok(connection) = Connection::session() else {
        return;
    };
    let warnings = events.clone();
    let spawned = std::thread::Builder::new()
        .name("soliton-accent".into())
        .spawn(move || {
            let mut last = None;
            let mut send = |connection: &Connection| {
                let Ok(Some(color)) = read(connection) else {
                    return true;
                };
                let colors = variants(color);
                if last == Some(colors) {
                    return true;
                }
                last = Some(colors);
                !matches!(
                    events.try_send(DesktopEvent::SystemAccent(colors)),
                    Err(crossbeam_channel::TrySendError::Disconnected(_))
                )
            };
            if !send(&connection) {
                return;
            }
            let Ok(proxy) = Proxy::new(&connection, PORTAL, PORTAL_PATH, SETTINGS) else {
                return;
            };
            let Ok(changes) =
                proxy.receive_signal_with_args("SettingChanged", &[(0, NAMESPACE), (1, KEY)])
            else {
                return;
            };
            for _ in changes {
                if !send(&connection) {
                    return;
                }
            }
        });
    if let Err(error) = spawned {
        let _ = warnings.try_send(DesktopEvent::Warning(format!("Accent color: {error}")));
    }
}

/// `None` when the desktop sets no accent: the portal then reports
/// components outside 0–1.
fn read(connection: &Connection) -> zbus::Result<Option<[f64; 3]>> {
    let reply = connection.call_method(
        Some(PORTAL),
        PORTAL_PATH,
        Some(SETTINGS),
        "ReadOne",
        &(NAMESPACE, KEY),
    )?;
    let value: OwnedValue = reply.body().deserialize()?;
    let (r, g, b) = <(f64, f64, f64)>::try_from(value)?;
    let color = [r, g, b];
    Ok(color
        .iter()
        .all(|c| (0.0..=1.0).contains(c))
        .then_some(color))
}

/// Light and dark theme variants of a desktop accent, readable as text and
/// under the black or white labels Soliton draws on accent fills: OKLab
/// lightness is capped at 0.55 for the light theme and raised to 0.72 for the
/// dark theme, hue kept and chroma reduced only to stay within sRGB.
pub(super) fn variants(srgb: [f64; 3]) -> [u32; 2] {
    let [l, a, b] = oklab(srgb.map(to_linear));
    let pack = |l: f64| {
        let at = |k: f64| from_oklab([l, a * k, b * k]);
        let inside = |rgb: [f64; 3]| rgb.iter().all(|c| (-1e-6..=1.0 + 1e-6).contains(c));
        let (mut low, mut high) = (0.0, 1.0);
        if !inside(at(high)) {
            for _ in 0..24 {
                let mid = (low + high) / 2.0;
                if inside(at(mid)) {
                    low = mid;
                } else {
                    high = mid;
                }
            }
            high = low;
        }
        let [r, g, b] = at(high).map(|c| (from_linear(c) * 255.0).round() as u32);
        (r << 16) | (g << 8) | b
    };
    [pack(l.min(0.55)), pack(l.max(0.72))]
}

fn to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn from_linear(c: f64) -> f64 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

fn oklab([r, g, b]: [f64; 3]) -> [f64; 3] {
    let l = (0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b).cbrt();
    let m = (0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b).cbrt();
    let s = (0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b).cbrt();
    [
        0.210_454_255_3 * l + 0.793_617_785_0 * m - 0.004_072_046_8 * s,
        1.977_998_495_1 * l - 2.428_592_205_0 * m + 0.450_593_709_9 * s,
        0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766_0 * s,
    ]
}

fn from_oklab([l, a, b]: [f64; 3]) -> [f64; 3] {
    let l_ = (l + 0.396_337_777_4 * a + 0.215_803_757_3 * b).powi(3);
    let m_ = (l - 0.105_561_345_8 * a - 0.063_854_172_8 * b).powi(3);
    let s_ = (l - 0.089_484_177_5 * a - 1.291_485_548_0 * b).powi(3);
    [
        4.076_741_662_1 * l_ - 3.307_711_591_3 * m_ + 0.230_969_929_2 * s_,
        -1.268_438_004_6 * l_ + 2.609_757_401_1 * m_ - 0.341_319_396_5 * s_,
        -0.004_196_086_3 * l_ - 0.703_418_614_7 * m_ + 1.707_614_701_0 * s_,
    ]
}

#[cfg(test)]
#[path = "tests/appearance.rs"]
mod tests;
