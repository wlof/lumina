# lumina

Rust library for controlling Klipsch ProMedia Lumina 2.1 speakers over USB:
EQ, sound mode, subwoofer level, night mode, input, lighting and the other
settings of Klipsch Control. The `lumina` command line tool is in the
`lumina-cli` crate.

```no_run
use lumina::{Lumina, SoundMode};

let mut speakers = Lumina::open(None)?;
speakers.set_sound_mode(SoundMode::Music)?;
if let Some(db) = speakers.state()?.sub_level {
    println!("subwoofer at {db} dB");
}
# Ok::<(), lumina::Error>(())
```

See the [repository README](https://github.com/wlof/lumina#readme) for
platform support and the
[protocol notes](https://github.com/wlof/lumina/blob/main/docs/protocol.md).
