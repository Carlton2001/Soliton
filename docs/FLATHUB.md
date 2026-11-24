# Flathub submission

## Prepare the Soliton files

From a checkout of the published release tag, generate the matching offline
Cargo inputs and manifest. Replace `vX.Y.Z` with that tag.

```sh
python3 -I scripts/flatpak-cargo-sources.py
sh scripts/flathub-manifest.sh vX.Y.Z
```

The script writes `io.github.Carlton2001.Soliton.yml` and `cargo-sources.json`
to `target/flathub/`. The manifest points to the GitHub repository at the
release tag and commit. Check that the AppStream release entry and screenshot
URLs match the tag. To test a local bundle, build the release commit while it
is at the tip of `main`. See the [Linux guide](LINUX.md#build-the-flatpak).

## Submit or update

The maintainer handles submission and review. Use Flathub's current instructions
for the first submission or for updates to an accepted app. Review its
generative AI policy before preparing submission materials.

- [Initial submission](https://docs.flathub.org/docs/for-app-authors/submission)
- [Maintenance and updates](https://docs.flathub.org/docs/for-app-authors/maintenance)
- [Generative AI policy](https://docs.flathub.org/docs/for-app-authors/requirements#generative-ai-policy)
