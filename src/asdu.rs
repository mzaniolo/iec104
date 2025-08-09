use snafu::{OptionExt as _, ResultExt as _, Snafu};
use tracing::instrument;

use crate::{
	cot::{Cot, CotError},
	error::SpanTraceWrapper,
	types::{InformationObject, ParseError},
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
	pub positive: bool,
	pub information_objects: InformationObject,
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
		let positive = byte & 0b0100_0000 != 0;
		let cot = (byte & 0b0011_1111).try_into().context(InvalidCot)?;

		let originator_address = *bytes.get(3).context(NotEnoughBytes)?;

		let address_field = u16::from_le_bytes([
			*bytes.get(4).context(NotEnoughBytes)?,
			*bytes.get(5).context(NotEnoughBytes)?,
		]);

		let object_size = type_id.size();
		let remaining_bytes = bytes.get(6..).context(NotEnoughBytes)?;
		let remaining_bytes_size = remaining_bytes.len();

		// Check if the remaining bytes are a multiple of the object size
		// If it's a sequence we need to know the first address. So the first object has
		// object_size + 3 bytes for the address. The subsequent chunks only
		// have the object_size.
		let is_multiple = if sequence {
			(remaining_bytes_size - 3) % object_size != 0
		} else {
			remaining_bytes_size % (object_size + 3) != 0
		};

		// Check if the number of objects is correct
		// Here we have the same problem as above.
		let num_objs_expected = if sequence {
			(remaining_bytes_size - 3) / object_size != num_objs.into()
		} else {
			remaining_bytes_size / (object_size + 3) != num_objs.into()
		};

		if is_multiple || num_objs_expected {
			return NumberOfObjects {
				num_objs,
				object_size,
				remaining_bytes: remaining_bytes_size,
			}
			.fail();
		}

		let information_objects =
			InformationObject::from_bytes(type_id, sequence, num_objs, remaining_bytes)
				.context(InvalidInformationObject)?;

		Ok(Self {
			type_id,
			cot,
			originator_address,
			address_field,
			sequence,
			test,
			positive,
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
		if self.positive {
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
