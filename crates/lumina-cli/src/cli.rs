//! Command line definition.

use std::str::FromStr;

use clap::{Parser, Subcommand, ValueEnum};
use lumina::eq::Preset;
use lumina::light::{AuroraTone, MusicReactPreset};
use lumina::{FirmwarePreset, InputSource, Rgb, SoundMode};

/// Control Klipsch ProMedia Lumina speakers.
///
/// Setting commands without a value print the current one.
#[derive(Debug, Parser)]
#[command(version)]
pub(crate) struct Cli {
    /// USB serial number of the speakers to control, if several are connected.
    #[arg(long, global = true)]
    pub(crate) serial: Option<String>,

    /// Give up if the speakers do not answer within this many seconds.
    #[arg(long, global = true, value_name = "SECS", default_value_t = 20)]
    pub(crate) timeout: u64,

    #[command(subcommand)]
    pub(crate) command: Cmd,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Cmd {
    /// Show all settings.
    Status {
        #[arg(long)]
        json: bool,
    },

    /// Equalizer: presets and the six parametric bands.
    #[command(subcommand)]
    Eq(EqCmd),

    /// Sound mode: movie, music or surround.
    SoundMode {
        #[arg(value_parser = parse::<SoundMode>, value_name = "movie|music|surround")]
        mode: Option<SoundMode>,
    },

    /// Subwoofer level in dB, -20 to +10.
    Sub {
        #[arg(allow_negative_numbers = true, value_name = "DB")]
        level: Option<i8>,
    },

    /// Night mode: turns the bass down by half.
    NightMode { state: Option<OnOff> },

    /// Mute the speakers: on, off or toggle.
    Mute { state: Option<MuteArg> },

    /// Button and status tones ("Function Sound").
    Tones { state: Option<OnOff> },

    /// Go to standby when no signal is detected.
    AutoStandby {
        state: Option<OnOff>,
        /// Minutes of silence before standby (Klipsch Control offers 2, 5, 10, 15).
        #[arg(long, value_name = "MIN")]
        after: Option<u16>,
    },

    /// Input: bt, aux, usb or usb-disc.
    Input {
        #[arg(value_parser = parse::<InputSource>, value_name = "bt|aux|usb|usb-disc")]
        input: Option<InputSource>,
    },

    /// Bluetooth pairing mode. Needs the Bluetooth input.
    BtPair { state: Option<OnOff> },

    /// Lighting: built-in modes and per-LED colours. Shows the current mode
    /// without a subcommand.
    ///
    /// Colours are `#rrggbb`, `rrggbb` or a name: black, white, red, green,
    /// blue, cyan, magenta, purple, yellow, orange, pink.
    Light {
        #[command(subcommand)]
        cmd: Option<LightCmd>,
    },

    /// Power: standby.
    #[command(subcommand)]
    Power(PowerCmd),

    /// Restore factory settings. Also forgets Bluetooth pairings.
    FactoryReset {
        #[arg(long)]
        yes: bool,
    },

    /// Protocol exploration tools.
    #[command(subcommand)]
    Debug(DebugCmd),
}

#[derive(Clone, Copy, Debug, Subcommand)]
pub(crate) enum EqCmd {
    /// Apply one of Klipsch Control's presets, or show the current one.
    Preset {
        #[arg(value_parser = parse::<Preset>, value_name = "flat|vocal|bass|treble|rock|hip-hop")]
        preset: Option<Preset>,
    },

    /// Show the six bands.
    Bands {
        #[arg(long)]
        json: bool,
    },

    /// Change one band, keeping the others.
    Set {
        /// Band number, 1 to 6.
        #[arg(value_parser = clap::value_parser!(u8).range(1..=6))]
        band: u8,
        /// Centre frequency in Hz, 20 to 20000.
        #[arg(long, value_name = "HZ")]
        freq: Option<u16>,
        /// Gain in dB, -6 to +6.
        #[arg(long, allow_negative_numbers = true, value_name = "DB")]
        gain: Option<f32>,
        /// Q, 0.1 to 20.
        #[arg(long)]
        q: Option<f32>,
    },

    /// Restore the device's default bands (flat, Q 0.707).
    Reset,

    /// The firmware's own preset, which combines with the bands.
    Firmware {
        #[arg(value_parser = parse::<FirmwarePreset>, value_name = "flat|vocal|bass|rock|treble")]
        preset: Option<FirmwarePreset>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum LightCmd {
    /// One steady colour.
    Static {
        #[arg(value_parser = parse::<Rgb>)]
        color: Rgb,
    },

    /// Fade one colour in and out.
    Breathe {
        #[arg(value_parser = parse::<Rgb>)]
        color: Rgb,
    },

    /// Slow drifting colours, cool (blue/cyan) or warm (red/orange).
    Aurora {
        #[arg(value_parser = parse::<AuroraTone>, value_name = "cool|warm", default_value = "cool")]
        tone: AuroraTone,
    },

    /// Follow the music. The firmware only renders these colour pairs.
    MusicReact {
        #[arg(
            value_parser = parse::<MusicReactPreset>,
            value_name = "default|blue-purple|cyan-blue|red-pink|yellow-orange",
            default_value = "default"
        )]
        preset: MusicReactPreset,
    },

    /// Cycle through all colours.
    Rainbow,

    /// LEDs off.
    Off,

    /// Set the LEDs individually: 1 colour for all, 2 (left and right
    /// satellite), 4 (left top, left bottom, right top, right bottom) or 12
    /// (each satellite top row then bottom row, left to right as seen from
    /// the front).
    Leds {
        #[arg(value_parser = parse::<Rgb>, num_args = 1..=12, required = true)]
        colors: Vec<Rgb>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum PowerCmd {
    /// Put the speakers in standby. Only their power button wakes them.
    Standby {
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum DebugCmd {
    /// Print the raw state report.
    Dump,

    /// Send a frame with an arbitrary command ID and payload, then dump the state.
    Send {
        /// Command ID, e.g. 0x0D.
        #[arg(value_parser = parse_byte)]
        cmd: u8,
        /// Payload bytes, e.g. 01 or 0x01 0x00.
        #[arg(value_parser = parse_byte)]
        data: Vec<u8>,
        /// Only send the frame, do not read the state back.
        #[arg(long)]
        no_read: bool,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum OnOff {
    On,
    Off,
}

impl From<OnOff> for bool {
    fn from(v: OnOff) -> bool {
        matches!(v, OnOff::On)
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum MuteArg {
    On,
    Off,
    Toggle,
}

fn parse<T: FromStr<Err = lumina::Error>>(s: &str) -> Result<T, String> {
    s.parse().map_err(|e: lumina::Error| e.to_string())
}

/// A hex byte, with or without `0x`.
fn parse_byte(s: &str) -> Result<u8, String> {
    let hex = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
    // from_str_radix would also accept a sign
    if hex.is_empty() || hex.len() > 2 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("not a hex byte: {s}"));
    }
    u8::from_str_radix(hex, 16).map_err(|_| format!("not a hex byte: {s}"))
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn command_definition_is_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_negative_values() {
        let cli = Cli::try_parse_from(["lumina", "sub", "-3"]).unwrap();
        assert!(matches!(cli.command, Cmd::Sub { level: Some(-3) }));
        let cli = Cli::try_parse_from(["lumina", "eq", "set", "2", "--gain", "-1.5"]).unwrap();
        assert!(matches!(
            cli.command,
            Cmd::Eq(EqCmd::Set { band: 2, gain: Some(g), .. }) if (g + 1.5).abs() < f32::EPSILON
        ));
    }

    #[test]
    fn rejects_bad_values_at_parse_time() {
        for args in [
            &["lumina", "sound-mode", "loud"][..],
            &["lumina", "eq", "set", "7", "--gain", "1"],
            &["lumina", "light", "static", "#12345"],
            &["lumina", "light", "leds"],
            &["lumina", "debug", "send", "+1"],
        ] {
            assert!(
                Cli::try_parse_from(args).is_err(),
                "{args:?} should not parse"
            );
        }
    }

    #[test]
    fn parses_hex_bytes() {
        assert_eq!(parse_byte("0x0d"), Ok(0x0D));
        assert_eq!(parse_byte("ff"), Ok(0xFF));
        assert!(parse_byte("+1").is_err());
        assert!(parse_byte("100").is_err());
        assert!(parse_byte("").is_err());
    }
}
