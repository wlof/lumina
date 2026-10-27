//! State report read back from the device.
//!
//! Offsets are into the report as it appears on the wire (one less than in
//! a hidapi buffer, which prepends the report ID). Field order follows
//! Klipsch Control's device state properties; see `docs/protocol.md`.

use std::ops::Range;

use crate::Error;
use crate::color::Rgb;
use crate::eq::{self, Bands};
use crate::frame::{self, REPORT_LEN, Report};
use crate::types::{self, BtPairing, FirmwarePreset, InputSource, LightMode, SoundMode};

/// Byte 3 of a state report, where a command frame carries its command ID.
const STATE_MARKER: u8 = 0x7B;

const INPUT_SOURCE: usize = 5;
const BT_PAIRING: usize = 6;
const MASTER_VOLUME: usize = 7;
const SUB_LEVEL: usize = 8;
const PLAY_PAUSE: usize = 9;
const SOUND_MODE: usize = 10;
const FIRMWARE_PRESET: usize = 11;
const LIGHT_MODE: usize = 12;
const SERIAL: Range<usize> = 13..28;
const BT_MAC: usize = 28;
const FIRMWARE: Range<usize> = 34..39;
const MUTE: usize = 39;
const NIGHT_MODE: usize = 40;
const PEQ_MODE: usize = 43;
const AUTO_STANDBY: usize = 44;
const POWER_STATE: usize = 45;
const AUTO_STANDBY_TIMEOUT: usize = 47;
const UI_TONES: usize = 51;
const PRIMARY_RGB: usize = 53;
const SECONDARY_RGB: usize = 56;
const EQ_BANDS: Range<usize> = 59..59 + eq::BANDS_LEN;
const NAME: Range<usize> = 107..122;

/// The shortest reply that carries every field above (123 bytes seen).
const MIN_LEN: usize = NAME.end;

/// Everything the speakers report about themselves.
///
/// Enum fields are `None` when the device reports a value outside the
/// known set.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[expect(
    clippy::struct_excessive_bools,
    reason = "mirrors the on/off fields of the device report"
)]
#[non_exhaustive]
pub struct State {
    /// Model name, `ProMedia Lumina`.
    pub name: String,
    /// Serial number printed on the speakers.
    pub serial: String,
    /// Firmware version, e.g. `1.0.3`.
    pub firmware: String,
    /// Bluetooth MAC address, `aa:bb:cc:dd:ee:ff`.
    pub bt_mac: String,
    /// False only in states the host can still observe; in standby the
    /// speakers do not answer at all.
    pub power_on: bool,
    /// Selected input.
    pub input: Option<InputSource>,
    /// Bluetooth pairing state.
    pub bt_pairing: Option<BtPairing>,
    /// Follows the host playback volume; read only.
    pub master_volume: u8,
    /// True while the host streams audio.
    pub playing: bool,
    /// Speakers muted.
    pub mute: bool,
    /// Sound mode.
    pub sound_mode: Option<SoundMode>,
    /// Night mode (bass reduced).
    pub night_mode: bool,
    /// Subwoofer level in dB, −20…+10.
    pub sub_level: Option<i8>,
    /// The firmware's own EQ preset.
    pub firmware_preset: Option<FirmwarePreset>,
    /// Set by any `ParametricEq` write ("`PeqMode`"); nothing known clears it.
    pub custom_eq: bool,
    /// The six parametric bands.
    pub bands: Bands,
    /// The Klipsch Control preset the bands match, if any.
    pub eq_preset: Option<eq::Preset>,
    /// Automatic standby when no signal is detected.
    pub auto_standby: bool,
    /// Delay before automatic standby, in seconds.
    pub auto_standby_timeout: u16,
    /// Button and status tones.
    pub ui_tones: bool,
    /// Light mode.
    pub light_mode: Option<LightMode>,
    /// Primary and secondary colour used by Static, Breathe, Aurora and
    /// Music React. Cleared by every light mode change.
    pub light_colors: (Rgb, Rgb),
    /// Whole report, zero padded, for fields not decoded.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub raw: Report,
}

impl State {
    /// Decode a state report.
    ///
    /// The device answers with 123 bytes; anything from the last decoded
    /// field up to a full report is accepted.
    pub fn parse(report: &[u8]) -> Result<Self, Error> {
        if !frame::has_header(report) || report.get(3) != Some(&STATE_MARKER) {
            return Err(Error::NoState);
        }
        if report.len() < MIN_LEN || report.len() > REPORT_LEN {
            return Err(Error::BadLength(report.len()));
        }

        let mut raw = [0u8; REPORT_LEN];
        raw[..report.len()].copy_from_slice(report);

        let bands = eq::decode(array(&raw, EQ_BANDS.start));

        Ok(State {
            name: ascii(&raw[NAME]),
            serial: ascii(&raw[SERIAL]),
            firmware: ascii(&raw[FIRMWARE]),
            bt_mac: array::<6>(&raw, BT_MAC)
                .map(|b| format!("{b:02x}"))
                .join(":"),
            power_on: raw[POWER_STATE] == 1,
            input: InputSource::from_value(raw[INPUT_SOURCE]),
            bt_pairing: BtPairing::from_value(raw[BT_PAIRING]),
            master_volume: raw[MASTER_VOLUME],
            playing: raw[PLAY_PAUSE] == 0,
            mute: raw[MUTE] != 0,
            sound_mode: SoundMode::from_value(raw[SOUND_MODE]),
            night_mode: raw[NIGHT_MODE] != 0,
            sub_level: types::sub_level_from_byte(raw[SUB_LEVEL]),
            firmware_preset: FirmwarePreset::from_value(raw[FIRMWARE_PRESET]),
            custom_eq: raw[PEQ_MODE] != 0,
            eq_preset: eq::Preset::matching(&bands),
            bands,
            auto_standby: raw[AUTO_STANDBY] != 0,
            auto_standby_timeout: u16::from_le_bytes(array(&raw, AUTO_STANDBY_TIMEOUT)),
            ui_tones: raw[UI_TONES] != 0,
            light_mode: LightMode::from_value(raw[LIGHT_MODE]),
            light_colors: (
                array(&raw, PRIMARY_RGB).into(),
                array(&raw, SECONDARY_RGB).into(),
            ),
            raw,
        })
    }
}

/// `N` bytes of the report from `start`. Offsets are constants within the
/// report, so this cannot go out of bounds.
fn array<const N: usize>(raw: &Report, start: usize) -> [u8; N] {
    std::array::from_fn(|i| raw[start + i])
}

/// Printable ASCII up to the first other byte, trimmed.
fn ascii(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take_while(|b| b.is_ascii_graphic() || **b == b' ')
        .map(|b| char::from(*b))
        .collect::<String>()
        .trim()
        .to_owned()
}

#[cfg(test)]
pub(crate) mod tests {
    use proptest::prelude::*;

    use super::*;

    /// Report read from the hardware (firmware 1.0.3), 123 bytes.
    pub(crate) fn sample() -> Vec<u8> {
        include_str!("../tests/data/state-1.0.3.hex")
            .split_whitespace()
            .map(|b| u8::from_str_radix(b, 16).unwrap())
            .collect()
    }

    #[test]
    fn parses_hardware_sample() {
        let report = sample();
        assert_eq!(report.len(), 123);
        let s = State::parse(&report).unwrap();

        assert_eq!(s.name, "ProMedia Lumina");
        assert_eq!(s.serial, "123456789012345");
        assert_eq!(s.firmware, "1.0.3");
        assert_eq!(s.bt_mac, "02:00:00:00:00:01");
        assert!(s.power_on);
        assert_eq!(s.input, Some(InputSource::UsbPc));
        assert_eq!(s.bt_pairing, Some(BtPairing::Off));
        assert_eq!(s.master_volume, 13);
        assert!(!s.playing);
        assert!(!s.mute);
        assert_eq!(s.sound_mode, Some(SoundMode::Music));
        assert!(!s.night_mode);
        assert_eq!(s.sub_level, Some(0));
        assert_eq!(s.firmware_preset, Some(FirmwarePreset::Flat));
        assert!(s.custom_eq);
        assert_eq!(s.bands, eq::DEVICE_DEFAULT);
        assert_eq!(s.eq_preset, None);
        assert!(s.auto_standby);
        assert_eq!(s.auto_standby_timeout, 900);
        assert!(s.ui_tones);
        assert_eq!(s.light_mode, Some(LightMode::Static));
        assert_eq!(s.light_colors, (Rgb(255, 0, 255), Rgb(255, 0, 255)));
    }

    #[test]
    fn rejects_zeroed_report() {
        // What the device returns while booting or before a state request
        assert!(matches!(
            State::parse(&[0; REPORT_LEN]),
            Err(Error::NoState)
        ));
    }

    #[test]
    fn rejects_truncated_report() {
        assert!(matches!(
            State::parse(&sample()[..100]),
            Err(Error::BadLength(100))
        ));
    }

    #[test]
    fn accepts_padded_report() {
        let mut report = sample();
        report.resize(REPORT_LEN, 0);
        assert_eq!(State::parse(&report).unwrap().firmware, "1.0.3");
    }

    proptest! {
        /// Reports come from the device; no input may panic the parser.
        #[test]
        fn parse_never_panics(report in proptest::collection::vec(any::<u8>(), 0..300)) {
            let _ = State::parse(&report);
        }

        /// Any report with a valid header and length decodes.
        #[test]
        fn parses_any_valid_report(
            body in proptest::collection::vec(any::<u8>(), MIN_LEN - 4..=REPORT_LEN - 4),
        ) {
            let mut report = vec![0xA5, 0x5A, 0x70, STATE_MARKER];
            report.extend(body);
            prop_assert!(State::parse(&report).is_ok());
        }
    }
}
