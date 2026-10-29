#!/bin/sh
# Removes the per-user installation made by install.sh. The library, settings,
# playlists and caches in ~/.local/share/Soliton are kept.
set -eu

APP_ID=io.github.Carlton2001.Soliton
data=${XDG_DATA_HOME:-$HOME/.local/share}
entry=$data/applications/$APP_ID.desktop

if [ -f "$entry" ]; then
    if ! grep -qx 'X-Soliton-Install=user' "$entry"; then
        echo "$entry was not installed by install.sh; nothing is removed." >&2
        exit 1
    fi
# Without the entry, the licenses install.sh copies show it ran: another
# ~/.local/bin/soliton is never removed.
elif [ ! -f "$data/doc/soliton/LICENSE" ]; then
    echo "No installation made by install.sh was found; nothing is removed."
    exit 0
fi
rm -f "$HOME/.local/bin/soliton" "$entry" \
    "$data/metainfo/$APP_ID.metainfo.xml" \
    "$data/icons/hicolor/scalable/apps/$APP_ID.svg"
rm -rf "$data/doc/soliton"
if [ -d "$data/icons/hicolor" ]; then
    touch "$data/icons/hicolor"
fi
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q "$data/applications" || true
fi

echo "Soliton is uninstalled."
echo "Your library, settings and playlists remain in ${XDG_DATA_HOME:-$HOME/.local/share}/Soliton."
