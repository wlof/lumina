//! Typed access to the speakers' settings.

use std::thread;
use std::time::{Duration, Instant};

use crate::color::Rgb;
use crate::eq::{self, Bands};
use crate::frame;
use crate::light::{self, AuroraTone, Leds, MusicReactPreset};
use crate::transport::{HidTransport, Transport};
use crate::types::{self, FirmwarePreset, InputSource, LightMode, SoundMode};
use crate::{Command, Error, State};

/// The device ignores requests for about 12 s after power up or a USB reset.
const STATE_TIMEOUT: Duration = Duration::from_secs(15);
const STATE_DELAY: Duration = Duration::from_millis(100);

/// Spacing between frames. Lighting frames closer than ~5 ms are dropped;
/// Klipsch Control waits 100 ms after a subwoofer level write.
const FRAME_INTERVAL: Duration = Duration::from_millis(50);

/// After a light mode change, before colours or the LED unlock.
const MODE_DELAY: Duration = Duration::from_millis(300);
/// After the LED unlock, before the first per-LED frame.
const UNLOCK_DELAY: Duration = Duration::from_millis(200);

/// A connected pair of speakers.
///
/// Setters send one command each and return as soon as it is written; read
/// [`state`](Lumina::state) afterwards to confirm. Writes are spaced at
/// least 50 ms apart.
#[derive(Debug)]
pub struct Lumina<T: Transport = HidTransport> {
    transport: T,
    last_write: Option<Instant>,
}

impl Lumina {
    /// Open the first connected Lumina, or the one with the given USB serial.
    pub fn open(serial: Option<&str>) -> Result<Self, Error> {
        Ok(Lumina::with_transport(HidTransport::open(serial)?))
    }
}

impl<T: Transport> Lumina<T> {
    /// Use another transport, e.g. a mock in tests.
    pub fn with_transport(transport: T) -> Self {
        Lumina {
            transport,
            last_write: None,
        }
    }

    /// The transport, e.g. to inspect a simulator in tests.
    pub fn transport(&self) -> &T {
        &self.transport
    }

    /// Request and read the state report, waiting for the device to boot.
    pub fn state(&mut self) -> Result<State, Error> {
        let start = Instant::now();

        loop {
            self.write(&frame::state_request())?;
            thread::sleep(STATE_DELAY);

            match State::parse(&self.transport.read()?) {
                Err(Error::NoState) if start.elapsed() < STATE_TIMEOUT => {}
                result => return result,
            }
        }
    }

    /// Movie, Music or Virtual Surround.
    pub fn set_sound_mode(&mut self, mode: SoundMode) -> Result<(), Error> {
        self.send(Command::SoundMode, &[mode.value()])
    }

    /// The firmware's own preset. It combines with the parametric bands.
    pub fn set_firmware_preset(&mut self, preset: FirmwarePreset) -> Result<(), Error> {
        self.send(Command::EqPreset, &[preset.value()])
    }

    /// Write all six parametric bands. Sets the device's custom EQ flag,
    /// which nothing known clears.
    pub fn set_bands(&mut self, bands: &Bands) -> Result<(), Error> {
        self.send(Command::ParametricEq, &eq::encode(bands)?)
    }

    /// Apply one of Klipsch Control's presets, the way the app does.
    pub fn set_eq_preset(&mut self, preset: eq::Preset) -> Result<(), Error> {
        self.set_bands(&preset.bands())
    }

    /// Subwoofer level in dB, −20…+10.
    pub fn set_sub_level(&mut self, db: i8) -> Result<(), Error> {
        self.send(Command::SubwooferLevel, &[types::sub_level_to_byte(db)?])
    }

    /// Night mode turns the bass down by half.
    pub fn set_night_mode(&mut self, on: bool) -> Result<(), Error> {
        self.send(Command::NightMode, &[on.into()])
    }

    /// Mute or unmute the speakers.
    pub fn set_mute(&mut self, on: bool) -> Result<(), Error> {
        self.send(Command::Mute, &[on.into()])
    }

    /// Button and status tones ("Function Sound").
    pub fn set_ui_tones(&mut self, on: bool) -> Result<(), Error> {
        self.send(Command::UiTones, &[on.into()])
    }

    /// Go to standby automatically when no signal is detected.
    pub fn set_auto_standby(&mut self, on: bool) -> Result<(), Error> {
        self.send(Command::AutoStandby, &[on.into()])
    }

    /// Delay before automatic standby, within [`STANDBY_TIMEOUT_RANGE`](crate::STANDBY_TIMEOUT_RANGE).
    pub fn set_auto_standby_timeout(&mut self, seconds: u16) -> Result<(), Error> {
        if !types::STANDBY_TIMEOUT_RANGE.contains(&seconds) {
            return Err(Error::OutOfRange("standby timeout", format!("{seconds} s")));
        }
        self.send(Command::AutoStandbyTimeout, &seconds.to_le_bytes())
    }

    /// Switch input. Switching to USB from another input makes the device
    /// drop off USB and come back about 6 s later; a failed transfer is then
    /// reported as [`Error::Reconnecting`] and this handle is dead.
    pub fn set_input(&mut self, input: InputSource) -> Result<(), Error> {
        match self.send(Command::InputSource, &[input.value()]) {
            Err(Error::NoResponse(_)) if input == InputSource::UsbPc => Err(Error::Reconnecting),
            result => result,
        }
    }

    /// Enter or leave Bluetooth pairing mode. The speakers ignore this
    /// unless the input is [`InputSource::Bluetooth`].
    pub fn set_bt_pairing(&mut self, on: bool) -> Result<(), Error> {
        self.send(Command::BtPairing, &[on.into()])
    }

    /// Put the speakers in standby. They then stop answering on USB
    /// entirely; only the power button wakes them.
    pub fn standby(&mut self) -> Result<(), Error> {
        self.send(Command::PowerState, &[0])
    }

    /// Restore factory settings, including forgetting Bluetooth pairings.
    pub fn factory_reset(&mut self) -> Result<(), Error> {
        self.send(Command::FactoryReset, &[1])
    }

    /// Switch light mode. This clears the stored colour pair, so set colours
    /// afterwards; the helpers below do both.
    pub fn set_light_mode(&mut self, mode: LightMode) -> Result<(), Error> {
        self.send(Command::LightMode, &[mode.value()])?;
        thread::sleep(MODE_DELAY);
        Ok(())
    }

    /// Primary and secondary colour for Static, Breathe, Aurora and Music
    /// React. Static and Breathe show only the primary.
    pub fn set_light_colors(&mut self, primary: Rgb, secondary: Rgb) -> Result<(), Error> {
        let mut data = [0u8; 6];
        data[..3].copy_from_slice(&primary.bytes());
        data[3..].copy_from_slice(&secondary.bytes());
        self.send(Command::CustomRgb, &data)
    }

    /// Steady colour.
    pub fn light_static(&mut self, color: Rgb) -> Result<(), Error> {
        self.set_light_mode(LightMode::Static)?;
        self.set_light_colors(color, color)
    }

    /// One colour fading in and out.
    pub fn light_breathe(&mut self, color: Rgb) -> Result<(), Error> {
        self.set_light_mode(LightMode::Breathe)?;
        self.set_light_colors(color, color)
    }

    /// Drifting colours, cool or warm.
    pub fn light_aurora(&mut self, tone: AuroraTone) -> Result<(), Error> {
        let (p, s) = tone.colors();
        self.set_light_mode(LightMode::Aurora)?;
        self.set_light_colors(p, s)
    }

    /// Lights following the audio, in one of the presets the firmware renders.
    pub fn light_music_react(&mut self, preset: MusicReactPreset) -> Result<(), Error> {
        let (p, s) = preset.colors();
        self.set_light_mode(LightMode::MusicReact)?;
        self.set_light_colors(p, s)
    }

    /// Set every LED, switching to Direct mode first if needed. The frame
    /// stays after the handle is closed. The firmware drops frames closer
    /// than a few milliseconds apart; writes are spaced 50 ms apart.
    pub fn set_leds(&mut self, leds: &Leds) -> Result<(), Error> {
        if self.state()?.light_mode != Some(LightMode::Direct) {
            self.set_light_mode(LightMode::Direct)?;
            self.state()?;
        }
        // Each handle has to unlock again, even if already in Direct mode
        self.transport.unlock_leds()?;
        thread::sleep(UNLOCK_DELAY);
        self.send(Command::CustomLighting, &light::encode_leds(leds))
    }

    /// Send a frame with an arbitrary command ID, for protocol exploration.
    pub fn send_raw(&mut self, cmd: u8, data: &[u8]) -> Result<(), Error> {
        self.write(&frame::encode(cmd, data)?)
    }

    fn send(&mut self, cmd: Command, data: &[u8]) -> Result<(), Error> {
        self.send_raw(cmd.id(), data)
    }

    fn write(&mut self, report: &frame::Report) -> Result<(), Error> {
        if let Some(wait) = self
            .last_write
            .and_then(|last| FRAME_INTERVAL.checked_sub(last.elapsed()))
        {
            thread::sleep(wait);
        }

        let result = self.transport.write(report);
        self.last_write = Some(Instant::now());
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::Report;
    use crate::state;

    #[derive(Debug, Default)]
    struct Mock {
        written: Vec<Report>,
        replies: Vec<Vec<u8>>,
        fail_writes: bool,
        unlocks: usize,
    }

    impl Transport for Mock {
        fn write(&mut self, report: &Report) -> Result<(), Error> {
            if self.fail_writes {
                return Err(Error::NoResponse("mock".into()));
            }
            self.written.push(*report);
            Ok(())
        }

        fn read(&mut self) -> Result<Vec<u8>, Error> {
            Ok(if self.replies.is_empty() {
                vec![0; 64]
            } else {
                self.replies.remove(0)
            })
        }

        fn unlock_leds(&mut self) -> Result<(), Error> {
            self.unlocks += 1;
            Ok(())
        }
    }

    fn lumina() -> Lumina<Mock> {
        Lumina::with_transport(Mock::default())
    }

    /// Command ID and payload of each command frame written, skipping
    /// state requests (bare headers).
    fn frames(l: &Lumina<Mock>) -> Vec<(u8, Vec<u8>)> {
        l.transport
            .written
            .iter()
            .filter(|r| r[4] != 0)
            .map(|r| {
                let len = r[4] as usize;
                (r[3], r[6..len - 1].to_vec())
            })
            .collect()
    }

    #[test]
    fn setters_send_expected_frames() {
        let mut l = lumina();
        l.set_sound_mode(SoundMode::VirtualSurround).unwrap();
        l.set_firmware_preset(FirmwarePreset::Bass).unwrap();
        l.set_sub_level(-10).unwrap();
        l.set_night_mode(true).unwrap();
        l.set_mute(false).unwrap();
        l.set_ui_tones(true).unwrap();
        l.set_auto_standby(false).unwrap();
        l.set_auto_standby_timeout(900).unwrap();
        l.set_input(InputSource::Aux).unwrap();
        l.set_bt_pairing(true).unwrap();
        l.standby().unwrap();

        assert_eq!(
            frames(&l),
            vec![
                (0x08, vec![3]),
                (0x09, vec![2]),
                (0x04, vec![10]),
                (0x0D, vec![1]),
                (0x0C, vec![0]),
                (0x10, vec![1]),
                (0x14, vec![0]),
                (0x18, vec![0x84, 0x03]),
                (0x01, vec![1]),
                (0x02, vec![1]),
                (0x16, vec![0]),
            ]
        );
    }

    #[test]
    fn eq_preset_writes_bands() {
        let mut l = lumina();
        l.set_eq_preset(eq::Preset::Rock).unwrap();
        let sent = frames(&l);
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].0, 0x0A);
        assert_eq!(
            eq::decode(sent[0].1.clone().try_into().unwrap()),
            eq::Preset::Rock.bands()
        );
    }

    #[test]
    fn rejects_out_of_range_without_sending() {
        let mut l = lumina();
        assert!(l.set_sub_level(11).is_err());
        assert!(l.set_auto_standby_timeout(10).is_err());
        assert_eq!(l.transport.written.len(), 0);
    }

    #[test]
    fn state_retries_until_report() {
        let mut l = lumina();
        l.transport.replies = vec![vec![0; 64], state::tests::sample()];
        let s = l.state().unwrap();
        assert_eq!(s.firmware, "1.0.3");
        // Two state requests, each a bare header
        assert_eq!(l.transport.written.len(), 2);
        assert!(
            l.transport
                .written
                .iter()
                .all(|r| r[..4] == [0xA5, 0x5A, 0x70, 0])
        );
    }

    #[test]
    fn light_modes_send_mode_then_colors() {
        let mut l = lumina();
        l.light_static(Rgb(1, 2, 3)).unwrap();
        l.light_aurora(AuroraTone::Warm).unwrap();
        l.light_music_react(MusicReactPreset::Default).unwrap();
        assert_eq!(
            frames(&l),
            vec![
                (0x0B, vec![3]),
                (0x1A, vec![1, 2, 3, 1, 2, 3]),
                (0x0B, vec![4]),
                (0x1A, vec![255, 0, 0, 255, 127, 0]),
                (0x0B, vec![5]),
                (0x1A, vec![0; 6]),
            ]
        );
    }

    #[test]
    fn set_leds_enters_direct_and_unlocks() {
        let mut l = lumina();
        // The sample is in Static mode
        l.transport.replies = vec![state::tests::sample(), state::tests::sample()];
        let leds = [Rgb(9, 9, 9); light::LED_COUNT];
        l.set_leds(&leds).unwrap();

        let sent = frames(&l);
        assert_eq!(sent.len(), 2);
        assert_eq!(sent[0], (0x0B, vec![7]));
        assert_eq!(sent[1], (0x11, vec![9; 36]));
        assert_eq!(l.transport.unlocks, 1);
    }

    #[test]
    fn usb_input_failure_means_reconnecting() {
        let mut l = lumina();
        l.transport.fail_writes = true;
        assert!(matches!(
            l.set_input(InputSource::UsbPc),
            Err(Error::Reconnecting)
        ));
        assert!(matches!(
            l.set_input(InputSource::Aux),
            Err(Error::NoResponse(_))
        ));
    }
}
