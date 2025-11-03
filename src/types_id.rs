#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms, non_camel_case_types)]
pub enum TypeId {
	/// Single-point information
	M_SP_NA_1 = 1,
	/// Single-point information with time tag
	M_SP_TA_1 = 2,
	/// Double-point information
	M_DP_NA_1 = 3,
	/// Double-point information with time tag
	M_DP_TA_1 = 4,
	/// Step position information
	M_ST_NA_1 = 5,
	/// Step position information with time tag
	M_ST_TA_1 = 6,
	/// Bitstring of 32 bit
	M_BO_NA_1 = 7,
	/// Measured value, normalized value
	M_ME_NA_1 = 9,
	/// Measured value, normalized value with time tag
	M_ME_TA_1 = 10,
	/// Measured value, scaled value
	M_ME_NB_1 = 11,
	/// Measured value, scaled value with time tag
	M_ME_TB_1 = 12,
	/// Measured value, short floating point number
	M_ME_NC_1 = 13,
	/// Measured value, short floating point number with time tag
	M_ME_TC_1 = 14,
	/// Integrated totals
	M_IT_NA_1 = 15,
	/// Event of protection equipment with time tag
	M_EP_TA_1 = 17,
	/// Packed start events of protection equipment with time tag
	M_EP_TB_1 = 18,
	/// Packed output circuit information of protection equipment with time tag
	M_EP_TC_1 = 19,
	/// Packed single point information with status change detection
	M_PS_NA_1 = 20,
	/// Measured value, normalized value without quality descriptor
	M_ME_ND_1 = 21,
	/// Single-point information with time tag
	M_SP_TB_1 = 30,
	/// Double-point information with time tag
	M_DP_TB_1 = 31,
	/// Step position information with time tag
	M_ST_TB_1 = 32,
	/// Bitstring of 32 bit with time tag
	M_BO_TB_1 = 33,
	/// Measured value, normalized value with time tag
	M_ME_TD_1 = 34,
	/// Measured value, scaled value with time tag
	M_ME_TE_1 = 35,
	/// Measured value, short floating point number with time tag
	M_ME_TF_1 = 36,
	/// Integrated totals with time tag
	M_IT_TB_1 = 37,
	/// Event of protection equipment with time tag
	M_EP_TD_1 = 38,
	/// Packed start events of protection equipment with time tag
	M_EP_TE_1 = 39,
	/// Packed output circuit information of protection equipment with time tag
	M_EP_TF_1 = 40,
	/// Single command
	C_SC_NA_1 = 45,
	/// Double command
	C_DC_NA_1 = 46,
	/// Regulating step command
	C_RC_NA_1 = 47,
	/// Set-point Command, normalized value
	C_SE_NA_1 = 48,
	/// Set-point Command, scaled value
	C_SE_NB_1 = 49,
	/// Set-point Command, short floating point number
	C_SE_NC_1 = 50,
	/// Bitstring 32 bit command
	C_BO_NA_1 = 51,
	/// Single command with time tag
	C_SC_TA_1 = 58,
	/// Double command with time tag
	C_DC_TA_1 = 59,
	/// Regulating step command with time tag
	C_RC_TA_1 = 60,
	/// Measured value, normalized value command with time tag
	C_SE_TA_1 = 61,
	/// Measured value, scaled value command with time tag
	C_SE_TB_1 = 62,
	/// Measured value, short floating point number command with time tag
	C_SE_TC_1 = 63,
	/// Bitstring of 32 bit command with time tag
	C_BO_TA_1 = 64,
	/// End of Initialization
	M_EI_NA_1 = 70,
	/// Interrogation command
	C_IC_NA_1 = 100,
	/// Counter interrogation command
	C_CI_NA_1 = 101,
	/// Read Command
	C_RD_NA_1 = 102,
	/// Clock synchronisation command
	C_CS_NA_1 = 103,
	/// Test command
	C_TS_NA_1 = 104,
	/// Reset process command
	C_RP_NA_1 = 105,
	/// Delay acquisition command
	C_CD_NA_1 = 106,
	/// Test command with time tag
	C_TS_TA_1 = 107,
	/// Parameter of measured values, normalized value
	P_ME_NA_1 = 110,
	/// Parameter of measured values, scaled value
	P_ME_NB_1 = 111,
	/// Parameter of measured values, short floating point number
	P_ME_NC_1 = 112,
	/// Parameter activation
	P_AC_NA_1 = 113,
	/// File ready
	F_FR_NA_1 = 120,
	/// Section ready
	F_SR_NA_1 = 121,
	/// Call directory, select file, call file, call section
	F_SC_NA_1 = 122,
	/// Last section, last segment
	F_LS_NA_1 = 123,
	/// ACK file, ACK section
	F_FA_NA_1 = 124,
	/// Segment
	F_SG_NA_1 = 125,
	/// Directory
	F_DR_TA_1 = 126,

	ASDU_TYPEUNDEF = 0,
	ASDU_TYPE_8 = 8,
	ASDU_TYPE_16 = 16,
	ASDU_TYPE_22 = 22,
	ASDU_TYPE_23 = 23,
	ASDU_TYPE_24 = 24,
	ASDU_TYPE_25 = 25,
	ASDU_TYPE_26 = 26,
	ASDU_TYPE_27 = 27,
	ASDU_TYPE_28 = 28,
	ASDU_TYPE_29 = 29,
	ASDU_TYPE_41 = 41,
	ASDU_TYPE_42 = 42,
	ASDU_TYPE_43 = 43,
	ASDU_TYPE_44 = 44,
	ASDU_TYPE_52 = 52,
	ASDU_TYPE_53 = 53,
	ASDU_TYPE_54 = 54,
	ASDU_TYPE_55 = 55,
	ASDU_TYPE_56 = 56,
	ASDU_TYPE_57 = 57,
	ASDU_TYPE_65 = 65,
	ASDU_TYPE_66 = 66,
	ASDU_TYPE_67 = 67,
	ASDU_TYPE_68 = 68,
	ASDU_TYPE_69 = 69,
	ASDU_TYPE_71 = 71,
	ASDU_TYPE_72 = 72,
	ASDU_TYPE_73 = 73,
	ASDU_TYPE_74 = 74,
	ASDU_TYPE_75 = 75,
	ASDU_TYPE_76 = 76,
	ASDU_TYPE_77 = 77,
	ASDU_TYPE_78 = 78,
	ASDU_TYPE_79 = 79,
	ASDU_TYPE_80 = 80,
	ASDU_TYPE_81 = 81,
	ASDU_TYPE_82 = 82,
	ASDU_TYPE_83 = 83,
	ASDU_TYPE_84 = 84,
	ASDU_TYPE_85 = 85,
	ASDU_TYPE_86 = 86,
	ASDU_TYPE_87 = 87,
	ASDU_TYPE_88 = 88,
	ASDU_TYPE_89 = 89,
	ASDU_TYPE_90 = 90,
	ASDU_TYPE_91 = 91,
	ASDU_TYPE_92 = 92,
	ASDU_TYPE_93 = 93,
	ASDU_TYPE_94 = 94,
	ASDU_TYPE_95 = 95,
	ASDU_TYPE_96 = 96,
	ASDU_TYPE_97 = 97,
	ASDU_TYPE_98 = 98,
	ASDU_TYPE_99 = 99,
	ASDU_TYPE_108 = 108,
	ASDU_TYPE_109 = 109,
	ASDU_TYPE_114 = 114,
	ASDU_TYPE_115 = 115,
	ASDU_TYPE_116 = 116,
	ASDU_TYPE_117 = 117,
	ASDU_TYPE_118 = 118,
	ASDU_TYPE_119 = 119,
	ASDU_TYPE_127 = 127,
	ASDU_TYPE_128 = 128,
	ASDU_TYPE_129 = 129,
	ASDU_TYPE_130 = 130,
	ASDU_TYPE_131 = 131,
	ASDU_TYPE_132 = 132,
	ASDU_TYPE_133 = 133,
	ASDU_TYPE_134 = 134,
	ASDU_TYPE_135 = 135,
	ASDU_TYPE_136 = 136,
	ASDU_TYPE_137 = 137,
	ASDU_TYPE_138 = 138,
	ASDU_TYPE_139 = 139,
	ASDU_TYPE_140 = 140,
	ASDU_TYPE_141 = 141,
	ASDU_TYPE_142 = 142,
	ASDU_TYPE_143 = 143,
	ASDU_TYPE_144 = 144,
	ASDU_TYPE_145 = 145,
	ASDU_TYPE_146 = 146,
	ASDU_TYPE_147 = 147,
	ASDU_TYPE_148 = 148,
	ASDU_TYPE_149 = 149,
	ASDU_TYPE_150 = 150,
	ASDU_TYPE_151 = 151,
	ASDU_TYPE_152 = 152,
	ASDU_TYPE_153 = 153,
	ASDU_TYPE_154 = 154,
	ASDU_TYPE_155 = 155,
	ASDU_TYPE_156 = 156,
	ASDU_TYPE_157 = 157,
	ASDU_TYPE_158 = 158,
	ASDU_TYPE_159 = 159,
	ASDU_TYPE_160 = 160,
	ASDU_TYPE_161 = 161,
	ASDU_TYPE_162 = 162,
	ASDU_TYPE_163 = 163,
	ASDU_TYPE_164 = 164,
	ASDU_TYPE_165 = 165,
	ASDU_TYPE_166 = 166,
	ASDU_TYPE_167 = 167,
	ASDU_TYPE_168 = 168,
	ASDU_TYPE_169 = 169,
	ASDU_TYPE_170 = 170,
	ASDU_TYPE_171 = 171,
	ASDU_TYPE_172 = 172,
	ASDU_TYPE_173 = 173,
	ASDU_TYPE_174 = 174,
	ASDU_TYPE_175 = 175,
	ASDU_TYPE_176 = 176,
	ASDU_TYPE_177 = 177,
	ASDU_TYPE_178 = 178,
	ASDU_TYPE_179 = 179,
	ASDU_TYPE_180 = 180,
	ASDU_TYPE_181 = 181,
	ASDU_TYPE_182 = 182,
	ASDU_TYPE_183 = 183,
	ASDU_TYPE_184 = 184,
	ASDU_TYPE_185 = 185,
	ASDU_TYPE_186 = 186,
	ASDU_TYPE_187 = 187,
	ASDU_TYPE_188 = 188,
	ASDU_TYPE_189 = 189,
	ASDU_TYPE_190 = 190,
	ASDU_TYPE_191 = 191,
	ASDU_TYPE_192 = 192,
	ASDU_TYPE_193 = 193,
	ASDU_TYPE_194 = 194,
	ASDU_TYPE_195 = 195,
	ASDU_TYPE_196 = 196,
	ASDU_TYPE_197 = 197,
	ASDU_TYPE_198 = 198,
	ASDU_TYPE_199 = 199,
	ASDU_TYPE_200 = 200,
	ASDU_TYPE_201 = 201,
	ASDU_TYPE_202 = 202,
	ASDU_TYPE_203 = 203,
	ASDU_TYPE_204 = 204,
	ASDU_TYPE_205 = 205,
	ASDU_TYPE_206 = 206,
	ASDU_TYPE_207 = 207,
	ASDU_TYPE_208 = 208,
	ASDU_TYPE_209 = 209,
	ASDU_TYPE_210 = 210,
	ASDU_TYPE_211 = 211,
	ASDU_TYPE_212 = 212,
	ASDU_TYPE_213 = 213,
	ASDU_TYPE_214 = 214,
	ASDU_TYPE_215 = 215,
	ASDU_TYPE_216 = 216,
	ASDU_TYPE_217 = 217,
	ASDU_TYPE_218 = 218,
	ASDU_TYPE_219 = 219,
	ASDU_TYPE_220 = 220,
	ASDU_TYPE_221 = 221,
	ASDU_TYPE_222 = 222,
	ASDU_TYPE_223 = 223,
	ASDU_TYPE_224 = 224,
	ASDU_TYPE_225 = 225,
	ASDU_TYPE_226 = 226,
	ASDU_TYPE_227 = 227,
	ASDU_TYPE_228 = 228,
	ASDU_TYPE_229 = 229,
	ASDU_TYPE_230 = 230,
	ASDU_TYPE_231 = 231,
	ASDU_TYPE_232 = 232,
	ASDU_TYPE_233 = 233,
	ASDU_TYPE_234 = 234,
	ASDU_TYPE_235 = 235,
	ASDU_TYPE_236 = 236,
	ASDU_TYPE_237 = 237,
	ASDU_TYPE_238 = 238,
	ASDU_TYPE_239 = 239,
	ASDU_TYPE_240 = 240,
	ASDU_TYPE_241 = 241,
	ASDU_TYPE_242 = 242,
	ASDU_TYPE_243 = 243,
	ASDU_TYPE_244 = 244,
	ASDU_TYPE_245 = 245,
	ASDU_TYPE_246 = 246,
	ASDU_TYPE_247 = 247,
	ASDU_TYPE_248 = 248,
	ASDU_TYPE_249 = 249,
	ASDU_TYPE_250 = 250,
	ASDU_TYPE_251 = 251,
	ASDU_TYPE_252 = 252,
	ASDU_TYPE_253 = 253,
	ASDU_TYPE_254 = 254,
	ASDU_TYPE_255 = 255,
}

impl From<u8> for TypeId {
	fn from(value: u8) -> Self {
		match value {
			1 => TypeId::M_SP_NA_1,
			2 => TypeId::M_SP_TA_1,
			3 => TypeId::M_DP_NA_1,
			4 => TypeId::M_DP_TA_1,
			5 => TypeId::M_ST_NA_1,
			6 => TypeId::M_ST_TA_1,
			7 => TypeId::M_BO_NA_1,
			9 => TypeId::M_ME_NA_1,
			10 => TypeId::M_ME_TA_1,
			11 => TypeId::M_ME_NB_1,
			12 => TypeId::M_ME_TB_1,
			13 => TypeId::M_ME_NC_1,
			14 => TypeId::M_ME_TC_1,
			15 => TypeId::M_IT_NA_1,
			17 => TypeId::M_EP_TA_1,
			18 => TypeId::M_EP_TB_1,
			19 => TypeId::M_EP_TC_1,
			20 => TypeId::M_PS_NA_1,
			21 => TypeId::M_ME_ND_1,
			30 => TypeId::M_SP_TB_1,
			31 => TypeId::M_DP_TB_1,
			32 => TypeId::M_ST_TB_1,
			33 => TypeId::M_BO_TB_1,
			34 => TypeId::M_ME_TD_1,
			35 => TypeId::M_ME_TE_1,
			36 => TypeId::M_ME_TF_1,
			37 => TypeId::M_IT_TB_1,
			38 => TypeId::M_EP_TD_1,
			39 => TypeId::M_EP_TE_1,
			40 => TypeId::M_EP_TF_1,
			45 => TypeId::C_SC_NA_1,
			46 => TypeId::C_DC_NA_1,
			47 => TypeId::C_RC_NA_1,
			48 => TypeId::C_SE_NA_1,
			49 => TypeId::C_SE_NB_1,
			50 => TypeId::C_SE_NC_1,
			51 => TypeId::C_BO_NA_1,
			58 => TypeId::C_SC_TA_1,
			59 => TypeId::C_DC_TA_1,
			60 => TypeId::C_RC_TA_1,
			61 => TypeId::C_SE_TA_1,
			62 => TypeId::C_SE_TB_1,
			63 => TypeId::C_SE_TC_1,
			64 => TypeId::C_BO_TA_1,
			70 => TypeId::M_EI_NA_1,
			100 => TypeId::C_IC_NA_1,
			101 => TypeId::C_CI_NA_1,
			102 => TypeId::C_RD_NA_1,
			103 => TypeId::C_CS_NA_1,
			104 => TypeId::C_TS_NA_1,
			105 => TypeId::C_RP_NA_1,
			106 => TypeId::C_CD_NA_1,
			107 => TypeId::C_TS_TA_1,
			110 => TypeId::P_ME_NA_1,
			111 => TypeId::P_ME_NB_1,
			112 => TypeId::P_ME_NC_1,
			113 => TypeId::P_AC_NA_1,
			120 => TypeId::F_FR_NA_1,
			121 => TypeId::F_SR_NA_1,
			122 => TypeId::F_SC_NA_1,
			123 => TypeId::F_LS_NA_1,
			124 => TypeId::F_FA_NA_1,
			125 => TypeId::F_SG_NA_1,
			126 => TypeId::F_DR_TA_1,
			0 => TypeId::ASDU_TYPEUNDEF,
			8 => TypeId::ASDU_TYPE_8,
			16 => TypeId::ASDU_TYPE_16,
			22 => TypeId::ASDU_TYPE_22,
			23 => TypeId::ASDU_TYPE_23,
			24 => TypeId::ASDU_TYPE_24,
			25 => TypeId::ASDU_TYPE_25,
			26 => TypeId::ASDU_TYPE_26,
			27 => TypeId::ASDU_TYPE_27,
			28 => TypeId::ASDU_TYPE_28,
			29 => TypeId::ASDU_TYPE_29,
			41 => TypeId::ASDU_TYPE_41,
			42 => TypeId::ASDU_TYPE_42,
			43 => TypeId::ASDU_TYPE_43,
			44 => TypeId::ASDU_TYPE_44,
			52 => TypeId::ASDU_TYPE_52,
			53 => TypeId::ASDU_TYPE_53,
			54 => TypeId::ASDU_TYPE_54,
			55 => TypeId::ASDU_TYPE_55,
			56 => TypeId::ASDU_TYPE_56,
			57 => TypeId::ASDU_TYPE_57,
			65 => TypeId::ASDU_TYPE_65,
			66 => TypeId::ASDU_TYPE_66,
			67 => TypeId::ASDU_TYPE_67,
			68 => TypeId::ASDU_TYPE_68,
			69 => TypeId::ASDU_TYPE_69,
			71 => TypeId::ASDU_TYPE_71,
			72 => TypeId::ASDU_TYPE_72,
			73 => TypeId::ASDU_TYPE_73,
			74 => TypeId::ASDU_TYPE_74,
			75 => TypeId::ASDU_TYPE_75,
			76 => TypeId::ASDU_TYPE_76,
			77 => TypeId::ASDU_TYPE_77,
			78 => TypeId::ASDU_TYPE_78,
			79 => TypeId::ASDU_TYPE_79,
			80 => TypeId::ASDU_TYPE_80,
			81 => TypeId::ASDU_TYPE_81,
			82 => TypeId::ASDU_TYPE_82,
			83 => TypeId::ASDU_TYPE_83,
			84 => TypeId::ASDU_TYPE_84,
			85 => TypeId::ASDU_TYPE_85,
			86 => TypeId::ASDU_TYPE_86,
			87 => TypeId::ASDU_TYPE_87,
			88 => TypeId::ASDU_TYPE_88,
			89 => TypeId::ASDU_TYPE_89,
			90 => TypeId::ASDU_TYPE_90,
			91 => TypeId::ASDU_TYPE_91,
			92 => TypeId::ASDU_TYPE_92,
			93 => TypeId::ASDU_TYPE_93,
			94 => TypeId::ASDU_TYPE_94,
			95 => TypeId::ASDU_TYPE_95,
			96 => TypeId::ASDU_TYPE_96,
			97 => TypeId::ASDU_TYPE_97,
			98 => TypeId::ASDU_TYPE_98,
			99 => TypeId::ASDU_TYPE_99,
			108 => TypeId::ASDU_TYPE_108,
			109 => TypeId::ASDU_TYPE_109,
			114 => TypeId::ASDU_TYPE_114,
			115 => TypeId::ASDU_TYPE_115,
			116 => TypeId::ASDU_TYPE_116,
			117 => TypeId::ASDU_TYPE_117,
			118 => TypeId::ASDU_TYPE_118,
			119 => TypeId::ASDU_TYPE_119,
			127 => TypeId::ASDU_TYPE_127,
			128 => TypeId::ASDU_TYPE_128,
			129 => TypeId::ASDU_TYPE_129,
			130 => TypeId::ASDU_TYPE_130,
			131 => TypeId::ASDU_TYPE_131,
			132 => TypeId::ASDU_TYPE_132,
			133 => TypeId::ASDU_TYPE_133,
			134 => TypeId::ASDU_TYPE_134,
			135 => TypeId::ASDU_TYPE_135,
			136 => TypeId::ASDU_TYPE_136,
			137 => TypeId::ASDU_TYPE_137,
			138 => TypeId::ASDU_TYPE_138,
			139 => TypeId::ASDU_TYPE_139,
			140 => TypeId::ASDU_TYPE_140,
			141 => TypeId::ASDU_TYPE_141,
			142 => TypeId::ASDU_TYPE_142,
			143 => TypeId::ASDU_TYPE_143,
			144 => TypeId::ASDU_TYPE_144,
			145 => TypeId::ASDU_TYPE_145,
			146 => TypeId::ASDU_TYPE_146,
			147 => TypeId::ASDU_TYPE_147,
			148 => TypeId::ASDU_TYPE_148,
			149 => TypeId::ASDU_TYPE_149,
			150 => TypeId::ASDU_TYPE_150,
			151 => TypeId::ASDU_TYPE_151,
			152 => TypeId::ASDU_TYPE_152,
			153 => TypeId::ASDU_TYPE_153,
			154 => TypeId::ASDU_TYPE_154,
			155 => TypeId::ASDU_TYPE_155,
			156 => TypeId::ASDU_TYPE_156,
			157 => TypeId::ASDU_TYPE_157,
			158 => TypeId::ASDU_TYPE_158,
			159 => TypeId::ASDU_TYPE_159,
			160 => TypeId::ASDU_TYPE_160,
			161 => TypeId::ASDU_TYPE_161,
			162 => TypeId::ASDU_TYPE_162,
			163 => TypeId::ASDU_TYPE_163,
			164 => TypeId::ASDU_TYPE_164,
			165 => TypeId::ASDU_TYPE_165,
			166 => TypeId::ASDU_TYPE_166,
			167 => TypeId::ASDU_TYPE_167,
			168 => TypeId::ASDU_TYPE_168,
			169 => TypeId::ASDU_TYPE_169,
			170 => TypeId::ASDU_TYPE_170,
			171 => TypeId::ASDU_TYPE_171,
			172 => TypeId::ASDU_TYPE_172,
			173 => TypeId::ASDU_TYPE_173,
			174 => TypeId::ASDU_TYPE_174,
			175 => TypeId::ASDU_TYPE_175,
			176 => TypeId::ASDU_TYPE_176,
			177 => TypeId::ASDU_TYPE_177,
			178 => TypeId::ASDU_TYPE_178,
			179 => TypeId::ASDU_TYPE_179,
			180 => TypeId::ASDU_TYPE_180,
			181 => TypeId::ASDU_TYPE_181,
			182 => TypeId::ASDU_TYPE_182,
			183 => TypeId::ASDU_TYPE_183,
			184 => TypeId::ASDU_TYPE_184,
			185 => TypeId::ASDU_TYPE_185,
			186 => TypeId::ASDU_TYPE_186,
			187 => TypeId::ASDU_TYPE_187,
			188 => TypeId::ASDU_TYPE_188,
			189 => TypeId::ASDU_TYPE_189,
			190 => TypeId::ASDU_TYPE_190,
			191 => TypeId::ASDU_TYPE_191,
			192 => TypeId::ASDU_TYPE_192,
			193 => TypeId::ASDU_TYPE_193,
			194 => TypeId::ASDU_TYPE_194,
			195 => TypeId::ASDU_TYPE_195,
			196 => TypeId::ASDU_TYPE_196,
			197 => TypeId::ASDU_TYPE_197,
			198 => TypeId::ASDU_TYPE_198,
			199 => TypeId::ASDU_TYPE_199,
			200 => TypeId::ASDU_TYPE_200,
			201 => TypeId::ASDU_TYPE_201,
			202 => TypeId::ASDU_TYPE_202,
			203 => TypeId::ASDU_TYPE_203,
			204 => TypeId::ASDU_TYPE_204,
			205 => TypeId::ASDU_TYPE_205,
			206 => TypeId::ASDU_TYPE_206,
			207 => TypeId::ASDU_TYPE_207,
			208 => TypeId::ASDU_TYPE_208,
			209 => TypeId::ASDU_TYPE_209,
			210 => TypeId::ASDU_TYPE_210,
			211 => TypeId::ASDU_TYPE_211,
			212 => TypeId::ASDU_TYPE_212,
			213 => TypeId::ASDU_TYPE_213,
			214 => TypeId::ASDU_TYPE_214,
			215 => TypeId::ASDU_TYPE_215,
			216 => TypeId::ASDU_TYPE_216,
			217 => TypeId::ASDU_TYPE_217,
			218 => TypeId::ASDU_TYPE_218,
			219 => TypeId::ASDU_TYPE_219,
			220 => TypeId::ASDU_TYPE_220,
			221 => TypeId::ASDU_TYPE_221,
			222 => TypeId::ASDU_TYPE_222,
			223 => TypeId::ASDU_TYPE_223,
			224 => TypeId::ASDU_TYPE_224,
			225 => TypeId::ASDU_TYPE_225,
			226 => TypeId::ASDU_TYPE_226,
			227 => TypeId::ASDU_TYPE_227,
			228 => TypeId::ASDU_TYPE_228,
			229 => TypeId::ASDU_TYPE_229,
			230 => TypeId::ASDU_TYPE_230,
			231 => TypeId::ASDU_TYPE_231,
			232 => TypeId::ASDU_TYPE_232,
			233 => TypeId::ASDU_TYPE_233,
			234 => TypeId::ASDU_TYPE_234,
			235 => TypeId::ASDU_TYPE_235,
			236 => TypeId::ASDU_TYPE_236,
			237 => TypeId::ASDU_TYPE_237,
			238 => TypeId::ASDU_TYPE_238,
			239 => TypeId::ASDU_TYPE_239,
			240 => TypeId::ASDU_TYPE_240,
			241 => TypeId::ASDU_TYPE_241,
			242 => TypeId::ASDU_TYPE_242,
			243 => TypeId::ASDU_TYPE_243,
			244 => TypeId::ASDU_TYPE_244,
			245 => TypeId::ASDU_TYPE_245,
			246 => TypeId::ASDU_TYPE_246,
			247 => TypeId::ASDU_TYPE_247,
			248 => TypeId::ASDU_TYPE_248,
			249 => TypeId::ASDU_TYPE_249,
			250 => TypeId::ASDU_TYPE_250,
			251 => TypeId::ASDU_TYPE_251,
			252 => TypeId::ASDU_TYPE_252,
			253 => TypeId::ASDU_TYPE_253,
			254 => TypeId::ASDU_TYPE_254,
			255 => TypeId::ASDU_TYPE_255,
		}
	}
}

impl TypeId {
	#[must_use]
	pub const fn size(self) -> usize {
		match self {
			TypeId::M_SP_NA_1 => 1,
			TypeId::M_SP_TA_1 => 4,
			TypeId::M_DP_NA_1 => 1,
			TypeId::M_DP_TA_1 => 4,
			TypeId::M_ST_NA_1 => 2,
			TypeId::M_ST_TA_1 => 5,
			TypeId::M_BO_NA_1 => 5,
			TypeId::M_ME_NA_1 => 3,
			TypeId::M_ME_TA_1 => 6,
			TypeId::M_ME_NB_1 => 3,
			TypeId::M_ME_TB_1 => 6,
			TypeId::M_ME_NC_1 => 5,
			TypeId::M_ME_TC_1 => 8,
			TypeId::M_IT_NA_1 => 5,
			TypeId::M_EP_TA_1 => 6,
			TypeId::M_EP_TB_1 => 7,
			TypeId::M_EP_TC_1 => 7,
			TypeId::M_PS_NA_1 => 5,
			TypeId::M_ME_ND_1 => 2,
			TypeId::M_SP_TB_1 => 8,
			TypeId::M_DP_TB_1 => 8,
			TypeId::M_ST_TB_1 => 9,
			TypeId::M_BO_TB_1 => 12,
			TypeId::M_ME_TD_1 => 10,
			TypeId::M_ME_TE_1 => 10,
			TypeId::M_ME_TF_1 => 12,
			TypeId::M_IT_TB_1 => 12,
			TypeId::M_EP_TD_1 => 10,
			TypeId::M_EP_TE_1 => 11,
			TypeId::M_EP_TF_1 => 11,
			TypeId::C_SC_NA_1 => 1,
			TypeId::C_DC_NA_1 => 1,
			TypeId::C_RC_NA_1 => 1,
			TypeId::C_SE_NA_1 => 3,
			TypeId::C_SE_NB_1 => 3,
			TypeId::C_SE_NC_1 => 5,
			TypeId::C_BO_NA_1 => 4,
			TypeId::C_SC_TA_1 => 8,
			TypeId::C_DC_TA_1 => 8,
			TypeId::C_RC_TA_1 => 8,
			TypeId::C_SE_TA_1 => 10,
			TypeId::C_SE_TB_1 => 10,
			TypeId::C_SE_TC_1 => 12,
			TypeId::C_BO_TA_1 => 11,
			TypeId::M_EI_NA_1 => 1,
			TypeId::C_IC_NA_1 => 1,
			TypeId::C_CI_NA_1 => 1,
			TypeId::C_RD_NA_1 => 0,
			TypeId::C_CS_NA_1 => 7,
			TypeId::C_TS_NA_1 => 2,
			TypeId::C_RP_NA_1 => 1,
			TypeId::C_CD_NA_1 => 2,
			TypeId::C_TS_TA_1 => 9,
			TypeId::P_ME_NA_1 => 3,
			TypeId::P_ME_NB_1 => 3,
			TypeId::P_ME_NC_1 => 5,
			TypeId::P_AC_NA_1 => 1,
			//TODO: Check if these are correct
			TypeId::F_FR_NA_1 => 6,
			TypeId::F_SR_NA_1 => 6,
			TypeId::F_SC_NA_1 => 6,
			TypeId::F_LS_NA_1 => 6,
			TypeId::F_FA_NA_1 => 6,
			TypeId::F_SG_NA_1 => 6,
			TypeId::F_DR_TA_1 => 6,
			_ => 0,
		}
	}
	/// Returns `true` if the type is standard, returns `false` if it is custom.
	#[must_use]
	pub const fn is_standard(self) -> bool {
		match self {
			TypeId::M_SP_NA_1
			| TypeId::M_SP_TA_1
			| TypeId::M_DP_NA_1
			| TypeId::M_DP_TA_1
			| TypeId::M_ST_NA_1
			| TypeId::M_ST_TA_1
			| TypeId::M_BO_NA_1
			| TypeId::M_ME_NA_1
			| TypeId::M_ME_TA_1
			| TypeId::M_ME_NB_1
			| TypeId::M_ME_TB_1
			| TypeId::M_ME_NC_1
			| TypeId::M_ME_TC_1
			| TypeId::M_IT_NA_1
			| TypeId::M_EP_TA_1
			| TypeId::M_EP_TB_1
			| TypeId::M_EP_TC_1
			| TypeId::M_PS_NA_1
			| TypeId::M_ME_ND_1
			| TypeId::M_SP_TB_1
			| TypeId::M_DP_TB_1
			| TypeId::M_ST_TB_1
			| TypeId::M_BO_TB_1
			| TypeId::M_ME_TD_1
			| TypeId::M_ME_TE_1
			| TypeId::M_ME_TF_1
			| TypeId::M_IT_TB_1
			| TypeId::M_EP_TD_1
			| TypeId::M_EP_TE_1
			| TypeId::M_EP_TF_1
			| TypeId::C_SC_NA_1
			| TypeId::C_DC_NA_1
			| TypeId::C_RC_NA_1
			| TypeId::C_SE_NA_1
			| TypeId::C_SE_NB_1
			| TypeId::C_SE_NC_1
			| TypeId::C_BO_NA_1
			| TypeId::C_SC_TA_1
			| TypeId::C_DC_TA_1
			| TypeId::C_RC_TA_1
			| TypeId::C_SE_TA_1
			| TypeId::C_SE_TB_1
			| TypeId::C_SE_TC_1
			| TypeId::C_BO_TA_1
			| TypeId::M_EI_NA_1
			| TypeId::C_IC_NA_1
			| TypeId::C_CI_NA_1
			| TypeId::C_RD_NA_1
			| TypeId::C_CS_NA_1
			| TypeId::C_TS_NA_1
			| TypeId::C_RP_NA_1
			| TypeId::C_CD_NA_1
			| TypeId::C_TS_TA_1
			| TypeId::P_ME_NA_1
			| TypeId::P_ME_NB_1
			| TypeId::P_ME_NC_1
			| TypeId::P_AC_NA_1
			| TypeId::F_FR_NA_1
			| TypeId::F_SR_NA_1
			| TypeId::F_SC_NA_1
			| TypeId::F_LS_NA_1
			| TypeId::F_FA_NA_1
			| TypeId::F_SG_NA_1
			| TypeId::F_DR_TA_1 => true,
			_ => false,
		}
	}
}
