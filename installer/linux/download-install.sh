# The release workflow prepends the repository, version and Minisign public key.
set -eu

fail() {
    case ${LC_ALL:-${LC_MESSAGES:-${LANG:-C}}} in
        fr*) printf '%s\n' "$1" >&2 ;;
        *) printf '%s\n' "$2" >&2 ;;
    esac
    exit 1
}

# Read the complete function before running when the script is piped to sh.
main() {
    if [ "$(uname -s):$(uname -m)" != Linux:x86_64 ]; then
        fail "Ce script nécessite Linux x86_64." "This script requires Linux x86_64."
    fi
    if [ "$(id -u)" -eq 0 ]; then
        fail "Lancez ce script sans sudo." "Run this script without sudo."
    fi
    for tool in curl minisign tar base64; do
        command -v "$tool" >/dev/null 2>&1 ||
            fail "Installez $tool puis relancez le script." "Install $tool and run the script again."
    done

    work=$(mktemp -d)
    trap 'rm -rf -- "$work"' EXIT
    trap 'exit 1' HUP INT TERM
    cd "$work"

    name=Soliton-$version-x86_64-linux
    archive=$name.tar.gz
    release=https://github.com/$repository/releases/download/v$version

    download() {
        curl -q --fail --silent --show-error --location --proto '=https' \
            --proto-redir '=https' --output "$1" "$release/$1"
    }

    download "$archive.sig"
    base64 -d "$archive.sig" > archive.minisig
    download "$archive"
    if ! comment=$(minisign -VQ -P "$public_key" -m "$archive" -x archive.minisig) ||
        [ "$comment" != "soliton-version:$version" ]; then
        fail "La signature de l’archive Soliton est invalide." "The Soliton archive signature is invalid."
    fi

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
