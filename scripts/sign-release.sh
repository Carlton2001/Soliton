#!/bin/sh
# Signs the release files in place: Minisign for the Windows installer read by
# the updater, an embedded GPG signature for the RPM and a detached one (.asc)
# for every other package. Each signature is checked against the public keys
# committed in installer/signing before the script succeeds.
# Needs minisign, gpg, rpmsign and base64. Keys come from the environment:
#   SOLITON_UPDATE_PRIVATE_KEY, SOLITON_UPDATE_PRIVATE_KEY_PASSWORD (Minisign)
#   SOLITON_GPG_PRIVATE_KEY, SOLITON_GPG_PASSPHRASE (armored secret key)
# Usage: scripts/sign-release.sh <version> <directory>
set -eu

if [ $# -ne 2 ]; then
    echo "Usage: $0 <version> <directory>" >&2
    exit 2
fi
version=$1
dir=$2
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
keys=$root/installer/signing
for name in SOLITON_UPDATE_PRIVATE_KEY SOLITON_UPDATE_PRIVATE_KEY_PASSWORD \
    SOLITON_GPG_PRIVATE_KEY SOLITON_GPG_PASSPHRASE; do
    if [ -z "$(printenv "$name" || true)" ]; then
        echo "Missing $name." >&2
        exit 1
    fi
done
# The public key compiled into Windows builds must be the committed one.
if [ "${SOLITON_UPDATE_PUBLIC_KEY:-}" != "$(base64 -w0 "$keys/soliton-update.pub")" ]; then
    echo "SOLITON_UPDATE_PUBLIC_KEY differs from installer/signing/soliton-update.pub." >&2
    exit 1
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
chmod 700 "$work"
umask 077
printf '%s\n' "$SOLITON_UPDATE_PRIVATE_KEY" > "$work/update.key"
printf '%s\n' "$SOLITON_GPG_PASSPHRASE" > "$work/passphrase"
export GNUPGHOME=$work/gnupg
mkdir "$GNUPGHOME"
printf '%s\n' "$SOLITON_GPG_PRIVATE_KEY" | gpg --batch --quiet --import
fingerprint=$(gpg --batch --with-colons --show-keys "$keys/soliton-packages.asc" | awk -F: '$1 == "fpr" { print $10; exit }')
gpg() { command gpg --batch --pinentry-mode loopback --passphrase-file "$work/passphrase" "$@"; }

# The updater reads the Minisign signature as base64 text from latest.json, and
# only accepts a trusted comment naming the version it offers.
setup=$dir/Soliton_${version}_x64-setup.exe
printf '%s\n' "$SOLITON_UPDATE_PRIVATE_KEY_PASSWORD" |
    minisign -S -s "$work/update.key" -m "$setup" -x "$work/setup.minisig" \
        -t "soliton-version:$version" >/dev/null
minisign -Vq -p "$keys/soliton-update.pub" -m "$setup" -x "$work/setup.minisig"
base64 -w0 "$work/setup.minisig" > "$setup.sig"

for rpm in "$dir"/*.rpm; do
    rpmsign --addsign --define "__gpg /usr/bin/gpg" --define "_gpg_name $fingerprint" \
        --define "_gpg_sign_cmd_extra_args --batch --pinentry-mode loopback --passphrase-file $work/passphrase" \
        "$rpm" >/dev/null
    # Signing rewrites the header, so the checksum follows.
    (cd "$dir" && sha256sum "$(basename "$rpm")" > "$(basename "$rpm").sha256")
done

# A keyring holding only the committed public key checks every signature.
check=$work/check
mkdir "$check"
command gpg --batch --quiet --homedir "$check" --import "$keys/soliton-packages.asc"
rpm --dbpath "$work/rpmdb" --initdb
rpmkeys --dbpath "$work/rpmdb" --import "$keys/soliton-packages.asc"
for rpm in "$dir"/*.rpm; do
    if ! rpmkeys --dbpath "$work/rpmdb" -K "$rpm" | grep -q 'signatures OK'; then
        echo "RPM signature check failed: $rpm" >&2
        exit 1
    fi
done
for file in "$dir"/*; do
    case "$file" in
        *.sha256 | *.sig | *.asc | *.rpm | */latest.json) continue ;;
    esac
    gpg --local-user "$fingerprint" --armor --detach-sign -o "$file.asc" "$file"
    command gpg --batch --quiet --homedir "$check" --verify "$file.asc" "$file" 2>/dev/null
done
echo "Signed with Minisign and GPG key $fingerprint."
