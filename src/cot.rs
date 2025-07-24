use snafu::Snafu;

use crate::error::SpanTraceWrapper;

/// COT
///
/// COT is the Control Object Type. It is used to identify the type of control
/// object that is being sent.
///
/// COT is a 6-bit field that is used to identify the type of control object
/// that is being sent.
#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Cot {
	Unused = 0,
	Cyclic = 1,
	Background = 2,
	SpontaneousData = 3,
	Initiated = 4,
	Request = 5,
	Activation = 6,
	ActivationConfirmation = 7,
	Deactivation = 8,
	DeactivationConfirmation = 9,
	ActivationTermination = 10,
	ReturnRemote = 11,
	ReturnLocal = 12,
	File = 13,
	Reserved14 = 14,
	Reserved15 = 15,
	Reserved16 = 16,
	Reserved17 = 17,
	Reserved18 = 18,
	Reserved19 = 19,
	InterrogationGeneral = 20,
	InterrogationGroup1 = 21,
	InterrogationGroup2 = 22,
	InterrogationGroup3 = 23,
	InterrogationGroup4 = 24,
	InterrogationGroup5 = 25,
	InterrogationGroup6 = 26,
	InterrogationGroup7 = 27,
	InterrogationGroup8 = 28,
	InterrogationGroup9 = 29,
	InterrogationGroup10 = 30,
	InterrogationGroup11 = 31,
	InterrogationGroup12 = 32,
	InterrogationGroup13 = 33,
	InterrogationGroup14 = 34,
	InterrogationGroup15 = 35,
	InterrogationGroup16 = 36,
	CounterInterrogationGeneral = 37,
	CounterInterrogationGroup1 = 38,
	CounterInterrogationGroup2 = 39,
	CounterInterrogationGroup3 = 40,
	CounterInterrogationGroup4 = 41,
	Reserved42 = 42,
	Reserved43 = 43,
	UnknownType = 44,
	UnknownCause = 45,
	UnknownAsduAddress = 46,
	UnknownObjectAddress = 47,
	Reserved48 = 48,
	Reserved49 = 49,
	Reserved50 = 50,
	Reserved51 = 51,
	Reserved52 = 52,
	Reserved53 = 53,
	Reserved54 = 54,
	Reserved55 = 55,
	Reserved56 = 56,
	Reserved57 = 57,
	Reserved58 = 58,
	Reserved59 = 59,
	Reserved60 = 60,
	Reserved61 = 61,
	Reserved62 = 62,
	Reserved63 = 63,
}

impl TryFrom<u8> for Cot {
	type Error = CotError;

	fn try_from(value: u8) -> Result<Self, Self::Error> {
		match value {
			0 => Ok(Self::Unused),
			1 => Ok(Self::Cyclic),
			2 => Ok(Self::Background),
			3 => Ok(Self::SpontaneousData),
			4 => Ok(Self::Initiated),
			5 => Ok(Self::Request),
			6 => Ok(Self::Activation),
			7 => Ok(Self::ActivationConfirmation),
			8 => Ok(Self::Deactivation),
			9 => Ok(Self::DeactivationConfirmation),
			10 => Ok(Self::ActivationTermination),
			11 => Ok(Self::ReturnRemote),
			12 => Ok(Self::ReturnLocal),
			13 => Ok(Self::File),
			14 => Ok(Self::Reserved14),
			15 => Ok(Self::Reserved15),
			16 => Ok(Self::Reserved16),
			17 => Ok(Self::Reserved17),
			18 => Ok(Self::Reserved18),
			19 => Ok(Self::Reserved19),
			20 => Ok(Self::InterrogationGeneral),
			21 => Ok(Self::InterrogationGroup1),
			22 => Ok(Self::InterrogationGroup2),
			23 => Ok(Self::InterrogationGroup3),
			24 => Ok(Self::InterrogationGroup4),
			25 => Ok(Self::InterrogationGroup5),
			26 => Ok(Self::InterrogationGroup6),
			27 => Ok(Self::InterrogationGroup7),
			28 => Ok(Self::InterrogationGroup8),
			29 => Ok(Self::InterrogationGroup9),
			30 => Ok(Self::InterrogationGroup10),
			31 => Ok(Self::InterrogationGroup11),
			32 => Ok(Self::InterrogationGroup12),
			33 => Ok(Self::InterrogationGroup13),
			34 => Ok(Self::InterrogationGroup14),
			35 => Ok(Self::InterrogationGroup15),
			36 => Ok(Self::InterrogationGroup16),
			37 => Ok(Self::CounterInterrogationGeneral),
			38 => Ok(Self::CounterInterrogationGroup1),
			39 => Ok(Self::CounterInterrogationGroup2),
			40 => Ok(Self::CounterInterrogationGroup3),
			41 => Ok(Self::CounterInterrogationGroup4),
			42 => Ok(Self::Reserved42),
			43 => Ok(Self::Reserved43),
			44 => Ok(Self::UnknownType),
			45 => Ok(Self::UnknownCause),
			46 => Ok(Self::UnknownAsduAddress),
			47 => Ok(Self::UnknownObjectAddress),
			48 => Ok(Self::Reserved48),
			49 => Ok(Self::Reserved49),
			50 => Ok(Self::Reserved50),
			51 => Ok(Self::Reserved51),
			52 => Ok(Self::Reserved52),
			53 => Ok(Self::Reserved53),
			54 => Ok(Self::Reserved54),
			55 => Ok(Self::Reserved55),
			56 => Ok(Self::Reserved56),
			57 => Ok(Self::Reserved57),
			58 => Ok(Self::Reserved58),
			59 => Ok(Self::Reserved59),
			60 => Ok(Self::Reserved60),
			61 => Ok(Self::Reserved61),
			62 => Ok(Self::Reserved62),
			63 => Ok(Self::Reserved63),
			_ => InvalidCot { value }.fail(),
		}
	}
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub), context(suffix(false)))]
pub enum CotError {
	#[snafu(display("Invalid COT: {value}"))]
	InvalidCot {
		value: u8,
		#[snafu(implicit)]
		context: SpanTraceWrapper,
	},
}
