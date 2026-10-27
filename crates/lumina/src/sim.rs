//! A simulated pair of speakers, for tests and for developing front ends
//! without hardware.
//!
//! [`Simulator`] implements [`Transport`] and applies each command to a
//! state report the way the real speakers do, as recorded in
//! `docs/protocol.md`. It checks every frame's header, length and checksum
//! and fails on malformed ones, so it also tests the encoder.
//!
//! ```
//! use lumina::{Lumina, SoundMode, sim::Simulator};
//!
//! let mut speakers = Lumina::with_transport(Simulator::new());
//! speakers.set_sound_mode(SoundMode::Movie)?;
//! assert_eq!(speakers.state()?.sound_mode, Some(SoundMode::Movie));
//! # Ok::<(), lumina::Error>(())
//! ```

use crate::frame::{HEADER, REPORT_LEN, Report};
use crate::{Command, Error, Transport};

/// State report of real speakers (firmware 1.0.3), the simulator's start.
const INITIAL: &str = include_str!("../tests/data/state-1.0.3.hex");

/// Length of the state report the real speakers send.
const REPLY_LEN: usize = 123;

/// Simulated speakers. See the [module documentation](self).
#[derive(Clone, Debug)]
pub struct Simulator {
    state: Report,
    leds: Option<[u8; 36]>,
    leds_unlocked: bool,
    standby: bool,
    /// Every command frame received, as (command ID, payload).
    pub received: Vec<(u8, Vec<u8>)>,
}

impl Default for Simulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Simulator {
    /// Speakers in the state captured from real hardware: USB input, Music,
    /// default bands, static magenta light.
    #[must_use]
    pub fn new() -> Self {
        let mut state = [0u8; REPORT_LEN];
        for (byte, hex) in state.iter_mut().zip(INITIAL.split_whitespace()) {
            // The fixture is a constant; a bad byte would fail every test
            *byte = u8::from_str_radix(hex, 16).unwrap_or_default();
        }
        Simulator {
            state,
            leds: None,
            leds_unlocked: false,
            standby: false,
            received: Vec::new(),
        }
    }

    /// The per-LED frame on display, if any, in wire order.
    #[must_use]
    pub fn leds(&self) -> Option<&[u8; 36]> {
        self.leds.as_ref()
    }

    /// Whether the speakers were put in standby. They then stop answering,
    /// as the real ones do.
    #[must_use]
    pub fn in_standby(&self) -> bool {
        self.standby
    }

    fn apply(&mut self, cmd: u8, data: &[u8]) {
        let byte = data.first().copied().unwrap_or_default();
        let s = &mut self.state;
        match cmd {
            c if c == Command::InputSource.id() && byte <= 3 => s[5] = byte,
            // Pairing only works on the Bluetooth input
            c if c == Command::BtPairing.id() && s[5] == 0 => s[6] = byte.min(1),
            c if c == Command::SubwooferLevel.id() && byte <= 30 => s[8] = byte,
            c if c == Command::SoundMode.id() && (1..=3).contains(&byte) => s[10] = byte,
            c if c == Command::EqPreset.id() && byte <= 4 => s[11] = byte,
            c if c == Command::ParametricEq.id() && data.len() == 48 => {
                s[59..107].copy_from_slice(data);
                s[43] = 1;
            }
            c if c == Command::LightMode.id() && (1..=8).contains(&byte) => {
                s[12] = byte;
                // A mode change clears the colour pair
                s[53..59].fill(0);
            }
            c if c == Command::Mute.id() => s[39] = byte,
            c if c == Command::NightMode.id() => s[40] = byte,
            c if c == Command::UiTones.id() => s[51] = byte,
            c if c == Command::CustomLighting.id() && data.len() == 36 => {
                // Shown only in Direct mode, after the unlock
                if s[12] == 7 && self.leds_unlocked {
                    self.leds = data.try_into().ok();
                }
            }
            c if c == Command::AutoStandby.id() => s[44] = byte,
            c if c == Command::PowerState.id() && byte == 0 => self.standby = true,
            c if c == Command::AutoStandbyTimeout.id() && data.len() == 2 => {
                s[47..49].copy_from_slice(data);
            }
            c if c == Command::FactoryReset.id() => *self = Simulator::new(),
            c if c == Command::CustomRgb.id() && data.len() == 6 => {
                s[53..59].copy_from_slice(data);
            }
            // Unknown commands and invalid values are ignored, like the
            // firmware does
            _ => {}
        }
    }
}

impl Transport for Simulator {
    fn write(&mut self, report: &Report) -> Result<(), Error> {
        if self.standby {
            return Err(Error::NoResponse("simulated standby".into()));
        }
        if report[..3] != HEADER {
            return Err(Error::NoResponse("malformed frame: bad header".into()));
        }
        let cmd = report[3];
        let len = usize::from(report[4]);
        // A bare header is a state request
        if cmd == 0 && len == 0 {
            return Ok(());
        }
        if !(7..=REPORT_LEN).contains(&len) {
            return Err(Error::NoResponse("malformed frame: bad length".into()));
        }
        let sum = report[..len]
            .iter()
            .fold(0u8, |acc, b| acc.wrapping_add(*b));
        if sum != 0 {
            return Err(Error::NoResponse("malformed frame: bad checksum".into()));
        }
        let data = &report[6..len - 1];
        self.received.push((cmd, data.to_vec()));
        self.apply(cmd, data);
        Ok(())
    }

    fn read(&mut self) -> Result<Vec<u8>, Error> {
        if self.standby {
            return Err(Error::NoResponse("simulated standby".into()));
        }
        Ok(self.state[..REPLY_LEN].to_vec())
    }

    fn unlock_leds(&mut self) -> Result<(), Error> {
        self.leds_unlocked = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eq::Preset;
    use crate::light::{self, AuroraTone};
    use crate::{InputSource, LightMode, Lumina, Rgb};

    fn speakers() -> Lumina<Simulator> {
        Lumina::with_transport(Simulator::new())
    }

    #[test]
    fn starts_from_the_hardware_capture() {
        let s = speakers().state().unwrap();
        assert_eq!(s.firmware, "1.0.3");
        assert_eq!(s.input, Some(InputSource::UsbPc));
    }

    #[test]
    fn applies_settings() {
        let mut l = speakers();
        l.set_sub_level(-7).unwrap();
        l.set_eq_preset(Preset::Bass).unwrap();
        l.light_aurora(AuroraTone::Warm).unwrap();
        let s = l.state().unwrap();
        assert_eq!(s.sub_level, Some(-7));
        assert_eq!(s.eq_preset, Some(Preset::Bass));
        assert_eq!(s.light_mode, Some(LightMode::Aurora));
        assert_eq!(s.light_colors, AuroraTone::Warm.colors());
    }

    #[test]
    fn ignores_pairing_off_bluetooth() {
        let mut l = speakers();
        l.set_bt_pairing(true).unwrap();
        assert_eq!(l.state().unwrap().bt_pairing, Some(crate::BtPairing::Off));
        l.set_input(InputSource::Bluetooth).unwrap();
        l.set_bt_pairing(true).unwrap();
        assert_eq!(
            l.state().unwrap().bt_pairing,
            Some(crate::BtPairing::Pairing)
        );
    }

    #[test]
    fn shows_leds_after_unlock() {
        let mut l = speakers();
        l.set_leds(&light::expand(&[Rgb(1, 2, 3)]).unwrap())
            .unwrap();
        assert_eq!(l.state().unwrap().light_mode, Some(LightMode::Direct));
        assert_eq!(
            l.transport().leds(),
            Some(&[1, 2, 3].repeat(12).try_into().unwrap())
        );
    }

    #[test]
    fn stops_answering_in_standby() {
        let mut l = speakers();
        l.standby().unwrap();
        assert!(matches!(l.state(), Err(Error::NoResponse(_))));
    }

    #[test]
    fn rejects_bad_checksums() {
        let mut sim = Simulator::new();
        let mut report = crate::frame::encode(Command::Mute.id(), &[1]).unwrap();
        report[7] ^= 1;
        assert!(sim.write(&report).is_err());
    }
}
