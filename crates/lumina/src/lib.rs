#![doc = include_str!("../README.md")]
//!
//! # Overview
//!
//! [`Lumina`] is the entry point: open the speakers, read their [`State`]
//! and change settings with its typed setters. Every setter validates its
//! input before anything is sent.
//!
//! The speakers expose Klipsch's command set on vendor HID interface 4, as
//! 256-byte reports carried by HID class control transfers. The [`frame`]
//! module builds those reports; [`eq`] and [`light`] hold the EQ and
//! lighting data models. `docs/protocol.md` in the repository records what
//! is known about the protocol and how it was verified.
//!
//! # Platforms
//!
//! Access goes through hidapi: its libusb back end on Linux and FreeBSD
//! (interface 4 has no hidraw node there), `hid.dll` on Windows and IOKit on
//! macOS. On Linux and FreeBSD only one process can hold the interface at a
//! time, see [`Error::Busy`].
#![warn(clippy::print_stdout, clippy::print_stderr)]

mod color;
mod command;
mod device;
pub mod eq;
pub mod frame;
pub mod light;
#[cfg(any(test, feature = "sim"))]
pub mod sim;
mod state;
mod transport;
mod types;

pub use color::Rgb;
pub use command::Command;
pub use device::Lumina;
pub use eq::{Band, Bands};
pub use state::State;
pub use transport::{HidTransport, PID, Transport, VID};
pub use types::{
    BtPairing, FirmwarePreset, InputSource, LightMode, STANDBY_TIMEOUT_RANGE, SUB_LEVEL_RANGE,
    SoundMode,
};

/// Error source from the platform's HID layer, kept opaque so that the HID
/// library does not leak into this crate's API.
pub type HidError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Everything that can go wrong talking to the speakers.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
// Variants wrapping a HID error return it from `source()` and leave it out of
// their message, so error reports that print the chain show it only once.
pub enum Error {
    /// No Lumina is connected, or none with the requested serial number.
    #[error("no Klipsch ProMedia Lumina found")]
    NotFound,

    /// The control interface could not be opened. On Linux and FreeBSD this
    /// usually means OpenRGB or a virtual machine holds it.
    #[error("cannot open the speakers' control interface (is OpenRGB or a VM using it?)")]
    Busy(#[source] HidError),

    /// A transfer failed: the speakers are in standby, which only their
    /// power button ends, or were disconnected.
    #[error(
        "speakers did not respond; they may be in standby (press the power button) or disconnected"
    )]
    NoResponse(#[source] HidError),

    /// The speakers dropped off USB after switching to USB input, as they
    /// always do. Open them again after a few seconds.
    #[error("speakers are reconnecting after switching to USB input")]
    Reconnecting,

    /// The HID layer could not be initialised.
    #[error("cannot initialise HID access")]
    Hid(#[source] HidError),

    /// A value is outside what the speakers (or Klipsch Control) accept.
    #[error("{0} out of range: {1}")]
    OutOfRange(&'static str, String),

    /// A name that matches no known value.
    #[error("unknown {0}: {1}")]
    Unknown(&'static str, String),

    /// A command payload larger than one report can carry.
    #[error("payload of {0} bytes does not fit in one frame")]
    PayloadTooLong(usize),

    /// A state report of an unexpected size.
    #[error("unexpected state report length: {0} bytes")]
    BadLength(usize),

    /// The device answered with something other than a state report; it
    /// does so while booting.
    #[error("device did not return a state report")]
    NoState,
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;

    use super::*;

    #[test]
    fn hid_errors_are_reported_once() {
        for err in [
            Error::Busy("device busy".into()),
            Error::NoResponse("device busy".into()),
            Error::Hid("device busy".into()),
        ] {
            assert!(!err.to_string().contains("device busy"), "{err}");
            assert_eq!(
                err.source().map(ToString::to_string).as_deref(),
                Some("device busy")
            );
        }
    }
}
