//! Human-readable output. Everything here returns strings so that it can be
//! tested without speakers.

use std::fmt::{Display, Write as _};

use lumina::eq::{self, Preset};
use lumina::light::{AuroraTone, MusicReactPreset};
use lumina::{LightMode, State};

/// The full `lumina status` view.
pub(crate) fn status(s: &State) -> String {
    let mut out = format!(
        "{}  (serial {}, firmware {})\n",
        s.name, s.serial, s.firmware
    );
    field(&mut out, "Input", opt(s.input));
    field(&mut out, "Sound mode", opt(s.sound_mode));
    field(&mut out, "EQ", eq_name(s));
    out.push_str(&bands(s));
    field(&mut out, "Subwoofer", sub_level(s.sub_level));
    field(&mut out, "Night mode", on_off(s.night_mode));
    field(&mut out, "Mute", on_off(s.mute));
    field(&mut out, "UI tones", on_off(s.ui_tones));
    field(&mut out, "Auto standby", auto_standby(s));
    field(&mut out, "Bluetooth", bluetooth(s));
    field(&mut out, "Light", light(s));
    out
}

/// `label:` padded to a column, then the value.
fn field(out: &mut String, label: &str, value: impl Display) {
    // Writing to a String cannot fail
    let _ = writeln!(out, "{:<16}{value}", format!("{label}:"));
}

/// One line per band.
pub(crate) fn bands(s: &State) -> String {
    let mut out = String::new();
    for (i, b) in s.bands.iter().enumerate() {
        // Writing to a String cannot fail
        let _ = writeln!(
            out,
            "  band {}:       {:>5} Hz  Q {:<5.3} {:+.1} dB",
            i + 1,
            b.freq,
            b.q,
            b.gain
        );
    }
    out
}

/// The preset the bands match, plus the firmware preset.
pub(crate) fn eq_name(s: &State) -> String {
    let bands = match s.eq_preset {
        Some(p) => p.name(),
        None if s.bands == eq::DEVICE_DEFAULT => "device default",
        None => "custom",
    };
    format!("{bands} (firmware preset {})", opt(s.firmware_preset))
}

/// The names `eq preset` accepts.
pub(crate) fn preset_names() -> String {
    Preset::ALL
        .iter()
        .map(|p| p.name())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Light mode with its colours or preset.
pub(crate) fn light(s: &State) -> String {
    let (p, sec) = s.light_colors;
    match s.light_mode {
        Some(LightMode::Static) => format!("static {p}"),
        Some(LightMode::Breathe) => format!("breathe {p}"),
        Some(LightMode::Aurora) => match AuroraTone::matching(p, sec) {
            Some(t) => format!("aurora {t}"),
            None => format!("aurora ({p} {sec})"),
        },
        Some(LightMode::MusicReact) => match MusicReactPreset::matching(p, sec) {
            Some(m) => format!("music-react {m}"),
            None => format!("music-react ({p} {sec}, not a preset)"),
        },
        Some(LightMode::Direct) => "per-LED (direct)".into(),
        Some(m) => m.to_string(),
        None => "unknown".into(),
    }
}

pub(crate) fn auto_standby(s: &State) -> String {
    format!(
        "{} (after {})",
        on_off(s.auto_standby),
        minutes(s.auto_standby_timeout)
    )
}

pub(crate) fn bluetooth(s: &State) -> String {
    format!("{} ({})", opt(s.bt_pairing), s.bt_mac)
}

pub(crate) fn sub_level(db: Option<i8>) -> String {
    db.map_or_else(|| "unknown".into(), |db| format!("{db:+} dB"))
}

fn minutes(seconds: u16) -> String {
    if seconds.is_multiple_of(60) {
        format!("{} min", seconds / 60)
    } else {
        format!("{} min {} s", seconds / 60, seconds % 60)
    }
}

pub(crate) fn opt<T: Display>(v: Option<T>) -> String {
    v.map_or_else(|| "unknown".into(), |v| v.to_string())
}

pub(crate) fn on_off(b: bool) -> &'static str {
    if b { "on" } else { "off" }
}

/// The raw report, 16 bytes per line with decimal offsets.
pub(crate) fn hexdump(s: &State) -> String {
    let mut out = String::new();
    for (row, chunk) in s.raw.chunks(16).enumerate() {
        let bytes: Vec<String> = chunk.iter().map(|b| format!("{b:02x}")).collect();
        // Writing to a String cannot fail
        let _ = writeln!(out, "{:3}: {}", row * 16, bytes.join(" "));
    }
    out
}

#[cfg(test)]
mod tests {
    use lumina::Rgb;

    use super::*;

    /// The state report captured from the hardware.
    fn sample() -> State {
        let bytes: Vec<u8> = include_str!("../../lumina/tests/data/state-1.0.3.hex")
            .split_whitespace()
            .map(|b| u8::from_str_radix(b, 16).unwrap())
            .collect();
        State::parse(&bytes).unwrap()
    }

    #[test]
    fn renders_status() {
        assert_eq!(
            status(&sample()),
            "\
ProMedia Lumina  (serial 123456789012345, firmware 1.0.3)
Input:          usb
Sound mode:     music
EQ:             device default (firmware preset flat)
  band 1:          50 Hz  Q 0.707 +0.0 dB
  band 2:         150 Hz  Q 0.707 +0.0 dB
  band 3:         400 Hz  Q 0.707 +0.0 dB
  band 4:        1000 Hz  Q 0.707 +0.0 dB
  band 5:        3500 Hz  Q 0.707 +0.0 dB
  band 6:        8000 Hz  Q 0.707 +0.0 dB
Subwoofer:      +0 dB
Night mode:     off
Mute:           off
UI tones:       on
Auto standby:   on (after 15 min)
Bluetooth:      off (02:00:00:00:00:01)
Light:          static #ff00ff
"
        );
    }

    #[test]
    fn names_light_presets() {
        let mut s = sample();
        s.light_mode = Some(LightMode::Aurora);
        s.light_colors = AuroraTone::Warm.colors();
        assert_eq!(light(&s), "aurora warm");
        s.light_mode = Some(LightMode::MusicReact);
        s.light_colors = (Rgb(1, 2, 3), Rgb(4, 5, 6));
        assert_eq!(light(&s), "music-react (#010203 #040506, not a preset)");
        s.light_mode = None;
        assert_eq!(light(&s), "unknown");
    }

    #[test]
    fn names_eq_presets() {
        let mut s = sample();
        s.bands = Preset::Rock.bands();
        s.eq_preset = Some(Preset::Rock);
        assert_eq!(eq_name(&s), "rock (firmware preset flat)");
        s.bands[0].gain = 1.0;
        s.eq_preset = None;
        assert_eq!(eq_name(&s), "custom (firmware preset flat)");
    }

    #[test]
    fn formats_values() {
        assert_eq!(sub_level(Some(-4)), "-4 dB");
        assert_eq!(sub_level(None), "unknown");
        assert_eq!(minutes(90), "1 min 30 s");
        assert_eq!(preset_names(), "flat, vocal, bass, treble, rock, hip-hop");
    }

    #[test]
    fn dumps_full_report() {
        let dump = hexdump(&sample());
        assert_eq!(dump.lines().count(), 16);
        assert!(dump.starts_with("  0: a5 5a 70 7b 00 02"));
    }
}
