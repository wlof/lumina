# lumina

[![CI](https://github.com/wlof/lumina/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/wlof/lumina/actions/workflows/ci.yml?query=branch%3Amain)

Command-line control for Klipsch ProMedia Lumina 2.1 speakers on Linux and
other platforms: EQ presets and parametric bands, sound mode, subwoofer level, night mode,
input, lighting effects and per-LED colours, and the other settings of
Klipsch's Windows app, Klipsch Control. For lighting synced with other RGB
devices, [OpenRGB](https://gitlab.com/CalcProgrammer1/OpenRGB) also
supports the speakers.

The USB protocol was worked out from the speakers and from Klipsch Control
1.11.0; see [docs/protocol.md](docs/protocol.md). Plans and open questions
are in [PLAN.md](PLAN.md).

## Status

The CLI covers every audio, lighting and system setting the speakers
expose over USB. Every command except `power standby` and `factory-reset`
has been run against real speakers (firmware 1.0.3) and checked by reading
the state back. Sound mode, EQ, subwoofer level, night mode, mute and input
were also checked by listening, and every lighting effect by eye.

CI builds and tests every platform below against a simulated device; only
Linux has been tried with real speakers.

| Platform | State |
|----------|-------|
| Linux | Works, tested on hardware |
| FreeBSD | Builds and passes CI; should work (same hidapi/libusb path as the OpenRGB driver) |
| Windows | Builds and passes CI; should work (hid.dll, like Klipsch Control) |
| macOS | Builds and passes CI; unknown whether macOS exposes the speakers' control interface |

## Compared with Klipsch Control

✅ implemented · ➖ left to another tool · ❌ not implemented

### Audio

| Klipsch Control | lumina | Notes |
|-----------------|--------|-------|
| EQ presets: Flat, Vocal Boost, Bass Boost, Treble Boost, Rock, Hip Hop | ✅ `eq preset` | Same band tables as the app |
| Custom EQ (six bands) | ✅ `eq set`, `eq bands`, `eq reset` | Per band frequency, Q and gain (±6 dB like the app) |
| Manual PEQ from a JSON file (developer mode) | ❌ | `eq set` covers the same ground one band at a time |
| — | ✅ `eq firmware` | Firmware presets (Flat, Vocal, Bass, Rock, Treble); the app never uses them, but they are audible and combine with the bands |
| Sound mode: Movie, Music, Virtual Surround | ✅ `sound-mode` | |
| Subwoofer level, −20 to +10 dB | ✅ `sub` | |
| Night mode | ✅ `night-mode` | |
| Mute | ✅ `mute` | Plus `toggle` |
| Master volume | ➖ system mixer | The app sets the Windows volume too, not the speakers |
| Input source (Bluetooth, Aux, USB) | ✅ `input` | Also USB Disc; waits for the speakers to reconnect after switching back to USB |
| Bluetooth pairing | ✅ `bt-pair` | Needs the Bluetooth input |
| Play/pause, next, previous | ❌ | Commands known (0x05–0x07); on the PC side MPRIS does this |

### Speaker settings

| Klipsch Control | lumina | Notes |
|-----------------|--------|-------|
| Function Sound (UI tones) | ✅ `tones` | |
| Auto standby on/off | ✅ `auto-standby` | |
| Auto standby delay: 2, 5, 10, 15 min | ✅ `auto-standby --after` | Any 1 to 60 min |
| Wake on connect | ❌ | Not needed while the speakers are on; nothing sent over USB wakes them from standby |
| — | ✅ `power standby --yes` | Only the power button wakes the speakers afterwards |
| Factory reset | ✅ `factory-reset --yes` | Not tried on hardware |
| Product information (serial, firmware version) | ✅ `status` | Also Bluetooth MAC and name |
| — | ✅ `status --json`, `eq bands --json` | For scripts |

### Lighting

| Klipsch Control | lumina | Notes |
|-----------------|--------|-------|
| Static and Breathe with a custom colour | ✅ `light static`, `light breathe` | Any colour, `#rrggbb` or a name |
| Aurora, cool or warm | ✅ `light aurora` | |
| Music React with its colour presets | ✅ `light music-react` | Default (cycling) and the four pairs; the firmware ignores other colours |
| Rainbow, Off | ✅ `light rainbow`, `light off` | |
| Screen React (screen colours on the LEDs, with performance and smoothing settings) | ❌ | The per-LED mode it uses is there (`light leds`); sampling the screen is not |
| — | ✅ `light leds` | Static per-LED colours: 1, 2, 4 or 12 colours; they stay after `lumina` exits |
| Windows Dynamic Lighting | ➖ | Windows only |
| LED brightness | ❌ | The command exists but has no visible effect, in the app too |

### Firmware and app

| Klipsch Control | lumina | Notes |
|-----------------|--------|-------|
| Firmware version | ✅ `status` | |
| Online firmware update check, scheduled auto-update | ❌ | Uses Klipsch's web API with the app's key; not used here |
| Firmware update from a `.bin` file | ❌ | Separate update path (CDC/DFU), not investigated; Klipsch also documents a USB update method |
| Product registration (anonymous on connect, or with an e-mail) | ❌ | Not wanted |
| Crash and usage telemetry (Sentry) | ❌ | Not wanted |
| App self-update | ➖ package manager | |
| Start with Windows, close to tray, always on top | ❌ | Desktop integration; for a later GUI or tray applet |
| Encrypted log files and log viewer | ❌ | `lumina` keeps no logs; `debug dump` shows the raw state |

## Build

Needs Rust 1.88 or later and a C compiler. With rustup,
`rust-toolchain.toml` selects the toolchain CI uses. On Linux also
pkg-config and the libusb headers (`libusb-1.0` / `libusb-1.0-0-dev`); on
FreeBSD pkg-config and the `hidapi` package. Windows and macOS need nothing
else.

```
cargo build --release                            # target/release/lumina
cargo install --locked --path crates/lumina-cli  # or into ~/.cargo/bin
```

## Permissions (Linux)

Install the udev rule so the logged-in user can access the device:

```
sudo install -m 644 udev/60-klipsch-lumina.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger
```

## Usage

```
lumina status                     # everything at a glance, --json for scripts
lumina eq preset bass             # flat, vocal, bass, treble, rock, hip-hop
lumina eq set 4 --gain -2 --q 1.4 # change one of the six bands
lumina eq reset                   # back to the device's flat defaults
lumina eq firmware rock           # the firmware's own preset, on top of the bands
lumina sound-mode surround        # movie, music, surround
lumina sub -3                     # subwoofer level, -20 to +10 dB
lumina night-mode on
lumina mute toggle
lumina input aux                  # bt, aux, usb, usb-disc
lumina bt-pair on                 # needs the Bluetooth input
lumina tones off
lumina auto-standby on --after 10
lumina power standby --yes        # only the power button wakes them

lumina light                      # current light mode
lumina light static orange        # also #ff7f00 or ff7f00
lumina light breathe blue
lumina light aurora warm          # cool, warm
lumina light music-react red-pink # default, blue-purple, cyan-blue, red-pink, yellow-orange
lumina light rainbow
lumina light off
lumina light leds red blue        # left satellite red, right blue
lumina light leds red green blue white  # left top, left bottom, right top, right bottom
```

`light leds` also takes 12 colours: per satellite (left first, as seen
facing the speakers), the top row then the bottom row, each left to right.

Setting commands without a value print the current one. Every change is
read back from the speakers and reported if it did not take. Values out
of range are refused before anything is sent.

## Known limitations

- On Linux and FreeBSD only one program can use the speakers' control
  interface at a time. `lumina` cannot reach them while OpenRGB (with the
  Lumina driver loaded) or a virtual machine holds them. Starting OpenRGB
  also applies its own lighting over `lumina`'s.
- In standby the speakers stop answering on USB. `lumina` gives up after
  `--timeout` seconds (default 20) and suggests pressing the power button.
- Writing any custom EQ band sets a flag in the speakers ("PeqMode") that
  nothing known clears except possibly a factory reset. Klipsch Control
  sets it too, so this is the state after using the app.
- Switching the input back to USB makes the speakers reconnect; the audio
  output disappears for about 20 seconds.
- `--serial` picks speakers by USB serial number, but the tested set reports
  `20190808`, which looks like a fixed value. With two sets connected it
  probably cannot tell them apart; untested.

## Library

The CLI is built on the `klipsch-lumina` library
([crates/lumina](crates/lumina)), which other front ends can use directly:
`Lumina::open` finds the speakers, typed setters change settings and
`state()` reads them all back. Its `sim` feature provides a simulated device
that behaves like the real speakers, for developing and testing without
hardware. `cargo doc --open -p klipsch-lumina` shows the API.

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md). In short: `just ci` runs every check
CI runs; most tests use a simulated device, and `just test-hardware` runs a
round trip on connected speakers.

## License

MIT, see [LICENSE](LICENSE).
