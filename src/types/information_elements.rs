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

impl Siq {
	pub const fn from_byte(byte: u8) -> Self {
		let iv = byte & 0b1000_0000 != 0;
		let nt = byte & 0b0100_0000 != 0;
		let sb = byte & 0b0010_0000 != 0;
		let bl = byte & 0b0001_0000 != 0;
		let spi = Spi::from_byte(byte & 0b0000_0001);
		Siq { iv, nt, sb, bl, spi }
	}
}

impl Spi {
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
	pub const fn from_byte(byte: u8) -> Self {
		let iv = byte & 0b1000_0000 != 0;
		let nt = byte & 0b0100_0000 != 0;
		let sb = byte & 0b0010_0000 != 0;
		let bl = byte & 0b0001_0000 != 0;
		let dpi = Dpi::from_byte(byte & 0b0000_0011);
		Diq { iv, nt, sb, bl, dpi }
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
	pub const fn from_byte(bytes: &[u8]) -> Self {
		let value = bytes[0] & 0b0111_1111;
		let transient = bytes[0] & 0b1000_0000 != 0;
		let qds = Qds::from_byte(bytes[1]);
		Vti { value, transient, qds }
	}
}

/// Bit string of 32 bits
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Bsi {
	/// Value
	pub value: u32,
}

impl Bsi {
	pub const fn from_byte(bytes: &[u8]) -> Self {
		let value = u32::from_be_bytes([bytes[3], bytes[2], bytes[1], bytes[0]]);
		Bsi { value }
	}
}

/// Normalized value
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Nva {
	/// Value
	pub value: u16,
}

impl Nva {
	pub const fn from_bytes(bytes: &[u8]) -> Self {
		let value = u16::from_be_bytes([bytes[1], bytes[0]]);
		Nva { value }
	}
}

/// Scaled value
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Sva {
	/// Value
	pub value: u16,
}

impl Sva {
	pub const fn from_bytes(bytes: &[u8]) -> Self {
		let value = u16::from_be_bytes([bytes[1], bytes[0]]);
		Sva { value }
	}
}

/// Short floating point
#[derive(Debug, Clone, PartialEq, Default)]
pub struct R32 {
	/// Value
	pub value: f32,
}

impl R32 {
	pub const fn from_bytes(bytes: &[u8]) -> Self {
		let value = f32::from_be_bytes([bytes[3], bytes[2], bytes[1], bytes[0]]);
		R32 { value }
	}
}

impl Eq for R32 {}

/// Binary counter reading
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Bcr {
	/// Value
	pub value: u32,
}

impl Bcr {
	pub const fn from_byte(bytes: &[u8]) -> Self {
		let value = u32::from_be_bytes([bytes[3], bytes[2], bytes[1], bytes[0]]);
		Bcr { value }
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
	const fn from_byte(byte: u8) -> Self {
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
	pub const fn from_byte(byte: u8) -> Self {
		let iv = byte & 0b1000_0000 != 0;
		let nt = byte & 0b0100_0000 != 0;
		let sb = byte & 0b0010_0000 != 0;
		let bl = byte & 0b0001_0000 != 0;
		let ei = byte & 0b0000_1000 != 0;
		let es = EventState::from_byte(byte & 0b0000_0011);
		Sep { iv, nt, sb, bl, ei, es }
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
	pub const fn from_byte(byte: u8) -> Self {
		let srd = byte & 0b0010_0000 != 0;
		let sie = byte & 0b0001_0000 != 0;
		let sl3 = byte & 0b0000_1000 != 0;
		let sl2 = byte & 0b0000_0100 != 0;
		let sl1 = byte & 0b0000_0010 != 0;
		let gs = byte & 0b0000_0001 != 0;
		StartEp { srd, sie, sl3, sl2, sl1, gs }
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
	pub const fn from_byte(byte: u8) -> Self {
		let cl3 = byte & 0b0000_1000 != 0;
		let cl2 = byte & 0b0000_0100 != 0;
		let cl1 = byte & 0b0000_0010 != 0;
		let gc = byte & 0b0000_0001 != 0;
		Oci { cl3, cl2, cl1, gc }
	}
}

/// Select/execute command
#[derive(Debug, Clone, Eq, PartialEq, Default)]
#[repr(u8)]
pub enum SelectExecute {
	/// Execute
	Execute = 0,
	#[default]
	/// Select
	Select = 1,
}

impl SelectExecute {
	pub const fn from_bool(select: bool) -> Self {
		match select {
			true => SelectExecute::Select,
			false => SelectExecute::Execute,
		}
	}
}

/// Local parameter change
#[derive(Debug, Clone, Eq, PartialEq, Default)]
#[repr(u8)]
pub enum Lpc {
	#[default]
	/// No change
	NoChange = 0,
	/// Changed
	Changed = 1,
}

impl Lpc {
	pub const fn from_bool(state: bool) -> Self {
		match state {
			true => Lpc::Changed,
			false => Lpc::NoChange,
		}
	}
}

/// Cause of initialization
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub enum Coi {
	#[default]
	/// Local power on
	LocalPowerOn,
	/// Local manual reset
	LocalManualReset,
	/// Remote reset
	RemoteReset,
	/// Other (custom)
	Other(u8),
}

impl Coi {
	pub const fn from_byte(byte: u8) -> Self {
		match byte {
			0 => Coi::LocalPowerOn,
			1 => Coi::LocalManualReset,
			2 => Coi::RemoteReset,
			_ => Coi::Other(byte),
		}
	}
}
