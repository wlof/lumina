//! Command IDs of the vendor protocol.
//!
//! The table matches `Data.Commands` in Klipsch Control 1.11.0. Payloads are
//! one byte unless noted; see `docs/protocol.md` for values.

/// A command ID, byte 3 of an output frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub enum Command {
    /// Input source, see [`InputSource`](crate::InputSource).
    InputSource = 0x01,
    /// Bluetooth pairing mode: 0 leaves, 1 enters (Bluetooth input only).
    BtPairing = 0x02,
    /// Not sent by Klipsch Control, which sets the OS volume instead.
    MasterVolume = 0x03,
    /// Subwoofer level as dB + 20, 0–30.
    SubwooferLevel = 0x04,
    /// 0 play, 1 pause, 2 toggle.
    PlayPause = 0x05,
    /// Next track.
    NextTrack = 0x06,
    /// Previous track.
    PreviousTrack = 0x07,
    /// Sound mode, see [`SoundMode`](crate::SoundMode).
    SoundMode = 0x08,
    /// Firmware preset 0–4. Not sent by Klipsch Control.
    EqPreset = 0x09,
    /// 48 bytes, 6 × 8-byte bands, see [`eq`](crate::eq).
    ParametricEq = 0x0A,
    /// Light mode, see [`LightMode`](crate::LightMode).
    LightMode = 0x0B,
    /// 0 or 1.
    Mute = 0x0C,
    /// 0 or 1.
    NightMode = 0x0D,
    /// 2 bytes. Brightness; accepted but has no visible effect.
    LedLevel = 0x0F,
    /// Button and status tones, 0 or 1.
    UiTones = 0x10,
    /// 36 bytes, 12 × RGB, see [`light`](crate::light).
    CustomLighting = 0x11,
    /// 0 or 1.
    AutoStandby = 0x14,
    /// 0 standby, 1 on. Standby cannot be left over USB.
    PowerState = 0x16,
    /// 2 bytes, seconds, little endian.
    AutoStandbyTimeout = 0x18,
    /// 1. Also forgets Bluetooth pairings.
    FactoryReset = 0x19,
    /// 6 bytes, primary and secondary RGB for the built-in light modes.
    CustomRgb = 0x1A,
}

impl Command {
    /// The byte sent on the wire.
    #[must_use]
    pub fn id(self) -> u8 {
        self as u8
    }
}
