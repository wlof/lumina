//! Lighting: colour presets and the LED layout.
//!
//! Each satellite has two rows of three LEDs. Per-LED frames list the left
//! satellite first, each as top 1–3 then bottom 1–3. Seen from the front,
//! the top row runs 1, 2, 3 from left to right but the bottom row runs
//! 1, 3, 2 (verified on hardware for the OpenRGB driver).

use crate::Error;
use crate::color::Rgb;

/// Number of LEDs, six per satellite.
pub const LED_COUNT: usize = 12;

/// Frame position of each LED in physical order: left satellite then
/// right, each top row left to right, then bottom row left to right.
const FRAME_POSITION: [usize; LED_COUNT] = [0, 1, 2, 3, 5, 4, 6, 7, 8, 9, 11, 10];

/// Per-LED colours in physical order: left satellite then right, each top
/// row left to right, then bottom row left to right.
pub type Leds = [Rgb; LED_COUNT];

/// Payload of a `CustomLighting` command.
#[must_use]
pub fn encode_leds(leds: &Leds) -> [u8; LED_COUNT * 3] {
    let mut out = [0u8; LED_COUNT * 3];
    for (physical, rgb) in leds.iter().enumerate() {
        let pos = FRAME_POSITION[physical];
        out[pos * 3..pos * 3 + 3].copy_from_slice(&rgb.bytes());
    }
    out
}

/// Spread 1 (all), 2 (per satellite), 4 (per row) or 12 colours over the LEDs.
pub fn expand(colors: &[Rgb]) -> Result<Leds, Error> {
    let per_led = |i: usize| match colors.len() {
        1 => colors[0],
        2 => colors[i / 6],
        4 => colors[i / 3],
        _ => colors[i],
    };
    match colors.len() {
        1 | 2 | 4 | LED_COUNT => Ok(std::array::from_fn(per_led)),
        n => Err(Error::OutOfRange(
            "number of colours (1, 2, 4 or 12)",
            n.to_string(),
        )),
    }
}

macro_rules! color_presets {
    (
        $(#[$meta:meta])*
        $name:ident, $what:literal {
            $($(#[$vmeta:meta])* $variant:ident = $text:literal, $p:expr, $s:expr;)+
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum $name {
            $($(#[$vmeta])* $variant,)+
        }

        impl $name {
            /// Every preset, in Klipsch Control's order.
            pub const ALL: &[$name] = &[$($name::$variant,)+];

            /// Name as used on the command line.
            #[must_use]
            pub fn name(self) -> &'static str {
                match self {
                    $($name::$variant => $text,)+
                }
            }

            /// Primary and secondary colour sent with `CustomRgb`.
            #[must_use]
            pub fn colors(self) -> (Rgb, Rgb) {
                match self {
                    $($name::$variant => ($p, $s),)+
                }
            }

            /// The preset with exactly these colours, if any.
            #[must_use]
            pub fn matching(primary: Rgb, secondary: Rgb) -> Option<Self> {
                Self::ALL.iter().copied().find(|p| p.colors() == (primary, secondary))
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.name())
            }
        }

        impl std::str::FromStr for $name {
            type Err = Error;

            fn from_str(s: &str) -> Result<Self, Error> {
                let s = s.to_ascii_lowercase().replace(['_', ' ', '/'], "-");
                Self::ALL
                    .iter()
                    .copied()
                    .find(|p| p.name() == s)
                    .ok_or(Error::Unknown($what, s))
            }
        }

        #[cfg(feature = "serde")]
        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(self.name())
            }
        }
    };
}

color_presets! {
    /// Music React only renders these pairs; other colours are stored but
    /// ignored. All black cycles through a default set of colours.
    MusicReactPreset, "Music React preset" {
        /// Cycles through a built-in set of colours.
        Default = "default", Rgb(0, 0, 0), Rgb(0, 0, 0);
        /// Blue and purple.
        BluePurple = "blue-purple", Rgb(0, 0, 255), Rgb(255, 0, 255);
        /// Cyan and blue.
        CyanBlue = "cyan-blue", Rgb(0, 255, 255), Rgb(0, 0, 255);
        /// Red and pink.
        RedPink = "red-pink", Rgb(255, 0, 0), Rgb(255, 0, 127);
        /// Yellow and orange.
        YellowOrange = "yellow-orange", Rgb(255, 255, 0), Rgb(255, 127, 0);
    }
}

color_presets! {
    /// Aurora's two colour sets in Klipsch Control.
    AuroraTone, "Aurora tone" {
        /// Blue and cyan.
        Cool = "cool", Rgb(0, 0, 255), Rgb(0, 255, 255);
        /// Red and orange.
        Warm = "warm", Rgb(255, 0, 0), Rgb(255, 127, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_physical_order_to_frame() {
        let mut leds = [Rgb::BLACK; LED_COUNT];
        // Left satellite, bottom row, middle LED is frame position 5
        leds[4] = Rgb(1, 2, 3);
        // Right satellite, bottom row, right LED is frame position 10
        leds[11] = Rgb(4, 5, 6);
        let frame = encode_leds(&leds);
        assert_eq!(&frame[15..18], &[1, 2, 3]);
        assert_eq!(&frame[30..33], &[4, 5, 6]);
        assert_eq!(frame.iter().filter(|b| **b != 0).count(), 6);
    }

    #[test]
    fn expands_colour_lists() {
        let (r, g, b, w) = (
            Rgb(255, 0, 0),
            Rgb(0, 255, 0),
            Rgb(0, 0, 255),
            Rgb(255, 255, 255),
        );
        assert!(expand(&[r]).unwrap().iter().all(|c| *c == r));
        let two = expand(&[r, g]).unwrap();
        assert!(two[..6].iter().all(|c| *c == r) && two[6..].iter().all(|c| *c == g));
        let four = expand(&[r, g, b, w]).unwrap();
        assert_eq!([four[0], four[3], four[6], four[9]], [r, g, b, w]);
        assert!(expand(&[r, g, b]).is_err());
    }

    #[test]
    fn presets_parse_and_match() {
        assert_eq!(
            "Red/Pink".parse::<MusicReactPreset>().unwrap(),
            MusicReactPreset::RedPink
        );
        let (p, s) = AuroraTone::Warm.colors();
        assert_eq!(AuroraTone::matching(p, s), Some(AuroraTone::Warm));
        assert_eq!(AuroraTone::matching(p, p), None);
    }
}
