use snafu::{OptionExt as _, ResultExt as _, Snafu};
use tracing::instrument;

use crate::{
	cot::{Cot, CotError},
	error::SpanTraceWrapper,
	types::{InformationObjects, ParseError},
	types_id::TypeId,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Asdu {
	pub type_id: TypeId,
	pub cot: Cot,
	pub originator_address: u8,
	pub address_field: u16,
	pub sequence: bool,
	pub test: bool,
	pub negative: bool,
	pub information_objects: InformationObjects,
}

impl Asdu {
	#[instrument]
	pub fn parse(bytes: &[u8]) -> Result<Self, AsduError> {
		tracing::trace!("Parsing ASDU: {:?}", bytes);
		let type_id: TypeId = (*bytes.first().context(NotEnoughBytes)?).into();

		let byte = bytes.get(1).context(NotEnoughBytes)?;
		let sequence = byte & 0b1000_0000 != 0;
		let num_objs = byte & 0b0111_1111;

		let byte = bytes.get(2).context(NotEnoughBytes)?;
		let test = byte & 0b1000_0000 != 0;
		let negative = byte & 0b0100_0000 != 0;
		let cot = (byte & 0b0011_1111).try_into().context(InvalidCot)?;

		let originator_address = *bytes.get(3).context(NotEnoughBytes)?;

		let address_field = u16::from_le_bytes([
			*bytes.get(4).context(NotEnoughBytes)?,
			*bytes.get(5).context(NotEnoughBytes)?,
		]);

		let remaining_bytes = bytes.get(6..).context(NotEnoughBytes)?;
		if type_id.is_standard() {
			let object_size = type_id.size();
			let remaining_bytes_size = remaining_bytes.len();

			// Validate the wire layout in one place using checked arithmetic.
			// A sequence carries one 3-byte IOA followed by `num_objs` payloads
			// of `object_size` bytes each; a non-sequence is `num_objs` repeats
			// of `(IOA + payload)`. Both pre-fix branches did
			// `remaining_bytes_size - 3` which underflows on a malformed
			// short ASDU.
			let expected = if sequence {
				(num_objs as usize)
					.checked_mul(object_size)
					.and_then(|n| n.checked_add(3))
			} else {
				(num_objs as usize).checked_mul(object_size.saturating_add(3))
			};
			if expected != Some(remaining_bytes_size) {
				return NumberOfObjects {
					num_objs,
					object_size,
					remaining_bytes: remaining_bytes_size,
				}
				.fail();
			}
		}

		let information_objects =
			InformationObjects::from_bytes(type_id, sequence, num_objs, remaining_bytes)
				.context(InvalidInformationObject)?;

		Ok(Self {
			type_id,
			cot,
			originator_address,
			address_field,
			sequence,
			test,
			negative,
			information_objects,
		})
	}
	pub fn to_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), AsduError> {
		buffer.push(self.type_id as u8);
		let num_objs = self.information_objects.len();
		if num_objs > 127 {
			return TooManyObjects { num_objs }.fail();
		}
		let mut byte: u8 = num_objs as u8;
		if self.sequence {
			byte |= 0b1000_0000;
		}
		buffer.push(byte);

		let mut byte: u8 = self.cot as u8;
		if self.test {
			byte |= 0b1000_0000;
		}
		if self.negative {
			byte |= 0b0100_0000;
		}
		buffer.push(byte);

		buffer.push(self.originator_address);

		let address_field = self.address_field.to_le_bytes();
		buffer.push(address_field[0]);
		buffer.push(address_field[1]);

		self.information_objects.to_bytes(buffer).context(InvalidInformationObject)?;
		Ok(())
	}
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub), context(suffix(false)))]
pub enum AsduError {
	#[snafu(display("Invalid COT"))]
	InvalidCot {
		source: CotError,
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},

	#[snafu(display(
		"Invalid number of objects. Expected {num_objs} objects, got {remaining_bytes}/{object_size} objects."
	))]
	NumberOfObjects {
		num_objs: u8,
		object_size: usize,
		remaining_bytes: usize,
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},

	#[snafu(display("Invalid information object"))]
	InvalidInformationObject {
		source: ParseError,
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
	#[snafu(display("Too many objects. Max number of objects is 127, got {num_objs} objects."))]
	TooManyObjects {
		num_objs: usize,
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},

	#[snafu(display("Not enough bytes"))]
	NotEnoughBytes {
		#[snafu(implicit)]
		context: Box<SpanTraceWrapper>,
	},
}

#[cfg(test)]
mod tests {
	use super::*;

	/// Header common to all the malformed-tail tests below: type
	/// `M_SP_NA_1` (size = 1 byte), 1 object, COT = SpontaneousData, OA = 0,
	/// CA = 1.
	const HEADER: [u8; 6] = [0x01, 0x01, 0x03, 0x00, 0x01, 0x00];

	#[test]
	fn parse_sequence_with_short_tail_does_not_panic() {
		// sequence=true means SQ bit set on byte 1 (0x80 | 0x01 = 0x81).
		// A sequence ASDU needs at least 3 bytes (IOA) + N*size — give
		// it only 2 bytes so the pre-fix `remaining_bytes_size - 3`
		// would underflow.
		let mut bytes = HEADER;
		bytes[1] = 0x81;
		let mut input = bytes.to_vec();
		input.extend_from_slice(&[0x00, 0x00]); // 2-byte tail (< 3)
		let err = Asdu::parse(&input).expect_err("must reject short sequence ASDU");
		assert!(matches!(err, AsduError::NumberOfObjects { .. }), "got: {err:?}");
	}

	#[test]
	fn parse_sequence_with_zero_tail_does_not_panic() {
		// Even tighter: zero tail bytes after the 6-byte ASDU header.
		let mut bytes = HEADER;
		bytes[1] = 0x81;
		let err = Asdu::parse(&bytes).expect_err("must reject empty sequence tail");
		assert!(matches!(err, AsduError::NumberOfObjects { .. }), "got: {err:?}");
	}

	#[test]
	fn parse_non_sequence_with_short_tail_rejected() {
		// Non-sequence M_SP_NA_1 with num_objs=1 needs 4 bytes (3 IOA + 1
		// payload). Provide only 3 bytes (just the IOA).
		let mut input = HEADER.to_vec();
		input.extend_from_slice(&[0x10, 0x00, 0x00]);
		let err = Asdu::parse(&input).expect_err("must reject short non-sequence ASDU");
		assert!(matches!(err, AsduError::NumberOfObjects { .. }), "got: {err:?}");
	}

	#[test]
	fn parse_non_sequence_with_extra_byte_rejected() {
		// Trailing byte beyond what num_objs implies — the previous
		// `chunks` (rather than `chunks_exact`) would have surfaced this
		// only later as a panic on indexing the short last chunk.
		let mut input = HEADER.to_vec();
		input.extend_from_slice(&[0x10, 0x00, 0x00, 0x00, 0xFF]);
		let err = Asdu::parse(&input).expect_err("must reject extra-byte non-sequence ASDU");
		assert!(matches!(err, AsduError::NumberOfObjects { .. }), "got: {err:?}");
	}
}
