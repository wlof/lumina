# Contributing

## Setup

- Rust via [rustup](https://rustup.rs). `rust-toolchain.toml` pins the
  toolchain; rustup installs it on first use.
- A C compiler and pkg-config. On Linux the libusb headers
  (`libusb-1.0-0-dev` / `libusb1-devel`), on FreeBSD the `hidapi` package.
- The checks use a few cargo tools:

  ```
  cargo install --locked just cargo-nextest cargo-hack cargo-deny cargo-machete \
      cargo-llvm-cov typos-cli taplo-cli
  ```

## Checks

`just ci` runs everything CI runs except the other operating systems:
formatting (rustfmt, taplo), clippy with warnings as errors for every
feature combination, tests, docs, cargo-deny, unused dependencies, spelling
and a build with the minimum supported Rust version. `just` lists the
individual recipes.

CI also tests on macOS, Windows and FreeBSD and records coverage.

## Tests

Most tests need no hardware:

- unit and property tests in each module (proptest for the encoders and for
  parsing arbitrary device input);
- CLI tests that run commands against `klipsch_lumina::sim::Simulator`, a simulated
  device built from a real state report (`crates/lumina/tests/data`).

With speakers connected and nothing else holding them (OpenRGB, a VM),
`just test-hardware` changes every reversible setting, checks it and
restores the original state.

When the protocol docs say something was verified on hardware, it was; keep
that true. `docs/protocol.md` tags every fact with how it is known.

## Code

- No `unsafe` (forbidden workspace-wide).
- Public items are documented; `cargo doc` runs with warnings as errors.
- Lints are configured in the workspace `Cargo.toml`. Silence one locally
  with `#[expect(..., reason = "...")]`, never globally without discussion.
- Values coming from the device are untrusted: parsing must never panic.
- Raising `rust-version` is a deliberate change: note why next to it.

## Commits and changelog

`CHANGELOG.md` is generated from commit messages when a release is
prepared (see [RELEASING.md](RELEASING.md)), so write them for users and use
[Conventional Commits](https://www.conventionalcommits.org/) prefixes:
`feat:` and `fix:` become Added and Fixed entries, `feat!:` or a
`BREAKING CHANGE:` footer marks a breaking change. Other prefixes (`docs:`,
`ci:`, `refactor:`, ...) and unprefixed messages are listed under Other.
Don't edit `CHANGELOG.md` by hand; polish it in the release PR instead.
