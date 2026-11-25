# The release workflow prepends the repository, version and Minisign public key.
# Distribution selection follows the Oxyde FM installer (Carlton2001, GPL-3.0).
set -eu

say() {
    case ${LC_ALL:-${LC_MESSAGES:-${LANG:-C}}} in
        fr*) printf '%s\n' "$1" ;;
        *) printf '%s\n' "$2" ;;
    esac
}

fail() {
    say "$@" >&2
    exit 1
}

is_debian_family() (
    ID= ID_LIKE=
    if [ -r /etc/os-release ]; then
        . /etc/os-release
    elif [ -r /usr/lib/os-release ]; then
        . /usr/lib/os-release
    fi
    case " $ID $ID_LIKE " in
        *" debian "*|*" ubuntu "*) exit 0 ;;
        *) exit 1 ;;
    esac
)

installed_from_archive() {
    entry=${XDG_DATA_HOME:-$HOME/.local/share}/applications/io.github.Carlton2001.Soliton.desktop
    [ -x "$HOME/.local/bin/soliton" ] && grep -qx 'X-Soliton-Install=user' "$entry" 2>/dev/null
}

download() {
    curl -q --fail --silent --show-error --location --proto '=https' \
        --proto-redir '=https' --output "$1" "$release/$1"
}

download_verified() {
    download "$1.sig"
    base64 -d "$1.sig" > asset.minisig
    download "$1"
    if ! comment=$(minisign -VQ -P "$public_key" -m "$1" -x asset.minisig) ||
        [ "$comment" != "soliton-version:$version" ]; then
        fail "La signature du fichier Soliton est invalide." "The Soliton file signature is invalid."
    fi
}

install_deb() {
    package=soliton_${version}-1_amd64.deb
    download_verified "$package"
    # APT's sandbox user needs access to the public package.
    chmod 755 "$work"
    chmod 644 "$package"
    say "Installation du paquet .deb avec APT." "Installing the .deb package with APT."
    if [ -t 1 ] && { : < /dev/tty; } 2>/dev/null; then
        sudo apt-get install "$work/$package" < /dev/tty
    else
        sudo apt-get install "$work/$package" < /dev/null
    fi
}

# Read the complete function before running when the script is piped to sh.
main() {
    if [ "$(uname -s):$(uname -m)" != Linux:x86_64 ]; then
        fail "Ce script nécessite Linux x86_64." "This script requires Linux x86_64."
    fi
    if [ "$(id -u)" -eq 0 ]; then
        fail "Lancez ce script sans sudo." "Run this script without sudo."
    fi
    deb=false
    if is_debian_family && ! installed_from_archive; then
        deb=true
        set -- sudo apt-get
    else
        set -- tar
    fi
    for tool in curl minisign base64 "$@"; do
        command -v "$tool" >/dev/null 2>&1 ||
            fail "Installez $tool puis relancez le script." "Install $tool and run the script again."
    done

    work=$(mktemp -d)
    trap 'rm -rf -- "$work"' EXIT
    trap 'exit 1' HUP INT TERM
    cd "$work"

    release=https://github.com/$repository/releases/download/v$version
    if [ "$deb" = true ]; then
        install_deb
        return
    fi

    name=Soliton-$version-x86_64-linux
    archive=$name.tar.gz
    download_verified "$archive"
    tar -xzf "$archive"
    if [ -t 0 ] && [ -t 1 ]; then
        sh "$name/install.sh"
    elif [ -t 1 ] && { : < /dev/tty; } 2>/dev/null; then
        sh "$name/install.sh" < /dev/tty
    else
        sh "$name/install.sh" --install
    fi
}

main
