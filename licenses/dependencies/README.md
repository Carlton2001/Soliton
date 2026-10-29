# Dependency licenses

Mechanism adapted from Oxyde-GPUI scripts, Copyright (C) 2026 Carlton2001,
GPL-3.0 (see LICENSE at the root).

```powershell
./scripts/install-cargo-about.ps1
./scripts/update-third-party-licenses.ps1
./scripts/check-third-party-licenses.ps1
```

```sh
cargo +1.98.1 install cargo-about --version 0.9.2 --locked --features cli
RUSTUP_TOOLCHAIN=1.98.1 scripts/update-third-party-licenses.sh
scripts/check-third-party-licenses.sh
```

- **Generator**: cargo-about 0.9.2, official Windows x64 binary pinned by SHA-256,
  or built from crates.io on Linux (sh and Python 3). Both produce identical
  notices and inventory. Generation reads Cargo.lock without changing it and fails
  on an unresolved license; `about.toml` lists the accepted licenses.
- **`THIRD-PARTY-NOTICES.md`**: texts, copyrights, names and versions of the crates
  of the whole workspace (all targets, build and dev dependencies, `vendor/`
  copies), hence more than the binaries contain; an extra section keeps upstream
  NOTICE, COPYRIGHT and AUTHORS files.
- **`assets/`**: Lucide (`LUCIDE-LICENSE.txt`), Inter, JetBrains Mono and GPUI
  licenses and Soliton's attributions (paths relative to the source `assets/`).
- **`inputs.json`**: hashes (UTF-8 text, LF line endings) of the lock, workspace
  and vendored manifests, configuration, generator, template, attributions and
  outputs. The check rejects changed or deleted inputs and new manifests, but does
  not rescan local sources: regenerate after any change to dependencies,
  attributions or vendored notices.
- **`-Offline` / `--offline`**: generation from the cache, which can lose the
  copyrights of badly packaged crates; generate online for releases.

Every shipped package includes this folder and Soliton's LICENSE.
