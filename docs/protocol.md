# Klipsch ProMedia Lumina USB protocol

Sources:

- **[hw]** verified on hardware by state readback
- **[ear]** audible effect confirmed in a listening test (A/B tones or noise)
- **[eye]** lighting effect confirmed by looking at the satellites
- **[app]** read from Klipsch Control 1.11.0 (`KlipschControl.dll`): command
  table in `Data.Commands`, enums in `Enums`, encodings in `Models.EQBand`
  and the `MainWindowViewModel` setters
- **[?]** unknown

The lighting side was first worked out for the OpenRGB driver
(`Controllers/KlipschLuminaController`).

## Device

USB `18b5:0070`, full speed, one configuration, six interfaces:

| If | Class | Purpose |
|----|-------|---------|
| 0 | Audio control | Feature unit 2 (speaker): mute + per-channel volume. Feature unit 5 (mic). |
| 1 | Audio streaming | Playback, 16-bit stereo, 44.1/48 kHz, EP 5 OUT isochronous |
| 2 | Audio streaming | Capture, 16-bit stereo, 44.1/48 kHz, EP 4 IN isochronous |
| 3 | HID | EP 1 IN interrupt, media keys, see HID descriptors |
| 4 | HID, page FF00 usage 55AA | **Vendor command channel**, no endpoint |
| 5 | HID, page 0x59 | LampArray (1 lamp), no endpoint. Lighting only. |

Interfaces 4 and 5 have no interrupt endpoint, so Linux `usbhid` does not
bind them and there is no hidraw node. All traffic uses HID class control
transfers on the default pipe [hw]:

- send: `SET_REPORT` (`bmRequestType 0x21`, `bRequest 0x09`, `wValue 0x0200`, `wIndex 4`), 256 bytes
- read: `GET_REPORT` (`bmRequestType 0xA1`, `bRequest 0x01`, `wValue 0x0100`, `wIndex 4`),
  256 bytes requested, 123 returned

On Windows the interface is a regular HID device (`MI_04`), opened shared;
Klipsch Control uses `WriteFile` and `HidD_GetInputReport` on it. On Linux
and FreeBSD hidapi's libusb back end issues the control transfers itself.

Byte offsets below are into the report as on the wire. A hidapi buffer is
one byte longer (report ID 0 first), so its offsets are one higher.

### HID descriptors [hw]

Read with `lsusb -v` and from sysfs (interface 3), and the LampArray
feature reports with `GET_REPORT` (`wValue 0x03nn`).

- **Interface 3:** Consumer Control, no report ID, one 8-bit input report,
  one bit per key: Volume Up, Volume Down, Next Track, Previous Track,
  Mute, Fast Forward, Play/Pause, Stop. The speakers have no remote. The
  volume buttons send Volume Up and Volume Down, one press report followed
  at once by a release (`00`); the host's volume changes with them. The
  input button sends nothing: input changes only show in the state
  report. The other six keys have no known button. USB serial string
  `20190808`, the same field as `HID_UNIQ`.
- **Interface 4:** 256-byte input (the state report), 256-byte output
  (command frames) and an 8-byte feature report, all without report ID.
  The feature report is not implemented: `GET_REPORT` returns as many bytes
  as requested (256 tried), starting with the request's own setup packet
  and followed by unrelated buffer contents. `lumina` does not use it;
  whether Klipsch Control does is unknown [?]. The pointers in that data
  fall in `0x2000_0000`, which suggests an ARM Cortex-M controller [?].
- **Interface 5:** standard LampArray, reports 1–6 (attributes, lamp
  attributes request and response, multi update, range update, control).
  Only report 6 (LampArrayControl) is used, see Lighting. Reports 4 and 5
  are untested.

LampArrayAttributes (report 1): 1 lamp, bounding box 100 × 100 × 100 mm,
LampArrayKind 13, MinUpdateInterval 50 ms.

LampAttributesResponse (report 3, lamp 0, read without writing report 2):
position (0, 50, 50) mm, update latency 50 ms, purposes 0x20
(Presentation), 255 levels for red, green and blue, 100 intensity levels,
programmable, no input binding. Windows Dynamic Lighting therefore sees
the speakers as one lamp, while CustomLightingMode drives 12 LEDs.

## Frames [hw]

```
A5 5A 70 <cmd> <len> 00 <data...> <chk> <zero padding to 256>
```

- `len` = total frame length = data length + 7
- `chk` makes the sum of all frame bytes 0 mod 256

## State report [hw]

Write a bare `A5 5A 70` frame (rest zero), wait ~100 ms, then `GET_REPORT`.
Without the request the device returns zeros, or the last cached reply. For
about 12 s after power up or a USB reset it returns zeros or times out.
Klipsch Control sends PowerState = 1 followed by the bare header as its
"wake-up buffer" when it connects [app].

The reply is 123 bytes and starts `A5 5A 70 7B 00`. Field names and order
are the app's device state properties [app]; offsets come from matching
them to a dump.

| Offset | Len | Field | Values | Status |
|--------|-----|-------|--------|--------|
| 5 | 1 | InputSource | 0 Bluetooth, 1 Aux, 2 USB PC, 3 USB Disc | [hw] |
| 6 | 1 | BtPairingStatus | 0 off, 1 pairing, 2 paired | [hw] |
| 7 | 1 | MasterVolume | follows the host playback volume: 13 at PipeWire 0.70, 20 at 0.80, 0 when turned down | [hw] |
| 8 | 1 | SubVolume | dB + 20, i.e. 0–30 for −20…+10 dB | [hw] |
| 9 | 1 | PlayPause | 0 play, 1 pause; goes to 0 when the host starts streaming | [hw] |
| 10 | 1 | SoundMode | 1 Movie, 2 Music, 3 Virtual Surround | [hw] |
| 11 | 1 | PresetEQ | 0–4, see EQ below | [hw] |
| 12 | 1 | LightMode | see LightMode command | [hw] |
| 13 | 15 | SerialNumber 1–3 (ASCII, 3 × 5) | | [hw] |
| 28 | 6 | BTMacAddress | | [app] |
| 34 | 5 | FirmwareVersion (ASCII, e.g. `1.0.3`) | | [hw] |
| 39 | 1 | Mute | 0/1 | [hw] |
| 40 | 1 | NightMode | 0/1 | [hw] |
| 41 | 1 | SubToggle | 0 seen | [?] |
| 42 | 1 | NumberOfPEQFilters | 6 | [app] |
| 43 | 1 | PeqMode | 1 after any ParametricEq write, see EQ | [hw] |
| 44 | 1 | PowerModeAutoStandby | 0/1 | [hw] |
| 45 | 1 | PowerState | 1 = on; unreadable in standby | [hw] |
| 46 | 1 | OperatingMode | 0 seen | [?] |
| 47 | 2 | AutoStandbyTimeout | seconds, u16 LE (900 seen) | [hw] |
| 49 | 2 | LedLevel | LED brightness 0–100, the same value in both bytes (100, 66, 33, 0 from the button) | [hw] |
| 51 | 1 | UITones ("Function Sound") | 0/1 | [hw] |
| 52 | 1 | NewLightModeToggle | | [app] |
| 53 | 6 | CustomRGB (primary, secondary) | cleared by a light mode change | [hw] |
| 59 | 48 | EqBands, 6 × 8 bytes | see EQ | [hw] |
| 107 | 15 | PersonalizedName (`ProMedia Lumina`) | | [hw] |

Sample report (firmware 1.0.3, before any ParametricEq write):

```
  0: a5 5a 70 7b 00 02 00 0d 14 01 02 00 03 31 32 33
 16: 34 35 36 37 38 39 30 31 32 33 34 35 02 00 00 00
 32: 00 01 31 2e 30 2e 33 00 00 00 06 00 01 01 00 84
 48: 03 64 64 01 00 ff 00 ff ff 00 ff 00 00 32 00 d4
 64: 02 00 00 01 00 96 00 d4 02 00 00 02 00 90 01 d4
 80: 02 00 00 03 00 e8 03 d4 02 00 00 04 00 ac 0d d4
 96: 02 00 00 05 00 40 1f d4 02 00 00 50 72 6f 4d 65
112: 64 69 61 20 4c 75 6d 69 6e 61 ff
```

## Commands

Payloads are 1 byte unless noted [app].

| ID | Name | Payload | Status |
|----|------|---------|--------|
| 0x01 | InputSource | 0 Bluetooth, 1 Aux, 2 USB PC, 3 USB Disc. Switching to USB PC re-enumerates the device, see quirks | [hw, ear] |
| 0x02 | BtPairing | 0 exit, 1 enter. Ignored unless the input is Bluetooth | [hw] |
| 0x03 | MasterVolume | [?]; the app never sends it, it sets the Windows endpoint volume instead | [app] |
| 0x04 | SubwooferLevel | dB + 20 (0–30) | [hw, ear] |
| 0x05 | PlayPause | 0 play, 1 pause, 2 toggle | [app] |
| 0x06 | NextTrack | | [app] |
| 0x07 | PreviousTrack | | [app] |
| 0x08 | SoundMode | 1 Movie, 2 Music, 3 Virtual Surround; others ignored. All three sound different | [hw, ear] |
| 0x09 | EQPreset | 0–4; others ignored. The app never sends it. Audible even with PeqMode 1 | [hw, ear] |
| 0x0A | ParametricEq | 48 bytes, 6 bands, see EQ | [hw, ear] |
| 0x0B | LightMode | 1 Rainbow, 2 Breathe, 3 Static, 4 Aurora, 5 Music React, 6 Off, 7 Screen React, 8 Dynamic Lighting | [hw] |
| 0x0C | Mute | 0/1 | [hw, ear] |
| 0x0D | NightMode | 0/1 ("turns bass output down by 50%") | [hw, ear] |
| 0x0F | LedLevel | 2 bytes: selector (1 or 2), brightness 0–100. Anything else is ignored. See Lighting | [hw, eye] |
| 0x10 | UiTones | 0/1 | [hw] |
| 0x11 | CustomLightingMode | 36 bytes, 12 × RGB | [hw] |
| 0x14 | AutoStandby | 0/1 | [hw] |
| 0x16 | PowerState | 0 = standby (lights off), 1 = on (sent by the app on connect). Nothing sent over USB wakes standby, see quirks | [hw] |
| 0x18 | AutoStandbyTimeout | u16 LE seconds; the app offers 120, 300, 600, 900 | [hw] |
| 0x19 | FactoryReset | 1. Also forgets Bluetooth pairings | [app] |
| 0x1A | CustomRGB | 6 bytes, primary + secondary RGB | [hw] |

## EQ

### Band encoding [app, hw]

Each band is 8 bytes (`EQBand.ToBytes`):

| Byte | Field | Encoding |
|------|-------|----------|
| 0 | Index | 0–5 |
| 1 | Type | filter type, 0 in every default and preset [?] |
| 2–3 | Fc | Hz, u16 LE, clamped to 20–20000 |
| 4–5 | Q | Q × 1024, u16 LE |
| 6–7 | Gain | dB × 256, i16 LE |

Device default bands: 50, 150, 400, 1000, 3500, 8000 Hz, Q 0.707 (724),
gain 0. The app's UI limits gains to ±6 dB. A +6 dB change on the 1 kHz
band (Q 1.0) is clearly audible on a 1 kHz tone [ear].

### Presets

The firmware has its own preset byte (EQPreset, 0–4) but Klipsch Control
never uses it. The app implements every preset by writing a full band
table with ParametricEq, and remembers the selection itself. Its tables
(`Data.EqPresets`), gains in dB for 50 / 150 / 400 / 1k / 3.5k / 8k Hz:

| # | Name | Gains | Q |
|---|------|-------|---|
| 0 | Flat | 0 0 0 0 0 0 | 3.5 4 4 3 3.5 4 |
| 1 | Vocal Boost | −2 −3 2 2 1 0 | 3.5 4 4 3 3.5 4 |
| 2 | Bass Boost | 3 2 1 0 0 0 | 3.5 4 4 3 3.5 4 |
| 3 | Treble Boost | 0 0 0 1 2 3 | 1 1 1 1 1 1 |
| 4 | Rock | 3 2 3 1 −2 4 | 3.5 4 4 3 3.5 4 |
| 5 | Hip Hop | 4 4 −2 3 3 2 | 3.5 4 4 3 3.5 4 |
| 6 | Custom | user bands, based on one of the above | |

Names 3–5 are matched by UI string order and curve shape; the `EQPreset`
enum only has Flat, Vocal, Bass, Rock, Treble.

### PeqMode

Any ParametricEq write sets PeqMode (state byte 43) to 1, even one that
rewrites the current bands [hw]. Changing EQPreset does not clear it [hw],
and the app has no code that writes it [app]. Presumably 1 means the
custom bands are in effect rather than the firmware preset. A factory reset
is the only known way back to 0 [?].

The firmware preset stays in effect with PeqMode 1: switching EQPreset
between Flat and Bass is audible on top of flat custom bands [ear]. So the
two layers combine, or PeqMode means something else.

## Lighting [hw, eye]

All verified on the hardware with `lumina light` and checked by eye.

- **Mode then colours.** A LightMode change clears the stored CustomRGB
  pair, so the colours go after the mode. The OpenRGB driver and `lumina`
  wait 300 ms in between.
- **Static, Breathe:** only the primary colour is shown; Breathe with a
  different secondary still breathes the primary alone.
- **Aurora:** Klipsch Control offers two pairs: cool `0000FF`/`00FFFF`
  (blue/cyan) and warm `FF0000`/`FF7F00` (red/orange) [app]. Both drift as
  expected. Other pairs are stored and read back, but the satellites show
  the cool animation (`FF00FF`/`00FF00` tried) [hw, eye].
- **Music React:** only the app's pairs render (default `000000`/`000000`
  cycles through several colours; blue/purple `0000FF`/`FF00FF`, cyan/blue
  `00FFFF`/`0000FF`, red/pink `FF0000`/`FF007F`, yellow/orange
  `FFFF00`/`FF7F00`) [app]. Other pairs are stored but ignored (OpenRGB
  tests). Red/pink and default were checked against an audio signal.
- **Rainbow, Off** need no colours.
- **Direct (mode 7, "Screen React"):** CustomLightingMode (0x11) frames of
  12 × RGB. The firmware shows white and ignores frames until the host has
  written LampArrayControl (feature report ID 6, AutonomousMode = 1) on
  interface 5. Windows' DWM does this itself, so `lumina` skips it there;
  per-LED colours showed correctly on Windows 10 with Klipsch Control
  running [hw, eye]. Sequence: mode 7, 300 ms, state request and read,
  LampArrayControl, 200 ms, frame. Frames closer than ~5 ms make the
  firmware drop to white; 20–50 ms spacing works. A frame stays on the LEDs
  after the host closes the device.
  LampArrayControl with AutonomousMode = 0 makes the firmware jump to mode 8
  (Dynamic Lighting).
- **LED order in a frame:** left satellite first (left as seen facing the
  speakers), each as top 1–3 then bottom 1–3. Seen from the front the top
  row runs 1, 2, 3 left to right and the bottom row 1, 3, 2.
- **Brightness.** LedLevel is the satellites' LED brightness, 0–100; the
  small front LED does not change. The light button's short press steps
  it high, medium, low, off (state 100, 66, 33, 0) and back to high. The
  LedLevel command (0x0F) takes two bytes: a selector, 1 or 2, then the
  brightness. Both bytes of the state then show the brightness, e.g.
  `01 32` gives 50, 50 and visibly half brightness. Swept with state
  readback: selectors 0, 3 and 255 are ignored, and with selector 1 every
  brightness 0–100 is stored and 101–255 ignored [hw, eye]. Selectors 1
  and 2 look the same and both survive a power cycle (30 with selector 1,
  then 60 with selector 2, each read back after power-cycling) [hw];
  untested in Screen React and Dynamic Lighting, what distinguishes them
  is not known [?]. Sending
  the same value twice changes nothing, so it is a level, not a step.
  Earlier tests sent the brightness in both bytes, which only works for
  1 and 2 and made the command look ineffective. Klipsch Control's
  brightness setting showed no visible effect either; what it sends is
  not known.
- **Light button.** A long press (over 1 s) steps LightMode 1–6 in
  order (rainbow, breathe, static, aurora, music react, off), never 7 or
  8, and leaves CustomRGB untouched. Neither press sends a HID report
  [hw, eye].

## Timing quirks [hw]

- Two frames closer than ~5 ms can drop the lighting to white. `lumina`
  spaces all frames at least 50 ms apart, as the OpenRGB driver does for
  colour frames. The app waits 100 ms after a
  subwoofer level write [app]. Back-to-back audio commands are untested.
- A light mode change zeroes the stored custom RGB pair.
- Switching InputSource from Aux to USB PC (2) makes the device drop off
  USB and re-enumerate about 6 s later; the SET_REPORT itself reports an
  error. PipeWire recreates the sink about 20 s after the command. Sending
  USB PC while already on USB PC, and switching to Aux, keep the
  connection. All settings survive the re-enumeration [hw].
- The input button cycles USB, Bluetooth, Aux. As with the command, only
  the switch from Aux back to USB re-enumerates the device: the HID
  interfaces are back about 6 s after the disconnect, the state report
  about 12 s later again, and the sound returns 5–10 s after the switch
  [hw, ear]. Switching to Bluetooth with the button enters pairing
  (BtPairingStatus 1) at once; leaving Bluetooth sets it back to 0 [hw].
  One earlier round showed an extra disconnect, not reproduced [?].
- The front LED shows the input: white for USB, blue for Bluetooth
  (blinking while not paired), green for Aux [eye].
- PowerState 0 puts the speakers in standby: lights off. The device stays
  enumerated, but its USB controller stops answering anything except the
  bus-level presence [hw]:
  - `lumina` calls (enumerate, open, SET_REPORT or GET_REPORT) do not
    return within 20 s; which step blocks is not known yet. The standby
    command itself, sent just before, completes
  - an audio stream stalls: no audible output, `pw-play` blocks
  - a USB port reset (USBDEVFS_RESET) fails: descriptor reads time out
    (-110), the device does not accept an address (-62), and the kernel
    drops it after about a minute; the port then reports "not attached"
  Tried and failed to wake it: PowerState 1, playing audio for 20 s, USB
  port reset. The power button wakes it; it re-attaches on USB about 9 s
  after the press and answers state requests 12 s later, settings kept.
  Klipsch Control's PowerState 1 "wake-up buffer" on connect cannot wake
  this state either, so it presumably targets something else.
  Not tried: Bluetooth or Aux signal, the remote/other buttons.

## Sharing the device

On Linux and FreeBSD only one process can claim interface 4. A running
OpenRGB instance with the Lumina detector holds it, and a VM with the
device passed through holds the whole device. On Windows the HID handle
is shared: `lumina` works while Klipsch Control runs [hw].
