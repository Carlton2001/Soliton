# Soliton product decisions

Soliton is a desktop player for local music collections. It works without an
account, mandatory cloud services, telemetry or commercial streaming. Google
Cast may use the receiver's Google services.

This document records current product decisions. Update it when a feature
changes one of those decisions. See [AGENTS.md](../AGENTS.md) for project
invariants, the [README](../README.md) for features and shortcuts, and the
[documentation index](README.md) for architecture, distribution and validation.

## Library and metadata

- Music folders can be on local drives or network shares mounted by the OS.
  Playback is limited to files in registered sources.
- An unreachable source keeps its albums, metadata and cached artwork. Its tracks
  are unavailable until the source comes back. Unavailable albums are hidden by
  default, and a remounted source returns without a manual refresh.
- Embedded tags take precedence over metadata inferred from file and folder
  names. Inference uses paths below the source root. Inferred values appear in
  italics. Missing values use "Unknown artist" or "Unknown album", with unknown
  albums grouped by folder.
- Editions of the same album are selectable together when the metadata supports
  the match. Ambiguous matches stay separate. Within a folder and album title,
  a track without an album-artist tag can join an edition when its track artist
  matches the sole explicit album artist. This exception excludes compilations
  and cases with conflicting or inferred artist names.
- A compilation groups tracks with different artists under one album. Artist
  views and the artist filter use album artists. Track credits also appear on
  their tracks and in search.
- Artwork comes from embedded images. Albums without embedded artwork use a
  neutral placeholder.

## Interface

- Keep keyboard focus visible and primary navigation available in the sidebar.
- Album details open beside the collection on wide windows and on a dedicated
  page on narrow windows.
- Icon-only buttons provide a tooltip. A button whose label is fully visible
  shows a tooltip only when it provides additional information.
- The Windows installer and the Linux per-user archive can update from the app.
  Soliton checks for updates at startup when enabled. The user starts the
  installation and Soliton restarts after installing the signed update.
  Linux system packages and Flatpak use their distribution channels.

## Playback and collections

- Manual playlist names are case-insensitively unique. Each file can appear
  once per playlist or queue. Unavailable tracks stay visible in their playlists.
- Soliton restores the playback session, including the track, position, volume,
  selected output and playback modes. It keeps no listening history.
- With the system default device selected, playback follows changes to that
  device and respects pause requests from the system. If the explicitly chosen
  output disappears, playback pauses at the same position and shows a warning.
  Resuming uses the system default until the preferred output returns.
- Playback has one active output. When streaming, Soliton serves the active
  track and its artwork only to the selected receiver and reports unsupported
  formats.
