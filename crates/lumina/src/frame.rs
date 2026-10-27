//! Command frames exchanged on the vendor HID interface.
//!
//! Output reports are 256 bytes with no report ID. A frame starts at byte 0:
//!
//! ```text
//! A5 5A 70 <cmd> <len> 00 <data...> <checksum> <zero padding>
//! ```
//!
//! `len` is the total frame length (`data.len() + 7`) and the checksum makes
//! the sum of all frame bytes zero modulo 256.

use crate::Error;

/// Size of every output report, and the largest input report.
pub const REPORT_LEN: usize = 256;
/// First three bytes of every frame and state report.
pub const HEADER: [u8; 3] = [0xA5, 0x5A, 0x70];

/// Header, command, length and reserved byte before the data, checksum after.
const OVERHEAD: usize = 7;

/// Largest data payload that fits in one report.
pub const MAX_DATA_LEN: usize = u8::MAX as usize - OVERHEAD;

/// One whole report.
pub type Report = [u8; REPORT_LEN];

/// Build an output report carrying `cmd` and `data`.
pub fn encode(cmd: u8, data: &[u8]) -> Result<Report, Error> {
    if data.len() > MAX_DATA_LEN {
        return Err(Error::PayloadTooLong(data.len()));
    }

    let frame_len = data.len() + OVERHEAD;
    let len_byte = u8::try_from(frame_len).map_err(|_| Error::PayloadTooLong(data.len()))?;
    let mut report = [0u8; REPORT_LEN];

    report[..3].copy_from_slice(&HEADER);
    report[3] = cmd;
    report[4] = len_byte;
    report[6..6 + data.len()].copy_from_slice(data);
    report[frame_len - 1] = checksum(&report[..frame_len - 1]);

    Ok(report)
}

/// Bare header with no command: makes the device refresh its state report.
#[must_use]
pub fn state_request() -> Report {
    let mut report = [0u8; REPORT_LEN];
    report[..3].copy_from_slice(&HEADER);
    report
}

/// Whether `report` starts with [`HEADER`].
#[must_use]
pub fn has_header(report: &[u8]) -> bool {
    report.starts_with(&HEADER)
}

fn checksum(bytes: &[u8]) -> u8 {
    bytes
        .iter()
        .fold(0u8, |sum, b| sum.wrapping_add(*b))
        .wrapping_neg()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn frame_sum(report: &Report) -> u8 {
        let len = usize::from(report[4]);
        report[..len].iter().fold(0u8, |s, b| s.wrapping_add(*b))
    }

    #[test]
    fn light_mode_frame() {
        // Light mode Static
        let report = encode(0x0B, &[0x03]).unwrap();
        assert_eq!(
            &report[..8],
            &[0xA5, 0x5A, 0x70, 0x0B, 0x08, 0x00, 0x03, 0x7B]
        );
        assert!(report[8..].iter().all(|b| *b == 0));
        assert_eq!(frame_sum(&report), 0);
    }

    #[test]
    fn checksum_wraps() {
        let report = encode(0x11, &[0xFF; 36]).unwrap();
        assert_eq!(report[4], 43);
        assert_eq!(frame_sum(&report), 0);
    }

    #[test]
    fn empty_payload() {
        let report = encode(0x0C, &[]).unwrap();
        assert_eq!(report[4], 7);
        assert_eq!(frame_sum(&report), 0);
    }

    #[test]
    fn rejects_oversized_payload() {
        assert!(encode(0x0A, &[0; MAX_DATA_LEN]).is_ok());
        assert!(matches!(
            encode(0x0A, &[0; MAX_DATA_LEN + 1]),
            Err(Error::PayloadTooLong(_))
        ));
    }

    proptest! {
        /// Every frame the device accepts sums to zero and carries its payload.
        #[test]
        fn frames_are_well_formed(cmd: u8, data in proptest::collection::vec(any::<u8>(), 0..=MAX_DATA_LEN)) {
            let report = encode(cmd, &data).unwrap();
            let len = usize::from(report[4]);
            prop_assert_eq!(len, data.len() + OVERHEAD);
            prop_assert_eq!(frame_sum(&report), 0);
            prop_assert_eq!(&report[6..6 + data.len()], &data[..]);
            prop_assert!(report[len..].iter().all(|b| *b == 0));
        }
    }
}
