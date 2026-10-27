//! Round trips every setting on connected speakers and restores it.
//!
//! Needs the speakers on USB and not held by OpenRGB or a VM:
//! `cargo test -p lumina --test hardware -- --ignored --test-threads 1`
//! Input, Bluetooth pairing, standby and factory reset are left out.

use lumina::eq::Preset;
use lumina::{FirmwarePreset, Lumina, SoundMode};

#[test]
#[ignore = "needs hardware"]
fn settings_round_trip() {
    let mut l = Lumina::open(None).expect("open speakers");
    let before = l.state().expect("read state");

    let other = |m| {
        if m == Some(SoundMode::Movie) {
            SoundMode::Music
        } else {
            SoundMode::Movie
        }
    };
    l.set_sound_mode(other(before.sound_mode)).unwrap();
    let preset = if before.firmware_preset == Some(FirmwarePreset::Rock) {
        FirmwarePreset::Vocal
    } else {
        FirmwarePreset::Rock
    };
    l.set_firmware_preset(preset).unwrap();
    let sub = if before.sub_level == Some(-5) { -6 } else { -5 };
    l.set_sub_level(sub).unwrap();
    l.set_night_mode(!before.night_mode).unwrap();
    l.set_ui_tones(!before.ui_tones).unwrap();
    l.set_auto_standby(!before.auto_standby).unwrap();
    let timeout = if before.auto_standby_timeout == 600 {
        300
    } else {
        600
    };
    l.set_auto_standby_timeout(timeout).unwrap();
    l.set_eq_preset(Preset::Rock).unwrap();

    let changed = l.state().expect("read state");

    // Restore before asserting, so a failure leaves the speakers as found
    if let Some(m) = before.sound_mode {
        l.set_sound_mode(m).unwrap();
    }
    if let Some(p) = before.firmware_preset {
        l.set_firmware_preset(p).unwrap();
    }
    l.set_sub_level(before.sub_level.expect("known subwoofer level"))
        .unwrap();
    l.set_night_mode(before.night_mode).unwrap();
    l.set_ui_tones(before.ui_tones).unwrap();
    l.set_auto_standby(before.auto_standby).unwrap();
    l.set_auto_standby_timeout(before.auto_standby_timeout)
        .unwrap();
    l.set_bands(&before.bands).unwrap();
    let after = l.state().expect("read state");

    assert_eq!(changed.sound_mode, Some(other(before.sound_mode)));
    assert_eq!(changed.firmware_preset, Some(preset));
    assert_eq!(changed.sub_level, Some(sub));
    assert_eq!(changed.night_mode, !before.night_mode);
    assert_eq!(changed.ui_tones, !before.ui_tones);
    assert_eq!(changed.auto_standby, !before.auto_standby);
    assert_eq!(changed.auto_standby_timeout, timeout);
    assert_eq!(changed.eq_preset, Some(Preset::Rock));
    assert!(changed.custom_eq);

    // Everything but the play state, which follows the host stream
    let mut a = after.raw;
    let mut b = before.raw;
    a[9] = 0;
    b[9] = 0;
    assert_eq!(a, b, "state not restored");
}
