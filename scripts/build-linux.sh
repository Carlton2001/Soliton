#!/bin/sh
# Builds the Linux x86_64 archive: release binary, desktop entry, AppStream
# metadata, icon, licenses and per-user install/uninstall scripts.
# Usage: scripts/build-linux.sh [--skip-build]
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
APP_ID=io.github.Carlton2001.Soliton
skip_build=false
case "${1:-}" in
    --skip-build) skip_build=true ;;
    "") ;;
    *) echo "Usage: $0 [--skip-build]" >&2; exit 2 ;;
esac

sh scripts/check-third-party-licenses.sh
version=$(cargo pkgid -p soliton-app | sed 's/.*[#@]//')
case "$version" in
    [0-9]*.[0-9]*.[0-9]*) ;;
    *) echo "Expected a numeric major.minor.patch version, got $version." >&2; exit 1 ;;
esac
if ! grep -q "<release version=\"$version\"" "installer/linux/$APP_ID.metainfo.xml"; then
    echo "Add release $version to installer/linux/$APP_ID.metainfo.xml." >&2
    exit 1
fi
if [ "$skip_build" = false ]; then
    cargo build --locked --release -p soliton-app --bin Soliton
fi

name=Soliton-$version-x86_64-linux
output=$root/target/distribution/output
stage=$root/target/distribution/linux/$name
rm -rf "$root/target/distribution/linux"
mkdir -p "$stage" "$output"
install -Dm755 -s target/release/Soliton "$stage/bin/soliton"
install -Dm644 "installer/linux/$APP_ID.desktop" "$stage/share/applications/$APP_ID.desktop"
install -Dm644 "installer/linux/$APP_ID.metainfo.xml" "$stage/share/metainfo/$APP_ID.metainfo.xml"
install -Dm644 assets/logos/graphite-mint.svg "$stage/share/icons/hicolor/scalable/apps/$APP_ID.svg"
install -Dm755 installer/linux/install.sh "$stage/install.sh"
install -Dm755 installer/linux/uninstall.sh "$stage/uninstall.sh"
install -Dm644 installer/linux/README.txt "$stage/README.txt"
install -Dm644 LICENSE "$stage/LICENSE"
mkdir -p "$stage/licenses"
cp -R licenses/dependencies "$stage/licenses/dependencies"

archive=$output/$name.tar.gz
tar -C "$(dirname "$stage")" --owner=0 --group=0 --numeric-owner -czf "$archive" "$name"
# Every staged file must be present in the archive with its size.
tar -tvzf "$archive" > "$stage.list"
(cd "$(dirname "$stage")" && find "$name" -type f) | while read -r file; do
    size=$(stat -c %s "$(dirname "$stage")/$file")
    if ! grep -qE "^-\S+ +\S+ +$size .* $file\$" "$stage.list"; then
        echo "Missing or truncated archive entry: $file" >&2
        exit 1
    fi
done
(cd "$output" && sha256sum "$name.tar.gz" > "$name.tar.gz.sha256")
echo "$archive"
