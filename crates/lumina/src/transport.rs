//! Report transport: hidapi on real hardware, a mock in tests.

use std::ffi::CString;
use std::fmt;

use hidapi::{HidApi, HidDevice};

use crate::Error;
use crate::frame::{REPORT_LEN, Report};

/// USB vendor ID of Klipsch Group, Inc.
pub const VID: u16 = 0x18B5;
/// USB product ID of the ProMedia Lumina.
pub const PID: u16 = 0x0070;

const VENDOR_INTERFACE: i32 = 4;
const LAMPARRAY_INTERFACE: i32 = 5;
/// `LampArrayControl` feature report: report ID, then `AutonomousMode`.
const LAMPARRAY_CONTROL: [u8; 2] = [6, 1];

/// Reports have no report ID; hidapi buffers start with a 0 byte for it.
const BUFFER_LEN: usize = REPORT_LEN + 1;

/// Moves whole reports to and from the vendor interface.
///
/// [`HidTransport`] is the real implementation; tests substitute a mock.
pub trait Transport {
    /// Send one output report.
    fn write(&mut self, report: &Report) -> Result<(), Error>;

    /// Read the current input report; may be shorter than [`REPORT_LEN`].
    fn read(&mut self) -> Result<Vec<u8>, Error>;

    /// Make the firmware render per-LED frames. Until the `LampArrayControl`
    /// report has been written on interface 5 it shows white instead.
    fn unlock_leds(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

/// Interface 4 through hidapi.
///
/// Interface 4 has no interrupt endpoint. Windows and macOS still expose it
/// as a HID device, Linux and FreeBSD do not (no hidraw node), so hidapi
/// uses its libusb back end there. Either way reports travel as HID class
/// control transfers: `SET_REPORT(Output)` and `GET_REPORT(Input)`.
pub struct HidTransport {
    api: HidApi,
    device: HidDevice,
    lamparray: Option<CString>,
}

impl fmt::Debug for HidTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HidTransport")
            .field("lamparray", &self.lamparray)
            .finish_non_exhaustive()
    }
}

impl HidTransport {
    /// Open the first connected Lumina, or the one with the given USB serial.
    pub fn open(serial: Option<&str>) -> Result<Self, Error> {
        // Not HidApi::disable_device_discovery(): on libusb back ends it stops
        // libusb from finding any device at all
        let api = HidApi::new().map_err(|e| Error::Hid(e.into()))?;

        // Leave the device usable by OpenRGB and Klipsch's app at the same time
        #[cfg(target_os = "macos")]
        api.set_open_exclusive(false);

        let lumina = || {
            api.device_list()
                .filter(|d| d.vendor_id() == VID && d.product_id() == PID)
                .filter(move |d| serial.is_none() || d.serial_number() == serial)
        };
        let info = lumina()
            .find(|d| d.interface_number() == VENDOR_INTERFACE)
            .ok_or(Error::NotFound)?;
        let lamparray = lumina()
            .find(|d| d.interface_number() == LAMPARRAY_INTERFACE)
            .map(|d| d.path().to_owned());

        let device = info.open_device(&api).map_err(|e| Error::Busy(e.into()))?;

        Ok(HidTransport {
            api,
            device,
            lamparray,
        })
    }
}

impl Transport for HidTransport {
    fn write(&mut self, report: &Report) -> Result<(), Error> {
        let mut buf = [0u8; BUFFER_LEN];
        buf[1..].copy_from_slice(report);

        self.device
            .write(&buf)
            .map_err(|e| Error::NoResponse(e.into()))?;
        Ok(())
    }

    fn read(&mut self) -> Result<Vec<u8>, Error> {
        let mut buf = [0u8; BUFFER_LEN];
        let len = self
            .device
            .get_input_report(&mut buf)
            .map_err(|e| Error::NoResponse(e.into()))?;

        // The count includes the report ID byte
        Ok(buf[1..len.clamp(1, BUFFER_LEN)].to_vec())
    }

    /// Windows' Dynamic Lighting provider (DWM) holds interface 5 and writes
    /// the report itself, so this only matters with the libusb back end.
    fn unlock_leds(&mut self) -> Result<(), Error> {
        if cfg!(target_os = "windows") {
            return Ok(());
        }
        let path = self.lamparray.as_ref().ok_or(Error::NotFound)?;
        let lamparray = self
            .api
            .open_path(path)
            .map_err(|e| Error::Busy(e.into()))?;
        lamparray
            .send_feature_report(&LAMPARRAY_CONTROL)
            .map_err(|e| Error::NoResponse(e.into()))
    }
}
