# Dependencies

Resolved versions are in Cargo.lock. No direct dependency on Oxyde.

| Area | Choice | License | Notes |
| --- | --- | --- | --- |
| GUI | gpui 0.2.2 (crates.io) | Apache-2.0 | Windows D3D, Linux X11/Wayland backends; pre-1.0, unstable API |
| Tags | lofty 0.25.4 | MIT / Apache-2.0 | Pure Rust, tags and embedded images, audio properties enabled |
| Database | rusqlite 0.36, bundled | MIT | SQLite compiled in, no SQLite DLL to install |
| Scanning | dua-core 5.0.0 | MIT | Bounded workers and results, links not followed |
| Text | unicode-normalization 0.1 | MIT / Apache-2.0 | Pure Rust, already in the tree; accent-free comparison key |
| Channels | crossbeam-channel 0.5 | MIT / Apache-2.0 | Bounded queues between services |
| Watching | notify 8.2 | CC0-1.0 | ReadDirectoryChangesW on Windows, inotify on Linux; Soliton's own coalescing, no debouncer crate |
| Decoding | symphonia 0.5.5 | MPL-2.0 | Pure Rust, MP3/FLAC/AAC-LC/ALAC/Vorbis/PCM/AIFF; no Opus/WMA/APE |
| Output | cpal 0.16 | Apache-2.0 | WASAPI on Windows; not built on Linux |
| Linux output | pipewire 0.10.1 | MIT | Native PipeWire stream and registry; `pipewire-devel` and `clang-devel` (bindgen) at build time, `libpipewire-0.3` at runtime |
| Resampling | rubato 0.16.2 | MIT | Sinc resampling outside the callback, delay and final length compensated |
| PCM buffer | crossbeam-queue 0.3 | MIT / Apache-2.0 | Preallocated queue of complete mono/stereo frames, no channel shift after underrun |
| Windows integration | windows 0.61.3 | MIT / Apache-2.0 | Already resolved by GPUI; SMTC through HWND interop, Shell_NotifyIcon, native menu; Windows target only |
| Autostart | windows-registry 0.5.3 | MIT / Apache-2.0 | Already resolved; current-user Run key, no elevation |
| Window handle | raw-window-handle 0.6 | MIT / Apache-2.0 | Portable contract used by GPUI; no HWND in common code |
| Notification icon | resvg 0.45.1 | MIT / Apache-2.0 | Already resolved by GPUI; rasterizes the official Lucide `play.svg`, path unchanged |
| Taskbar thumbnail | image 0.25.10, windows 0.61.3 | MIT / Apache-2.0 | Already resolved; PNG decoding from the cache on the worker, DIB/GDI, DWM and ITaskbarList3; Lucide glyphs through resvg |
| Async network | tokio 1.54, axum 0.8.9 | MIT | Two runtime threads, eight blocking threads at most; HTTP server on the chosen receiver's interface, no control endpoint |
| UPnP HTTP client | reqwest 0.13.5, roxmltree 0.20 | MIT / Apache-2.0 | Local HTTP only, proxy and redirects off, responses capped at 1 MiB; XML without DTD |
| Cast discovery | mdns-sd 0.20.3 | MIT / Apache-2.0 | IPv4 mDNS discovery, independent of decoding and GPUI |
| Google Cast | oxicast 0.0.3 | MIT / Apache-2.0 | Isolated adapter, bounded requests, automatic reconnection off, default media receiver tested on SHIELD; no official Rust SDK |
| HTTP capabilities | uuid 1.28, futures-util 0.3 | MIT / Apache-2.0 | Random UUID per media, revoked on replacement/stop, eight HTTP bodies at most |
| SSDP interfaces | if-addrs 0.15, socket2 0.6 | MIT / Apache-2.0 | Eight IPv4 interfaces at most, four concurrent description reads; no system type in common contracts |
| Bluetooth (Windows) | windows 0.61.3, windows-collections 0.2.0, windows-future 0.2.1 | MIT / Apache-2.0 | Devices Enumeration, Bluetooth and Media Devices APIs; Windows target only |

GPUI 0.2.2, proc-macro-error2 2.0.1 and the Unix dependency xattr 0.2.3 are
vendored with targeted patches through `[patch.crates-io]`:
[vendor/README.md](../vendor/README.md).

Cast uses rustls with ring and AWS-LC, statically linked (no DLL to install).
Self-signed Cast receiver certificates are accepted only by this client, for
discovered destinations; no DeviceAuth authentication.
