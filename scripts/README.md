# Scripts

Run from the repository root. Builds and smoke-test outputs go to `target/`.
Package builders write to `target/distribution/output/`.

## Windows (PowerShell)

| Script | Purpose |
| --- | --- |
| `build-distribution.ps1` | Release build, portable ZIP and Inno Setup installer with licenses and SHA-256 (`-PortableOnly`, `-SkipBuild`, `-IsccPath`) |
| `write-update-manifest.ps1` | Writes `latest.json` for the updater (release workflow) |
| `bump-version.ps1` | Sets the version, commits and tags locally |
| `install-cargo-about.ps1` | Installs the pinned official cargo-about binary |
| `update-third-party-licenses.ps1` | Regenerates `licenses/dependencies` (`-Offline`) |
| `check-third-party-licenses.ps1` | Fails if the license inventory is stale |
| `install-cargo-audit.ps1` | Installs the pinned official cargo-audit binary |
| `audit-security.ps1` | RustSec audit, tools and database kept in `target/` |
| `generate-audio-fixtures.ps1` | Codec fixtures for ignored decoder tests (needs FFmpeg) |
| `smoke-windows.ps1` | Launches Soliton on synthetic data and captures the window ([native test options](../docs/DEVELOPMENT.md#native-tests)) |
| `smoke-icons-windows.ps1` | Selects each of the six icon palettes, checks it is saved and captures the window (`-Language`, `-Light`) |
| `tests/*.tests.ps1` | CI tests of the license check, update manifest and distribution scripts |

## Linux (sh, Python 3)

| Script | Purpose |
| --- | --- |
| `build-linux.sh` | Release build and per-user archive with SHA-256 (`--skip-build`) |
| `build-rpm.sh` | Fedora RPM from the archive binary (`--skip-build`) |
| `build-deb.sh` | .deb built on Ubuntu 24.04, in podman elsewhere, checked by lintian |
| `bump-version.sh` | Sets the version and AppStream release, commits and tags locally |
| `update-third-party-licenses.sh` | Regenerates `licenses/dependencies`, identical to the PowerShell output (`--offline`) |
| `check-third-party-licenses.sh` | Fails if the license inventory is stale |
| `sign-release.sh` | Signs release files (Minisign, GPG) and verifies them; release workflow only |
