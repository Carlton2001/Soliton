# Distribution and updates

Windows packaging and update scripts were adapted from Oxyde-GPUI, Copyright
(C) 2026 Carlton2001, GPL-3.0. See the [Linux guide](LINUX.md) for Linux packages.

## Updates (Windows)

The updater checks for a newer stable release at startup when enabled, and on
demand in Settings. It defaults to `Carlton2001/Soliton`. Set
`SOLITON_UPDATE_REPOSITORY` before building to target another repository.
GitHub Actions sets it to the repository running the workflow.

The published release's `latest.json` specifies the version, installer URL and
base64 Minisign signature. The updater checks the signature against the compiled
public key and requires its trusted comment to match `soliton-version:<version>`.
WinHTTP runs on a worker, with a 64 KiB limit for the manifest and 512 MiB for
the installer.

Installation starts when the user clicks the update notice. Setup waits for
Soliton to exit, replaces the program files and relaunches it. On failure,
Soliton stays open and can retry. Portable mode opens the download page.

## Build on Windows x64

Use the [Windows development tools](DEVELOPMENT.md#run) and Inno Setup 6.3 or
later for the installer.

```powershell
./scripts/install-cargo-about.ps1
./scripts/build-distribution.ps1               # or -PortableOnly
```

The script regenerates and checks the license inventory, then writes the
portable ZIP, Setup and local checksums to `target/distribution/output/`.
`-PortableOnly` skips Setup. `-SkipBuild` repackages the existing release
binary, which must use the intended update repository and static MSVC runtime.

- For portable mode, extract the whole ZIP. The `Soliton/data` folder keeps
  library data next to the executable.
- Setup installs per user in `%LOCALAPPDATA%\Programs\Soliton` and keeps
  `%LOCALAPPDATA%\Soliton` on uninstall.
- `SOLITON_DATA_DIR` overrides the data location in both modes.

## Release workflow

The [release workflow](../.github/workflows/release.yml) checks that the
`vX.Y.Z` tag matches the workspace version, runs the Windows tests and
dependency audit, builds both platforms, signs the artifacts and creates a
draft. Publish the draft manually to make it available to the updater.

Routine checks are defined in the [Windows tests](../.github/workflows/tests.yml),
[Linux tests](../.github/workflows/linux.yml),
[Windows packaging](../.github/workflows/distribution.yml) and
[dependency audit](../.github/workflows/security.yml) workflows. Hardware
scenarios are in the [development guide](DEVELOPMENT.md#manual-hardware-scenarios).

## Signing

Windows builds use Minisign for the updater. SmartScreen can still display a
warning on first launch.

| Key | Public (repository) | Signs |
|---|---|---|
| Minisign | `installer/signing/soliton-update.pub` | the Windows installer, checked by the updater |
| GPG ed25519 `7BFC00A65179DAEDD7CD26298EED1CC9F8CE693B` | `installer/signing/soliton-packages.asc` | the RPM (embedded signature) and `SHA256SUMS` (detached `SHA256SUMS.asc`), which lists every package |

Private keys stay outside the repository. The
[signing script](../scripts/sign-release.sh) verifies the signatures against
the repository public keys and replaces individual checksums with
`SHA256SUMS` and `SHA256SUMS.asc`.

The release workflow uses these values.

- Repository variable `SOLITON_UPDATE_PUBLIC_KEY` is the `base64 -w0` output for
  `soliton-update.pub`. Set the same value for local builds that use the updater.
- The `release` environment uses `SOLITON_UPDATE_PRIVATE_KEY` (content of
  `update.key`), `SOLITON_UPDATE_PRIVATE_KEY_PASSWORD`, `SOLITON_GPG_PRIVATE_KEY`
  (`gpg --armor --export-secret-keys`) and `SOLITON_GPG_PASSPHRASE`.

A Minisign key rotation must let older versions verify new installers. Check
the GPG key fingerprint against the table before importing it. Run these
commands alongside the downloaded files.

```sh
gpg --import soliton-packages.asc
gpg --verify SHA256SUMS.asc SHA256SUMS
sha256sum -c --ignore-missing SHA256SUMS
```

## Prepare a version

Run the version script from a clean `main` checkout with a free version number.

```powershell
./scripts/bump-version.ps1 -Version X.Y.Z
```

```sh
sh scripts/bump-version.sh X.Y.Z
```

Replace `X.Y.Z` with the new version. The script updates the manifest, lockfile,
license notices and AppStream metadata, then creates a local commit and tag.
See [AGENTS.md](../AGENTS.md#contribution-workflow) for push authorization.
