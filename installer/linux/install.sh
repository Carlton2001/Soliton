#!/bin/sh
# Installs Soliton for the current user, without root, from the extracted archive.
# The in-app updater invokes --update with the running Soliton PID.
set -eu

APP_ID=io.github.Carlton2001.Soliton
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
data=${XDG_DATA_HOME:-$HOME/.local/share}
bin=$HOME/.local/bin
doc=$data/doc/soliton

say() {
    case ${LC_ALL:-${LC_MESSAGES:-${LANG:-C}}} in
        fr*) printf '%s\n' "$1" ;;
        *) printf '%s\n' "$2" ;;
    esac
}

usage() {
    say "Utilisation : $0 [--install | --uninstall [--purge] | --help]" \
        "Usage: $0 [--install | --uninstall [--purge] | --help]"
}

if [ "$#" -gt 2 ] || { [ "$#" -eq 2 ] && [ -z "$2" ]; }; then
    usage >&2
    exit 2
fi
case ${1-}:${2-} in
    --help:) usage; exit 0 ;;
    :|--install:|--uninstall:|--uninstall:--purge) ;;
    --update:*)
        case ${2-} in ''|0|*[!0-9]*) exit 2 ;; esac
        ;;
    *) usage >&2; exit 2 ;;
esac

if [ "$(id -u)" -eq 0 ]; then
    say "Lancez ce script depuis votre compte utilisateur, sans sudo." \
        "Run this script as your own user, without sudo." >&2
    exit 1
fi

action=${1-}
option=${2-}
if [ -z "$action" ] && [ -t 0 ] && [ -t 1 ]; then
    entry=$data/applications/$APP_ID.desktop
    installed=false
    if [ -f "$entry" ]; then
        if [ -n "$(sed -n '/^X-Soliton-Install=user$/p' "$entry")" ]; then
            installed=true
        fi
    elif [ -f "$doc/LICENSE" ]; then
        installed=true
    fi

    printf '\nSoliton\n'
    if [ "$installed" = true ]; then
        say "1) Mettre à jour ou réinstaller Soliton" "1) Update or reinstall Soliton"
        say "2) Désinstaller Soliton (conserver les données)" "2) Uninstall Soliton (keep user data)"
        say "3) Tout désinstaller (bibliothèque, playlists, réglages et cache inclus)" \
            "3) Uninstall everything (including library, playlists, settings and cache)"
    else
        say "1) Installer Soliton" "1) Install Soliton"
    fi
    say "0) Quitter" "0) Quit"
    while :; do
        say "Choisissez une action :" "Choose an action:"
        IFS= read -r choice || exit 0
        case $choice in
            1) break ;;
            2|3)
                if [ "$installed" = true ]; then
                    if [ "$choice" = 3 ]; then
                        say "Effacer définitivement les données de Soliton dans $data/Soliton ? [o/N]" \
                            "Permanently delete Soliton data in $data/Soliton? [y/N]"
                        IFS= read -r confirm || exit 0
                        case $confirm in
                            o|O|y|Y) ;;
                            *) exit 0 ;;
                        esac
                        option=--purge
                    fi
                    action=--uninstall
                    break
                fi
                ;;
            0|'') exit 0 ;;
        esac
        say "Choix invalide." "Invalid choice."
    done
fi
if [ "$action" = --uninstall ]; then
    if [ "$option" = --purge ]; then
        exec sh "$here/uninstall.sh" --purge
    fi
    exec sh "$here/uninstall.sh"
fi

for file in bin/soliton "share/applications/$APP_ID.desktop" \
    "share/metainfo/$APP_ID.metainfo.xml" \
    "share/icons/hicolor/scalable/apps/$APP_ID.svg" LICENSE; do
    if [ ! -f "$here/$file" ]; then
        echo "Incomplete archive: $file is missing." >&2
        exit 1
    fi
done
if [ ! -d "$here/licenses" ]; then
    say "Archive incomplète. Les licences sont manquantes." \
        "The archive is incomplete. The licenses are missing." >&2
    exit 1
fi

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

mkdir -p "$bin"
# Serialize archive installations without leaving a lock file behind.
exec 9<"$bin"
if ! flock -n -x 9; then
    say "Impossible de verrouiller l’installation de Soliton." \
        "Could not lock the Soliton installation." >&2
    exit 1
fi
check_update_target() {
    if [ "$action" = --update ] && {
        ! grep -qx 'X-Soliton-Install=user' "$data/applications/$APP_ID.desktop" ||
            [ -L "$bin/soliton" ] || [ ! "$bin/soliton" -ef "/proc/$option/exe" ];
    }; then
        say "L’installation de Soliton a changé. Relancez l’application." \
            "The Soliton installation changed. Relaunch the application." >&2
        exit 1
    fi
}
check_update_target

# Exec takes one quoted argument: escape " ` $ \ for the quoting, then \ again
# for the string value, and double % (Desktop Entry specification).
exec_line=$(printf '%s' "$bin/soliton" | sed -e 's/[\\"`$]/\\&/g' -e 's/\\/\\\\/g' -e 's/%/%%/g')
entry=$(mktemp)
executable=
license_stage=
trap 'rm -f "$entry" ${executable:+"$executable"}; if [ -n "$license_stage" ]; then rm -rf "$license_stage"; fi' EXIT
executable=$(mktemp "$bin/.soliton.XXXXXXXX")
install -m755 "$here/bin/soliton" "$executable"
mkdir -p "$doc"
license_stage=$(mktemp -d "$doc/.licenses.XXXXXXXX")
cp -R "$here/licenses" "$license_stage/licenses"
while IFS= read -r line || [ -n "$line" ]; do
    if [ "$line" = "Exec=@EXEC@" ]; then
        printf 'Exec="%s"\n' "$exec_line"
    else
        printf '%s\n' "$line"
    fi
done < "$here/share/applications/$APP_ID.desktop" > "$entry"
install -DTm644 "$entry" "$data/applications/$APP_ID.desktop"
install -DTm644 "$here/share/metainfo/$APP_ID.metainfo.xml" "$data/metainfo/$APP_ID.metainfo.xml"
# Soliton replaces this icon with the palette chosen in its settings.
if [ ! -f "$data/icons/hicolor/scalable/apps/$APP_ID.svg" ]; then
    install -DTm644 "$here/share/icons/hicolor/scalable/apps/$APP_ID.svg" \
        "$data/icons/hicolor/scalable/apps/$APP_ID.svg"
fi
touch "$data/icons/hicolor"
install -DTm644 "$here/LICENSE" "$doc/LICENSE"
rm -rf "$doc/licenses"
mv -T "$license_stage/licenses" "$doc/licenses"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q "$data/applications" || true
fi

# Keep the old executable usable if installing any resources fails.
check_update_target
mv -fT "$executable" "$bin/soliton"

echo "Soliton is installed: $bin/soliton"
echo "It appears in the applications overview. Remove it with uninstall.sh."
case ":$PATH:" in
    *":$bin:"*) ;;
    *) echo "Note: $bin is not in PATH; the launcher still works." ;;
esac
