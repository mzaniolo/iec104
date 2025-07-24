use crate::types::information_elements::SelectExecute;

/// Quality descriptor
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Qds {
	/// Invalid
	pub iv: bool,
	/// Not topical
	pub nt: bool,
	/// Substituted
	pub sb: bool,
	/// Blocked
	pub bl: bool,
	/// Overflow
	pub ov: bool,
}

impl Qds {
	pub const fn from_byte(byte: u8) -> Self {
		let iv = byte & 0b1000_0000 != 0;
		let nt = byte & 0b0100_0000 != 0;
		let sb = byte & 0b0010_0000 != 0;
		let bl = byte & 0b0001_0000 != 0;
		let ov = byte & 0b0000_0001 != 0;
		Qds { iv, nt, sb, bl, ov }
	}
}

/// Sequence quality descriptor
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct SeqQd {
	/// Invalid
	pub iv: bool,
	/// Adjusted flag
	pub ca: bool,
	/// Carry flag
	pub cy: bool,
	/// Sequence
	pub seq: u8,
}

impl SeqQd {
	pub const fn from_byte(byte: u8) -> Self {
		let iv = byte & 0b1000_0000 != 0;
		let ca = byte & 0b0100_0000 != 0;
		let cy = byte & 0b0010_0000 != 0;
		let seq = byte & 0b0001_1111;
		SeqQd { iv, ca, cy, seq }
	}
}

/// Quality descriptor of protection equipment
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Qdp {
	/// Invalid
	pub iv: bool,
	/// Not topical
	pub nt: bool,
	/// Substituted
	pub sb: bool,
	/// Blocked
	pub bl: bool,
	/// Elapsed flag
	pub ei: bool,
}

impl Qdp {
	pub const fn from_byte(byte: u8) -> Self {
		let iv = byte & 0b1000_0000 != 0;
		let nt = byte & 0b0100_0000 != 0;
		let sb = byte & 0b0010_0000 != 0;
		let bl = byte & 0b0001_0000 != 0;
		let ei = byte & 0b0000_1000 != 0;
		Qdp { iv, nt, sb, bl, ei }
	}
}

/// Qualifier of set point command
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Qos {
	/// Select/execute
	pub se: SelectExecute,
	/// Qualifier
	pub ql: bool,
}

impl Qos {
	pub const fn from_byte(byte: u8) -> Self {
		let se = SelectExecute::from_bool(byte & 0b1000_0000 != 0);
		let ql = byte & 0b0000_0001 != 0;
		Qos { se, ql }
	}
}
