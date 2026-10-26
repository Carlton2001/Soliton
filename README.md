# Soliton

<p align="center">
  <img src="screenshots/hero.webp" alt="Soliton music library interface" width="100%" />
</p>

Soliton is a native music player built in **Rust** with **GPUI**. It brings your local music collection into a responsive desktop interface, with rich metadata, album artwork, multiple editions, and playback on local or network devices.

Windows is the primary build and validation target. Windows distribution is prepared, with package and hardware validation still pending. Soliton also runs on Linux (Fedora Workstation).

On Linux, Soliton plays through the system sound output (PipeWire or PulseAudio), integrates with the desktop media controls and keys through MPRIS, can start at sign-in, and installs per user from a release archive. See the [Linux guide](docs/LINUX.md) for installation, building, and remaining hardware tests.

## Your Music, Kept Local

Browse music from multiple folders, external drives, or mounted network shares in one library. Soliton reads metadata and artwork from your files and stores its database and caches separately. **Your music files, tags, and embedded artwork remain untouched.**

Background indexing and filesystem monitoring keep the library up to date. When a source goes offline, its albums stay in your library and are marked unavailable until it returns.

## Key Features

- **Album and Artist Browsing**: Explore grid or list views, open album details, and browse compilations under Various Artists.
- **Multiple Editions**: Group matching album editions and put the best available edition first.
- **Search and Filters**: Search regardless of case or accents. Filter by genre, period, artist, format, or quality, and sort by artist, title, year, or recent additions.
- **Playback and Queue**: Use a persistent player bar, Now Playing, fullscreen mode, shuffle, repeat, manual playlists, and an editable queue.
- **Local and Network Outputs**: Choose a local audio device, use Bluetooth through Windows, or play to DLNA / UPnP and Google Cast receivers. Cast has been validated on NVIDIA SHIELD; DLNA has only been tested with a simulated receiver.
- **Windows Integration**: Use media keys, system media controls, taskbar thumbnail controls, and the notification area. Configure closing behavior and automatic startup.
- **Personalization**: Choose English or French, system/light/dark appearance, accent color, artwork size, and the application icon.

Tested formats include **WAV, FLAC, MP3, AAC-LC/M4A, ALAC, Vorbis, and AIFF**. Local playback uses shared audio output; exclusive mode, bit-perfect playback, and an equalizer are not currently available. See the [development guide](docs/DEVELOPMENT.md#limites) for playback and network limitations.

## Technology Stack

- **Application and UI**: Rust and [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui), with native GPU-accelerated rendering.
- **Audio**: Symphonia for decoding, CPAL for local output, and an audio engine independent of the interface.
- **Library**: SQLite through rusqlite, Lofty for tags and artwork, and background indexing with cancellation and bounded concurrency.
- **System Integration**: Dedicated platform interfaces with Windows-specific implementations.
- **Icons**: Official [Lucide](https://lucide.dev/) SVG assets.

## Getting Started

### Prerequisites

For the current Windows build:

- Rust with the MSVC x64 toolchain. The current validation baseline is Rust 1.98.1.
- Visual Studio Build Tools with the Visual C++ build tools and Windows SDK.

### Build and Run

From the repository root:

```powershell
cargo run -p soliton-app
```

For release builds, Windows packaging, and update behavior, see the [distribution guide](docs/DISTRIBUTION.md). Development checks, smoke tests, and hardware probes are documented in the [development guide](docs/DEVELOPMENT.md).

## Keyboard Shortcuts

| Action | Shortcut |
| --- | --- |
| Search | Ctrl + F |
| Play / Pause | Space |
| Seek backward / forward 10 seconds | Ctrl + Left / Right |
| Adjust volume | Ctrl + Up / Down |
| Navigate history | Alt + Left / Right |
| Fullscreen | F11 |
| Go back | Escape |
| Navigate / activate controls | Tab / Enter |

## Documentation

See the [documentation index](docs/README.md) for development and validation details, architecture, dependencies, distribution, and product specifications.

## License

Soliton is licensed under the **GNU GPL v3 (GPL-3.0-only)**. See [LICENSE](LICENSE) for details, [asset attributions](assets/NOTICE.md), and [dependency license notices](licenses/dependencies/README.md).
