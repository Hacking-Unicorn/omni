//! Omni ISA — complete 16-bit token partition, 128-opcode specification table,
//! balanced ternary algebra, and Pack5 radix-3 codec.

// 1. PURE NON-HUMAN 16-BIT TOKEN PARTITION CONSTANTS (0x0000 .. 0x7FFF)
// ==============================================================================

pub const VOCAB_SIZE: usize = 32_768;
pub const ENGLISH_MAX_TOKEN: u16 = 0x7DFF;

pub const REG_BASE: u16 = 0x7E00; // 0x7E00 .. 0x7EBF (192 SSA registers)
pub const REG_MAX: u16  = 0x7EBF;
pub const PAR_BASE: u16 = 0x7EC0; // 0x7EC0 .. 0x7EDF (32 function parameters)
pub const PAR_MAX: u16  = 0x7EDF;

pub const TOK_MOD_BEGIN: u16  = 0x7EE0;
pub const TOK_SCHEMA_DEF: u16 = 0x7EE1;
pub const TOK_EXPERT_DEF: u16 = 0x7EE2;
pub const TOK_FN_DEF: u16     = 0x7EE3;
pub const TOK_INS_END: u16    = 0x7EE4;
pub const TOK_IMM_I16: u16    = 0x7EE5;
pub const TOK_IMM_I32: u16    = 0x7EE6;
pub const TOK_TRIT_NEG: u16   = 0x7EE7;
pub const TOK_TRIT_ZERO: u16  = 0x7EE8;
pub const TOK_TRIT_POS: u16   = 0x7EE9;
pub const TOK_ENG_SPAN: u16   = 0x7EEA;
pub const TOK_SYM_SCHEMA: u16 = 0x7EEB;
pub const TOK_SYM_FN: u16     = 0x7EEC;
pub const TOK_SYM_SKILL: u16  = 0x7EED;
pub const TOK_FIELD_KV: u16   = 0x7EEE;
pub const TOK_MOD_END: u16    = 0x7EEF;

pub const LBL_BASE: u16 = 0x7EF0;
pub const LBL_MAX: u16  = 0x7EFF;

pub const OP_BASE: u16 = 0x7F00;
pub const OP_MAX: u16  = 0x7F7F;

pub const TY_VOID: u16     = 0x7F80;
pub const TY_I64: u16      = 0x7F81;
pub const TY_F64: u16      = 0x7F82;
pub const TY_TRIT: u16     = 0x7F83;
pub const TY_STR: u16      = 0x7F84;
pub const TY_UUID: u16     = 0x7F85;
pub const TY_BYTES: u16    = 0x7F86;
pub const TY_TRIT_VEC: u16 = 0x7F87;
pub const TY_FLT_VEC: u16  = 0x7F88;
pub const TY_ARRAY: u16    = 0x7F89;
pub const TY_ARENA: u16    = 0x7F8A;
pub const TY_MODEL_T: u16  = 0x7F8B;
pub const TY_DOM: u16      = 0x7F8C;
pub const TY_STRUCT: u16   = 0x7F8D;
pub const TY_ERR: u16      = 0x7F8E;

pub const TC_BASE: u16          = 0x7FC0;
pub const TC_UNTRUSTED_BIT: u16 = 0x0020;

pub type CapMask = u16;
pub const CAP_ID_PURE: u16   = 0;
pub const CAP_ID_DB_R: u16   = 1;
pub const CAP_ID_DB_W: u16   = 2;
pub const CAP_ID_NET: u16    = 3;
pub const CAP_ID_DOM: u16    = 4;
pub const CAP_ID_AI: u16     = 5;
pub const CAP_ID_VISION: u16 = 6;
pub const CAP_ID_AUDIO: u16  = 7;
pub const CAP_ID_AUTH: u16   = 8;
pub const CAP_ID_FS: u16     = 9;
pub const CAP_ID_SYS: u16    = 10;

#[inline(always)]
pub const fn cap_bit(cap_id: u16) -> CapMask {
    if cap_id == 0 { 0 } else { 1u16 << (cap_id - 1) }
}

#[inline(always)]
pub const fn pack_tc(untrusted: bool, cap_id: u16) -> u16 {
    TC_BASE | (if untrusted { TC_UNTRUSTED_BIT } else { 0 }) | (cap_id & 0x1F)
}

#[inline(always)]
pub const fn unpack_tc(tc_tok: u16) -> (bool, u16, CapMask) {
    let raw = tc_tok.saturating_sub(TC_BASE);
    let untrusted = (raw & TC_UNTRUSTED_BIT) != 0;
    let cap_id = raw & 0x1F;
    (untrusted, cap_id, cap_bit(cap_id))
}

// ==============================================================================
// 2. COMPLETE 128-OPCODE BINARY SPECIFICATION TABLE (0x7F00 .. 0x7F7F)
// ==============================================================================

#[derive(Clone, Copy, Debug)]
pub struct OpcodeSpec {
    pub token_id: u16,
    pub req_cap_mask: CapMask,
    pub sink_requires_safe: bool,
    pub is_sanitizer: bool,
    pub has_dest_reg: bool,
    pub debug_label: &'static str,
}

pub static OMNI_ISA_TABLE: [OpcodeSpec; 128] = [
    OpcodeSpec { token_id: 0x7F00, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: false, debug_label: "OP_7F00_NOP" },
    OpcodeSpec { token_id: 0x7F01, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F01_ARENA" },
    OpcodeSpec { token_id: 0x7F02, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F02_BORROW_R" },
    OpcodeSpec { token_id: 0x7F03, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F03_BORROW_W" },
    OpcodeSpec { token_id: 0x7F04, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F04_MOVE" },
    OpcodeSpec { token_id: 0x7F05, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: false, debug_label: "OP_7F05_DROP" },
    OpcodeSpec { token_id: 0x7F06, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F06_CINT" },
    OpcodeSpec { token_id: 0x7F07, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F07_CFLT" },
    OpcodeSpec { token_id: 0x7F08, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F08_CSTR" },
    OpcodeSpec { token_id: 0x7F09, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F09_CTRIT" },
    OpcodeSpec { token_id: 0x7F0A, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F0A_SNEW" },
    OpcodeSpec { token_id: 0x7F0B, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F0B_FGET" },
    OpcodeSpec { token_id: 0x7F0C, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F0C_FSET" },
    OpcodeSpec { token_id: 0x7F0D, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F0D_ANEW" },
    OpcodeSpec { token_id: 0x7F0E, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F0E_IGET" },
    OpcodeSpec { token_id: 0x7F0F, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F0F_ISET" },
    OpcodeSpec { token_id: 0x7F10, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F10_IADD" },
    OpcodeSpec { token_id: 0x7F11, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F11_ISUB" },
    OpcodeSpec { token_id: 0x7F12, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F12_IMUL" },
    OpcodeSpec { token_id: 0x7F13, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F13_IDIV" },
    OpcodeSpec { token_id: 0x7F14, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F14_IMOD" },
    OpcodeSpec { token_id: 0x7F15, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F15_INEG" },
    OpcodeSpec { token_id: 0x7F16, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F16_IABS" },
    OpcodeSpec { token_id: 0x7F17, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F17_IMIN" },
    OpcodeSpec { token_id: 0x7F18, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F18_IMAX" },
    OpcodeSpec { token_id: 0x7F19, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F19_FADD" },
    OpcodeSpec { token_id: 0x7F1A, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F1A_FSUB" },
    OpcodeSpec { token_id: 0x7F1B, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F1B_FMUL" },
    OpcodeSpec { token_id: 0x7F1C, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F1C_FDIV" },
    OpcodeSpec { token_id: 0x7F1D, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F1D_FSQRT" },
    OpcodeSpec { token_id: 0x7F1E, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F1E_BAND" },
    OpcodeSpec { token_id: 0x7F1F, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F1F_BOR" },
    OpcodeSpec { token_id: 0x7F20, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F20_BXOR" },
    OpcodeSpec { token_id: 0x7F21, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F21_BNOT" },
    OpcodeSpec { token_id: 0x7F22, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F22_SHL" },
    OpcodeSpec { token_id: 0x7F23, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F23_SHR" },
    OpcodeSpec { token_id: 0x7F24, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F24_CEQ" },
    OpcodeSpec { token_id: 0x7F25, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F25_CNE" },
    OpcodeSpec { token_id: 0x7F26, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F26_CLT" },
    OpcodeSpec { token_id: 0x7F27, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F27_CGT" },
    OpcodeSpec { token_id: 0x7F28, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F28_TNOT" },
    OpcodeSpec { token_id: 0x7F29, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F29_TAND" },
    OpcodeSpec { token_id: 0x7F2A, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F2A_TOR" },
    OpcodeSpec { token_id: 0x7F2B, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F2B_TXOR" },
    OpcodeSpec { token_id: 0x7F2C, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F2C_TMAJ" },
    OpcodeSpec { token_id: 0x7F2D, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F2D_TCMP" },
    OpcodeSpec { token_id: 0x7F2E, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F2E_TADD" },
    OpcodeSpec { token_id: 0x7F2F, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F2F_TMUL" },
    OpcodeSpec { token_id: 0x7F30, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F30_TPACK5" },
    OpcodeSpec { token_id: 0x7F31, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F31_TUNPACK5" },
    OpcodeSpec { token_id: 0x7F32, req_cap_mask: cap_bit(CAP_ID_AI),       sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F32_TQUANT" },
    OpcodeSpec { token_id: 0x7F33, req_cap_mask: cap_bit(CAP_ID_AI),       sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F33_TMATMUL" },
    OpcodeSpec { token_id: 0x7F34, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F34_TDOT" },
    OpcodeSpec { token_id: 0x7F35, req_cap_mask: cap_bit(CAP_ID_AI),       sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F35_EGATE" },
    OpcodeSpec { token_id: 0x7F36, req_cap_mask: cap_bit(CAP_ID_AI),       sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F36_LOOKAHEAD" },
    OpcodeSpec { token_id: 0x7F37, req_cap_mask: cap_bit(CAP_ID_AI),       sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F37_PREFETCH" },
    OpcodeSpec { token_id: 0x7F38, req_cap_mask: cap_bit(CAP_ID_AI),       sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F38_HMATMUL" },
    OpcodeSpec { token_id: 0x7F39, req_cap_mask: cap_bit(CAP_ID_AI),       sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F39_HOLO_EXEC" },
    OpcodeSpec { token_id: 0x7F3A, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F3A_TALL" },
    OpcodeSpec { token_id: 0x7F3B, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F3B_TANY" },
    OpcodeSpec { token_id: 0x7F3C, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F3C_TVNEW" },
    OpcodeSpec { token_id: 0x7F3D, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F3D_TVGET" },
    OpcodeSpec { token_id: 0x7F3E, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F3E_TVSET" },
    OpcodeSpec { token_id: 0x7F3F, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F3F_TLOGIC" },
    OpcodeSpec { token_id: 0x7F40, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: false, debug_label: "OP_7F40_LABEL" },
    OpcodeSpec { token_id: 0x7F41, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: false, debug_label: "OP_7F41_JMP" },
    OpcodeSpec { token_id: 0x7F42, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: false, debug_label: "OP_7F42_BR_TRIT" },
    OpcodeSpec { token_id: 0x7F43, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: false, debug_label: "OP_7F43_BR_POS" },
    OpcodeSpec { token_id: 0x7F44, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: false, debug_label: "OP_7F44_BR_NEG" },
    OpcodeSpec { token_id: 0x7F45, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F45_SEL_TRIT" },
    OpcodeSpec { token_id: 0x7F46, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: false, debug_label: "OP_7F46_LBOUND" },
    OpcodeSpec { token_id: 0x7F47, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: false, debug_label: "OP_7F47_LNEXT" },
    OpcodeSpec { token_id: 0x7F48, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F48_CALL" },
    OpcodeSpec { token_id: 0x7F49, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: false, debug_label: "OP_7F49_TCALL" },
    OpcodeSpec { token_id: 0x7F4A, req_cap_mask: 0,                        sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: false, debug_label: "OP_7F4A_RET" },
    OpcodeSpec { token_id: 0x7F4B, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F4B_TRY" },
    OpcodeSpec { token_id: 0x7F4C, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F4C_ERR" },
    OpcodeSpec { token_id: 0x7F4D, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F4D_IS_OK" },
    OpcodeSpec { token_id: 0x7F4E, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F4E_UNWRAP" },
    OpcodeSpec { token_id: 0x7F4F, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: false, debug_label: "OP_7F4F_ASSERT" },
    OpcodeSpec { token_id: 0x7F50, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F50_SLEN" },
    OpcodeSpec { token_id: 0x7F51, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F51_SCAT" },
    OpcodeSpec { token_id: 0x7F52, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F52_SSLICE" },
    OpcodeSpec { token_id: 0x7F53, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F53_SFIND" },
    OpcodeSpec { token_id: 0x7F54, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F54_SSPLIT" },
    OpcodeSpec { token_id: 0x7F55, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F55_S2I" },
    OpcodeSpec { token_id: 0x7F56, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F56_I2S" },
    OpcodeSpec { token_id: 0x7F57, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: true,  has_dest_reg: true,  debug_label: "OP_7F57_SAN_SQL" },
    OpcodeSpec { token_id: 0x7F58, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: true,  has_dest_reg: true,  debug_label: "OP_7F58_SAN_DOM" },
    OpcodeSpec { token_id: 0x7F59, req_cap_mask: cap_bit(CAP_ID_AI),       sink_requires_safe: false, is_sanitizer: true,  has_dest_reg: true,  debug_label: "OP_7F59_SAN_PRM" },
    OpcodeSpec { token_id: 0x7F5A, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: true,  has_dest_reg: true,  debug_label: "OP_7F5A_SAN_PTH" },
    OpcodeSpec { token_id: 0x7F5B, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: true,  has_dest_reg: true,  debug_label: "OP_7F5B_VAL_UUID" },
    OpcodeSpec { token_id: 0x7F5C, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: true,  has_dest_reg: true,  debug_label: "OP_7F5C_VAL_JSON" },
    OpcodeSpec { token_id: 0x7F5D, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F5D_FNV1A" },
    OpcodeSpec { token_id: 0x7F5E, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F5E_HEX_ENC" },
    OpcodeSpec { token_id: 0x7F5F, req_cap_mask: 0,                        sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F5F_HEX_DEC" },
    OpcodeSpec { token_id: 0x7F60, req_cap_mask: cap_bit(CAP_ID_DB_W),     sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F60_TX_BEG" },
    OpcodeSpec { token_id: 0x7F61, req_cap_mask: cap_bit(CAP_ID_DB_W),     sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F61_TX_COM" },
    OpcodeSpec { token_id: 0x7F62, req_cap_mask: cap_bit(CAP_ID_DB_W),     sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F62_TX_RBK" },
    OpcodeSpec { token_id: 0x7F63, req_cap_mask: cap_bit(CAP_ID_DB_W),     sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F63_DB_INS" },
    OpcodeSpec { token_id: 0x7F64, req_cap_mask: cap_bit(CAP_ID_DB_R),     sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F64_DB_GET" },
    OpcodeSpec { token_id: 0x7F65, req_cap_mask: cap_bit(CAP_ID_DB_R),     sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F65_DB_QRY" },
    OpcodeSpec { token_id: 0x7F66, req_cap_mask: cap_bit(CAP_ID_DB_W),     sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F66_DB_UPD" },
    OpcodeSpec { token_id: 0x7F67, req_cap_mask: cap_bit(CAP_ID_DB_W),     sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F67_DB_DEL" },
    OpcodeSpec { token_id: 0x7F68, req_cap_mask: cap_bit(CAP_ID_DB_R),     sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F68_DB_CNT" },
    OpcodeSpec { token_id: 0x7F69, req_cap_mask: cap_bit(CAP_ID_DB_W),     sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F69_TV_INS" },
    OpcodeSpec { token_id: 0x7F6A, req_cap_mask: cap_bit(CAP_ID_DB_R),     sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F6A_TV_SRCH" },
    OpcodeSpec { token_id: 0x7F6B, req_cap_mask: cap_bit(CAP_ID_DB_W),     sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F6B_KV_PUT" },
    OpcodeSpec { token_id: 0x7F6C, req_cap_mask: cap_bit(CAP_ID_NET),      sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F6C_ROUTE" },
    OpcodeSpec { token_id: 0x7F6D, req_cap_mask: cap_bit(CAP_ID_NET),      sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F6D_REQ_PAR" },
    OpcodeSpec { token_id: 0x7F6E, req_cap_mask: cap_bit(CAP_ID_NET),      sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F6E_RSP_JSON" },
    OpcodeSpec { token_id: 0x7F6F, req_cap_mask: cap_bit(CAP_ID_NET),      sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F6F_RSP_HTML" },
    OpcodeSpec { token_id: 0x7F70, req_cap_mask: cap_bit(CAP_ID_AUTH),     sink_requires_safe: false, is_sanitizer: true,  has_dest_reg: true,  debug_label: "OP_7F70_AUTH_VER" },
    OpcodeSpec { token_id: 0x7F71, req_cap_mask: cap_bit(CAP_ID_NET),      sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F71_RATELIM" },
    OpcodeSpec { token_id: 0x7F72, req_cap_mask: cap_bit(CAP_ID_SYS),      sink_requires_safe: false, is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F72_TIME_MS" },
    OpcodeSpec { token_id: 0x7F73, req_cap_mask: cap_bit(CAP_ID_SYS),      sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F73_LOG" },
    OpcodeSpec { token_id: 0x7F74, req_cap_mask: cap_bit(CAP_ID_SYS),      sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F74_ENV_GET" },
    OpcodeSpec { token_id: 0x7F75, req_cap_mask: cap_bit(CAP_ID_FS),       sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F75_FS_READ" },
    OpcodeSpec { token_id: 0x7F76, req_cap_mask: cap_bit(CAP_ID_DOM),      sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F76_DOM_NODE" },
    OpcodeSpec { token_id: 0x7F77, req_cap_mask: cap_bit(CAP_ID_DOM),      sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F77_DOM_ATTR" },
    OpcodeSpec { token_id: 0x7F78, req_cap_mask: cap_bit(CAP_ID_DOM),      sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F78_DOM_TEXT" },
    OpcodeSpec { token_id: 0x7F79, req_cap_mask: cap_bit(CAP_ID_DOM),      sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F79_DOM_EVT" },
    OpcodeSpec { token_id: 0x7F7A, req_cap_mask: cap_bit(CAP_ID_DOM),      sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F7A_DOM_MNT" },
    OpcodeSpec { token_id: 0x7F7B, req_cap_mask: cap_bit(CAP_ID_DOM),      sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F7B_WGPU" },
    OpcodeSpec { token_id: 0x7F7C, req_cap_mask: cap_bit(CAP_ID_AUDIO),    sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F7C_WAUDIO" },
    OpcodeSpec { token_id: 0x7F7D, req_cap_mask: cap_bit(CAP_ID_AI),       sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F7D_EMBED" },
    OpcodeSpec { token_id: 0x7F7E, req_cap_mask: cap_bit(CAP_ID_VISION),   sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F7E_SK_LOAD" },
    OpcodeSpec { token_id: 0x7F7F, req_cap_mask: cap_bit(CAP_ID_VISION),   sink_requires_safe: true,  is_sanitizer: false, has_dest_reg: true,  debug_label: "OP_7F7F_SK_EXEC" },
];

#[inline(always)]
pub fn decode_opcode_token(tok: u16) -> Option<&'static OpcodeSpec> {
    if (OP_BASE..=OP_MAX).contains(&tok) {
        Some(&OMNI_ISA_TABLE[(tok - OP_BASE) as usize])
    } else {
        None
    }
}

/// Explicit consistency check for standalone x86-64 ELF64 AOT lowering (Roadmap 2.4).
pub fn is_x86_64_aot_supported_opcode(op_tok: u16) -> bool {
    matches!(
        op_tok,
        0x7F00
            | 0x7F02..=0x7F04
            | 0x7F06
            | 0x7F09
            | 0x7F0D..=0x7F18
            | 0x7F1E..=0x7F2D
            | 0x7F40..=0x7F45
            | 0x7F4A
            | 0x7F50
            | 0x7F57..=0x7F59
    )
}

/// Explicit consistency check for standalone WebAssembly v1 AOT lowering (Roadmap 2.4).
pub fn is_wasm_aot_supported_opcode(op_tok: u16) -> bool {
    matches!(
        op_tok,
        0x7F00
            | 0x7F02..=0x7F04
            | 0x7F06
            | 0x7F09
            | 0x7F0D..=0x7F18
            | 0x7F1E..=0x7F2D
            | 0x7F40..=0x7F45
            | 0x7F4A
            | 0x7F50
            | 0x7F57..=0x7F59
    )
}

#[inline(always)]
pub fn tnot(a: i8) -> i8 { -a.clamp(-1, 1) }
#[inline(always)]
pub fn tand(a: i8, b: i8) -> i8 { a.clamp(-1, 1).min(b.clamp(-1, 1)) }
#[inline(always)]
pub fn tor(a: i8, b: i8) -> i8 { a.clamp(-1, 1).max(b.clamp(-1, 1)) }
#[inline(always)]
pub fn txor(a: i8, b: i8) -> i8 { -(a.clamp(-1, 1) * b.clamp(-1, 1)) }
#[inline(always)]
pub fn tmaj(a: i8, b: i8, c: i8) -> i8 {
    let s = (a.clamp(-1, 1) as i16) + (b.clamp(-1, 1) as i16) + (c.clamp(-1, 1) as i16);
    if s > 0 { 1 } else if s < 0 { -1 } else { 0 }
}

pub const PACK5_MAX_VALID: u8 = 242;
pub const SENTINEL_SPECIAL_TOKEN: u8 = 243;
pub const SENTINEL_SPARSE_RUN_ZERO: u8 = 244;
pub const SENTINEL_PARITY_CHECK: u8 = 245;
pub const SENTINEL_CRC_ANCHOR: u8 = 246;

pub const LUT_UNPACK5: [[i8; 5]; 243] = {
    let mut table = [[0i8; 5]; 243];
    let mut b: usize = 0;
    while b < 243 {
        let mut rem = b;
        let mut k = 0;
        while k < 5 {
            let u = (rem % 3) as i8;
            table[b][k] = u - 1;
            rem /= 3;
            k += 1;
        }
        b += 1;
    }
    table
};

#[inline]
pub fn pack5_encode(trits: &[i8]) -> Vec<u8> {
    let mut out = Vec::with_capacity((trits.len() + 4) / 5);
    for chunk in trits.chunks(5) {
        let mut acc: u8 = 0;
        let mut pow3: u8 = 1;
        for i in 0..5 {
            let t = if i < chunk.len() { chunk[i].clamp(-1, 1) } else { 0 };
            let u = (t + 1) as u8;
            acc += u * pow3;
            if i < 4 {
                pow3 *= 3;
            }
        }
        out.push(acc);
    }
    out
}

#[inline]
pub fn pack5_decode(bytes: &[u8], expected_trits: usize) -> Result<Vec<i8>, &'static str> {
    let mut out = Vec::with_capacity(expected_trits);
    for &b in bytes {
        if b > PACK5_MAX_VALID {
            return Err("PACK5_SENTINEL_OR_CORRUPT_BYTE");
        }
        let unpacked = LUT_UNPACK5[b as usize];
        for &v in &unpacked {
            if out.len() < expected_trits {
                out.push(v);
            }
        }
    }
    Ok(out)
}

// ==============================================================================
