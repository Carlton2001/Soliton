# Distribution and updates

Scripts and workflows adapted from Oxyde-GPUI, Copyright (C) 2026 Carlton2001,
GPL-3.0, without depending on its checkout. Linux packages: [LINUX.md](LINUX.md).

## Updates (Windows)

Reference repository: `Carlton2001/Soliton` (never the Oxyde one). Workflows pass
`github.repository` through `SOLITON_UPDATE_REPOSITORY`, read **at compile time**;
for a local build targeting another repository:

```powershell
$env:SOLITON_UPDATE_REPOSITORY = 'owner/repo'
cargo run -p soliton-app
```

- Check at startup (on by default) and on demand in the settings; WinHTTP on a
  worker, network timeouts, response capped at 64 KiB, errors visible, check can
  be retried.
- `latest.json`, attached to the latest published stable release: `major.minor.patch`
  version (only newer ones are offered), exact installer URL and its Minisign
  signature (base64 `.minisig`). Without a published release, nothing is offered.
- The sidebar `info.svg` icon appears, in the accent color, only when an update
  exists. On click: download (512 MiB max), check with the compiled public key,
  launch Setup, which waits for Soliton to exit, replaces the files and relaunches
  it. On failure Soliton stays open and can retry. In portable mode the click opens
  the download page and reminds to keep `data`.
- No built-in updates on Linux.

## Build on Windows x64

Rust MSVC 1.98.1, C++ Build Tools, Windows SDK and, for the installer, Inno Setup
6.3 or later.

```powershell
./scripts/install-cargo-about.ps1
./scripts/build-distribution.ps1               # or -PortableOnly
```

Output in `target/distribution/output/`: portable ZIP and Setup (both with a
bilingual `README.txt`), SHA-256 hashes (the
release job adds the signature and `latest.json`). Licenses are regenerated and
checked before assembly. The MSVC C runtime is statically linked; the script
rejects any vcruntime/msvcp DLL dependency. `-SkipBuild` reassembles existing
binaries, built with the same options and the intended update repository.

- **Portable**: extract the whole ZIP; the `Soliton/data` folder enables portable
  mode (data next to the executable).
- **Installed**: per user in `%LOCALAPPDATA%/Programs/Soliton`, no elevation, data
  in `%LOCALAPPDATA%/Soliton`, kept on uninstall.
- `SOLITON_DATA_DIR` still takes precedence; music stays read-only.

## GitHub Actions

- `linux.yml` (Fedora): license inventory, script syntax, formatting, check, tests and build, on main and pull requests.
- `tests.yml`: formatting, check of all targets and Windows tests, on main and pull
  requests. No implicit hardware test.
- `security.yml`: RustSec audit for Windows x64 on dependency changes, every Monday
  and manually; vulnerabilities and yanked crates block releases; reports kept as
  artifacts.
- `distribution.yml`: release cache and assembly on distribution manifest/script
  changes, or manually.
- `_linux-distribution.yml` (reusable): archive and RPM (Fedora 44 container), .deb
  (Ubuntu 24.04), Flatpak bundle.
- `release.yml`: on a `vX.Y.Z` tag, compares the tag with the workspace, tests,
  audits, builds once (Windows and Linux), signs, then creates a **draft** release,
  to be published manually to expose it to update checks.

Third-party actions are pinned to the revisions taken from Oxyde; permissions are
read-only except for the final release job.

## Signing

Free only, no Authenticode certificate (user decision): SmartScreen warns on the
first download of the Windows executable.

| Key | Public (repository) | Signs |
|---|---|---|
| Minisign | `installer/signing/soliton-update.pub` | the Windows installer, checked by the updater |
| GPG ed25519 `7BFC00A65179DAEDD7CD26298EED1CC9F8CE693B` | `installer/signing/soliton-packages.asc` | the RPM (embedded signature) and every other file (detached `.asc`) |

Private keys stay outside the repository (`~/.soliton-keys/update.key`, the
author's GPG keyring), backed up off the machine. `scripts/sign-release.sh` checks
every signature against the repository public keys before creating the release.
GitHub setup (Actions enabled on `Carlton2001/Soliton`):

- repository variable `SOLITON_UPDATE_PUBLIC_KEY`: `base64 -w0` of
  `soliton-update.pub`, compiled into Windows builds (the script rejects a value
  differing from the file; pass it for local builds too);
- `release` environment secrets: `SOLITON_UPDATE_PRIVATE_KEY` (content of
  `update.key`), `SOLITON_UPDATE_PRIVATE_KEY_PASSWORD`, `SOLITON_GPG_PRIVATE_KEY`
  (`gpg --armor --export-secret-keys`), `SOLITON_GPG_PASSPHRASE`.

A Minisign key rotation must let older versions verify new installers. Verify a
package: `gpg --import soliton-packages.asc` then `gpg --verify <file>.asc`, or
`sudo rpm --import soliton-packages.asc` before installing the RPM.

## Prepare a version

On a clean tree:

```sh
./scripts/bump-version.ps1 -Version 0.2.0   # Windows
sh scripts/bump-version.sh 0.2.0            # Linux
```

The script updates the workspace version, Cargo.lock and the AppStream metainfo
(version and screenshot URLs), creates an English commit and a local tag, and
pushes nothing. Every push needs an explicit user request.
