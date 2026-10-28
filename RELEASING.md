# Releasing

Both crates, `klipsch-lumina` (the library) and `klipsch-lumina-cli` (the
`lumina` binary), always share one version.

## How it works

1. Every push to `main` runs [release-plz](https://release-plz.dev)
   (`.github/workflows/release-plz.yml`). It opens or updates a release PR
   that bumps the version, following SemVer from the commit messages and
   `cargo-semver-checks`, and adds a `CHANGELOG.md` section generated from
   the commits.
2. Review the PR, edit the changelog if needed, and merge it.
3. release-plz then publishes both crates to crates.io and pushes a
   `vX.Y.Z` tag.
4. The tag starts [dist](https://opensource.axo.dev/cargo-dist/)
   (`.github/workflows/release.yml`), which builds `lumina` for Linux
   (x86_64, aarch64), macOS (Apple Silicon, Intel) and Windows (x86_64), and
   creates the GitHub Release with the archives, checksums and shell and
   PowerShell installers.

Configuration: `release-plz.toml` and `dist-workspace.toml`. After changing
`dist-workspace.toml`, run `dist generate` to rewrite `release.yml`; never
edit `release.yml` by hand (the `plan` job fails when it is out of date).
Dependabot skips `release.yml`, so its action pins are in
`dist-workspace.toml`.

## Repository setup

- **`RELEASE_PLZ_TOKEN` secret**: a fine-grained personal access token for
  this repository with *Contents* and *Pull requests* read/write. With the
  default `GITHUB_TOKEN`, the release PR would not run CI and the tag would
  not start `release.yml`.
- **`release` environment** (Settings → Environments), used by the publish
  job. Optionally add yourself as a required reviewer so every publish
  needs an approval.
- **crates.io trusted publishing**, once the crates exist (see below): for
  each crate, Settings → Trusted Publishing → add GitHub with repository
  `wlof/lumina`, workflow `release-plz.yml` and environment `release`. No
  crates.io token is stored in GitHub.

## First release (0.1.0)

Trusted publishing can only be set up for crates that already exist, and
release-plz does not open a PR for a version nobody has published yet, so
0.1.0 is released by hand:

1. In `CHANGELOG.md`, add `## [0.1.0] - YYYY-MM-DD` under
   `## [Unreleased]`, so the existing entries become the 0.1.0 section.
   Commit and push.
2. Publish both crates with a crates.io API token (scope
   *publish-new*):

   ```
   cargo login
   cargo publish --workspace
   ```

3. Tag and push; dist builds the binaries and the GitHub Release:

   ```
   git tag v0.1.0
   git push origin v0.1.0
   ```

4. Set up trusted publishing for both crates as above, then revoke the API
   token.
