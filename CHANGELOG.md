# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project
uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.1](https://github.com/wlof/lumina/compare/v0.1.0...v0.1.1) - 2026-10-28

### Fixed

- show HID errors once in error messages

## [0.1.0] - 2026-10-28

### Added

- `lumina` command line tool: status, EQ presets and bands, firmware EQ
  presets, sound mode, subwoofer level, night mode, mute, UI tones, auto
  standby, input, Bluetooth pairing, standby, factory reset, lighting modes
  and per-LED colours.
- `klipsch-lumina` library with typed setters, state decoding and a device
  simulator (`sim` feature).
- Protocol notes in `docs/protocol.md`.
- Published on crates.io as `klipsch-lumina` (library) and
  `klipsch-lumina-cli` (the `lumina` binary); prebuilt binaries for
  Linux, macOS and Windows on GitHub Releases.
