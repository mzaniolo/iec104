use snafu::ResultExt as _;
use tracing::instrument;

use crate::{
	asdu::Asdu,
	error::{self, Error, InvalidAsdu},
};

pub(crate) const TELEGRAN_HEADER: u8 = 0x68;
pub(crate) const APUD_MAX_LENGTH: u8 = 253;

#[derive(Debug, Clone, PartialEq)]
pub struct Apdu {
	length: u8,
	frame: Frame,
}

impl Apdu {
	#[instrument]
	pub fn from_bytes(data: &[u8]) -> Result<Self, Box<Error>> {
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

	pub fn to_bytes(&self) -> Result<Vec<u8>, Box<Error>> {
		// The total length of the APDU is the length of the frame plus 2 bytes, one for
		// the header and one for the length
		let mut bytes = Vec::with_capacity(self.length as usize + 2);
		bytes.push(TELEGRAN_HEADER);
		bytes.push(self.length);
		self.frame.to_bytes(&mut bytes)?;
		Ok(bytes)
	}
}

#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
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

	pub fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<Error>> {
		match self {
			Frame::I(i) => i.to_bytes(buffer),
			Frame::S(s) => s.to_bytes(buffer),
			Frame::U(u) => u.to_bytes(buffer),
		}
	}
	pub fn to_apdu_bytes(&self) -> Result<Vec<u8>, Box<Error>> {
		let mut buffer = Vec::new();
		buffer.push(TELEGRAN_HEADER);
		buffer.push(0); // length placeholder
		self.to_bytes(&mut buffer)?;
		buffer[1] = (buffer.len() - 2) as u8; // update length
		Ok(buffer)
	}
}

/// I-Frame
///
/// Used for frames containing ASDUs
#[derive(Debug, Clone, PartialEq)]
pub struct IFrame {
	pub send_sequence_number: u16,
	pub receive_sequence_number: u16,
	pub asdu: Asdu,
}

impl IFrame {
	#[instrument]
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

	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<Error>> {
		let rsn = self.receive_sequence_number.to_le_bytes();
		let ssn = self.send_sequence_number.to_le_bytes();
		buffer.push(ssn[0] << 1);
		buffer.push(ssn[1]);
		buffer.push(rsn[0] << 1);
		buffer.push(rsn[1]);
		self.asdu.to_bytes(buffer).context(InvalidAsdu)?;
		Ok(())
	}
}

/// S-Frame
///
/// Used to confirm I-frames
/// The station sends an S-frame to acknowledge receipt of I-frames whose SSN is
/// less than the RSN specified in the S-frame. This frame is sent by the
/// station if it does not have any data that it would like to send via I-frame
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SFrame {
	pub receive_sequence_number: u16,
}

impl SFrame {
	#[instrument]
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

	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<Error>> {
		let rsn = self.receive_sequence_number.to_le_bytes();
		buffer.push(0b0000_0001);
		buffer.push(0b0000_0000);
		buffer.push(rsn[0] << 1);
		buffer.push(rsn[1]);
		Ok(())
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
///   the TCP channel when no other frame has arrived for a long time.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UFrame {
	pub start_dt_activation: bool,
	pub start_dt_confirmation: bool,

	pub stop_dt_activation: bool,
	pub stop_dt_confirmation: bool,

	pub test_fr_activation: bool,
	pub test_fr_confirmation: bool,
}

impl UFrame {
	#[instrument]
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

	fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), Box<Error>> {
		let mut byte: u8 = 0b0000_0011;
		if self.start_dt_activation {
			byte |= 0b0000_0100;
		}
		if self.start_dt_confirmation {
			byte |= 0b0000_1000;
		}
		if self.stop_dt_activation {
			byte |= 0b0001_0000;
		}
		if self.stop_dt_confirmation {
			byte |= 0b0010_0000;
		}
		if self.test_fr_activation {
			byte |= 0b0100_0000;
		}
		if self.test_fr_confirmation {
			byte |= 0b1000_0000;
		}
		buffer.push(byte);
		buffer.push(0);
		buffer.push(0);
		buffer.push(0);
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		cot::Cot,
		types::{
			InformationObject,
			commands::{Frz, Rqt},
		},
		types_id::TypeId,
	};

	#[test]
	fn test_s_frame() -> Result<(), Box<Error>> {
		let bytes = [0x68, 0x04, 0x01, 0x00, 0x7E, 0x14];
		let apdu = Apdu::from_bytes(&bytes)?;
		assert_eq!(apdu.length, 4);

		let Frame::S(s_frame) = &apdu.frame else { panic!("Frame was expected to be an S-frame") };
		assert_eq!(s_frame.receive_sequence_number, 5183);

		let apdu_bytes = apdu.to_bytes()?;
		assert_eq!(apdu_bytes, bytes);

		Ok(())
	}

	#[test]
	fn test_i_frame() -> Result<(), Box<Error>> {
		let bytes = [
			0x68, 0x34, 0x5A, 0x14, 0x7C, 0x00, 0x0B, 0x07, 0x03, 0x00, 0x0C, 0x00, 0x10, 0x30,
			0x00, 0xBE, 0x09, 0x00, 0x11, 0x30, 0x00, 0x90, 0x09, 0x00, 0x0E, 0x30, 0x00, 0x75,
			0x00, 0x00, 0x28, 0x30, 0x00, 0x25, 0x09, 0x00, 0x29, 0x30, 0x00, 0x75, 0x00, 0x00,
			0x0F, 0x30, 0x00, 0x0F, 0x0A, 0x00, 0x2E, 0x30, 0x00, 0xAE, 0x05, 0x00,
		];
		let apdu = Apdu::from_bytes(&bytes)?;
		assert_eq!(apdu.length, 52);

		let Frame::I(i_frame) = &apdu.frame else { panic!("Frame was expected to be an I-frame") };

		assert_eq!(i_frame.send_sequence_number, 5165);
		assert_eq!(i_frame.receive_sequence_number, 62);
		assert_eq!(i_frame.asdu.type_id, TypeId::M_ME_NB_1);
		assert_eq!(i_frame.asdu.information_objects.len(), 7);
		assert_eq!(i_frame.asdu.cot, Cot::SpontaneousData);
		assert_eq!(i_frame.asdu.originator_address, 0);
		assert_eq!(i_frame.asdu.address_field, 12);
		assert!(!i_frame.asdu.sequence);
		assert!(!i_frame.asdu.test);
		assert!(!i_frame.asdu.positive);
		let InformationObject::MMeNb1(objects) = &i_frame.asdu.information_objects else {
			panic!("Information objects were expected to be a MMeNb1")
		};
		assert_eq!(objects.len(), 7);

		assert_eq!(objects[0].address, 12304);
		assert_eq!(objects[0].object.sva.value, 2494);
		assert!(!objects[0].object.qds.iv);
		assert!(!objects[0].object.qds.nt);
		assert!(!objects[0].object.qds.sb);
		assert!(!objects[0].object.qds.bl);
		assert!(!objects[0].object.qds.ov);

		assert_eq!(objects[1].address, 12305);
		assert_eq!(objects[1].object.sva.value, 2448);
		assert!(!objects[1].object.qds.iv);
		assert!(!objects[1].object.qds.nt);
		assert!(!objects[1].object.qds.sb);
		assert!(!objects[1].object.qds.bl);
		assert!(!objects[1].object.qds.ov);

		assert_eq!(objects[2].address, 12302);
		assert_eq!(objects[2].object.sva.value, 117);
		assert!(!objects[2].object.qds.iv);
		assert!(!objects[2].object.qds.nt);
		assert!(!objects[2].object.qds.sb);
		assert!(!objects[2].object.qds.bl);
		assert!(!objects[2].object.qds.ov);

		assert_eq!(objects[3].address, 12328);
		assert_eq!(objects[3].object.sva.value, 2341);
		assert!(!objects[3].object.qds.iv);
		assert!(!objects[3].object.qds.nt);
		assert!(!objects[3].object.qds.sb);
		assert!(!objects[3].object.qds.bl);
		assert!(!objects[3].object.qds.ov);

		assert_eq!(objects[4].address, 12329);
		assert_eq!(objects[4].object.sva.value, 117);
		assert!(!objects[4].object.qds.iv);
		assert!(!objects[4].object.qds.nt);
		assert!(!objects[4].object.qds.sb);
		assert!(!objects[4].object.qds.bl);
		assert!(!objects[4].object.qds.ov);

		assert_eq!(objects[5].address, 12303);
		assert_eq!(objects[5].object.sva.value, 2575);
		assert!(!objects[5].object.qds.iv);
		assert!(!objects[5].object.qds.nt);
		assert!(!objects[5].object.qds.sb);
		assert!(!objects[5].object.qds.bl);
		assert!(!objects[5].object.qds.ov);

		assert_eq!(objects[6].address, 12334);
		assert_eq!(objects[6].object.sva.value, 1454);
		assert!(!objects[6].object.qds.iv);
		assert!(!objects[6].object.qds.nt);
		assert!(!objects[6].object.qds.sb);
		assert!(!objects[6].object.qds.bl);
		assert!(!objects[6].object.qds.ov);

		let apdu_bytes = apdu.to_bytes()?;
		assert_eq!(apdu_bytes, bytes);

		Ok(())
	}

	#[test]
	fn test_i_frame_command() -> Result<(), Box<Error>> {
		let bytes = [
			0x68, 0x0E, 0x4E, 0x14, 0x7C, 0x00, 0x65, 0x01, 0x0A, 0x00, 0x0C, 0x00, 0x00, 0x00,
			0x00, 0x05,
		];
		let apdu = Apdu::from_bytes(&bytes)?;
		assert_eq!(apdu.length, 14);

		let Frame::I(i_frame) = &apdu.frame else { panic!("Frame was expected to be an I-frame") };
		assert_eq!(i_frame.send_sequence_number, 5159);
		assert_eq!(i_frame.receive_sequence_number, 62);
		assert_eq!(i_frame.asdu.type_id, TypeId::C_CI_NA_1);
		assert_eq!(i_frame.asdu.information_objects.len(), 1);
		assert_eq!(i_frame.asdu.cot, Cot::ActivationTermination);
		assert_eq!(i_frame.asdu.originator_address, 0);
		assert_eq!(i_frame.asdu.address_field, 12);
		assert!(!i_frame.asdu.sequence);
		assert!(!i_frame.asdu.test);
		assert!(!i_frame.asdu.positive);
		let InformationObject::CCiNa1(objects) = &i_frame.asdu.information_objects else {
			panic!("Information objects were expected to be a CCiNa1")
		};
		assert_eq!(objects.len(), 1);
		assert_eq!(objects[0].address, 0);
		assert_eq!(objects[0].object.rqt, Rqt::ReqCoGen);
		assert_eq!(objects[0].object.frz, Frz::Read);

		let apdu_bytes = apdu.to_bytes()?;
		assert_eq!(apdu_bytes, bytes);

		Ok(())
	}

	#[test]
	fn test_u_frame() -> Result<(), Box<Error>> {
		let bytes = [0x68, 0x04, 0x01, 0x00, 0x7E, 0x14];
		let apdu = Apdu::from_bytes(&bytes)?;
		assert_eq!(apdu.length, 4);

		Ok(())
	}
}
