//! What each command does. Every function returns the text to print, and
//! works with any transport so that it can be tested against the simulator.

use std::fmt::{Debug, Write as _};

use anyhow::{Context, Result, bail};
use klipsch_lumina::{BtPairing, InputSource, LightMode, Lumina, Transport, eq, light};

use crate::cli::{Cmd, DebugCmd, EqCmd, LightCmd, MuteArg, OnOff, PowerCmd};
use crate::render;

/// Opens the speakers again after they dropped off USB.
pub(crate) type Reconnect<'a, T> = dyn FnMut() -> Result<Lumina<T>> + 'a;

pub(crate) fn run<T: Transport>(
    l: &mut Lumina<T>,
    cmd: Cmd,
    reconnect: &mut Reconnect<'_, T>,
) -> Result<String> {
    match cmd {
        Cmd::Status { json } => status(l, json),
        Cmd::Eq(cmd) => run_eq(l, cmd),
        Cmd::SoundMode { mode } => {
            if let Some(m) = mode {
                l.set_sound_mode(m)?;
            }
            let s = l.state()?;
            check(mode, s.sound_mode, "sound mode")?;
            Ok(format!("Sound mode: {}\n", render::opt(s.sound_mode)))
        }
        Cmd::Sub { level } => {
            if let Some(db) = level {
                l.set_sub_level(db)?;
            }
            let s = l.state()?;
            check(level, s.sub_level, "subwoofer level")?;
            Ok(format!("Subwoofer: {}\n", render::sub_level(s.sub_level)))
        }
        Cmd::NightMode { state } => {
            let want = state.map(bool::from);
            if let Some(on) = want {
                l.set_night_mode(on)?;
            }
            let s = l.state()?;
            check(want, Some(s.night_mode), "night mode")?;
            Ok(format!("Night mode: {}\n", render::on_off(s.night_mode)))
        }
        Cmd::Mute { state } => mute(l, state),
        Cmd::Tones { state } => {
            let want = state.map(bool::from);
            if let Some(on) = want {
                l.set_ui_tones(on)?;
            }
            let s = l.state()?;
            check(want, Some(s.ui_tones), "UI tones")?;
            Ok(format!("UI tones: {}\n", render::on_off(s.ui_tones)))
        }
        Cmd::AutoStandby { state, after } => auto_standby(l, state, after),
        Cmd::Input { input } => set_input(l, input, reconnect),
        Cmd::BtPair { state } => bt_pair(l, state),
        Cmd::Light { cmd } => run_light(l, cmd.as_ref()),
        Cmd::Power(PowerCmd::Standby { yes }) => {
            if !yes {
                bail!(
                    "standby can only be left with the speakers' power button; \
                     pass --yes to go ahead"
                );
            }
            l.standby()?;
            Ok("Speakers in standby. Press their power button to wake them.\n".into())
        }
        Cmd::FactoryReset { yes } => {
            if !yes {
                bail!(
                    "this restores factory settings and forgets Bluetooth pairings; \
                     pass --yes to go ahead"
                );
            }
            l.factory_reset()?;
            Ok("Factory reset sent.\n".into())
        }
        Cmd::Debug(DebugCmd::Dump) => Ok(render::hexdump(&l.state().context("reading state")?)),
        Cmd::Debug(DebugCmd::Send { cmd, data, no_read }) => {
            l.send_raw(cmd, &data)
                .with_context(|| format!("sending command {cmd:#04x}"))?;
            if no_read {
                return Ok(String::new());
            }
            Ok(render::hexdump(&l.state().context("reading state")?))
        }
    }
}

fn status<T: Transport>(l: &mut Lumina<T>, json: bool) -> Result<String> {
    let s = l.state()?;
    Ok(if json {
        serde_json::to_string_pretty(&s)? + "\n"
    } else {
        render::status(&s)
    })
}

fn mute<T: Transport>(l: &mut Lumina<T>, state: Option<MuteArg>) -> Result<String> {
    let want = match state {
        None => None,
        Some(MuteArg::On) => Some(true),
        Some(MuteArg::Off) => Some(false),
        Some(MuteArg::Toggle) => Some(!l.state()?.mute),
    };
    if let Some(on) = want {
        l.set_mute(on)?;
    }
    let s = l.state()?;
    check(want, Some(s.mute), "mute")?;
    Ok(format!("Mute: {}\n", render::on_off(s.mute)))
}

fn auto_standby<T: Transport>(
    l: &mut Lumina<T>,
    state: Option<OnOff>,
    after: Option<u16>,
) -> Result<String> {
    let want = state.map(bool::from);
    let seconds = after
        .map(|min| min.checked_mul(60).context("auto standby delay too long"))
        .transpose()?;
    if let Some(on) = want {
        l.set_auto_standby(on)?;
    }
    if let Some(secs) = seconds {
        l.set_auto_standby_timeout(secs)?;
    }
    let s = l.state()?;
    check(want, Some(s.auto_standby), "auto standby")?;
    check(seconds, Some(s.auto_standby_timeout), "auto standby delay")?;
    Ok(format!("Auto standby: {}\n", render::auto_standby(&s)))
}

fn set_input<T: Transport>(
    l: &mut Lumina<T>,
    input: Option<InputSource>,
    reconnect: &mut Reconnect<'_, T>,
) -> Result<String> {
    if let Some(i) = input {
        let before = l.state()?.input;
        match l.set_input(i) {
            Ok(()) | Err(klipsch_lumina::Error::Reconnecting) => {}
            Err(e) => return Err(e.into()),
        }
        // Switching to USB from another input re-enumerates the device
        if i == InputSource::UsbPc && before != Some(InputSource::UsbPc) {
            eprintln!("Waiting for the speakers to reconnect...");
            *l = reconnect()?;
        }
    }
    let s = l.state()?;
    check(input, s.input, "input")?;
    Ok(format!("Input: {}\n", render::opt(s.input)))
}

fn bt_pair<T: Transport>(l: &mut Lumina<T>, state: Option<OnOff>) -> Result<String> {
    if let Some(on) = state {
        // The speakers ignore the command on any other input
        if bool::from(on) && l.state()?.input != Some(InputSource::Bluetooth) {
            bail!("pairing needs the Bluetooth input; run `lumina input bt` first");
        }
        l.set_bt_pairing(on.into())?;
    }
    let s = l.state()?;
    if matches!(state, Some(OnOff::On)) {
        check(Some(BtPairing::Pairing), s.bt_pairing, "Bluetooth pairing")?;
    }
    Ok(format!("Bluetooth: {}\n", render::bluetooth(&s)))
}

fn run_eq<T: Transport>(l: &mut Lumina<T>, cmd: EqCmd) -> Result<String> {
    match cmd {
        EqCmd::Preset { preset } => {
            if let Some(p) = preset {
                l.set_eq_preset(p)?;
            }
            let s = l.state()?;
            check(preset, s.eq_preset, "EQ preset")?;
            let mut out = format!("EQ: {}\n", render::eq_name(&s));
            if preset.is_none() {
                // Writing to a String cannot fail
                let _ = writeln!(out, "Presets: {}", render::preset_names());
            }
            Ok(out)
        }
        EqCmd::Bands { json } => {
            let s = l.state()?;
            Ok(if json {
                serde_json::to_string_pretty(&s.bands)? + "\n"
            } else {
                format!("EQ: {}\n{}", render::eq_name(&s), render::bands(&s))
            })
        }
        EqCmd::Set {
            band,
            freq,
            gain,
            q,
        } => {
            if freq.is_none() && gain.is_none() && q.is_none() {
                bail!("give at least one of --freq, --gain or --q");
            }
            let mut bands = l.state()?.bands;
            // clap limits band to 1..=6
            let b = &mut bands[usize::from(band - 1)];
            b.freq = freq.unwrap_or(b.freq);
            b.gain = gain.unwrap_or(b.gain);
            b.q = q.unwrap_or(b.q);
            l.set_bands(&bands)?;
            let s = l.state()?;
            // Compare at the wire's resolution: Q in 1/1024, gain in 1/256 dB
            let want = eq::decode(eq::encode(&bands)?);
            check(Some(want), Some(s.bands), "bands")?;
            Ok(render::bands(&s))
        }
        EqCmd::Reset => {
            l.set_bands(&eq::DEVICE_DEFAULT)?;
            let s = l.state()?;
            check(Some(eq::DEVICE_DEFAULT), Some(s.bands), "bands")?;
            Ok(format!("EQ: {}\n", render::eq_name(&s)))
        }
        EqCmd::Firmware { preset } => {
            if let Some(p) = preset {
                l.set_firmware_preset(p)?;
            }
            let s = l.state()?;
            check(preset, s.firmware_preset, "firmware preset")?;
            Ok(format!(
                "Firmware preset: {}\n",
                render::opt(s.firmware_preset)
            ))
        }
    }
}

fn run_light<T: Transport>(l: &mut Lumina<T>, cmd: Option<&LightCmd>) -> Result<String> {
    let (mode, colors) = match cmd {
        None => (None, None),
        Some(&LightCmd::Static { color }) => {
            l.light_static(color)?;
            (Some(LightMode::Static), Some((color, color)))
        }
        Some(&LightCmd::Breathe { color }) => {
            l.light_breathe(color)?;
            (Some(LightMode::Breathe), Some((color, color)))
        }
        Some(&LightCmd::Aurora { tone }) => {
            l.light_aurora(tone)?;
            (Some(LightMode::Aurora), Some(tone.colors()))
        }
        Some(&LightCmd::MusicReact { preset }) => {
            l.light_music_react(preset)?;
            (Some(LightMode::MusicReact), Some(preset.colors()))
        }
        Some(LightCmd::Rainbow) => {
            l.set_light_mode(LightMode::Rainbow)?;
            (Some(LightMode::Rainbow), None)
        }
        Some(LightCmd::Off) => {
            l.set_light_mode(LightMode::Off)?;
            (Some(LightMode::Off), None)
        }
        Some(LightCmd::Leds { colors }) => {
            l.set_leds(&light::expand(colors)?)?;
            (Some(LightMode::Direct), None)
        }
    };

    let s = l.state()?;
    check(mode, s.light_mode, "light mode")?;
    check(colors, Some(s.light_colors), "light colours")?;
    Ok(format!("Light: {}\n", render::light(&s)))
}

/// Fail if the speakers report something other than what was asked.
fn check<T: Copy + PartialEq + Debug>(want: Option<T>, got: Option<T>, what: &str) -> Result<()> {
    match want {
        Some(w) if got.as_ref() != Some(&w) => {
            bail!("the speakers report {what} {got:?} instead of {w:?}")
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use klipsch_lumina::sim::Simulator;

    use super::*;
    use crate::cli::Cli;

    /// Run a command line against fresh simulated speakers.
    fn run_on(sim: Simulator, args: &[&str]) -> (Result<String>, Lumina<Simulator>) {
        let mut l = Lumina::with_transport(sim);
        let cli = Cli::try_parse_from(std::iter::once("lumina").chain(args.iter().copied()))
            .expect("valid command line");
        // Reconnected speakers come back on USB; a fresh simulator starts there
        let mut reconnect = || Ok(Lumina::with_transport(Simulator::new()));
        let out = run(&mut l, cli.command, &mut reconnect);
        (out, l)
    }

    fn run_ok(args: &[&str]) -> (String, Lumina<Simulator>) {
        let (out, l) = run_on(Simulator::new(), args);
        (out.expect("command succeeds"), l)
    }

    #[test]
    fn getters_print_current_values() {
        assert_eq!(run_ok(&["sound-mode"]).0, "Sound mode: music\n");
        assert_eq!(run_ok(&["sub"]).0, "Subwoofer: +0 dB\n");
        assert_eq!(run_ok(&["light"]).0, "Light: static #ff00ff\n");
        assert!(run_ok(&["status"]).0.starts_with("ProMedia Lumina"));
    }

    #[test]
    fn setters_apply_and_confirm() {
        let (out, mut l) = run_ok(&["sub", "-4"]);
        assert_eq!(out, "Subwoofer: -4 dB\n");
        assert_eq!(l.state().unwrap().sub_level, Some(-4));

        let (out, _) = run_ok(&["eq", "preset", "hip-hop"]);
        assert_eq!(out, "EQ: hip-hop (firmware preset flat)\n");

        let (out, _) = run_ok(&["light", "aurora", "warm"]);
        assert_eq!(out, "Light: aurora warm\n");
    }

    #[test]
    fn mute_toggles() {
        let (out, l) = run_ok(&["mute", "toggle"]);
        assert_eq!(out, "Mute: on\n");
        let (out, _) = run_on(l.transport().clone(), &["mute", "toggle"]);
        assert_eq!(out.unwrap(), "Mute: off\n");
    }

    #[test]
    fn eq_set_changes_one_band() {
        let (_, mut l) = run_ok(&["eq", "set", "4", "--gain", "-2.5", "--q", "1.4"]);
        let bands = l.state().unwrap().bands;
        assert!((bands[3].gain + 2.5).abs() < 0.01);
        assert!((bands[3].q - 1.4).abs() < 0.01);
        assert_eq!(bands[0], eq::DEVICE_DEFAULT[0]);
    }

    #[test]
    fn bt_pair_needs_bluetooth_input() {
        let (out, _) = run_on(Simulator::new(), &["bt-pair", "on"]);
        assert!(out.unwrap_err().to_string().contains("lumina input bt"));

        let (_, l) = run_ok(&["input", "bt"]);
        let (out, _) = run_on(l.transport().clone(), &["bt-pair", "on"]);
        assert!(out.unwrap().starts_with("Bluetooth: pairing"));
    }

    #[test]
    fn switching_back_to_usb_reconnects() {
        let (_, l) = run_ok(&["input", "aux"]);
        let (out, l) = run_on(l.transport().clone(), &["input", "usb"]);
        assert_eq!(out.unwrap(), "Input: usb\n");
        // The handle was replaced by the reconnected one
        assert_eq!(l.transport().received, Vec::<(u8, Vec<u8>)>::new());
    }

    #[test]
    fn standby_and_factory_reset_need_confirmation() {
        let (out, l) = run_on(Simulator::new(), &["power", "standby"]);
        assert!(out.is_err());
        assert!(!l.transport().in_standby());

        let (_, l) = run_ok(&["power", "standby", "--yes"]);
        assert!(l.transport().in_standby());

        let (out, l) = run_on(Simulator::new(), &["factory-reset"]);
        assert!(out.is_err());
        assert_eq!(l.transport().received.len(), 0);
    }

    #[test]
    fn per_led_colours_reach_the_leds() {
        let (out, l) = run_ok(&["light", "leds", "red", "blue"]);
        assert_eq!(out, "Light: per-LED (direct)\n");
        let leds = l.transport().leds().expect("frame shown");
        // Left satellite red, right satellite blue
        assert_eq!(&leds[..3], &[255, 0, 0]);
        assert_eq!(&leds[33..], &[0, 0, 255]);
    }

    #[test]
    fn on_off_settings() {
        assert_eq!(run_ok(&["night-mode", "on"]).0, "Night mode: on\n");
        assert_eq!(run_ok(&["tones", "off"]).0, "UI tones: off\n");
        assert_eq!(run_ok(&["mute", "on"]).0, "Mute: on\n");
        assert_eq!(
            run_ok(&["auto-standby", "off", "--after", "10"]).0,
            "Auto standby: off (after 10 min)\n"
        );
        assert!(
            run_on(Simulator::new(), &["auto-standby", "--after", "0"])
                .0
                .is_err()
        );
    }

    #[test]
    fn eq_commands() {
        assert_eq!(
            run_ok(&["sound-mode", "surround"]).0,
            "Sound mode: surround\n"
        );
        assert_eq!(
            run_ok(&["eq", "firmware", "rock"]).0,
            "Firmware preset: rock\n"
        );
        assert!(run_ok(&["eq", "preset"]).0.contains("Presets: flat, vocal"));
        assert_eq!(run_ok(&["eq", "bands"]).0.lines().count(), 7);
        assert!(
            run_ok(&["eq", "bands", "--json"])
                .0
                .contains("\"freq\": 8000")
        );
        assert!(run_on(Simulator::new(), &["eq", "set", "1"]).0.is_err());
        assert!(
            run_on(Simulator::new(), &["eq", "set", "1", "--gain", "9"])
                .0
                .is_err()
        );

        let (_, l) = run_ok(&["eq", "preset", "rock"]);
        let (out, _) = run_on(l.transport().clone(), &["eq", "reset"]);
        assert_eq!(out.unwrap(), "EQ: device default (firmware preset flat)\n");
    }

    #[test]
    fn light_modes() {
        assert_eq!(
            run_ok(&["light", "static", "orange"]).0,
            "Light: static #ff7f00\n"
        );
        assert_eq!(
            run_ok(&["light", "breathe", "blue"]).0,
            "Light: breathe #0000ff\n"
        );
        assert_eq!(
            run_ok(&["light", "music-react", "red-pink"]).0,
            "Light: music-react red-pink\n"
        );
        assert_eq!(run_ok(&["light", "rainbow"]).0, "Light: rainbow\n");
        assert_eq!(run_ok(&["light", "off"]).0, "Light: off\n");
        assert!(
            run_on(Simulator::new(), &["light", "leds", "red", "green", "blue"])
                .0
                .is_err()
        );
    }

    #[test]
    fn status_json_and_debug() {
        let json = run_ok(&["status", "--json"]).0;
        assert!(json.contains("\"firmware\": \"1.0.3\""));
        assert!(json.contains("\"input\": \"usb\""));
        assert!(run_ok(&["debug", "dump"]).0.starts_with("  0: a5 5a 70 7b"));
        let (out, l) = run_ok(&["debug", "send", "0x0d", "01", "--no-read"]);
        assert_eq!(out, "");
        assert_eq!(l.transport().received, vec![(0x0D, vec![1])]);
    }

    #[test]
    fn factory_reset_with_confirmation() {
        let (_, l) = run_ok(&["sub", "5"]);
        let (out, mut l) = run_on(l.transport().clone(), &["factory-reset", "--yes"]);
        assert_eq!(out.unwrap(), "Factory reset sent.\n");
        assert_eq!(l.state().unwrap().sub_level, Some(0));
    }

    #[test]
    fn standby_makes_commands_fail() {
        let (_, l) = run_ok(&["power", "standby", "--yes"]);
        let (out, _) = run_on(l.transport().clone(), &["status"]);
        assert!(out.unwrap_err().to_string().contains("standby"));
    }

    #[test]
    fn reports_disagreement() {
        assert!(check(Some(1), Some(2), "thing").is_err());
        assert!(check(Some(1), None, "thing").is_err());
        assert!(check(None, Some(2), "thing").is_ok());
    }
}
