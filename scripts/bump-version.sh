#!/bin/sh
# Linux counterpart of bump-version.ps1: sets the package version, adds the
# AppStream release, commits and creates the matching tag. Pushing the tag
# (done by hand) starts the release workflow.
# Usage: scripts/bump-version.sh <major.minor.patch>
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
METAINFO=installer/linux/io.github.Carlton2001.Soliton.metainfo.xml
version=${1:-}
case "$version" in
    *[!0-9.]* | "" | .* | *. | *..*) version= ;;
esac
if [ "$(printf '%s' "$version" | tr -cd . | wc -c)" -ne 2 ]; then
    echo "Usage: $0 <major.minor.patch>, e.g. $0 0.2.0" >&2
    exit 2
fi
tag=v$version

if [ "$(git branch --show-current)" != main ]; then
    echo "Release from the main branch." >&2
    exit 1
fi
if [ -n "$(git status --porcelain)" ]; then
    echo "Commit or stash local changes first." >&2
    exit 1
fi
remote=$(git ls-remote --tags origin "refs/tags/$tag")
if [ -n "$(git tag --list "$tag")" ] || [ -n "$remote" ]; then
    echo "Tag $tag already exists." >&2
    exit 1
fi
current=$(sed -n 's/^version = "\([^"]*\)"$/\1/p' Cargo.toml | head -n 1)
if [ -z "$current" ]; then
    echo "Package version not found in Cargo.toml." >&2
    exit 1
fi
if [ "$version" = "$current" ] || [ "$(printf '%s\n%s\n' "$current" "$version" | sort -V | tail -n 1)" != "$version" ]; then
    echo "Version $version must be greater than $current." >&2
    exit 1
fi

sed -i "0,/^version = \"$current\"\$/s//version = \"$version\"/" Cargo.toml
cargo update --workspace --offline
# The notices record the Cargo.lock hash, which the version change alters.
sh scripts/update-third-party-licenses.sh --offline
sed -i "s#^\(\s*\)<releases>\$#&\n\1  <release version=\"$version\" date=\"$(date -u +%F)\"/>#" "$METAINFO"
# Screenshot URLs follow the tag, which Flathub reads when it builds.
sed -i "s#/Soliton/v[0-9.]*/screenshots/#/Soliton/$tag/screenshots/#g" "$METAINFO"
git add Cargo.toml Cargo.lock "$METAINFO" licenses/dependencies
git commit -q -m "Release $tag"
git tag -a "$tag" -m "Soliton $tag"
echo "Version $current -> $version committed and tagged $tag."
echo "Publish with: git push origin main $tag"
