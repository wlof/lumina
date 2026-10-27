//! RGB colours for the lighting commands.

use std::fmt;
use std::str::FromStr;

use crate::Error;

/// An RGB colour, one byte per channel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// All channels off.
    pub const BLACK: Rgb = Rgb(0, 0, 0);

    /// The colour as sent on the wire: red, green, blue.
    #[must_use]
    pub fn bytes(self) -> [u8; 3] {
        [self.0, self.1, self.2]
    }
}

impl From<[u8; 3]> for Rgb {
    fn from([r, g, b]: [u8; 3]) -> Self {
        Rgb(r, g, b)
    }
}

/// Names accepted by [`Rgb::from_str`].
const NAMES: &[(&str, Rgb)] = &[
    ("black", Rgb(0, 0, 0)),
    ("white", Rgb(255, 255, 255)),
    ("red", Rgb(255, 0, 0)),
    ("green", Rgb(0, 255, 0)),
    ("blue", Rgb(0, 0, 255)),
    ("cyan", Rgb(0, 255, 255)),
    ("magenta", Rgb(255, 0, 255)),
    ("purple", Rgb(128, 0, 255)),
    ("yellow", Rgb(255, 255, 0)),
    ("orange", Rgb(255, 127, 0)),
    ("pink", Rgb(255, 0, 127)),
];

impl fmt::Display for Rgb {
    /// `#rrggbb`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
}

impl FromStr for Rgb {
    type Err = Error;

    /// `#rrggbb`, `rrggbb` or a colour name, case insensitive.
    fn from_str(s: &str) -> Result<Self, Error> {
        let lower = s.to_ascii_lowercase();
        if let Some((_, rgb)) = NAMES.iter().find(|(n, _)| *n == lower) {
            return Ok(*rgb);
        }

        let unknown = || Error::Unknown("colour", s.to_owned());
        let hex = lower.strip_prefix('#').unwrap_or(&lower);
        // Digits only: from_str_radix would also take a sign, and slicing a
        // multi-byte character would panic
        if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(unknown());
        }
        let channel = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| unknown());
        Ok(Rgb(channel(0)?, channel(2)?, channel(4)?))
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Rgb {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn parses_hex_and_names() {
        assert_eq!("#ff7f00".parse::<Rgb>().unwrap(), Rgb(255, 127, 0));
        assert_eq!("00FFff".parse::<Rgb>().unwrap(), Rgb(0, 255, 255));
        assert_eq!("Orange".parse::<Rgb>().unwrap(), Rgb(255, 127, 0));
        assert!("#fff".parse::<Rgb>().is_err());
        assert!("teal".parse::<Rgb>().is_err());
        assert!("+1ff00".parse::<Rgb>().is_err());
        assert_eq!(Rgb(1, 2, 255).to_string(), "#0102ff");
    }

    #[test]
    fn rejects_multibyte_input_without_panicking() {
        // 6 bytes, but "é" straddles the first channel boundary
        assert!("aébcd".parse::<Rgb>().is_err());
    }

    proptest! {
        #[test]
        fn display_round_trips(r: u8, g: u8, b: u8) {
            let rgb = Rgb(r, g, b);
            prop_assert_eq!(rgb.to_string().parse::<Rgb>().unwrap(), rgb);
        }

        #[test]
        fn parse_never_panics(s in "\\PC{0,12}") {
            let _ = s.parse::<Rgb>();
        }
    }
}
