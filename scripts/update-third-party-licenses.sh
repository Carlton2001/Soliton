#!/bin/sh
# Linux counterpart of update-third-party-licenses.ps1: same notices, same
# inventory (keys, order and hashes), so either system can regenerate them.
# Usage: scripts/update-third-party-licenses.sh [--offline]
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
offline=
case "${1:-}" in
    --offline) offline=--offline ;;
    "") ;;
    *) echo "Usage: $0 [--offline]" >&2; exit 2 ;;
esac
destination=licenses/dependencies
temporary=$destination/THIRD-PARTY-NOTICES.md.tmp
trap 'rm -f "$temporary"' EXIT
mkdir -p "$destination"
if [ "$(cargo about --version 2>/dev/null)" != "cargo-about 0.9.2" ]; then
    echo "Install cargo-about 0.9.2: cargo install cargo-about --version 0.9.2 --locked --features cli" >&2
    exit 1
fi
if ! cargo about generate --workspace --locked --fail $offline \
    --output-file "$temporary" scripts/third-party-notices.hbs; then
    echo "License generation failed. Resolve the cargo-about diagnostics." >&2
    exit 1
fi
if [ ! -s "$temporary" ]; then
    echo "Empty third-party notices." >&2
    exit 1
fi
metadata=$(mktemp)
trap 'rm -f "$temporary" "$metadata"' EXIT
cargo metadata --locked --offline --format-version 1 >"$metadata"

# Python 3 is part of every Fedora installation; -I ignores the environment
# and the current directory.
python3 -I - "$root" "$metadata" <<'EOF'
import hashlib, json, os, re, shutil, sys

root, metadata = sys.argv[1], sys.argv[2]
destination = os.path.join(root, "licenses", "dependencies")
temporary = os.path.join(destination, "THIRD-PARTY-NOTICES.md.tmp")
notices = os.path.join(destination, "THIRD-PARTY-NOTICES.md")
NOTICE = re.compile(r"^(NOTICE|NOTICES|COPYRIGHT|AUTHORS)([-_.].*)?$", re.IGNORECASE)
LICENSE = re.compile(
    r"^(LICENSE|LICENCE|NOTICE|NOTICES|COPYRIGHT|AUTHORS)([-_.].*)?$", re.IGNORECASE
)
ASSETS = [
    "assets/NOTICE.md",
    "assets/icons/LICENSE",
    "assets/fonts/Inter-LICENSE.txt",
    "assets/fonts/JetBrainsMono-LICENSE.txt",
    "assets/GPUI-APACHE-2.0",
]


def files(directory, pattern):
    """Matching files below a directory, sorted like PowerShell (ignoring case)."""
    found = [
        os.path.join(parent, name)
        for parent, _, names in os.walk(directory)
        for name in names
        if pattern.match(name)
    ]
    return sorted(found, key=str.lower)


def text(path):
    # Same decoding as .NET ReadAllText: UTF-8, BOM removed, line endings kept.
    with open(path, encoding="utf-8-sig", errors="replace", newline="") as file:
        return file.read()


def digest(path):
    """SHA-256 of the UTF-8 text with LF line endings, as license-hash.ps1."""
    data = text(path).replace("\r\n", "\n").encode("utf-8")
    return hashlib.sha256(data).hexdigest().upper()


def relative(path):
    return os.path.relpath(path, root).replace(os.sep, "/")


# NOTICE/COPYRIGHT files are not necessarily recognized as license texts by cargo-about.
with open(metadata, encoding="utf-8") as file:
    graph = json.load(file)
members = set(graph["workspace_members"])
supplement = [
    "\n# Additional upstream notices\n\n"
    "Original NOTICE, COPYRIGHT and AUTHORS files, identified by package and source path.\n"
]
packages = sorted(
    graph["packages"],
    key=lambda p: (p["name"].lower(), p["version"].lower(), (p["source"] or "").lower()),
)
for package in packages:
    if package["id"] in members:
        continue
    directory = os.path.dirname(package["manifest_path"])
    for path in files(directory, NOTICE):
        contents = text(path)
        # Use a fence longer than any run in the upstream text, preserving it verbatim.
        longest = max((len(run) for run in re.findall(r"~+", contents)), default=0)
        fence = "~" * max(3, longest + 1)
        name = os.path.relpath(path, directory).replace(os.sep, "/")
        supplement.append(
            f"\n## {package['name']} {package['version']} — {name}\n\n"
            f"Source: {package['source'] or ''}\n\n{fence}text\n{contents}\n{fence}\n"
        )
with open(temporary, "a", encoding="utf-8", newline="") as file:
    file.write("".join(supplement))
# Git stores the notices with LF endings; upstream CRLF texts would otherwise
# show as changes. The hashes ignore the difference.
with open(temporary, encoding="utf-8", newline="") as file:
    generated = file.read().replace("\r\n", "\n")
with open(notices, "w", encoding="utf-8", newline="") as file:
    file.write(generated)
os.remove(temporary)

inputs = [
    "Cargo.lock",
    "Cargo.toml",
    "about.toml",
    "scripts/third-party-notices.hbs",
    "scripts/update-third-party-licenses.ps1",
    "scripts/license-hash.ps1",
    "scripts/update-third-party-licenses.sh",
]
for parent in ("crates", "vendor"):
    for name in sorted(os.listdir(os.path.join(root, parent)), key=str.lower):
        if os.path.isfile(os.path.join(root, parent, name, "Cargo.toml")):
            inputs.append(f"{parent}/{name}/Cargo.toml")
inputs += [relative(path) for path in files(os.path.join(root, "vendor"), LICENSE)]
inputs += ASSETS
copies = os.path.join(destination, "assets")
os.makedirs(copies, exist_ok=True)
for asset in ASSETS:
    name = "LUCIDE-LICENSE.txt" if asset == "assets/icons/LICENSE" else os.path.basename(asset)
    shutil.copyfile(os.path.join(root, asset), os.path.join(copies, name))
hashes = {path: digest(os.path.join(root, path)) for path in inputs}
hashes["licenses/dependencies/THIRD-PARTY-NOTICES.md"] = digest(notices)
for name in sorted(os.listdir(copies), key=str.lower):
    if os.path.isfile(os.path.join(copies, name)):
        hashes["licenses/dependencies/assets/" + name] = digest(os.path.join(copies, name))
with open(os.path.join(destination, "inputs.json"), "w", encoding="utf-8", newline="\n") as file:
    file.write(json.dumps(hashes, indent=2, ensure_ascii=False) + "\n")
EOF
echo "Third-party license notices regenerated."
