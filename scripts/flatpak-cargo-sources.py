#!/usr/bin/env python3
"""Writes installer/linux/flatpak/cargo-sources.json from Cargo.lock: every
crates.io package as a verified archive, vendored for the offline build, in the
format of flatpak-cargo-generator. Run it after each change of Cargo.lock."""
import json
import pathlib
import sys
import tomllib

ROOT = pathlib.Path(__file__).resolve().parent.parent
REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"
CONFIG = """[source.vendored-sources]
directory = "cargo/vendor"

[source.crates-io]
replace-with = "vendored-sources"
"""

lock = tomllib.loads((ROOT / "Cargo.lock").read_text())
sources = []
for package in lock["package"]:
    source = package.get("source")
    if source is None:
        continue  # Workspace crates and the patches in vendor/.
    if source != REGISTRY:
        sys.exit(f"{package['name']} comes from {source}: only crates.io is supported.")
    name, version, checksum = package["name"], package["version"], package["checksum"]
    dest = f"cargo/vendor/{name}-{version}"
    sources.append({
        "type": "archive",
        "archive-type": "tar-gzip",
        "url": f"https://static.crates.io/crates/{name}/{name}-{version}.crate",
        "sha256": checksum,
        "dest": dest,
    })
    sources.append({
        "type": "inline",
        "contents": json.dumps({"package": checksum, "files": {}}),
        "dest": dest,
        "dest-filename": ".cargo-checksum.json",
    })
sources.append({"type": "inline", "contents": CONFIG, "dest": "cargo", "dest-filename": "config"})

output = ROOT / "installer/linux/flatpak/cargo-sources.json"
output.write_text(json.dumps(sources, indent=4) + "\n")
print(output.relative_to(ROOT))
