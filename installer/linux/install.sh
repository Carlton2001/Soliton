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

# Based on the dependency checks in Oxyde FM (Carlton2001, GPL-3.0).
has_library() {
    case "$library_cache" in *"$1 "*) return 0 ;; esac
    for dir in /usr/lib64 /usr/lib /usr/lib/x86_64-linux-gnu /lib64 /lib/x86_64-linux-gnu; do
        [ ! -e "$dir/$1" ] || return 0
    done
    return 1
}

dependency_present() {
    case $1 in
        cmd:*) command -v "${1#cmd:}" >/dev/null 2>&1 ;;
        lib:*) has_library "${1#lib:}" ;;
        vulkan)
            for manifest in /usr/share/vulkan/icd.d/*.json /etc/vulkan/icd.d/*.json; do
                [ ! -f "$manifest" ] || return 0
            done
            return 1
            ;;
        portal)
            [ -f /usr/share/dbus-1/services/org.freedesktop.portal.Desktop.service ]
            ;;
    esac
}

check_dependencies() {
    family=$(
        ID= ID_LIKE=
        if [ -r /etc/os-release ]; then
            . /etc/os-release
        elif [ -r /usr/lib/os-release ]; then
            . /usr/lib/os-release
        fi
        for id in $ID $ID_LIKE; do
            case $id in
                debian|ubuntu) echo debian; break ;;
                fedora|rhel|centos) echo fedora; break ;;
                arch) echo arch; break ;;
                suse|opensuse*) echo suse; break ;;
            esac
        done
    )
    library_cache=
    for cache_reader in ldconfig /sbin/ldconfig /usr/sbin/ldconfig; do
        library_cache=$(LC_ALL=C "$cache_reader" -p 2>/dev/null) && break
    done

    required=
    recommended=
    packages=
    # Level, probe, feature in French and English, then distribution packages.
    while IFS='|' read -r level probe fr en fedora debian arch suse; do
        dependency_present "$probe" && continue
        case $family in
            debian) names=$debian ;;
            arch) names=$arch ;;
            suse) names=$suse ;;
            *) names=$fedora ;;
        esac
        item=$(say "  - $fr ($names)" "  - $en ($names)")
        case $level in
            required) required="$required
$item" ;;
            recommended) recommended="$recommended
$item" ;;
        esac
        for name in $names; do
            case " $packages " in
                *" $name "*) ;;
                *) packages="${packages:+$packages }$name" ;;
            esac
        done
    done <<'EOF'
required|cmd:flock|Verrouillage de l’installation|Installation locking|util-linux|util-linux|util-linux|util-linux
required|lib:libvulkan.so.1|Chargeur Vulkan|Vulkan loader|vulkan-loader|libvulkan1|vulkan-icd-loader|libvulkan1
required|vulkan|Pilote Vulkan|Vulkan driver|mesa-vulkan-drivers|mesa-vulkan-drivers|vulkan-swrast|libvulkan_lvp
required|lib:libpipewire-0.3.so.0|Lecture audio PipeWire|PipeWire audio playback|pipewire-libs|libpipewire-0.3-0 pipewire-bin|pipewire|libpipewire-0_3-0
required|lib:libwayland-client.so.0|Fenêtres Wayland|Wayland windows|libwayland-client|libwayland-client0|wayland|libwayland-client0
required|lib:libxkbcommon.so.0|Clavier|Keyboard input|libxkbcommon|libxkbcommon0|libxkbcommon|libxkbcommon0
required|lib:libxkbcommon-x11.so.0|Clavier sous X11|X11 keyboard input|libxkbcommon-x11|libxkbcommon-x11-0|libxkbcommon-x11|libxkbcommon-x11-0
required|lib:libxcb.so.1|Fenêtres X11|X11 windows|libxcb|libxcb1|libxcb|libxcb1
required|lib:libfontconfig.so.1|Polices système|System fonts|fontconfig|libfontconfig1|fontconfig|libfontconfig1
recommended|portal|Sélection des dossiers|Folder picker|xdg-desktop-portal|xdg-desktop-portal|xdg-desktop-portal|xdg-desktop-portal
recommended|cmd:curl|Mises à jour depuis Soliton|Updates from Soliton|curl|curl|curl|curl
EOF
    say "Composants système" "System components"
    if [ -z "$packages" ]; then
        say "Tous les composants vérifiés sont présents." "All checked components are present."
        return 0
    fi
    if [ -n "$required" ]; then
        say "Requis pour l’installation ou le fonctionnement de Soliton$required" \
            "Required to install or run Soliton$required"
    fi
    if [ -n "$recommended" ]; then
        say "Recommandés pour les fonctions indiquées$recommended" \
            "Recommended for the listed features$recommended"
    fi

    if [ -z "$family" ]; then
        say "Les noms affichés sont ceux des paquets Fedora. Installez les équivalents de votre distribution." \
            "The listed names are Fedora packages. Install the equivalents for your distribution."
        return 0
    fi

    # Package names come from the table above. Split them into command arguments.
    set -- $packages
    atomic=false
    case $family in
        debian) set -- sudo apt-get install "$@" ;;
        fedora)
            if [ -e /run/ostree-booted ]; then
                atomic=true
                set -- rpm-ostree install "$@"
            else
                set -- sudo dnf install "$@"
            fi
            ;;
        arch) set -- sudo pacman -S --needed "$@" ;;
        suse) set -- sudo zypper install "$@" ;;
    esac
    say "Commande à exécuter pour ajouter les composants manquants :" \
        "Command to add the missing components:"
    printf '    %s\n' "$*"
    if [ "$atomic" = true ]; then
        say "Ces paquets seront ajoutés à l’image système et disponibles après redémarrage." \
            "These packages will be added to the system image and available after a restart."
    fi

    # The in-app updater and redirected input must never start a package manager.
    if [ -t 0 ] && [ -t 2 ]; then
        say "Ajouter les composants manquants maintenant ? [o/N]" \
            "Add the missing components now? [y/N]"
        IFS= read -r answer || answer=
        case $answer in
            o|O|y|Y)
                if "$@"; then
                    say "Commande terminée avec succès." "Package manager completed successfully."
                    return 0
                fi
                say "La commande a échoué. Vous pouvez la relancer manuellement." \
                    "The command failed. You can run it manually."
                ;;
        esac
    fi
    if [ -n "$required" ]; then
        say "Certaines fonctions de Soliton peuvent être indisponibles tant que ces composants manquent." \
            "Some Soliton functionality may be unavailable until these components are installed."
    fi
}

if [ "$action" != --update ]; then
    check_dependencies >&2
fi
if ! command -v flock >/dev/null 2>&1; then
    say "Installez flock (util-linux), puis relancez ce script." \
        "Install flock (util-linux), then run this script again." >&2
    exit 1
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
