#!/bin/sh
# Builds the Fedora RPM from the Linux archive, then checks its content.
# Usage: scripts/build-rpm.sh [--skip-build]
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
case "${1:-}" in
    --skip-build | "") ;;
    *) echo "Usage: $0 [--skip-build]" >&2; exit 2 ;;
esac
for tool in rpmbuild desktop-file-validate appstreamcli; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "$tool is missing: sudo dnf install rpm-build desktop-file-utils appstream" >&2
        exit 1
    fi
done

# Captured whole: set -e stops on a failed archive build, not on its last line.
archive=$(sh scripts/build-linux.sh ${1:+"$1"})
archive=$(printf '%s\n' "$archive" | tail -n 1)
version=$(cargo pkgid -p soliton-app | sed 's/.*[#@]//')
output=$root/target/distribution/output
top=$root/target/distribution/rpm
rm -rf "$top"
mkdir -p "$top/SOURCES"
cp "$archive" "$top/SOURCES/"
date=$(LC_ALL=C date -u -d "@$(git log -1 --format=%ct)" '+%a %b %d %Y')
license=$(python3 -I scripts/rpm-license.py)
rpmbuild -bb --quiet \
    --define "soliton_date $date" \
    --define "soliton_license $license" \
    --define "_topdir $top" \
    --define "soliton_version $version" \
    installer/linux/rpm/soliton.spec

rpm=$(find "$top/RPMS" -name "soliton-$version-*.x86_64.rpm")
for file in /usr/bin/soliton \
    /usr/share/applications/io.github.Carlton2001.Soliton.desktop \
    /usr/share/metainfo/io.github.Carlton2001.Soliton.metainfo.xml \
    /usr/share/icons/hicolor/scalable/apps/io.github.Carlton2001.Soliton.svg \
    /usr/share/licenses/soliton/LICENSE; do
    if ! rpm -qlp "$rpm" | grep -qx "$file"; then
        echo "Missing package entry: $file" >&2
        exit 1
    fi
done
if command -v rpmlint >/dev/null 2>&1; then
    rpmlint "$rpm" || true
fi
cp "$rpm" "$output/"
name=$(basename "$rpm")
(cd "$output" && sha256sum "$name" > "$name.sha256")
echo "$output/$name"
