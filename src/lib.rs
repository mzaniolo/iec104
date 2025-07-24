#![allow(dead_code, missing_docs, clippy::missing_docs_in_private_items)]

mod asdu;
mod cot;
mod error;
mod types;
mod types_id;

use error::Error;
use snafu::ResultExt as _;

use crate::{asdu::Asdu, error::InvalidAsdu};

const TELEGRAN_HEADER: u8 = 0x68;
const APUD_MAX_LENGTH: u8 = 253;

struct Apdu {
	length: u8,
	frame: Frame,
}

impl Apdu {
	pub fn parse(data: &[u8]) -> Result<Self, Box<Error>> {
		// Check if the data is long enough to contain the APDU header
		if data.len() < 6 {
			return error::ApduTooShort.fail()?;
		}
		if data[0] != TELEGRAN_HEADER {
			return error::InvalidTelegramHeader.fail()?;
		}

		let length = data[1];

		if length > APUD_MAX_LENGTH {
			return error::InvalidLength.fail()?;
		}

		let control_fields = &data[2..6];
		let frame = if length == 4_u8 {
			Frame::from_control_fields(control_fields)
		} else {
			Frame::from_asdu(control_fields, &data[6..])
		}?;

		#[allow(clippy::expect_used)]
		Ok(Self { length, frame })
	}
}

enum Frame {
	I(IFrame),
	S(SFrame),
	U(UFrame),
}

impl Frame {
	fn from_control_fields(control_fields: &[u8]) -> Result<Self, Box<Error>> {
		match control_fields[0] & 0b0000_0011 {
			0b0000_0011 => Ok(Frame::U(UFrame::from_control_fields(control_fields)?)),
			0b0000_0001 => Ok(Frame::S(SFrame::from_control_fields(control_fields)?)),
			_ => error::InvalidControlField.fail()?,
		}
	}

	fn from_asdu(control_fields: &[u8], asdu: &[u8]) -> Result<Self, Box<Error>> {
		Ok(Frame::I(IFrame::from_asdu(control_fields, asdu)?))
	}
}

/// I-Frame
///
/// Used for frames containing ASDUs
struct IFrame {
	send_sequence_number: u16,
	receive_sequence_number: u16,
	asdu: Asdu,
}

impl IFrame {
	fn from_asdu(control_fields: &[u8], asdu: &[u8]) -> Result<Self, Box<Error>> {
		if (control_fields[0] & 0b0000_0001) != 0 || (control_fields[2] & 0b0000_0001) != 0 {
			return error::InvalidIFrameControlFields.fail()?;
		}
		Ok(Self {
			send_sequence_number: u16::from_be_bytes([control_fields[1], control_fields[0] >> 1]),
			receive_sequence_number: u16::from_be_bytes([
				control_fields[3],
				control_fields[2] >> 1,
			]),
			asdu: Asdu::parse(asdu).context(InvalidAsdu)?,
		})
	}
}

/// S-Frame
///
/// Used to confirm I-frames
/// The station sends an S-frame to acknowledge receipt of I-frames whose SSN is
/// less than the RSN specified in the S-frame. This frame is sent by the
/// station if it does not have any data that it would like to send via I-frame
struct SFrame {
	receive_sequence_number: u16,
}

impl SFrame {
	fn from_control_fields(control_fields: &[u8]) -> Result<Self, Box<Error>> {
		if control_fields[0] != 0b0000_0001 || control_fields[1] != 0b0000_0000 {
			return error::InvalidSFrameControlFields.fail()?;
		}
		Ok(Self {
			receive_sequence_number: u16::from_be_bytes([
				control_fields[3],
				control_fields[2] >> 1,
			]),
		})
	}
}

/// U-Frame
///
/// used in 3 special cases:
/// - StartDT activation/confirmation - request for sending data via the
///   established connection and its confirmation (request is sent by the
///   client, confirmation by the server).
///
/// - StopDT activation/confirmation - request to stop sending data and its
///   confirmation (request is sent by the client, confirmation by the server).
///
/// - TestFR activation/confirmation - sending the test frame and responding to
///   it. Test frames can be sent by both parties to verify the functionality of
///   the TCP channel when no other frame has arrived for a long time. The
///   standard defines the idle time as t3 timeout with a default value of 20
///   seconds and it can be changed in the protocol parameters in the D2000
struct UFrame {
	start_dt_activation: bool,
	start_dt_confirmation: bool,

	stop_dt_activation: bool,
	stop_dt_confirmation: bool,

	test_fr_activation: bool,
	test_fr_confirmation: bool,
}

impl UFrame {
	fn from_control_fields(control_fields: &[u8]) -> Result<Self, Box<Error>> {
		if control_fields[1] != 0 || control_fields[2] != 0 || control_fields[3] != 0 {
			return error::InvalidUFrameControlFields.fail()?;
		}
		Ok(Self {
			start_dt_activation: control_fields[0] & 0b0000_0100 != 0,
			start_dt_confirmation: control_fields[0] & 0b0000_1000 != 0,

			stop_dt_activation: control_fields[0] & 0b0001_0000 != 0,
			stop_dt_confirmation: control_fields[0] & 0b0010_0000 != 0,

			test_fr_activation: control_fields[0] & 0b0100_0000 != 0,
			test_fr_confirmation: control_fields[0] & 0b1000_0000 != 0,
		})
	}
}
