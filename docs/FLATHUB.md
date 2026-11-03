# Flathub submission

Memo for the author. Flathub refuses AI-made submissions: the pull requests,
their descriptions and the answers to reviewers are written by the author.

## Files to submit

Both come from the repository, for any release tag present locally
(`git fetch --tags`):

```sh
sh scripts/flathub-manifest.sh vX.Y.Z
```

Output in `target/flathub/`:

- `io.github.Carlton2001.Soliton.yml`: the manifest, its source pointing to the
  GitHub repository at the tag and its commit;
- `cargo-sources.json`: the Rust dependencies taken from `Cargo.lock`.

They are not attached to the GitHub release. Before submitting, check the build
locally with `sh scripts/build-flatpak.sh` (it runs `flatpak-builder-lint`).

Prerequisites: two-factor authentication enabled on the GitHub account (needed
to receive write access to the Flathub repository), and the metainfo up to date
(release entry for the version, screenshots reachable at the tag URL).

## Initial submission

Done once, on `flathub/flathub`.

1. Fork `https://github.com/flathub/flathub` on GitHub. Leave "Copy the master
   branch only" **unchecked**: the submission starts from branch `new-pr`.
2. Clone the fork and branch from `new-pr`:
   ```sh
   git clone --branch new-pr git@github.com:Carlton2001/flathub.git
   cd flathub
   git checkout -b add-soliton
   ```
3. Copy the two files of `target/flathub/` to the root of the clone, then commit
   and push to the fork:
   ```sh
   git add io.github.Carlton2001.Soliton.yml cargo-sources.json
   git commit -m "Add io.github.Carlton2001.Soliton"
   git push -u origin add-soliton
   ```
4. Open the pull request from `add-soliton` to **`flathub/flathub:new-pr`**
   (not `master`), titled `Add io.github.Carlton2001.Soliton`, and fill in the
   checklist of the template.
5. Comment `bot, build` to start a test build; the bot answers with the result
   and a test bundle install command.
6. Answer the reviewers by pushing new commits to the same branch (for a manifest
   fix, change `installer/linux/flatpak/` in Soliton first, then regenerate).
   Review can take several days to a few weeks.
7. After the merge, Flathub creates the repository
   `flathub/io.github.Carlton2001.Soliton` and sends a GitHub invitation:
   accept it (it expires after a week). The first build is published within a
   few hours; the `io.github.Carlton2001` app ID is verified through the GitHub
   account.

## Updates

Done for each new version, on `flathub/io.github.Carlton2001.Soliton` (write
access, no fork needed).

1. Once the `vX.Y.Z` tag is published, regenerate the files:
   `sh scripts/flathub-manifest.sh vX.Y.Z`.
2. In a clone of the Flathub repository, branch from `master`:
   ```sh
   git clone git@github.com:flathub/io.github.Carlton2001.Soliton.git
   cd io.github.Carlton2001.Soliton
   git checkout -b update-X.Y.Z
   ```
3. Replace the manifest and `cargo-sources.json` with the regenerated ones, then
   commit (`Update to X.Y.Z`) and push the branch.
4. Open the pull request to `master` within the same repository. The bot builds
   it automatically and posts a test bundle; install it and check the app.
5. Merge the pull request yourself once the build is green. The new version
   reaches users a few hours later.

No reviewer is involved for updates. A change of permissions (`finish-args`)
is still checked by Flathub before publication, so mention why in the pull
request.
