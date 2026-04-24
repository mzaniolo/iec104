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
	#[must_use]
	pub const fn from_byte(byte: u8) -> Self {
		let iv = byte & 0b1000_0000 != 0;
		let nt = byte & 0b0100_0000 != 0;
		let sb = byte & 0b0010_0000 != 0;
		let bl = byte & 0b0001_0000 != 0;
		let ov = byte & 0b0000_0001 != 0;
		Qds { iv, nt, sb, bl, ov }
	}

	#[must_use]
	pub const fn to_byte(&self) -> u8 {
		let mut byte: u8 = 0;
		byte |= (self.iv as u8) << 7;
		byte |= (self.nt as u8) << 6;
		byte |= (self.sb as u8) << 5;
		byte |= (self.bl as u8) << 4;
		byte |= self.ov as u8;
		byte
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
	#[must_use]
	pub const fn from_byte(byte: u8) -> Self {
		let iv = byte & 0b1000_0000 != 0;
		let ca = byte & 0b0100_0000 != 0;
		let cy = byte & 0b0010_0000 != 0;
		let seq = byte & 0b0001_1111;
		SeqQd { iv, ca, cy, seq }
	}

	#[must_use]
	pub const fn to_byte(&self) -> u8 {
		let mut byte: u8 = 0;
		byte |= (self.iv as u8) << 7;
		byte |= (self.ca as u8) << 6;
		byte |= (self.cy as u8) << 5;
		byte |= self.seq & 0b0001_1111;
		byte
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
	#[must_use]
	pub const fn from_byte(byte: u8) -> Self {
		let iv = byte & 0b1000_0000 != 0;
		let nt = byte & 0b0100_0000 != 0;
		let sb = byte & 0b0010_0000 != 0;
		let bl = byte & 0b0001_0000 != 0;
		let ei = byte & 0b0000_1000 != 0;
		Qdp { iv, nt, sb, bl, ei }
	}

	#[must_use]
	pub const fn to_byte(&self) -> u8 {
		let mut byte: u8 = 0;
		byte |= (self.iv as u8) << 7;
		byte |= (self.nt as u8) << 6;
		byte |= (self.sb as u8) << 5;
		byte |= (self.bl as u8) << 4;
		byte |= (self.ei as u8) << 3;
		byte
	}
}

/// Qualifier of set point command
///
/// Per IEC 60870-5-101 §7.2.6.39:
///   bits 0-6 = QL (7-bit qualifier, 0 = default)
///   bit 7    = S/E (select/execute)
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Qos {
	/// Select/execute
	pub se: SelectExecute,
	/// Qualifier (7-bit, 0..=127)
	pub ql: u8,
}

impl Qos {
	#[must_use]
	pub const fn from_byte(byte: u8) -> Self {
		let se = SelectExecute::from_bool(byte & 0b1000_0000 != 0);
		let ql = byte & 0b0111_1111;
		Qos { se, ql }
	}

	#[must_use]
	pub const fn to_byte(&self) -> u8 {
		let mut byte: u8 = 0;
		byte |= (self.se as u8) << 7;
		byte |= self.ql & 0b0111_1111;
		byte
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	// ---- Qos: QL is a 7-bit qualifier, not a bool ----

	#[test]
	fn qos_ql_preserves_full_seven_bits() {
		// QL = 0x42 (66), S/E = Execute -> byte = 0b0100_0010
		let q = Qos::from_byte(0b0100_0010);
		assert_eq!(q.ql, 0x42);
		assert_eq!(q.se, SelectExecute::Execute);
		assert_eq!(q.to_byte(), 0b0100_0010);
	}

	#[test]
	fn qos_ql_max_value_round_trips() {
		// QL = 0x7F (max 7-bit), S/E = Select -> 0xFF
		let q = Qos::from_byte(0xFF);
		assert_eq!(q.ql, 0x7F);
		assert_eq!(q.se, SelectExecute::Select);
		assert_eq!(q.to_byte(), 0xFF);
	}

	#[test]
	fn qos_serialization_masks_ql_to_seven_bits() {
		// If a caller sets ql > 127, only the low 7 bits go on the wire.
		let q = Qos { se: SelectExecute::Execute, ql: 0xFF };
		assert_eq!(q.to_byte(), 0x7F);
	}

	// ---- SeqQd: BCR's quality descriptor (used by M_IT_NA_1 / M_IT_TB_1) ----

	#[test]
	fn seq_qd_decodes_each_flag() {
		// IV=1 CA=0 CY=1 seq=0b0_0001 -> 0b1010_0001 = 0xA1
		let q = SeqQd::from_byte(0xA1);
		assert!(q.iv);
		assert!(!q.ca);
		assert!(q.cy);
		assert_eq!(q.seq, 1);
		assert_eq!(q.to_byte(), 0xA1);
	}

	#[test]
	fn seq_qd_seq_max_round_trip() {
		// seq = 31 (max 5-bit), all flags clear
		let q = SeqQd::from_byte(0x1F);
		assert_eq!(q.seq, 31);
		assert_eq!(q.to_byte(), 0x1F);
	}
}
