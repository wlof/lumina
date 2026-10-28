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

Already in place; listed for reference, for example to recreate the token.

- **`RELEASE_PLZ_TOKEN` secret**: a fine-grained personal access token for
  this repository with *Contents* and *Pull requests* read/write. With the
  default `GITHUB_TOKEN`, the release PR would not run CI and the tag would
  not start `release.yml`.
- **`release` environment** (Settings → Environments), used by the publish
  job and limited to `main`. Optionally add yourself as a required
  reviewer so every publish needs an approval.
- **crates.io trusted publishing**: for each crate, Settings → Trusted
  Publishing → GitHub with repository `wlof/lumina`, workflow
  `release-plz.yml` and environment `release`. No crates.io token is stored
  in GitHub.

When the token expires, release-plz fails on every push to `main`. Create a
new one with the same permissions and run
`gh secret set RELEASE_PLZ_TOKEN --repo wlof/lumina`.
