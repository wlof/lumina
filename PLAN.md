# Plan

Goal: control the Klipsch ProMedia Lumina audio settings from Linux, without
reproducing the Windows app 1:1. CLI first, GUI later. Lighting stays in
OpenRGB.

## Feature scope

Klipsch Control 1.11.0 exposes these through the vendor protocol:

| Feature | Command | In scope |
|---------|---------|----------|
| EQ presets (Flat, Vocal, Bass, Treble, Rock, Hip Hop) | 0x0A with the app's band tables | yes |
| Custom parametric EQ, 6 bands | 0x0A | yes |
| Firmware EQ preset | 0x09 | read only, maybe; the app never sets it |
| Sound mode (Movie, Music, Virtual Surround) | 0x08 | yes |
| Subwoofer level, −20…+10 dB | 0x04 | yes |
| Night mode | 0x0D | yes |
| Mute | 0x0C | yes |
| Input source (Bluetooth, Aux, USB) | 0x01 | yes |
| Bluetooth pairing | 0x02 | yes |
| Play/pause, next, previous | 0x05–0x07 | maybe; MPRIS covers this |
| UI tones ("Function Sound") | 0x10 | yes |
| Auto standby on/off, timeout | 0x14, 0x18 | yes |
| Power state | 0x16 | yes |
| Factory reset | 0x19 | yes, behind `--yes` |
| Device info (serial, firmware, BT MAC) | state report | yes |
| Lighting modes, colours, per-LED | 0x0B, 0x11, 0x1A | yes (added after phase 3; OpenRGB also drives them) |
| Screen React (screen sampling) | 0x11 stream | later, needs a long-running process |

Master volume stays out of scope: Klipsch Control itself sets the OS volume
rather than sending command 0x03, and on Linux that is PipeWire's job.

## Architecture

Cargo workspace:

- `crates/lumina`: library. Frame codec and state parsing (pure, unit
  tested), typed commands, `Lumina` device handle over `hidapi`. A future GUI
  reuses it as is.
- `crates/lumina-cli`: the `lumina` binary (clap).
- later `crates/lumina-gui`.

Transport: the `hidapi` crate, so the same code runs on every platform
OpenRGB supports for this device. Reports are HID class control transfers
on interface 4. Only interface 4 is opened, so audio playback
(interfaces 0–2) and the LampArray (interface 5) are untouched.

| Platform | hidapi back end | Shared with OpenRGB / Klipsch Control | Status |
|----------|-----------------|---------------------------------------|--------|
| Linux | libusb, bundled hidapi (no hidraw node for interface 4) | no, exclusive claim | works |
| FreeBSD | libusb, system `hidapi` package | no, exclusive claim | to test in the FreeBSD VM |
| Windows | hid.dll | yes | to test in the Win10 VM |
| macOS | IOKit, opened non-exclusive | only if the other side is non-exclusive too | untested, no hardware; unknown whether IOKit exposes the endpoint-less interface |

`nusb` was tried first and dropped: it needs WinUSB on Windows, cannot
reach interfaces owned by the macOS HID driver, and has no FreeBSD support.

## Phases

### 0. Repository setup (done)

Workspace, frame codec with tests, state parser for the verified fields,
`hidapi` transport, `lumina status` and `lumina debug dump|send`. Verified on
Linux.

### 1. Decode the audio payloads (done)

Command table, enums, EQ band encoding and the app's preset tables were
read from `KlipschControl.dll` (copied out of the Win10 VM, parsed with
small metadata/IL readers, nothing executed). Most commands were then
confirmed from Linux with `lumina debug send` and state diffs. See
`docs/protocol.md`.

Left open: audible checks, the meaning of PeqMode (state byte 43) and how
to clear it, SubToggle and OperatingMode.

### 2. Library API (done)

- `Lumina` setters with range checks: sound mode, firmware preset,
  parametric bands and app presets, subwoofer level, night mode, mute, UI
  tones, auto standby and timeout, input, Bluetooth pairing, standby,
  factory reset.
- `State` decodes every mapped field; `eq::Preset::matching` recognises the
  app's presets in the bands.
- 50 ms spacing between frames.
- `Transport` trait; unit tests use a recording mock. `tests/hardware.rs`
  (ignored by default) round trips every reversible setting on real
  speakers and checks the state is restored.
- Errors say what to do: not found, busy (OpenRGB/VM), no response
  (standby or disconnected), reconnecting (input switched to USB).

Carried over to phase 3: hidapi calls can block for a long time when the
speakers are in standby, and the blocking step is not identified yet. The
CLI needs a watchdog timeout, and the step should be pinned down on the
next standby test.

### 3. CLI (done)

```
lumina status [--json]
lumina eq preset [flat|vocal|bass|treble|rock|hip-hop]
lumina eq bands [--json]
lumina eq set <1-6> [--freq HZ] [--gain DB] [--q Q]
lumina eq reset
lumina eq firmware [flat|vocal|bass|rock|treble]
lumina sound-mode [movie|music|surround]
lumina sub [<dB>]
lumina night-mode [on|off]
lumina mute [on|off|toggle]
lumina tones [on|off]
lumina auto-standby [on|off] [--after MIN]
lumina input [bt|aux|usb|usb-disc]
lumina bt-pair [on|off]
lumina power standby --yes
lumina factory-reset --yes
lumina debug dump | send <cmd> <bytes...> [--no-read]
```

Every setter reads the state back and fails if the speakers disagree.
`input usb` from another input waits for the re-enumeration. A watchdog
thread (`--timeout`, default 20 s) ends the process if the speakers stop
answering. All commands except standby and factory reset were run on the
hardware with the state restored afterwards.

Open: the watchdog has not been exercised against a real standby yet, and
the blocking hidapi step is still unidentified. Profiles (`lumina profile
save|load`) were left out for now.

### 3b. Lighting (done)

`lumina light` with static, breathe, aurora, music-react, rainbow, off and
per-LED `leds`, using the app's colour presets and the OpenRGB driver's
verified Direct-mode sequence. Every effect was checked by eye on the
hardware, including the LED order and that per-LED frames persist.

### Engineering baseline (done)

Pinned toolchain and verified MSRV, workspace lints (clippy pedantic,
`unsafe` forbidden, documented public API), property tests, a device
simulator for CLI tests, cargo-deny, GitHub Actions CI on Linux, macOS,
Windows and FreeBSD with SHA-pinned actions, Dependabot, `just ci` locally.
See CONTRIBUTING.md.

### 4. Portability and packaging

- Build and test on Windows and FreeBSD in the VMs; CI builds for Linux,
  Windows and macOS

- `udev/60-klipsch-lumina.rules` (uaccess), documented in the README
- shell completions and man page via `clap_complete` / `clap_mangen`
- PKGBUILD
- CI: fmt, clippy, test

### 5. GUI (later)

Separate decision once the CLI is stable. Candidates: `iced` or
`gtk4-rs`/`relm4` for a desktop app, or a tray applet.

## Open issues

- **Interface 4 is exclusive on Linux and FreeBSD** (libusb claim; Windows
  shares it). A running OpenRGB server with the Lumina detector holds it,
  so `lumina` fails to open the device. Options: tell the user
  to stop OpenRGB, have the OpenRGB driver release the interface when idle,
  or route through OpenRGB somehow. The CLI claims the interface only for
  the duration of a command.
- **Hardware is shared with the OpenRGB work.** The device is sometimes
  passed through to the `win10-openrgb` and `freebsd-openrgb` VMs, which
  also makes them the test machines for the Windows and FreeBSD builds.
- **Boot delay.** The device ignores requests for ~12 s after power up or
  reset. `state()` already polls for 15 s; setters may need the same.
