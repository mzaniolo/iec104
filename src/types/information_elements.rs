use crate::types::quality_descriptors::Qds;

/// Single point information with quality descriptor
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Siq {
	/// Invalid
	pub iv: bool,
	/// Not topical
	pub nt: bool,
	/// Substituted
	pub sb: bool,
	/// Blocked
	pub bl: bool,
	/// Single point information
	pub spi: Spi,
}

impl Siq {
	#[must_use]
	pub const fn from_byte(byte: u8) -> Self {
		let iv = byte & 0b1000_0000 != 0;
		let nt = byte & 0b0100_0000 != 0;
		let sb = byte & 0b0010_0000 != 0;
		let bl = byte & 0b0001_0000 != 0;
		let spi = Spi::from_byte(byte & 0b0000_0001);
		Siq { iv, nt, sb, bl, spi }
	}
}

impl Siq {
	#[must_use]
	pub const fn to_byte(&self) -> u8 {
		let mut byte: u8 = 0;
		byte |= (self.iv as u8) << 7;
		byte |= (self.nt as u8) << 6;
		byte |= (self.sb as u8) << 5;
		byte |= (self.bl as u8) << 4;
		byte |= self.spi as u8;
		byte
	}
}

/// Single point information
#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum Spi {
	#[default]
	/// Off
	Off = 0,
	/// On
	On = 1,
}

impl Spi {
	#[must_use]
	pub const fn from_byte(byte: u8) -> Self {
		if byte == 0 { Spi::Off } else { Spi::On }
	}
}
/// Double point information with quality descriptor
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Diq {
	/// Invalid
	pub iv: bool,
	/// Not topical
	pub nt: bool,
	/// Substituted
	pub sb: bool,
	/// Blocked
	pub bl: bool,
	/// Double point information
	pub dpi: Dpi,
}

impl Diq {
	#[must_use]
	pub const fn from_byte(byte: u8) -> Self {
		let iv = byte & 0b1000_0000 != 0;
		let nt = byte & 0b0100_0000 != 0;
		let sb = byte & 0b0010_0000 != 0;
		let bl = byte & 0b0001_0000 != 0;
		let dpi = Dpi::from_byte(byte & 0b0000_0011);
		Diq { iv, nt, sb, bl, dpi }
	}

	#[must_use]
	pub const fn to_byte(&self) -> u8 {
		let mut byte: u8 = 0;
		byte |= (self.iv as u8) << 7;
		byte |= (self.nt as u8) << 6;
		byte |= (self.sb as u8) << 5;
		byte |= (self.bl as u8) << 4;
		byte |= self.dpi as u8;
		byte
	}
}

/// Double-point information
#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum Dpi {
	#[default]
	/// Indeterminate 0
	Indeterminate = 0,
	/// Off
	Off = 1,
	/// On
	On = 2,
	/// Invalid
	Invalid = 3,
}

impl Dpi {
	#[must_use]
	pub const fn from_byte(byte: u8) -> Self {
		match byte {
			0 => Dpi::Indeterminate,
			1 => Dpi::Off,
			2 => Dpi::On,
			_ => Dpi::Invalid,
		}
	}
}

/// Value with transient state indication
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Vti {
	/// Value
	pub value: u8,
	/// Transient state indication
	pub transient: bool,
	/// Quality descriptor
	pub qds: Qds,
}

impl Vti {
	#[must_use]
	pub const fn from_byte(bytes: [u8; 2]) -> Self {
		let value = bytes[0] & 0b0111_1111;
		let transient = bytes[0] & 0b1000_0000 != 0;
		let qds = Qds::from_byte(bytes[1]);
		Vti { value, transient, qds }
	}

	#[must_use]
	pub const fn to_bytes(&self) -> [u8; 2] {
		let mut bytes: [u8; 2] = [0, 0];
		bytes[0] |= self.value & 0b0111_1111;
		bytes[0] |= (self.transient as u8) << 7;
		bytes[1] |= self.qds.to_byte();
		bytes
	}
}

/// Event state (single event of protection equipment)
#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum EventState {
	#[default]
	/// Indeterminate
	Indeterminate = 0,
	/// Off
	Off = 1,
	/// On
	On = 2,
	/// Invalid
	Invalid = 3,
}

impl EventState {
	#[must_use]
	pub const fn from_byte(byte: u8) -> Self {
		match byte {
			0 => EventState::Indeterminate,
			1 => EventState::Off,
			2 => EventState::On,
			_ => EventState::Invalid,
		}
	}
}

/// Single event of protection equipment
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Sep {
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
	/// Event state
	pub es: EventState,
}

impl Sep {
	#[must_use]
	pub const fn from_byte(byte: u8) -> Self {
		let iv = byte & 0b1000_0000 != 0;
		let nt = byte & 0b0100_0000 != 0;
		let sb = byte & 0b0010_0000 != 0;
		let bl = byte & 0b0001_0000 != 0;
		let ei = byte & 0b0000_1000 != 0;
		let es = EventState::from_byte(byte & 0b0000_0011);
		Sep { iv, nt, sb, bl, ei, es }
	}

	#[must_use]
	pub const fn to_byte(&self) -> u8 {
		let mut byte: u8 = 0;
		byte |= (self.iv as u8) << 7;
		byte |= (self.nt as u8) << 6;
		byte |= (self.sb as u8) << 5;
		byte |= (self.bl as u8) << 4;
		byte |= (self.ei as u8) << 3;
		byte |= self.es as u8;
		byte
	}
}

/// Start events of protection equipment
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct StartEp {
	/// SRD
	pub srd: bool,
	/// SIE
	pub sie: bool,
	/// SL3
	pub sl3: bool,
	/// SL2
	pub sl2: bool,
	/// SL1
	pub sl1: bool,
	/// GS
	pub gs: bool,
}

impl StartEp {
	#[must_use]
	pub const fn from_byte(byte: u8) -> Self {
		let srd = byte & 0b0010_0000 != 0;
		let sie = byte & 0b0001_0000 != 0;
		let sl3 = byte & 0b0000_1000 != 0;
		let sl2 = byte & 0b0000_0100 != 0;
		let sl1 = byte & 0b0000_0010 != 0;
		let gs = byte & 0b0000_0001 != 0;
		StartEp { srd, sie, sl3, sl2, sl1, gs }
	}

	#[must_use]
	pub const fn to_byte(&self) -> u8 {
		let mut byte: u8 = 0;
		byte |= (self.srd as u8) << 5;
		byte |= (self.sie as u8) << 4;
		byte |= (self.sl3 as u8) << 3;
		byte |= (self.sl2 as u8) << 2;
		byte |= (self.sl1 as u8) << 1;
		byte |= self.gs as u8;
		byte
	}
}

/// Output circuit information
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Oci {
	/// CL3
	pub cl3: bool,
	/// CL2
	pub cl2: bool,
	/// CL1
	pub cl1: bool,
	/// GC
	pub gc: bool,
}

impl Oci {
	#[must_use]
	pub const fn from_byte(byte: u8) -> Self {
		let cl3 = byte & 0b0000_1000 != 0;
		let cl2 = byte & 0b0000_0100 != 0;
		let cl1 = byte & 0b0000_0010 != 0;
		let gc = byte & 0b0000_0001 != 0;
		Oci { cl3, cl2, cl1, gc }
	}

	#[must_use]
	pub const fn to_byte(&self) -> u8 {
		let mut byte: u8 = 0;
		byte |= (self.cl3 as u8) << 3;
		byte |= (self.cl2 as u8) << 2;
		byte |= (self.cl1 as u8) << 1;
		byte |= self.gc as u8;
		byte
	}
}

/// Select/execute command
#[derive(Debug, Clone, Eq, PartialEq, Default, Copy)]
#[repr(u8)]
pub enum SelectExecute {
	/// Execute
	Execute = 0,
	#[default]
	/// Select
	Select = 1,
}

impl SelectExecute {
	#[must_use]
	pub const fn from_bool(select: bool) -> Self {
		match select {
			true => SelectExecute::Select,
			false => SelectExecute::Execute,
		}
	}
}

/// Local parameter change
#[derive(Debug, Clone, Eq, PartialEq, Default, Copy)]
#[repr(u8)]
pub enum Lpc {
	#[default]
	/// No change
	NoChange = 0,
	/// Changed
	Changed = 1,
}

impl Lpc {
	#[must_use]
	pub const fn from_bool(state: bool) -> Self {
		match state {
			true => Lpc::Changed,
			false => Lpc::NoChange,
		}
	}
}

/// Cause of initialization
#[derive(Debug, Clone, Eq, PartialEq, Default)]
#[repr(u8)]
pub enum Coi {
	#[default]
	/// Local power on
	LocalPowerOn = 0,
	/// Local manual reset
	LocalManualReset = 1,
	/// Remote reset
	RemoteReset = 2,
	/// Other (custom)
	Other(u8),
}

impl Coi {
	#[must_use]
	pub const fn from_byte(byte: u8) -> Self {
		match byte {
			0 => Coi::LocalPowerOn,
			1 => Coi::LocalManualReset,
			2 => Coi::RemoteReset,
			_ => Coi::Other(byte),
		}
	}

	#[must_use]
	pub const fn to_byte(&self) -> u8 {
		match self {
			Coi::LocalPowerOn => 0,
			Coi::LocalManualReset => 1,
			Coi::RemoteReset => 2,
			Coi::Other(byte) => *byte,
		}
	}
}
