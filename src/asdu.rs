use snafu::{ResultExt as _, Snafu};
use tracing::instrument;

use crate::{
	cot::{Cot, CotError},
	error::SpanTraceWrapper,
	types::{InformationObject, ParseError},
	types_id::TypeId,
};

pub struct Asdu {
	type_id: TypeId,
	cot: Cot,
	originator_address: u8,
	address_field: u16,
	sequence: bool,
	test: bool,
	positive: bool,
	information_objects: InformationObject,
}

impl Asdu {
	#[instrument]
	pub fn parse(bytes: &[u8]) -> Result<Self, AsduError> {
		let type_id: TypeId = bytes[0].into();

		let sequence = bytes[1] & 0b1000_0000 != 0;
		let num_objs = bytes[1] & 0b0111_1111;

		let test = bytes[2] & 0b1000_0000 != 0;
		let positive = bytes[2] & 0b0100_0000 != 0;
		let cot = (bytes[2] & 0b0011_1111).try_into().context(InvalidCot)?;

		let originator_address = bytes[3];

		let address_field = u16::from_be_bytes([bytes[5], bytes[4]]);

		let object_size = type_id.size();
		let remaining_bytes = bytes[6..].len();

		// Check if the remaining bytes are a multiple of the object size
		// If it's a sequence we need to know the first address. So the first object has
		// object_size + 3 bytes for the address. The subsequent chunks only
		// have the object_size.
		let is_multiple = if sequence {
			(remaining_bytes - 3) % object_size != 0
		} else {
			remaining_bytes % (object_size + 3) != 0
		};

		// Check if the number of objects is correct
		// Here we have the same problem as above.
		let num_objs_expected = if sequence {
			(remaining_bytes - 3) / object_size != num_objs.into()
		} else {
			remaining_bytes / (object_size + 3) != num_objs.into()
		};

		if is_multiple || num_objs_expected {
			NumberOfObjects { num_objs, object_size, remaining_bytes }.fail()?;
		}

		let information_objects =
			InformationObject::from_bytes(type_id, sequence, num_objs, &bytes[6..])
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
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub), context(suffix(false)))]
pub enum AsduError {
	#[snafu(display("Invalid COT"))]
	InvalidCot {
		source: CotError,
		#[snafu(implicit)]
		context: SpanTraceWrapper,
	},

	#[snafu(display(
		"Invalid number of objects. Expected {num_objs} objects, got {remaining_bytes}/{object_size} objects."
	))]
	NumberOfObjects {
		num_objs: u8,
		object_size: usize,
		remaining_bytes: usize,
		#[snafu(implicit)]
		context: SpanTraceWrapper,
	},

	#[snafu(display("Invalid information object"))]
	InvalidInformationObject {
		source: Box<ParseError>,
		#[snafu(implicit)]
		context: SpanTraceWrapper,
	},
}
