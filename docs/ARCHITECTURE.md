# Architecture

## Crates

| Crate | Role |
| --- | --- |
| `soliton-core` | Domain types, contracts, commands, events, configuration, text comparison key. No GPUI. |
| `soliton-library` | Scanning, tags, SQLite, grouping, artwork cache, watching, search. |
| `soliton-audio` | Decoding, local output backends, queue, local/network output arbitration. |
| `soliton-network` | SSDP/mDNS discovery, DLNA and Cast control, media HTTP server. No GPUI or decoding. |
| `soliton-platform` | System integrations behind `Desktop`; native code confined to `windows/` and `linux/`. |
| `soliton-app` | Service assembly and GPUI interface. |

SQLite stays a module of `soliton-library`. Settings, database and artwork live in
the Soliton data folder (`SOLITON_DATA_DIR` isolates development), never in a
source. An invalid configuration file is reported, not overwritten; each write is
synced to disk before replacing the previous one.

GPUI 0.2.2, proc-macro-error2 2.0.1 and xattr 0.2.3 are vendored with targeted
patches: see [vendor/README.md](../vendor/README.md). Oxyde served as a reference
(bounded pools, cancellation, fonts, `SetWindowSubclass`, `cx.listener`) without
copying application code.

## Library

**Threads.** A single SQLite writer receives commands, watch events and
deadlines; between two batches of a scan it lets playlist changes through, other
work waits for the scan to end. Settings have their own worker, which writes only
the latest value. A WAL reader prepares snapshots, facets, track indexes and
searches; it rebuilds only when `PRAGMA data_version` changes. During indexing the
UI rereads the snapshot every 750 ms.

**Scanning.** `dua-core` (isolated in `discovery.rs`): two workers, two pending
batches per worker, 64 discovered entries pending; one to four tag readers, 16
pending jobs. One scan at a time, links not followed, native metadata reused
without a second `stat`.

**Incremental.** Size and modification time avoid rereads; extraction failures
are remembered by size, time and extractor version. A path with no Unicode
representation is reported and skipped without stopping the scan or its
conclusions.

**Removals.** After a full scan without traversal errors, on a source verified
reachable before and after, files not encountered are removed. Cancellation, an
unattributed traversal error or a source becoming unreachable prevent any
conclusion. A folder that cannot be listed only keeps its own subtree: its
entries stay, flagged unavailable, and the rest of the source is concluded.
`lost+found`, `System Volume Information` and `$RECYCLE.BIN` below the root are
not traversed. A root
with no audio file at all is treated as an unmounted volume, unless it is still on
the volume that held its music (stored device id, schema v8): the music was then
deleted. The volume id is `st_dev` on Unix and the volume serial number on
Windows; without one, doubt remains and entries are kept.

**Watching.** `notify`, queue bounded to 256 events; an overflow triggers a full
scan. Coalescing: 2 s of quiet, 15 s at most, 4,096 paths at most, turned into
folders to scan. `read_dir` probe on a dedicated thread, 5 s timeout, every 5 s;
a refusal makes a source offline at once, unanswered probes only after 30 s, so
that a NAS spinning up its disks does not disappear. Offline sources are probed at the same pace (1 s timeout) and become available
again as soon as their scan starts, without manual refresh. Full scan at startup
and every 30 minutes. The writer thread never touches a source without such a
probe: a scan checks its source before and after (5 s), a new watch is only
registered after a 1 s answer, and a source that does not answer keeps its state.

**Schema.** Sequential migrations in `schema.rs` (`user_version`), each
transactional with `foreign_key_check`. Sources, artists, albums, editions,
logical tracks, physical files, playlists and entries (unique path per playlist,
nullable track reference to survive removal from the catalog).

**Grouping** (`grouping.rs`, pure functions). Album folder = parent, skipping a
`CD1`/`Disc 2`/`Disque 3` level. Edition identity: source, folder, normalized
album title and artist. Album: tagged album artist, else compilation (flag or
differing artists, "Various Artists"), else track artist or folder. Merge across
folders only if artist, title, compilation and majority year are identical;
inferred title or missing year: never merged.

**Text.** `soliton_core::text::fold` (NFKD, no diacritics, expanded ligatures,
lowercase, normalized spaces) is used for grouping, sorting, search and facets,
which display the most frequent spelling.

**Artwork.** Content-addressed cache, 160 and 512 px thumbnails; missing entries
regenerated; orphan thumbnails and temporaries older than one hour removed after
each full scan, in the Soliton cache only.

## Audio

Symphonia decodes packet by packet on a dedicated worker; Rubato resamples on that
same worker. Shared mode, f32, native rate and channels when possible. Mono/stereo
sources only; an output with more than two channels (5.1, 7.1) gets stereo on
front left and right, other channels silent.

The callback only does atomic reads, pops complete frames from a preallocated
lock-free buffer and applies volume. Pending PCM is bounded to two seconds. Seek
keeps the stream open: the worker stops consuming, waits for two full callbacks,
clears the buffer, then resets decoder and resampler; successive seeks are
coalesced. Position comes from samples actually consumed.

An undecodable packet is skipped and reported once per track; eight in a row make
the file unreadable. A read error during playback ends the track with the audio
already decoded, reported the same way; a chained stream (Ogg) gets a new decoder.
In automatic playback an unreadable or unavailable track is
skipped, at most one queue round, and skipping stops after a failure that took a
second or more (hung source); an output or network failure stops playback.

The engine owns the queue: occurrences with stable ids, deduplication by path,
playlist origin kept, immutable snapshots shared through `Arc`. Changing what
comes next does not reopen the current stream.

**Local outputs.** One backend per target behind the same internal interface
(`backend` in `soliton-audio`): CPAL/WASAPI outside Linux, identified by name (the
only portable id in CPAL 0.16) and discovered every 5 s on a bounded worker;
native PipeWire on Linux, identified by node name and tracked live through the
registry. Ambiguous ids cannot be selected. `None` = default device, resolved at
each open under CPAL, followed continuously by PipeWire.

- A switch prepares the new session before replacing the old one; a failure keeps
  the old one.
- With `None`, a lost output is replaced at once by a session on the new default,
  at the played position, if the lost stream had run (a default failing in a loop
  pauses).
- A chosen output lost or missing pauses at the same position (`interruption`,
  cleared on resume); streams then open on the default (`fallback`) until the list
  shows it again, and the session returns to it. Never playback on an output other
  than the chosen one without a user action.

**Bluetooth.** Windows enumeration (Devices Enumeration, Bluetooth, Media Devices)
off the UI thread; matched to outputs by container id, never by name.

## Network

Started only when searching for devices. Tokio runtime with two threads, eight
blocking threads at most; commands limited to 12 s (16 s on the caller side).
While switching to a receiver, the audio worker keeps feeding the local stream
until the receiver accepts the media.
Progress follows receiver acknowledgements; one generation per load discards
states from a previous track.

**HTTP.** Server on the receiver's IPv4 interface, ephemeral port, random UUID per
media, access restricted to the chosen IP, active track and its artwork only. GET,
HEAD, OPTIONS, byte ranges; eight concurrent bodies, 64 KiB blocks. Revoked on
replacement, stop and disconnect.

**DLNA.** Bounded SSDP; XML and SOAP capped at 1 MiB, no DTD; HTTP URLs to the
announcing IP only, no proxy or redirect. AVTransport, RenderingControl when
present, ConnectionManager for MIME types.

**Cast.** `oxicast` and the Default Media Receiver, automatic reconnection off.
Only FINISHED advances to the next track; an IDLE during LOADING stays Buffering.
Soliton never drives or closes a session it did not open.

The network output lasts for the session; returning to local resumes at the remote
position. No failure switches to another output.

## Interface

Lists, grids and pickers virtualized (`uniform_list`). FR/EN strings centralized
in `i18n.rs`, errors included: engine and library produce
`soliton_core::failure::Failure` (kind without text, system detail kept) that the
UI translates under the label of their origin. Embedded Inter, digits in embedded
JetBrains Mono. Text field adapted from the GPUI example (Apache-2.0). Official
Lucide SVGs, unmodified, loaded with a 1.5 stroke instead of 2 (only that
attribute changes). Accents checked at a contrast of at least 4.5:1. Errors kept
(50 at most) in a banner.

## Windows integration

A worker owns the WinRT MTA apartment, SMTC and a hidden window for the
notification icon and Shell messages. States coalesced in a one-slot queue, system
commands in a 64-slot queue. Media keys go through SMTC.

- **SMTC**: artwork read from the cache (8 MiB max) into an
  `InMemoryRandomAccessStream`, because `StorageFile` rejects `\\?\` paths.
- **Taskbar** (`windows/taskbar/`): 640×360 image prepared on the worker every
  second and on each change; the HWND subclass only does a bounded GDI copy and
  sends commands. Restored on `TaskbarButtonCreated`.
- **Notification icon**: restored on `TaskbarCreated`; if it fails, hiding is
  disallowed.
- **Icons**: six SVGs from `assets/logos/` rasterized at seven sizes by
  `build.rs`, applied to the window, thumbnail and notification icon. If the UI
  does not respond yet, the window icon is retried every 250 ms; a failure is
  reported only after 10 s.
- **Autostart**: Soliton value under `HKCU\…\CurrentVersion\Run`, written only on a
  user action.
- **Closing**: cancel work, stop the engine, drop subscriptions and the icon,
  final save, then `cx.quit()`.

## Linux integration

`Desktop` exposes separate capabilities: `HIDES` (notification area) and
`AUTOSTART` are Windows only (without a notification area, GNOME would open the
window at every login). On Linux a worker owns the D-Bus session connection
(blocking `zbus`, already pulled in by GPUI); the UI sends coalesced state through
a one-slot queue, commands come back through a 64-slot queue. The window handle is
not requested: GPUI does not provide it on X11.

- **MPRIS**: `org.mpris.MediaPlayer2` and `.Player`; only status, metadata and
  capabilities emit `PropertiesChanged`, position is read on demand, and a drift
  above 1.5 s from the expected playback emits `Seeked`. A track id per occurrence
  protects `SetPosition` from stale requests.
- **Launcher icon**: rewritten only if the desktop entry carries the `install.sh`
  marker; the `hicolor` theme timestamp is bumped for caches.
- **Window**: `Raise` activates the window through GPUI; the saved maximized state
  is reapplied after opening.
