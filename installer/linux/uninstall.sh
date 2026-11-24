#!/bin/sh
# Removes the per-user installation made by install.sh.
# With --purge, also removes data created by the native application.
set -eu

APP_ID=io.github.Carlton2001.Soliton
data=${XDG_DATA_HOME:-$HOME/.local/share}
entry=$data/applications/$APP_ID.desktop
root=$data/Soliton

say() {
    case ${LC_ALL:-${LC_MESSAGES:-${LANG:-C}}} in
        fr*) printf '%s\n' "$1" ;;
        *) printf '%s\n' "$2" ;;
    esac
}

purge_covers() {
    if [ -d "$1" ] && [ ! -L "$1" ]; then
        # Match the content-addressed thumbnails and temporary names in covers.rs.
        LC_ALL=C find -P "$1" -maxdepth 1 -regextype posix-extended -type f \
            -regex '.*/[0-9a-f]{64}(\.small)?\.png(\.[0-9]+\.[0-9]+\.tmp)?' -delete
        rmdir --ignore-fail-on-non-empty -- "$1"
    fi
}

purge_data() {
    if [ ! -d "$root" ]; then
        return
    fi
    find -P "$root" -maxdepth 1 -type f \( \
        -name library.sqlite3 -o -name library.sqlite3-wal -o \
        -name library.sqlite3-shm -o -name library.sqlite3-journal -o \
        -name settings.json -o -name settings.json.tmp \) -delete
    if [ ! -L "$root/covers" ]; then
        purge_covers "$root/covers/loose"
        purge_covers "$root/covers"
    fi
    rmdir --ignore-fail-on-non-empty -- "$root"
}

if [ "$#" -gt 1 ] || { [ "$#" -eq 1 ] && [ "$1" != --purge ]; }; then
    say "Utilisation : $0 [--purge]" "Usage: $0 [--purge]" >&2
    exit 2
fi
if [ "${1-}" = --purge ]; then
    if [ "$(id -u)" -eq 0 ]; then
        say "Lancez la purge sans sudo." "Run the purge without sudo." >&2
        exit 1
    fi
    case $root in
        /*) ;;
        *)
            say "La purge exige un chemin absolu : $root" \
                "Purging requires an absolute path: $root" >&2
            exit 1
            ;;
    esac
    if [ -L "$root" ]; then
        say "La purge exige un dossier Soliton sans lien symbolique : $root" \
            "Purging requires a Soliton directory that is not a symbolic link: $root" >&2
        exit 1
    fi
fi

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
if [ "${1-}" = --purge ]; then
    purge_data
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

say "Soliton a été désinstallé." "Soliton was uninstalled."
if [ "${1-}" = --purge ]; then
    say "Les données natives de Soliton ont été supprimées de $root." \
        "Soliton's native data has been removed from $root."
else
    say "Bibliothèque, réglages et playlists conservés dans $root." \
        "Library, settings and playlists kept in $root."
fi
