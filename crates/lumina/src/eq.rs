//! Parametric EQ: six peaking bands, written with the `ParametricEq` command.
//!
//! Each band is 8 bytes: index, filter type, Fc (Hz, u16 LE), Q × 1024
//! (u16 LE) and gain in dB × 256 (i16 LE), as in Klipsch Control's
//! `EQBand.ToBytes`.

use std::ops::RangeInclusive;
use std::str::FromStr;

use crate::Error;

/// Number of bands.
pub const BAND_COUNT: usize = 6;
/// Encoded size of one band.
pub const BAND_LEN: usize = 8;
/// Encoded size of all bands, the `ParametricEq` payload.
pub const BANDS_LEN: usize = BAND_COUNT * BAND_LEN;

/// Accepted centre frequencies, in Hz (as clamped by Klipsch Control).
pub const FREQ_RANGE: RangeInclusive<u16> = 20..=20000;
/// Accepted Q values.
pub const Q_RANGE: RangeInclusive<f32> = 0.1..=20.0;
/// Klipsch Control limits gains to ±6 dB; so do we.
pub const GAIN_RANGE: RangeInclusive<f32> = -6.0..=6.0;

/// Centre frequencies of the device defaults and of every app preset.
pub const DEFAULT_FREQS: [u16; BAND_COUNT] = [50, 150, 400, 1000, 3500, 8000];

/// One peaking filter.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Band {
    /// Centre frequency in Hz.
    pub freq: u16,
    /// Quality factor; higher is narrower.
    pub q: f32,
    /// Gain in dB.
    pub gain: f32,
    /// Filter type byte; 0 in every known table.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub kind: u8,
}

impl Band {
    /// A band with the usual filter type.
    #[must_use]
    pub const fn new(freq: u16, q: f32, gain: f32) -> Self {
        Band {
            freq,
            q,
            gain,
            kind: 0,
        }
    }

    /// Check frequency, Q and gain against [`FREQ_RANGE`], [`Q_RANGE`] and
    /// [`GAIN_RANGE`]. NaN fails.
    pub fn validate(&self) -> Result<(), Error> {
        if !FREQ_RANGE.contains(&self.freq) {
            return Err(Error::OutOfRange("frequency", self.freq.to_string()));
        }
        if !Q_RANGE.contains(&self.q) {
            return Err(Error::OutOfRange("Q", self.q.to_string()));
        }
        if !GAIN_RANGE.contains(&self.gain) {
            return Err(Error::OutOfRange("gain", self.gain.to_string()));
        }
        Ok(())
    }

    /// Wire form; only for validated bands.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "validate() bounds Q to 0.1..=20 and gain to ±6 dB, so both fit"
    )]
    fn encode(&self, index: u8) -> [u8; BAND_LEN] {
        let q = (self.q * 1024.0).round() as u16;
        let gain = (self.gain * 256.0).round() as i16;
        let mut out = [0u8; BAND_LEN];
        out[0] = index;
        out[1] = self.kind;
        out[2..4].copy_from_slice(&self.freq.to_le_bytes());
        out[4..6].copy_from_slice(&q.to_le_bytes());
        out[6..8].copy_from_slice(&gain.to_le_bytes());
        out
    }

    fn decode(bytes: [u8; BAND_LEN]) -> Self {
        Band {
            freq: u16::from_le_bytes([bytes[2], bytes[3]]),
            q: f32::from(u16::from_le_bytes([bytes[4], bytes[5]])) / 1024.0,
            gain: f32::from(i16::from_le_bytes([bytes[6], bytes[7]])) / 256.0,
            kind: bytes[1],
        }
    }
}

/// All six bands, lowest first.
pub type Bands = [Band; BAND_COUNT];

/// Payload of a `ParametricEq` command. Fails if any band is out of range.
pub fn encode(bands: &Bands) -> Result<[u8; BANDS_LEN], Error> {
    let mut out = [0u8; BANDS_LEN];
    let (chunks, _) = out.as_chunks_mut::<BAND_LEN>();
    for ((index, band), chunk) in (0u8..).zip(bands).zip(chunks) {
        band.validate()?;
        chunk.copy_from_slice(&band.encode(index));
    }
    Ok(out)
}

/// Bands as stored in the state report, ordered by position.
#[must_use]
pub fn decode(bytes: [u8; BANDS_LEN]) -> Bands {
    let (chunks, _) = bytes.as_chunks::<BAND_LEN>();
    std::array::from_fn(|i| Band::decode(chunks[i]))
}

/// The device's own default bands: flat, Q 0.707.
pub const DEVICE_DEFAULT: Bands = {
    let q = 724.0 / 1024.0;
    [
        Band::new(50, q, 0.0),
        Band::new(150, q, 0.0),
        Band::new(400, q, 0.0),
        Band::new(1000, q, 0.0),
        Band::new(3500, q, 0.0),
        Band::new(8000, q, 0.0),
    ]
};

/// Klipsch Control's presets (`Data.EqPresets`). The app applies them by
/// writing these bands, not with the firmware's `EqPreset` command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Preset {
    /// No boost or cut.
    Flat,
    /// Vocal Boost: mids up, lows down.
    Vocal,
    /// Bass Boost.
    Bass,
    /// Treble Boost.
    Treble,
    /// Rock.
    Rock,
    /// Hip Hop.
    HipHop,
}

impl Preset {
    /// Every preset, in Klipsch Control's order.
    pub const ALL: [Preset; 6] = [
        Preset::Flat,
        Preset::Vocal,
        Preset::Bass,
        Preset::Treble,
        Preset::Rock,
        Preset::HipHop,
    ];

    /// Name as used on the command line.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Preset::Flat => "flat",
            Preset::Vocal => "vocal",
            Preset::Bass => "bass",
            Preset::Treble => "treble",
            Preset::Rock => "rock",
            Preset::HipHop => "hip-hop",
        }
    }

    /// The preset's band table.
    #[must_use]
    pub fn bands(self) -> Bands {
        const Q: [f32; BAND_COUNT] = [3.5, 4.0, 4.0, 3.0, 3.5, 4.0];
        let (gains, qs): ([f32; BAND_COUNT], [f32; BAND_COUNT]) = match self {
            Preset::Flat => ([0.0, 0.0, 0.0, 0.0, 0.0, 0.0], Q),
            Preset::Vocal => ([-2.0, -3.0, 2.0, 2.0, 1.0, 0.0], Q),
            Preset::Bass => ([3.0, 2.0, 1.0, 0.0, 0.0, 0.0], Q),
            Preset::Treble => ([0.0, 0.0, 0.0, 1.0, 2.0, 3.0], [1.0; BAND_COUNT]),
            Preset::Rock => ([3.0, 2.0, 3.0, 1.0, -2.0, 4.0], Q),
            Preset::HipHop => ([4.0, 4.0, -2.0, 3.0, 3.0, 2.0], Q),
        };
        std::array::from_fn(|i| Band::new(DEFAULT_FREQS[i], qs[i], gains[i]))
    }

    /// The preset whose bands equal `bands`, if any.
    #[must_use]
    pub fn matching(bands: &Bands) -> Option<Preset> {
        Preset::ALL.into_iter().find(|p| p.bands() == *bands)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Preset {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.name())
    }
}

impl FromStr for Preset {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        let s = s.to_ascii_lowercase().replace(['_', ' '], "-");
        Preset::ALL
            .into_iter()
            .find(|p| p.name() == s || (s == "hiphop" && *p == Preset::HipHop))
            .ok_or(Error::Unknown("EQ preset", s))
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// State bytes 59..107 of a device with default bands.
    const DEFAULT_BYTES: &str = "00 00 32 00 d4 02 00 00 01 00 96 00 d4 02 00 00 \
        02 00 90 01 d4 02 00 00 03 00 e8 03 d4 02 00 00 04 00 ac 0d d4 02 00 00 \
        05 00 40 1f d4 02 00 00";

    fn hex(s: &str) -> Vec<u8> {
        s.split_whitespace()
            .map(|b| u8::from_str_radix(b, 16).unwrap())
            .collect()
    }

    #[test]
    fn decodes_device_defaults() {
        assert_eq!(
            decode(hex(DEFAULT_BYTES).try_into().unwrap()),
            DEVICE_DEFAULT
        );
    }

    #[test]
    fn device_defaults_round_trip() {
        assert_eq!(
            encode(&DEVICE_DEFAULT).unwrap().to_vec(),
            hex(DEFAULT_BYTES)
        );
    }

    #[test]
    fn encodes_gain_and_q() {
        // Bass preset band 0: 50 Hz, Q 3.5, +3 dB
        let bytes = encode(&Preset::Bass.bands()).unwrap();
        assert_eq!(&bytes[..8], &[0, 0, 0x32, 0x00, 0x00, 0x0E, 0x00, 0x03]);
        // Vocal preset band 1: 150 Hz, Q 4, -3 dB
        let bytes = encode(&Preset::Vocal.bands()).unwrap();
        assert_eq!(&bytes[8..16], &[1, 0, 0x96, 0x00, 0x00, 0x10, 0x00, 0xFD]);
    }

    #[test]
    fn presets_round_trip_and_match() {
        for preset in Preset::ALL {
            let decoded = decode(encode(&preset.bands()).unwrap());
            assert_eq!(Preset::matching(&decoded), Some(preset));
        }
        assert_eq!(Preset::matching(&DEVICE_DEFAULT), None);
    }

    #[test]
    fn rejects_out_of_range() {
        let mut bands = Preset::Flat.bands();
        bands[2].gain = 6.5;
        assert!(matches!(encode(&bands), Err(Error::OutOfRange("gain", _))));
        let mut bands = Preset::Flat.bands();
        bands[0].freq = 10;
        assert!(matches!(
            encode(&bands),
            Err(Error::OutOfRange("frequency", _))
        ));
    }

    #[test]
    fn parses_preset_names() {
        assert_eq!("Hip Hop".parse::<Preset>().unwrap(), Preset::HipHop);
        assert_eq!("hiphop".parse::<Preset>().unwrap(), Preset::HipHop);
        assert_eq!("BASS".parse::<Preset>().unwrap(), Preset::Bass);
        assert!("loud".parse::<Preset>().is_err());
    }

    fn band() -> impl Strategy<Value = Band> {
        (FREQ_RANGE, 0.1f32..=20.0, -6.0f32..=6.0).prop_map(|(f, q, g)| Band::new(f, q, g))
    }

    proptest! {
        /// Valid bands survive the trip to the wire within its resolution.
        #[test]
        fn valid_bands_round_trip(bands in proptest::array::uniform6(band())) {
            let decoded = decode(encode(&bands).unwrap());
            for (sent, got) in bands.iter().zip(&decoded) {
                prop_assert_eq!(sent.freq, got.freq);
                prop_assert!((sent.q - got.q).abs() <= 0.5 / 1024.0);
                prop_assert!((sent.gain - got.gain).abs() <= 0.5 / 256.0);
            }
        }

        /// Out-of-range gains never reach the wire.
        #[test]
        fn rejects_excess_gain(gain in prop_oneof![-100.0f32..-6.001, 6.001f32..100.0]) {
            let mut bands = DEVICE_DEFAULT;
            bands[0].gain = gain;
            prop_assert!(encode(&bands).is_err());
        }
    }
}
