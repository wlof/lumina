# lumina

[![CI](https://github.com/wlof/lumina/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/wlof/lumina/actions/workflows/ci.yml?query=branch%3Amain)
[![crates.io](https://img.shields.io/crates/v/klipsch-lumina-cli.svg)](https://crates.io/crates/klipsch-lumina-cli)
[![docs.rs](https://docs.rs/klipsch-lumina/badge.svg)](https://docs.rs/klipsch-lumina)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Control your **Klipsch ProMedia Lumina 2.1** speakers from the command line:
EQ, sound mode, subwoofer level, night mode, input, lighting and the other
settings of Klipsch's Windows-only app, Klipsch Control. Works on Linux and
Windows, and builds for macOS and FreeBSD.

```console
$ lumina eq preset bass
EQ: bass (firmware preset flat)
$ lumina sub -3
Subwoofer: -3 dB
$ lumina light static orange
Light: static #ff7f00
$ lumina status
ProMedia Lumina  (serial 123456789012345, firmware 1.0.3)
Input:          usb
Sound mode:     music
EQ:             bass (firmware preset flat)
  band 1:          50 Hz  Q 3.500 +3.0 dB
  band 2:         150 Hz  Q 4.000 +2.0 dB
  band 3:         400 Hz  Q 4.000 +1.0 dB
  band 4:        1000 Hz  Q 3.000 +0.0 dB
  band 5:        3500 Hz  Q 3.500 +0.0 dB
  band 6:        8000 Hz  Q 4.000 +0.0 dB
Subwoofer:      -3 dB
Night mode:     off
Mute:           off
UI tones:       on
Auto standby:   on (after 15 min)
Bluetooth:      off (02:00:00:00:00:01)
Light:          static #ff7f00
```

## Install

### Prebuilt binaries

Linux and macOS:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/wlof/lumina/releases/latest/download/klipsch-lumina-cli-installer.sh | sh
```

Windows (PowerShell):

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/wlof/lumina/releases/latest/download/klipsch-lumina-cli-installer.ps1 | iex"
```

The installers put `lumina` in `~/.cargo/bin`. You can also download the
archive for your platform from the
[latest release](https://github.com/wlof/lumina/releases/latest) and put
`lumina` anywhere on your `PATH`. The Linux binaries need libusb 1.0, which
most distributions install by default. Archives from releases after 0.1.1
can be checked with `gh attestation verify <archive> --repo wlof/lumina`.

### With Cargo

```sh
cargo install --locked klipsch-lumina-cli
```

This builds from source: it needs Rust 1.88 or later and a C compiler, and on
Linux also pkg-config and the libusb headers (`libusb-1.0-0-dev` on Debian
and Ubuntu, `libusb1-devel` on Fedora, `libusb` on Arch). On FreeBSD, install
the `hidapi` and `pkgconf` packages first.

### Linux: allow access to the speakers

Without this, `lumina` only works as root. The udev rule gives the user
logged in at the computer access to the speakers:

```sh
curl -fsSL https://raw.githubusercontent.com/wlof/lumina/main/udev/60-klipsch-lumina.rules \
  | sudo tee /etc/udev/rules.d/60-klipsch-lumina.rules
sudo udevadm control --reload && sudo udevadm trigger
```

The rule is also in the release archives and in `udev/` in this repository.

### Platform support

| Platform | Status |
|----------|--------|
| Linux | Tested with real speakers |
| Windows | Tested with real speakers (Windows 10); works alongside Klipsch Control |
| macOS | Builds and passes the tests; not yet tried with real speakers, and macOS may not let programs reach the speakers' control interface |
| FreeBSD | Builds and passes the tests; not yet tried with real speakers; build it with Cargo |

If you try `lumina` on macOS or FreeBSD, please
[open an issue](https://github.com/wlof/lumina/issues) with the result.

## Usage

`lumina status` shows every setting at once (`--json` for scripts). Run a
setting's command without a value to see its current value, for example
`lumina sub`. Every change is read back from the speakers and reported if it
did not take, and out-of-range values are refused before anything is sent.
`lumina help <command>` lists all the options.

The tables below follow Klipsch Control's settings, so you can find each
one; extras that the app doesn't have are marked *(lumina only)*.

### Sound

| Setting | Command | Notes |
|---------|---------|-------|
| EQ preset | `lumina eq preset bass` | `flat`, `vocal`, `bass`, `treble`, `rock`, `hip-hop`; the same curves as the app |
| Custom EQ: six bands | `lumina eq set 4 --gain -2 --q 1.4` | Per band: `--freq` 20–20000 Hz, `--gain` −6 to +6 dB, `--q` 0.1–20 |
| Show the bands | `lumina eq bands` | `--json` for scripts |
| Reset the bands | `lumina eq reset` | The speakers' flat defaults |
| Firmware EQ preset *(lumina only)* | `lumina eq firmware rock` | `flat`, `vocal`, `bass`, `rock`, `treble`; built into the speakers and applied on top of the bands; the app never changes it |
| Sound mode | `lumina sound-mode surround` | `movie`, `music`, `surround` (Virtual Surround) |
| Subwoofer level | `lumina sub -3` | −20 to +10 dB |
| Night mode | `lumina night-mode on` | Turns the bass down by half |
| Mute | `lumina mute toggle` | `on`, `off`, `toggle` |
| Input | `lumina input aux` | `bt`, `aux`, `usb`, `usb-disc`; after switching to USB the speakers reconnect, which takes about 20 seconds |
| Bluetooth pairing | `lumina bt-pair on` | Only on the Bluetooth input |
| Volume | Not supported | Use your system's volume control; the app does the same |
| Play/pause, next, previous | Not supported | Use your desktop's media keys |
| EQ from a JSON file (app developer mode) | Not supported | Use `eq set` for each band |

### Speaker settings

| Setting | Command | Notes |
|---------|---------|-------|
| Button tones ("Function Sound") | `lumina tones off` | |
| Auto standby | `lumina auto-standby on --after 10` | Standby after this many minutes without a signal, 1 to 60 (the app offers 2, 5, 10, 15) |
| Standby now *(lumina only)* | `lumina power standby --yes` | Only the speakers' power button wakes them again |
| Factory reset | `lumina factory-reset --yes` | Also forgets Bluetooth pairings; not yet tried on real speakers |
| Product information | `lumina status` | Serial number, firmware version, Bluetooth address |
| Wake on connect | Not supported | Nothing sent over USB wakes the speakers from standby |

### Lighting

Colours are a name (`black`, `white`, `red`, `green`, `blue`, `cyan`,
`magenta`, `purple`, `yellow`, `orange`, `pink`) or a hex code (`#ff7f00`
or `ff7f00`).

| Setting | Command | Notes |
|---------|---------|-------|
| Current mode | `lumina light` | |
| Static colour | `lumina light static orange` | |
| Breathe | `lumina light breathe blue` | Fades one colour in and out |
| Aurora | `lumina light aurora warm` | `cool` (blue and cyan) or `warm` (red and orange) |
| Music React | `lumina light music-react red-pink` | `default` (cycling colours), `blue-purple`, `cyan-blue`, `red-pink`, `yellow-orange` |
| Rainbow | `lumina light rainbow` | |
| Off | `lumina light off` | |
| Per-LED colours *(lumina only)* | `lumina light leds red blue` | See below |
| Screen React | Not supported | `light leds` shows fixed colours; following the screen is not implemented |
| LED brightness | Not supported | The speakers ignore it, in the app too |
| Windows Dynamic Lighting | Not supported | Windows only |

`light leds` takes 1, 2, 4 or 12 colours: one for all LEDs; left and right
speaker; left top, left bottom, right top, right bottom; or all twelve LEDs,
for each speaker (left first, as seen from the front) the top row then the
bottom row, left to right. The colours stay after `lumina` exits.

### App features

| Feature | Command | Notes |
|---------|---------|-------|
| Firmware version | `lumina status` | |
| Firmware updates | Not supported | Use Klipsch Control or Klipsch's USB update instructions |
| Product registration, telemetry | Not supported | `lumina` sends nothing anywhere |
| Start with Windows, tray icon | Not supported | |

## Troubleshooting

- **"no Klipsch ProMedia Lumina found"**: check that the speakers are on and
  connected to the computer with the USB cable.
- **"cannot open the speakers' control interface"**: on Linux, install the
  udev rule above. On Linux and FreeBSD only one program can control the
  speakers at a time, so close OpenRGB (if it has the Lumina driver) and
  detach the speakers from any virtual machine.
- **"speakers did not respond"**: the speakers are in standby, where they
  stop answering over USB. Press their power button. `lumina` gives up after
  `--timeout` seconds (default 20).
- **Lighting changes back on its own**: OpenRGB applies its own lighting
  when it starts.
- **Two sets of speakers**: `--serial` selects one by its USB serial number,
  but the tested speakers report a fixed-looking serial (`20190808`), so two
  sets may not be told apart. Untested.
- After any custom EQ change, the speakers keep a "custom EQ" flag that only
  a factory reset might clear. Klipsch Control sets it too; it has no known
  effect.

## Technical details

### Library

The CLI is built on the [`klipsch-lumina`](https://docs.rs/klipsch-lumina)
library ([crates/lumina](crates/lumina)), which other programs can use:
`Lumina::open` finds the speakers, typed setters change settings and
`state()` reads them all back. Its `sim` feature provides a simulated device
that behaves like the real speakers, for development and tests without
hardware.

### Protocol

The USB protocol was worked out from the speakers and from Klipsch Control
1.11.0; see [docs/protocol.md](docs/protocol.md). Every command except
`power standby` and `factory-reset` has been checked on real speakers
(firmware 1.0.3) by reading the state back; sound settings were also
checked by listening and lighting by eye. Plans and open questions are in
[PLAN.md](PLAN.md).

### Building from source

Needs Rust 1.88 or later and a C compiler; on Linux also pkg-config and the
libusb headers, on FreeBSD the `hidapi` package. With rustup,
`rust-toolchain.toml` selects the toolchain CI uses.

```sh
cargo build --release                            # target/release/lumina
cargo install --locked --path crates/lumina-cli  # or into ~/.cargo/bin
```

### Development

See [CONTRIBUTING.md](CONTRIBUTING.md). In short: `just ci` runs every check
CI runs, most tests use the simulated device, and `just test-hardware` runs a
round trip on connected speakers. Releases are automated; see
[RELEASING.md](RELEASING.md).

## License

MIT, see [LICENSE](LICENSE).
