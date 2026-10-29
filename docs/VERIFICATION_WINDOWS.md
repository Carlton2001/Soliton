# Windows checks after Linux support

Linux work (`76ea35f Make Soliton functional on Linux`, then native PipeWire
output) is validated on Linux. On Windows only `soliton-platform` and
`soliton-audio` were cross-checked (`cargo check` / `cargo xwin check`, tests
included); the app cannot be cross-built from Linux (GPUI shaders, `aws-lc-sys`).

## What touches Windows

- **Integration startup**: `Desktop::start` now takes the GPUI window
  (`&impl HasWindowHandle`) instead of a `RawWindowHandle` and reads the handle
  itself. This path starts SMTC, the taskbar thumbnail, the notification icon and
  the icons.
- **`Desktop` capabilities**: `Desktop::SUPPORTED` is complemented by
  `Desktop::HIDES` and `Desktop::AUTOSTART`. Windows must behave as before:
  configurable closing, hiding, autostart.
- **License inventory**: hashes of `Cargo.lock`, `crates/soliton-app/Cargo.toml`
  and `crates/soliton-platform/Cargo.toml` changed; regenerated notices are identical.
- **Audio output**: CPAL code moved to `cpal_backend.rs`, built outside Linux only,
  with no intended behavior change. The callback renderer takes the sample
  conversion as a parameter. Outputs carry a label (equal to the name on Windows)
  and a Bluetooth flag (always false on Windows).
- **Output loss** (shared): playback pauses instead of failing; resume uses the
  default output, then Soliton returns to the chosen output when enumeration
  shows it again.
- **Default output loss** (shared): with "System default device", a lost output
  no longer pauses; the engine reopens the session on the new Windows default at
  the same position. Pause only if none opens or if the lost stream never ran.
- **Window restore**: without a known screen at open, the saved position is
  restored unchecked. Windows knows its screens at open, so the check still applies.
- **Distribution**: `scripts/build-distribution.ps1` now builds the `Soliton`
  binary (not `soliton`), which made the "Windows distribution" workflow fail.
- **No effect on Windows**: restoring the maximized state at startup is Linux only.

## 1. Build and tests

```powershell
cargo +1.98.1 fmt --check
cargo +1.98.1 check --workspace --all-targets --locked
cargo +1.98.1 test --workspace --locked
./scripts/check-third-party-licenses.ps1
```

- [ ] `fmt --check` passes
- [ ] `check` passes without project warnings
- [ ] Workspace tests pass
- [ ] License inventory up to date

## 2. Native Windows integration test

Goes through the adapted window handle path:

```powershell
cargo +1.98.1 test -p soliton-platform native_tray_smtc_artwork_commands_and_cleanup -- --ignored
```

- [ ] Passes

Native audio tests, at zero volume:

```powershell
cargo +1.98.1 test -p soliton-audio native_output -- --ignored
cargo +1.98.1 test -p soliton-audio unavailable_preference -- --ignored
cargo +1.98.1 test -p soliton-audio default_output_loss -- --ignored
```

- [ ] Pass

## 3. Manual app check

```powershell
cargo +1.98.1 run -p soliton-app --bin Soliton --locked
```

Window and icons:

- [ ] Window and taskbar icon shown at startup.
- [ ] Changing the palette in Settings → Application icon updates the window,
  taskbar and notification icon.

Media panel and taskbar:

- [ ] During playback the Windows media panel shows title, artist and artwork.
- [ ] Play/Pause, Previous and Next work from the panel and media keys.
- [ ] Taskbar thumbnail buttons work.

Settings:

- [ ] The "Windows integration" section is still present, with "On close"
  (Quit / Hide / Ask) and "Launch Soliton at login".
- [ ] The autostart box toggles the `Soliton` value under
  `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
- [ ] "Audio output" still lists devices, not just the system output, without the
  "choose in Sound settings" note.
- [ ] Choosing a device during playback keeps track and position; the displayed
  format and the player's output tooltip name that device.
- [ ] The note under the list still says the system default applies when a track
  starts.
- [ ] With "System default device", unplugging or switching off the default
  output (USB or Bluetooth headset) during playback continues without pause on the
  new Windows default, without warning.
- [ ] A chosen output unplugged pauses playback with the "Playback paused" warning
  above the player bar. Resume plays on the default output; plugging the output
  back returns playback to it once Windows reports the device.

Closing:

- [ ] "Hide" hides the window; the notification icon restores it.
- [ ] "Ask" shows the confirmation dialog with the Hide option.
- [ ] "Quit", from the window or the notification icon, exits cleanly.

## 4. Windows distribution

```powershell
./scripts/build-distribution.ps1 -PortableOnly
```

- [ ] The portable distribution builds (or the "Windows distribution" workflow passes).

On failure, note the command, output or observed behavior.
