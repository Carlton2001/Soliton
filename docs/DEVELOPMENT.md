# Development

Run commands from the repository root.

## Run

Use at least the Rust version declared in [Cargo.toml](../Cargo.toml). On
Windows, install the x64 MSVC toolchain, Visual Studio C++ Build Tools and the
Windows SDK. Use PowerShell 7 for the project scripts. On Linux, install the
[development packages](LINUX.md#prepare-fedora-for-development).

```sh
cargo run -p soliton-app --bin Soliton --locked
```

## Data

Set `SOLITON_DATA_DIR` to a separate directory to isolate a development run.
Native installs use `%LOCALAPPDATA%\Soliton` on Windows and
`${XDG_DATA_HOME:-$HOME/.local/share}/Soliton` on Linux.

`Soliton-index` inspects or indexes a test collection without opening the app.
Pass the data directory alone to inspect an existing index.

```sh
cargo run -p soliton-app --bin Soliton-index --locked -- ./target/index-test "/path/to/test-music"
```

## Check

```sh
cargo fmt --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Use the [script index](../scripts/README.md) for packaging and dependency
checks. In the Windows sandbox, `$env:CARGO_INCREMENTAL = '0'` avoids the hard
link warning.

## Native tests

Run these Windows tests in a desktop session at low or zero volume. FFmpeg
generates the test fixtures.

```powershell
./scripts/generate-audio-fixtures.ps1
cargo test -p soliton-audio --locked -- --ignored
cargo test -p soliton-platform --locked -- --ignored
./scripts/smoke-windows.ps1
./scripts/smoke-icons-windows.ps1
```

`smoke-windows.ps1` opens Soliton on synthetic data in `target/`, captures the
window and fails on a nonzero exit code or a log error. Its options include
`-Language English -Light -List`, `-CloseScenario Early|Minimized|Hidden|Ask`,
`-Taskbar`, `-Collections`, `-AudioOutputs [-AudioOutputRow n]`,
`-NetworkOutputs`, `-Updates`. Each run starts from a fresh library. See the
[Linux native tests](LINUX.md#build-test-run) for PipeWire and MPRIS coverage.

Use a low volume and an idle receiver for the audio and network probes. Replace
the sample names and paths with those of the test device and file.

```sh
cargo run -p soliton-audio --example bluetooth_probe -- "output name"
cargo run -p soliton-network --example discover
cargo run -p soliton-network --example smoke_cast -- "receiver-id" "/path/to/test.flac"
cargo run -p soliton-audio --example network_session -- "receiver-id" "/path/to/test.flac"
```

## Manual hardware scenarios

Use these scenarios when changing the relevant subsystem. See the
[Linux validation record](LINUX.md#validation) for prior hardware coverage.

- **Library:** Index a large collection on SSD and NAS. Check startup and
  browsing after restarting. Disconnect and reconnect the NAS or source drive
  during playback and verify availability, alerts and periodic scan recovery.
- **Audio:** Switch outputs during playback and check that track and position
  are kept. Test unplugging a device in system-default and explicit-output
  modes. Check warning, resume and return behavior against the
  [product decisions](SOLITON_SPECIFICATIONS.md#playback-and-collections).
  Include a long listening session when changing the audio engine.
- **Windows desktop:** Check media panel metadata and artwork, physical media
  keys, taskbar controls, icon palette, close and restore flows, recovery after
  restarting Explorer, and launch at login.
- **Network:** Test a physical DLNA receiver and loss of Wi-Fi while casting.

## Known limitations

- Local playback uses shared output and accepts mono or stereo sources.
  Exclusive mode, bit-perfect playback and equalization are unavailable. The OS
  picks the Bluetooth codec.
- Windows identifies outputs by name. Select a renamed device again in Settings.
- Network playback uses local IPv4 and serves the original audio without
  transcoding. DLNA has been exercised with a simulated receiver.
