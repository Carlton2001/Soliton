# Soliton on Linux

Soliton uses Vulkan on Wayland and X11, PipeWire for playback, and MPRIS for
desktop media controls. Archive installations can update from Soliton. Use the
package manager to update the RPM and .deb packages.

Soliton's title bar uses Adwaita buttons on GNOME and Breeze buttons on KDE
Plasma. It follows the desktop's choice of minimize, maximize and close buttons,
including their order and placement on either side. GNOME changes apply as the
Settings portal reports them. On Plasma, restart Soliton after changing **System
Settings → Appearance → Window Decorations → Titlebar Buttons**. The Flatpak reads
the default Plasma configuration file at `~/.config/kwinrc` with read-only access.
When the desktop layout is unavailable, Soliton puts all three buttons on the right.

## Install from GitHub

This is the recommended installation method on Linux x86_64. It installs
Soliton for your user and supports in-app updates.

Install `curl` and `minisign` with your package manager, then run:

```sh
curl -fsSL https://github.com/Carlton2001/Soliton/releases/latest/download/install.sh | sh
```

The script downloads the Linux x86_64 archive for its release, verifies its
Minisign signature and opens the archive installer. Run it as your regular user.
For a specific release, download `install.sh` from that release's assets.

## Install from the archive

Extract the downloaded archive and run `./install.sh` from its directory. For
later versions, open **Settings → About**, check for updates and click **Update
Soliton**. The sidebar also shows when an update is available. Soliton verifies
the archive signature, installs it without `sudo`, saves the session and
restarts. The archive installer uses `flock` from util-linux. The in-app updater
also uses `curl`, CA certificates and `tar` with gzip support.

Run `./install.sh` from a newer archive to bring an older installation without
the updater up to date. The terminal menu also offers reinstall and uninstall.
For direct execution, use `./install.sh --install` or `./install.sh --uninstall`.
`./uninstall.sh` also removes the per-user installation.

The archive is built on Fedora 44 x86_64 and requires a compatible glibc,
Vulkan, a Wayland or X11 session, PipeWire 0.3.49 or newer, D-Bus and the
XDG folder picker portal. The archive installer lists missing required and
recommended components and prints the package manager command for Debian and
Ubuntu, Fedora (including rpm-ostree systems), Arch Linux, and openSUSE. In a
terminal, it can run the command after confirmation. If `flock` is still
missing, install it and run the script again. In-app updates skip the system
component checks and require `flock` to be present.

The binary is installed in `~/.local/bin`. The desktop entry, AppStream
metadata, icon and licenses are installed under
`${XDG_DATA_HOME:-$HOME/.local/share}`.

Uninstall keeps the library database, playlists, settings and cache by default.
The complete uninstall option removes those files from
`${XDG_DATA_HOME:-$HOME/.local/share}/Soliton`, a directory shared with the RPM
and .deb. Other files and separate `SOLITON_DATA_DIR` directories are kept.
Choose the purge option in the menu or run `./uninstall.sh --purge`.

## Behavior

- **Window and settings:** Soliton restores window size and maximized state.
  The Wayland compositor chooses the window position. Soliton follows the system
  accent when the desktop exposes one through the XDG Settings portal. Below
  100% background opacity, Soliton requests backdrop blur over Wayland's
  background-effect protocol (supported by GNOME 51) or the older KDE protocol.
  Without compositor blur, including under X11, the backgrounds are opaque and
  Settings hides the opacity control.
- **Graphics:** On hybrid systems Soliton prefers the Wayland compositor's GPU
  vendor when it can identify it. A filtered Vulkan startup failure triggers
  one retry with all drivers available. Explicit device overrides take priority.
  See the [GPU selection implementation](../crates/soliton-platform/src/linux/gpu.rs).
- **Audio:** PipeWire sinks, including Bluetooth devices, appear in the output
  picker as they become available. The displayed format names the output linked
  to the stream. WirePlumber may send a pause over MPRIS when an output disappears.
  Soliton respects that system request. See the
  [output behavior](SOLITON_SPECIFICATIONS.md#playback-and-collections).
- **Desktop:** Media keys and controls use MPRIS. Closing quits the app. The
  launcher icon follows the chosen palette for archive installs. Packages keep
  their installed icon.
- **Library:** When the inotify limit prevents watching an entire source,
  Soliton uses startup, periodic and manual scans for that source and retries
  watching after each scan.

## Prepare Fedora for development

```sh
sudo dnf install gcc gcc-c++ make cmake pkgconf-pkg-config \
  pipewire-devel clang-devel libxkbcommon-devel libxkbcommon-x11-devel \
  freetype-devel fontconfig-devel libxcb-devel wayland-devel rustup
rustup default stable
rustup component add rustfmt clippy
```

Use a Rust toolchain that meets the workspace minimum. `clang-devel` provides
libclang for generating the PipeWire bindings. See the
[development guide](DEVELOPMENT.md#run) for the build command.

## Build, test, run

Run the [workspace checks](DEVELOPMENT.md#check). Set `WAYLAND_DISPLAY=` when
running Soliton to test X11 in a Wayland session.

Run these tests in a desktop session at zero volume.

```sh
cargo test -p soliton-audio --locked native_output -- --ignored
cargo test -p soliton-audio --locked unavailable_preference -- --ignored
cargo test -p soliton-audio --locked chosen_sink -- --ignored
cargo test -p soliton-audio --locked engine_pauses -- --ignored
cargo test -p soliton-audio --locked default_following -- --ignored
cargo test -p soliton-audio --locked default_output_loss -- --ignored
cargo test -p soliton-platform --locked session_bus -- --ignored
cargo test -p soliton-library --locked watch_limit -- --ignored
cargo test -p soliton-app --locked remounted_source -- --ignored
```

`chosen_sink`, `engine_pauses` and `default_following` create temporary muted
PipeWire sinks. `default_following` uses `pw-metadata` from `pipewire-utils` to
change the system default briefly, which can reroute other apps. It restores
the previous output after the test. `watch_limit` requires an unprivileged user
namespace.

## Build the archive

```sh
scripts/build-linux.sh
```

Build on Fedora 44 for the same baseline as the release archive. The script
checks licenses and the AppStream release entry, then writes
`Soliton-<version>-x86_64-linux.tar.gz` and its checksum under
`target/distribution/output/`. Use `--skip-build` to repackage the existing
release binary.

## Build the RPM

```sh
sudo dnf install rpm-build desktop-file-utils appstream
scripts/build-rpm.sh
```

The RPM uses the Fedora 44 archive binary and targets Fedora 44 and later. It
installs under `/usr`. `dnf remove` keeps user data. When both the RPM and the
archive are installed, the archive's per-user desktop entry takes precedence.

## Build the .deb

```sh
scripts/build-deb.sh
```

The script targets Ubuntu 24.04 and later, Linux Mint 22 and Debian 13. On other
build hosts it uses a Podman container based on Ubuntu 24.04. It checks the
package with `lintian`. Set `SOLITON_MAINTAINER` to override the default
maintainer in the package metadata.

## Build the Flatpak

Flatpak builds and distribution are on hold. The commands below are kept for
when work resumes. See the [Flathub guide](FLATHUB.md) for the release workflow.

```sh
flatpak install --user flathub org.flatpak.Builder
scripts/build-flatpak.sh
```

Install the native `flatpak-builder` package as well. The script installs the
SDK and extensions declared by the
[manifest](../installer/linux/flatpak/io.github.Carlton2001.Soliton.yml), builds
the committed `main` sources with offline Cargo inputs and the Rust toolchain
pinned in the manifest, runs Flatpak lints and exports a bundle. Update the
Rust archives and their SHA-256 checksums when the release toolchain changes.

Flatpak grants read-only access to the Music folder and standard mount points
(`/run/media`, `/media`, `/mnt`). Other locations use the folder picker portal.
For portal-only folders, Soliton detects additions on a periodic scan or
restart and cannot detect source unavailability. This override grants
read-only access to folders under your home directory.

```sh
flatpak override --user --filesystem=home:ro io.github.Carlton2001.Soliton
```

Flatpak data lives in `~/.var/app/io.github.Carlton2001.Soliton/data/Soliton`.

## Validation

Manual validation on 28 and 29 October 2026 used Fedora 44 Workstation, GNOME
Wayland, Intel UHD Raptor Lake, PipeWire 1.6.10 and Rust 1.98.1.

- Checked Wayland and X11 rendering, window restoration, MPRIS, the folder
  picker, and PipeWire output changes including Bluetooth disconnect and return.
- Indexed a real collection of 7,206 tracks on NTFS. Checked album editions,
  compilations, source remount and recovery from the inotify watch limit.
- Tested Google Cast discovery and playback. The physical DLNA case remains a
  [manual test scenario](DEVELOPMENT.md#manual-hardware-scenarios).
- Installed and removed the archive and RPM on Fedora. Checked .deb installation
  and purge in Ubuntu 24.04, Ubuntu 26.04 and Debian 13 containers. Ubuntu desktop
  integration was outside that run.
- Tested Flatpak launch, playback, media controls, artwork, folder selection on
  a mounted disk and Bluetooth output.
