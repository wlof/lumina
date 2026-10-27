//! `lumina`: command line control for Klipsch ProMedia Lumina speakers.

mod cli;
mod commands;
mod render;

use std::process::{self, ExitCode};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::Parser;
use lumina::Lumina;

use crate::cli::{Cli, Cmd};

/// Switching to USB makes the speakers re-enumerate and boot again.
const RECONNECT_TIMEOUT: Duration = Duration::from_secs(45);

/// Exit status when the watchdog gives up.
const EXIT_NO_ANSWER: i32 = 2;

fn main() -> ExitCode {
    let cli = Cli::parse();

    let budget = match cli.command {
        Cmd::Input { input: Some(_) } => RECONNECT_TIMEOUT.max(Duration::from_secs(cli.timeout)),
        _ => Duration::from_secs(cli.timeout),
    };
    watchdog(budget);

    match run(cli) {
        Ok(out) => {
            print!("{out}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<String> {
    let serial = cli.serial.as_deref();
    let mut l = Lumina::open(serial)?;
    commands::run(&mut l, cli.command, &mut || reopen(serial))
}

/// hidapi calls can block for a long time when the speakers are in standby,
/// so give up from a separate thread.
fn watchdog(budget: Duration) {
    thread::spawn(move || {
        thread::sleep(budget);
        eprintln!(
            "Error: no answer from the speakers within {} s; they may be in standby \
             (press the power button) or disconnected",
            budget.as_secs()
        );
        process::exit(EXIT_NO_ANSWER);
    });
}

/// Open the speakers again after they dropped off USB, and wait until they
/// answer state requests.
fn reopen(serial: Option<&str>) -> Result<Lumina> {
    let start = Instant::now();
    // Give the old device time to disappear
    thread::sleep(Duration::from_secs(2));

    loop {
        match Lumina::open(serial).and_then(|mut l| l.state().map(|_| l)) {
            Ok(l) => return Ok(l),
            Err(_) if start.elapsed() < RECONNECT_TIMEOUT => thread::sleep(Duration::from_secs(1)),
            Err(e) => return Err(e).context("speakers did not come back"),
        }
    }
}
