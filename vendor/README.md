# Dependency patches

Copies of packages published on crates.io, with no version or API change, applied
through `[patch.crates-io]` in `Cargo.toml` and excluded from the workspace. Their
licenses and sources are kept; local Cargo caches are not modified. On an upstream
update, review these changes and drop them once the same behavior is fixed and
validated in the chosen version.

## GPUI 0.2.2, Apache-2.0

Source: https://crates.io/crates/gpui/0.2.2

- `src/platform/windows/directx_devices.rs`: a failure of the optional DXGI Debug
  probe is a debug diagnostic. Real device creation keeps its error handling and
  the fallback without Graphics Tools stays active.
- `src/platform/windows/events.rs` (custom title bar): as in later GPUI versions,
  a non-client click is consumed only if a handler stops its propagation, no longer
  as soon as focus calls `prevent_default`, so dragging and native buttons keep
  working. The top resize edge is tested before control areas.
- `src/platform/windows/events.rs`: `WM_DESTROY` marks the window destroyed,
  removes its callbacks and revokes drag and drop before the HWND is lost.
- `src/platform/windows/window.rs`: the destructor removes callbacks to the dropped
  GPUI entity and does not destroy an already destroyed HWND again, checked once
  more when the deferred destroy task runs.
- `src/taffy.rs`: annotate two `f32` literals to avoid the type fallback Rust 1.98
  reports as a future incompatibility (rust-lang/rust#154024).
- `src/elements/text.rs` and `src/text_system/line_wrapper.rs`: each truncated text
  measurement uses its own runs, so a first narrow measurement cannot leave stale
  lengths for a wider one, split a UTF-8 character and panic in DirectWrite.
- `src/platform/linux/wayland/client.rs`: accept `xdg_wm_base` from version 1 (as
  announced by WSLg) while negotiating up to version 5. The requests used exist in
  version 1; tiling states added in version 2 stay optional.

No global log filter, no disabled debug assertions and no cursor change are applied.

## proc-macro-error2 2.0.1, MIT OR Apache-2.0

Source: https://crates.io/crates/proc-macro-error2/2.0.1

`src/lib.rs`: make `extern crate proc_macro` public so its public re-export is
valid (E0365, rust-lang/rust#127909). No macro logic change.

## xattr 0.2.3, MIT OR Apache-2.0

Source: https://crates.io/crates/xattr/0.2.3

Indirect Unix dependency of GPUI through `gpui_http_client` and `zed-async-tar`.
`src/sys/mod.rs`: use `libc::ENODATA` for a missing attribute on Linux; `libc`
0.2.191 no longer exposes the old `ENOATTR` alias for this target. The internal
`ENOATTR` name and the crate API are kept; other targets are unchanged
([getxattr(2)](https://man7.org/linux/man-pages/man2/getxattr.2.html)). No
attribute write is added to Soliton's library scan.
