//! One-byte settings with named values.

use std::ops::RangeInclusive;

use crate::Error;

/// An enum of named byte values: wire value, CLI name (plus aliases),
/// `Display`, `FromStr` and, with the `serde` feature, `Serialize` using the
/// same names.
macro_rules! byte_enum {
    (
        $(#[$meta:meta])*
        $name:ident, $what:literal {
            $($(#[$vmeta:meta])* $variant:ident = $value:literal, $text:literal $(| $alias:literal)*;)+
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum $name {
            $($(#[$vmeta])* $variant,)+
        }

        impl $name {
            /// Every value, in wire order.
            pub const ALL: &[$name] = &[$($name::$variant,)+];

            /// The byte sent on the wire and found in the state report.
            #[must_use]
            pub fn value(self) -> u8 {
                match self {
                    $($name::$variant => $value,)+
                }
            }

            /// The value for a wire byte, if it is a known one.
            #[must_use]
            pub fn from_value(value: u8) -> Option<Self> {
                match value {
                    $($value => Some($name::$variant),)+
                    _ => None,
                }
            }

            /// Name as used on the command line.
            #[must_use]
            pub fn name(self) -> &'static str {
                match self {
                    $($name::$variant => $text,)+
                }
            }
        }

        #[cfg(feature = "serde")]
        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(self.name())
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.name())
            }
        }

        impl std::str::FromStr for $name {
            type Err = Error;

            /// The name or an alias; case, `_` and spaces are ignored.
            fn from_str(s: &str) -> Result<Self, Error> {
                let s = s.to_ascii_lowercase().replace(['_', ' '], "-");
                match s.as_str() {
                    $($text $(| $alias)* => Ok($name::$variant),)+
                    _ => Err(Error::Unknown($what, s)),
                }
            }
        }
    };
}

byte_enum! {
    /// Sound mode (`SoundMode` command).
    SoundMode, "sound mode" {
        /// Movie.
        Movie = 1, "movie";
        /// Music.
        Music = 2, "music";
        /// Virtual Surround.
        VirtualSurround = 3, "surround" | "virtual-surround";
    }
}

byte_enum! {
    /// The firmware's own EQ presets (`EqPreset` command). Klipsch Control
    /// never sets these, but they are audible and combine with the
    /// parametric bands.
    FirmwarePreset, "firmware preset" {
        /// No boost or cut.
        Flat = 0, "flat";
        /// Vocal.
        Vocal = 1, "vocal";
        /// Bass.
        Bass = 2, "bass";
        /// Rock.
        Rock = 3, "rock";
        /// Treble.
        Treble = 4, "treble";
    }
}

byte_enum! {
    /// Input source (`InputSource` command).
    InputSource, "input" {
        /// Bluetooth.
        Bluetooth = 0, "bluetooth" | "bt";
        /// 3.5 mm analogue input.
        Aux = 1, "aux";
        /// USB audio from the computer.
        UsbPc = 2, "usb" | "usb-pc";
        /// USB storage.
        UsbDisc = 3, "usb-disc";
    }
}

byte_enum! {
    /// Bluetooth pairing state, as reported in the state report.
    BtPairing, "Bluetooth pairing state" {
        /// Not pairing.
        Off = 0, "off";
        /// In pairing mode.
        Pairing = 1, "pairing";
        /// A device is paired.
        Paired = 2, "paired";
    }
}

byte_enum! {
    /// Built-in light modes (`LightMode` command).
    LightMode, "light mode" {
        /// Cycle through all colours.
        Rainbow = 1, "rainbow";
        /// Fade the primary colour in and out.
        Breathe = 2, "breathe";
        /// Steady primary colour.
        Static = 3, "static";
        /// Drifting colour pair.
        Aurora = 4, "aurora";
        /// Follow the audio.
        MusicReact = 5, "music-react";
        /// LEDs off.
        Off = 6, "off";
        /// Per-LED colours from the host ("Screen React" in Klipsch Control).
        Direct = 7, "direct" | "screen-react";
        /// Windows Dynamic Lighting drives the LEDs.
        DynamicLighting = 8, "dynamic-lighting";
    }
}

/// Subwoofer level in dB. Sent and reported as dB + 20, so 0–30 on the wire.
pub const SUB_LEVEL_RANGE: RangeInclusive<i8> = -20..=10;
const SUB_LEVEL_OFFSET: i8 = 20;

pub(crate) fn sub_level_to_byte(db: i8) -> Result<u8, Error> {
    if !SUB_LEVEL_RANGE.contains(&db) {
        return Err(Error::OutOfRange("subwoofer level", db.to_string()));
    }
    // In range, so db + 20 is 0..=30
    u8::try_from(db + SUB_LEVEL_OFFSET)
        .map_err(|_| Error::OutOfRange("subwoofer level", db.to_string()))
}

/// The level for a reported byte, `None` outside 0–30.
pub(crate) fn sub_level_from_byte(value: u8) -> Option<i8> {
    let db = i8::try_from(value).ok()?.checked_sub(SUB_LEVEL_OFFSET)?;
    SUB_LEVEL_RANGE.contains(&db).then_some(db)
}

/// Accepted auto standby delays, in seconds. Klipsch Control offers 2, 5, 10
/// and 15 minutes; the device takes any u16.
pub const STANDBY_TIMEOUT_RANGE: RangeInclusive<u16> = 60..=3600;

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn values_round_trip() {
        for mode in SoundMode::ALL {
            assert_eq!(SoundMode::from_value(mode.value()), Some(*mode));
        }
        assert_eq!(SoundMode::from_value(0), None);
        assert_eq!(InputSource::UsbPc.value(), 2);
        assert_eq!(FirmwarePreset::Treble.value(), 4);
    }

    #[test]
    fn parses_names_and_aliases() {
        assert_eq!(
            "Virtual Surround".parse::<SoundMode>().unwrap(),
            SoundMode::VirtualSurround
        );
        assert_eq!(
            "surround".parse::<SoundMode>().unwrap(),
            SoundMode::VirtualSurround
        );
        assert_eq!("BT".parse::<InputSource>().unwrap(), InputSource::Bluetooth);
        assert!("hdmi".parse::<InputSource>().is_err());
    }

    #[test]
    fn sub_level_encoding() {
        assert_eq!(sub_level_to_byte(0).unwrap(), 20);
        assert_eq!(sub_level_to_byte(-20).unwrap(), 0);
        assert_eq!(sub_level_to_byte(10).unwrap(), 30);
        assert!(sub_level_to_byte(11).is_err());
        assert_eq!(sub_level_from_byte(10), Some(-10));
        assert_eq!(sub_level_from_byte(31), None);
        assert_eq!(sub_level_from_byte(200), None);
    }

    proptest! {
        #[test]
        fn sub_level_round_trips(db in SUB_LEVEL_RANGE) {
            prop_assert_eq!(sub_level_from_byte(sub_level_to_byte(db).unwrap()), Some(db));
        }

        #[test]
        fn sub_level_decodes_any_byte(value: u8) {
            let _ = sub_level_from_byte(value);
        }
    }
}
