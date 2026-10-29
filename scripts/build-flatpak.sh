#!/bin/sh
# Builds the Flatpak from the last commit with org.flatpak.Builder, lints it,
# and exports a single-file bundle for local installation.
# Usage: scripts/build-flatpak.sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
APP_ID=io.github.Carlton2001.Soliton
MANIFEST=installer/linux/flatpak/$APP_ID.yml
# flatpak-builder runs natively: inside the org.flatpak.Builder sandbox it does
# not see the SDK extensions of the user installation. That Flatpak still
# provides flatpak-builder-lint.
lint() { flatpak run --command=flatpak-builder-lint org.flatpak.Builder "$@"; }
if ! command -v flatpak-builder >/dev/null 2>&1 || ! flatpak info org.flatpak.Builder >/dev/null 2>&1; then
    echo "Install the tools: sudo dnf install flatpak-builder;" \
        "flatpak install --user flathub org.flatpak.Builder" >&2
    exit 1
fi
if ! git diff --quiet HEAD -- . ':!installer/linux/flatpak'; then
    echo "Note: uncommitted changes are not part of the build, which uses the last commit." >&2
fi
python3 -I scripts/flatpak-cargo-sources.py >/dev/null
version=$(cargo pkgid -p soliton-app | sed 's/.*[#@]//')

work=$root/target/flatpak
output=$root/target/distribution/output
mkdir -p "$work" "$output"
flatpak install --user -y --noninteractive flathub \
    org.freedesktop.Sdk//26.08 org.freedesktop.Platform//26.08 \
    org.freedesktop.Sdk.Extension.rust-stable//26.08 \
    org.freedesktop.Sdk.Extension.llvm22//26.08 >/dev/null
flatpak-builder --user --force-clean \
    --state-dir="$work/state" --repo="$work/repo" "$work/build" "$MANIFEST"
lint manifest "$MANIFEST" || true
lint repo "$work/repo" || true
bundle=Soliton-$version-x86_64.flatpak
flatpak build-bundle --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo \
    "$work/repo" "$output/$bundle" "$APP_ID"
(cd "$output" && sha256sum "$bundle" > "$bundle.sha256")
echo "target/distribution/output/$bundle"
