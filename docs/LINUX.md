# Soliton on Linux

Library, local playback, GPUI Wayland/X11 interface, desktop media controls
(MPRIS), per-user archive, RPM, .deb and Flatpak. Windows keeps its GPUI
backend, WASAPI and native integrations. Development and validation
environment: Fedora Workstation (GNOME, Wayland, PipeWire).

## Behavior

- **Window**: client-side decorations; left-click drag from the title bar and
  logo, double-click to maximize/restore, minimize, close, and resize through a
  transparent margin around the window that carries the drop shadow and the
  resize cursor of free edges. Size and maximized state are restored at startup
  (GPUI does not apply the maximized state at open on Linux). On Wayland the
  compositor picks the position and GPUI knows no screen before opening, so the
  saved size is restored without a screen check. The identifier
  `io.github.Carlton2001.Soliton` is shared by the window, desktop entry, icon
  and MPRIS.
- **Audio**: Symphonia → Rubato → native PipeWire stream (`pipewire` 0.10, no
  CPAL or ALSA on Linux), 32-bit float, mono or stereo at the track rate;
  PipeWire converts to the output format. Outputs are PipeWire sinks (node name
  persisted, description shown, Bluetooth icon for `bluez5` devices), tracked
  live by the registry: a paired headset appears at once, without polling.
  - "System default device" follows the output chosen in Sound settings
    immediately. If that output fails (Bluetooth headset switched off), the
    stream reopens on the new default without pausing. WirePlumber still pauses
    players whose output disappears, through MPRIS (`linking.pause-playback`,
    on by default, Firefox included); Soliton respects it.
    `wpctl settings --save linking.pause-playback false` disables it for all
    applications.
  - A chosen output is targeted with `target.object`, `node.dont-reconnect` and
    `node.dont-fallback`: PipeWire never replaces it. Losing it pauses playback
    with a warning until resume; resume plays on the default output, then
    Soliton returns to the chosen output as soon as it reappears.
  - The displayed format names the output actually linked to the stream.
    PulseAudio alone (without PipeWire) is not supported.
- **MPRIS** (`org.mpris.MediaPlayer2.soliton`, `.instance<pid>` suffix for a
  second instance): title, artist, album, duration, artwork from the Soliton
  cache, status, position and `Seeked`; play, pause, toggle, stop, previous,
  next, `Seek`, `SetPosition`, `Raise` and `Quit`. GNOME media keys and media
  panel go through it. No URI or file outside the library is accepted. Without
  a session bus, playback continues and a warning is shown.
- **GPU**: at launch, Vulkan drivers of cards with no connected screen
  (`/sys/class/drm/card*-*/status`) are excluded through
  `VK_LOADER_DRIVERS_DISABLE` (NVIDIA, AMD or Intel by vendor), so a sleeping
  card is not woken at startup, kept awake, or woken at exit (about 2 s
  observed). Nothing changes if no screen is detected or if the user or desktop
  already selects a GPU (`VK_LOADER_DRIVERS_SELECT`, `VK_LOADER_DRIVERS_DISABLE`,
  `VK_DRIVER_FILES`, `VK_ICD_FILENAMES`, `MESA_VK_DEVICE_SELECT`, `DRI_PRIME`,
  `__NV_PRIME_RENDER_OFFLOAD`, the last one set by GNOME's "Launch using
  Discrete Graphics Card").
- **System accent**: read from the XDG Settings portal
  (`org.freedesktop.appearance`, `accent-color`, GNOME 47+ and KDE Plasma,
  Flatpak included) on the integration thread, every second. Light and dark
  variants keep the hue and cap OKLab lightness at 0.55 (light theme) or raise
  it to 0.72 (dark theme) to stay legible under black or white labels. Without
  a published accent, blue applies.
- **Closing**: GNOME has no notification area, so closing always quits through
  the normal cleanup, without hiding. For the same reason no autostart is
  offered (the window would open at every login), and the desktop integration
  section of the settings is absent.
- **Icon**: when installed by `install.sh` (marker `X-Soliton-Install=user` in
  its desktop entry), the chosen palette replaces
  `~/.local/share/icons/hicolor/scalable/apps/io.github.Carlton2001.Soliton.svg`.
  An entry or icon installed otherwise is never modified.
- **Not available on Linux**: autostart, built-in updates, native audio output
  notifications, exclusive audio. Google Cast is validated; DLNA is built but
  not validated on Linux.
- **Watching**: inotify on every source folder. If the system limit
  (`fs.inotify.max_user_watches`) prevents watching a whole source, its partial
  watches are removed: it is covered only by scans (startup, every 30 min,
  refresh), a warning is logged, and watching is retried after each scan.
  Reaching the limit later (new folder) triggers a full scan.
- **Sources**: an unmounted folder keeps its albums, marked unavailable and
  hidden by default; once remounted it comes back within seconds without a
  refresh (offline sources probed every 5 s).
- Library, settings and caches live in `~/.local/share/Soliton`
  (`SOLITON_DATA_DIR` isolates a test run), never in music sources.

## Install from the archive

```sh
tar -xzf Soliton-0.1.0-x86_64-linux.tar.gz
cd Soliton-0.1.0-x86_64-linux
./install.sh
```

Per-user install, without root: `~/.local/bin/soliton`, desktop entry, AppStream
metadata, icon and licenses under `~/.local/share`. Running `install.sh` from a
newer archive updates. `./uninstall.sh` removes these files, keeps
`~/.local/share/Soliton`, and refuses to touch a desktop entry it did not install.

Runtime requirements: Vulkan, a Wayland or X11 session, D-Bus and the XDG
portal (folder picker), PipeWire 0.3.49 or later. The binary links
`libpipewire-0.3`, `libxcb` and `libxkbcommon` (Vulkan, Wayland and Fontconfig
are loaded at runtime); built on Fedora 44, it needs that glibc or newer.

## Prepare Fedora for development

```sh
sudo dnf install gcc gcc-c++ make cmake pkgconf-pkg-config \
  pipewire-devel clang-devel libxkbcommon-devel libxkbcommon-x11-devel \
  freetype-devel fontconfig-devel libxcb-devel wayland-devel rustup
rustup-init -y --default-toolchain 1.98.1 --profile minimal --component rustfmt
. "$HOME/.cargo/env"
```

With rustup already present: `rustup toolchain install 1.98.1 --profile minimal
--component rustfmt`. `clang-devel` generates the PipeWire bindings (`bindgen`).

## Build, test, run

```sh
cargo +1.98.1 fmt --check
cargo +1.98.1 check --workspace --all-targets --locked
cargo +1.98.1 test --workspace --locked
export SOLITON_DATA_DIR="$HOME/.local/share/Soliton-dev"
RUST_LOG=info cargo +1.98.1 run -p soliton-app --bin Soliton --locked
```

`WAYLAND_DISPLAY= cargo run …` forces X11 (XWayland). `Soliton-index` checks
indexing without a window:

```sh
cargo +1.98.1 run -p soliton-app --bin Soliton-index --locked -- \
  "$SOLITON_DATA_DIR" /path/to/a/test-collection
```

Explicit tests in a desktop session (zero volume for audio):

```sh
cargo +1.98.1 test -p soliton-audio native_output -- --ignored
cargo +1.98.1 test -p soliton-audio unavailable_preference -- --ignored
cargo +1.98.1 test -p soliton-audio chosen_sink -- --ignored
cargo +1.98.1 test -p soliton-audio engine_pauses -- --ignored
cargo +1.98.1 test -p soliton-audio default_following -- --ignored
cargo +1.98.1 test -p soliton-audio default_output_loss -- --ignored
cargo +1.98.1 test -p soliton-platform session_bus -- --ignored
cargo +1.98.1 test -p soliton-library watch_limit -- --ignored
cargo +1.98.1 test -p soliton-app remounted_source -- --ignored
```

- `chosen_sink`, `engine_pauses` and `default_following` create a temporary
  muted PipeWire sink. `default_following` briefly makes it the system default
  (`pw-metadata`, package `pipewire-utils`): other applications move to it for
  about a second, then the configured output is restored, even on failure.
- `watch_limit` reruns itself in a user namespace (`unshare`) with a lowered
  inotify limit, leaving the system limit untouched.
- `remounted_source` renames then restores a temporary source folder and waits
  for its return without a manual scan (about 5 s).

## Build the archive

```sh
RUSTUP_TOOLCHAIN=1.98.1 scripts/build-linux.sh
```

The script checks the license inventory (`scripts/check-third-party-licenses.sh`,
same hashes as the PowerShell version), requires a `<release>` entry for the
current version in the AppStream metadata, builds in release, assembles
`target/distribution/output/Soliton-<version>-x86_64-linux.tar.gz` (binary,
desktop entry, AppStream metadata, icon, `LICENSE`, `licenses/dependencies`,
`install.sh`, `uninstall.sh`, bilingual `README.txt`), checks its content (stripped binary) and writes
its SHA-256. `--skip-build` reuses the existing release binary. Sources for the
desktop entry, metadata and scripts are in `installer/linux/`.

After a dependency change, regenerate the notices first: see
[licenses/dependencies](../licenses/dependencies/README.md).

## Build the RPM

```sh
RUSTUP_TOOLCHAIN=1.98.1 scripts/build-rpm.sh
```

Requires `sudo dnf install rpm-build desktop-file-utils appstream`. The script
builds the archive first (passing `--skip-build`), then
`target/distribution/output/soliton-<version>-1.fc44.x86_64.rpm` and its SHA-256
from `installer/linux/rpm/soliton.spec`. The binary is the archive's, built on
Fedora 44: the package targets Fedora 44 and later.

- Installs binary, desktop entry, AppStream metadata and icon under `/usr`,
  licenses under `/usr/share/licenses/soliton`. The system entry has no
  `install.sh` marker, so its icon keeps the default palette.
- Data stays in `~/.local/share/Soliton`, shared with the archive and kept by
  `dnf remove`. If the archive is also installed, its entry in
  `~/.local/share/applications` hides the package's.
- Both entries declare `StartupNotify=false`: GPUI does not consume the startup
  token (`XDG_ACTIVATION_TOKEN`, `DESKTOP_STARTUP_ID`), and GNOME would show the
  busy cursor until its timeout.
- Declares `vulkan-loader`, `mesa-vulkan-drivers`, `libwayland-client` (loaded at
  runtime) and `default-fonts-core-sans` (characters missing from the embedded
  Inter and JetBrains Mono); `rpmbuild` detects the other libraries.
- `License` is computed by `scripts/rpm-license.py` from the crates linked on
  Linux (`cargo metadata`, build and dev dependencies excluded) and the embedded
  assets (Inter, Lucide). The `%changelog` date is the last commit's; release
  notes stay in the AppStream metadata.

## Build the .deb

```sh
scripts/build-deb.sh
```

The binary is built on Ubuntu 24.04 (glibc 2.39): the package targets Ubuntu
24.04 and later, Linux Mint 22 and Debian 13. Outside Ubuntu 24.04 the script
reruns itself in a `podman` container built from
`installer/linux/deb/Containerfile` (Rust 1.98.1, Debian tools), with its own
`target/ubuntu-24.04` folder and a `soliton-ubuntu-cargo` volume for the crate
cache. It produces `target/distribution/output/soliton_<version>-1_amd64.deb`
and its SHA-256, then checks it with `lintian`.

Layout and data as for the RPM; licenses in `/usr/share/doc/soliton`
(Debian-format `copyright`, crate notices). `dpkg-shlibdeps` detects linked
libraries; also declared: `libvulkan1`, `mesa-vulkan-drivers`,
`libwayland-client0` (loaded at runtime), `pipewire-bin` (provides
`client.conf`, without which the PipeWire client fails) and a system font
(`fonts-dejavu-core` or equivalent). `Maintainer` is `carlton2001@gmail.com`,
or `SOLITON_MAINTAINER`.

## Build the Flatpak

```sh
scripts/build-flatpak.sh
```

Requires `flatpak-builder` (Fedora package) and, for `flatpak-builder-lint`,
`flatpak install --user flathub org.flatpak.Builder`; the script installs the
Freedesktop 26.08 SDK and its `rust-stable` and `llvm22` extensions (libclang
for the PipeWire bindings). The build follows
`installer/linux/flatpak/io.github.Carlton2001.Soliton.yml`, offline, from the
last commit of `main`: `scripts/flatpak-cargo-sources.py` regenerates
`cargo-sources.json` from `Cargo.lock` (crates.io archives checked by hash,
`flatpak-cargo-generator` format). Output:
`target/distribution/output/Soliton-<version>-x86_64.flatpak`, checked by
`flatpak-builder-lint`.

For Flathub, `scripts/flathub-manifest.sh vX.Y.Z` writes to `target/flathub` a
copy of the manifest whose source is the GitHub repository at that tag and
commit, with `cargo-sources.json`; the release attaches them too. The author
opens the submission (pull request on `flathub/flathub`) personally: Flathub
refuses AI-made submissions. Screenshot URLs in the metainfo point to the tag;
the version bump scripts update them.

Minimal permissions: Wayland (X11 fallback), `dri` for Vulkan, PipeWire socket
only, network for Cast and DLNA, MPRIS name. Read-only files: Music folder,
`/run/media`, `/media` and `/mnt`. The portal always returns a document portal
path; when the folder is directly readable, Soliton replaces it with its real
path (`GetHostPaths`), which restores watching and availability. Any other
folder goes through the document portal: indexed and played, but additions are
seen only at the periodic scan (30 min) or restart, and unavailability is not
detected. To avoid it:
`flatpak override --user --filesystem=home:ro io.github.Carlton2001.Soliton`.
Data lives in `~/.var/app/io.github.Carlton2001.Soliton/data/Soliton`, separate
from native installs.

## Validation

Real hardware, 28 and 29 October 2026: Fedora 44 Workstation, GNOME Wayland,
Intel UHD Raptor Lake (Mesa), PipeWire 1.6.10, Rust 1.98.1.

| Check | Result |
| --- | --- |
| Format, `cargo check` and `cargo clippy` of the workspace without warnings | Passed |
| Workspace test suite | Passed, 11 native tests ignored by default |
| Native tests: play/pause/seek/next/stop and explicit selection on PipeWire, unavailable preference without fallback, MPRIS | Passed |
| Native test: temporary chosen sink linked then removed; failure reported without moving to another output | Passed |
| Native engine test: pause on chosen sink loss, resume on default, return to recreated sink | Passed |
| Native engine test: default output loss, session reopened on the new one without pause or warning, position kept | Passed |
| Native test: playback without chosen output moved to a new system default and back, without pause or fallback, displayed output follows | Passed |
| `soliton-audio` and `soliton-library` for `x86_64-pc-windows-msvc` (`cargo xwin check`, tests included) | Passed |
| Size and maximized state restored on Wayland and X11 | Passed |
| Binary links `libpipewire-0.3`, no longer `libasound` (`ldd`) | Verified |
| Accelerated GPUI rendering: Blade on "Intel(R) Graphics (RPL-S)", MAILBOX presentation, no warning | Verified |
| Wayland and X11 window: rendering, 2× scale, fonts | Passed |
| Title bar interactions (drag, double-click, buttons) and resize on Wayland | Passed (manual) |
| GNOME media panel and media keys | Passed (manual) |
| Portal folder picker when adding a source | Passed (manual) |
| Launcher icon following the palette (`install.sh` copy) | Passed (manual) |
| Grid columns adjusted on resize, resize margin included | Passed (manual) |
| Bluetooth headset: chosen in Soliton, switched off (pause, warning, resume on default) and back | Passed (manual) |
| Default output: headset on, playback follows it; headset off, session reopened on the new default, then pause requested by WirePlumber (`linking.pause-playback`, seen with `dbus-monitor`) | Passed (manual); system pause respected |
| Source unmounted: albums unavailable; remounted: back without refresh | Passed |
| inotify limit reached (user namespace, limit 8): partial watches removed, quota given to another source | Passed |
| Real collection of 7,206 tracks on NTFS (`ntfs-3g`, 99 folders): 372 inotify watches for a 557,977 limit | Verified |
| Indexing: FLAC 16/44.1 and 24/96 grouped as two editions, multi-disc MP3, WAV compilation, artwork | Passed |
| Google Cast: discovery and playback | Passed (manual) |
| Notices from `update-third-party-licenses.sh` byte-identical to the Windows output for the same lock | Verified |
| `build-linux.sh`: license check, release build, archive and hash | Passed |
| Archive: install in an isolated `HOME`, valid desktop entry, launch, palette icon, uninstall keeping data, foreign entry refused | Passed |
| `appstreamcli validate` and `desktop-file-validate` | Valid; pedantic note on identifier capitals |

RPM, `fedora:44` minimal container and development machine:

| Check | Result |
| --- | --- |
| `dnf install`: dependencies resolved, no missing library (`ldd`) | Passed |
| `rpmlint` 2.8.0 | No error; missing man page and documentation reported |
| `dnf remove`: files removed, `~/.local/share/Soliton` kept | Passed |
| Installed on the development machine: launcher, icon, window tied to the dock, media panel, existing library found | Passed (manual); expected `dnf` warning for an unsigned local package |
| Launch from the app grid with `StartupNotify=false`: no busy cursor, window in front | Passed (manual) |
| Digits in embedded JetBrains Mono (numbers, durations, player, sliders), tooltips in Inter: no clipped digit | Passed (manual) |

.deb, `ubuntu:24.04`, `debian:13` and `ubuntu:26.04` minimal containers:

| Check | Result |
| --- | --- |
| Release build on Ubuntu 24.04 (PipeWire 1.0) | Passed; binary limited to glibc 2.39 |
| `apt install`: dependencies resolved, no missing library (`ldd`), Vulkan and Wayland present | Passed on all three |
| `lintian` | No error; missing man page and closed bug reported |
| Clean Ubuntu 24.04 toolbox (Ubuntu binary checked via `/proc/<pid>/root`), no system font: rendering, audio, media panel, library found | Passed (manual); Ubuntu desktop integration (launcher, dock) not verified |
| `apt purge`: files removed, `~/.local/share/Soliton` kept | Passed on all three |

Flatpak on the development machine, current permissions:

| Check | Result |
| --- | --- |
| Rendering, fonts, audio, media panel and artwork, launcher | Passed (manual); fixed icon, as for the RPM |
| Source on a mounted disk added through the portal (`/run/media/…/_FLAC`) | Passed (manual): stored under its real path, not `/run/flatpak/doc` |
| Bluetooth headset chosen in Soliton and playback | Passed (manual) |
