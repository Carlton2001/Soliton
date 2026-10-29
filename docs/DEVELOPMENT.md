# Development

Features and shortcuts: [README](../README.md). Linux specifics:
[LINUX.md](LINUX.md). Run commands from the repository root.

## Run

Windows: Rust MSVC, C++ Build Tools and Windows SDK. Linux: packages from
[LINUX.md](LINUX.md#prepare-fedora-for-development).

```sh
cargo run -p soliton-app --bin Soliton
```

## Data

SQLite, settings and artwork: `%LOCALAPPDATA%\Soliton` or
`~/.local/share/Soliton`, outside the sources. `SOLITON_DATA_DIR` isolates
development. Music is opened read-only.

## Check

```sh
cargo fmt --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
```

Native and hardware tests on Windows (FFmpeg only generates the fixtures):

```powershell
./scripts/generate-audio-fixtures.ps1
cargo test -p soliton-audio -- --include-ignored
cargo test -p soliton-platform -- --include-ignored
./scripts/smoke-windows.ps1
./scripts/smoke-icons-windows.ps1
```

`smoke-windows.ps1` opens Soliton on synthetic data in `target/`, captures the
window and fails on an exit code or a log error. Options:
`-Language English -Light -List`, `-CloseScenario Early|Minimized|Hidden|Ask`,
`-Taskbar`, `-Collections`, `-AudioOutputs [-AudioOutputRow n]`,
`-NetworkOutputs -LocalOutputCount n`. Linux native tests:
[LINUX.md](LINUX.md#build-test-run).

Explicit hardware examples (zero or low volume, idle Cast receiver):

```sh
cargo run -p soliton-audio --example bluetooth_probe -- "<output name>"
cargo run -p soliton-network --example discover
cargo run -p soliton-network --example smoke_cast -- <id> <file>
cargo run -p soliton-audio --example network_session -- <id> <file>
cargo run -p soliton-app --bin Soliton-index -- <soliton-data-dir> [music-dir]
```

In the Windows sandbox, `$env:CARGO_INCREMENTAL = '0'` avoids the hard link warning.

## Known limitations

- Shared output, mono/stereo sources (stereo on the front of a 5.1/7.1 output); no equalizer, exclusive mode or bit-perfect.
- Tested formats: WAV, FLAC, MP3, AAC-LC/M4A, ALAC, Vorbis, AIFF. No Opus, WMA, APE.
- CPAL outputs (Windows) are identified by name: a renamed output must be chosen again. The displayed format is what Soliton sends to the system.
- Bluetooth: codec chosen by the system, neither lossless nor bit-perfect guaranteed.
- Network: local IPv4, one active output, no transcoding or multiroom; DLNA tested only on a simulated receiver.
- Editions merge only when artist, title and year match; the edition choice is not remembered.
