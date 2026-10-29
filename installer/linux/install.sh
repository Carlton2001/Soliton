#!/bin/sh
# Installs Soliton for the current user, without root, from the extracted archive.
# Run it again to update. User data in ~/.local/share/Soliton is never touched.
set -eu

APP_ID=io.github.Carlton2001.Soliton
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
data=${XDG_DATA_HOME:-$HOME/.local/share}
bin=$HOME/.local/bin
doc=$data/doc/soliton

if [ "$(id -u)" -eq 0 ]; then
    echo "Run this script as your own user, not as root." >&2
    exit 1
fi
for file in bin/soliton "share/applications/$APP_ID.desktop" \
    "share/metainfo/$APP_ID.metainfo.xml" \
    "share/icons/hicolor/scalable/apps/$APP_ID.svg" LICENSE; do
    if [ ! -f "$here/$file" ]; then
        echo "Incomplete archive: $file is missing." >&2
        exit 1
    fi
done

# GPUI renders through Vulkan; the loader is a system package.
vulkan=
for dir in /usr/lib64 /usr/lib /usr/lib/x86_64-linux-gnu /lib64 /lib/x86_64-linux-gnu; do
    [ -e "$dir/libvulkan.so.1" ] && vulkan=1
done
if [ -z "$vulkan" ] && command -v ldconfig >/dev/null 2>&1; then
    case $(ldconfig -p 2>/dev/null) in *libvulkan.so.1*) vulkan=1 ;; esac
fi
if [ -z "$vulkan" ]; then
    echo "Warning: libvulkan.so.1 was not found; Soliton needs Vulkan to start." >&2
    echo "Install vulkan-loader and mesa-vulkan-drivers (Fedora) or libvulkan1 and mesa-vulkan-drivers (Debian, Ubuntu)." >&2
fi

install -Dm755 "$here/bin/soliton" "$bin/soliton"
# Exec takes one quoted argument: escape " ` $ \ for the quoting, then \ again
# for the string value, and double % (Desktop Entry specification).
exec_line=$(printf '%s' "$bin/soliton" | sed -e 's/[\\"`$]/\\&/g' -e 's/\\/\\\\/g' -e 's/%/%%/g')
entry=$(mktemp)
trap 'rm -f "$entry"' EXIT
while IFS= read -r line || [ -n "$line" ]; do
    if [ "$line" = "Exec=@EXEC@" ]; then
        printf 'Exec="%s"\n' "$exec_line"
    else
        printf '%s\n' "$line"
    fi
done < "$here/share/applications/$APP_ID.desktop" > "$entry"
install -Dm644 "$entry" "$data/applications/$APP_ID.desktop"
install -Dm644 "$here/share/metainfo/$APP_ID.metainfo.xml" "$data/metainfo/$APP_ID.metainfo.xml"
# Soliton replaces this icon with the palette chosen in its settings.
if [ ! -f "$data/icons/hicolor/scalable/apps/$APP_ID.svg" ]; then
    install -Dm644 "$here/share/icons/hicolor/scalable/apps/$APP_ID.svg" \
        "$data/icons/hicolor/scalable/apps/$APP_ID.svg"
fi
touch "$data/icons/hicolor"
install -Dm644 "$here/LICENSE" "$doc/LICENSE"
rm -rf "$doc/licenses"
mkdir -p "$doc"
cp -R "$here/licenses" "$doc/licenses"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q "$data/applications" || true
fi

echo "Soliton is installed: $bin/soliton"
echo "It appears in the applications overview. Remove it with uninstall.sh."
case ":$PATH:" in
    *":$bin:"*) ;;
    *) echo "Note: $bin is not in PATH; the launcher still works." ;;
esac
