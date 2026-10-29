# Soliton specifications

**License:** GPL-3.0-only · **Language:** Rust · **UI:** GPUI ·
**Platforms:** Windows, Linux

This document sets the scope, product rules and remaining phases. Current
features are in the [README](../README.md), technical choices in
[ARCHITECTURE.md](ARCHITECTURE.md).

---

# 1. Goal and principles

Desktop audio player for a structured local music library: modern, fast and
sober, with MusicBee's richness without its visual clutter. Execution quality
over feature count.

- Native app, local operation, no mandatory account or cloud. Google Cast may
  rely on the receiver's Google services.
- Strictly read-only library; metadata and artwork come only from local files,
  never from online enrichment.
- UI independent of the audio engine; system integrations isolated.
- Pure Rust dependencies preferred. No telemetry, social features, purchases or
  commercial streaming.
- French and English UI.

---

# 2. Status

Implemented and validated on Windows: workspace and settings, library and
incremental indexing, library UI, local playback, editions and compilations,
artwork cache, source watching, playlists and queue, Windows integration (SMTC,
taskbar, notification area, closing, autostart), local and Bluetooth outputs,
DLNA / UPnP and Google Cast outputs.

Linux: working on Fedora Workstation (library, PipeWire playback, Wayland/X11
UI, MPRIS); archive, RPM, .deb and Flatpak built and validated manually; see
[LINUX.md](LINUX.md).

Remaining: publishing the Windows and Linux distributions, hardware validation
(phase 11). Deliverables are prepared, not published: publishing a release is a
separate request.

---

# 3. Product rules

These rules apply to every change.

## 3.1 Library

- Several source folders: internal or external drives, SMB shares and NAS as
  ordinary paths. No built-in SMB client.
- Soliton never modifies, moves, renames or deletes files, tags or embedded
  artwork. Databases, caches and temporary files stay outside source folders.
- Initial target under 10,000 tracks, with no artificial limit; virtualized
  lists and grids; no heavy work on the GPUI thread.
- SQLite versioned by migrations. Only settings and playlists cannot be rebuilt
  from the files.
- An unreachable source keeps its albums, metadata and cached artwork; its
  tracks are marked unavailable and cannot be played. A deletion is established
  only after a full scan of a reachable source. "Hide unavailable albums" is on
  by default; a remounted source reappears by itself within seconds.
- Background indexing: incremental, cancellable, tolerant of corrupt files, with
  progress. Real-time watching plus a startup and periodic scan.

## 3.2 Metadata, editions and artwork

- Embedded tags (`lofty`); otherwise inferred from the `Artist/Album/01 - Title`
  tree, stored only in Soliton and kept distinct from tags. Only folders below
  the source root name artist and album. Missing values show as "Unknown artist"
  and "Unknown album" (grouped by folder, "Unknown artist" card last); inferred
  values in italics.
- Distinct model: file, track, edition, grouped album, artist.
- Several editions of an album (e.g. 16/44.1 and 24/96) are shown together, with
  an edition choice. Cautious grouping: when ambiguous, two albums rather than a
  wrong merge.
- In one folder and for one title, a missing album artist does not create an
  extra edition if exactly one explicit album artist exists and the untagged
  tracks carry that same artist. Compilations and contradictory or inferred
  cases stay separate.
- Compilations: album artist distinct from track artist, single album.
- Artwork only embedded in files (no `folder.jpg`, no download), local cache
  invalidated when the image changes, neutral visual otherwise.

## 3.3 Interface

- Minimal, close to native, artwork in focus, discreet animations. Avoid needless
  panels and excessive options.
- Controls of one type share size, typography and states. Selection uses a tinted
  background, without weight or border change. Visible keyboard focus. Page
  titles raised under the title bar by a negative top margin computed from the
  font, without clipping letters or accents. Spacing: 0.75 rem above and below
  page content and between sections (not cumulative), same outer margins in list
  and grid; 0.5 rem between related actions. The sidebar logo keeps its own top
  margin.
- Navigation always visible in the sidebar, extended to the top edge with the
  Soliton icon above the buttons; title bar with no name to its right. Icons and
  labels: Artists, Albums, Playlists, Queue, Settings. No Back button; `Alt+←/→`
  browse history. Each main section shows its title under the compact title bar
  and above its controls; the title bar stays independent, 32 logical pixels high
  with 12 logical pixel system symbols. Detail pages (album, playlist) keep their
  own header; Now Playing has none.
- Title bar, navigation and player share the app background. A line separates
  navigation, player and each page header from the content. Search left of the
  sort controls, Grid/List on the right, same row; search shrinks if needed.
  Metadata filters only in Albums; genres and years as filters; the menu fits the
  displayed values.
- Albums: grid or list, choice remembered; on a wide window the detail opens in a
  right panel, on a narrow one in its own page.
- Album detail: large artwork, metadata, duration, tracks, formats, editions;
  per-track quality when it differs. Disc.track numbers always shown.
- Icon buttons: full label or icon only with tooltip, never in between. All
  buttons, grouped ones included, shrink together, playback buttons last. Menu
  entries keep their label. Secondary track actions in an "Actions" menu.
- Player: progress on top; artwork and metadata left; Shuffle, Previous,
  Play/Pause (centered), Next, Repeat, Queue; volume right, in a popover on narrow
  windows. Now Playing through the Expand button, which becomes Collapse to close
  it, like Escape; artwork and metadata grouped in a card. F11: fullscreen
  listening, the card alone with progress and playback controls; Escape, F11, its
  Collapse button, and artist, album and Queue links exit it.
- Light, dark and system themes, configurable accent, centralized design system.
- Icon: continuous wave on a rounded square, six palettes (Graphite and mint by
  default), applied without restart, independent of the theme. Window title
  "Soliton".
- No playback of files outside the library and no drag and drop; folders are
  added from the settings or an empty view.

## 3.4 Playback

- Engine independent of GPUI, driven by commands and events; audio callback
  without allocation, I/O, logging or blocking lock.
- Album, selection, shuffle, repeat track/album, navigation. Gapless not a
  priority.
- Named manual playlists and an editable queue. One occurrence per file: repeated
  additions are ignored. An unavailable track stays visible in its playlist. No
  smart playlists or import/export.
- Persisted session (track, position, volume, output, modes) without listening
  history or statistics.

## 3.5 Outputs and network

- Local shared output (`cpal`, PipeWire on Linux), persisted selection, switching
  without losing track or position, never a silent fallback to another output.
- With "System default device", a lost output (headset off) lets playback
  continue on the new system default; pause only if none opens. A pause requested
  by the system (WirePlumber on Linux) is respected.
- Chosen output lost or missing: pause at the same position, warning shown until
  resume. Resume plays on the system default; Soliton returns to the chosen
  output as soon as it reappears, without changing the saved preference.
- Linux: PipeWire sinks listed live, Bluetooth included; the system default
  follows Sound settings. Never a raw ALSA card.
- Bluetooth through system settings and the outputs the system exposes. Soliton
  does not claim to connect devices, claims neither lossless nor bit-perfect, and
  does not change the Windows default output.
- DLNA / UPnP and Google Cast: separate control layers, common selection, HTTP
  server limited to the active media and the chosen receiver, no path accepted
  from the network. Unsupported formats reported, no transcoding. One active
  output, no multiroom.

## 3.6 System integration

- Windows: SMTC with artwork, media keys, taskbar thumbnail with controls,
  notification icon (Show, Play/Pause, Previous, Next, Quit), configurable
  closing (quit, hide, ask) with an explicit Quit command, optional autostart.
- The notification icon must not be the only way to bring back a hidden window.
- Linux: MPRIS (desktop media panel and media keys, no URI opening), launcher
  icon following the palette for an `install.sh` install. Without a notification
  area on GNOME, closing always quits and no autostart is offered. Notifications
  postponed.

## 3.7 Distribution

- Windows: release profile without console, executable metadata (version,
  publisher, icon). Per-user Inno Setup installer without elevation, shortcuts,
  uninstall keeping user data; portable ZIP whose `data` folder keeps data next
  to the executable. No separate runtime to install.
- Updates (Windows): GitHub Releases with `latest.json`, can be disabled, off the
  UI thread, never automatic. On explicit click: download, signature check, clean
  shutdown, relaunch after install. Portable mode opens the download page instead.
- Linux: identifier `io.github.Carlton2001.Soliton`; RPM (Fedora 44+), .deb
  (Ubuntu 24.04+, Linux Mint 22, Debian 13), Flatpak (Flathub, Silverblue
  included, Freedesktop 26.08, offline build, minimal permissions, read-only
  Music folder and mount points, portal for other folders). Data in
  `~/.local/share/Soliton`, kept on uninstall. Packages declare Vulkan (lavapipe
  fallback accepted) and `install.sh` warns without `libvulkan.so.1`.
- Free signing only: Minisign for updates, GPG for Linux packages, no
  Authenticode ([DISTRIBUTION.md](DISTRIBUTION.md#signing)).
- Short FR/EN user guide, dependency license inventory and attributions
  shipped with every package.

---

# 4. Not planned

- Local control API (former phase 8): HTTP on `127.0.0.1` only, separate from
  the streaming server, mandatory authentication for remote access.
- Exclusive mode, bit-perfect, equalizer.
- Opus, WMA, APE and other formats needing extra decoders.
- Network transcoding, Alexa Wi-Fi.
- Several simultaneous outputs, multiroom or synchronized playback: one active
  output at a time.
- Linux: built-in updates, packages for other families (Arch, openSUSE…).

---

# 5. Phase 11: hardware validation

- Real collection of several thousand tracks: initial indexing, restart, SSD/NAS
  performance.
- Real NAS/SMB: disconnect and return, notifications, periodic scan.
- Physically unplugging a drive and an audio output during playback.
- Long listening sessions.
- Physical media keys, taskbar thumbnail hover and clicks, Explorer restart,
  launch at login.
- Physical DLNA receiver; Wi-Fi loss while casting.

**Deliverable:** known limitations updated, fixes for the defects found.

---

# 6. Risks

- **GPUI**: pre-1.0, unstable API; local patches in `vendor/`.
- **Network**: device-dependent behavior; Cast without an official Rust SDK,
  tested only on an NVIDIA SHIELD.
- **NAS**: variable notifications and performance; the periodic scan is the
  fallback.
- **Linux**: incomplete GPUI Linux backends (X11 window handle, maximized state at
  open, Wayland activation): work around them in Soliton rather than patching GPUI.
- **Distribution**: antivirus and SmartScreen warnings without Authenticode.
