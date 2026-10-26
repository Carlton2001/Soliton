#!/bin/sh
# Linux counterpart of check-third-party-licenses.ps1: same inventory, same
# hashes (SHA-256 of the UTF-8 text with LF line endings).
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
inventory=$root/licenses/dependencies/inputs.json
if [ ! -f "$inventory" ]; then
    echo "Run scripts/update-third-party-licenses.ps1 first." >&2
    exit 1
fi
hash() {
    sed 's/\r$//' "$1" | sha256sum | cut -d' ' -f1 | tr '[:lower:]' '[:upper:]'
}
tracked=$(sed -n 's/^ *"\([^"]*\)": *"\([0-9A-F]*\)",\{0,1\}$/\1 \2/p' "$inventory")
if [ -z "$tracked" ]; then
    echo "Unreadable license inventory." >&2
    exit 1
fi
printf '%s\n' "$tracked" | while read -r path expected; do
    if [ ! -f "$root/$path" ] || [ "$(hash "$root/$path")" != "$expected" ]; then
        echo "Third-party notices are stale: $path. Run scripts/update-third-party-licenses.ps1." >&2
        exit 1
    fi
done
for manifest in "$root"/crates/*/Cargo.toml "$root"/vendor/*/Cargo.toml; do
    relative=${manifest#"$root/"}
    if ! printf '%s\n' "$tracked" | cut -d' ' -f1 | grep -qxF "$relative"; then
        echo "Untracked license input: $relative. Regenerate notices." >&2
        exit 1
    fi
done
echo "Third-party license notices are up to date."
