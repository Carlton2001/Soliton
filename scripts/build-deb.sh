#!/bin/sh
# Builds the .deb in an Ubuntu 24.04 environment (glibc 2.39), then checks it
# with lintian. Elsewhere, reruns itself in a podman container.
# Usage: scripts/build-deb.sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
APP_ID=io.github.Carlton2001.Soliton
IMAGE=localhost/soliton-build-ubuntu:24.04
# Debian requires a contact address; GitHub's no-reply form by default.
maintainer=${SOLITON_MAINTAINER:-Carlton2001 <carlton2001@gmail.com>}

. /etc/os-release
if [ "$ID:$VERSION_ID" != "ubuntu:24.04" ]; then
    podman build -q -t "$IMAGE" installer/linux/deb >/dev/null
    podman volume create --ignore soliton-ubuntu-cargo >/dev/null
    exec podman run --rm --userns=keep-id -e HOME=/tmp \
        -e SOLITON_MAINTAINER="$maintainer" \
        -v "$root:/src:Z" -v soliton-ubuntu-cargo:/opt/cargo/registry:U \
        -w /src "$IMAGE" sh scripts/build-deb.sh
fi

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
# A target directory of its own: these objects link against Ubuntu's libraries.
target=$root/target/ubuntu-24.04
cargo build --locked --release -p soliton-app --bin Soliton --target-dir "$target"

package=soliton_$version-1_amd64
stage=$root/target/distribution/deb/$package
output=$root/target/distribution/output
rm -rf "$root/target/distribution/deb"
mkdir -p "$stage/DEBIAN" "$output"
doc=$stage/usr/share/doc/soliton
install -Dm755 -s "$target/release/Soliton" "$stage/usr/bin/soliton"
# The per-user marker would let Soliton rewrite the icon; system entries stay fixed.
mkdir -p "$stage/usr/share/applications"
sed -e 's/^Exec=@EXEC@$/Exec=soliton/' -e '/^X-Soliton-Install=user$/d' \
    "installer/linux/$APP_ID.desktop" > "$stage/usr/share/applications/$APP_ID.desktop"
install -Dm644 "installer/linux/$APP_ID.metainfo.xml" "$stage/usr/share/metainfo/$APP_ID.metainfo.xml"
install -Dm644 assets/logos/graphite-mint.svg "$stage/usr/share/icons/hicolor/scalable/apps/$APP_ID.svg"
install -Dm644 installer/linux/deb/copyright "$doc/copyright"
mkdir -p "$doc"
cp -R licenses/dependencies "$doc/dependencies"
date=$(LC_ALL=C date -u -R -d "@$(git -c safe.directory="$root" log -1 --format=%ct)")
printf 'soliton (%s-1) unstable; urgency=medium\n\n  * Soliton %s; release notes in the AppStream metadata.\n\n -- %s  %s\n' \
    "$version" "$version" "$maintainer" "$date" | gzip -9n > "$doc/changelog.Debian.gz"
chmod -R u=rwX,go=rX "$stage"

desktop-file-validate "$stage/usr/share/applications/$APP_ID.desktop"
appstreamcli validate --no-net "$stage/usr/share/metainfo/$APP_ID.metainfo.xml"

# dpkg-shlibdeps reads a debian/control of the source package.
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/debian"
printf 'Source: soliton\n\nPackage: soliton\nArchitecture: amd64\n' > "$work/debian/control"
detected=$(cd "$work" && dpkg-shlibdeps -O "$stage/usr/bin/soliton" | sed 's/^shlibs:Depends=//')
# Loaded at run time, so not detected from the binary; Mesa provides lavapipe.
# pipewire-bin ships the client.conf of libpipewire; a system font covers the
# characters missing from the embedded fonts.
depends="$detected, pipewire-bin, libvulkan1, mesa-vulkan-drivers, libwayland-client0"
depends="$depends, fonts-dejavu-core | fonts-noto-core | fonts-cantarell"
size=$(du -sk --exclude=DEBIAN "$stage" | cut -f1)
# '#' as delimiter: alternatives in Depends use '|'.
sed -e "s#@VERSION@#$version-1#" -e "s#@SIZE@#$size#" -e "s#@DEPENDS@#$depends#" \
    -e "s#@MAINTAINER@#$maintainer#" installer/linux/deb/control > "$stage/DEBIAN/control"

SOURCE_DATE_EPOCH=$(git -c safe.directory="$root" log -1 --format=%ct) \
    dpkg-deb --root-owner-group -Zxz --build "$stage" "$output/$package.deb" >/dev/null
lintian --tag-display-limit 0 "$output/$package.deb" || true
(cd "$output" && sha256sum "$package.deb" > "$package.deb.sha256")
echo "target/distribution/output/$package.deb"
