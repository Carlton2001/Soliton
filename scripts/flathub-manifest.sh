#!/bin/sh
# Writes the Flathub copy of the Flatpak manifest: the local repository source
# becomes the GitHub repository at a release tag and its commit. The output
# directory holds the files of the Flathub submission (manifest and cargo
# sources), copied there by hand.
# Usage: scripts/flathub-manifest.sh <tag> [output directory]
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
APP_ID=io.github.Carlton2001.Soliton
URL=https://github.com/Carlton2001/Soliton.git
tag=${1:-}
output=${2:-$root/target/flathub}
case "$tag" in
    v[0-9]*.[0-9]*.[0-9]*) ;;
    *) echo "Usage: $0 <vX.Y.Z> [output directory]" >&2; exit 2 ;;
esac
commit=$(git rev-parse --verify "$tag^{commit}")

manifest=installer/linux/flatpak/$APP_ID.yml
mkdir -p "$output"
awk -v url="$URL" -v tag="$tag" -v commit="$commit" '
    /^ *path: \.\.\/\.\.\/\.\.$/ { sub(/path: .*/, "url: " url); print; found++; next }
    /^ *branch: main$/ {
        indent = $0; sub(/branch: main/, "", indent)
        print indent "tag: " tag; print indent "commit: " commit; found++; next
    }
    /# The last commit of main/ { next }
    { print }
    END { if (found != 2) exit 1 }
' "$manifest" > "$output/$APP_ID.yml" || {
    echo "Unexpected source block in $manifest." >&2
    exit 1
}
cp installer/linux/flatpak/cargo-sources.json "$output/"
echo "$output/$APP_ID.yml"
