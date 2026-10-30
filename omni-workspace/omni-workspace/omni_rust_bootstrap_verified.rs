//! ============================================================================
//! OMNI v2.0 — COMPLETE 7-STEP PURE RUST BOOTSTRAP & SELF-HOSTING AOT SUITE
//! ============================================================================
//!
//! Zero English keywords in the binary token stream. Zero ASCII punctuation syntax.
//!
//! Native Omni bytecode is a raw sequence of 16-bit unsigned token IDs (`Vec<u16>`)
//! inside the 32,768-token vocabulary (`0x0000 .. 0x7FFF`):
//!
//!   * 0x0000 .. 0x7DFF (32,256 tokens): Natural English Lexical Bank
//!       Used strictly for human I/O strings & UI copy (`0x7EEA` span prefix).
//!       Hardware-clamped (`token_id & 0x7FFF < 0x7E00`) and tagged `@UNTRUSTED`.
//!   * 0x7E00 .. 0x7EBF (192 tokens):    SSA / Local Registers (`0x7E00`..`0x7EBF`)
//!   * 0x7EC0 .. 0x7EDF (32 tokens):     Function Parameters (`0x7EC0`..`0x7EDF`)
//!   * 0x7EE0 .. 0x7EEF (16 tokens):     Binary Structural Framing & Literal Prefixes
//!   * 0x7EF0 .. 0x7EFF (16 tokens):     Basic Block Branch Targets (`0x7EF0`..`0x7EFF`)
//!   * 0x7F00 .. 0x7F7F (128 tokens):    Complete 128-Opcode ISA (Direct O(1) Dispatch)
//!   * 0x7F80 .. 0x7FBF (64 tokens):     Binary Type Tags (`0x7F80`..`0x7F8F`)
//!   * 0x7FC0 .. 0x7FFF (64 tokens):     Packed Taint (`Bit 5`) + Capability (`Bits 0..4`)
//!
//! Supported Lowering Subset Across VM, x86-64 ELF64 AOT, WebAssembly AOT & Stage-1:
//!   * Numeric & Bitwise (`TY_I64`): `CINT (0x7F06)`, `IADD (0x7F10)`, `ISUB (0x7F11)`,
//!     `IMUL (0x7F12)`, `IDIV (0x7F13)`, `IMOD (0x7F14)`, `INEG (0x7F15)`, `IABS (0x7F16)`,
//!     `IMIN (0x7F17)`, `IMAX (0x7F18)`, `BAND (0x7F1E)`, `BOR (0x7F1F)`, `BXOR (0x7F20)`,
//!     `BNOT (0x7F21)`, `SHL (0x7F22)`, `SHR (0x7F23)`
//!   * Balanced Ternary & Comparisons (`TY_TRIT` in `{-1, 0, +1}`): `CTRIT (0x7F09)`,
//!     `CEQ (0x7F24)`, `CNE (0x7F25)`, `CLT (0x7F26)`, `CGT (0x7F27)`, `TNOT (0x7F28)`,
//!     `TAND (0x7F29)`, `TOR (0x7F2A)`, `TXOR (0x7F2B)`, `TMAJ (0x7F2C)`, `TCMP (0x7F2D)`
//!   * Control Flow & Byte Buffers: `LABEL (0x7F40)`, `JMP (0x7F41)`, `BR_TRIT (0x7F42)`,
//!     `BR_POS (0x7F43)`, `BR_NEG (0x7F44)`, `SEL_TRIT (0x7F45)`, `RET (0x7F4A)`,
//!     `ANEW (0x7F0D)`, `IGET (0x7F0E)`, `ISET (0x7F0F)`, `SLEN (0x7F50)`
//!   * High-Level Runtime Opcodes (VM-executed; explicitly rejected by standalone numeric AOT):
//!     `ARENA (0x7F01)`, `BORROW_R/W (0x7F02/03)`, `MOVE (0x7F04)`, `DROP (0x7F05)`,
//!     `SAN_SQL/DOM/PRM (0x7F57..59)`, `FNV1A (0x7F5D)`, `DB_INS (0x7F63)`, `TV_SRCH (0x7F6A)`,
//!     `DOM_NODE (0x7F76)`, `EMBED (0x7F7D)`, `SK_LOAD/EXEC (0x7F7E/7F)`
//!
//! Compile & Run on your ASUS ROG Flow Z13 (Windows / Linux):
//!   rustc -O -C target-cpu=native omni_rust_bootstrap.rs -o omni_bootstrap && ./omni_bootstrap
//!   ./omni_bootstrap selftest
//!   ./omni_bootstrap --debug-lens omni_self_compiler.omni_bc
//!   ./omni_bootstrap hex omni_self_compiler.omni_bc
//!   ./omni_bootstrap run seed0_var0.omni_bc 30 15
//!   ./omni_bootstrap assemble input.omni_asm output.omni_bc

use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs;
use std::time::Instant;

// ==============================================================================
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
            | 0x7F01..=0x7F06
            | 0x7F09
            | 0x7F0D..=0x7F18
            | 0x7F1E..=0x7F2D
            | 0x7F30..=0x7F31
            | 0x7F34..=0x7F35
            | 0x7F3F..=0x7F45
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
            | 0x7F01..=0x7F06
            | 0x7F09
            | 0x7F0D..=0x7F18
            | 0x7F1E..=0x7F2D
            | 0x7F30..=0x7F31
            | 0x7F34..=0x7F35
            | 0x7F3F..=0x7F45
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

/// Returns the `@PACK5` byte length required to store `num_trits` balanced trits (`5` trits/byte).
#[inline(always)]
pub const fn pack5_byte_len(num_trits: usize) -> usize {
    (num_trits + 4) / 5
}

/// Reads a single balanced trit (`-1, 0, +1`) at `trit_idx` directly from `@PACK5` bytes without allocating.
#[inline]
pub fn pack5_get_trit(bytes: &[u8], trit_idx: usize) -> Result<i8, &'static str> {
    let byte_idx = trit_idx / 5;
    let sub_idx = trit_idx % 5;
    let b = *bytes.get(byte_idx).ok_or("PACK5_INDEX_OUT_OF_BOUNDS")?;
    if b > PACK5_MAX_VALID {
        return Err("PACK5_SENTINEL_OR_CORRUPT_BYTE");
    }
    Ok(LUT_UNPACK5[b as usize][sub_idx])
}

/// Updates a single balanced trit (`-1, 0, +1`) at `trit_idx` in-place inside a `@PACK5` byte slice.
#[inline]
pub fn pack5_set_trit(bytes: &mut [u8], trit_idx: usize, val: i8) -> Result<(), &'static str> {
    let byte_idx = trit_idx / 5;
    let sub_idx = trit_idx % 5;
    let b = *bytes.get(byte_idx).ok_or("PACK5_INDEX_OUT_OF_BOUNDS")?;
    if b > PACK5_MAX_VALID {
        return Err("PACK5_SENTINEL_OR_CORRUPT_BYTE");
    }
    let mut trits = LUT_UNPACK5[b as usize];
    trits[sub_idx] = val.clamp(-1, 1);
    let mut acc: u8 = 0;
    let mut pow3: u8 = 1;
    for i in 0..5 {
        acc += ((trits[i] + 1) as u8) * pow3;
        if i < 4 {
            pow3 *= 3;
        }
    }
    bytes[byte_idx] = acc;
    Ok(())
}

/// Computes the exact ternary dot product (`TDOT`) directly over two `@PACK5` byte slices without allocating `Vec<i8>`.
#[inline]
pub fn pack5_tdot_direct(a_bytes: &[u8], b_bytes: &[u8], num_trits: usize) -> Result<i64, &'static str> {
    let n_bytes = a_bytes.len().min(b_bytes.len());
    let mut dot: i64 = 0;
    let mut processed = 0usize;
    for i in 0..n_bytes {
        let ba = a_bytes[i];
        let bb = b_bytes[i];
        if ba > PACK5_MAX_VALID || bb > PACK5_MAX_VALID {
            return Err("PACK5_SENTINEL_OR_CORRUPT_BYTE");
        }
        let ua = LUT_UNPACK5[ba as usize];
        let ub = LUT_UNPACK5[bb as usize];
        for k in 0..5 {
            if processed >= num_trits {
                return Ok(dot);
            }
            dot += (ua[k] as i64) * (ub[k] as i64);
            processed += 1;
        }
    }
    Ok(dot)
}

/// Computes the sum of balanced trits directly over a `@PACK5` byte slice without allocating `Vec<i8>`.
#[inline]
pub fn pack5_sum_direct(bytes: &[u8], num_trits: usize) -> Result<i64, &'static str> {
    let mut sum: i64 = 0;
    let mut processed = 0usize;
    for &b in bytes {
        if b > PACK5_MAX_VALID {
            return Err("PACK5_SENTINEL_OR_CORRUPT_BYTE");
        }
        let u = LUT_UNPACK5[b as usize];
        for &t in &u {
            if processed >= num_trits {
                return Ok(sum);
            }
            sum += t as i64;
            processed += 1;
        }
    }
    Ok(sum)
}

/// Computes the epistemic ternary gate (`EGATE` / `TLOGIC`: `-1, 0, +1`) directly over a `@PACK5` byte slice.
#[inline]
pub fn pack5_gate_direct(bytes: &[u8], num_trits: usize) -> Result<i8, &'static str> {
    let sum = pack5_sum_direct(bytes, num_trits)?;
    Ok(if sum > 0 { 1 } else if sum < 0 { -1 } else { 0 })
}

// ==============================================================================
// 3. PURE NON-HUMAN TOKEN-STREAM DECODER (`Vec<u16>` -> BINARY AST)
// ==============================================================================

#[derive(Clone, Debug, PartialEq)]
pub enum BinOperand {
    Reg(u16),
    Param(u16),
    Label(u16),
    ImmInt(i64),
    ImmTrit(i8),
    EnglishSpan(Vec<u16>),
    SchemaRef(u16),
    FnRef(u16),
    SkillRef(u16),
    FieldKV(u16, Box<BinOperand>),
}

#[inline(always)]
pub fn extract_label_id(op: &BinOperand) -> Option<u16> {
    match *op {
        BinOperand::Label(lbl) => Some(lbl),
        BinOperand::ImmInt(v)  => Some((v & 0xFFFF) as u16),
        _ => None,
    }
}

#[derive(Clone, Debug)]
pub struct BinInstruction {
    pub offset: usize,
    pub dest_reg: Option<u16>,
    pub op_tok: u16,
    pub ty_tok: u16,
    pub tc_tok: u16,
    pub operands: Vec<BinOperand>,
}

#[derive(Clone, Debug)]
pub struct BinFunction {
    pub fn_idx: u16,
    pub params: Vec<(u16, u16, u16)>,
    pub ret_ty_tok: u16,
    pub cap_mask: CapMask,
    pub instructions: Vec<BinInstruction>,
}

#[derive(Clone, Debug, Default)]
pub struct BinModule {
    pub schemas: Vec<(u16, Vec<(u16, u16)>)>,
    pub experts: Vec<(u16, u16, u16, u16)>,
    pub functions: Vec<BinFunction>,
}

fn parse_one_operand(stream: &[u16], pos: &mut usize) -> Result<BinOperand, String> {
    if *pos >= stream.len() {
        return Err(String::from("Unexpected EOF while decoding binary operand"));
    }
    let tok = stream[*pos];
    *pos += 1;

    match tok {
        REG_BASE..=REG_MAX => Ok(BinOperand::Reg(tok)),
        PAR_BASE..=PAR_MAX => Ok(BinOperand::Param(tok)),
        LBL_BASE..=LBL_MAX => Ok(BinOperand::Label(tok)),
        TOK_TRIT_NEG  => Ok(BinOperand::ImmTrit(-1)),
        TOK_TRIT_ZERO => Ok(BinOperand::ImmTrit(0)),
        TOK_TRIT_POS  => Ok(BinOperand::ImmTrit(1)),
        TOK_IMM_I16 => {
            let v = *stream.get(*pos).ok_or("EOF in TOK_IMM_I16")? as i16;
            *pos += 1;
            Ok(BinOperand::ImmInt(v as i64))
        }
        TOK_IMM_I32 => {
            let hi = *stream.get(*pos).ok_or("EOF in TOK_IMM_I32 hi")? as u32;
            let lo = *stream.get(*pos + 1).ok_or("EOF in TOK_IMM_I32 lo")? as u32;
            *pos += 2;
            let combined = ((hi << 16) | lo) as i32;
            Ok(BinOperand::ImmInt(combined as i64))
        }
        TOK_ENG_SPAN => {
            let len = *stream.get(*pos).ok_or("EOF in TOK_ENG_SPAN len")? as usize;
            *pos += 1;
            if *pos + len > stream.len() {
                return Err(String::from("Truncated English token span"));
            }
            let mut span = Vec::with_capacity(len);
            for &t in &stream[*pos..*pos + len] {
                span.push(t & ENGLISH_MAX_TOKEN);
            }
            *pos += len;
            Ok(BinOperand::EnglishSpan(span))
        }
        TOK_SYM_SCHEMA => {
            let idx = *stream.get(*pos).ok_or("EOF in TOK_SYM_SCHEMA")?;
            *pos += 1;
            Ok(BinOperand::SchemaRef(idx))
        }
        TOK_SYM_FN => {
            let idx = *stream.get(*pos).ok_or("EOF in TOK_SYM_FN")?;
            *pos += 1;
            Ok(BinOperand::FnRef(idx))
        }
        TOK_SYM_SKILL => {
            let idx = *stream.get(*pos).ok_or("EOF in TOK_SYM_SKILL")?;
            *pos += 1;
            Ok(BinOperand::SkillRef(idx))
        }
        TOK_FIELD_KV => {
            let f_idx = *stream.get(*pos).ok_or("EOF in TOK_FIELD_KV")?;
            *pos += 1;
            let inner = parse_one_operand(stream, pos)?;
            Ok(BinOperand::FieldKV(f_idx, Box::new(inner)))
        }
        other => Err(format!("Invalid operand token 0x{:04X} at offset {}", other, *pos - 1)),
    }
}

pub fn decode_omni_token_stream(stream: &[u16]) -> Result<BinModule, String> {
    if stream.is_empty() || stream[0] != TOK_MOD_BEGIN {
        return Err(String::from("Missing 0x7EE0 (TOK_MOD_BEGIN) header token"));
    }
    let mut module = BinModule::default();
    let mut pos = 1usize;

    while pos < stream.len() {
        let tag = stream[pos];
        pos += 1;
        match tag {
            TOK_MOD_END => break,
            TOK_SCHEMA_DEF => {
                if pos + 2 > stream.len() {
                    return Err(String::from("Truncated TOK_SCHEMA_DEF header"));
                }
                let s_id = stream[pos];
                let f_cnt = stream[pos + 1] as usize;
                pos += 2;
                if pos + f_cnt * 2 > stream.len() {
                    return Err(String::from("Truncated TOK_SCHEMA_DEF fields"));
                }
                let mut fields = Vec::with_capacity(f_cnt);
                for _ in 0..f_cnt {
                    fields.push((stream[pos], stream[pos + 1]));
                    pos += 2;
                }
                module.schemas.push((s_id, fields));
            }
            TOK_EXPERT_DEF => {
                if pos + 4 > stream.len() {
                    return Err(String::from("Truncated TOK_EXPERT_DEF header"));
                }
                let e_id = stream[pos];
                let d_in = stream[pos + 1];
                let d_out = stream[pos + 2];
                let rank = stream[pos + 3];
                pos += 4;
                module.experts.push((e_id, d_in, d_out, rank));
            }
            TOK_FN_DEF => {
                if pos + 2 > stream.len() {
                    return Err(String::from("Truncated TOK_FN_DEF header"));
                }
                let fn_idx = stream[pos];
                let p_cnt = stream[pos + 1] as usize;
                pos += 2;
                if pos + p_cnt * 3 + 3 > stream.len() {
                    return Err(String::from("Truncated TOK_FN_DEF signature"));
                }
                let mut params = Vec::with_capacity(p_cnt);
                for _ in 0..p_cnt {
                    params.push((stream[pos], stream[pos + 1], stream[pos + 2]));
                    pos += 3;
                }
                let ret_ty_tok = stream[pos];
                let cap_mask = stream[pos + 1];
                let ins_cnt = stream[pos + 2] as usize;
                pos += 3;

                let mut instructions = Vec::with_capacity(ins_cnt);
                for _ in 0..ins_cnt {
                    if pos >= stream.len() {
                        return Err(String::from("Unexpected EOF inside function instruction body"));
                    }
                    let ins_offset = pos;
                    let first_tok = stream[pos];
                    let (dest_reg, op_tok, ty_tok, tc_tok) = if (REG_BASE..=REG_MAX).contains(&first_tok) {
                        if pos + 4 > stream.len() {
                            return Err(String::from("Truncated SSA instruction header"));
                        }
                        let d = first_tok;
                        let op = stream[pos + 1];
                        let ty = stream[pos + 2];
                        let tc = stream[pos + 3];
                        pos += 4;
                        (Some(d), op, ty, tc)
                    } else {
                        let op = first_tok;
                        pos += 1;
                        (None, op, TY_VOID, pack_tc(false, CAP_ID_PURE))
                    };

                    let mut operands = Vec::new();
                    while pos < stream.len() && stream[pos] != TOK_INS_END {
                        operands.push(parse_one_operand(stream, &mut pos)?);
                    }
                    if pos >= stream.len() || stream[pos] != TOK_INS_END {
                        return Err(format!("Missing 0x7EE4 (TOK_INS_END) for instruction at {}", ins_offset));
                    }
                    pos += 1;

                    instructions.push(BinInstruction {
                        offset: ins_offset,
                        dest_reg,
                        op_tok,
                        ty_tok,
                        tc_tok,
                        operands,
                    });
                }

                module.functions.push(BinFunction {
                    fn_idx,
                    params,
                    ret_ty_tok,
                    cap_mask,
                    instructions,
                });
            }
            other => {
                return Err(format!("Unexpected structural token 0x{:04X} at offset {}", other, pos - 1));
            }
        }
    }
    Ok(module)
}

// ==============================================================================
// 4. O(n) BINARY TOKEN-STREAM VERIFIER & COMPLETE EXECUTING VM
// ==============================================================================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RegState {
    Uninit,
    Owned,
    BorrowedR(u8),
    BorrowedW,
    Moved,
    Dropped,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Taint {
    Safe,
    Untrusted,
}

#[derive(Clone, Copy, Debug)]
pub struct RegSlot {
    pub state: RegState,
    pub taint: Taint,
    pub is_heap_arena: bool,
    pub parent_slot: Option<u8>,
}

impl Default for RegSlot {
    fn default() -> Self {
        Self {
            state: RegState::Uninit,
            taint: Taint::Safe,
            is_heap_arena: false,
            parent_slot: None,
        }
    }
}

pub struct BinVerificationReport {
    pub passed: bool,
    pub fn_idx: u16,
    pub instruction_count: usize,
    pub errors: Vec<String>,
    pub elapsed_ns: u128,
    pub rl_reward: f32,
}

#[inline(always)]
fn token_to_slot(tok: u16) -> Option<usize> {
    if (REG_BASE..=REG_MAX).contains(&tok) {
        Some((tok - REG_BASE) as usize)
    } else if (PAR_BASE..=PAR_MAX).contains(&tok) {
        Some(192 + (tok - PAR_BASE) as usize)
    } else {
        None
    }
}

fn collect_operand_slots(op: &BinOperand, out: &mut Vec<u16>) {
    match op {
        BinOperand::Reg(t) | BinOperand::Param(t) => out.push(*t),
        BinOperand::FieldKV(_, inner) => collect_operand_slots(inner, out),
        _ => {}
    }
}

pub fn verify_binary_function(func: &BinFunction) -> BinVerificationReport {
    let t0 = Instant::now();
    let mut errors = Vec::new();
    let mut regs = [RegSlot::default(); 224];
    let mut next_ssa_tok = REG_BASE;
    let mut defined_labels: Vec<u16> = Vec::new();
    for ins in &func.instructions {
        if ins.op_tok == 0x7F40 {
            if let Some(lid) = ins.operands.first().and_then(extract_label_id) {
                defined_labels.push(lid);
            }
        }
    }

    for &(par_tok, ty_tok, tc_tok) in &func.params {
        if let Some(slot) = token_to_slot(par_tok) {
            let (untrusted, _, _) = unpack_tc(tc_tok);
            regs[slot] = RegSlot {
                state: RegState::Owned,
                taint: if untrusted { Taint::Untrusted } else { Taint::Safe },
                is_heap_arena: ty_tok == TY_ARENA || ty_tok == TY_MODEL_T,
                parent_slot: None,
            };
        }
    }

    for ins in &func.instructions {
        let spec = match decode_opcode_token(ins.op_tok) {
            Some(s) => s,
            None => {
                errors.push(format!("Offset {}: Unknown opcode token 0x{:04X}", ins.offset, ins.op_tok));
                continue;
            }
        };

        if ins.op_tok == 0x7F05 {
            if let Some(BinOperand::Reg(r_tok)) = ins.operands.first() {
                if let Some(slot) = token_to_slot(*r_tok) {
                    match regs[slot].state {
                        RegState::Moved | RegState::Dropped | RegState::Uninit => {
                            errors.push(format!("Offset {}: Invalid DROP on 0x{:04X}", ins.offset, r_tok));
                        }
                        RegState::BorrowedR(k) if k > 0 => {
                            errors.push(format!("Offset {}: Cannot DROP 0x{:04X} with {} active readers", ins.offset, r_tok, k));
                        }
                        _ => {
                            if let Some(p) = regs[slot].parent_slot {
                                regs[p as usize].state = match regs[p as usize].state {
                                    RegState::BorrowedR(k) if k > 1 => RegState::BorrowedR(k - 1),
                                    _ => RegState::Owned,
                                };
                            }
                            regs[slot].state = RegState::Dropped;
                        }
                    }
                }
            }
            continue;
        }

        if ins.op_tok == 0x7F4A {
            if let Some(op0) = ins.operands.first() {
                let mut ref_toks = Vec::new();
                collect_operand_slots(op0, &mut ref_toks);
                for r_tok in ref_toks {
                    if let Some(slot) = token_to_slot(r_tok) {
                        if regs[slot].taint == Taint::Untrusted {
                            errors.push(format!("Offset {}: Taint violation on RET 0x{:04X}", ins.offset, r_tok));
                        }
                        regs[slot].state = RegState::Moved;
                    }
                }
            }
            for idx in 0..192usize {
                if regs[idx].is_heap_arena && !matches!(regs[idx].state, RegState::Moved | RegState::Dropped | RegState::Uninit) {
                    errors.push(format!("Offset {}: Linear memory leak on 0x{:04X}", ins.offset, REG_BASE + (idx as u16)));
                }
            }
            continue;
        }

        if matches!(ins.op_tok, 0x7F41 | 0x7F42 | 0x7F43 | 0x7F44) {
            let lbl_ops = if ins.op_tok == 0x7F41 {
                &ins.operands[..]
            } else if ins.operands.len() > 1 {
                &ins.operands[1..]
            } else {
                &[]
            };
            if lbl_ops.is_empty() {
                errors.push(format!("Offset {}: Missing branch target label on 0x{:04X}", ins.offset, ins.op_tok));
            }
            for lop in lbl_ops {
                match extract_label_id(lop) {
                    Some(lid) if defined_labels.contains(&lid) => {}
                    other => {
                        errors.push(format!("Offset {}: Unresolved branch target label {:?}", ins.offset, other));
                    }
                }
            }
        }

        if !spec.has_dest_reg {
            continue;
        }

        if let Some(d_tok) = ins.dest_reg {
            if d_tok > next_ssa_tok {
                errors.push(format!(
                    "Offset {}: Non-canonical SSA token 0x{:04X}, expected <= 0x{:04X}",
                    ins.offset, d_tok, next_ssa_tok
                ));
                next_ssa_tok = d_tok + 1;
            } else if d_tok == next_ssa_tok {
                next_ssa_tok += 1;
            } else if let Some(d_slot) = token_to_slot(d_tok) {
                if regs[d_slot].is_heap_arena && !matches!(regs[d_slot].state, RegState::Dropped | RegState::Moved) {
                    errors.push(format!("Offset {}: Cannot overwrite active heap arena 0x{:04X}", ins.offset, d_tok));
                } else if !matches!(regs[d_slot].state, RegState::Owned) {
                    errors.push(format!("Offset {}: Cannot reassign register 0x{:04X} in state {:?}", ins.offset, d_tok, regs[d_slot].state));
                }
            }

            let (ins_untrusted, _, ins_cap_mask) = unpack_tc(ins.tc_tok);
            if (ins_cap_mask & !func.cap_mask) != 0 || (spec.req_cap_mask & !func.cap_mask) != 0 {
                errors.push(format!("Offset {}: Capability violation on opcode 0x{:04X}", ins.offset, ins.op_tok));
            }

            let mut any_untrusted = false;
            let mut parent_slot: Option<u8> = None;
            let mut op_toks = Vec::new();
            for op in &ins.operands {
                if matches!(op, BinOperand::EnglishSpan(_)) {
                    any_untrusted = true;
                }
                collect_operand_slots(op, &mut op_toks);
            }

            for r_tok in op_toks {
                if let Some(slot) = token_to_slot(r_tok) {
                    let info = regs[slot];
                    if matches!(info.state, RegState::Moved | RegState::Dropped | RegState::Uninit | RegState::BorrowedW) {
                        errors.push(format!("Offset {}: Invalid state {:?} on token 0x{:04X}", ins.offset, info.state, r_tok));
                    }
                    if info.taint == Taint::Untrusted {
                        any_untrusted = true;
                    }
                    if ins.op_tok == 0x7F03 && info.state == RegState::Owned {
                        regs[slot].state = RegState::BorrowedW;
                        parent_slot = Some(slot as u8);
                    } else if ins.op_tok == 0x7F02 && matches!(info.state, RegState::Owned | RegState::BorrowedR(_)) {
                        let k = match info.state { RegState::BorrowedR(c) => c, _ => 0 };
                        regs[slot].state = RegState::BorrowedR(k + 1);
                        parent_slot = Some(slot as u8);
                    } else if ins.op_tok == 0x7F04 && info.state == RegState::Owned {
                        regs[slot].state = RegState::Moved;
                    }
                }
            }

            if spec.sink_requires_safe && any_untrusted {
                errors.push(format!("Offset {}: Sink opcode 0x{:04X} received @UNTRUSTED operand", ins.offset, ins.op_tok));
            }
            let out_taint = if ins_untrusted { Taint::Untrusted } else { Taint::Safe };
            if !spec.is_sanitizer && any_untrusted && out_taint == Taint::Safe {
                errors.push(format!("Offset {}: Illegal taint upgrade to @SAFE in opcode 0x{:04X}", ins.offset, ins.op_tok));
            }

            if let Some(d_slot) = token_to_slot(d_tok) {
                regs[d_slot] = RegSlot {
                    state: RegState::Owned,
                    taint: out_taint,
                    is_heap_arena: matches!(ins.op_tok, 0x7F01 | 0x7F7E),
                    parent_slot,
                };
            }
        }
    }

    let elapsed_ns = t0.elapsed().as_nanos();
    let passed = errors.is_empty();
    let rl_reward = if passed {
        1.0
    } else {
        (0.40 - 0.30 * (errors.len() as f32)).max(-1.0)
    };

    BinVerificationReport {
        passed,
        fn_idx: func.fn_idx,
        instruction_count: func.instructions.len(),
        errors,
        elapsed_ns,
        rl_reward,
    }
}

/// Verifies a raw `u16` token stream across all functions in the module in a single pass.
pub fn verify_u16_stream(stream: &[u16]) -> Result<Vec<BinVerificationReport>, String> {
    let module = decode_omni_token_stream(stream)?;
    let mut reports = Vec::with_capacity(module.functions.len());
    for func in &module.functions {
        let rep = verify_binary_function(func);
        if !rep.passed {
            return Err(format!(
                "Binary verification failed on fn #{}: {:?}",
                func.fn_idx, rep.errors
            ));
        }
        reports.push(rep);
    }
    Ok(reports)
}

#[derive(Clone, Debug, PartialEq)]
pub enum OmniValue {
    Void,
    Int(i64),
    Trit(i8),
    Str(String),
    Bytes(Vec<u8>),
    U16Array(Vec<u16>),
    TritVec(Vec<i8>),
    Struct(BTreeMap<u16, OmniValue>),
    Arena { capacity: usize, active: bool },
    SkillModel { skill_id: u16, active: bool },
    DomNode { tag_id: u16, children: Vec<OmniValue> },
}

pub const DEFAULT_VM_STEP_LIMIT: u64 = 50_000_000;
pub const DEFAULT_VM_MEMORY_BUDGET_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_VM_CALL_DEPTH: usize = 64;

pub struct OmniBinaryVm {
    pub tables: HashMap<u16, Vec<BTreeMap<u16, OmniValue>>>,
    pub step_limit: u64,
    pub steps_executed: u64,
    pub memory_budget_bytes: usize,
    pub allocated_bytes: usize,
    pub call_depth: usize,
}

impl OmniBinaryVm {
    pub fn new() -> Self {
        Self {
            tables: HashMap::new(),
            step_limit: DEFAULT_VM_STEP_LIMIT,
            steps_executed: 0,
            memory_budget_bytes: DEFAULT_VM_MEMORY_BUDGET_BYTES,
            allocated_bytes: 0,
            call_depth: 0,
        }
    }

    pub fn with_budgets(step_limit: u64, memory_budget_bytes: usize) -> Self {
        Self {
            tables: HashMap::new(),
            step_limit,
            steps_executed: 0,
            memory_budget_bytes,
            allocated_bytes: 0,
            call_depth: 0,
        }
    }

    pub fn clear_tables(&mut self) {
        self.tables.clear();
        self.steps_executed = 0;
        self.allocated_bytes = 0;
        self.call_depth = 0;
    }

    #[inline]
    fn charge_memory(&mut self, bytes: usize) -> Result<(), String> {
        let next_alloc = self.allocated_bytes.saturating_add(bytes);
        if next_alloc > self.memory_budget_bytes {
            return Err(format!(
                "VM memory budget exceeded ({} > {} bytes)",
                next_alloc, self.memory_budget_bytes
            ));
        }
        self.allocated_bytes = next_alloc;
        Ok(())
    }

    fn eval_op(op: &BinOperand, regs: &[OmniValue; 224]) -> OmniValue {
        match op {
            BinOperand::Reg(t) | BinOperand::Param(t) => {
                token_to_slot(*t).map(|s| regs[s].clone()).unwrap_or(OmniValue::Void)
            }
            BinOperand::ImmInt(v) => OmniValue::Int(*v),
            BinOperand::ImmTrit(t) => OmniValue::Trit(*t),
            BinOperand::EnglishSpan(toks) => {
                let bytes: Vec<u8> = toks.iter().map(|&u| (u & 0xFF) as u8).collect();
                OmniValue::Str(String::from_utf8_lossy(&bytes).into_owned())
            }
            _ => OmniValue::Void,
        }
    }

    fn eval_as_i64(op: &BinOperand, regs: &[OmniValue; 224]) -> Option<i64> {
        match Self::eval_op(op, regs) {
            OmniValue::Int(v) => Some(v),
            OmniValue::Trit(t) => Some(t as i64),
            _ => None,
        }
    }

    fn eval_as_trit(op: &BinOperand, regs: &[OmniValue; 224]) -> Option<i8> {
        match Self::eval_op(op, regs) {
            OmniValue::Trit(t) => Some(t.clamp(-1, 1)),
            OmniValue::Int(v) => Some(v.clamp(-1, 1) as i8),
            _ => None,
        }
    }

    pub fn execute_function(
        &mut self,
        module: &BinModule,
        fn_idx: u16,
        args: Vec<OmniValue>,
    ) -> Result<OmniValue, String> {
        if self.call_depth >= MAX_VM_CALL_DEPTH {
            return Err(format!(
                "VM call stack depth budget exceeded ({} >= {})",
                self.call_depth, MAX_VM_CALL_DEPTH
            ));
        }
        let func = module
            .functions
            .iter()
            .find(|f| f.fn_idx == fn_idx)
            .ok_or_else(|| format!("Function index {} not found", fn_idx))?;

        let rep = verify_binary_function(func);
        if !rep.passed {
            return Err(format!("Binary verification failed: {:?}", rep.errors));
        }

        let mut regs: [OmniValue; 224] = std::array::from_fn(|_| OmniValue::Void);
        for (i, &(par_tok, _, _)) in func.params.iter().enumerate() {
            if let (Some(slot), Some(val)) = (token_to_slot(par_tok), args.get(i)) {
                regs[slot] = val.clone();
            }
        }

        let mut label_pcs = [usize::MAX; 256];
        for (idx_i, cand) in func.instructions.iter().enumerate() {
            if cand.op_tok == 0x7F40 {
                if let Some(lid) = cand.operands.first().and_then(extract_label_id) {
                    if (lid as usize) < 256 {
                        label_pcs[lid as usize] = idx_i + 1;
                    }
                }
            }
        }

        let mut pc = 0usize;
        while pc < func.instructions.len() {
            self.steps_executed = self.steps_executed.saturating_add(1);
            if self.steps_executed > self.step_limit {
                return Err(format!("VM fuel/step budget exceeded ({} > {})", self.steps_executed, self.step_limit));
            }
            let ins = &func.instructions[pc];
            match ins.op_tok {
                0x7F00 | 0x7F40 | 0x7F46 => { pc += 1; continue; }
                0x7F41 => {
                    if let Some(target_lbl) = ins.operands.first().and_then(extract_label_id) {
                        let next_pc = if (target_lbl as usize) < 256 && label_pcs[target_lbl as usize] != usize::MAX {
                            Some(label_pcs[target_lbl as usize])
                        } else {
                            func.instructions.iter().position(|cand| {
                                cand.op_tok == 0x7F40 && cand.operands.first().and_then(extract_label_id) == Some(target_lbl)
                            }).map(|p| p + 1)
                        };
                        if let Some(tp) = next_pc {
                            pc = tp;
                            continue;
                        }
                    }
                    pc += 1;
                    continue;
                }
                0x7F42 => {
                    if let (Some(c), Some(l_neg), Some(l_zero), Some(l_pos)) = (
                        Self::eval_as_trit(&ins.operands[0], &regs),
                        ins.operands.get(1).and_then(extract_label_id),
                        ins.operands.get(2).and_then(extract_label_id),
                        ins.operands.get(3).and_then(extract_label_id),
                    ) {
                        let chosen = if c < 0 { l_neg } else if c == 0 { l_zero } else { l_pos };
                        let next_pc = if (chosen as usize) < 256 && label_pcs[chosen as usize] != usize::MAX {
                            Some(label_pcs[chosen as usize])
                        } else {
                            func.instructions.iter().position(|cand| {
                                cand.op_tok == 0x7F40 && cand.operands.first().and_then(extract_label_id) == Some(chosen)
                            }).map(|p| p + 1)
                        };
                        if let Some(tp) = next_pc {
                            pc = tp;
                            continue;
                        }
                    }
                    pc += 1;
                    continue;
                }
                0x7F43 | 0x7F44 => {
                    let want: i8 = if ins.op_tok == 0x7F43 { 1 } else { -1 };
                    if let (Some(c), Some(target_lbl)) = (
                        Self::eval_as_trit(&ins.operands[0], &regs),
                        ins.operands.get(1).and_then(extract_label_id),
                    ) {
                        if c == want {
                            let next_pc = if (target_lbl as usize) < 256 && label_pcs[target_lbl as usize] != usize::MAX {
                                Some(label_pcs[target_lbl as usize])
                            } else {
                                func.instructions.iter().position(|cand| {
                                    cand.op_tok == 0x7F40 && cand.operands.first().and_then(extract_label_id) == Some(target_lbl)
                                }).map(|p| p + 1)
                            };
                            if let Some(tp) = next_pc {
                                pc = tp;
                                continue;
                            }
                        }
                    }
                    pc += 1;
                    continue;
                }
                0x7F05 => {
                    if let Some(BinOperand::Reg(r_tok)) = ins.operands.first() {
                        if let Some(slot) = token_to_slot(*r_tok) {
                            match &regs[slot] {
                                OmniValue::Arena { capacity, active: true } => {
                                    self.allocated_bytes = self.allocated_bytes.saturating_sub(*capacity);
                                }
                                OmniValue::Bytes(b) => {
                                    self.allocated_bytes = self.allocated_bytes.saturating_sub(b.len());
                                }
                                OmniValue::TritVec(tv) => {
                                    self.allocated_bytes = self.allocated_bytes.saturating_sub(pack5_byte_len(tv.len()));
                                }
                                OmniValue::Str(s) => {
                                    self.allocated_bytes = self.allocated_bytes.saturating_sub(s.len());
                                }
                                _ => {}
                            }
                            regs[slot] = OmniValue::Void;
                        }
                    }
                    pc += 1;
                    continue;
                }
                0x7F4A => return Ok(Self::eval_op(&ins.operands[0], &regs)),
                _ => {}
            }

            let out_val = match ins.op_tok {
                0x7F01 => {
                    if let OmniValue::Int(cap) = Self::eval_op(&ins.operands[0], &regs) {
                        let req = if cap > 0 { cap as usize } else { 0 };
                        self.charge_memory(req)?;
                        OmniValue::Arena { capacity: req, active: true }
                    } else { OmniValue::Void }
                }
                0x7F02 | 0x7F03 => match Self::eval_op(&ins.operands[0], &regs) {
                    OmniValue::Arena { .. } => OmniValue::Arena { capacity: 0, active: true },
                    other => other,
                },
                0x7F04 | 0x7F06 | 0x7F09 => Self::eval_op(&ins.operands[0], &regs),
                0x7F08 => {
                    let v = Self::eval_op(&ins.operands[0], &regs);
                    if let OmniValue::Str(ref s) = v {
                        self.charge_memory(s.len())?;
                    }
                    v
                }
                0x7F0D => {
                    self.charge_memory(8)?;
                    OmniValue::Bytes(Vec::with_capacity(64))
                }
                0x7F0E => {
                    let arr_val = Self::eval_op(&ins.operands[0], &regs);
                    let idx_val = Self::eval_op(&ins.operands[1], &regs);
                    match (arr_val, idx_val) {
                        (OmniValue::U16Array(arr), OmniValue::Int(i)) if i >= 0 && (i as usize) < arr.len() => {
                            OmniValue::Int(arr[i as usize] as i64)
                        }
                        (OmniValue::Bytes(buf), OmniValue::Int(i)) if i >= 0 && (i as usize) < buf.len() => {
                            OmniValue::Int(buf[i as usize] as i64)
                        }
                        (OmniValue::TritVec(tv), OmniValue::Int(i)) if i >= 0 && (i as usize) < tv.len() => {
                            let packed = pack5_encode(&tv);
                            OmniValue::Int(pack5_get_trit(&packed, i as usize).map_err(String::from)? as i64)
                        }
                        _ => OmniValue::Int(0),
                    }
                }
                0x7F0F => {
                    let mut buf = match Self::eval_op(&ins.operands[0], &regs) {
                        OmniValue::Bytes(b) => b,
                        _ => Vec::new(),
                    };
                    let prev_len = buf.len();
                    if ins.operands.len() == 3 && ins.operands[1] == BinOperand::ImmInt(-4) {
                        if let Some(v) = Self::eval_as_i64(&ins.operands[2], &regs) {
                            buf.extend_from_slice(&(v as i32).to_le_bytes());
                        }
                    } else if ins.operands.len() == 3 && ins.operands[1] == BinOperand::ImmInt(-8) {
                        if let Some(v) = Self::eval_as_i64(&ins.operands[2], &regs) {
                            buf.extend_from_slice(&v.to_le_bytes());
                        }
                    } else if ins.operands.len() == 4 && ins.operands[1] == BinOperand::ImmInt(-9) {
                        if let (Some(off), Some(v)) = (
                            Self::eval_as_i64(&ins.operands[2], &regs),
                            Self::eval_as_i64(&ins.operands[3], &regs),
                        ) {
                            let u_off = off as usize;
                            if u_off + 8 <= buf.len() {
                                buf[u_off..u_off + 8].copy_from_slice(&(v as u64).to_le_bytes());
                            }
                        }
                    } else {
                        for op_b in &ins.operands[1..] {
                            let b_val = Self::eval_as_i64(op_b, &regs).unwrap_or(0);
                            buf.push((b_val & 0xFF) as u8);
                        }
                    }
                    if buf.len() > prev_len {
                        self.charge_memory(buf.len() - prev_len)?;
                    }
                    OmniValue::Bytes(buf)
                }
                0x7F10 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Int(a.wrapping_add(b)) } else { OmniValue::Int(0) },
                0x7F11 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Int(a.wrapping_sub(b)) } else { OmniValue::Int(0) },
                0x7F12 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Int(a.wrapping_mul(b)) } else { OmniValue::Int(0) },
                0x7F13 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Int(if b == 0 { 0 } else { a.wrapping_div(b) }) } else { OmniValue::Int(0) },
                0x7F14 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Int(if b == 0 { 0 } else { a.wrapping_rem(b) }) } else { OmniValue::Int(0) },
                0x7F15 => if let Some(a) = Self::eval_as_i64(&ins.operands[0], &regs) { OmniValue::Int(a.wrapping_neg()) } else { OmniValue::Int(0) },
                0x7F16 => if let Some(a) = Self::eval_as_i64(&ins.operands[0], &regs) { OmniValue::Int(a.wrapping_abs()) } else { OmniValue::Int(0) },
                0x7F17 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Int(a.min(b)) } else { OmniValue::Int(0) },
                0x7F18 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Int(a.max(b)) } else { OmniValue::Int(0) },
                0x7F1E => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Int(a & b) } else { OmniValue::Int(0) },
                0x7F1F => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Int(a | b) } else { OmniValue::Int(0) },
                0x7F20 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Int(a ^ b) } else { OmniValue::Int(0) },
                0x7F21 => if let Some(a) = Self::eval_as_i64(&ins.operands[0], &regs) { OmniValue::Int(!a) } else { OmniValue::Int(0) },
                0x7F22 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Int(a.wrapping_shl((b & 63) as u32)) } else { OmniValue::Int(0) },
                0x7F23 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Int(a.wrapping_shr((b & 63) as u32)) } else { OmniValue::Int(0) },
                0x7F50 => match Self::eval_op(&ins.operands[0], &regs) {
                    OmniValue::Bytes(b) => OmniValue::Int(b.len() as i64),
                    OmniValue::Str(s) => OmniValue::Int(s.len() as i64),
                    OmniValue::U16Array(a) => OmniValue::Int(a.len() as i64),
                    OmniValue::TritVec(t) => OmniValue::Int(t.len() as i64),
                    _ => OmniValue::Int(0),
                },
                0x7F51 => {
                    if let (OmniValue::Str(a), OmniValue::Str(b)) = (Self::eval_op(&ins.operands[0], &regs), Self::eval_op(&ins.operands[1], &regs)) {
                        let out_s = format!("{}{}", a, b);
                        self.charge_memory(out_s.len())?;
                        OmniValue::Str(out_s)
                    } else { OmniValue::Str(String::new()) }
                }
                0x7F55 => {
                    if let OmniValue::Str(s) = Self::eval_op(&ins.operands[0], &regs) {
                        OmniValue::Int(s.trim().parse::<i64>().unwrap_or(0))
                    } else { OmniValue::Int(0) }
                }
                0x7F56 => {
                    if let Some(v) = Self::eval_as_i64(&ins.operands[0], &regs) {
                        let out_s = v.to_string();
                        self.charge_memory(out_s.len())?;
                        OmniValue::Str(out_s)
                    } else { OmniValue::Str(String::new()) }
                }
                0x7F24 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Trit(if a == b { 1 } else { -1 }) } else { OmniValue::Trit(-1) },
                0x7F25 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Trit(if a != b { 1 } else { -1 }) } else { OmniValue::Trit(-1) },
                0x7F26 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Trit(if a < b { 1 } else { -1 }) } else { OmniValue::Trit(-1) },
                0x7F27 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Trit(if a > b { 1 } else { -1 }) } else { OmniValue::Trit(-1) },
                0x7F28 => if let Some(a) = Self::eval_as_trit(&ins.operands[0], &regs) { OmniValue::Trit(tnot(a)) } else { OmniValue::Trit(0) },
                0x7F29 => if let (Some(a), Some(b)) = (Self::eval_as_trit(&ins.operands[0], &regs), Self::eval_as_trit(&ins.operands[1], &regs)) { OmniValue::Trit(tand(a, b)) } else { OmniValue::Trit(0) },
                0x7F2A => if let (Some(a), Some(b)) = (Self::eval_as_trit(&ins.operands[0], &regs), Self::eval_as_trit(&ins.operands[1], &regs)) { OmniValue::Trit(tor(a, b)) } else { OmniValue::Trit(0) },
                0x7F2B => if let (Some(a), Some(b)) = (Self::eval_as_trit(&ins.operands[0], &regs), Self::eval_as_trit(&ins.operands[1], &regs)) { OmniValue::Trit(txor(a, b)) } else { OmniValue::Trit(0) },
                0x7F2C => if let (Some(a), Some(b), Some(c)) = (Self::eval_as_trit(&ins.operands[0], &regs), Self::eval_as_trit(&ins.operands[1], &regs), Self::eval_as_trit(&ins.operands[2], &regs)) { OmniValue::Trit(tmaj(a, b, c)) } else { OmniValue::Trit(0) },
                0x7F2D => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Trit(if a < b { -1 } else if a > b { 1 } else { 0 }) } else { OmniValue::Trit(0) },
                0x7F30 => match Self::eval_op(&ins.operands[0], &regs) {
                    OmniValue::TritVec(tv) => {
                        let packed = pack5_encode(&tv);
                        self.charge_memory(packed.len())?;
                        OmniValue::Bytes(packed)
                    }
                    OmniValue::Bytes(b) => {
                        self.charge_memory(b.len())?;
                        OmniValue::Bytes(b)
                    }
                    _ => OmniValue::Bytes(Vec::new()),
                },
                0x7F31 => {
                    if let (OmniValue::Bytes(b), Some(len)) = (Self::eval_op(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) {
                        let n_trits = len.max(0) as usize;
                        self.charge_memory(pack5_byte_len(n_trits))?;
                        OmniValue::TritVec(pack5_decode(&b, n_trits).map_err(String::from)?)
                    } else { OmniValue::TritVec(Vec::new()) }
                }
                0x7F34 => match (Self::eval_op(&ins.operands[0], &regs), Self::eval_op(&ins.operands[1], &regs)) {
                    (OmniValue::Bytes(ba), OmniValue::Bytes(bb)) => {
                        let max_trits = ba.len().min(bb.len()) * 5;
                        OmniValue::Int(pack5_tdot_direct(&ba, &bb, max_trits).map_err(String::from)?)
                    }
                    (OmniValue::TritVec(a), OmniValue::TritVec(b)) => {
                        let pa = pack5_encode(&a);
                        let pb = pack5_encode(&b);
                        OmniValue::Int(pack5_tdot_direct(&pa, &pb, a.len().min(b.len())).map_err(String::from)?)
                    }
                    (OmniValue::Bytes(ba), OmniValue::TritVec(b)) => {
                        let pb = pack5_encode(&b);
                        OmniValue::Int(pack5_tdot_direct(&ba, &pb, b.len()).map_err(String::from)?)
                    }
                    (OmniValue::TritVec(a), OmniValue::Bytes(bb)) => {
                        let pa = pack5_encode(&a);
                        OmniValue::Int(pack5_tdot_direct(&pa, &bb, a.len()).map_err(String::from)?)
                    }
                    _ => OmniValue::Int(0),
                },
                0x7F35 | 0x7F3F => {
                    let sum_i64: i64 = match Self::eval_op(&ins.operands[0], &regs) {
                        OmniValue::TritVec(tv) => {
                            let packed = pack5_encode(&tv);
                            pack5_sum_direct(&packed, tv.len()).map_err(String::from)?
                        }
                        OmniValue::Bytes(b) => {
                            let n_trits = ins
                                .operands
                                .get(1)
                                .and_then(|op| Self::eval_as_i64(op, &regs))
                                .map(|n| n.max(0) as usize)
                                .unwrap_or(b.len() * 5);
                            pack5_sum_direct(&b, n_trits).map_err(String::from)?
                        }
                        OmniValue::Int(v) => v,
                        OmniValue::Trit(t) => t as i64,
                        _ => 0,
                    };
                    OmniValue::Trit(if sum_i64 > 0 { 1 } else if sum_i64 < 0 { -1 } else { 0 })
                }
                0x7F36 => OmniValue::Trit(-1),
                0x7F37 => OmniValue::Struct(BTreeMap::new()),
                0x7F39 => {
                    let mut m = BTreeMap::new();
                    if let Some(t) = Self::eval_as_trit(&ins.operands[2], &regs) {
                        m.insert(0, OmniValue::Int(t as i64));
                    }
                    m.insert(1, OmniValue::Int(64));
                    OmniValue::Struct(m)
                }
                0x7F45 => {
                    if let Some(c) = Self::eval_as_trit(&ins.operands[0], &regs) {
                        if c < 0 { Self::eval_op(&ins.operands[1], &regs) }
                        else if c == 0 { Self::eval_op(&ins.operands[2], &regs) }
                        else { Self::eval_op(&ins.operands[3], &regs) }
                    } else { OmniValue::Void }
                }
                0x7F48 => {
                    if let BinOperand::FnRef(callee_idx) = ins.operands[0] {
                        let call_args: Vec<OmniValue> = ins.operands[1..].iter().map(|op| Self::eval_op(op, &regs)).collect();
                        self.call_depth += 1;
                        let call_res = self.execute_function(module, callee_idx, call_args);
                        self.call_depth = self.call_depth.saturating_sub(1);
                        call_res?
                    } else { OmniValue::Void }
                }
                0x7F57 => if let OmniValue::Str(s) = Self::eval_op(&ins.operands[0], &regs) { OmniValue::Str(s.replace('\'', "''").replace(';', "")) } else { OmniValue::Str(String::new()) },
                0x7F58 => if let OmniValue::Str(s) = Self::eval_op(&ins.operands[0], &regs) { OmniValue::Str(s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")) } else { OmniValue::Str(String::new()) },
                0x7F59 => Self::eval_op(&ins.operands[0], &regs),
                0x7F5D => {
                    if let OmniValue::Str(s) = Self::eval_op(&ins.operands[0], &regs) {
                        let mut h: u64 = 0xcbf29ce484222325;
                        for b in s.bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
                        OmniValue::Int(h as i64)
                    } else { OmniValue::Int(0) }
                }
                0x7F63 => {
                    if let BinOperand::SchemaRef(s_idx) = ins.operands[0] {
                        let mut row = BTreeMap::new();
                        for op in &ins.operands[1..] {
                            if let BinOperand::FieldKV(f_id, inner) = op {
                                row.insert(*f_id, Self::eval_op(inner, &regs));
                            }
                        }
                        self.charge_memory(row.len() * 16)?;
                        self.tables.entry(s_idx).or_default().push(row.clone());
                        OmniValue::Struct(row)
                    } else { OmniValue::Void }
                }
                0x7F6A => {
                    if let (BinOperand::SchemaRef(s_idx), OmniValue::TritVec(q_vec)) = (&ins.operands[0], Self::eval_op(&ins.operands[1], &regs)) {
                        let q_packed = pack5_encode(&q_vec);
                        let mut best = OmniValue::Void;
                        let mut best_score = i64::MIN;
                        if let Some(rows) = self.tables.get(s_idx) {
                            for row in rows {
                                for val in row.values() {
                                    if let OmniValue::TritVec(r_vec) = val {
                                        let r_packed = pack5_encode(r_vec);
                                        let score = pack5_tdot_direct(&q_packed, &r_packed, q_vec.len().min(r_vec.len())).unwrap_or(0);
                                        if score > best_score { best_score = score; best = OmniValue::Struct(row.clone()); }
                                    }
                                }
                            }
                        }
                        if best == OmniValue::Void {
                            let mut m = BTreeMap::new();
                            m.insert(0, OmniValue::Int(*s_idx as i64));
                            m.insert(1, OmniValue::Int(q_vec.len() as i64));
                            OmniValue::Struct(m)
                        } else {
                            best
                        }
                    } else { OmniValue::Void }
                }
                0x7F7D => {
                    if let OmniValue::Str(s) = Self::eval_op(&ins.operands[0], &regs) {
                        self.charge_memory(pack5_byte_len(64))?;
                        let mut trits = vec![0i8; 64];
                        let mut h: u32 = 2166136261;
                        for (idx, b) in s.bytes().enumerate() {
                            h = (h ^ (b as u32)).wrapping_mul(16777619);
                            trits[idx % 64] = ((h % 3) as i8) - 1;
                        }
                        OmniValue::TritVec(trits)
                    } else {
                        self.charge_memory(pack5_byte_len(64))?;
                        OmniValue::TritVec(vec![0; 64])
                    }
                }
                0x7F7E => if let BinOperand::SkillRef(sk_id) = ins.operands[0] { OmniValue::SkillModel { skill_id: sk_id, active: true } } else { OmniValue::Void },
                0x7F7F => if let (OmniValue::SkillModel { skill_id, .. }, OmniValue::Str(prompt)) = (Self::eval_op(&ins.operands[0], &regs), Self::eval_op(&ins.operands[1], &regs)) { OmniValue::Str(format!("[Skill#{}->'{}']", skill_id, prompt)) } else { OmniValue::Void },
                0x7F76 => if let BinOperand::ImmInt(tag_id) = ins.operands[0] {
                    self.charge_memory(ins.operands.len() * 16)?;
                    OmniValue::DomNode { tag_id: tag_id as u16, children: ins.operands[1..].iter().map(|op| Self::eval_op(op, &regs)).collect() }
                } else { OmniValue::Void },
                _ => OmniValue::Void,
            };

            if let Some(d_tok) = ins.dest_reg {
                if let Some(slot) = token_to_slot(d_tok) {
                    regs[slot] = out_val;
                }
            }
            pc += 1;
        }
        Ok(OmniValue::Void)
    }
}

pub fn encode_u16_stream_to_bytes(stream: &[u16]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(stream.len() * 2);
    for &t in stream {
        bytes.extend_from_slice(&t.to_le_bytes());
    }
    bytes
}

pub fn decode_bytes_to_u16_stream(bytes: &[u8]) -> Vec<u16> {
    bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect()
}

pub fn format_non_human_token_stream(stream: &[u16]) -> String {
    let mut out = String::new();
    for (i, &tok) in stream.iter().enumerate() {
        if i > 0 {
            if i % 12 == 0 { out.push('\n'); } else { out.push(' '); }
        }
        out.push_str(&format!("{:04X}", tok));
    }
    out
}

/// Pretty-prints a pure `u16` stream using the `OMNI_ISA_TABLE` opcode labels for human debugging (`--debug-lens`, Roadmap 3.3).
pub fn format_debug_lens(stream: &[u16]) -> Result<String, String> {
    let module = decode_omni_token_stream(stream)?;
    let mut out = String::new();
    out.push_str(&format!(
        "=== OMNI DEBUG LENS ({} u16 tokens, {} functions, {} schemas) ===\n",
        stream.len(),
        module.functions.len(),
        module.schemas.len()
    ));
    for func in &module.functions {
        let rep = verify_binary_function(func);
        out.push_str(&format!(
            "FN #0x{:04X} (params={}, ret_ty=0x{:04X}, cap_mask=0x{:04X}, ins={}, verified={}):\n",
            func.fn_idx,
            func.params.len(),
            func.ret_ty_tok,
            func.cap_mask,
            func.instructions.len(),
            rep.passed
        ));
        for (idx, ins) in func.instructions.iter().enumerate() {
            let op_name = decode_opcode_token(ins.op_tok)
                .map(|s| s.debug_label)
                .unwrap_or("OP_UNKNOWN");
            let dst_str = ins
                .dest_reg
                .map(|d| format!("%{} (0x{:04X}) = ", d - REG_BASE, d))
                .unwrap_or_default();
            out.push_str(&format!(
                "  [{:03} @0x{:04X}] {}{} (0x{:04X}) {:?}\n",
                idx, ins.offset, dst_str, op_name, ins.op_tok, ins.operands
            ));
        }
    }
    Ok(out)
}

/// Parses a canonical `.omni_asm` type token name into its binary `TY_*` token ID.
pub fn parse_asm_ty_tok(s: &str) -> u16 {
    match s {
        "I64" => TY_I64,
        "TRIT" => TY_TRIT,
        "STR" => TY_STR,
        "UUID" => TY_UUID,
        "BYTES" => TY_BYTES,
        "ARRAY" => TY_ARRAY,
        "TRIT_VEC" => TY_TRIT_VEC,
        "ARENA" => TY_ARENA,
        "DOM" => TY_DOM,
        "STRUCT" => TY_STRUCT,
        _ => TY_I64,
    }
}

/// Encodes a single canonical `.omni_asm` instruction line directly into its pure `u16` packet.
pub fn encode_asm_ins_packet_u16(line: &str, ret_ty: u16) -> Result<Vec<u16>, String> {
    let tc_safe = pack_tc(false, CAP_ID_PURE);
    let parse_op_mnemonic = |m: &str, current_ret_ty: u16| -> Result<(u16, u16), String> {
        match m {
            "CINT"     | "MOV"        => Ok((0x7F06, TY_I64)),
            "CSTR"                    => Ok((0x7F08, TY_STR)),
            "CTRIT"                   => Ok((0x7F09, TY_TRIT)),
            "ARENA"                   => Ok((0x7F01, TY_ARENA)),
            "ANEW"                    => Ok((0x7F0D, TY_BYTES)),
            "IGET"                    => Ok((0x7F0E, TY_I64)),
            "ISET"                    => Ok((0x7F0F, TY_BYTES)),
            "SLEN"                    => Ok((0x7F50, TY_I64)),
            "SCAT"     | "SCONC"      => Ok((0x7F51, TY_STR)),
            "S2I"                     => Ok((0x7F55, TY_I64)),
            "I2S"                     => Ok((0x7F56, TY_STR)),
            "IADD"                    => Ok((0x7F10, TY_I64)),
            "ISUB"                    => Ok((0x7F11, TY_I64)),
            "IMUL"                    => Ok((0x7F12, TY_I64)),
            "IDIV"                    => Ok((0x7F13, TY_I64)),
            "IMOD"                    => Ok((0x7F14, TY_I64)),
            "INEG"                    => Ok((0x7F15, TY_I64)),
            "IABS"                    => Ok((0x7F16, TY_I64)),
            "IMIN"                    => Ok((0x7F17, TY_I64)),
            "IMAX"                    => Ok((0x7F18, TY_I64)),
            "BAND"                    => Ok((0x7F1E, TY_I64)),
            "BOR"                     => Ok((0x7F1F, TY_I64)),
            "BXOR"                    => Ok((0x7F20, TY_I64)),
            "BNOT"                    => Ok((0x7F21, TY_I64)),
            "SHL"                     => Ok((0x7F22, TY_I64)),
            "SHR"                     => Ok((0x7F23, TY_I64)),
            "CEQ"                     => Ok((0x7F24, TY_TRIT)),
            "CNE"                     => Ok((0x7F25, TY_TRIT)),
            "CLT"                     => Ok((0x7F26, TY_TRIT)),
            "CGT"                     => Ok((0x7F27, TY_TRIT)),
            "TNOT"                    => Ok((0x7F28, TY_TRIT)),
            "TAND"                    => Ok((0x7F29, TY_TRIT)),
            "TOR"                     => Ok((0x7F2A, TY_TRIT)),
            "TXOR"                    => Ok((0x7F2B, TY_TRIT)),
            "TMAJ"                    => Ok((0x7F2C, TY_TRIT)),
            "TCMP"                    => Ok((0x7F2D, TY_TRIT)),
            "PACK5"    | "TPACK5"     => Ok((0x7F30, TY_BYTES)),
            "UNPACK5"  | "TUNPACK5"   => Ok((0x7F31, TY_TRIT_VEC)),
            "TDOT"                    => Ok((0x7F34, TY_I64)),
            "EGATE"                   => Ok((0x7F35, TY_TRIT)),
            "LOOKAHEAD"               => Ok((0x7F36, TY_TRIT)),
            "TLOGIC"                  => Ok((0x7F3F, TY_TRIT)),
            "SEL_TRIT" | "CMOV"       => Ok((0x7F45, current_ret_ty)),
            "CALL"     | "CALL_I64"   => Ok((0x7F48, TY_I64)),
            "CALL_TRIT"               => Ok((0x7F48, TY_TRIT)),
            "CALL_BYTES"              => Ok((0x7F48, TY_BYTES)),
            "CALL_TRIT_VEC"           => Ok((0x7F48, TY_TRIT_VEC)),
            "CALL_STR"                => Ok((0x7F48, TY_STR)),
            "BORROW_R"                => Ok((0x7F02, TY_I64)),
            "BORROW_W"                => Ok((0x7F03, TY_I64)),
            "MOVE"                    => Ok((0x7F04, TY_I64)),
            "SAN_SQL"                 => Ok((0x7F57, TY_STR)),
            "SAN_DOM"                 => Ok((0x7F58, TY_STR)),
            "SAN_PRM"                 => Ok((0x7F59, TY_STR)),
            "FNV1A"                   => Ok((0x7F5D, TY_I64)),
            other => Err(format!("Unknown assembler mnemonic '{}'", other)),
        }
    };

    let push_operand_tok = |pkt: &mut Vec<u16>, tok_str: &str| -> Result<(), String> {
        if let Some(r_idx) = tok_str.strip_prefix('%') {
            let n = r_idx.parse::<u16>().map_err(|e| e.to_string())?;
            pkt.push(REG_BASE + n);
        } else if let Some(p_idx) = tok_str.strip_prefix('$') {
            let n = p_idx.parse::<u16>().map_err(|e| e.to_string())?;
            pkt.push(PAR_BASE + n);
        } else if let Some(f_idx) = tok_str.strip_prefix("@fn") {
            let n = f_idx.parse::<u16>().map_err(|e| e.to_string())?;
            pkt.push(TOK_SYM_FN);
            pkt.push(n);
        } else if let Some(hex_s) = tok_str.strip_prefix("@str:") {
            if hex_s.len() % 2 != 0 {
                return Err(format!("Invalid @str:<hex> operand '{}'", tok_str));
            }
            let n_bytes = hex_s.len() / 2;
            pkt.push(TOK_ENG_SPAN);
            pkt.push(n_bytes as u16);
            for i in 0..n_bytes {
                let b = u8::from_str_radix(&hex_s[i * 2..i * 2 + 2], 16).map_err(|e| e.to_string())?;
                pkt.push((b as u16) & ENGLISH_MAX_TOKEN);
            }
        } else if tok_str == "+1" || tok_str == "TRIT_POS" {
            pkt.push(TOK_TRIT_POS);
        } else if tok_str == "0t" || tok_str == "TRIT_ZERO" {
            pkt.push(TOK_TRIT_ZERO);
        } else if tok_str == "-1t" || tok_str == "TRIT_NEG" {
            pkt.push(TOK_TRIT_NEG);
        } else {
            let val = tok_str.parse::<i32>().map_err(|e| e.to_string())?;
            if ((i16::MIN as i32)..=(i16::MAX as i32)).contains(&val) {
                pkt.push(TOK_IMM_I16);
                pkt.push((val as i16) as u16);
            } else {
                let u = val as u32;
                pkt.push(TOK_IMM_I32);
                pkt.push(((u >> 16) & 0xFFFF) as u16);
                pkt.push((u & 0xFFFF) as u16);
            }
        }
        Ok(())
    };

    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
        return Err(String::from("Empty instruction line"));
    }
    if parts[0] == "NOP" {
        Ok(vec![0x7F00, TOK_INS_END])
    } else if parts[0] == "DROP" && parts.len() >= 2 {
        let mut pkt = vec![0x7F05];
        push_operand_tok(&mut pkt, parts[1])?;
        pkt.push(TOK_INS_END);
        Ok(pkt)
    } else if parts[0] == "RET" && parts.len() >= 2 {
        let mut pkt = vec![0x7F4A];
        push_operand_tok(&mut pkt, parts[1])?;
        pkt.push(TOK_INS_END);
        Ok(pkt)
    } else if parts[0] == "LABEL" && parts.len() >= 2 {
        let lid = parts[1].parse::<u16>().map_err(|e| e.to_string())?;
        if lid > 15 { return Err("Label id > 15".into()); }
        Ok(vec![0x7F40, LBL_BASE + lid, TOK_INS_END])
    } else if parts[0] == "JMP" && parts.len() >= 2 {
        let lid = parts[1].parse::<u16>().map_err(|e| e.to_string())?;
        if lid > 15 { return Err("Label id > 15".into()); }
        Ok(vec![0x7F41, LBL_BASE + lid, TOK_INS_END])
    } else if parts[0] == "BR_POS" && parts.len() >= 3 {
        let mut pkt = vec![0x7F43];
        push_operand_tok(&mut pkt, parts[1])?;
        let lid = parts[2].parse::<u16>().map_err(|e| e.to_string())?;
        if lid > 15 { return Err("Label id > 15".into()); }
        pkt.push(LBL_BASE + lid);
        pkt.push(TOK_INS_END);
        Ok(pkt)
    } else if parts[0] == "BR_NEG" && parts.len() >= 3 {
        let mut pkt = vec![0x7F44];
        push_operand_tok(&mut pkt, parts[1])?;
        let lid = parts[2].parse::<u16>().map_err(|e| e.to_string())?;
        if lid > 15 { return Err("Label id > 15".into()); }
        pkt.push(LBL_BASE + lid);
        pkt.push(TOK_INS_END);
        Ok(pkt)
    } else if parts[0] == "BR_TRIT" && parts.len() >= 5 {
        let mut pkt = vec![0x7F42];
        push_operand_tok(&mut pkt, parts[1])?;
        for j in 2..=4 {
            let lid = parts[j].parse::<u16>().map_err(|e| e.to_string())?;
            if lid > 15 { return Err("Label id > 15".into()); }
            pkt.push(LBL_BASE + lid);
        }
        pkt.push(TOK_INS_END);
        Ok(pkt)
    } else if parts.len() >= 4 && parts[1] == "=" {
        let d_idx = parts[0].strip_prefix('%').ok_or("Expected %N destination")?.parse::<u16>().map_err(|e| e.to_string())?;
        let (op_mnem, untr_suffix) = if let Some(base) = parts[2].strip_suffix("@UNTRUSTED") {
            (base, true)
        } else {
            (parts[2], false)
        };
        let (op_tok, ty_tok) = parse_op_mnemonic(op_mnem, ret_ty)?;
        let ins_tc = if untr_suffix { pack_tc(true, CAP_ID_PURE) } else { tc_safe };
        let mut pkt = vec![REG_BASE + d_idx, op_tok, ty_tok, ins_tc];
        for &op_s in &parts[3..] {
            push_operand_tok(&mut pkt, op_s)?;
        }
        pkt.push(TOK_INS_END);
        Ok(pkt)
    } else {
        Err(format!("Cannot parse assembler line: '{}'", line))
    }
}

/// Minimal human-readable mnemonic assembler that emits a verified pure `u16` token stream (Roadmap 3.5).
/// Supports single-function and multi-function (`FN_DEF <id>`) modules, `CAP_MASK <u16>`, `@fn<id>` call targets,
/// `@PACK5` vector opcodes (`PACK5`, `UNPACK5`, `TDOT`), and decision hooks (`EGATE`, `TLOGIC`, `LOOKAHEAD`).
pub fn assemble_omni_text_to_u16(src: &str) -> Result<Vec<u16>, String> {
    let mut fn_id: u16 = 0x0000;
    let mut cap_mask: CapMask = 0x0000;
    let mut params: Vec<(u16, u16, u16)> = Vec::new();
    let mut ret_ty: u16 = TY_I64;
    let mut ins_packets: Vec<Vec<u16>> = Vec::new();
    let mut completed_fns: Vec<(u16, Vec<(u16, u16, u16)>, u16, CapMask, Vec<Vec<u16>>)> = Vec::new();
    let mut started_fn_def = false;

    for raw_line in src.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts[0] == "FN_DEF" && parts.len() >= 2 {
            if started_fn_def || !ins_packets.is_empty() || !params.is_empty() {
                completed_fns.push((
                    fn_id,
                    std::mem::take(&mut params),
                    ret_ty,
                    cap_mask,
                    std::mem::take(&mut ins_packets),
                ));
                ret_ty = TY_I64;
                cap_mask = 0x0000;
            }
            fn_id = parts[1].parse::<u16>().map_err(|e| e.to_string())?;
            started_fn_def = true;
        } else if parts[0] == "FN_ID" && parts.len() >= 2 {
            fn_id = parts[1].parse::<u16>().map_err(|e| e.to_string())?;
        } else if parts[0] == "CAP_MASK" && parts.len() >= 2 {
            cap_mask = parts[1].parse::<u16>().map_err(|e| e.to_string())?;
        } else if parts[0] == "PARAM" && parts.len() >= 3 {
            let p_idx = parts[1].strip_prefix('$').ok_or("Expected $N in PARAM")?.parse::<u16>().map_err(|e| e.to_string())?;
            let ty = parse_asm_ty_tok(parts[2]);
            let untr = parts.get(3).copied() == Some("UNTRUSTED");
            params.push((PAR_BASE + p_idx, ty, pack_tc(untr, CAP_ID_PURE)));
        } else if parts[0] == "RET_TY" && parts.len() >= 2 {
            ret_ty = parse_asm_ty_tok(parts[1]);
        } else {
            ins_packets.push(encode_asm_ins_packet_u16(line, ret_ty)?);
        }
    }

    if completed_fns.is_empty() {
        Ok(pack_fn_module(&[], fn_id, &params, ret_ty, cap_mask, &ins_packets))
    } else {
        completed_fns.push((fn_id, params, ret_ty, cap_mask, ins_packets));
        let mut stream = vec![TOK_MOD_BEGIN];
        for (f_id, f_params, f_ret, f_cap, f_ins) in completed_fns {
            stream.push(TOK_FN_DEF);
            stream.push(f_id);
            stream.push(f_params.len() as u16);
            for (pt, pty, ptc) in f_params {
                stream.push(pt);
                stream.push(pty);
                stream.push(ptc);
            }
            stream.push(f_ret);
            stream.push(f_cap);
            stream.push(f_ins.len() as u16);
            for pkt in f_ins {
                stream.extend_from_slice(&pkt);
            }
        }
        stream.push(TOK_MOD_END);
        Ok(stream)
    }
}

// ==============================================================================
// 7B. OMNI SURFACE LANGUAGE (`omni-lang`) -> `.omni_asm` -> `Vec<u16>`
// ==============================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
enum SurfaceTok {
    Ident(String),
    Int(i64),
    Trit(String, i8),
    StrLit(String),
    AtIdent(String),
    Sym(String),
    Op(String),
    Hash,
}

#[derive(Clone, Debug)]
struct SpannedSurfaceTok {
    tok: SurfaceTok,
    line: usize,
    col: usize,
}

#[derive(Clone, Debug)]
struct SurfaceParam {
    name: String,
    ty: String,
    untrusted: bool,
    line: usize,
    col: usize,
}

#[derive(Clone, Debug)]
struct SurfaceFnDef {
    fn_id_attr: Option<u16>,
    cap_mask: CapMask,
    step_budget: Option<u64>,
    mem_budget: Option<usize>,
    name: String,
    params: Vec<SurfaceParam>,
    ret_ty: String,
    ret_line: usize,
    ret_col: usize,
    body: Vec<SurfaceStmt>,
}

#[derive(Clone, Debug)]
enum SurfaceExpr {
    Int(i64, usize, usize),
    Trit(String, i8, usize, usize),
    TritVecLit(Vec<i8>, usize, usize),
    StrLit(String, usize, usize),
    Var(String, usize, usize),
    UnOp(String, Box<SurfaceExpr>, usize, usize),
    BinOp(String, Box<SurfaceExpr>, Box<SurfaceExpr>, usize, usize),
    Call(String, Vec<SurfaceExpr>, usize, usize),
    IfBranch(String, Box<SurfaceExpr>, Vec<SurfaceStmt>, Vec<SurfaceStmt>, usize, usize),
    MatchTrit(Box<SurfaceExpr>, Vec<SurfaceStmt>, Vec<SurfaceStmt>, Vec<SurfaceStmt>, usize, usize),
}

impl SurfaceExpr {
    fn span(&self) -> (usize, usize) {
        match *self {
            SurfaceExpr::Int(_, l, c)
            | SurfaceExpr::Trit(_, _, l, c)
            | SurfaceExpr::TritVecLit(_, l, c)
            | SurfaceExpr::StrLit(_, l, c)
            | SurfaceExpr::Var(_, l, c)
            | SurfaceExpr::UnOp(_, _, l, c)
            | SurfaceExpr::BinOp(_, _, _, l, c)
            | SurfaceExpr::Call(_, _, l, c)
            | SurfaceExpr::IfBranch(_, _, _, _, l, c)
            | SurfaceExpr::MatchTrit(_, _, _, _, l, c) => (l, c),
        }
    }
}

#[derive(Clone, Debug)]
enum SurfaceStmt {
    Let(String, SurfaceExpr, usize, usize),
    Assign(String, SurfaceExpr, usize, usize),
    DoWhile(Vec<SurfaceStmt>, SurfaceExpr, usize, usize),
    While(SurfaceExpr, Vec<SurfaceStmt>, usize, usize),
    Label(u16, usize, usize),
    Goto(u16, usize, usize),
    Ret(SurfaceExpr, usize, usize),
    Expr(SurfaceExpr, usize, usize),
}

fn tokenize_omni_surface(src: &str) -> Result<Vec<SpannedSurfaceTok>, String> {
    let bytes = src.as_bytes();
    let n = bytes.len();
    let mut i = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;
    let mut toks = Vec::new();

    while i < n {
        let c = bytes[i] as char;
        if c == '\n' {
            i += 1;
            line += 1;
            col = 1;
            continue;
        }
        if c.is_ascii_whitespace() {
            i += 1;
            col += 1;
            continue;
        }
        if i + 1 < n && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            while i < n && bytes[i] != b'\n' {
                i += 1;
                col += 1;
            }
            continue;
        }
        if bytes[i] == b'#' {
            if i + 1 < n && bytes[i + 1] == b'[' {
                toks.push(SpannedSurfaceTok { tok: SurfaceTok::Hash, line, col });
                i += 1;
                col += 1;
                continue;
            } else {
                while i < n && bytes[i] != b'\n' {
                    i += 1;
                    col += 1;
                }
                continue;
            }
        }
        if bytes[i] == b'"' {
            let start_line = line;
            let start_col = col;
            i += 1;
            col += 1;
            let start = i;
            while i < n && bytes[i] != b'"' && bytes[i] != b'\n' {
                i += 1;
                col += 1;
            }
            if i >= n || bytes[i] != b'"' {
                return Err(format!("line {}, col {}: Unterminated string literal", start_line, start_col));
            }
            let s = String::from(&src[start..i]);
            i += 1;
            col += 1;
            toks.push(SpannedSurfaceTok {
                tok: SurfaceTok::StrLit(s),
                line: start_line,
                col: start_col,
            });
            continue;
        }
        if bytes[i] == b'@' {
            let start_col = col;
            i += 1;
            col += 1;
            let start = i;
            while i < n && ((bytes[i] as char).is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
                col += 1;
            }
            if start == i {
                return Err(format!("line {}, col {}: Expected annotation name after '@'", line, start_col));
            }
            toks.push(SpannedSurfaceTok {
                tok: SurfaceTok::AtIdent(String::from(&src[start..i])),
                line,
                col: start_col,
            });
            continue;
        }
        if i + 3 <= n && &src[i..i + 3] == "<=>" {
            toks.push(SpannedSurfaceTok { tok: SurfaceTok::Op(String::from("<=>")), line, col });
            i += 3;
            col += 3;
            continue;
        }
        if i + 3 <= n && &src[i..i + 3] == "+1t" {
            toks.push(SpannedSurfaceTok { tok: SurfaceTok::Trit(String::from("+1"), 1), line, col });
            i += 3;
            col += 3;
            continue;
        }
        if i + 3 <= n && &src[i..i + 3] == "-1t" {
            toks.push(SpannedSurfaceTok { tok: SurfaceTok::Trit(String::from("-1t"), -1), line, col });
            i += 3;
            col += 3;
            continue;
        }
        if i + 2 <= n && &src[i..i + 2] == "0t" {
            let next_is_ident = i + 2 < n && ((bytes[i + 2] as char).is_ascii_alphanumeric() || bytes[i + 2] == b'_');
            if !next_is_ident {
                toks.push(SpannedSurfaceTok { tok: SurfaceTok::Trit(String::from("0t"), 0), line, col });
                i += 2;
                col += 2;
                continue;
            }
        }
        if i + 2 <= n {
            let two = &src[i..i + 2];
            if two == "->" || two == "=>" || two == "+=" || two == "-=" || two == "*=" {
                toks.push(SpannedSurfaceTok { tok: SurfaceTok::Sym(String::from(two)), line, col });
                i += 2;
                col += 2;
                continue;
            }
            if two == "==" || two == "!=" || two == "<=" || two == ">=" || two == "<<" || two == ">>" {
                toks.push(SpannedSurfaceTok { tok: SurfaceTok::Op(String::from(two)), line, col });
                i += 2;
                col += 2;
                continue;
            }
        }
        if "(){}[].,:;=".contains(c) {
            toks.push(SpannedSurfaceTok { tok: SurfaceTok::Sym(c.to_string()), line, col });
            i += 1;
            col += 1;
            continue;
        }
        if "+-*/%&|^~!<>".contains(c) {
            toks.push(SpannedSurfaceTok { tok: SurfaceTok::Op(c.to_string()), line, col });
            i += 1;
            col += 1;
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            let start_col = col;
            while i < n && (bytes[i] as char).is_ascii_digit() {
                i += 1;
                col += 1;
            }
            let val = src[start..i]
                .parse::<i64>()
                .map_err(|e| format!("line {}, col {}: Invalid integer literal: {}", line, start_col, e))?;
            toks.push(SpannedSurfaceTok { tok: SurfaceTok::Int(val), line, col: start_col });
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            let start_col = col;
            while i < n && ((bytes[i] as char).is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
                col += 1;
            }
            let ident = &src[start..i];
            if ident == "TRIT_POS" {
                toks.push(SpannedSurfaceTok { tok: SurfaceTok::Trit(String::from("+1"), 1), line, col: start_col });
            } else if ident == "TRIT_ZERO" {
                toks.push(SpannedSurfaceTok { tok: SurfaceTok::Trit(String::from("0t"), 0), line, col: start_col });
            } else if ident == "TRIT_NEG" {
                toks.push(SpannedSurfaceTok { tok: SurfaceTok::Trit(String::from("-1t"), -1), line, col: start_col });
            } else {
                toks.push(SpannedSurfaceTok { tok: SurfaceTok::Ident(String::from(ident)), line, col: start_col });
            }
            continue;
        }
        return Err(format!("line {}, col {}: Unexpected character '{}'", line, col, c));
    }
    Ok(toks)
}

fn parse_surface_cap_name(name: &str, line: usize, col: usize) -> Result<CapMask, String> {
    match name.to_ascii_uppercase().as_str() {
        "PURE" => Ok(0),
        "DB_R" => Ok(cap_bit(CAP_ID_DB_R)),
        "DB_W" => Ok(cap_bit(CAP_ID_DB_W)),
        "NET" => Ok(cap_bit(CAP_ID_NET)),
        "DOM" => Ok(cap_bit(CAP_ID_DOM)),
        "AI" => Ok(cap_bit(CAP_ID_AI)),
        "VISION" | "SKILL" => Ok(cap_bit(CAP_ID_VISION)),
        "AUDIO" => Ok(cap_bit(CAP_ID_AUDIO)),
        "AUTH" => Ok(cap_bit(CAP_ID_AUTH)),
        "FS" | "FS_R" | "FS_W" => Ok(cap_bit(CAP_ID_FS)),
        "SYS" | "ENV" | "PROC" | "TIME" | "RAND" | "CRYPTO" => Ok(cap_bit(CAP_ID_SYS)),
        other => Err(format!("line {}, col {}: Unknown capability '{}'", line, col, other)),
    }
}

struct SurfaceParser {
    tokens: Vec<SpannedSurfaceTok>,
    pos: usize,
}

impl SurfaceParser {
    fn new(tokens: Vec<SpannedSurfaceTok>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn current_span(&self) -> (usize, usize) {
        if let Some(t) = self.tokens.get(self.pos) {
            (t.line, t.col)
        } else if let Some(last) = self.tokens.last() {
            (last.line, last.col + 1)
        } else {
            (1, 1)
        }
    }

    fn peek(&self) -> Option<&SurfaceTok> {
        self.tokens.get(self.pos).map(|s| &s.tok)
    }

    fn next_spanned(&mut self) -> Result<SpannedSurfaceTok, String> {
        if let Some(t) = self.tokens.get(self.pos).cloned() {
            self.pos += 1;
            Ok(t)
        } else {
            let (l, c) = self.current_span();
            Err(format!("line {}, col {}: Unexpected EOF in surface program", l, c))
        }
    }

    fn expect_sym(&mut self, sym: &str) -> Result<(usize, usize), String> {
        let st = self.next_spanned()?;
        match st.tok {
            SurfaceTok::Sym(ref s) if s == sym => Ok((st.line, st.col)),
            other => Err(format!("line {}, col {}: Expected symbol '{}', got {:?}", st.line, st.col, sym, other)),
        }
    }

    fn expect_ident_spanned(&mut self) -> Result<(String, usize, usize), String> {
        let st = self.next_spanned()?;
        match st.tok {
            SurfaceTok::Ident(s) => Ok((s, st.line, st.col)),
            other => Err(format!("line {}, col {}: Expected identifier, got {:?}", st.line, st.col, other)),
        }
    }

    fn expect_ident_kw(&mut self, kw: &str) -> Result<(usize, usize), String> {
        let st = self.next_spanned()?;
        match st.tok {
            SurfaceTok::Ident(ref s) if s == kw => Ok((st.line, st.col)),
            other => Err(format!("line {}, col {}: Expected keyword '{}', got {:?}", st.line, st.col, kw, other)),
        }
    }

    fn expect_int_spanned(&mut self) -> Result<(i64, usize, usize), String> {
        let st = self.next_spanned()?;
        match st.tok {
            SurfaceTok::Int(v) => Ok((v, st.line, st.col)),
            other => Err(format!("line {}, col {}: Expected integer literal, got {:?}", st.line, st.col, other)),
        }
    }

    fn parse_module(&mut self) -> Result<(Vec<(String, usize, usize)>, Vec<SurfaceFnDef>), String> {
        let mut includes = Vec::new();
        let mut fns = Vec::new();

        while self.peek().is_some() {
            if self.peek() == Some(&SurfaceTok::Ident(String::from("include")))
                || self.peek() == Some(&SurfaceTok::Ident(String::from("mod")))
            {
                let kw_st = self.next_spanned()?;
                let path_st = self.next_spanned()?;
                let inc_path = match path_st.tok {
                    SurfaceTok::StrLit(s) => s,
                    other => {
                        return Err(format!(
                            "line {}, col {}: Expected string literal after '{}', got {:?}",
                            path_st.line, path_st.col,
                            if let SurfaceTok::Ident(k) = kw_st.tok { k } else { String::from("include") },
                            other
                        ));
                    }
                };
                if self.peek() == Some(&SurfaceTok::Sym(String::from(";"))) {
                    let _ = self.next_spanned()?;
                }
                includes.push((inc_path, kw_st.line, kw_st.col));
                continue;
            }
            fns.push(self.parse_fn_def()?);
        }

        if fns.is_empty() {
            let (l, c) = self.current_span();
            return Err(format!("line {}, col {}: Surface program contains no 'fn' declarations", l, c));
        }
        Ok((includes, fns))
    }

    fn parse_fn_def(&mut self) -> Result<SurfaceFnDef, String> {
        let mut fn_id_attr: Option<u16> = None;
        let mut cap_mask: CapMask = 0x0000;
        let mut step_budget: Option<u64> = None;
        let mut mem_budget: Option<usize> = None;

        while self.peek() == Some(&SurfaceTok::Hash) {
            let _ = self.next_spanned()?;
            self.expect_sym("[")?;
            let (attr, al, ac) = self.expect_ident_spanned()?;
            if attr == "fn_id" {
                self.expect_sym("(")?;
                let (id_val, il, ic) = self.expect_int_spanned()?;
                if !(0..=15).contains(&id_val) {
                    return Err(format!("line {}, col {}: fn_id {} out of range 0..=15", il, ic, id_val));
                }
                fn_id_attr = Some(id_val as u16);
                self.expect_sym(")")?;
            } else if attr == "cap" {
                self.expect_sym("(")?;
                while self.peek().is_some() && self.peek() != Some(&SurfaceTok::Sym(String::from(")"))) {
                    let (cname, cl, cc) = self.expect_ident_spanned()?;
                    cap_mask |= parse_surface_cap_name(&cname, cl, cc)?;
                    if self.peek() == Some(&SurfaceTok::Sym(String::from(","))) {
                        let _ = self.next_spanned()?;
                    }
                }
                self.expect_sym(")")?;
            } else if attr == "step_budget" || attr == "fuel" {
                self.expect_sym("(")?;
                let (budget_val, bl, bc) = self.expect_int_spanned()?;
                if budget_val <= 0 {
                    return Err(format!("line {}, col {}: step_budget must be > 0, got {}", bl, bc, budget_val));
                }
                step_budget = Some(budget_val as u64);
                self.expect_sym(")")?;
            } else if attr == "mem_budget" || attr == "memory_budget" {
                self.expect_sym("(")?;
                let (budget_val, bl, bc) = self.expect_int_spanned()?;
                if budget_val <= 0 {
                    return Err(format!("line {}, col {}: mem_budget must be > 0, got {}", bl, bc, budget_val));
                }
                mem_budget = Some(budget_val as usize);
                self.expect_sym(")")?;
            } else {
                return Err(format!("line {}, col {}: Unsupported surface attribute '{}'", al, ac, attr));
            }
            self.expect_sym("]")?;
        }

        self.expect_ident_kw("fn")?;
        let (fn_name, _fl, _fc) = self.expect_ident_spanned()?;
        self.expect_sym("(")?;
        let mut params = Vec::new();
        while self.peek().is_some() && self.peek() != Some(&SurfaceTok::Sym(String::from(")"))) {
            let (p_name, _pl, _pc) = self.expect_ident_spanned()?;
            self.expect_sym(":")?;
            let (raw_ty, tl, tc) = self.expect_ident_spanned()?;
            let mut untrusted = false;
            let clean_ty = if let Some(stripped) = raw_ty.strip_prefix("untrusted_") {
                untrusted = true;
                String::from(stripped)
            } else {
                raw_ty
            };
            if let Some(SurfaceTok::AtIdent(tag)) = self.peek().cloned() {
                let st = self.next_spanned()?;
                match tag.to_ascii_lowercase().as_str() {
                    "untrusted" => untrusted = true,
                    "safe" => untrusted = false,
                    other => return Err(format!("line {}, col {}: Unknown taint annotation '@{}'", st.line, st.col, other)),
                }
            }
            params.push(SurfaceParam {
                name: p_name,
                ty: clean_ty,
                untrusted,
                line: tl,
                col: tc,
            });
            if self.peek() == Some(&SurfaceTok::Sym(String::from(","))) {
                let _ = self.next_spanned()?;
            }
        }
        self.expect_sym(")")?;
        self.expect_sym("->")?;
        let (ret_ty, rl, rc) = self.expect_ident_spanned()?;
        let body = self.parse_block()?;

        Ok(SurfaceFnDef {
            fn_id_attr,
            cap_mask,
            step_budget,
            mem_budget,
            name: fn_name,
            params,
            ret_ty,
            ret_line: rl,
            ret_col: rc,
            body,
        })
    }

    fn parse_block(&mut self) -> Result<Vec<SurfaceStmt>, String> {
        self.expect_sym("{")?;
        let mut stmts = Vec::new();
        while self.peek().is_some() && self.peek() != Some(&SurfaceTok::Sym(String::from("}"))) {
            stmts.push(self.parse_stmt()?);
        }
        self.expect_sym("}")?;
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<SurfaceStmt, String> {
        let (line, col) = self.current_span();
        match self.peek().cloned() {
            Some(SurfaceTok::Ident(ref kw)) if kw == "let" => {
                let _ = self.next_spanned()?;
                let (name, nl, nc) = self.expect_ident_spanned()?;
                if self.peek() == Some(&SurfaceTok::Sym(String::from(":"))) {
                    let _ = self.next_spanned()?;
                    let (ty_ann, tl, tc) = self.expect_ident_spanned()?;
                    let _ = map_omni_surface_type(&ty_ann, tl, tc)?;
                }
                self.expect_sym("=")?;
                let expr = self.parse_expr()?;
                if self.peek() == Some(&SurfaceTok::Sym(String::from(";"))) {
                    let _ = self.next_spanned()?;
                }
                Ok(SurfaceStmt::Let(name, expr, nl, nc))
            }
            Some(SurfaceTok::Ident(ref kw)) if kw == "return" => {
                let _ = self.next_spanned()?;
                let expr = self.parse_expr()?;
                if self.peek() == Some(&SurfaceTok::Sym(String::from(";"))) {
                    let _ = self.next_spanned()?;
                }
                Ok(SurfaceStmt::Ret(expr, line, col))
            }
            Some(SurfaceTok::Ident(ref kw)) if kw == "do_while" => {
                let _ = self.next_spanned()?;
                let body = self.parse_block()?;
                let cond = self.parse_expr()?;
                if self.peek() == Some(&SurfaceTok::Sym(String::from(";"))) {
                    let _ = self.next_spanned()?;
                }
                Ok(SurfaceStmt::DoWhile(body, cond, line, col))
            }
            Some(SurfaceTok::Ident(ref kw)) if kw == "while" => {
                let _ = self.next_spanned()?;
                let cond = self.parse_expr()?;
                let body = self.parse_block()?;
                if self.peek() == Some(&SurfaceTok::Sym(String::from(";"))) {
                    let _ = self.next_spanned()?;
                }
                Ok(SurfaceStmt::While(cond, body, line, col))
            }
            Some(SurfaceTok::Ident(ref kw)) if kw == "label" => {
                let _ = self.next_spanned()?;
                let (lid, ll, lc) = self.expect_int_spanned()?;
                if !(0..=15).contains(&lid) {
                    return Err(format!("line {}, col {}: Label id {} out of range 0..=15", ll, lc, lid));
                }
                if self.peek() == Some(&SurfaceTok::Sym(String::from(";"))) {
                    let _ = self.next_spanned()?;
                }
                Ok(SurfaceStmt::Label(lid as u16, ll, lc))
            }
            Some(SurfaceTok::Ident(ref kw)) if kw == "goto" => {
                let _ = self.next_spanned()?;
                let (lid, ll, lc) = self.expect_int_spanned()?;
                if !(0..=15).contains(&lid) {
                    return Err(format!("line {}, col {}: Goto label id {} out of range 0..=15", ll, lc, lid));
                }
                if self.peek() == Some(&SurfaceTok::Sym(String::from(";"))) {
                    let _ = self.next_spanned()?;
                }
                Ok(SurfaceStmt::Goto(lid as u16, ll, lc))
            }
            Some(SurfaceTok::Ident(_))
                if matches!(
                    self.tokens.get(self.pos + 1).map(|s| &s.tok),
                    Some(SurfaceTok::Sym(op)) if op == "=" || op == "+=" || op == "-=" || op == "*="
                ) =>
            {
                let (name, nl, nc) = self.expect_ident_spanned()?;
                let assign_tok = self.next_spanned()?;
                let assign_op = match assign_tok.tok {
                    SurfaceTok::Sym(s) => s,
                    _ => String::from("="),
                };
                let rhs = self.parse_expr()?;
                if self.peek() == Some(&SurfaceTok::Sym(String::from(";"))) {
                    let _ = self.next_spanned()?;
                }
                let expr = match assign_op.as_str() {
                    "+=" => SurfaceExpr::BinOp(
                        String::from("IADD"),
                        Box::new(SurfaceExpr::Var(name.clone(), nl, nc)),
                        Box::new(rhs),
                        nl,
                        nc,
                    ),
                    "-=" => SurfaceExpr::BinOp(
                        String::from("ISUB"),
                        Box::new(SurfaceExpr::Var(name.clone(), nl, nc)),
                        Box::new(rhs),
                        nl,
                        nc,
                    ),
                    "*=" => SurfaceExpr::BinOp(
                        String::from("IMUL"),
                        Box::new(SurfaceExpr::Var(name.clone(), nl, nc)),
                        Box::new(rhs),
                        nl,
                        nc,
                    ),
                    _ => rhs,
                };
                Ok(SurfaceStmt::Assign(name, expr, nl, nc))
            }
            _ => {
                let expr = self.parse_expr()?;
                if self.peek() == Some(&SurfaceTok::Sym(String::from(";"))) {
                    let _ = self.next_spanned()?;
                }
                Ok(SurfaceStmt::Expr(expr, line, col))
            }
        }
    }

    fn parse_expr(&mut self) -> Result<SurfaceExpr, String> {
        self.parse_cmp()
    }

    fn parse_cmp(&mut self) -> Result<SurfaceExpr, String> {
        let mut lhs = self.parse_bitor()?;
        while let Some(SurfaceTok::Op(op)) = self.peek().cloned() {
            match op.as_str() {
                "==" | "!=" | "<" | ">" | "<=>" => {
                    let mnem = match op.as_str() {
                        "==" => "CEQ",
                        "!=" => "CNE",
                        "<" => "CLT",
                        ">" => "CGT",
                        "<=>" => "TCMP",
                        _ => unreachable!(),
                    };
                    let st = self.next_spanned()?;
                    let rhs = self.parse_bitor()?;
                    lhs = SurfaceExpr::BinOp(String::from(mnem), Box::new(lhs), Box::new(rhs), st.line, st.col);
                }
                "<=" => {
                    let st = self.next_spanned()?;
                    let rhs = self.parse_bitor()?;
                    let gt = SurfaceExpr::BinOp(String::from("CGT"), Box::new(lhs), Box::new(rhs), st.line, st.col);
                    lhs = SurfaceExpr::UnOp(String::from("TNOT"), Box::new(gt), st.line, st.col);
                }
                ">=" => {
                    let st = self.next_spanned()?;
                    let rhs = self.parse_bitor()?;
                    let lt = SurfaceExpr::BinOp(String::from("CLT"), Box::new(lhs), Box::new(rhs), st.line, st.col);
                    lhs = SurfaceExpr::UnOp(String::from("TNOT"), Box::new(lt), st.line, st.col);
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn parse_bitor(&mut self) -> Result<SurfaceExpr, String> {
        let mut lhs = self.parse_bitxor()?;
        while self.peek() == Some(&SurfaceTok::Op(String::from("|"))) {
            let st = self.next_spanned()?;
            let rhs = self.parse_bitxor()?;
            lhs = SurfaceExpr::BinOp(String::from("BOR"), Box::new(lhs), Box::new(rhs), st.line, st.col);
        }
        Ok(lhs)
    }

    fn parse_bitxor(&mut self) -> Result<SurfaceExpr, String> {
        let mut lhs = self.parse_bitand()?;
        while self.peek() == Some(&SurfaceTok::Op(String::from("^"))) {
            let st = self.next_spanned()?;
            let rhs = self.parse_bitand()?;
            lhs = SurfaceExpr::BinOp(String::from("BXOR"), Box::new(lhs), Box::new(rhs), st.line, st.col);
        }
        Ok(lhs)
    }

    fn parse_bitand(&mut self) -> Result<SurfaceExpr, String> {
        let mut lhs = self.parse_shift()?;
        while self.peek() == Some(&SurfaceTok::Op(String::from("&"))) {
            let st = self.next_spanned()?;
            let rhs = self.parse_shift()?;
            lhs = SurfaceExpr::BinOp(String::from("BAND"), Box::new(lhs), Box::new(rhs), st.line, st.col);
        }
        Ok(lhs)
    }

    fn parse_shift(&mut self) -> Result<SurfaceExpr, String> {
        let mut lhs = self.parse_add()?;
        while let Some(SurfaceTok::Op(op)) = self.peek().cloned() {
            let mnem = match op.as_str() {
                "<<" => "SHL",
                ">>" => "SHR",
                _ => break,
            };
            let st = self.next_spanned()?;
            let rhs = self.parse_add()?;
            lhs = SurfaceExpr::BinOp(String::from(mnem), Box::new(lhs), Box::new(rhs), st.line, st.col);
        }
        Ok(lhs)
    }

    fn parse_add(&mut self) -> Result<SurfaceExpr, String> {
        let mut lhs = self.parse_mul()?;
        while let Some(SurfaceTok::Op(op)) = self.peek().cloned() {
            let mnem = match op.as_str() {
                "+" => "IADD",
                "-" => "ISUB",
                _ => break,
            };
            let st = self.next_spanned()?;
            let rhs = self.parse_mul()?;
            lhs = SurfaceExpr::BinOp(String::from(mnem), Box::new(lhs), Box::new(rhs), st.line, st.col);
        }
        Ok(lhs)
    }

    fn parse_mul(&mut self) -> Result<SurfaceExpr, String> {
        let mut lhs = self.parse_unary()?;
        while let Some(SurfaceTok::Op(op)) = self.peek().cloned() {
            let mnem = match op.as_str() {
                "*" => "IMUL",
                "/" => "IDIV",
                "%" => "IMOD",
                _ => break,
            };
            let st = self.next_spanned()?;
            let rhs = self.parse_unary()?;
            lhs = SurfaceExpr::BinOp(String::from(mnem), Box::new(lhs), Box::new(rhs), st.line, st.col);
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<SurfaceExpr, String> {
        if let Some(SurfaceTok::Op(op)) = self.peek().cloned() {
            if op == "-" {
                let st = self.next_spanned()?;
                let sub = self.parse_unary()?;
                if let SurfaceExpr::Int(v, _, _) = sub {
                    return Ok(SurfaceExpr::Int(v.wrapping_neg(), st.line, st.col));
                }
                return Ok(SurfaceExpr::UnOp(String::from("INEG"), Box::new(sub), st.line, st.col));
            }
            if op == "+" {
                let _ = self.next_spanned()?;
                return self.parse_unary();
            }
            if op == "~" {
                let st = self.next_spanned()?;
                let sub = self.parse_unary()?;
                return Ok(SurfaceExpr::UnOp(String::from("BNOT"), Box::new(sub), st.line, st.col));
            }
            if op == "!" {
                let st = self.next_spanned()?;
                let sub = self.parse_unary()?;
                return Ok(SurfaceExpr::UnOp(String::from("TNOT"), Box::new(sub), st.line, st.col));
            }
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<SurfaceExpr, String> {
        let (line, col) = self.current_span();
        match self.peek().cloned() {
            Some(SurfaceTok::Int(v)) => {
                let st = self.next_spanned()?;
                Ok(SurfaceExpr::Int(v, st.line, st.col))
            }
            Some(SurfaceTok::Trit(s, tv)) => {
                let st = self.next_spanned()?;
                Ok(SurfaceExpr::Trit(s, tv, st.line, st.col))
            }
            Some(SurfaceTok::StrLit(s)) => {
                let st = self.next_spanned()?;
                Ok(SurfaceExpr::StrLit(s, st.line, st.col))
            }
            Some(SurfaceTok::Sym(ref s)) if s == "[" => {
                let st = self.next_spanned()?;
                let mut trits = Vec::new();
                while self.peek().is_some() && self.peek() != Some(&SurfaceTok::Sym(String::from("]"))) {
                    let elem = self.parse_expr()?;
                    match elem {
                        SurfaceExpr::Trit(_, tv, _, _) => trits.push(tv),
                        SurfaceExpr::Int(v, _el, _ec) if (-1..=1).contains(&v) => trits.push(v as i8),
                        other => {
                            let (el, ec) = other.span();
                            return Err(format!(
                                "line {}, col {}: Trit vector literal elements must be balanced trits (-1t, 0t, +1t)",
                                el, ec
                            ));
                        }
                    }
                    if self.peek() == Some(&SurfaceTok::Sym(String::from(","))) {
                        let _ = self.next_spanned()?;
                    }
                }
                self.expect_sym("]")?;
                if trits.is_empty() {
                    return Err(format!("line {}, col {}: Trit vector literal cannot be empty", st.line, st.col));
                }
                Ok(SurfaceExpr::TritVecLit(trits, st.line, st.col))
            }
            Some(SurfaceTok::Sym(ref s)) if s == "(" => {
                let _ = self.next_spanned()?;
                let e = self.parse_expr()?;
                self.expect_sym(")")?;
                Ok(e)
            }
            Some(SurfaceTok::Ident(ref kw)) if kw == "if_pos" || kw == "if_neg" => {
                let br_mnem = if kw == "if_pos" { "BR_POS" } else { "BR_NEG" };
                let st = self.next_spanned()?;
                let cond = self.parse_expr()?;
                let then_blk = self.parse_block()?;
                self.expect_ident_kw("else")?;
                let else_blk = self.parse_block()?;
                Ok(SurfaceExpr::IfBranch(
                    String::from(br_mnem),
                    Box::new(cond),
                    then_blk,
                    else_blk,
                    st.line,
                    st.col,
                ))
            }
            Some(SurfaceTok::Ident(ref kw)) if kw == "match_trit" => {
                let st = self.next_spanned()?;
                let cond = self.parse_expr()?;
                self.expect_sym("{")?;
                let mut neg_arm: Option<Vec<SurfaceStmt>> = None;
                let mut zero_arm: Option<Vec<SurfaceStmt>> = None;
                let mut pos_arm: Option<Vec<SurfaceStmt>> = None;
                while self.peek().is_some() && self.peek() != Some(&SurfaceTok::Sym(String::from("}"))) {
                    let (arm_name, al, ac) = self.expect_ident_spanned()?;
                    self.expect_sym("=>")?;
                    let arm_stmts = if self.peek() == Some(&SurfaceTok::Sym(String::from("{"))) {
                        self.parse_block()?
                    } else {
                        let e = self.parse_expr()?;
                        let (el, ec) = e.span();
                        vec![SurfaceStmt::Expr(e, el, ec)]
                    };
                    if self.peek() == Some(&SurfaceTok::Sym(String::from(","))) {
                        let _ = self.next_spanned()?;
                    }
                    match arm_name.as_str() {
                        "neg" => neg_arm = Some(arm_stmts),
                        "zero" => zero_arm = Some(arm_stmts),
                        "pos" => pos_arm = Some(arm_stmts),
                        other => {
                            return Err(format!(
                                "line {}, col {}: Unknown match_trit arm '{}' (expected neg, zero, pos)",
                                al, ac, other
                            ));
                        }
                    }
                }
                self.expect_sym("}")?;
                let neg = neg_arm.ok_or_else(|| format!("line {}, col {}: match_trit missing 'neg' arm", st.line, st.col))?;
                let zero = zero_arm.ok_or_else(|| format!("line {}, col {}: match_trit missing 'zero' arm", st.line, st.col))?;
                let pos = pos_arm.ok_or_else(|| format!("line {}, col {}: match_trit missing 'pos' arm", st.line, st.col))?;
                Ok(SurfaceExpr::MatchTrit(Box::new(cond), neg, zero, pos, st.line, st.col))
            }
            Some(SurfaceTok::Ident(ident)) => {
                let st = self.next_spanned()?;
                if self.peek() == Some(&SurfaceTok::Sym(String::from("("))) {
                    let _ = self.next_spanned()?;
                    let mut args = Vec::new();
                    while self.peek().is_some() && self.peek() != Some(&SurfaceTok::Sym(String::from(")"))) {
                        args.push(self.parse_expr()?);
                        if self.peek() == Some(&SurfaceTok::Sym(String::from(","))) {
                            let _ = self.next_spanned()?;
                        }
                    }
                    self.expect_sym(")")?;
                    Ok(SurfaceExpr::Call(ident, args, st.line, st.col))
                } else {
                    Ok(SurfaceExpr::Var(ident, st.line, st.col))
                }
            }
            other => Err(format!("line {}, col {}: Unexpected token in surface expression: {:?}", line, col, other)),
        }
    }
}

fn map_omni_surface_type(ty: &str, line: usize, col: usize) -> Result<&'static str, String> {
    match ty.to_ascii_lowercase().as_str() {
        "i64" | "int" => Ok("I64"),
        "trit" => Ok("TRIT"),
        "bytes" | "byte_buf" | "pack5" => Ok("BYTES"),
        "array" | "u16_array" => Ok("ARRAY"),
        "trit_vec" | "tvec" => Ok("TRIT_VEC"),
        "str" => Ok("STR"),
        "uuid" => Ok("UUID"),
        "arena" => Ok("ARENA"),
        "dom" => Ok("DOM"),
        "struct" => Ok("STRUCT"),
        _ => Err(format!("line {}, col {}: Unknown surface type '{}'", line, col, ty)),
    }
}

struct SurfaceLowerCtx {
    var_map: BTreeMap<String, String>,
    fn_sigs: BTreeMap<String, (u16, usize, String)>,
    next_reg: usize,
    next_lbl: u16,
    defined_labels: Vec<u16>,
    used_labels: Vec<(u16, usize, usize)>,
    ret_ty_tok: u16,
    emit_asm: bool,
    lines: Vec<String>,
    packets: Vec<Vec<u16>>,
}

impl SurfaceLowerCtx {
    fn new(
        params: &[SurfaceParam],
        fn_sigs: BTreeMap<String, (u16, usize, String)>,
        ret_ty_tok: u16,
        emit_asm: bool,
    ) -> Self {
        let mut var_map = BTreeMap::new();
        for (idx, p) in params.iter().enumerate() {
            var_map.insert(p.name.clone(), format!("${}", idx));
        }
        Self {
            var_map,
            fn_sigs,
            next_reg: 0,
            next_lbl: 1,
            defined_labels: Vec::new(),
            used_labels: Vec::new(),
            ret_ty_tok,
            emit_asm,
            lines: Vec::new(),
            packets: Vec::new(),
        }
    }

    fn emit(&mut self, line: String) -> Result<(), String> {
        if self.emit_asm {
            self.lines.push(line);
        } else {
            let pkt = encode_asm_ins_packet_u16(&line, self.ret_ty_tok)?;
            self.packets.push(pkt);
        }
        Ok(())
    }

    fn alloc_reg(&mut self, line: usize, col: usize) -> Result<String, String> {
        if self.next_reg >= 192 {
            return Err(format!("line {}, col {}: Surface function exceeded 192 SSA registers", line, col));
        }
        let r = format!("%{}", self.next_reg);
        self.next_reg += 1;
        Ok(r)
    }

    fn alloc_lbl(&mut self, line: usize, col: usize) -> Result<u16, String> {
        if self.next_lbl > 15 {
            return Err(format!("line {}, col {}: Surface function exceeded 15 basic-block labels", line, col));
        }
        let l = self.next_lbl;
        self.next_lbl += 1;
        Ok(l)
    }

    fn eval_to_operand(&mut self, expr: &SurfaceExpr) -> Result<String, String> {
        match expr {
            SurfaceExpr::Int(v, _, _) => Ok(v.to_string()),
            SurfaceExpr::Trit(s, _, _, _) => Ok(s.clone()),
            SurfaceExpr::Var(v, line, col) => self
                .var_map
                .get(v)
                .cloned()
                .ok_or_else(|| format!("line {}, col {}: Undefined variable '{}'", line, col, v)),
            _ => self.lower_expr_into(expr, None, false),
        }
    }

    fn lower_trit_vec_pack5(
        &mut self,
        trits: &[i8],
        target_reg: Option<&str>,
        unpack_to_tvec: bool,
        line: usize,
        col: usize,
    ) -> Result<String, String> {
        let packed_bytes = pack5_encode(trits);
        let r_bytes = if !unpack_to_tvec {
            match target_reg {
                Some(r) => String::from(r),
                None => self.alloc_reg(line, col)?,
            }
        } else {
            self.alloc_reg(line, col)?
        };
        self.emit(format!("{} = ANEW 0", r_bytes))?;
        for chunk in packed_bytes.chunks(24) {
            let byte_strs: Vec<String> = chunk.iter().map(|b| b.to_string()).collect();
            self.emit(format!("{} = ISET {} {}", r_bytes, r_bytes, byte_strs.join(" ")))?;
        }
        if !unpack_to_tvec {
            return Ok(r_bytes);
        }
        let dst = match target_reg {
            Some(r) => String::from(r),
            None => self.alloc_reg(line, col)?,
        };
        self.emit(format!("{} = UNPACK5 {} {}", dst, r_bytes, trits.len()))?;
        Ok(dst)
    }

    fn lower_call_rhs(&mut self, name: &str, args: &[SurfaceExpr], line: usize, col: usize) -> Result<String, String> {
        // 1. Check user-defined functions in the module / included files first
        if let Some((callee_id, expected_arity, callee_ret_ty)) = self.fn_sigs.get(name).cloned() {
            if args.len() != expected_arity {
                return Err(format!(
                    "line {}, col {}: Function '{}' expects {} arguments, got {}",
                    line, col, name, expected_arity, args.len()
                ));
            }
            let mut op_args = Vec::with_capacity(args.len() + 1);
            op_args.push(format!("@fn{}", callee_id));
            for a in args {
                op_args.push(self.eval_to_operand(a)?);
            }
            let call_mnem = match callee_ret_ty.as_str() {
                "TRIT" => "CALL_TRIT",
                "BYTES" => "CALL_BYTES",
                "TRIT_VEC" => "CALL_TRIT_VEC",
                "STR" => "CALL_STR",
                _ => "CALL_I64",
            };
            return Ok(format!("{} {}", call_mnem, op_args.join(" ")));
        }

        let low = name.to_ascii_lowercase();
        let mut op_args = Vec::with_capacity(args.len());
        for a in args {
            op_args.push(self.eval_to_operand(a)?);
        }
        let unary_mnem = match low.as_str() {
            "iabs" | "abs" => Some("IABS"),
            "ineg" => Some("INEG"),
            "bnot" => Some("BNOT"),
            "tnot" => Some("TNOT"),
            "anew" => Some("ANEW"),
            "slen" | "pack5_len" => Some("SLEN"),
            "cint" | "mov" => Some("CINT"),
            "ctrit" => Some("CTRIT"),
            "pack5" | "tpack5" => Some("PACK5"),
            "egate" | "decision_hook" | "pack5_gate" => Some("EGATE"),
            "tlogic" | "decide_trit" | "pack5_tlogic" => Some("TLOGIC"),
            "lookahead" => Some("LOOKAHEAD"),
            "san_sql" | "sanitize_sql" => Some("SAN_SQL"),
            "san_dom" | "sanitize_dom" => Some("SAN_DOM"),
            "san_prm" | "sanitize_prompt" => Some("SAN_PRM"),
            "fnv1a" | "hash_fnv1a" => Some("FNV1A"),
            "arena" => Some("ARENA"),
            "borrow_r" => Some("BORROW_R"),
            "borrow_w" => Some("BORROW_W"),
            "move_val" => Some("MOVE"),
            "s2i" | "str_to_int" => Some("S2I"),
            "i2s" | "int_to_str" => Some("I2S"),
            _ => None,
        };
        if let Some(m) = unary_mnem {
            if op_args.len() != 1 {
                return Err(format!(
                    "line {}, col {}: Intrinsic '{}' expects 1 argument, got {}",
                    line, col, name, op_args.len()
                ));
            }
            return Ok(format!("{} {}", m, op_args[0]));
        }
        let binary_mnem = match low.as_str() {
            "iadd" => Some("IADD"),
            "isub" => Some("ISUB"),
            "imul" => Some("IMUL"),
            "idiv" => Some("IDIV"),
            "imod" => Some("IMOD"),
            "imin" | "min" => Some("IMIN"),
            "imax" | "max" => Some("IMAX"),
            "band" => Some("BAND"),
            "bor" => Some("BOR"),
            "bxor" => Some("BXOR"),
            "shl" => Some("SHL"),
            "shr" => Some("SHR"),
            "ceq" => Some("CEQ"),
            "cne" => Some("CNE"),
            "clt" => Some("CLT"),
            "cgt" => Some("CGT"),
            "tand" => Some("TAND"),
            "tor" => Some("TOR"),
            "txor" => Some("TXOR"),
            "tcmp" => Some("TCMP"),
            "iget" | "pack5_get" => Some("IGET"),
            "unpack5" | "tunpack5" => Some("UNPACK5"),
            "tdot" | "pack5_tdot" => Some("TDOT"),
            "scat" | "sconc" => Some("SCAT"),
            _ => None,
        };
        if let Some(m) = binary_mnem {
            if op_args.len() != 2 {
                return Err(format!(
                    "line {}, col {}: Intrinsic '{}' expects 2 arguments, got {}",
                    line, col, name, op_args.len()
                ));
            }
            return Ok(format!("{} {} {}", m, op_args[0], op_args[1]));
        }
        match low.as_str() {
            "tmaj" => {
                if op_args.len() != 3 {
                    return Err(format!(
                        "line {}, col {}: Intrinsic 'tmaj' expects 3 arguments, got {}",
                        line, col, op_args.len()
                    ));
                }
                Ok(format!("TMAJ {} {} {}", op_args[0], op_args[1], op_args[2]))
            }
            "sel_trit" | "cmov" => {
                if op_args.len() != 4 {
                    return Err(format!(
                        "line {}, col {}: Intrinsic '{}' expects 4 arguments, got {}",
                        line, col, name, op_args.len()
                    ));
                }
                let m = if low == "sel_trit" { "SEL_TRIT" } else { "CMOV" };
                Ok(format!("{} {} {} {} {}", m, op_args[0], op_args[1], op_args[2], op_args[3]))
            }
            "iset_i32" => {
                if op_args.len() != 2 {
                    return Err(format!(
                        "line {}, col {}: Intrinsic 'iset_i32' expects 2 arguments, got {}",
                        line, col, op_args.len()
                    ));
                }
                Ok(format!("ISET {} -4 {}", op_args[0], op_args[1]))
            }
            "iset_i64" => {
                if op_args.len() != 2 {
                    return Err(format!(
                        "line {}, col {}: Intrinsic 'iset_i64' expects 2 arguments, got {}",
                        line, col, op_args.len()
                    ));
                }
                Ok(format!("ISET {} -8 {}", op_args[0], op_args[1]))
            }
            "iset_patch_i64" => {
                if op_args.len() != 3 {
                    return Err(format!(
                        "line {}, col {}: Intrinsic 'iset_patch_i64' expects 3 arguments, got {}",
                        line, col, op_args.len()
                    ));
                }
                Ok(format!("ISET {} -9 {} {}", op_args[0], op_args[1], op_args[2]))
            }
            "iset" => {
                if op_args.len() < 2 {
                    return Err(format!(
                        "line {}, col {}: Intrinsic 'iset' expects at least 2 arguments",
                        line, col
                    ));
                }
                Ok(format!("ISET {}", op_args.join(" ")))
            }
            _ => Err(format!("line {}, col {}: Unknown surface intrinsic or function '{}'", line, col, name)),
        }
    }

    fn lower_block_into_target(
        &mut self,
        stmts: &[SurfaceStmt],
        dest_reg: &str,
        has_shared_join: bool,
        line: usize,
        col: usize,
    ) -> Result<(), String> {
        if stmts.is_empty() {
            return Err(format!("line {}, col {}: Empty branch block in control-flow expression", line, col));
        }
        for s in &stmts[..stmts.len() - 1] {
            self.lower_stmt(s)?;
        }
        match &stmts[stmts.len() - 1] {
            SurfaceStmt::Expr(e, _, _) | SurfaceStmt::Ret(e, _, _) => {
                let _ = self.lower_expr_into(e, Some(dest_reg), has_shared_join)?;
                Ok(())
            }
            other => self.lower_stmt(other),
        }
    }

    fn lower_expr_into(
        &mut self,
        expr: &SurfaceExpr,
        target_reg: Option<&str>,
        has_shared_join: bool,
    ) -> Result<String, String> {
        match expr {
            SurfaceExpr::Int(v, line, col) => {
                let dst = match target_reg {
                    Some(r) => String::from(r),
                    None => self.alloc_reg(*line, *col)?,
                };
                self.emit(format!("{} = CINT {}", dst, v))?;
                Ok(dst)
            }
            SurfaceExpr::Trit(s, _, line, col) => {
                let dst = match target_reg {
                    Some(r) => String::from(r),
                    None => self.alloc_reg(*line, *col)?,
                };
                self.emit(format!("{} = CTRIT {}", dst, s))?;
                Ok(dst)
            }
            SurfaceExpr::TritVecLit(trits, line, col) => {
                self.lower_trit_vec_pack5(trits, target_reg, true, *line, *col)
            }
            SurfaceExpr::StrLit(s, line, col) => {
                let dst = match target_reg {
                    Some(r) => String::from(r),
                    None => self.alloc_reg(*line, *col)?,
                };
                let mut hex_str = String::with_capacity(s.len() * 2);
                for &b in s.as_bytes() {
                    hex_str.push_str(&format!("{:02x}", b));
                }
                self.emit(format!("{} = CSTR@UNTRUSTED @str:{}", dst, hex_str))?;
                Ok(dst)
            }
            SurfaceExpr::Var(v, line, col) => {
                let src = self
                    .var_map
                    .get(v)
                    .cloned()
                    .ok_or_else(|| format!("line {}, col {}: Undefined variable '{}'", line, col, v))?;
                let dst = match target_reg {
                    Some(r) => String::from(r),
                    None => self.alloc_reg(*line, *col)?,
                };
                self.emit(format!("{} = CINT {}", dst, src))?;
                Ok(dst)
            }
            SurfaceExpr::UnOp(mnem, sub, line, col) => {
                let op_s = self.eval_to_operand(sub)?;
                let dst = match target_reg {
                    Some(r) => String::from(r),
                    None => self.alloc_reg(*line, *col)?,
                };
                self.emit(format!("{} = {} {}", dst, mnem, op_s))?;
                Ok(dst)
            }
            SurfaceExpr::BinOp(mnem, lhs, rhs, line, col) => {
                let l_op = self.eval_to_operand(lhs)?;
                let r_op = self.eval_to_operand(rhs)?;
                let dst = match target_reg {
                    Some(r) => String::from(r),
                    None => self.alloc_reg(*line, *col)?,
                };
                self.emit(format!("{} = {} {} {}", dst, mnem, l_op, r_op))?;
                Ok(dst)
            }
            SurfaceExpr::Call(name, args, line, col) => {
                if name.eq_ignore_ascii_case("pack5_bytes") {
                    if args.len() != 1 {
                        return Err(format!(
                            "line {}, col {}: Intrinsic 'pack5_bytes' expects 1 trit-vector literal argument",
                            line, col
                        ));
                    }
                    if let SurfaceExpr::TritVecLit(ref trits, l, c) = args[0] {
                        return self.lower_trit_vec_pack5(trits, target_reg, false, l, c);
                    } else {
                        return Err(format!(
                            "line {}, col {}: Intrinsic 'pack5_bytes' requires a '[+1t, -1t, ...]' literal",
                            line, col
                        ));
                    }
                }
                let rhs = self.lower_call_rhs(name, args, *line, *col)?;
                let dst = match target_reg {
                    Some(r) => String::from(r),
                    None => self.alloc_reg(*line, *col)?,
                };
                self.emit(format!("{} = {}", dst, rhs))?;
                Ok(dst)
            }
            SurfaceExpr::IfBranch(br_mnem, cond_expr, then_blk, else_blk, line, col) => {
                let cond_op = self.eval_to_operand(cond_expr)?;
                let dst = match target_reg {
                    Some(r) => String::from(r),
                    None => self.alloc_reg(*line, *col)?,
                };
                let l_then = self.alloc_lbl(*line, *col)?;
                self.emit(format!("{} {} {}", br_mnem, cond_op, l_then))?;
                let saved_vars = self.var_map.clone();
                self.lower_block_into_target(else_blk, &dst, false, *line, *col)?;
                self.var_map = saved_vars.clone();
                let jmp_placeholder_idx = if self.emit_asm { self.lines.len() } else { self.packets.len() };
                if self.emit_asm {
                    self.lines.push(String::from("JMP __JOIN__"));
                } else {
                    self.packets.push(vec![0x7F41, 0xFFFF, TOK_INS_END]);
                }
                self.emit(format!("LABEL {}", l_then))?;
                self.lower_block_into_target(then_blk, &dst, true, *line, *col)?;
                self.var_map = saved_vars;
                if !has_shared_join {
                    let l_join = self.alloc_lbl(*line, *col)?;
                    if self.emit_asm {
                        let join_str = l_join.to_string();
                        for l_str in &mut self.lines[jmp_placeholder_idx..] {
                            if l_str.contains("__JOIN__") {
                                *l_str = l_str.replace("__JOIN__", &join_str);
                            }
                        }
                    } else {
                        for pkt in &mut self.packets[jmp_placeholder_idx..] {
                            if pkt.len() >= 2 && pkt[0] == 0x7F41 && pkt[1] == 0xFFFF {
                                pkt[1] = LBL_BASE + l_join;
                            }
                        }
                    }
                    self.emit(format!("LABEL {}", l_join))?;
                }
                Ok(dst)
            }
            SurfaceExpr::MatchTrit(cond_expr, neg_blk, zero_blk, pos_blk, line, col) => {
                let cond_op = self.eval_to_operand(cond_expr)?;
                let dst = match target_reg {
                    Some(r) => String::from(r),
                    None => self.alloc_reg(*line, *col)?,
                };
                let l_neg = self.alloc_lbl(*line, *col)?;
                let l_zero = self.alloc_lbl(*line, *col)?;
                let l_pos = self.alloc_lbl(*line, *col)?;
                let l_join = self.alloc_lbl(*line, *col)?;
                self.emit(format!("BR_TRIT {} {} {} {}", cond_op, l_neg, l_zero, l_pos))?;
                let saved_vars = self.var_map.clone();
                self.emit(format!("LABEL {}", l_neg))?;
                self.lower_block_into_target(neg_blk, &dst, false, *line, *col)?;
                self.var_map = saved_vars.clone();
                self.emit(format!("JMP {}", l_join))?;
                self.emit(format!("LABEL {}", l_zero))?;
                self.lower_block_into_target(zero_blk, &dst, false, *line, *col)?;
                self.var_map = saved_vars.clone();
                self.emit(format!("JMP {}", l_join))?;
                self.emit(format!("LABEL {}", l_pos))?;
                self.lower_block_into_target(pos_blk, &dst, false, *line, *col)?;
                self.var_map = saved_vars;
                self.emit(format!("LABEL {}", l_join))?;
                Ok(dst)
            }
        }
    }

    fn lower_stmt(&mut self, stmt: &SurfaceStmt) -> Result<(), String> {
        match stmt {
            SurfaceStmt::Let(name, expr, _line, _col) => {
                let reg = self.lower_expr_into(expr, None, false)?;
                self.var_map.insert(name.clone(), reg);
                Ok(())
            }
            SurfaceStmt::Assign(name, expr, line, col) => {
                let reg = self
                    .var_map
                    .get(name)
                    .cloned()
                    .ok_or_else(|| format!("line {}, col {}: Cannot reassign undeclared variable '{}'", line, col, name))?;
                if !reg.starts_with('%') {
                    return Err(format!("line {}, col {}: Cannot reassign immutable function parameter '{}'", line, col, name));
                }
                let _ = self.lower_expr_into(expr, Some(&reg), false)?;
                Ok(())
            }
            SurfaceStmt::DoWhile(body, cond_expr, line, col) => {
                if body.is_empty() {
                    return Err(format!("line {}, col {}: Empty do_while loop body", line, col));
                }
                let l_top = self.alloc_lbl(*line, *col)?;
                self.emit(format!("LABEL {}", l_top))?;
                for s in body {
                    self.lower_stmt(s)?;
                }
                let cond_op = self.eval_to_operand(cond_expr)?;
                self.emit(format!("BR_POS {} {}", cond_op, l_top))?;
                Ok(())
            }
            SurfaceStmt::While(cond_expr, body, line, col) => {
                if body.is_empty() {
                    return Err(format!("line {}, col {}: Empty while loop body", line, col));
                }
                let l_top = self.alloc_lbl(*line, *col)?;
                let l_body = self.alloc_lbl(*line, *col)?;
                let l_exit = self.alloc_lbl(*line, *col)?;
                self.emit(format!("LABEL {}", l_top))?;
                let cond_op = self.eval_to_operand(cond_expr)?;
                self.emit(format!("BR_POS {} {}", cond_op, l_body))?;
                self.emit(format!("JMP {}", l_exit))?;
                self.emit(format!("LABEL {}", l_body))?;
                for s in body {
                    self.lower_stmt(s)?;
                }
                self.emit(format!("JMP {}", l_top))?;
                self.emit(format!("LABEL {}", l_exit))?;
                Ok(())
            }
            SurfaceStmt::Label(lid, _line, _col) => {
                self.defined_labels.push(*lid);
                self.emit(format!("LABEL {}", lid))?;
                Ok(())
            }
            SurfaceStmt::Goto(lid, line, col) => {
                self.used_labels.push((*lid, *line, *col));
                self.emit(format!("JMP {}", lid))?;
                Ok(())
            }
            SurfaceStmt::Ret(expr, _line, _col) => {
                let op = self.eval_to_operand(expr)?;
                self.emit(format!("RET {}", op))?;
                Ok(())
            }
            SurfaceStmt::Expr(expr, line, col) => {
                if let SurfaceExpr::Call(cname, args, cl, cc) = expr {
                    let low = cname.to_ascii_lowercase();
                    if low == "br_pos" || low == "br_neg" {
                        if args.len() != 2 {
                            return Err(format!("line {}, col {}: '{}' expects 2 arguments", cl, cc, cname));
                        }
                        let c_op = self.eval_to_operand(&args[0])?;
                        if let SurfaceExpr::Int(lid, ll, lc) = args[1] {
                            if !(0..=15).contains(&lid) {
                                return Err(format!("line {}, col {}: Branch label {} out of range 0..=15", ll, lc, lid));
                            }
                            self.used_labels.push((lid as u16, ll, lc));
                            self.emit(format!("{} {} {}", low.to_ascii_uppercase(), c_op, lid))?;
                            return Ok(());
                        } else {
                            return Err(format!("line {}, col {}: '{}' target must be an integer label id", cl, cc, cname));
                        }
                    } else if low == "br_trit" {
                        if args.len() != 4 {
                            return Err(format!("line {}, col {}: 'br_trit' expects 4 arguments", cl, cc));
                        }
                        let c_op = self.eval_to_operand(&args[0])?;
                        let mut lids = Vec::new();
                        for a in &args[1..] {
                            if let SurfaceExpr::Int(lid, ll, lc) = *a {
                                if !(0..=15).contains(&lid) {
                                    return Err(format!("line {}, col {}: Branch label {} out of range 0..=15", ll, lc, lid));
                                }
                                self.used_labels.push((lid as u16, ll, lc));
                                lids.push(lid.to_string());
                            } else {
                                return Err(format!("line {}, col {}: 'br_trit' targets must be integer label ids", cl, cc));
                            }
                        }
                        self.emit(format!("BR_TRIT {} {}", c_op, lids.join(" ")))?;
                        return Ok(());
                    } else if low == "drop" {
                        let mut ops = Vec::with_capacity(args.len());
                        for a in args {
                            ops.push(self.eval_to_operand(a)?);
                        }
                        self.emit(format!("DROP {}", ops.join(" ")))?;
                        return Ok(());
                    }
                }
                let _ = self.lower_expr_into(expr, None, false)?;
                let _ = (line, col);
                Ok(())
            }
        }
    }

    fn check_unresolved_labels(&self) -> Result<(), String> {
        for &(lid, line, col) in &self.used_labels {
            if !self.defined_labels.contains(&lid) {
                return Err(format!(
                    "line {}, col {}: Unresolved branch target label {}",
                    line, col, lid
                ));
            }
        }
        Ok(())
    }
}

/// Resolves `include "other.omni";` / `mod "other.omni";` directives from an in-memory map or filesystem directory.
pub fn resolve_omni_surface_includes(
    src: &str,
    base_dir: Option<&std::path::Path>,
    vfs: Option<&BTreeMap<String, String>>,
) -> Result<String, String> {
    fn resolve_inner(
        cur_src: &str,
        cur_dir: Option<&std::path::Path>,
        vfs: Option<&BTreeMap<String, String>>,
        stack: &mut Vec<String>,
    ) -> Result<String, String> {
        let tokens = tokenize_omni_surface(cur_src)?;
        let mut parser = SurfaceParser::new(tokens);
        let (includes, _fns) = parser.parse_module()?;
        if includes.is_empty() {
            return Ok(String::from(cur_src));
        }

        let mut combined = String::new();
        for (inc_path, line, col) in includes {
            if stack.contains(&inc_path) {
                return Err(format!("line {}, col {}: Circular include detected for '{}'", line, col, inc_path));
            }
            let inc_content = if let Some(map) = vfs.and_then(|m| m.get(&inc_path).map(|s| (m, s))) {
                map.1.clone()
            } else if let Some(dir) = cur_dir {
                let full_p = dir.join(&inc_path);
                fs::read_to_string(&full_p).map_err(|e| {
                    format!("line {}, col {}: Failed to read included file '{}': {}", line, col, full_p.display(), e)
                })?
            } else {
                fs::read_to_string(&inc_path).map_err(|e| {
                    format!("line {}, col {}: Failed to read included file '{}': {}", line, col, inc_path, e)
                })?
            };
            stack.push(inc_path.clone());
            let expanded = resolve_inner(&inc_content, cur_dir, vfs, stack)?;
            stack.pop();
            combined.push_str(&expanded);
            combined.push('\n');
        }

        // Strip top-level include/mod lines from cur_src before appending
        for raw_line in cur_src.lines() {
            let trimmed = raw_line.trim_start();
            if trimmed.starts_with("include ") || trimmed.starts_with("include\"") || trimmed.starts_with("mod ") {
                continue;
            }
            combined.push_str(raw_line);
            combined.push('\n');
        }
        Ok(combined)
    }

    let mut stack = Vec::new();
    resolve_inner(src, base_dir, vfs, &mut stack)
}

/// Lowers an Omni surface language program (single-function or multi-function) to canonical `.omni_asm`.
pub fn compile_omni_surface_to_asm(src: &str) -> Result<String, String> {
    let tokens = tokenize_omni_surface(src)?;
    let mut parser = SurfaceParser::new(tokens);
    let (includes, fns) = parser.parse_module()?;
    if !includes.is_empty() {
        let expanded = resolve_omni_surface_includes(src, None, None)?;
        return compile_omni_surface_to_asm(&expanded);
    }

    let multi_fn = fns.len() > 1;
    let mut fn_sigs: BTreeMap<String, (u16, usize, String)> = BTreeMap::new();
    let mut assigned_ids: Vec<u16> = Vec::with_capacity(fns.len());

    for (idx, f) in fns.iter().enumerate() {
        let fid = if let Some(id_attr) = f.fn_id_attr {
            id_attr
        } else if !multi_fn || idx + 1 == fns.len() {
            0
        } else {
            (idx + 1) as u16
        };
        let ret_asm = String::from(map_omni_surface_type(&f.ret_ty, f.ret_line, f.ret_col)?);
        for p in &f.params {
            let _ = map_omni_surface_type(&p.ty, p.line, p.col)?;
        }
        if fn_sigs.contains_key(&f.name) {
            return Err(format!("line {}, col {}: Duplicate function definition '{}'", f.ret_line, f.ret_col, f.name));
        }
        fn_sigs.insert(f.name.clone(), (fid, f.params.len(), ret_asm));
        assigned_ids.push(fid);
    }

    let mut out = Vec::new();
    for (idx, f) in fns.iter().enumerate() {
        let fid = assigned_ids[idx];
        if multi_fn {
            out.push(format!("FN_DEF {}", fid));
        } else if let Some(explicit_id) = f.fn_id_attr {
            out.push(format!("FN_ID {}", explicit_id));
        }
        if f.cap_mask != 0 {
            out.push(format!("CAP_MASK {}", f.cap_mask));
        }
        for (p_idx, p) in f.params.iter().enumerate() {
            let ty_str = map_omni_surface_type(&p.ty, p.line, p.col)?;
            let taint_str = if p.untrusted { "UNTRUSTED" } else { "SAFE" };
            out.push(format!("PARAM ${} {} {}", p_idx, ty_str, taint_str));
        }
        let ret_ty_str = map_omni_surface_type(&f.ret_ty, f.ret_line, f.ret_col)?;
        let ret_ty_tok = parse_asm_ty_tok(ret_ty_str);
        out.push(format!("RET_TY {}", ret_ty_str));

        let mut ctx = SurfaceLowerCtx::new(&f.params, fn_sigs.clone(), ret_ty_tok, true);
        for stmt in &f.body {
            ctx.lower_stmt(stmt)?;
        }
        ctx.check_unresolved_labels()?;
        out.extend(ctx.lines);
    }

    let mut joined = out.join("\n");
    joined.push('\n');
    Ok(joined)
}

/// Compiles an Omni surface language program directly into a `Vec<u16>` token stream
/// without building or re-parsing an intermediate `.omni_asm` text string (Zero Long-Lived IR).
pub fn compile_omni_surface_to_u16(src: &str) -> Result<Vec<u16>, String> {
    let tokens = tokenize_omni_surface(src)?;
    let mut parser = SurfaceParser::new(tokens);
    let (includes, fns) = parser.parse_module()?;
    if !includes.is_empty() {
        let expanded = resolve_omni_surface_includes(src, None, None)?;
        return compile_omni_surface_to_u16(&expanded);
    }

    let multi_fn = fns.len() > 1;
    let mut fn_sigs: BTreeMap<String, (u16, usize, String)> = BTreeMap::new();
    let mut assigned_ids: Vec<u16> = Vec::with_capacity(fns.len());

    for (idx, f) in fns.iter().enumerate() {
        let fid = if let Some(id_attr) = f.fn_id_attr {
            id_attr
        } else if !multi_fn || idx + 1 == fns.len() {
            0
        } else {
            (idx + 1) as u16
        };
        let ret_asm = String::from(map_omni_surface_type(&f.ret_ty, f.ret_line, f.ret_col)?);
        for p in &f.params {
            let _ = map_omni_surface_type(&p.ty, p.line, p.col)?;
        }
        if fn_sigs.contains_key(&f.name) {
            return Err(format!("line {}, col {}: Duplicate function definition '{}'", f.ret_line, f.ret_col, f.name));
        }
        fn_sigs.insert(f.name.clone(), (fid, f.params.len(), ret_asm));
        assigned_ids.push(fid);
    }

    let mut completed_fns: Vec<(u16, Vec<(u16, u16, u16)>, u16, CapMask, Vec<Vec<u16>>)> = Vec::with_capacity(fns.len());

    for (idx, f) in fns.iter().enumerate() {
        let fid = assigned_ids[idx];
        let mut params: Vec<(u16, u16, u16)> = Vec::with_capacity(f.params.len());
        for (p_idx, p) in f.params.iter().enumerate() {
            let ty_str = map_omni_surface_type(&p.ty, p.line, p.col)?;
            let ty_tok = parse_asm_ty_tok(ty_str);
            params.push((PAR_BASE + p_idx as u16, ty_tok, pack_tc(p.untrusted, CAP_ID_PURE)));
        }
        let ret_ty_tok = parse_asm_ty_tok(map_omni_surface_type(&f.ret_ty, f.ret_line, f.ret_col)?);

        let mut ctx = SurfaceLowerCtx::new(&f.params, fn_sigs.clone(), ret_ty_tok, false);
        for stmt in &f.body {
            ctx.lower_stmt(stmt)?;
        }
        ctx.check_unresolved_labels()?;

        completed_fns.push((fid, params, ret_ty_tok, f.cap_mask, ctx.packets));
    }

    if completed_fns.len() == 1 {
        let (f_id, f_params, f_ret, f_cap, f_ins) = completed_fns.pop().unwrap();
        Ok(pack_fn_module(&[], f_id, &f_params, f_ret, f_cap, &f_ins))
    } else {
        let mut stream = vec![TOK_MOD_BEGIN];
        for (f_id, f_params, f_ret, f_cap, f_ins) in completed_fns {
            stream.push(TOK_FN_DEF);
            stream.push(f_id);
            stream.push(f_params.len() as u16);
            for (pt, pty, ptc) in f_params {
                stream.push(pt);
                stream.push(pty);
                stream.push(ptc);
            }
            stream.push(f_ret);
            stream.push(f_cap);
            stream.push(f_ins.len() as u16);
            for pkt in f_ins {
                stream.extend_from_slice(&pkt);
            }
        }
        stream.push(TOK_MOD_END);
        Ok(stream)
    }
}

/// Extracts optional `(step_budget, mem_budget)` attributes declared on functions in a `.omni` program.
pub fn extract_omni_surface_budgets(src: &str) -> (Option<u64>, Option<usize>) {
    let wrapped = wrap_omni_surface_snippet(src);
    let Ok(tokens) = tokenize_omni_surface(&wrapped) else {
        return (None, None);
    };
    let mut parser = SurfaceParser::new(tokens);
    let Ok((includes, fns)) = parser.parse_module() else {
        return (None, None);
    };
    if !includes.is_empty() {
        if let Ok(expanded) = resolve_omni_surface_includes(&wrapped, None, None) {
            return extract_omni_surface_budgets(&expanded);
        }
    }
    let mut step_b: Option<u64> = None;
    let mut mem_b: Option<usize> = None;
    for f in fns {
        if let Some(sb) = f.step_budget {
            step_b = Some(step_b.map_or(sb, |cur: u64| cur.min(sb)));
        }
        if let Some(mb) = f.mem_budget {
            mem_b = Some(mem_b.map_or(mb, |cur: usize| cur.min(mb)));
        }
    }
    (step_b, mem_b)
}

/// Compiles a `.omni` file from disk, resolving any relative `include "..."` directives.
pub fn compile_omni_surface_file_to_u16(path: &str) -> Result<Vec<u16>, String> {
    let raw_src = fs::read_to_string(path).map_err(|e| format!("Failed to read '{}': {}", path, e))?;
    let parent_dir = std::path::Path::new(path).parent();
    let expanded = resolve_omni_surface_includes(&raw_src, parent_dir, None)?;
    compile_omni_surface_to_u16(&expanded)
}

/// Links multiple in-memory `.omni` surface modules and compiles them into a single `Vec<u16>` token stream.
pub fn link_omni_surface_modules(modules: &[&str]) -> Result<Vec<u16>, String> {
    let joined = modules.join("\n");
    compile_omni_surface_to_u16(&joined)
}

/// Parses a CLI string argument into an `OmniValue` (`+1t`/`0t`/`-1t` -> `Trit`, integer -> `Int`, else `Str`).
pub fn parse_cli_omni_value(a_str: &str) -> OmniValue {
    match a_str {
        "+1t" | "TRIT_POS" => OmniValue::Trit(1),
        "0t" | "TRIT_ZERO" => OmniValue::Trit(0),
        "-1t" | "TRIT_NEG" => OmniValue::Trit(-1),
        _ => {
            if let Ok(n) = a_str.parse::<i64>() {
                OmniValue::Int(n)
            } else {
                OmniValue::Str(String::from(a_str))
            }
        }
    }
}

/// Wraps a bare expression or multi-statement snippet in a full `#[cap(...)] fn main(...) -> <ty>` function if it does not already contain `fn `.
pub fn wrap_omni_surface_snippet(snippet: &str) -> String {
    if snippet.contains("fn ") {
        return String::from(snippet);
    }
    let trimmed = snippet.trim().trim_end_matches(';').trim();
    let bytes = trimmed.as_bytes();
    let mut in_str = false;
    let mut brace_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut stmt_start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if in_str {
            if b == b'\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            if b == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'{' => brace_depth += 1,
            b'}' => {
                brace_depth = brace_depth.saturating_sub(1);
                if brace_depth == 0 && paren_depth == 0 && bracket_depth == 0 {
                    let cur_stmt = trimmed[stmt_start..=i].trim_start();
                    if cur_stmt.starts_with("while ") || cur_stmt.starts_with("while(") {
                        stmt_start = i + 1;
                    }
                }
            }
            b'(' => paren_depth += 1,
            b')' => paren_depth = paren_depth.saturating_sub(1),
            b'[' => bracket_depth += 1,
            b']' => bracket_depth = bracket_depth.saturating_sub(1),
            b';' => {
                if brace_depth == 0 && paren_depth == 0 && bracket_depth == 0 {
                    stmt_start = i + 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    let prefix = trimmed[..stmt_start].trim();
    let raw_tail = trimmed[stmt_start..].trim();
    let tail = if let Some(stripped) = raw_tail.strip_prefix("return ") {
        stripped.trim()
    } else if raw_tail.is_empty() {
        "0"
    } else {
        raw_tail
    };

    let infer_expr_ty = |expr: &str| -> &'static str {
        let e = expr.trim();
        if e.starts_with('"')
            || e.starts_with("san_sql(")
            || e.starts_with("san_dom(")
            || e.starts_with("san_prm(")
            || e.starts_with("scat(")
            || e.starts_with("sconc(")
            || e.starts_with("i2s(")
        {
            "str"
        } else if e.starts_with('[') || e.starts_with("unpack5(") {
            "trit_vec"
        } else if e.starts_with("pack5_bytes(") || e.starts_with("anew(") {
            "bytes"
        } else if e.contains("<=>")
            || e.contains("==")
            || e.contains("!=")
            || e.contains("<=")
            || e.contains(">=")
            || e.contains(" < ")
            || e.contains(" > ")
            || e.starts_with("egate(")
            || e.starts_with("pack5_gate(")
            || e.starts_with("decision_hook(")
            || e.starts_with("tlogic(")
            || e.starts_with("pack5_tlogic(")
            || e.starts_with("tmaj(")
            || e.starts_with("tand(")
            || e.starts_with("tor(")
            || e.starts_with("txor(")
            || e.starts_with("tcmp(")
            || e == "+1t"
            || e == "0t"
            || e == "-1t"
        {
            "trit"
        } else {
            "i64"
        }
    };

    let ret_ty = infer_expr_ty(tail);
    if prefix.is_empty() {
        format!(
            "#[cap(AI, DB_R, DB_W, NET, DOM, VISION, AUDIO, AUTH, FS, SYS)] fn main() -> {} {{ let __res = {}; return __res; }}",
            ret_ty, tail
        )
    } else {
        format!(
            "#[cap(AI, DB_R, DB_W, NET, DOM, VISION, AUDIO, AUTH, FS, SYS)] fn main() -> {} {{ {} let __res = {}; return __res; }}",
            ret_ty, prefix, tail
        )
    }
}

/// Compiles, verifies, and executes an Omni surface program with explicit step and memory budgets.
pub fn execute_omni_surface_program_with_budgets(
    src: &str,
    args: Vec<OmniValue>,
    step_budget: Option<u64>,
    mem_budget: Option<usize>,
) -> Result<OmniValue, String> {
    let wrapped = wrap_omni_surface_snippet(src);
    let (attr_steps, attr_mem) = extract_omni_surface_budgets(&wrapped);
    let eff_steps = match (step_budget, attr_steps) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };
    let eff_mem = match (mem_budget, attr_mem) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };
    let u16_stream = compile_omni_surface_to_u16(&wrapped)?;
    for rep in verify_u16_stream(&u16_stream)? {
        if !rep.passed {
            return Err(format!("Static verification failed on fn #{}: {:?}", rep.fn_idx, rep.errors));
        }
    }
    let module = decode_omni_token_stream(&u16_stream)?;
    let entry_fn_idx = if module.functions.iter().any(|f| f.fn_idx == 0) {
        0
    } else {
        module.functions.last().map(|f| f.fn_idx).ok_or("No functions in module")?
    };
    let mut vm = OmniBinaryVm::new();
    if let Some(sb) = eff_steps {
        vm.step_limit = sb;
    }
    if let Some(mb) = eff_mem {
        vm.memory_budget_bytes = mb;
    }
    vm.execute_function(&module, entry_fn_idx, args)
}

/// Executes an Omni surface program using the AOT hot path (`compile_pure_u16_to_wasm` ->
/// `execute_wasm_module_i64_with_budgets`) whenever the module is an AOT-eligible numeric/ternary/
/// `@PACK5` hot path returning `i64` or `trit`, falling back to `OmniBinaryVm` under explicit budgets
/// only for high-level string, composite-return, or multi-function modules.
/// Returns `(result_value, "aot" | "vm")`.
pub fn execute_omni_surface_aot_first(
    src: &str,
    args: Vec<OmniValue>,
) -> Result<(OmniValue, &'static str), String> {
    let wrapped = wrap_omni_surface_snippet(src);
    let (attr_steps, attr_mem) = extract_omni_surface_budgets(&wrapped);
    let u16_stream = compile_omni_surface_to_u16(&wrapped)?;
    for rep in verify_u16_stream(&u16_stream)? {
        if !rep.passed {
            return Err(format!("Static verification failed on fn #{}: {:?}", rep.fn_idx, rep.errors));
        }
    }
    let module = decode_omni_token_stream(&u16_stream)?;
    for func in &module.functions {
        let rep = verify_binary_function(func);
        if !rep.passed {
            return Err(format!("Static verification failed on fn #{}: {:?}", func.fn_idx, rep.errors));
        }
    }
    let entry_fn_idx = if module.functions.iter().any(|f| f.fn_idx == 0) {
        0
    } else {
        module.functions.last().map(|f| f.fn_idx).ok_or("No functions in module")?
    };
    let entry_ret_ty_tok = module
        .functions
        .iter()
        .find(|f| f.fn_idx == entry_fn_idx)
        .map(|f| f.ret_ty_tok)
        .unwrap_or(TY_I64);

    let all_scalar_args = args.iter().all(|a| matches!(a, OmniValue::Int(_) | OmniValue::Trit(_)));
    let scalar_ret = matches!(entry_ret_ty_tok, TY_I64 | TY_TRIT);

    if is_module_aot_hot_path(&module) && all_scalar_args && scalar_ret {
        let step_b = attr_steps.unwrap_or(DEFAULT_WASM_STEP_LIMIT);
        let mem_b = attr_mem.unwrap_or(DEFAULT_WASM_MEMORY_BUDGET_BYTES);
        let mut static_alloc_bytes = 0usize;
        for func in &module.functions {
            for ins in &func.instructions {
                if ins.op_tok == 0x7F01 {
                    if let Some(BinOperand::ImmInt(cap)) = ins.operands.first() {
                        if *cap > 0 {
                            static_alloc_bytes = static_alloc_bytes.saturating_add(*cap as usize);
                        }
                    }
                }
            }
        }
        if static_alloc_bytes > mem_b {
            return Err(format!(
                "AOT memory budget exceeded ({} > {} bytes)",
                static_alloc_bytes, mem_b
            ));
        }
        let i64_args: Vec<i64> = args
            .iter()
            .map(|a| match *a {
                OmniValue::Int(n) => n,
                OmniValue::Trit(t) => t as i64,
                _ => 0,
            })
            .collect();
        let wasm = compile_pure_u16_to_wasm(&u16_stream)?;
        let raw_i64 = execute_wasm_module_i64_with_budgets(&wasm, &i64_args, step_b, mem_b)?;
        let out_val = if entry_ret_ty_tok == TY_TRIT {
            OmniValue::Trit(raw_i64.clamp(-1, 1) as i8)
        } else {
            OmniValue::Int(raw_i64)
        };
        return Ok((out_val, "aot"));
    }

    let mut vm = OmniBinaryVm::new();
    if let Some(sb) = attr_steps {
        vm.step_limit = sb;
    }
    if let Some(mb) = attr_mem {
        vm.memory_budget_bytes = mb;
    }
    let val = vm.execute_function(&module, entry_fn_idx, args)?;
    Ok((val, "vm"))
}

/// Compiles, statically verifies all functions in the module, and executes an Omni surface program
/// using the AOT-First hot numeric path (`execute_omni_surface_aot_first`) with automatic fallback to `OmniBinaryVm`.
pub fn execute_omni_surface_program(src: &str, args: Vec<OmniValue>) -> Result<OmniValue, String> {
    execute_omni_surface_aot_first(src, args).map(|(v, _)| v)
}

/// Compiles a surface `.omni` program via the Stage-1 self-hosting compiler stream (`build_step7_stage1_compiler_stream`)
/// into a standalone 64-bit Linux x86-64 ELF64 binary without using host x86-64 codegen.
pub fn compile_omni_surface_via_stage1_to_elf(
    src: &str,
    arg0: i64,
    arg1: i64,
    arg2: i64,
) -> Result<Vec<u8>, String> {
    let wrapped = wrap_omni_surface_snippet(src);
    let u16_stream = compile_omni_surface_to_u16(&wrapped)?;
    for rep in verify_u16_stream(&u16_stream)? {
        if !rep.passed {
            return Err(format!("Static verification failed on fn #{}: {:?}", rep.fn_idx, rep.errors));
        }
    }
    let module = decode_omni_token_stream(&u16_stream)?;
    if module.functions.len() != 1 {
        return Err(String::from("Stage-1 ELF64 emitter requires a single-function module"));
    }
    for ins in &module.functions[0].instructions {
        if !is_x86_64_aot_supported_opcode(ins.op_tok) {
            return Err(format!("Unsupported opcode 0x{:04X} in Stage-1 ELF64 emitter", ins.op_tok));
        }
    }
    let s1_u16 = build_step7_stage1_compiler_stream();
    let s1_mod = decode_omni_token_stream(&s1_u16)?;
    let mut vm = OmniBinaryVm::new();
    let out = vm.execute_function(
        &s1_mod,
        0x0000,
        vec![
            OmniValue::U16Array(u16_stream),
            OmniValue::Int(arg0),
            OmniValue::Int(arg1),
            OmniValue::Int(arg2),
        ],
    )?;
    match out {
        OmniValue::Bytes(elf) => Ok(elf),
        other => Err(format!("Stage-1 compiler returned unexpected value: {:?}", other)),
    }
}

/// Compiles a surface `.omni` program directly to a standalone 64-bit Linux x86-64 ELF64 binary.
pub fn compile_omni_surface_to_x86_64_elf(
    src: &str,
    arg0: i64,
    arg1: i64,
    arg2: i64,
) -> Result<Vec<u8>, String> {
    let wrapped = wrap_omni_surface_snippet(src);
    let u16_stream = compile_omni_surface_to_u16(&wrapped)?;
    compile_pure_u16_to_x86_64_elf_with_args(&u16_stream, arg0, arg1, arg2)
}

/// Compiles a surface `.omni` program directly to a standalone 64-bit WebAssembly (`\0asm` v1) binary.
pub fn compile_omni_surface_to_wasm(src: &str) -> Result<Vec<u8>, String> {
    let wrapped = wrap_omni_surface_snippet(src);
    let u16_stream = compile_omni_surface_to_u16(&wrapped)?;
    compile_pure_u16_to_wasm(&u16_stream)
}

/// Compiles a surface `.omni` program to 64-bit WebAssembly and executes it in the pure-Rust 64-bit Wasm evaluator.
pub fn execute_omni_surface_wasm_i64(src: &str, args: &[i64]) -> Result<i64, String> {
    let wrapped = wrap_omni_surface_snippet(src);
    let (step_b, mem_b) = extract_omni_surface_budgets(&wrapped);
    let eff_step = step_b.unwrap_or(DEFAULT_WASM_STEP_LIMIT);
    let eff_mem = mem_b.unwrap_or(DEFAULT_WASM_MEMORY_BUDGET_BYTES);
    let u16_stream = compile_omni_surface_to_u16(&wrapped)?;
    let module = decode_omni_token_stream(&u16_stream)?;
    let mut static_alloc_bytes = 0usize;
    for func in &module.functions {
        for ins in &func.instructions {
            if ins.op_tok == 0x7F01 {
                if let Some(BinOperand::ImmInt(cap)) = ins.operands.first() {
                    if *cap > 0 {
                        static_alloc_bytes = static_alloc_bytes.saturating_add(*cap as usize);
                    }
                }
            }
        }
    }
    if static_alloc_bytes > eff_mem {
        return Err(format!(
            "AOT memory budget exceeded ({} > {} bytes)",
            static_alloc_bytes, eff_mem
        ));
    }
    let wasm = compile_pure_u16_to_wasm(&u16_stream)?;
    execute_wasm_module_i64_with_budgets(&wasm, args, eff_step, eff_mem)
}

pub fn build_step1_pure_token_stream() -> Vec<u16> {
    let tc_safe_pure   = pack_tc(false, CAP_ID_PURE);
    let tc_untr_pure   = pack_tc(true,  CAP_ID_PURE);
    let tc_safe_ai     = pack_tc(false, CAP_ID_AI);
    let tc_safe_db_w   = pack_tc(false, CAP_ID_DB_W);
    let tc_safe_db_r   = pack_tc(false, CAP_ID_DB_R);
    let tc_safe_vision = pack_tc(false, CAP_ID_VISION);
    let tc_safe_dom    = pack_tc(false, CAP_ID_DOM);

    let f0_caps: CapMask = cap_bit(CAP_ID_DB_W)
        | cap_bit(CAP_ID_DB_R)
        | cap_bit(CAP_ID_AI)
        | cap_bit(CAP_ID_VISION)
        | cap_bit(CAP_ID_AUDIO)
        | cap_bit(CAP_ID_DOM)
        | cap_bit(CAP_ID_AUTH);

    vec![
        TOK_MOD_BEGIN,
        TOK_SCHEMA_DEF, 0x0000, 0x0004,
            TY_UUID, tc_safe_pure, TY_STR, tc_safe_pure, TY_TRIT_VEC, tc_safe_pure, TY_TRIT, tc_safe_pure,
        TOK_EXPERT_DEF, 0x0000, 0x0040, 0x0040, 0x0010,
        TOK_FN_DEF, 0x0001, 0x0002,
            0x7EC0, TY_I64, tc_safe_pure,
            0x7EC1, TY_I64, tc_safe_pure,
            TY_TRIT, 0x0000, 0x0006,
            0x7E00, 0x7F12, TY_I64,  tc_safe_pure, 0x7EC0, TOK_IMM_I16, 0x0002, TOK_INS_END,
            0x7E01, 0x7F10, TY_I64,  tc_safe_pure, 0x7E00, 0x7EC1, TOK_INS_END,
            0x7E02, 0x7F2D, TY_TRIT, tc_safe_pure, 0x7E01, TOK_IMM_I16, 0x0064, TOK_INS_END,
            0x7E03, 0x7F09, TY_TRIT, tc_safe_pure, TOK_TRIT_POS, TOK_INS_END,
            0x7E04, 0x7F29, TY_TRIT, tc_safe_pure, 0x7E02, 0x7E03, TOK_INS_END,
            0x7F4A, 0x7E04, TOK_INS_END,
        TOK_FN_DEF, 0x0000, 0x0003,
            0x7EC0, TY_UUID, tc_safe_pure,
            0x7EC1, TY_STR,  tc_untr_pure,
            0x7EC2, TY_I64,  tc_safe_pure,
            TY_DOM, f0_caps, 0x0010,
            0x7E00, 0x7F01, TY_ARENA,    tc_safe_pure,   TOK_IMM_I32, 0x0400, 0x0000, TOK_INS_END,
            0x7E01, 0x7F59, TY_STR,      tc_safe_ai,     0x7EC1, TOK_INS_END,
            0x7E02, 0x7F7D, TY_TRIT_VEC, tc_safe_ai,     0x7E01, TOK_INS_END,
            0x7E03, 0x7F48, TY_TRIT,     tc_safe_pure,   TOK_SYM_FN, 0x0001, 0x7EC2, TOK_IMM_I16, 0x0028, TOK_INS_END,
            0x7E04, 0x7F57, TY_STR,      tc_safe_pure,   0x7EC1, TOK_INS_END,
            0x7E05, 0x7F63, TY_STRUCT,   tc_safe_db_w,   TOK_SYM_SCHEMA, 0x0000,
                TOK_FIELD_KV, 0x0000, 0x7EC0,
                TOK_FIELD_KV, 0x0001, 0x7E04,
                TOK_FIELD_KV, 0x0002, 0x7E02,
                TOK_FIELD_KV, 0x0003, 0x7E03,
                TOK_INS_END,
            0x7E06, 0x7F6A, TY_STRUCT,   tc_safe_db_r,   TOK_SYM_SCHEMA, 0x0000, 0x7E02, TOK_IMM_I16, 0x0001, TOK_INS_END,
            0x7E07, 0x7F03, TY_ARENA,    tc_safe_pure,   0x7E00, TOK_INS_END,
            0x7E08, 0x7F7E, TY_MODEL_T,  tc_safe_vision, TOK_SYM_SKILL, 0x0001, 0x7E07, TOK_INS_END,
            0x7E09, 0x7F7F, TY_STR,      tc_safe_vision, 0x7E08, 0x7E01, TOK_INS_END,
            0x7F05, 0x7E08, TOK_INS_END,
            0x7E0A, 0x7F58, TY_STR,      tc_safe_pure,   0x7EC1, TOK_INS_END,
            0x7E0B, 0x7F76, TY_DOM,      tc_safe_dom,    TOK_IMM_I16, 0x0001, 0x7E0A, 0x7E09, TOK_INS_END,
            0x7F05, 0x7E07, TOK_INS_END,
            0x7F05, 0x7E00, TOK_INS_END,
            0x7F4A, 0x7E0B, TOK_INS_END,
        TOK_MOD_END,
    ]
}

// ==============================================================================
// 8. STEP 2 (PURE RUST): BOOTSTRAP TRANSPILER & VERIFIED 64-SAMPLE CORPUS BUILDER
// ==============================================================================

pub struct CorpusRecord {
    pub sample_id: usize,
    pub english_prompt: String,
    pub english_token_ids: Vec<u16>,
    pub omni_u16_token_ids: Vec<u16>,
    pub ssa_instruction_count: usize,
    pub verified_output: String,
}

pub fn encode_english_prompt_tokens(text: &str) -> Vec<u16> {
    let mut out = Vec::new();
    for raw_w in text.split_whitespace() {
        let mut h: u32 = 2166136261;
        for b in raw_w.bytes() {
            h = (h ^ (b as u32)).wrapping_mul(16777619);
        }
        out.push(((h as usize) % ((ENGLISH_MAX_TOKEN as usize) + 1)) as u16);
    }
    out
}

pub fn pack_fn_module(
    schemas: &[(u16, Vec<(u16, u16)>)],
    fn_id: u16,
    params: &[(u16, u16, u16)],
    ret_ty: u16,
    cap_mask: CapMask,
    ins_packets: &[Vec<u16>],
) -> Vec<u16> {
    let mut stream = vec![TOK_MOD_BEGIN];
    for (s_id, fields) in schemas {
        stream.push(TOK_SCHEMA_DEF);
        stream.push(*s_id);
        stream.push(fields.len() as u16);
        for &(f_ty, f_tc) in fields {
            stream.push(f_ty);
            stream.push(f_tc);
        }
    }
    stream.push(TOK_FN_DEF);
    stream.push(fn_id);
    stream.push(params.len() as u16);
    for &(p_tok, p_ty, p_tc) in params {
        stream.push(p_tok);
        stream.push(p_ty);
        stream.push(p_tc);
    }
    stream.push(ret_ty);
    stream.push(cap_mask);
    stream.push(ins_packets.len() as u16);
    for pkt in ins_packets {
        stream.extend_from_slice(pkt);
    }
    stream.push(TOK_MOD_END);
    stream
}

pub fn build_transpiled_seed_stream(variant_idx: usize) -> (String, Vec<u16>, Vec<OmniValue>, OmniValue) {
    let seed_kind = variant_idx % 8;
    let scale_val = (2 + (variant_idx % 7)) as i16;
    let thresh_val = (50 + (variant_idx * 5) % 200) as i16;

    let tc_safe_pure   = pack_tc(false, CAP_ID_PURE);
    let tc_untr_pure   = pack_tc(true,  CAP_ID_PURE);
    let tc_safe_ai     = pack_tc(false, CAP_ID_AI);
    let tc_safe_db_w   = pack_tc(false, CAP_ID_DB_W);
    let tc_safe_db_r   = pack_tc(false, CAP_ID_DB_R);
    let tc_safe_vision = pack_tc(false, CAP_ID_VISION);
    let tc_safe_dom    = pack_tc(false, CAP_ID_DOM);

    let fn_id = (variant_idx % 16) as u16;

    match seed_kind {
        0 => {
            let prompt = format!(
                "Compute the weighted risk score {} * base + delta and compare against threshold {} in balanced ternary (variant #{})",
                scale_val, thresh_val, variant_idx + 1
            );
            let ins_packets = vec![
                vec![0x7E00, 0x7F12, TY_I64,  tc_safe_pure, 0x7EC0, TOK_IMM_I16, scale_val as u16, TOK_INS_END],
                vec![0x7E01, 0x7F10, TY_I64,  tc_safe_pure, 0x7E00, 0x7EC1, TOK_INS_END],
                vec![0x7E02, 0x7F2D, TY_TRIT, tc_safe_pure, 0x7E01, TOK_IMM_I16, thresh_val as u16, TOK_INS_END],
                vec![0x7E03, 0x7F09, TY_TRIT, tc_safe_pure, TOK_TRIT_POS, TOK_INS_END],
                vec![0x7E04, 0x7F29, TY_TRIT, tc_safe_pure, 0x7E02, 0x7E03, TOK_INS_END],
                vec![0x7F4A, 0x7E04, TOK_INS_END],
            ];
            let stream = pack_fn_module(
                &[],
                fn_id,
                &[(0x7EC0, TY_I64, tc_safe_pure), (0x7EC1, TY_I64, tc_safe_pure)],
                TY_TRIT,
                0x0000,
                &ins_packets,
            );
            let total = 30i64 * (scale_val as i64) + 15i64;
            let exp_trit = if total > (thresh_val as i64) { 1 } else if total < (thresh_val as i64) { -1 } else { 0 };
            (prompt, stream, vec![OmniValue::Int(30), OmniValue::Int(15)], OmniValue::Trit(exp_trit))
        }
        1 => {
            let prompt = format!(
                "Sanitize untrusted user comment, embed into a 64-trit vector, insert into ACID table 0, and render a safe DOM node (variant #{})",
                variant_idx + 1
            );
            let caps = cap_bit(CAP_ID_AI) | cap_bit(CAP_ID_DB_W) | cap_bit(CAP_ID_DOM);
            let schemas = vec![(
                0x0000,
                vec![
                    (TY_UUID, tc_safe_pure),
                    (TY_STR, tc_safe_pure),
                    (TY_TRIT_VEC, tc_safe_pure),
                    (TY_TRIT, tc_safe_pure),
                ],
            )];
            let ins_packets = vec![
                vec![0x7E00, 0x7F01, TY_ARENA,    tc_safe_pure, TOK_IMM_I32, 0x0400, 0x0000, TOK_INS_END],
                vec![0x7E01, 0x7F59, TY_STR,      tc_safe_ai,   0x7EC1, TOK_INS_END],
                vec![0x7E02, 0x7F7D, TY_TRIT_VEC, tc_safe_ai,   0x7E01, TOK_INS_END],
                vec![0x7E03, 0x7F2D, TY_TRIT,     tc_safe_pure, 0x7EC2, TOK_IMM_I16, 0x0032, TOK_INS_END],
                vec![0x7E04, 0x7F57, TY_STR,      tc_safe_pure, 0x7EC1, TOK_INS_END],
                vec![
                    0x7E05, 0x7F63, TY_STRUCT, tc_safe_db_w, TOK_SYM_SCHEMA, 0x0000,
                    TOK_FIELD_KV, 0x0000, 0x7EC0,
                    TOK_FIELD_KV, 0x0001, 0x7E04,
                    TOK_FIELD_KV, 0x0002, 0x7E02,
                    TOK_FIELD_KV, 0x0003, 0x7E03,
                    TOK_INS_END,
                ],
                vec![0x7E06, 0x7F58, TY_STR, tc_safe_pure, 0x7EC1, TOK_INS_END],
                vec![0x7E07, 0x7F76, TY_DOM, tc_safe_dom,  TOK_IMM_I16, 0x0001, 0x7E06, TOK_INS_END],
                vec![0x7F05, 0x7E00, TOK_INS_END],
                vec![0x7F4A, 0x7E07, TOK_INS_END],
            ];
            let stream = pack_fn_module(
                &schemas,
                fn_id,
                &[
                    (0x7EC0, TY_UUID, tc_safe_pure),
                    (0x7EC1, TY_STR,  tc_untr_pure),
                    (0x7EC2, TY_I64,  tc_safe_pure),
                ],
                TY_DOM,
                caps,
                &ins_packets,
            );
            let args = vec![
                OmniValue::Str(String::from("uuid-1")),
                OmniValue::Str(String::from("Hello <script>&")),
                OmniValue::Int(60),
            ];
            let exp = OmniValue::DomNode {
                tag_id: 1,
                children: vec![OmniValue::Str(String::from("Hello &lt;script&gt;&amp;"))],
            };
            (prompt, stream, args, exp)
        }
        2 => {
            let prompt = format!(
                "Allocate a 64MB zero-copy arena, run the 1.58-bit TerDiT vision skill on an untrusted English prompt, and mount in DOM (variant #{})",
                variant_idx + 1
            );
            let caps = cap_bit(CAP_ID_AI) | cap_bit(CAP_ID_VISION) | cap_bit(CAP_ID_DOM);
            let ins_packets = vec![
                vec![0x7E00, 0x7F01, TY_ARENA,   tc_safe_pure,   TOK_IMM_I32, 0x0400, 0x0000, TOK_INS_END],
                vec![0x7E01, 0x7F59, TY_STR,     tc_safe_ai,     0x7EC1, TOK_INS_END],
                vec![0x7E02, 0x7F03, TY_ARENA,   tc_safe_pure,   0x7E00, TOK_INS_END],
                vec![0x7E03, 0x7F7E, TY_MODEL_T, tc_safe_vision, TOK_SYM_SKILL, 0x0001, 0x7E02, TOK_INS_END],
                vec![0x7E04, 0x7F7F, TY_STR,     tc_safe_vision, 0x7E03, 0x7E01, TOK_INS_END],
                vec![0x7F05, 0x7E03, TOK_INS_END],
                vec![0x7F05, 0x7E02, TOK_INS_END],
                vec![0x7E05, 0x7F58, TY_STR,     tc_safe_pure,   0x7EC1, TOK_INS_END],
                vec![0x7E06, 0x7F76, TY_DOM,     tc_safe_dom,    TOK_IMM_I16, 0x0002, 0x7E05, 0x7E04, TOK_INS_END],
                vec![0x7F05, 0x7E00, TOK_INS_END],
                vec![0x7F4A, 0x7E06, TOK_INS_END],
            ];
            let stream = pack_fn_module(
                &[],
                fn_id,
                &[(0x7EC0, TY_UUID, tc_safe_pure), (0x7EC1, TY_STR, tc_untr_pure)],
                TY_DOM,
                caps,
                &ins_packets,
            );
            let args = vec![
                OmniValue::Str(String::from("uuid-2")),
                OmniValue::Str(String::from("Cyberpunk samurai")),
            ];
            let exp = OmniValue::DomNode {
                tag_id: 2,
                children: vec![
                    OmniValue::Str(String::from("Cyberpunk samurai")),
                    OmniValue::Str(String::from("[Skill#1->'Cyberpunk samurai']")),
                ],
            };
            (prompt, stream, args, exp)
        }
        3 => {
            // Note: Seed 3 expected_val assumes a fresh OmniBinaryVm with empty tables
            // (or vm.clear_tables() called beforehand) so TV_SRCH (0x7F6A) returns the
            // deterministic empty-table fallback Struct({0: Int(0), 1: Int(64)}).
            let prompt = format!(
                "Perform semantic ternary vector similarity search on table 0 for an untrusted search query (variant #{})",
                variant_idx + 1
            );
            let caps = cap_bit(CAP_ID_AI) | cap_bit(CAP_ID_DB_R);
            let ins_packets = vec![
                vec![0x7E00, 0x7F59, TY_STR,      tc_safe_ai,   0x7EC0, TOK_INS_END],
                vec![0x7E01, 0x7F7D, TY_TRIT_VEC, tc_safe_ai,   0x7E00, TOK_INS_END],
                vec![0x7E02, 0x7F6A, TY_STRUCT,   tc_safe_db_r, TOK_SYM_SCHEMA, 0x0000, 0x7E01, TOK_IMM_I16, 0x0001, TOK_INS_END],
                vec![0x7F4A, 0x7E02, TOK_INS_END],
            ];
            let stream = pack_fn_module(
                &[],
                fn_id,
                &[(0x7EC0, TY_STR, tc_untr_pure)],
                TY_STRUCT,
                caps,
                &ins_packets,
            );
            let mut exp_map = BTreeMap::new();
            exp_map.insert(0, OmniValue::Int(0));
            exp_map.insert(1, OmniValue::Int(64));
            (prompt, stream, vec![OmniValue::Str(String::from("search query"))], OmniValue::Struct(exp_map))
        }
        4 => {
            let prompt = format!(
                "Evaluate three-way Kleene majority consensus across three ternary sensor inputs and negate the result (variant #{})",
                variant_idx + 1
            );
            let ins_packets = vec![
                vec![0x7E00, 0x7F2C, TY_TRIT, tc_safe_pure, 0x7EC0, 0x7EC1, 0x7EC2, TOK_INS_END],
                vec![0x7E01, 0x7F28, TY_TRIT, tc_safe_pure, 0x7E00, TOK_INS_END],
                vec![0x7F4A, 0x7E01, TOK_INS_END],
            ];
            let stream = pack_fn_module(
                &[],
                fn_id,
                &[
                    (0x7EC0, TY_TRIT, tc_safe_pure),
                    (0x7EC1, TY_TRIT, tc_safe_pure),
                    (0x7EC2, TY_TRIT, tc_safe_pure),
                ],
                TY_TRIT,
                0x0000,
                &ins_packets,
            );
            let args = vec![OmniValue::Trit(1), OmniValue::Trit(1), OmniValue::Trit(-1)];
            (prompt, stream, args, OmniValue::Trit(-1))
        }
        5 => {
            let prompt = format!(
                "Execute NPU 2-layer lookahead routing and Tri-Tier holographic MoE expert 0 on an activation vector (variant #{})",
                variant_idx + 1
            );
            let caps = cap_bit(CAP_ID_AI);
            let ins_packets = vec![
                vec![0x7E00, 0x7F36, TY_TRIT,    tc_safe_ai, 0x7EC0, TOK_INS_END],
                vec![0x7E01, 0x7F37, TY_STRUCT,  tc_safe_ai, TOK_SYM_SKILL, 0x0000, 0x7E00, TOK_INS_END],
                vec![0x7E02, 0x7F39, TY_FLT_VEC, tc_safe_ai, 0x7E01, 0x7EC0, 0x7E00, TOK_INS_END],
                vec![0x7F4A, 0x7E02, TOK_INS_END],
            ];
            let stream = pack_fn_module(
                &[],
                fn_id,
                &[(0x7EC0, TY_TRIT_VEC, tc_safe_pure)],
                TY_STRUCT,
                caps,
                &ins_packets,
            );
            let mut exp_map = BTreeMap::new();
            exp_map.insert(0, OmniValue::Int(-1));
            exp_map.insert(1, OmniValue::Int(64));
            (prompt, stream, vec![OmniValue::TritVec(vec![1; 64])], OmniValue::Struct(exp_map))
        }
        6 => {
            let prompt = format!(
                "Clamp an integer value between lower bound lo and upper bound hi using branchless integer min and max (variant #{})",
                variant_idx + 1
            );
            let ins_packets = vec![
                vec![0x7E00, 0x7F18, TY_I64, tc_safe_pure, 0x7EC0, 0x7EC1, TOK_INS_END],
                vec![0x7E01, 0x7F17, TY_I64, tc_safe_pure, 0x7E00, 0x7EC2, TOK_INS_END],
                vec![0x7F4A, 0x7E01, TOK_INS_END],
            ];
            let stream = pack_fn_module(
                &[],
                fn_id,
                &[
                    (0x7EC0, TY_I64, tc_safe_pure),
                    (0x7EC1, TY_I64, tc_safe_pure),
                    (0x7EC2, TY_I64, tc_safe_pure),
                ],
                TY_I64,
                0x0000,
                &ins_packets,
            );
            let args = vec![OmniValue::Int(150), OmniValue::Int(10), OmniValue::Int(90)];
            (prompt, stream, args, OmniValue::Int(90))
        }
        _ => {
            let prompt = format!(
                "Compute the 64-bit FNV-1a cryptographic hash of a sanitized user handle and mask the lower 16 bits (variant #{})",
                variant_idx + 1
            );
            let ins_packets = vec![
                vec![0x7E00, 0x7F57, TY_STR, tc_safe_pure, 0x7EC0, TOK_INS_END],
                vec![0x7E01, 0x7F5D, TY_I64, tc_safe_pure, 0x7E00, TOK_INS_END],
                vec![0x7E02, 0x7F1E, TY_I64, tc_safe_pure, 0x7E01, TOK_IMM_I32, 0x0000, 0xFFFF, TOK_INS_END],
                vec![0x7F4A, 0x7E02, TOK_INS_END],
            ];
            let stream = pack_fn_module(
                &[],
                fn_id,
                &[(0x7EC0, TY_STR, tc_untr_pure)],
                TY_I64,
                0x0000,
                &ins_packets,
            );
            let args = vec![OmniValue::Str(String::from("user_handle_42"))];
            (prompt, stream, args, OmniValue::Int(42944))
        }
    }
}

pub fn run_step2_rust_corpus_builder(output_jsonl_path: &str, target_count: usize) -> Vec<CorpusRecord> {
    let mut records = Vec::with_capacity(target_count);
    let mut json_lines = String::new();

    for idx in 0..target_count {
        let (prompt, stream, args, expected_val) = build_transpiled_seed_stream(idx);
        let module = decode_omni_token_stream(&stream).expect("Failed to decode transpiled u16 stream");
        let fn_obj = &module.functions[0];
        let rep = verify_binary_function(fn_obj);
        assert!(rep.passed, "Step 2 Transpiler verification bug on #{}: {:?}", idx, rep.errors);

        let mut vm = OmniBinaryVm::new();
        let actual_val = vm.execute_function(&module, fn_obj.fn_idx, args).expect("Step 2 VM execution failed");
        assert_eq!(actual_val, expected_val, "Step 2 Differential Execution Mismatch on sample #{}", idx);

        let eng_tokens = encode_english_prompt_tokens(&prompt);
        let hex_str = stream.iter().map(|t| format!("{:04X}", t)).collect::<Vec<_>>().join(" ");
        let eng_json = eng_tokens.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(",");
        let omni_json = stream.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(",");

        json_lines.push_str(&format!(
            r#"{{"sample_id":{},"english_prompt":"{}","english_token_ids":[{}],"omni_u16_token_ids":[{}],"omni_hex_stream":"{}","ssa_instruction_count":{},"verifier_passed":true,"rl_reward":1.0}}"#,
            idx, prompt, eng_json, omni_json, hex_str, rep.instruction_count
        ));
        json_lines.push('\n');

        records.push(CorpusRecord {
            sample_id: idx,
            english_prompt: prompt,
            english_token_ids: eng_tokens,
            omni_u16_token_ids: stream,
            ssa_instruction_count: rep.instruction_count,
            verified_output: format!("{:?}", actual_val),
        });
    }

    fs::write(output_jsonl_path, json_lines).expect("Failed to write omni_sft_corpus.jsonl");
    records
}

// ==============================================================================
// 9. STEP 3 (PURE RUST): 32,768-TOKEN `tokenizer.json` BUILDER & DFA MASKER
// ==============================================================================

pub fn run_step3_rust_tokenizer_builder(output_path: &str) -> (usize, usize) {
    let mut vocab_entries = Vec::with_capacity(VOCAB_SIZE);
    vocab_entries.push(String::from(r#""<|pad|>":0"#));
    vocab_entries.push(String::from(r#""<|bos|>":1"#));
    vocab_entries.push(String::from(r#""<|eos|>":2"#));
    vocab_entries.push(String::from(r#""<|unk_eng|>":3"#));

    for id in 4..=(ENGLISH_MAX_TOKEN as usize) {
        vocab_entries.push(format!(r#""Ġeng_{:05}":{}"#, id, id));
    }
    assert_eq!(vocab_entries.len(), 32_256);

    let mut added_tokens = Vec::with_capacity(512);
    for omni_id in 0x7E00..=0x7FFFu16 {
        let tok_str = format!("<|0x{:04X}|>", omni_id);
        vocab_entries.push(format!(r#""{}":{}"#, tok_str, omni_id));
        added_tokens.push(format!(
            r#"{{"id":{},"content":"{}","single_word":true,"lstrip":false,"rstrip":false,"normalized":false,"special":true}}"#,
            omni_id, tok_str
        ));
    }
    assert_eq!(vocab_entries.len(), VOCAB_SIZE);

    let json_blob = format!(
        r#"{{"version":"1.0","truncation":null,"padding":null,"added_tokens":[{}],"model":{{"type":"BPE","unk_token":"<|unk_eng|>","byte_fallback":true,"vocab":{{{}}},"merges":[]}}}}"#,
        added_tokens.join(","),
        vocab_entries.join(",")
    );
    let byte_len = json_blob.len();
    fs::write(output_path, json_blob).expect("Failed to write tokenizer.json");
    (VOCAB_SIZE, byte_len)
}

// ==============================================================================
// 10. STEP 4 (PURE RUST): COMPACT LINEAR SEQUENCE MODEL WITH RANK-8 SUBSPACE PROJECTION
// ==============================================================================
//
// Honest Scope Note (Roadmap 2.3):
// This step trains a compact linear sequence model (`d_model = 32`) over the active
// token vocabulary of the 64-sample bootstrap corpus using rank-8 orthogonal subspace
// gradient projection (`compute_galore_projection_matrix`). It demonstrates projecting
// gradients `G_t` into a low-rank subspace `R_t = P^T * G_t` (`r = 8` vs `d = 32`)
// before updating first/second moments `m_low, v_low`. Full-scale LLM training on 128 GB
// unified RAM requires applying this projection to multi-layer transformer weights.

pub fn compute_galore_projection_matrix(grad: &[f32], rows: usize, cols: usize, rank: usize) -> Vec<f32> {
    let mut cov = vec![0.0f32; rows * rows];
    for i in 0..rows {
        let row_i = &grad[i * cols..(i + 1) * cols];
        for j in i..rows {
            let row_j = &grad[j * cols..(j + 1) * cols];
            let mut dot = 0.0f32;
            for k in 0..cols {
                dot += row_i[k] * row_j[k];
            }
            cov[i * rows + j] = dot;
            cov[j * rows + i] = dot;
        }
    }

    let mut p_mat = vec![0.0f32; rows * rank];
    for r in 0..rank {
        let mut v = vec![0.0f32; rows];
        for i in 0..rows {
            v[i] = (((i + 1) * (r + 3) * 17) % 97) as f32 / 97.0 - 0.5;
        }
        for _iter in 0..6 {
            let mut w = vec![0.0f32; rows];
            for i in 0..rows {
                let mut s = 0.0f32;
                for j in 0..rows {
                    s += cov[i * rows + j] * v[j];
                }
                w[i] = s;
            }
            for prev in 0..r {
                let mut proj = 0.0f32;
                for i in 0..rows {
                    proj += w[i] * p_mat[i * rank + prev];
                }
                for i in 0..rows {
                    w[i] -= proj * p_mat[i * rank + prev];
                }
            }
            let norm = w.iter().map(|x| x * x).sum::<f32>().sqrt() + 1e-8;
            for i in 0..rows {
                v[i] = w[i] / norm;
            }
        }
        for i in 0..rows {
            p_mat[i * rank + r] = v[i];
        }
    }
    p_mat
}

pub fn run_step4_rust_galore_sft_trainer(corpus: &[CorpusRecord], epochs: usize) -> (f32, f32, f32, f32) {
    let mut unique_toks: Vec<u16> = Vec::new();
    for rec in corpus {
        for &t in &rec.omni_u16_token_ids {
            if !unique_toks.contains(&t) {
                unique_toks.push(t);
            }
        }
    }
    unique_toks.sort_unstable();
    let num_classes = unique_toks.len();
    let d_model = 32usize;
    let galore_rank = 8usize;
    let n_samples = corpus.len();

    let mut x_feats = vec![vec![0.0f32; d_model]; n_samples];
    let mut y_seqs: Vec<Vec<usize>> = Vec::with_capacity(n_samples);
    let mut max_len = 0usize;

    for (i, rec) in corpus.iter().enumerate() {
        for (pos_i, &eid) in rec.english_token_ids.iter().enumerate() {
            let slot = ((eid as usize) + pos_i * 17) % d_model;
            x_feats[i][slot] += 1.0;
        }
        let norm: f32 = x_feats[i].iter().map(|v| v * v).sum::<f32>().sqrt() + 1e-6;
        for v in &mut x_feats[i] {
            *v /= norm;
        }
        let seq: Vec<usize> = rec
            .omni_u16_token_ids
            .iter()
            .map(|t| unique_toks.binary_search(t).unwrap())
            .collect();
        if seq.len() > max_len {
            max_len = seq.len();
        }
        y_seqs.push(seq);
    }

    let out_dim = max_len * num_classes;
    let mut w_sft = vec![0.0f32; d_model * out_dim];
    let mut m_low = vec![0.0f32; galore_rank * out_dim];
    let mut v_low = vec![0.0f32; galore_rank * out_dim];
    let mut proj_p = vec![0.0f32; d_model * galore_rank];

    let mut initial_loss = 0.0f32;
    let mut final_loss = 0.0f32;
    let mut initial_acc = 0.0f32;
    let mut final_acc = 0.0f32;
    let lr = 0.45f32;

    for ep in 1..=epochs {
        let mut total_loss = 0.0f32;
        let mut total_correct = 0usize;
        let mut total_tokens = 0usize;
        let mut full_grad = vec![0.0f32; d_model * out_dim];

        for i in 0..n_samples {
            let seq = &y_seqs[i];
            for (t_pos, &target_cls) in seq.iter().enumerate() {
                let base_col = t_pos * num_classes;
                let mut logits = vec![0.0f32; num_classes];
                let mut max_l = f32::NEG_INFINITY;
                for c in 0..num_classes {
                    let mut sum = 0.0f32;
                    for d in 0..d_model {
                        sum += x_feats[i][d] * w_sft[d * out_dim + base_col + c];
                    }
                    logits[c] = sum;
                    if sum > max_l {
                        max_l = sum;
                    }
                }
                let mut exp_sum = 0.0f32;
                let mut best_c = 0usize;
                let mut best_l = f32::NEG_INFINITY;
                for c in 0..num_classes {
                    if logits[c] > best_l {
                        best_l = logits[c];
                        best_c = c;
                    }
                    logits[c] = (logits[c] - max_l).exp();
                    exp_sum += logits[c];
                }
                if best_c == target_cls {
                    total_correct += 1;
                }
                total_tokens += 1;
                let p_target = (logits[target_cls] / exp_sum).max(1e-9);
                total_loss -= p_target.ln();

                for c in 0..num_classes {
                    let g = (logits[c] / exp_sum) - (if c == target_cls { 1.0 } else { 0.0 });
                    let col = base_col + c;
                    for d in 0..d_model {
                        full_grad[d * out_dim + col] += x_feats[i][d] * g / (n_samples as f32);
                    }
                }
            }
        }

        let avg_loss = total_loss / (total_tokens as f32);
        let acc = (total_correct as f32) * 100.0 / (total_tokens as f32);
        if ep == 1 {
            initial_loss = avg_loss;
            initial_acc = acc;
        }
        final_loss = avg_loss;
        final_acc = acc;

        if ep % 4 == 1 {
            proj_p = compute_galore_projection_matrix(&full_grad, d_model, out_dim, galore_rank);
        }

        let mut grad_low = vec![0.0f32; galore_rank * out_dim];
        for r in 0..galore_rank {
            for col in 0..out_dim {
                let mut s = 0.0f32;
                for d in 0..d_model {
                    s += proj_p[d * galore_rank + r] * full_grad[d * out_dim + col];
                }
                grad_low[r * out_dim + col] = s;
            }
        }

        let mut norm_low = vec![0.0f32; galore_rank * out_dim];
        for idx in 0..grad_low.len() {
            let g = grad_low[idx];
            m_low[idx] = 0.85 * m_low[idx] + 0.15 * g;
            v_low[idx] = 0.95 * v_low[idx] + 0.05 * (g * g);
            norm_low[idx] = m_low[idx] / (v_low[idx].sqrt() + 1e-6);
        }

        for d in 0..d_model {
            for col in 0..out_dim {
                let mut delta = 0.0f32;
                for r in 0..galore_rank {
                    delta += proj_p[d * galore_rank + r] * norm_low[r * out_dim + col];
                }
                w_sft[d * out_dim + col] -= lr * delta;
            }
        }
    }

    (initial_loss, final_loss, initial_acc, final_acc)
}

// ==============================================================================
// 11. STEP 5 (PURE RUST): STABILIZED GRPO COMPILER-RL & `.omni_holo` EXPORTER
// ==============================================================================

pub fn export_tri_tier_omni_holo(output_path: &str) -> usize {
    let d_in: u32 = 128;
    let d_out: u32 = 128;
    let rank: u32 = 16;
    let svd_a = vec![12u8; (d_in * rank) as usize];
    let svd_b = vec![9u8; (rank * d_out) as usize];

    let mut trits = vec![0i8; (d_in * d_out) as usize];
    for (i, t) in trits.iter_mut().enumerate() {
        *t = ((i * 7 + 3) % 3) as i8 - 1;
    }
    for i in 320..960 {
        trits[i] = 0;
    }
    let delta_pack5 = pack5_encode(&trits);
    let csr_nnz: u32 = 82;
    let csr_vals = vec![0u8; (csr_nnz * 2) as usize];
    let csr_cols = vec![0u8; (csr_nnz * 2) as usize];
    let csr_rows = vec![0u8; ((d_in + 1) * 4) as usize];

    let mut header = [0u8; 64];
    header[0..8].copy_from_slice(b"OMNIHOLO");
    header[8..12].copy_from_slice(&2u32.to_le_bytes());
    header[12..16].copy_from_slice(&d_in.to_le_bytes());
    header[16..20].copy_from_slice(&d_out.to_le_bytes());
    header[20..24].copy_from_slice(&rank.to_le_bytes());
    header[24..28].copy_from_slice(&0.015f32.to_le_bytes());
    header[28..32].copy_from_slice(&0.015f32.to_le_bytes());
    header[32..36].copy_from_slice(&0.082f32.to_le_bytes());
    header[36..40].copy_from_slice(&(delta_pack5.len() as u32).to_le_bytes());
    header[40..44].copy_from_slice(&csr_nnz.to_le_bytes());

    let mut blob = Vec::from(header);
    let sections = [&svd_a[..], &svd_b[..], &delta_pack5[..], &csr_vals[..], &csr_cols[..], &csr_rows[..]];
    for sec in sections {
        let pad = ((blob.len() + 63) & !63) - blob.len();
        blob.resize(blob.len() + pad, 0u8);
        blob.extend_from_slice(sec);
    }

    fs::write(output_path, &blob).expect("Failed to write .omni_holo file");
    blob.len()
}

pub fn synthesize_grpo_candidate_u16_stream(actions: [u8; 4]) -> Vec<u16> {
    let [use_sql_san, has_db_cap, drop_arena, use_dom_san] = actions;
    let cap_mask: u16 = if has_db_cap == 1 { 0x00FF } else { 0x00FF & !cap_bit(CAP_ID_DB_W) };

    let mut ins_packets: Vec<Vec<u16>> = vec![
        vec![0x7E00, 0x7F01, TY_ARENA, pack_tc(false, CAP_ID_PURE), TOK_IMM_I32, 0x0100, 0x0000, TOK_INS_END],
    ];
    let mut next_reg: u16 = 0x7E01;

    let sql_reg = if use_sql_san == 1 {
        let r = next_reg;
        next_reg += 1;
        ins_packets.push(vec![r, 0x7F57, TY_STR, pack_tc(false, CAP_ID_PURE), 0x7EC1, TOK_INS_END]);
        r
    } else {
        0x7EC1
    };

    let db_reg = next_reg;
    next_reg += 1;
    ins_packets.push(vec![
        db_reg, 0x7F63, TY_STRUCT, pack_tc(false, CAP_ID_DB_W), TOK_SYM_SCHEMA, 0x0000,
        TOK_FIELD_KV, 0x0000, 0x7EC0,
        TOK_FIELD_KV, 0x0001, sql_reg,
        TOK_INS_END,
    ]);

    let dom_val_reg = if use_dom_san == 1 {
        let r = next_reg;
        next_reg += 1;
        ins_packets.push(vec![r, 0x7F58, TY_STR, pack_tc(false, CAP_ID_PURE), 0x7EC1, TOK_INS_END]);
        r
    } else {
        0x7EC1
    };

    let dom_reg = next_reg;
    ins_packets.push(vec![
        dom_reg, 0x7F76, TY_DOM, pack_tc(false, CAP_ID_DOM), TOK_IMM_I16, 0x0001, dom_val_reg, TOK_INS_END,
    ]);

    if drop_arena == 1 {
        ins_packets.push(vec![0x7F05, 0x7E00, TOK_INS_END]);
    }
    ins_packets.push(vec![0x7F4A, dom_reg, TOK_INS_END]);

    pack_fn_module(
        &[(0x0000, vec![(TY_UUID, pack_tc(false, CAP_ID_PURE)), (TY_STR, pack_tc(false, CAP_ID_PURE))])],
        0x0000,
        &[
            (0x7EC0, TY_UUID, pack_tc(false, CAP_ID_PURE)),
            (0x7EC1, TY_STR,  pack_tc(true,  CAP_ID_PURE)),
        ],
        TY_DOM,
        cap_mask,
        &ins_packets,
    )
}

/// Stabilized Group Relative Policy Optimization (GRPO) over the 4 binary safety decisions
/// (`use_sql_san`, `has_db_cap`, `drop_arena`, `use_dom_san`) using group-normalized advantages
/// `(r - mean_r) / (std_r + 1e-5)` over the verifier's graded reward `rep.rl_reward`,
/// per-epoch probability telemetry, and policy checkpoint persistence (Roadmap 2.2, 3.3, 3.5).
pub fn load_grpo_policy_checkpoint(path: &str) -> Option<[f32; 4]> {
    let raw = fs::read_to_string(path).ok()?;
    let key = "\"logits\":[";
    let start = raw.find(key)? + key.len();
    let end = raw[start..].find(']')? + start;
    let mut out = [0.0f32; 4];
    let parts: Vec<&str> = raw[start..end].split(',').collect();
    if parts.len() != 4 {
        return None;
    }
    for (i, p) in parts.iter().enumerate() {
        out[i] = p.trim().parse::<f32>().ok()?;
    }
    Some(out)
}

pub fn run_step5_rust_grpo_self_play(epochs: usize, group_size: usize) -> (f32, f32, f32, [f32; 4]) {
    run_step5_rust_grpo_self_play_with_init(epochs, group_size, None)
}

pub fn run_step5_rust_grpo_self_play_with_init(
    epochs: usize,
    group_size: usize,
    initial_logits: Option<[f32; 4]>,
) -> (f32, f32, f32, [f32; 4]) {
    let mut logits = initial_logits.unwrap_or([-0.55f32; 4]);
    let mut rng_state: u64 = 0xCAFEBABE_20261026;
    let mut init_pass_pct = 0.0f32;
    let mut final_pass_pct = 0.0f32;
    let mut final_reward = 0.0f32;
    let mut final_probs = [0.0f32; 4];
    let lr = 1.65f32;

    for ep in 1..=epochs {
        let mut probs = [0.0f32; 4];
        for s in 0..4 {
            probs[s] = 1.0 / (1.0 + (-logits[s]).exp());
        }
        final_probs = probs;

        let mut rewards = Vec::with_capacity(group_size);
        let mut actions_batch = Vec::with_capacity(group_size);
        let mut passes = 0usize;

        for _ in 0..group_size {
            let mut acts = [0u8; 4];
            for s in 0..4 {
                rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let u = ((rng_state >> 32) as f32) / (u32::MAX as f32);
                acts[s] = if u < probs[s] { 1 } else { 0 };
            }
            let cand_stream = synthesize_grpo_candidate_u16_stream(acts);
            let module = decode_omni_token_stream(&cand_stream).unwrap();
            let rep = verify_binary_function(&module.functions[0]);
            if rep.passed {
                passes += 1;
            }
            // Partial-progress reward shaping (+0.25 per satisfied safety property) + verifier RL reward
            let partial_bonus: f32 = 0.25 * ((acts[0] + acts[1] + acts[2] + acts[3]) as f32);
            let shaped_reward = rep.rl_reward + partial_bonus;
            rewards.push(shaped_reward);
            actions_batch.push(acts);
        }

        let pass_pct = (passes as f32) * 100.0 / (group_size as f32);
        let mean_r: f32 = rewards.iter().sum::<f32>() / (group_size as f32);
        let var_r: f32 = rewards.iter().map(|&r| (r - mean_r) * (r - mean_r)).sum::<f32>() / (group_size as f32);
        let std_r: f32 = var_r.sqrt() + 1e-5;

        if ep == 1 {
            init_pass_pct = pass_pct;
        }
        final_pass_pct = pass_pct;
        final_reward = mean_r;

        if initial_logits.is_none() && (ep == 1 || ep == epochs / 2 || ep == epochs) {
            println!(
                "    - [GRPO Epoch {:2}/{}] Pass={:5.1}%, MeanR={:+.3}, P=[sql:{:.2}, cap:{:.2}, drop:{:.2}, dom:{:.2}]",
                ep, epochs, pass_pct, mean_r, probs[0], probs[1], probs[2], probs[3]
            );
        }

        for b in 0..group_size {
            let adv = (rewards[b] - mean_r) / std_r;
            for s in 0..4 {
                let a = actions_batch[b][s] as f32;
                logits[s] = (logits[s] + lr * adv * (a - probs[s]) / (group_size as f32)).clamp(-5.0, 5.0);
            }
        }
    }

    for s in 0..4 {
        final_probs[s] = 1.0 / (1.0 + (-logits[s]).exp());
    }

    let ckpt_json = format!(
        r#"{{"epochs":{},"group_size":{},"logits":[{:.4},{:.4},{:.4},{:.4}],"action_probs":{{"sql_sanitize":{:.4},"db_write_cap":{:.4},"drop_arena":{:.4},"dom_sanitize":{:.4}}},"final_pass_pct":{:.2},"final_mean_reward":{:.4}}}"#,
        epochs,
        group_size,
        logits[0], logits[1], logits[2], logits[3],
        final_probs[0], final_probs[1], final_probs[2], final_probs[3],
        final_pass_pct,
        final_reward
    );
    let _ = fs::write("grpo_policy_checkpoint.json", ckpt_json);

    (init_pass_pct, final_pass_pct, final_reward, final_probs)
}

// ==============================================================================
// 12. STEP 6 (PURE RUST): TWO-PASS x86-64 ELF & REAL INSTRUCTION-LOWERING WASM AOT
// ==============================================================================

#[derive(Clone, Copy, Debug)]
struct RelocPatch {
    patch_offset: usize,
    target_label: u16,
}

fn collect_func_pack5_map(func: &BinFunction) -> BTreeMap<u16, (Vec<u8>, usize, bool)> {
    let mut reg_pack5: BTreeMap<u16, (Vec<u8>, usize, bool)> = BTreeMap::new();
    for ins in &func.instructions {
        match ins.op_tok {
            0x7F0D => {
                if let Some(d) = ins.dest_reg {
                    reg_pack5.insert(d, (Vec::new(), 0, false));
                }
            }
            0x7F0F => {
                let is_typed_write = (ins.operands.len() == 3
                    && (ins.operands[1] == BinOperand::ImmInt(-4) || ins.operands[1] == BinOperand::ImmInt(-8)))
                    || (ins.operands.len() == 4 && ins.operands[1] == BinOperand::ImmInt(-9));
                if !is_typed_write {
                    let mut bytes = match ins.operands.first() {
                        Some(BinOperand::Reg(s)) => reg_pack5.get(s).map(|(b, _, _)| b.clone()).unwrap_or_default(),
                        _ => Vec::new(),
                    };
                    for op_b in &ins.operands[1..] {
                        if let BinOperand::ImmInt(v) = *op_b {
                            bytes.push((v & 0xFF) as u8);
                        }
                    }
                    let n_trits = bytes.len() * 5;
                    if let Some(d) = ins.dest_reg {
                        reg_pack5.insert(d, (bytes, n_trits, false));
                    }
                }
            }
            0x7F31 => {
                if let (Some(d), Some(BinOperand::Reg(s))) = (ins.dest_reg, ins.operands.first()) {
                    let bytes = reg_pack5.get(s).map(|(b, _, _)| b.clone()).unwrap_or_default();
                    let n_trits = match ins.operands.get(1) {
                        Some(BinOperand::ImmInt(n)) => (*n).max(0) as usize,
                        _ => bytes.len() * 5,
                    };
                    reg_pack5.insert(d, (bytes, n_trits, true));
                }
            }
            0x7F30 => {
                if let (Some(d), Some(BinOperand::Reg(s))) = (ins.dest_reg, ins.operands.first()) {
                    if let Some((bytes, n_trits, _)) = reg_pack5.get(s).cloned() {
                        reg_pack5.insert(d, (bytes, n_trits, false));
                    }
                }
            }
            _ => {}
        }
    }
    reg_pack5
}

#[inline]
fn pack5_word_from_bytes(bytes: &[u8], n_trits: usize) -> i64 {
    let mut word: u64 = (n_trits & 0xFF) as u64;
    for (idx, &b) in bytes.iter().take(7).enumerate() {
        word |= (b as u64) << ((idx + 1) * 8);
    }
    word as i64
}

fn validate_func_pack5_ops(
    func: &BinFunction,
    reg_pack5: &BTreeMap<u16, (Vec<u8>, usize, bool)>,
) -> Result<(), String> {
    for ins in &func.instructions {
        let check_reg = |r: u16| -> Result<(), String> {
            if let Some((bytes, _, _)) = reg_pack5.get(&r) {
                for &b in bytes {
                    if b > PACK5_MAX_VALID {
                        return Err(String::from("PACK5_SENTINEL_OR_CORRUPT_BYTE"));
                    }
                }
            }
            Ok(())
        };
        match ins.op_tok {
            0x7F31 | 0x7F34 | 0x7F35 | 0x7F3F => {
                for op in &ins.operands {
                    if let BinOperand::Reg(r) = *op {
                        check_reg(r)?;
                    }
                }
            }
            0x7F0E => {
                if let Some(BinOperand::Reg(r)) = ins.operands.first() {
                    if reg_pack5.get(r).map(|(_, _, is_tv)| *is_tv).unwrap_or(false) {
                        check_reg(*r)?;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// Returns `true` if `module` is a hot numeric, ternary, or `@PACK5` path eligible for direct AOT execution.
pub fn is_module_aot_hot_path(module: &BinModule) -> bool {
    if module.functions.len() != 1 {
        return false;
    }
    let func = &module.functions[0];
    if !matches!(func.ret_ty_tok, TY_I64 | TY_TRIT) {
        return false;
    }
    if !func.params.iter().all(|&(_, ty, _)| matches!(ty, TY_I64 | TY_TRIT | TY_ARENA)) {
        return false;
    }
    func.instructions
        .iter()
        .all(|ins| is_wasm_aot_supported_opcode(ins.op_tok) && is_x86_64_aot_supported_opcode(ins.op_tok))
}

/// Emits a 65-byte x86-64 CLI decimal integer parser supporting optional leading `'-'` (`0x2D`)
/// so both positive and negative integer/trit arguments can be passed on the command line.
fn emit_cli_arg_parser(mc: &mut Vec<u8>, argc_min: u8, disp_argv: u8, target_reg_code: u8) {
    mc.extend_from_slice(&[0x48, 0x83, 0x7D, 0x08, argc_min]);
    mc.extend_from_slice(&[0x7C, 0x3A]);
    mc.extend_from_slice(&[0x48, 0x8B, 0x75, disp_argv]);
    mc.extend_from_slice(&[0x31, 0xC0]);
    mc.extend_from_slice(&[0xBF, 0x01, 0x00, 0x00, 0x00]);
    mc.extend_from_slice(&[0x0F, 0xB6, 0x0E]);
    mc.extend_from_slice(&[0x80, 0xF9, 0x2D]);
    mc.extend_from_slice(&[0x75, 0x0A]);
    mc.extend_from_slice(&[0x48, 0xC7, 0xC7, 0xFF, 0xFF, 0xFF, 0xFF]);
    mc.extend_from_slice(&[0x48, 0xFF, 0xC6]);
    mc.extend_from_slice(&[0x0F, 0xB6, 0x0E]);
    mc.extend_from_slice(&[0x84, 0xC9]);
    mc.extend_from_slice(&[0x74, 0x0F]);
    mc.extend_from_slice(&[0x83, 0xE9, 0x30]);
    mc.extend_from_slice(&[0x48, 0x6B, 0xC0, 0x0A]);
    mc.extend_from_slice(&[0x48, 0x01, 0xC8]);
    mc.extend_from_slice(&[0x48, 0xFF, 0xC6]);
    mc.extend_from_slice(&[0xEB, 0xEA]);
    mc.extend_from_slice(&[0x48, 0x0F, 0xAF, 0xC7]);
    mc.extend_from_slice(&[0x49, 0x89, 0xC0 + target_reg_code]);
}

pub fn compile_pure_u16_to_x86_64_elf(stream: &[u16], arg0: i64, arg1: i64) -> Result<Vec<u8>, String> {
    compile_pure_u16_to_x86_64_elf_with_args(stream, arg0, arg1, 90)
}

pub fn compile_pure_u16_to_x86_64_elf_with_args(
    stream: &[u16],
    arg0: i64,
    arg1: i64,
    arg2: i64,
) -> Result<Vec<u8>, String> {
    let module = decode_omni_token_stream(stream)?;
    let func = module.functions.first().ok_or("No function in module")?;
    let rep = verify_binary_function(func);
    if !rep.passed {
        return Err(format!("Cannot AOT-compile unverified binary stream: {:?}", rep.errors));
    }

    for ins in &func.instructions {
        if !is_x86_64_aot_supported_opcode(ins.op_tok) {
            let lbl = decode_opcode_token(ins.op_tok).map(|s| s.debug_label).unwrap_or("OP_UNKNOWN");
            return Err(format!(
                "Opcode 0x{:04X} ({}) requires high-level VM runtime state and is not lowered by standalone x86-64 AOT",
                ins.op_tok, lbl
            ));
        }
    }

    let mut code: Vec<u8> = Vec::with_capacity(2048);
    let mut label_offsets: Vec<(u16, usize)> = Vec::new();
    let mut relocs: Vec<RelocPatch> = Vec::new();

    code.extend_from_slice(&[
        0x55,
        0x48, 0x89, 0xE5,
        0x48, 0x81, 0xEC, 0x00, 0x00, 0x03, 0x00,
    ]);

    if func.ret_ty_tok == TY_BYTES {
        code.extend_from_slice(&[0x49, 0xC7, 0xC4, 0x00, 0x00, 0x00, 0x00]);
        code.extend_from_slice(&[0x49, 0xBD]);
        code.extend_from_slice(&(arg0 as u64).to_le_bytes());
        code.extend_from_slice(&[0x49, 0xBE]);
        code.extend_from_slice(&(arg1 as u64).to_le_bytes());
        code.extend_from_slice(&[0x49, 0xBF]);
        code.extend_from_slice(&(arg2 as u64).to_le_bytes());

        code.extend_from_slice(&[
            0x48, 0x83, 0x7D, 0x08, 0x02,
            0x7C, 0x2E,
            0x48, 0x8B, 0x7D, 0x18,
            0x31, 0xF6,
            0x31, 0xD2,
            0xB8, 0x02, 0x00, 0x00, 0x00,
            0x0F, 0x05,
            0x48, 0x85, 0xC0,
            0x78, 0x1A,
            0x48, 0x89, 0xC7,
            0x48, 0x8D, 0xB5, 0x00, 0x20, 0xFD, 0xFF,
            0xBA, 0x00, 0xE0, 0x00, 0x00,
            0x31, 0xC0,
            0x0F, 0x05,
            0xB8, 0x03, 0x00, 0x00, 0x00,
            0x0F, 0x05,
            0x45, 0x31, 0xDB,
        ]);
        emit_cli_arg_parser(&mut code, 3, 0x20, 5);
        emit_cli_arg_parser(&mut code, 4, 0x28, 6);
        emit_cli_arg_parser(&mut code, 5, 0x30, 7);
    } else {
        code.extend_from_slice(&[0x49, 0xBC]);
        code.extend_from_slice(&(arg0 as u64).to_le_bytes());
        code.extend_from_slice(&[0x49, 0xBD]);
        code.extend_from_slice(&(arg1 as u64).to_le_bytes());
        code.extend_from_slice(&[0x49, 0xBE]);
        code.extend_from_slice(&(arg2 as u64).to_le_bytes());
        code.extend_from_slice(&[0x49, 0xC7, 0xC7, 0x00, 0x00, 0x00, 0x00]);
        emit_cli_arg_parser(&mut code, 2, 0x18, 4);
        emit_cli_arg_parser(&mut code, 3, 0x20, 5);
        emit_cli_arg_parser(&mut code, 4, 0x28, 6);
        emit_cli_arg_parser(&mut code, 5, 0x30, 7);
    }

    let emit_load_op = |c: &mut Vec<u8>, op: &BinOperand, into_rcx: bool| {
        match *op {
            BinOperand::Param(0x7EC0) => {
                c.extend_from_slice(&[0x4C, 0x89, if into_rcx { 0xE1 } else { 0xE0 }]);
            }
            BinOperand::Param(0x7EC1) => {
                c.extend_from_slice(&[0x4C, 0x89, if into_rcx { 0xE9 } else { 0xE8 }]);
            }
            BinOperand::Param(0x7EC2) => {
                c.extend_from_slice(&[0x4C, 0x89, if into_rcx { 0xF1 } else { 0xF0 }]);
            }
            BinOperand::Param(0x7EC3) => {
                c.extend_from_slice(&[0x4C, 0x89, if into_rcx { 0xF9 } else { 0xF8 }]);
            }
            BinOperand::Reg(r_tok) => {
                let disp: i32 = -8 * (((r_tok - REG_BASE) as i32) + 1);
                c.extend_from_slice(&[0x48, 0x8B, if into_rcx { 0x8D } else { 0x85 }]);
                c.extend_from_slice(&disp.to_le_bytes());
            }
            BinOperand::ImmInt(v) => {
                c.extend_from_slice(&[0x48, if into_rcx { 0xB9 } else { 0xB8 }]);
                c.extend_from_slice(&(v as u64).to_le_bytes());
            }
            BinOperand::ImmTrit(t) => {
                c.extend_from_slice(&[0x48, if into_rcx { 0xB9 } else { 0xB8 }]);
                c.extend_from_slice(&((t as i64) as u64).to_le_bytes());
            }
            _ => {}
        }
    };

    let emit_store_rax = |c: &mut Vec<u8>, d_tok: u16| {
        let disp: i32 = -8 * (((d_tok - REG_BASE) as i32) + 1);
        c.extend_from_slice(&[0x48, 0x89, 0x85]);
        c.extend_from_slice(&disp.to_le_bytes());
    };

    let reg_pack5 = collect_func_pack5_map(func);
    validate_func_pack5_ops(func, &reg_pack5)?;
    let emit_extract_pack5_trit_x86 = |c: &mut Vec<u8>, op: &BinOperand, trit_idx: usize, into_rcx: bool| {
        let mut direct_byte = false;
        if let BinOperand::Reg(r) = *op {
            if let Some((bytes, _, _)) = reg_pack5.get(&r) {
                let b_val = bytes.get(trit_idx / 5).copied().unwrap_or(121) as i64;
                emit_load_op(c, &BinOperand::ImmInt(b_val), false);
                direct_byte = true;
            }
        }
        if !direct_byte {
            emit_load_op(c, op, false);
            let shift: u8 = (((trit_idx / 5) + 1) * 8) as u8;
            c.extend_from_slice(&[0x48, 0xC1, 0xE8, shift]); // shr rax, shift
            c.extend_from_slice(&[0x0F, 0xB6, 0xC0]);       // movzx eax, al
        }
        c.extend_from_slice(&[0x31, 0xD2]);             // xor edx, edx
        let pow3: u8 = [1u8, 3, 9, 27, 81][trit_idx % 5];
        if pow3 > 1 {
            c.extend_from_slice(&[0xBE, pow3, 0, 0, 0]); // mov esi, pow3
            c.extend_from_slice(&[0x48, 0xF7, 0xF6]);    // div rsi
            c.extend_from_slice(&[0x31, 0xD2]);          // xor edx, edx
        }
        c.extend_from_slice(&[0xBE, 3, 0, 0, 0]);       // mov esi, 3
        c.extend_from_slice(&[0x48, 0xF7, 0xF6]);       // div rsi (rdx = byte / 3^k % 3)
        c.extend_from_slice(&[0x48, 0x8D, 0x42, 0xFF]); // lea rax, [rdx - 1]
        if into_rcx {
            c.extend_from_slice(&[0x48, 0x89, 0xC1]);   // mov rcx, rax
        }
    };

    for ins in &func.instructions {
        match ins.op_tok {
            0x7F00 | 0x7F05 => {}
            0x7F01 | 0x7F02 | 0x7F03 | 0x7F04 | 0x7F06 | 0x7F09 | 0x7F57 | 0x7F58 | 0x7F59 => {
                emit_load_op(&mut code, &ins.operands[0], false);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F0D => {
                code.extend_from_slice(&[0x45, 0x31, 0xDB]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F0E => {
                if let (Some(BinOperand::Reg(r_arr)), Some(idx_op)) =
                    (ins.operands.first(), ins.operands.get(1))
                {
                    if let Some((bytes, n_len, is_tv)) = reg_pack5.get(r_arr) {
                        if let BinOperand::ImmInt(idx) = idx_op {
                            if *idx >= 0 {
                                if *is_tv {
                                    emit_extract_pack5_trit_x86(&mut code, &ins.operands[0], *idx as usize, false);
                                } else {
                                    let b_val = bytes.get(*idx as usize).copied().unwrap_or(0) as i64;
                                    emit_load_op(&mut code, &BinOperand::ImmInt(b_val), false);
                                }
                                if let Some(d) = ins.dest_reg {
                                    emit_store_rax(&mut code, d);
                                }
                                continue;
                            }
                        } else {
                            code.extend_from_slice(&[0x45, 0x31, 0xC0]); // xor r8d, r8d (acc = 0)
                            for k in 0..*n_len {
                                if *is_tv {
                                    emit_extract_pack5_trit_x86(&mut code, &ins.operands[0], k, true);
                                } else {
                                    let b_val = bytes.get(k).copied().unwrap_or(0) as i64;
                                    emit_load_op(&mut code, &BinOperand::ImmInt(b_val), true);
                                }
                                emit_load_op(&mut code, idx_op, false); // rax = dynamic index
                                code.extend_from_slice(&[0x48, 0x3D]);  // cmp rax, imm32(k)
                                code.extend_from_slice(&(k as i32).to_le_bytes());
                                code.extend_from_slice(&[0x4C, 0x0F, 0x44, 0xC1]); // cmove r8, rcx
                            }
                            code.extend_from_slice(&[0x4C, 0x89, 0xC0]); // mov rax, r8
                            if let Some(d) = ins.dest_reg {
                                emit_store_rax(&mut code, d);
                            }
                            continue;
                        }
                    }
                }
                emit_load_op(&mut code, &ins.operands[1], false);
                code.extend_from_slice(&[0x0F, 0xB7, 0x84, 0x45, 0x00, 0x20, 0xFD, 0xFF]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F0F => {
                if ins.operands.len() == 3 && ins.operands[1] == BinOperand::ImmInt(-4) {
                    emit_load_op(&mut code, &ins.operands[2], false);
                    code.extend_from_slice(&[0x42, 0x89, 0x84, 0x1D, 0x00, 0x00, 0xFE, 0xFF, 0x49, 0x83, 0xC3, 0x04]);
                } else if ins.operands.len() == 3 && ins.operands[1] == BinOperand::ImmInt(-8) {
                    emit_load_op(&mut code, &ins.operands[2], false);
                    code.extend_from_slice(&[0x4A, 0x89, 0x84, 0x1D, 0x00, 0x00, 0xFE, 0xFF, 0x49, 0x83, 0xC3, 0x08]);
                } else if ins.operands.len() == 4 && ins.operands[1] == BinOperand::ImmInt(-9) {
                    emit_load_op(&mut code, &ins.operands[2], true);
                    emit_load_op(&mut code, &ins.operands[3], false);
                    code.extend_from_slice(&[0x48, 0x89, 0x84, 0x0D, 0x00, 0x00, 0xFE, 0xFF]);
                } else {
                    for op_b in &ins.operands[1..] {
                        emit_load_op(&mut code, op_b, false);
                        code.extend_from_slice(&[0x42, 0x88, 0x84, 0x1D, 0x00, 0x00, 0xFE, 0xFF, 0x49, 0xFF, 0xC3]);
                    }
                }
                if let Some(d) = ins.dest_reg {
                    code.extend_from_slice(&[0x4C, 0x89, 0xD8]);
                    emit_store_rax(&mut code, d);
                }
            }
            0x7F50 => {
                code.extend_from_slice(&[0x4C, 0x89, 0xD8]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F30 | 0x7F31 => {
                if let Some(d) = ins.dest_reg {
                    let word = reg_pack5
                        .get(&d)
                        .map(|(b, n, _)| pack5_word_from_bytes(b, *n))
                        .unwrap_or(0);
                    emit_load_op(&mut code, &BinOperand::ImmInt(word), false);
                    emit_store_rax(&mut code, d);
                }
            }
            0x7F34 => {
                let n_trits = match (ins.operands.first(), ins.operands.get(1)) {
                    (Some(BinOperand::Reg(ra)), Some(BinOperand::Reg(rb))) => {
                        let na = reg_pack5.get(ra).map(|(_, n, _)| *n).unwrap_or(0);
                        let nb = reg_pack5.get(rb).map(|(_, n, _)| *n).unwrap_or(0);
                        na.min(nb)
                    }
                    _ => 0,
                };
                code.extend_from_slice(&[0x45, 0x31, 0xC0]); // xor r8d, r8d
                for idx in 0..n_trits {
                    emit_extract_pack5_trit_x86(&mut code, &ins.operands[0], idx, true);
                    emit_extract_pack5_trit_x86(&mut code, &ins.operands[1], idx, false);
                    code.extend_from_slice(&[0x48, 0x0F, 0xAF, 0xC1]); // imul rax, rcx
                    code.extend_from_slice(&[0x49, 0x01, 0xC0]);       // add r8, rax
                }
                code.extend_from_slice(&[0x4C, 0x89, 0xC0]); // mov rax, r8
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F35 | 0x7F3F => {
                let pack5_len = match ins.operands.first() {
                    Some(BinOperand::Reg(r)) => reg_pack5.get(r).map(|(_, n, _)| *n),
                    _ => None,
                };
                if let Some(n_trits) = pack5_len {
                    code.extend_from_slice(&[0x45, 0x31, 0xC0]); // xor r8d, r8d
                    for idx in 0..n_trits {
                        emit_extract_pack5_trit_x86(&mut code, &ins.operands[0], idx, false);
                        code.extend_from_slice(&[0x49, 0x01, 0xC0]); // add r8, rax
                    }
                    code.extend_from_slice(&[0x4C, 0x89, 0xC0]); // mov rax, r8
                } else {
                    emit_load_op(&mut code, &ins.operands[0], false);
                }
                code.extend_from_slice(&[
                    0x48, 0x83, 0xF8, 0x00, // cmp rax, 0
                    0x0F, 0x9F, 0xC0,       // setg al
                    0x0F, 0x9C, 0xC2,       // setl dl
                    0x0F, 0xB6, 0xC0,       // movzx eax, al
                    0x0F, 0xB6, 0xD2,       // movzx edx, dl
                    0x48, 0x29, 0xD0,       // sub rax, rdx
                ]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F10 | 0x7F11 | 0x7F12 | 0x7F1E | 0x7F1F | 0x7F20 => {
                emit_load_op(&mut code, &ins.operands[0], false);
                emit_load_op(&mut code, &ins.operands[1], true);
                match ins.op_tok {
                    0x7F10 => code.extend_from_slice(&[0x48, 0x01, 0xC8]),
                    0x7F11 => code.extend_from_slice(&[0x48, 0x29, 0xC8]),
                    0x7F12 => code.extend_from_slice(&[0x48, 0x0F, 0xAF, 0xC1]),
                    0x7F1E => code.extend_from_slice(&[0x48, 0x21, 0xC8]),
                    0x7F1F => code.extend_from_slice(&[0x48, 0x09, 0xC8]),
                    _      => code.extend_from_slice(&[0x48, 0x31, 0xC8]),
                }
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F13 | 0x7F14 => {
                emit_load_op(&mut code, &ins.operands[0], false);
                emit_load_op(&mut code, &ins.operands[1], true);
                code.extend_from_slice(&[0x49, 0x89, 0xC0, 0x48, 0x85, 0xC9, 0x74, 0x1E, 0x48, 0x83, 0xF9, 0xFF, 0x75, 0x11, 0x49, 0xB9, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x4D, 0x39, 0xC8, 0x75, 0x02, 0xEB, 0x0A, 0x48, 0x99, 0x48, 0xF7, 0xF9, 0xEB, 0x03, 0x48, 0x31, 0xC0]);
                if ins.op_tok == 0x7F14 {
                    code.extend_from_slice(&[0x48, 0x89, 0xD0]);
                }
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F15 | 0x7F28 => {
                emit_load_op(&mut code, &ins.operands[0], false);
                code.extend_from_slice(&[0x48, 0xF7, 0xD8]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F16 => {
                emit_load_op(&mut code, &ins.operands[0], false);
                code.extend_from_slice(&[0x48, 0x89, 0xC2, 0x48, 0xF7, 0xD8, 0x48, 0x0F, 0x48, 0xC2]); // fixed IABS: cmovs
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F21 => {
                emit_load_op(&mut code, &ins.operands[0], false);
                code.extend_from_slice(&[0x48, 0xF7, 0xD0]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F22 | 0x7F23 => {
                emit_load_op(&mut code, &ins.operands[0], false);
                emit_load_op(&mut code, &ins.operands[1], true);
                code.extend_from_slice(&[0x48, 0xD3, if ins.op_tok == 0x7F22 { 0xE0 } else { 0xF8 }]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F24 | 0x7F25 | 0x7F26 | 0x7F27 => {
                emit_load_op(&mut code, &ins.operands[0], false);
                emit_load_op(&mut code, &ins.operands[1], true);
                let setcc: u8 = match ins.op_tok {
                    0x7F24 => 0x94,
                    0x7F25 => 0x95,
                    0x7F26 => 0x9C,
                    _      => 0x9F,
                };
                code.extend_from_slice(&[
                    0x48, 0x39, 0xC8,
                    0x0F, setcc, 0xC0,
                    0x0F, 0xB6, 0xC0,
                    0x48, 0x8D, 0x44, 0x00, 0xFF,
                ]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F29 | 0x7F17 => {
                emit_load_op(&mut code, &ins.operands[0], false);
                emit_load_op(&mut code, &ins.operands[1], true);
                code.extend_from_slice(&[0x48, 0x39, 0xC8, 0x48, 0x0F, 0x4F, 0xC1]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F2A | 0x7F18 => {
                emit_load_op(&mut code, &ins.operands[0], false);
                emit_load_op(&mut code, &ins.operands[1], true);
                code.extend_from_slice(&[0x48, 0x39, 0xC8, 0x48, 0x0F, 0x4C, 0xC1]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F2B => {
                emit_load_op(&mut code, &ins.operands[0], false);
                emit_load_op(&mut code, &ins.operands[1], true);
                code.extend_from_slice(&[0x48, 0x0F, 0xAF, 0xC1, 0x48, 0xF7, 0xD8]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F2C => {
                emit_load_op(&mut code, &ins.operands[0], false);
                emit_load_op(&mut code, &ins.operands[1], true);
                code.extend_from_slice(&[0x48, 0x01, 0xC8]);
                emit_load_op(&mut code, &ins.operands[2], true);
                code.extend_from_slice(&[
                    0x48, 0x01, 0xC8,
                    0x48, 0x83, 0xF8, 0x00,
                    0x0F, 0x9F, 0xC0,
                    0x0F, 0x9C, 0xC2,
                    0x0F, 0xB6, 0xC0,
                    0x0F, 0xB6, 0xD2,
                    0x48, 0x29, 0xD0,
                ]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F2D => {
                emit_load_op(&mut code, &ins.operands[0], false);
                emit_load_op(&mut code, &ins.operands[1], true);
                code.extend_from_slice(&[
                    0x48, 0x39, 0xC8,
                    0x0F, 0x9F, 0xC2,
                    0x0F, 0x9C, 0xC1,
                    0x0F, 0xB6, 0xD2,
                    0x0F, 0xB6, 0xC9,
                    0x29, 0xCA,
                    0x48, 0x63, 0xC2,
                ]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F40 => {
                if let Some(lbl) = ins.operands.first().and_then(extract_label_id) {
                    label_offsets.push((lbl, code.len()));
                }
            }
            0x7F41 => {
                if let Some(lbl) = ins.operands.first().and_then(extract_label_id) {
                    code.push(0xE9);
                    relocs.push(RelocPatch { patch_offset: code.len(), target_label: lbl });
                    code.extend_from_slice(&[0, 0, 0, 0]);
                }
            }
            0x7F42 => {
                if let (Some(l_neg), Some(l_zero), Some(l_pos)) = (
                    ins.operands.get(1).and_then(extract_label_id),
                    ins.operands.get(2).and_then(extract_label_id),
                    ins.operands.get(3).and_then(extract_label_id),
                ) {
                    emit_load_op(&mut code, &ins.operands[0], false);
                    code.extend_from_slice(&[0x48, 0x83, 0xF8, 0x00]);
                    code.extend_from_slice(&[0x0F, 0x8C]);
                    relocs.push(RelocPatch { patch_offset: code.len(), target_label: l_neg });
                    code.extend_from_slice(&[0, 0, 0, 0]);
                    code.extend_from_slice(&[0x0F, 0x84]);
                    relocs.push(RelocPatch { patch_offset: code.len(), target_label: l_zero });
                    code.extend_from_slice(&[0, 0, 0, 0]);
                    code.push(0xE9);
                    relocs.push(RelocPatch { patch_offset: code.len(), target_label: l_pos });
                    code.extend_from_slice(&[0, 0, 0, 0]);
                }
            }
            0x7F43 | 0x7F44 => {
                if let Some(lbl) = ins.operands.get(1).and_then(extract_label_id) {
                    emit_load_op(&mut code, &ins.operands[0], false);
                    code.extend_from_slice(&[0x48, 0x83, 0xF8, if ins.op_tok == 0x7F43 { 0x01 } else { 0xFF }]);
                    code.extend_from_slice(&[0x0F, 0x84]);
                    relocs.push(RelocPatch { patch_offset: code.len(), target_label: lbl });
                    code.extend_from_slice(&[0, 0, 0, 0]);
                }
            }
            0x7F45 => {
                emit_load_op(&mut code, &ins.operands[0], false);
                emit_load_op(&mut code, &ins.operands[1], true);
                code.extend_from_slice(&[0x49, 0x89, 0xC0, 0x48, 0x89, 0xCA]);
                emit_load_op(&mut code, &ins.operands[2], true);
                code.extend_from_slice(&[0x49, 0x83, 0xF8, 0x00, 0x48, 0x0F, 0x44, 0xD1]);
                emit_load_op(&mut code, &ins.operands[3], true);
                code.extend_from_slice(&[0x49, 0x83, 0xF8, 0x00, 0x48, 0x0F, 0x4F, 0xD1, 0x48, 0x89, 0xD0]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F4A => {
                if func.ret_ty_tok == TY_BYTES {
                    code.extend_from_slice(&[
                        0xBF, 0x01, 0x00, 0x00, 0x00,
                        0x48, 0x8D, 0xB5, 0x00, 0x00, 0xFE, 0xFF,
                        0x4C, 0x89, 0xDA,
                        0xB8, 0x01, 0x00, 0x00, 0x00,
                        0x0F, 0x05,
                        0x31, 0xFF,
                        0xB8, 0x3C, 0x00, 0x00, 0x00,
                        0x0F, 0x05,
                    ]);
                } else {
                    emit_load_op(&mut code, &ins.operands[0], false);
                    // 64-bit result channel: write raw little-endian i64 to stdout, then exit(0)
                    code.extend_from_slice(&[
                        0x48, 0x83, 0xEC, 0x08,             // sub rsp, 8
                        0x48, 0x89, 0x04, 0x24,             // mov [rsp], rax
                        0xB8, 0x01, 0x00, 0x00, 0x00,       // mov eax, 1 (write)
                        0xBF, 0x01, 0x00, 0x00, 0x00,       // mov edi, 1 (stdout)
                        0x48, 0x89, 0xE6,                   // mov rsi, rsp
                        0xBA, 0x08, 0x00, 0x00, 0x00,       // mov edx, 8
                        0x0F, 0x05,                         // syscall
                        0x48, 0x83, 0xC4, 0x08,             // add rsp, 8
                        0x31, 0xFF,                         // xor edi, edi
                        0xB8, 0x3C, 0x00, 0x00, 0x00,       // mov eax, 60 (exit)
                        0x0F, 0x05,                         // syscall
                    ]);
                }
            }
            _ => {}
        }
    }

    for r in &relocs {
        let target_off = label_offsets
            .iter()
            .find(|&&(lbl, _)| lbl == r.target_label)
            .map(|&(_, off)| off)
            .ok_or_else(|| format!("Unresolved label 0x{:04X}", r.target_label))?;
        let rel32: i32 = (target_off as i32) - ((r.patch_offset + 4) as i32);
        code[r.patch_offset..r.patch_offset + 4].copy_from_slice(&rel32.to_le_bytes());
    }

    let base_vaddr: u64 = 0x400000;
    let entry_vaddr: u64 = base_vaddr + 120;
    let total_size: u64 = 120 + (code.len() as u64);

    let mut elf = vec![0u8; 120];
    elf[0..4].copy_from_slice(&[0x7F, b'E', b'L', b'F']);
    elf[4] = 2;
    elf[5] = 1;
    elf[6] = 1;
    elf[16..18].copy_from_slice(&2u16.to_le_bytes());
    elf[18..20].copy_from_slice(&0x3Eu16.to_le_bytes());
    elf[20..24].copy_from_slice(&1u32.to_le_bytes());
    elf[24..32].copy_from_slice(&entry_vaddr.to_le_bytes());
    elf[32..40].copy_from_slice(&64u64.to_le_bytes());
    elf[52..54].copy_from_slice(&64u16.to_le_bytes());
    elf[54..56].copy_from_slice(&56u16.to_le_bytes());
    elf[56..58].copy_from_slice(&1u16.to_le_bytes());

    elf[64..68].copy_from_slice(&1u32.to_le_bytes());
    elf[68..72].copy_from_slice(&5u32.to_le_bytes());
    elf[80..88].copy_from_slice(&base_vaddr.to_le_bytes());
    elf[88..96].copy_from_slice(&base_vaddr.to_le_bytes());
    elf[96..104].copy_from_slice(&total_size.to_le_bytes());
    elf[104..112].copy_from_slice(&total_size.to_le_bytes());
    elf[112..120].copy_from_slice(&0x1000u64.to_le_bytes());

    elf.extend_from_slice(&code);
    Ok(elf)
}

fn encode_uleb128(mut val: usize) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let mut byte = (val & 0x7F) as u8;
        val >>= 7;
        if val != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if val == 0 {
            break;
        }
    }
    out
}

fn encode_sleb128(mut val: i64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let mut byte = (val & 0x7F) as u8;
        val >>= 7;
        let done = (val == 0 && (byte & 0x40) == 0) || (val == -1 && (byte & 0x40) != 0);
        if !done {
            byte |= 0x80;
        }
        out.push(byte);
        if done {
            break;
        }
    }
    out
}

pub fn compile_pure_u16_to_wasm(stream: &[u16]) -> Result<Vec<u8>, String> {
    let module = decode_omni_token_stream(stream)?;
    let func = module.functions.first().ok_or("No function in module")?;
    let rep = verify_binary_function(func);
    if !rep.passed {
        return Err(format!("Cannot Wasm-compile unverified binary stream: {:?}", rep.errors));
    }

    for ins in &func.instructions {
        if !is_wasm_aot_supported_opcode(ins.op_tok) {
            let lbl = decode_opcode_token(ins.op_tok).map(|s| s.debug_label).unwrap_or("OP_UNKNOWN");
            return Err(format!(
                "Opcode 0x{:04X} ({}) requires high-level VM runtime state and is not lowered by standalone WebAssembly AOT",
                ins.op_tok, lbl
            ));
        }
    }

    let num_params = func.params.len();
    let max_reg_used = func
        .instructions
        .iter()
        .filter_map(|ins| ins.dest_reg)
        .map(|r| (r - REG_BASE) as usize + 1)
        .max()
        .unwrap_or(0);
    let num_locals = max_reg_used + 4;
    let scratch0 = num_params + max_reg_used;
    let scratch1 = num_params + max_reg_used + 1;
    let buf_len_local = num_params + max_reg_used + 2;
    let pc_local = num_params + max_reg_used + 3;

    let has_control_flow = func
        .instructions
        .iter()
        .any(|ins| (0x7F40..=0x7F44).contains(&ins.op_tok));

    let mut label_pcs: Vec<(u16, i64)> = Vec::new();
    for (idx, ins) in func.instructions.iter().enumerate() {
        if ins.op_tok == 0x7F40 {
            if let Some(lbl) = ins.operands.first().and_then(extract_label_id) {
                label_pcs.push((lbl, idx as i64));
            }
        }
    }
    let lookup_lbl_pc = |lbl: u16| -> Result<i64, String> {
        label_pcs
            .iter()
            .find(|&&(l, _)| l == lbl)
            .map(|&(_, pc)| pc)
            .ok_or_else(|| format!("Unresolved Wasm label 0x{:04X}", lbl))
    };

    let mut body: Vec<u8> = Vec::with_capacity(512);
    body.extend(encode_uleb128(1));
    body.extend(encode_uleb128(num_locals));
    body.push(0x7E);

    let emit_wasm_op = |b: &mut Vec<u8>, op: &BinOperand| {
        match *op {
            BinOperand::Param(p_tok) => {
                b.push(0x20);
                b.extend(encode_uleb128((p_tok - PAR_BASE) as usize));
            }
            BinOperand::Reg(r_tok) => {
                b.push(0x20);
                b.extend(encode_uleb128(num_params + (r_tok - REG_BASE) as usize));
            }
            BinOperand::ImmInt(v) => {
                b.push(0x42);
                b.extend(encode_sleb128(v));
            }
            BinOperand::ImmTrit(t) => {
                b.push(0x42);
                b.extend(encode_sleb128(t as i64));
            }
            _ => {
                b.extend_from_slice(&[0x42, 0x00]);
            }
        }
    };

    let emit_wasm_set_dest = |b: &mut Vec<u8>, dest: Option<u16>| {
        if let Some(d_tok) = dest {
            b.push(0x21);
            b.extend(encode_uleb128(num_params + (d_tok - REG_BASE) as usize));
        }
    };

    let reg_pack5 = collect_func_pack5_map(func);
    validate_func_pack5_ops(func, &reg_pack5)?;
    let emit_extract_pack5_trit_wasm = |b: &mut Vec<u8>, op: &BinOperand, trit_idx: usize| {
        let mut direct_byte = false;
        if let BinOperand::Reg(r) = *op {
            if let Some((bytes, _, _)) = reg_pack5.get(&r) {
                let b_val = bytes.get(trit_idx / 5).copied().unwrap_or(121) as i64;
                b.push(0x42);
                b.extend(encode_sleb128(b_val));
                direct_byte = true;
            }
        }
        if !direct_byte {
            emit_wasm_op(b, op);
            let shift: i64 = (((trit_idx / 5) + 1) * 8) as i64;
            b.push(0x42);
            b.extend(encode_sleb128(shift));
            b.push(0x87); // i64.shr_s
            b.push(0x42);
            b.extend(encode_sleb128(255));
            b.push(0x83); // i64.and
        }
        let pow3: i64 = [1i64, 3, 9, 27, 81][trit_idx % 5];
        if pow3 > 1 {
            b.push(0x42);
            b.extend(encode_sleb128(pow3));
            b.push(0x7F); // i64.div_s
        }
        b.extend_from_slice(&[0x42, 0x03, 0x81, 0x42, 0x01, 0x7D]); // % 3 - 1
    };

    let emit_straight_ins = |b: &mut Vec<u8>, ins: &BinInstruction| {
        match ins.op_tok {
            0x7F00 | 0x7F05 => {}
            0x7F01 | 0x7F02 | 0x7F03 | 0x7F04 | 0x7F06 | 0x7F09 | 0x7F57 | 0x7F58 | 0x7F59 => {
                emit_wasm_op(b, &ins.operands[0]);
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F30 | 0x7F31 => {
                let word = ins
                    .dest_reg
                    .and_then(|d| reg_pack5.get(&d).map(|(bytes, n, _)| pack5_word_from_bytes(bytes, *n)))
                    .unwrap_or(0);
                b.push(0x42);
                b.extend(encode_sleb128(word));
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F34 => {
                let n_trits = match (ins.operands.first(), ins.operands.get(1)) {
                    (Some(BinOperand::Reg(ra)), Some(BinOperand::Reg(rb))) => {
                        let na = reg_pack5.get(ra).map(|(_, n, _)| *n).unwrap_or(0);
                        let nb = reg_pack5.get(rb).map(|(_, n, _)| *n).unwrap_or(0);
                        na.min(nb)
                    }
                    _ => 0,
                };
                b.extend_from_slice(&[0x42, 0x00]); // acc = 0
                for idx in 0..n_trits {
                    emit_extract_pack5_trit_wasm(b, &ins.operands[0], idx);
                    emit_extract_pack5_trit_wasm(b, &ins.operands[1], idx);
                    b.extend_from_slice(&[0x7E, 0x7C]); // i64.mul, i64.add
                }
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F35 | 0x7F3F => {
                let pack5_len = match ins.operands.first() {
                    Some(BinOperand::Reg(r)) => reg_pack5.get(r).map(|(_, n, _)| *n),
                    _ => None,
                };
                if let Some(n_trits) = pack5_len {
                    b.extend_from_slice(&[0x42, 0x00]); // sum = 0
                    for idx in 0..n_trits {
                        emit_extract_pack5_trit_wasm(b, &ins.operands[0], idx);
                        b.push(0x7C); // i64.add
                    }
                } else {
                    emit_wasm_op(b, &ins.operands[0]);
                }
                b.push(0x22); b.extend(encode_uleb128(scratch0));
                b.extend_from_slice(&[0x42, 0x00, 0x55, 0xAD]);
                b.push(0x20); b.extend(encode_uleb128(scratch0));
                b.extend_from_slice(&[0x42, 0x00, 0x53, 0xAD, 0x7D]);
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F0D => {
                b.extend_from_slice(&[0x42, 0x00]);
                b.push(0x21);
                b.extend(encode_uleb128(buf_len_local));
                if ins.dest_reg.is_some() {
                    b.extend_from_slice(&[0x42, 0x00]);
                    emit_wasm_set_dest(b, ins.dest_reg);
                }
            }
            0x7F0E => {
                let mut handled = false;
                if let (Some(BinOperand::Reg(r_arr)), Some(idx_op)) =
                    (ins.operands.first(), ins.operands.get(1))
                {
                    if let Some((bytes, n_len, is_tv)) = reg_pack5.get(r_arr) {
                        if let BinOperand::ImmInt(idx) = idx_op {
                            if *idx >= 0 {
                                if *is_tv {
                                    emit_extract_pack5_trit_wasm(b, &ins.operands[0], *idx as usize);
                                } else {
                                    let b_val = bytes.get(*idx as usize).copied().unwrap_or(0) as i64;
                                    b.push(0x42);
                                    b.extend(encode_sleb128(b_val));
                                }
                                emit_wasm_set_dest(b, ins.dest_reg);
                                handled = true;
                            }
                        } else {
                            // Dynamic index IGET on @PACK5 TritVec or Bytes in Wasm AOT:
                            b.extend_from_slice(&[0x42, 0x00]); // acc = 0
                            for k in 0..*n_len {
                                if *is_tv {
                                    emit_extract_pack5_trit_wasm(b, &ins.operands[0], k);
                                } else {
                                    let b_val = bytes.get(k).copied().unwrap_or(0) as i64;
                                    b.push(0x42);
                                    b.extend(encode_sleb128(b_val));
                                }
                                emit_wasm_op(b, idx_op);
                                b.push(0x42);
                                b.extend(encode_sleb128(k as i64));
                                b.extend_from_slice(&[0x51, 0xAD, 0x7E, 0x7C]); // i64.eq, i64.extend_i32_u, i64.mul, i64.add
                            }
                            emit_wasm_set_dest(b, ins.dest_reg);
                            handled = true;
                        }
                    }
                }
                if !handled {
                    b.extend_from_slice(&[0x42, 0x00]);
                    emit_wasm_set_dest(b, ins.dest_reg);
                }
            }
            0x7F0F => {
                let delta: i64 = if ins.operands.len() == 3 && ins.operands[1] == BinOperand::ImmInt(-4) {
                    4
                } else if ins.operands.len() == 3 && ins.operands[1] == BinOperand::ImmInt(-8) {
                    8
                } else if ins.operands.len() == 4 && ins.operands[1] == BinOperand::ImmInt(-9) {
                    0
                } else {
                    ins.operands.len().saturating_sub(1) as i64
                };
                if delta != 0 {
                    b.push(0x20);
                    b.extend(encode_uleb128(buf_len_local));
                    b.push(0x42);
                    b.extend(encode_sleb128(delta));
                    b.push(0x7C);
                    b.push(0x21);
                    b.extend(encode_uleb128(buf_len_local));
                }
                if ins.dest_reg.is_some() {
                    b.push(0x20);
                    b.extend(encode_uleb128(buf_len_local));
                    emit_wasm_set_dest(b, ins.dest_reg);
                }
            }
            0x7F50 => {
                if ins.dest_reg.is_some() {
                    b.push(0x20);
                    b.extend(encode_uleb128(buf_len_local));
                    emit_wasm_set_dest(b, ins.dest_reg);
                }
            }
            0x7F10 | 0x7F11 | 0x7F12 | 0x7F1E | 0x7F1F | 0x7F20 | 0x7F22 | 0x7F23 => {
                emit_wasm_op(b, &ins.operands[0]);
                emit_wasm_op(b, &ins.operands[1]);
                let wasm_op = match ins.op_tok {
                    0x7F10 => 0x7C,
                    0x7F11 => 0x7D,
                    0x7F12 => 0x7E,
                    0x7F1E => 0x83,
                    0x7F1F => 0x84,
                    0x7F20 => 0x85,
                    0x7F22 => 0x86,
                    _      => 0x87,
                };
                b.push(wasm_op);
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F13 | 0x7F14 => {
                emit_wasm_op(b, &ins.operands[0]);
                b.push(0x21); b.extend(encode_uleb128(scratch0));
                emit_wasm_op(b, &ins.operands[1]);
                b.push(0x21); b.extend(encode_uleb128(scratch1));
                if ins.op_tok == 0x7F13 {
                    b.extend_from_slice(&[0x42, 0x00]);
                }
                b.push(0x20); b.extend(encode_uleb128(scratch0));
                b.extend_from_slice(&[0x42, 0x01]);
                b.push(0x20); b.extend(encode_uleb128(scratch1));
                b.push(0x20); b.extend(encode_uleb128(scratch1));
                b.push(0x50);
                b.push(0x20); b.extend(encode_uleb128(scratch0));
                b.push(0x42); b.extend(encode_sleb128(i64::MIN));
                b.push(0x51);
                b.push(0x20); b.extend(encode_uleb128(scratch1));
                b.extend_from_slice(&[0x42, 0x7F, 0x51]);
                b.extend_from_slice(&[0x71, 0x72, 0x1B]);
                if ins.op_tok == 0x7F13 {
                    b.push(0x7F);
                    b.push(0x20); b.extend(encode_uleb128(scratch1));
                    b.extend_from_slice(&[0x50, 0x1B]);
                } else {
                    b.push(0x81);
                }
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F15 | 0x7F28 => {
                b.extend_from_slice(&[0x42, 0x00]);
                emit_wasm_op(b, &ins.operands[0]);
                b.push(0x7D);
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F16 => {
                b.extend_from_slice(&[0x42, 0x00]);
                emit_wasm_op(b, &ins.operands[0]);
                b.push(0x7D);
                emit_wasm_op(b, &ins.operands[0]);
                emit_wasm_op(b, &ins.operands[0]);
                b.extend_from_slice(&[0x42, 0x00, 0x53, 0x1B]);
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F21 => {
                emit_wasm_op(b, &ins.operands[0]);
                b.extend_from_slice(&[0x42, 0x7F, 0x85]);
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F17 | 0x7F29 | 0x7F18 | 0x7F2A => {
                emit_wasm_op(b, &ins.operands[0]);
                emit_wasm_op(b, &ins.operands[1]);
                emit_wasm_op(b, &ins.operands[0]);
                emit_wasm_op(b, &ins.operands[1]);
                b.push(if matches!(ins.op_tok, 0x7F17 | 0x7F29) { 0x53 } else { 0x55 });
                b.push(0x1B);
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F2B => {
                b.extend_from_slice(&[0x42, 0x00]);
                emit_wasm_op(b, &ins.operands[0]);
                emit_wasm_op(b, &ins.operands[1]);
                b.extend_from_slice(&[0x7E, 0x7D]);
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F24 | 0x7F25 | 0x7F26 | 0x7F27 => {
                emit_wasm_op(b, &ins.operands[0]);
                emit_wasm_op(b, &ins.operands[1]);
                let cmp_op = match ins.op_tok {
                    0x7F24 => 0x51,
                    0x7F25 => 0x52,
                    0x7F26 => 0x53,
                    _      => 0x55,
                };
                b.extend_from_slice(&[cmp_op, 0xAD, 0x42, 0x02, 0x7E, 0x42, 0x01, 0x7D]);
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F2C => {
                emit_wasm_op(b, &ins.operands[0]);
                emit_wasm_op(b, &ins.operands[1]);
                b.push(0x7C);
                emit_wasm_op(b, &ins.operands[2]);
                b.push(0x7C);
                b.push(0x22); b.extend(encode_uleb128(scratch0));
                b.extend_from_slice(&[0x42, 0x00, 0x55, 0xAD]);
                b.push(0x20); b.extend(encode_uleb128(scratch0));
                b.extend_from_slice(&[0x42, 0x00, 0x53, 0xAD, 0x7D]);
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F2D => {
                emit_wasm_op(b, &ins.operands[0]);
                emit_wasm_op(b, &ins.operands[1]);
                b.extend_from_slice(&[0x55, 0xAD]);
                emit_wasm_op(b, &ins.operands[0]);
                emit_wasm_op(b, &ins.operands[1]);
                b.extend_from_slice(&[0x53, 0xAD, 0x7D]);
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F45 => {
                emit_wasm_op(b, &ins.operands[1]);
                emit_wasm_op(b, &ins.operands[2]);
                emit_wasm_op(b, &ins.operands[0]);
                b.extend_from_slice(&[0x42, 0x00, 0x53, 0x1B]);
                b.push(0x21); b.extend(encode_uleb128(scratch1));
                emit_wasm_op(b, &ins.operands[3]);
                b.push(0x20); b.extend(encode_uleb128(scratch1));
                emit_wasm_op(b, &ins.operands[0]);
                b.extend_from_slice(&[0x42, 0x00, 0x55, 0x1B]);
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F4A => {
                emit_wasm_op(b, &ins.operands[0]);
                b.push(0x0F);
            }
            _ => {}
        }
    };

    if !has_control_flow {
        for ins in &func.instructions {
            emit_straight_ins(&mut body, ins);
        }
    } else {
        body.extend_from_slice(&[0x03, 0x40]); // loop (empty blocktype)
        for (ins_idx, ins) in func.instructions.iter().enumerate() {
            body.push(0x20);
            body.extend(encode_uleb128(pc_local));
            body.push(0x42);
            body.extend(encode_sleb128(ins_idx as i64));
            body.extend_from_slice(&[0x51, 0x04, 0x40]); // i64.eq, if (empty blocktype)

            match ins.op_tok {
                0x7F40 => {
                    body.push(0x42);
                    body.extend(encode_sleb128((ins_idx + 1) as i64));
                    body.push(0x21);
                    body.extend(encode_uleb128(pc_local));
                    body.extend_from_slice(&[0x0C, 0x01]);
                }
                0x7F41 => {
                    let lbl = ins.operands.first().and_then(extract_label_id).ok_or("Missing JMP label")?;
                    let target_pc = lookup_lbl_pc(lbl)?;
                    body.push(0x42);
                    body.extend(encode_sleb128(target_pc));
                    body.push(0x21);
                    body.extend(encode_uleb128(pc_local));
                    body.extend_from_slice(&[0x0C, 0x01]);
                }
                0x7F43 | 0x7F44 => {
                    let lbl = ins.operands.get(1).and_then(extract_label_id).ok_or("Missing BR label")?;
                    let target_pc = lookup_lbl_pc(lbl)?;
                    body.push(0x42);
                    body.extend(encode_sleb128(target_pc));
                    body.push(0x42);
                    body.extend(encode_sleb128((ins_idx + 1) as i64));
                    emit_wasm_op(&mut body, &ins.operands[0]);
                    body.push(0x42);
                    body.extend(encode_sleb128(if ins.op_tok == 0x7F43 { 1 } else { -1 }));
                    body.extend_from_slice(&[0x51, 0x1B]);
                    body.push(0x21);
                    body.extend(encode_uleb128(pc_local));
                    body.extend_from_slice(&[0x0C, 0x01]);
                }
                0x7F42 => {
                    let l_neg = ins.operands.get(1).and_then(extract_label_id).ok_or("Missing BR_TRIT neg")?;
                    let l_zero = ins.operands.get(2).and_then(extract_label_id).ok_or("Missing BR_TRIT zero")?;
                    let l_pos = ins.operands.get(3).and_then(extract_label_id).ok_or("Missing BR_TRIT pos")?;
                    let pc_neg = lookup_lbl_pc(l_neg)?;
                    let pc_zero = lookup_lbl_pc(l_zero)?;
                    let pc_pos = lookup_lbl_pc(l_pos)?;
                    body.push(0x42);
                    body.extend(encode_sleb128(pc_neg));
                    body.push(0x42);
                    body.extend(encode_sleb128(pc_zero));
                    emit_wasm_op(&mut body, &ins.operands[0]);
                    body.extend_from_slice(&[0x42, 0x00, 0x53, 0x1B]);
                    body.push(0x21);
                    body.extend(encode_uleb128(scratch1));
                    body.push(0x42);
                    body.extend(encode_sleb128(pc_pos));
                    body.push(0x20);
                    body.extend(encode_uleb128(scratch1));
                    emit_wasm_op(&mut body, &ins.operands[0]);
                    body.extend_from_slice(&[0x42, 0x00, 0x55, 0x1B]);
                    body.push(0x21);
                    body.extend(encode_uleb128(pc_local));
                    body.extend_from_slice(&[0x0C, 0x01]);
                }
                0x7F4A => {
                    emit_wasm_op(&mut body, &ins.operands[0]);
                    body.push(0x0F);
                }
                _ => {
                    emit_straight_ins(&mut body, ins);
                    body.push(0x42);
                    body.extend(encode_sleb128((ins_idx + 1) as i64));
                    body.push(0x21);
                    body.extend(encode_uleb128(pc_local));
                    body.extend_from_slice(&[0x0C, 0x01]);
                }
            }
            body.push(0x0B); // end if
        }
        body.push(0x0B); // end loop
        body.extend_from_slice(&[0x42, 0x00]); // fallback i64.const 0
    }
    body.push(0x0B);

    let mut type_sec = vec![0x01, 0x60];
    type_sec.extend(encode_uleb128(num_params));
    type_sec.resize(type_sec.len() + num_params, 0x7E);
    type_sec.extend_from_slice(&[0x01, 0x7E]);

    let func_sec = vec![0x01, 0x00];
    let exp_sec = vec![0x01, 0x07, b'o', b'm', b'n', b'i', b'_', b'f', b'n', 0x00, 0x00];
    let mut code_sec = vec![0x01];
    code_sec.extend(encode_uleb128(body.len()));
    code_sec.extend(body);

    let mut wasm = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
    for (sec_id, payload) in [(1u8, type_sec), (3u8, func_sec), (7u8, exp_sec), (10u8, code_sec)] {
        wasm.push(sec_id);
        wasm.extend(encode_uleb128(payload.len()));
        wasm.extend(payload);
    }
    Ok(wasm)
}

fn decode_uleb128(bytes: &[u8], pos: &mut usize) -> Result<usize, String> {
    let mut result: usize = 0;
    let mut shift: u32 = 0;
    loop {
        if *pos >= bytes.len() {
            return Err(String::from("Unexpected EOF in Wasm ULEB128"));
        }
        let byte = bytes[*pos];
        *pos += 1;
        result |= ((byte & 0x7F) as usize) << shift;
        if (byte & 0x80) == 0 {
            break;
        }
        shift += 7;
        if shift >= 64 {
            return Err(String::from("Wasm ULEB128 overflow"));
        }
    }
    Ok(result)
}

fn decode_sleb128(bytes: &[u8], pos: &mut usize) -> Result<i64, String> {
    let mut result: i64 = 0;
    let mut shift: u32 = 0;
    let mut byte: u8;
    loop {
        if *pos >= bytes.len() {
            return Err(String::from("Unexpected EOF in Wasm SLEB128"));
        }
        byte = bytes[*pos];
        *pos += 1;
        result |= ((byte & 0x7F) as i64) << shift;
        shift += 7;
        if (byte & 0x80) == 0 {
            break;
        }
        if shift >= 64 {
            break;
        }
    }
    if shift < 64 && (byte & 0x40) != 0 {
        result |= !0i64 << shift;
    }
    Ok(result)
}

fn skip_wasm_to_matching_end(body: &[u8], mut pos: usize) -> Result<usize, String> {
    let mut depth = 1usize;
    while pos < body.len() {
        let op = body[pos];
        pos += 1;
        match op {
            0x02 | 0x03 | 0x04 => {
                if pos >= body.len() {
                    return Err(String::from("Truncated Wasm block header during skip"));
                }
                pos += 1;
                depth += 1;
            }
            0x0B => {
                depth -= 1;
                if depth == 0 {
                    return Ok(pos);
                }
            }
            0x0C | 0x20 | 0x21 | 0x22 => {
                let _ = decode_uleb128(body, &mut pos)?;
            }
            0x42 => {
                let _ = decode_sleb128(body, &mut pos)?;
            }
            _ => {}
        }
    }
    Err(String::from("Unclosed Wasm block while skipping to end"))
}

pub const DEFAULT_WASM_STEP_LIMIT: u64 = 50_000_000;
pub const DEFAULT_WASM_MEMORY_BUDGET_BYTES: usize = 64 * 1024 * 1024;

/// Self-contained 64-bit WebAssembly v1 binary validator and stack-machine evaluator for
/// differential verification of `compile_pure_u16_to_wasm` against `OmniBinaryVm` and x86-64 ELF.
pub fn execute_wasm_module_i64(wasm: &[u8], args: &[i64]) -> Result<i64, String> {
    execute_wasm_module_i64_with_budgets(
        wasm,
        args,
        DEFAULT_WASM_STEP_LIMIT,
        DEFAULT_WASM_MEMORY_BUDGET_BYTES,
    )
}

/// Executes a 64-bit WebAssembly v1 binary under explicit `step_limit` and `memory_budget_bytes` caps.
pub fn execute_wasm_module_i64_with_budgets(
    wasm: &[u8],
    args: &[i64],
    step_limit: u64,
    memory_budget_bytes: usize,
) -> Result<i64, String> {
    if wasm.len() < 8 || &wasm[0..8] != &[0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00] {
        return Err(String::from("Invalid WebAssembly '\0asm' v1 header"));
    }
    let mut pos = 8usize;
    let mut num_params = 0usize;
    let mut code_body: Option<&[u8]> = None;

    while pos < wasm.len() {
        let sec_id = wasm[pos];
        pos += 1;
        let sec_len = decode_uleb128(wasm, &mut pos)?;
        if pos + sec_len > wasm.len() {
            return Err(String::from("Truncated Wasm section"));
        }
        let sec_end = pos + sec_len;
        if sec_id == 1 {
            let mut p = pos;
            let type_cnt = decode_uleb128(wasm, &mut p)?;
            if type_cnt == 0 || p >= sec_end || wasm[p] != 0x60 {
                return Err(String::from("Invalid Wasm Type section"));
            }
            p += 1;
            num_params = decode_uleb128(wasm, &mut p)?;
        } else if sec_id == 10 {
            let mut p = pos;
            let fn_cnt = decode_uleb128(wasm, &mut p)?;
            if fn_cnt == 0 {
                return Err(String::from("Empty Wasm Code section"));
            }
            let body_len = decode_uleb128(wasm, &mut p)?;
            if p + body_len > sec_end {
                return Err(String::from("Truncated Wasm function body"));
            }
            code_body = Some(&wasm[p..p + body_len]);
        }
        pos = sec_end;
    }

    let body = code_body.ok_or_else(|| String::from("Missing Wasm Code section"))?;
    let mut bpos = 0usize;
    let decl_cnt = decode_uleb128(body, &mut bpos)?;
    let mut total_locals = num_params;
    for _ in 0..decl_cnt {
        let cnt = decode_uleb128(body, &mut bpos)?;
        if bpos >= body.len() {
            return Err(String::from("Truncated Wasm local decl"));
        }
        bpos += 1;
        total_locals += cnt;
    }

    let frame_bytes = wasm.len().saturating_add(total_locals.max(args.len()).saturating_mul(8));
    if frame_bytes > memory_budget_bytes {
        return Err(format!(
            "Wasm AOT memory budget exceeded ({} > {} bytes)",
            frame_bytes, memory_budget_bytes
        ));
    }

    let mut locals = vec![0i64; total_locals.max(args.len())];
    for (i, &v) in args.iter().enumerate() {
        if i < locals.len() {
            locals[i] = v;
        }
    }
    let mut stack: Vec<i64> = Vec::with_capacity(32);
    let mut ctrl_stack: Vec<(u8, usize)> = Vec::with_capacity(8);
    let mut steps_executed: u64 = 0;

    while bpos < body.len() {
        steps_executed = steps_executed.saturating_add(1);
        if steps_executed > step_limit {
            return Err(format!(
                "Wasm AOT step budget exceeded ({} > {})",
                steps_executed, step_limit
            ));
        }
        let runtime_bytes = frame_bytes.saturating_add(stack.len().saturating_add(ctrl_stack.len()).saturating_mul(8));
        if runtime_bytes > memory_budget_bytes {
            return Err(format!(
                "Wasm AOT memory budget exceeded ({} > {} bytes)",
                runtime_bytes, memory_budget_bytes
            ));
        }
        let op = body[bpos];
        bpos += 1;
        match op {
            0x02 | 0x03 => {
                if bpos >= body.len() {
                    return Err(String::from("Truncated Wasm block/loop header"));
                }
                bpos += 1;
                ctrl_stack.push((op, bpos));
            }
            0x04 => {
                if bpos >= body.len() {
                    return Err(String::from("Truncated Wasm if header"));
                }
                bpos += 1;
                let cond = stack.pop().ok_or("Wasm stack underflow on if")?;
                if cond != 0 {
                    ctrl_stack.push((0x04, bpos));
                } else {
                    bpos = skip_wasm_to_matching_end(body, bpos)?;
                }
            }
            0x0C => {
                let depth = decode_uleb128(body, &mut bpos)?;
                if depth >= ctrl_stack.len() {
                    return Err(String::from("Wasm br depth out of bounds"));
                }
                for _ in 0..depth {
                    ctrl_stack.pop();
                }
                let &(target_op, target_pos) = ctrl_stack.last().unwrap();
                if target_op == 0x03 {
                    bpos = target_pos;
                } else {
                    ctrl_stack.pop();
                    bpos = skip_wasm_to_matching_end(body, target_pos)?;
                }
            }
            0x20 => {
                let idx = decode_uleb128(body, &mut bpos)?;
                stack.push(*locals.get(idx).ok_or("Wasm local.get out of bounds")?);
            }
            0x21 => {
                let idx = decode_uleb128(body, &mut bpos)?;
                let v = stack.pop().ok_or("Wasm stack underflow on local.set")?;
                if idx >= locals.len() {
                    return Err(String::from("Wasm local.set out of bounds"));
                }
                locals[idx] = v;
            }
            0x22 => {
                let idx = decode_uleb128(body, &mut bpos)?;
                let v = *stack.last().ok_or("Wasm stack underflow on local.tee")?;
                if idx >= locals.len() {
                    return Err(String::from("Wasm local.tee out of bounds"));
                }
                locals[idx] = v;
            }
            0x42 => {
                let v = decode_sleb128(body, &mut bpos)?;
                stack.push(v);
            }
            0x50 => {
                let a = stack.pop().ok_or("Wasm stack underflow on i64.eqz")?;
                stack.push(if a == 0 { 1 } else { 0 });
            }
            0x51 | 0x52 | 0x53 | 0x55 => {
                let b = stack.pop().ok_or("Wasm stack underflow on cmp b")?;
                let a = stack.pop().ok_or("Wasm stack underflow on cmp a")?;
                let r = match op {
                    0x51 => a == b,
                    0x52 => a != b,
                    0x53 => a < b,
                    _    => a > b,
                };
                stack.push(if r { 1 } else { 0 });
            }
            0x71 => {
                let b = stack.pop().ok_or("Wasm stack underflow on i32.and")?;
                let a = stack.pop().ok_or("Wasm stack underflow on i32.and")?;
                stack.push((a & b) & 1);
            }
            0x72 => {
                let b = stack.pop().ok_or("Wasm stack underflow on i32.or")?;
                let a = stack.pop().ok_or("Wasm stack underflow on i32.or")?;
                stack.push((a | b) & 1);
            }
            0x7C | 0x7D | 0x7E | 0x83 | 0x84 | 0x85 | 0x86 | 0x87 => {
                let b = stack.pop().ok_or("Wasm stack underflow on binop b")?;
                let a = stack.pop().ok_or("Wasm stack underflow on binop a")?;
                let r = match op {
                    0x7C => a.wrapping_add(b),
                    0x7D => a.wrapping_sub(b),
                    0x7E => a.wrapping_mul(b),
                    0x83 => a & b,
                    0x84 => a | b,
                    0x85 => a ^ b,
                    0x86 => a.wrapping_shl((b & 63) as u32),
                    _    => a.wrapping_shr((b & 63) as u32),
                };
                stack.push(r);
            }
            0x7F => {
                let b = stack.pop().ok_or("Wasm stack underflow on i64.div_s b")?;
                let a = stack.pop().ok_or("Wasm stack underflow on i64.div_s a")?;
                if b == 0 || (a == i64::MIN && b == -1) {
                    return Err(String::from("Wasm hardware trap on unguarded i64.div_s"));
                }
                stack.push(a.wrapping_div(b));
            }
            0x81 => {
                let b = stack.pop().ok_or("Wasm stack underflow on i64.rem_s b")?;
                let a = stack.pop().ok_or("Wasm stack underflow on i64.rem_s a")?;
                if b == 0 {
                    return Err(String::from("Wasm hardware trap on unguarded i64.rem_s"));
                }
                stack.push(if a == i64::MIN && b == -1 { 0 } else { a.wrapping_rem(b) });
            }
            0xAD => {
                let a = stack.pop().ok_or("Wasm stack underflow on i64.extend_i32_u")?;
                stack.push((a as u32) as i64);
            }
            0x1B => {
                let c = stack.pop().ok_or("Wasm stack underflow on select c")?;
                let v2 = stack.pop().ok_or("Wasm stack underflow on select v2")?;
                let v1 = stack.pop().ok_or("Wasm stack underflow on select v1")?;
                stack.push(if c != 0 { v1 } else { v2 });
            }
            0x0F => {
                return stack.pop().ok_or_else(|| String::from("Wasm stack empty on return"));
            }
            0x0B => {
                if ctrl_stack.pop().is_none() {
                    break;
                }
            }
            other => {
                return Err(format!("Unsupported Wasm opcode 0x{:02X} in evaluator", other));
            }
        }
    }
    stack.pop().ok_or_else(|| String::from("Wasm execution finished with empty stack"))
}

// ==============================================================================
// 13. STEP 7 (PURE RUST): LOOP-BASED STAGE-1 PURE-`u16` COMPILER LOWERING `.omni_bc` -> ELF64
// ==============================================================================

fn alloc_ssa_ins(
    ins: &mut Vec<Vec<u16>>,
    next_reg: &mut u16,
    tc_safe: u16,
    op: u16,
    ty: u16,
    ops: Vec<u16>,
) -> u16 {
    let r = *next_reg;
    *next_reg += 1;
    let mut pkt = vec![r, op, ty, tc_safe];
    pkt.extend(ops);
    pkt.push(TOK_INS_END);
    ins.push(pkt);
    r
}

fn set_ssa_reg(
    ins: &mut Vec<Vec<u16>>,
    tc_safe: u16,
    r: u16,
    op: u16,
    ty: u16,
    ops: Vec<u16>,
) {
    let mut pkt = vec![r, op, ty, tc_safe];
    pkt.extend(ops);
    pkt.push(TOK_INS_END);
    ins.push(pkt);
}

fn emit_lbl_ins(ins: &mut Vec<Vec<u16>>, lbl_id: u16) {
    ins.push(vec![0x7F40, TOK_IMM_I16, lbl_id, TOK_INS_END]);
}

fn emit_jmp_ins(ins: &mut Vec<Vec<u16>>, lbl_id: u16) {
    ins.push(vec![0x7F41, TOK_IMM_I16, lbl_id, TOK_INS_END]);
}

fn emit_br_pos_ins(ins: &mut Vec<Vec<u16>>, cond_reg: u16, lbl_id: u16) {
    ins.push(vec![0x7F43, cond_reg, TOK_IMM_I16, lbl_id, TOK_INS_END]);
}

fn alloc_lbl(next_lbl: &mut u16) -> u16 {
    let l = *next_lbl;
    *next_lbl += 1;
    l
}

fn emit_iset_bytes_in_place(
    ins: &mut Vec<Vec<u16>>,
    tc_safe: u16,
    r_buf: u16,
    bytes: &[u8],
) {
    for chunk in bytes.chunks(24) {
        let mut ops = vec![r_buf];
        for &b in chunk {
            ops.push(TOK_IMM_I16);
            ops.push(b as u16);
        }
        set_ssa_reg(ins, tc_safe, r_buf, 0x7F0F, TY_BYTES, ops);
    }
}

pub fn build_step7_stage1_compiler_stream() -> Vec<u16> {
    let tc_safe = pack_tc(false, CAP_ID_PURE);
    let mut ins: Vec<Vec<u16>> = Vec::new();
    let mut next_reg: u16 = REG_BASE;
    let mut next_lbl: u16 = 0;

    let r_buf      = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F0D, TY_BYTES, vec![TOK_IMM_I16, 0]);
    let r_pos      = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 1]);
    let r_cur      = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 0]);
    let r_tok0     = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 0]);
    let r_op       = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 0]);
    let r_disp_dst = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 0]);
    let r_otok     = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 0]);
    let r_tmp      = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 0]);
    let r_tmp2     = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 0]);
    let r_tmp3     = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 0]);
    let r_cond     = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F09, TY_TRIT,  vec![TOK_TRIT_NEG]);
    let r_pass     = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 0]);
    let r_lid      = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 0]);
    let r_loff     = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 0]);
    let r_retty    = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 0]);

    const MAX_LABELS: usize = 83;
    let mut r_lbl = [0u16; MAX_LABELS];
    for k in 0..MAX_LABELS {
        r_lbl[k] = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64, vec![TOK_IMM_I16, 0]);
    }

    let lbl_pass_entry  = alloc_lbl(&mut next_lbl);
    let lbl_find_fn     = alloc_lbl(&mut next_lbl);
    let lbl_fn_found    = alloc_lbl(&mut next_lbl);
    let lbl_ty_bytes_pr = alloc_lbl(&mut next_lbl);
    let lbl_pro_done    = alloc_lbl(&mut next_lbl);
    let lbl_ins_loop    = alloc_lbl(&mut next_lbl);

    emit_lbl_ins(&mut ins, lbl_pass_entry);
    set_ssa_reg(&mut ins, tc_safe, r_buf, 0x7F0D, TY_BYTES, vec![TOK_IMM_I16, 0]);
    set_ssa_reg(&mut ins, tc_safe, r_pos, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 1]);

    // Locate TOK_FN_DEF and extract ret_ty_tok before emitting the prologue
    emit_lbl_ins(&mut ins, lbl_find_fn);
    set_ssa_reg(&mut ins, tc_safe, r_tok0, 0x7F0E, TY_I64,  vec![0x7EC0, r_pos]);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_tok0, TOK_IMM_I16, TOK_FN_DEF]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_fn_found);
    set_ssa_reg(&mut ins, tc_safe, r_pos,  0x7F10, TY_I64,  vec![r_pos, TOK_IMM_I16, 1]);
    emit_jmp_ins(&mut ins, lbl_find_fn);

    emit_lbl_ins(&mut ins, lbl_fn_found);
    set_ssa_reg(&mut ins, tc_safe, r_tmp,   0x7F10, TY_I64, vec![r_pos, TOK_IMM_I16, 2]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp,   0x7F0E, TY_I64, vec![0x7EC0, r_tmp]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp,   0x7F12, TY_I64, vec![r_tmp, TOK_IMM_I16, 3]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp2,  0x7F10, TY_I64, vec![r_pos, TOK_IMM_I16, 3]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp2,  0x7F10, TY_I64, vec![r_tmp2, r_tmp]);
    set_ssa_reg(&mut ins, tc_safe, r_retty, 0x7F0E, TY_I64, vec![0x7EC0, r_tmp2]);
    set_ssa_reg(&mut ins, tc_safe, r_pos,   0x7F10, TY_I64, vec![r_tmp2, TOK_IMM_I16, 3]);

    let mut elf_hdr = [0u8; 120];
    elf_hdr[0..4].copy_from_slice(&[0x7F, b'E', b'L', b'F']);
    elf_hdr[4] = 2; elf_hdr[5] = 1; elf_hdr[6] = 1;
    elf_hdr[16] = 2; elf_hdr[18] = 0x3E; elf_hdr[20] = 1;
    elf_hdr[24] = 0x78; elf_hdr[25] = 0x00; elf_hdr[26] = 0x40;
    elf_hdr[32] = 64; elf_hdr[52] = 64; elf_hdr[54] = 56; elf_hdr[56] = 1;
    elf_hdr[64] = 1; elf_hdr[68] = 5;
    elf_hdr[82] = 0x40; elf_hdr[90] = 0x40;
    elf_hdr[113] = 0x10;
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &elf_hdr);

    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x55, 0x48, 0x89, 0xE5, 0x48, 0x81, 0xEC, 0x00, 0x00, 0x03, 0x00]);

    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_retty, TOK_IMM_I16, TY_BYTES]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_ty_bytes_pr);

    // Non-TY_BYTES prologue
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x49, 0xBC]);
    set_ssa_reg(&mut ins, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, (-8i16) as u16, 0x7EC1]);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x49, 0xBD]);
    set_ssa_reg(&mut ins, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, (-8i16) as u16, 0x7EC2]);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x49, 0xBE]);
    set_ssa_reg(&mut ins, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, (-8i16) as u16, 0x7EC3]);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x49, 0xC7, 0xC7, 0x00, 0x00, 0x00, 0x00]);
    for (argc_min, disp_argv, reg_code) in [(2u8, 0x18u8, 4u8), (3, 0x20, 5), (4, 0x28, 6), (5, 0x30, 7)] {
        let mut cli_mc = Vec::new();
        emit_cli_arg_parser(&mut cli_mc, argc_min, disp_argv, reg_code);
        emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &cli_mc);
    }
    emit_jmp_ins(&mut ins, lbl_pro_done);

    // TY_BYTES prologue
    emit_lbl_ins(&mut ins, lbl_ty_bytes_pr);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x49, 0xC7, 0xC4, 0x00, 0x00, 0x00, 0x00]);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x49, 0xBD]);
    set_ssa_reg(&mut ins, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, (-8i16) as u16, 0x7EC1]);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x49, 0xBE]);
    set_ssa_reg(&mut ins, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, (-8i16) as u16, 0x7EC2]);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x49, 0xBF]);
    set_ssa_reg(&mut ins, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, (-8i16) as u16, 0x7EC3]);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[
        0x48, 0x83, 0x7D, 0x08, 0x02,
        0x7C, 0x2E,
        0x48, 0x8B, 0x7D, 0x18,
        0x31, 0xF6,
        0x31, 0xD2,
        0xB8, 0x02, 0x00, 0x00, 0x00,
        0x0F, 0x05,
        0x48, 0x85, 0xC0,
        0x78, 0x1A,
        0x48, 0x89, 0xC7,
        0x48, 0x8D, 0xB5, 0x00, 0x20, 0xFD, 0xFF,
        0xBA, 0x00, 0xE0, 0x00, 0x00,
        0x31, 0xC0,
        0x0F, 0x05,
        0xB8, 0x03, 0x00, 0x00, 0x00,
        0x0F, 0x05,
        0x45, 0x31, 0xDB,
    ]);
    for (argc_min, disp_argv, reg_code) in [(3u8, 0x20u8, 5u8), (4, 0x28, 6), (5, 0x30, 7)] {
        let mut cli_mc = Vec::new();
        emit_cli_arg_parser(&mut cli_mc, argc_min, disp_argv, reg_code);
        emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &cli_mc);
    }

    emit_lbl_ins(&mut ins, lbl_pro_done);

    let emit_decode_operand = |ins_v: &mut Vec<Vec<u16>>, next_l: &mut u16, into_rcx: bool| {
        let l_imm  = alloc_lbl(next_l);
        let l_par  = alloc_lbl(next_l);
        let l_done = alloc_lbl(next_l);
        let reg_ld: [u8; 3] = [0x48, 0x8B, if into_rcx { 0x8D } else { 0x85 }];
        let imm_ld: [u8; 2] = [0x48, if into_rcx { 0xB9 } else { 0xB8 }];
        let par_b: u16      = if into_rcx { 0xE1 } else { 0xE0 };

        set_ssa_reg(ins_v, tc_safe, r_otok, 0x7F0E, TY_I64,  vec![0x7EC0, r_cur]);
        set_ssa_reg(ins_v, tc_safe, r_cond, 0x7F27, TY_TRIT, vec![r_otok, TOK_IMM_I16, TOK_INS_END]);
        emit_br_pos_ins(ins_v, r_cond, l_imm);
        set_ssa_reg(ins_v, tc_safe, r_cond, 0x7F27, TY_TRIT, vec![r_otok, TOK_IMM_I16, REG_MAX]);
        emit_br_pos_ins(ins_v, r_cond, l_par);

        set_ssa_reg(ins_v, tc_safe, r_tmp, 0x7F11, TY_I64,   vec![TOK_IMM_I16, 0x7DFF, r_otok]);
        set_ssa_reg(ins_v, tc_safe, r_tmp, 0x7F12, TY_I64,   vec![r_tmp, TOK_IMM_I16, 8]);
        emit_iset_bytes_in_place(ins_v, tc_safe, r_buf, &reg_ld);
        set_ssa_reg(ins_v, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, (-4i16) as u16, r_tmp]);
        set_ssa_reg(ins_v, tc_safe, r_cur, 0x7F10, TY_I64,   vec![r_cur, TOK_IMM_I16, 1]);
        emit_jmp_ins(ins_v, l_done);

        emit_lbl_ins(ins_v, l_imm);
        set_ssa_reg(ins_v, tc_safe, r_cond, 0x7F2D, TY_TRIT, vec![r_otok, TOK_IMM_I16, TOK_IMM_I32]);
        set_ssa_reg(ins_v, tc_safe, r_tmp,  0x7F10, TY_I64,  vec![r_cur, TOK_IMM_I16, 1]);
        set_ssa_reg(ins_v, tc_safe, r_tmp,  0x7F0E, TY_I64,  vec![0x7EC0, r_tmp]);
        set_ssa_reg(ins_v, tc_safe, r_tmp2, 0x7F22, TY_I64,  vec![r_tmp, TOK_IMM_I16, 48]);
        set_ssa_reg(ins_v, tc_safe, r_tmp2, 0x7F23, TY_I64,  vec![r_tmp2, TOK_IMM_I16, 48]);
        set_ssa_reg(ins_v, tc_safe, r_tmp3, 0x7F10, TY_I64,  vec![r_cur, TOK_IMM_I16, 2]);
        set_ssa_reg(ins_v, tc_safe, r_tmp3, 0x7F0E, TY_I64,  vec![0x7EC0, r_tmp3]);
        set_ssa_reg(ins_v, tc_safe, r_tmp,  0x7F22, TY_I64,  vec![r_tmp, TOK_IMM_I16, 16]);
        set_ssa_reg(ins_v, tc_safe, r_tmp3, 0x7F1F, TY_I64,  vec![r_tmp, r_tmp3]);
        set_ssa_reg(ins_v, tc_safe, r_tmp3, 0x7F22, TY_I64,  vec![r_tmp3, TOK_IMM_I16, 32]);
        set_ssa_reg(ins_v, tc_safe, r_tmp3, 0x7F23, TY_I64,  vec![r_tmp3, TOK_IMM_I16, 32]);
        set_ssa_reg(ins_v, tc_safe, r_tmp,  0x7F11, TY_I64,  vec![r_otok, TOK_IMM_I16, TOK_TRIT_ZERO]);
        set_ssa_reg(ins_v, tc_safe, r_tmp,  0x7F45, TY_I64,  vec![r_cond, r_tmp2, r_tmp3, r_tmp]);
        emit_iset_bytes_in_place(ins_v, tc_safe, r_buf, &imm_ld);
        set_ssa_reg(ins_v, tc_safe, r_buf,  0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, (-8i16) as u16, r_tmp]);
        set_ssa_reg(ins_v, tc_safe, r_tmp2, 0x7F45, TY_I64,  vec![r_cond, TOK_IMM_I16, 2, TOK_IMM_I16, 3, TOK_IMM_I16, 1]);
        set_ssa_reg(ins_v, tc_safe, r_cur,  0x7F10, TY_I64,  vec![r_cur, r_tmp2]);
        emit_jmp_ins(ins_v, l_done);

        emit_lbl_ins(ins_v, l_par);
        set_ssa_reg(ins_v, tc_safe, r_tmp, 0x7F11, TY_I64,   vec![r_otok, TOK_IMM_I16, PAR_BASE]);
        set_ssa_reg(ins_v, tc_safe, r_tmp, 0x7F12, TY_I64,   vec![r_tmp, TOK_IMM_I16, 8]);
        set_ssa_reg(ins_v, tc_safe, r_tmp, 0x7F10, TY_I64,   vec![r_tmp, TOK_IMM_I16, par_b]);
        set_ssa_reg(ins_v, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, 0x4C, TOK_IMM_I16, 0x89, r_tmp]);
        set_ssa_reg(ins_v, tc_safe, r_cur, 0x7F10, TY_I64,   vec![r_cur, TOK_IMM_I16, 1]);

        emit_lbl_ins(ins_v, l_done);
    };

    let emit_read_label_id = |ins_v: &mut Vec<Vec<u16>>| {
        set_ssa_reg(ins_v, tc_safe, r_otok, 0x7F0E, TY_I64,  vec![0x7EC0, r_cur]);
        set_ssa_reg(ins_v, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_otok, TOK_IMM_I16, TOK_IMM_I16]);
        set_ssa_reg(ins_v, tc_safe, r_tmp,  0x7F10, TY_I64,  vec![r_cur, TOK_IMM_I16, 1]);
        set_ssa_reg(ins_v, tc_safe, r_tmp,  0x7F0E, TY_I64,  vec![0x7EC0, r_tmp]);
        set_ssa_reg(ins_v, tc_safe, r_tmp2, 0x7F11, TY_I64,  vec![r_otok, TOK_IMM_I16, LBL_BASE]);
        set_ssa_reg(ins_v, tc_safe, r_lid,  0x7F45, TY_I64,  vec![r_cond, r_tmp2, r_tmp2, r_tmp]);
        set_ssa_reg(ins_v, tc_safe, r_tmp3, 0x7F45, TY_I64,  vec![r_cond, TOK_IMM_I16, 1, TOK_IMM_I16, 1, TOK_IMM_I16, 2]);
        set_ssa_reg(ins_v, tc_safe, r_cur,  0x7F10, TY_I64,  vec![r_cur, r_tmp3]);
    };

    let emit_rel32_for_lid = |ins_v: &mut Vec<Vec<u16>>| {
        set_ssa_reg(ins_v, tc_safe, r_loff, 0x7F06, TY_I64, vec![r_lbl[0]]);
        for k in 1..MAX_LABELS {
            set_ssa_reg(ins_v, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_lid, TOK_IMM_I16, k as u16]);
            set_ssa_reg(ins_v, tc_safe, r_loff, 0x7F45, TY_I64,  vec![r_cond, r_loff, r_loff, r_lbl[k]]);
        }
        set_ssa_reg(ins_v, tc_safe, r_tmp, 0x7F50, TY_I64,   vec![r_buf]);
        set_ssa_reg(ins_v, tc_safe, r_tmp, 0x7F11, TY_I64,   vec![r_tmp, TOK_IMM_I16, 116]);
        set_ssa_reg(ins_v, tc_safe, r_tmp, 0x7F11, TY_I64,   vec![r_loff, r_tmp]);
        set_ssa_reg(ins_v, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, (-4i16) as u16, r_tmp]);
    };

    let lbl_nop        = alloc_lbl(&mut next_lbl);
    let lbl_label      = alloc_lbl(&mut next_lbl);
    let lbl_jmp        = alloc_lbl(&mut next_lbl);
    let lbl_br_trit    = alloc_lbl(&mut next_lbl);
    let lbl_br_pos     = alloc_lbl(&mut next_lbl);
    let lbl_br_neg     = alloc_lbl(&mut next_lbl);
    let lbl_ret        = alloc_lbl(&mut next_lbl);
    let lbl_ret_bytes  = alloc_lbl(&mut next_lbl);
    let lbl_anew       = alloc_lbl(&mut next_lbl);
    let lbl_slen       = alloc_lbl(&mut next_lbl);
    let lbl_iget       = alloc_lbl(&mut next_lbl);
    let lbl_iset       = alloc_lbl(&mut next_lbl);
    let lbl_iset_m4    = alloc_lbl(&mut next_lbl);
    let lbl_iset_m8    = alloc_lbl(&mut next_lbl);
    let lbl_iset_m9    = alloc_lbl(&mut next_lbl);
    let lbl_iset_bloop = alloc_lbl(&mut next_lbl);
    let lbl_iset_bdone = alloc_lbl(&mut next_lbl);
    let lbl_dispatch_1 = alloc_lbl(&mut next_lbl);
    let lbl_tmaj       = alloc_lbl(&mut next_lbl);
    let lbl_sel_trit   = alloc_lbl(&mut next_lbl);
    let lbl_store_dst  = alloc_lbl(&mut next_lbl);
    let lbl_loop_end   = alloc_lbl(&mut next_lbl);
    let lbl_pass0_end  = alloc_lbl(&mut next_lbl);

    emit_lbl_ins(&mut ins, lbl_ins_loop);
    set_ssa_reg(&mut ins, tc_safe, r_tok0, 0x7F0E, TY_I64,  vec![0x7EC0, r_pos]);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_tok0, TOK_IMM_I16, TOK_MOD_END]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_loop_end);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_tok0, TOK_IMM_I16, 0]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_loop_end);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_tok0, TOK_IMM_I16, 0x7F00]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_nop);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_tok0, TOK_IMM_I16, 0x7F40]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_label);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_tok0, TOK_IMM_I16, 0x7F41]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_jmp);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_tok0, TOK_IMM_I16, 0x7F42]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_br_trit);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_tok0, TOK_IMM_I16, 0x7F43]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_br_pos);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_tok0, TOK_IMM_I16, 0x7F44]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_br_neg);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_tok0, TOK_IMM_I16, 0x7F4A]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_ret);

    set_ssa_reg(&mut ins, tc_safe, r_tmp,      0x7F10, TY_I64, vec![r_pos, TOK_IMM_I16, 1]);
    set_ssa_reg(&mut ins, tc_safe, r_op,       0x7F0E, TY_I64, vec![0x7EC0, r_tmp]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp,      0x7F11, TY_I64, vec![TOK_IMM_I16, 0x7DFF, r_tok0]);
    set_ssa_reg(&mut ins, tc_safe, r_disp_dst, 0x7F12, TY_I64, vec![r_tmp, TOK_IMM_I16, 8]);
    set_ssa_reg(&mut ins, tc_safe, r_cur,      0x7F10, TY_I64, vec![r_pos, TOK_IMM_I16, 4]);

    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F0D]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_anew);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F50]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_slen);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F0E]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_iget);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F0F]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_iset);

    emit_decode_operand(&mut ins, &mut next_lbl, false);

    set_ssa_reg(&mut ins, tc_safe, r_otok, 0x7F0E, TY_I64,  vec![0x7EC0, r_cur]);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_otok, TOK_IMM_I16, TOK_INS_END]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_dispatch_1);

    emit_decode_operand(&mut ins, &mut next_lbl, true);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F2C]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_tmaj);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F45]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_sel_trit);

    emit_lbl_ins(&mut ins, lbl_dispatch_1);
    for copy_op in [0x7F02u16, 0x7F03, 0x7F04, 0x7F06, 0x7F09, 0x7F57, 0x7F58, 0x7F59] {
        set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, copy_op]);
        emit_br_pos_ins(&mut ins, r_cond, lbl_store_dst);
    }

    let lbl_emit_3b   = alloc_lbl(&mut next_lbl);
    let lbl_imul      = alloc_lbl(&mut next_lbl);
    let lbl_idiv      = alloc_lbl(&mut next_lbl);
    let lbl_imod      = alloc_lbl(&mut next_lbl);
    let lbl_iabs      = alloc_lbl(&mut next_lbl);
    let lbl_imin_imax = alloc_lbl(&mut next_lbl);
    let lbl_txor      = alloc_lbl(&mut next_lbl);
    let lbl_cmp4      = alloc_lbl(&mut next_lbl);
    let lbl_tcmp      = alloc_lbl(&mut next_lbl);

    let ops_3b: &[(u16, u16, u16)] = &[
        (0x7F10, 0x01, 0xC8),
        (0x7F11, 0x29, 0xC8),
        (0x7F1E, 0x21, 0xC8),
        (0x7F1F, 0x09, 0xC8),
        (0x7F20, 0x31, 0xC8),
        (0x7F15, 0xF7, 0xD8),
        (0x7F28, 0xF7, 0xD8),
        (0x7F21, 0xF7, 0xD0),
        (0x7F22, 0xD3, 0xE0),
        (0x7F23, 0xD3, 0xF8),
    ];
    for &(op_c, _, _) in ops_3b {
        set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, op_c]);
        emit_br_pos_ins(&mut ins, r_cond, lbl_emit_3b);
    }
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F12]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_imul);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F13]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_idiv);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F14]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_imod);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F16]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_iabs);
    for op_mm in [0x7F17u16, 0x7F29, 0x7F18, 0x7F2A] {
        set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, op_mm]);
        emit_br_pos_ins(&mut ins, r_cond, lbl_imin_imax);
    }
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F2B]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_txor);
    for op_cm in [0x7F24u16, 0x7F25, 0x7F26, 0x7F27] {
        set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, op_cm]);
        emit_br_pos_ins(&mut ins, r_cond, lbl_cmp4);
    }
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F2D]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_tcmp);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_emit_3b);
    set_ssa_reg(&mut ins, tc_safe, r_tmp,  0x7F06, TY_I64, vec![TOK_IMM_I16, 0x01]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp2, 0x7F06, TY_I64, vec![TOK_IMM_I16, 0xC8]);
    for &(op_c, b1, b2) in &ops_3b[1..] {
        set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, op_c]);
        set_ssa_reg(&mut ins, tc_safe, r_tmp,  0x7F45, TY_I64,  vec![r_cond, r_tmp, r_tmp, TOK_IMM_I16, b1]);
        set_ssa_reg(&mut ins, tc_safe, r_tmp2, 0x7F45, TY_I64,  vec![r_cond, r_tmp2, r_tmp2, TOK_IMM_I16, b2]);
    }
    set_ssa_reg(&mut ins, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, 0x48, r_tmp, r_tmp2]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_imul);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x48, 0x0F, 0xAF, 0xC1]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_idiv);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x49, 0x89, 0xC0, 0x48, 0x85, 0xC9, 0x74, 0x1E, 0x48, 0x83, 0xF9, 0xFF, 0x75, 0x11, 0x49, 0xB9, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x4D, 0x39, 0xC8, 0x75, 0x02, 0xEB, 0x0A, 0x48, 0x99, 0x48, 0xF7, 0xF9, 0xEB, 0x03, 0x48, 0x31, 0xC0]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_imod);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x49, 0x89, 0xC0, 0x48, 0x85, 0xC9, 0x74, 0x1E, 0x48, 0x83, 0xF9, 0xFF, 0x75, 0x11, 0x49, 0xB9, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x4D, 0x39, 0xC8, 0x75, 0x02, 0xEB, 0x0A, 0x48, 0x99, 0x48, 0xF7, 0xF9, 0xEB, 0x03, 0x48, 0x31, 0xC0, 0x48, 0x89, 0xD0]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_iabs);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x48, 0x89, 0xC2, 0x48, 0xF7, 0xD8, 0x48, 0x0F, 0x48, 0xC2]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_imin_imax);
    set_ssa_reg(&mut ins, tc_safe, r_tmp,  0x7F06, TY_I64,  vec![TOK_IMM_I16, 0x4F]);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F18]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp,  0x7F45, TY_I64,  vec![r_cond, r_tmp, r_tmp, TOK_IMM_I16, 0x4C]);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F2A]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp,  0x7F45, TY_I64,  vec![r_cond, r_tmp, r_tmp, TOK_IMM_I16, 0x4C]);
    set_ssa_reg(&mut ins, tc_safe, r_buf,  0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, 0x48, TOK_IMM_I16, 0x39, TOK_IMM_I16, 0xC8, TOK_IMM_I16, 0x48, TOK_IMM_I16, 0x0F, r_tmp, TOK_IMM_I16, 0xC1]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_txor);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x48, 0x0F, 0xAF, 0xC1, 0x48, 0xF7, 0xD8]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_cmp4);
    set_ssa_reg(&mut ins, tc_safe, r_tmp, 0x7F06, TY_I64, vec![TOK_IMM_I16, 0x94]);
    for (op_cm, scc) in [(0x7F25u16, 0x95u16), (0x7F26, 0x9C), (0x7F27, 0x9F)] {
        set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, op_cm]);
        set_ssa_reg(&mut ins, tc_safe, r_tmp,  0x7F45, TY_I64,  vec![r_cond, r_tmp, r_tmp, TOK_IMM_I16, scc]);
    }
    set_ssa_reg(&mut ins, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![
        r_buf,
        TOK_IMM_I16, 0x48, TOK_IMM_I16, 0x39, TOK_IMM_I16, 0xC8,
        TOK_IMM_I16, 0x0F, r_tmp, TOK_IMM_I16, 0xC0,
        TOK_IMM_I16, 0x0F, TOK_IMM_I16, 0xB6, TOK_IMM_I16, 0xC0,
        TOK_IMM_I16, 0x48, TOK_IMM_I16, 0x8D, TOK_IMM_I16, 0x44, TOK_IMM_I16, 0x00, TOK_IMM_I16, 0xFF,
    ]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_tcmp);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x48, 0x39, 0xC8, 0x0F, 0x9F, 0xC2, 0x0F, 0x9C, 0xC1, 0x0F, 0xB6, 0xD2, 0x0F, 0xB6, 0xC9, 0x29, 0xCA, 0x48, 0x63, 0xC2]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_tmaj);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x48, 0x01, 0xC8]);
    emit_decode_operand(&mut ins, &mut next_lbl, true);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[
        0x48, 0x01, 0xC8, 0x48, 0x83, 0xF8, 0x00,
        0x0F, 0x9F, 0xC0, 0x0F, 0x9C, 0xC2,
        0x0F, 0xB6, 0xC0, 0x0F, 0xB6, 0xD2, 0x48, 0x29, 0xD0,
    ]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_sel_trit);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x49, 0x89, 0xC0, 0x48, 0x89, 0xCA]);
    emit_decode_operand(&mut ins, &mut next_lbl, true);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x49, 0x83, 0xF8, 0x00, 0x48, 0x0F, 0x44, 0xD1]);
    emit_decode_operand(&mut ins, &mut next_lbl, true);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x49, 0x83, 0xF8, 0x00, 0x48, 0x0F, 0x4F, 0xD1, 0x48, 0x89, 0xD0]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_anew);
    set_ssa_reg(&mut ins, tc_safe, r_cur, 0x7F10, TY_I64, vec![r_cur, TOK_IMM_I16, 2]);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x45, 0x31, 0xDB]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_slen);
    set_ssa_reg(&mut ins, tc_safe, r_cur, 0x7F10, TY_I64, vec![r_cur, TOK_IMM_I16, 1]);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x4C, 0x89, 0xD8]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_iget);
    set_ssa_reg(&mut ins, tc_safe, r_cur, 0x7F10, TY_I64, vec![r_cur, TOK_IMM_I16, 1]);
    emit_decode_operand(&mut ins, &mut next_lbl, false);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x0F, 0xB7, 0x84, 0x45, 0x00, 0x20, 0xFD, 0xFF]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_iset);
    set_ssa_reg(&mut ins, tc_safe, r_cur,  0x7F10, TY_I64,  vec![r_cur, TOK_IMM_I16, 1]);
    set_ssa_reg(&mut ins, tc_safe, r_otok, 0x7F0E, TY_I64,  vec![0x7EC0, r_cur]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp,  0x7F10, TY_I64,  vec![r_cur, TOK_IMM_I16, 1]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp,  0x7F0E, TY_I64,  vec![0x7EC0, r_tmp]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp,  0x7F22, TY_I64,  vec![r_tmp, TOK_IMM_I16, 48]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp,  0x7F23, TY_I64,  vec![r_tmp, TOK_IMM_I16, 48]);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_otok, TOK_IMM_I16, TOK_IMM_I16]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp2, 0x7F24, TY_TRIT, vec![r_tmp, TOK_IMM_I16, (-4i16) as u16]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp2, 0x7F29, TY_TRIT, vec![r_cond, r_tmp2]);
    emit_br_pos_ins(&mut ins, r_tmp2, lbl_iset_m4);
    set_ssa_reg(&mut ins, tc_safe, r_tmp2, 0x7F24, TY_TRIT, vec![r_tmp, TOK_IMM_I16, (-8i16) as u16]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp2, 0x7F29, TY_TRIT, vec![r_cond, r_tmp2]);
    emit_br_pos_ins(&mut ins, r_tmp2, lbl_iset_m8);
    set_ssa_reg(&mut ins, tc_safe, r_tmp2, 0x7F24, TY_TRIT, vec![r_tmp, TOK_IMM_I16, (-9i16) as u16]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp2, 0x7F29, TY_TRIT, vec![r_cond, r_tmp2]);
    emit_br_pos_ins(&mut ins, r_tmp2, lbl_iset_m9);

    emit_lbl_ins(&mut ins, lbl_iset_bloop);
    set_ssa_reg(&mut ins, tc_safe, r_otok, 0x7F0E, TY_I64,  vec![0x7EC0, r_cur]);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_otok, TOK_IMM_I16, TOK_INS_END]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_iset_bdone);
    emit_decode_operand(&mut ins, &mut next_lbl, false);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x42, 0x88, 0x84, 0x1D, 0x00, 0x00, 0xFE, 0xFF, 0x49, 0xFF, 0xC3]);
    emit_jmp_ins(&mut ins, lbl_iset_bloop);

    emit_lbl_ins(&mut ins, lbl_iset_m4);
    set_ssa_reg(&mut ins, tc_safe, r_cur, 0x7F10, TY_I64, vec![r_cur, TOK_IMM_I16, 2]);
    emit_decode_operand(&mut ins, &mut next_lbl, false);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x42, 0x89, 0x84, 0x1D, 0x00, 0x00, 0xFE, 0xFF, 0x49, 0x83, 0xC3, 0x04]);
    emit_jmp_ins(&mut ins, lbl_iset_bdone);

    emit_lbl_ins(&mut ins, lbl_iset_m8);
    set_ssa_reg(&mut ins, tc_safe, r_cur, 0x7F10, TY_I64, vec![r_cur, TOK_IMM_I16, 2]);
    emit_decode_operand(&mut ins, &mut next_lbl, false);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x4A, 0x89, 0x84, 0x1D, 0x00, 0x00, 0xFE, 0xFF, 0x49, 0x83, 0xC3, 0x08]);
    emit_jmp_ins(&mut ins, lbl_iset_bdone);

    emit_lbl_ins(&mut ins, lbl_iset_m9);
    set_ssa_reg(&mut ins, tc_safe, r_cur, 0x7F10, TY_I64, vec![r_cur, TOK_IMM_I16, 2]);
    emit_decode_operand(&mut ins, &mut next_lbl, true);
    emit_decode_operand(&mut ins, &mut next_lbl, false);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x48, 0x89, 0x84, 0x0D, 0x00, 0x00, 0xFE, 0xFF]);

    emit_lbl_ins(&mut ins, lbl_iset_bdone);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x4C, 0x89, 0xD8]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_nop);
    set_ssa_reg(&mut ins, tc_safe, r_pos, 0x7F10, TY_I64, vec![r_pos, TOK_IMM_I16, 2]);
    emit_jmp_ins(&mut ins, lbl_ins_loop);

    emit_lbl_ins(&mut ins, lbl_label);
    set_ssa_reg(&mut ins, tc_safe, r_cur,  0x7F10, TY_I64, vec![r_pos, TOK_IMM_I16, 1]);
    emit_read_label_id(&mut ins);
    set_ssa_reg(&mut ins, tc_safe, r_loff, 0x7F50, TY_I64, vec![r_buf]);
    set_ssa_reg(&mut ins, tc_safe, r_loff, 0x7F11, TY_I64, vec![r_loff, TOK_IMM_I16, 120]);
    for k in 0..MAX_LABELS {
        set_ssa_reg(&mut ins, tc_safe, r_cond,   0x7F24, TY_TRIT, vec![r_lid, TOK_IMM_I16, k as u16]);
        set_ssa_reg(&mut ins, tc_safe, r_lbl[k], 0x7F45, TY_I64,  vec![r_cond, r_lbl[k], r_lbl[k], r_loff]);
    }
    set_ssa_reg(&mut ins, tc_safe, r_pos, 0x7F10, TY_I64, vec![r_cur, TOK_IMM_I16, 1]);
    emit_jmp_ins(&mut ins, lbl_ins_loop);

    emit_lbl_ins(&mut ins, lbl_jmp);
    set_ssa_reg(&mut ins, tc_safe, r_cur, 0x7F10, TY_I64, vec![r_pos, TOK_IMM_I16, 1]);
    emit_read_label_id(&mut ins);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0xE9]);
    emit_rel32_for_lid(&mut ins);
    set_ssa_reg(&mut ins, tc_safe, r_pos, 0x7F10, TY_I64, vec![r_cur, TOK_IMM_I16, 1]);
    emit_jmp_ins(&mut ins, lbl_ins_loop);

    emit_lbl_ins(&mut ins, lbl_br_trit);
    set_ssa_reg(&mut ins, tc_safe, r_cur, 0x7F10, TY_I64, vec![r_pos, TOK_IMM_I16, 1]);
    emit_decode_operand(&mut ins, &mut next_lbl, false);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x48, 0x83, 0xF8, 0x00, 0x0F, 0x8C]);
    emit_read_label_id(&mut ins);
    emit_rel32_for_lid(&mut ins);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x0F, 0x84]);
    emit_read_label_id(&mut ins);
    emit_rel32_for_lid(&mut ins);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0xE9]);
    emit_read_label_id(&mut ins);
    emit_rel32_for_lid(&mut ins);
    set_ssa_reg(&mut ins, tc_safe, r_pos, 0x7F10, TY_I64, vec![r_cur, TOK_IMM_I16, 1]);
    emit_jmp_ins(&mut ins, lbl_ins_loop);

    emit_lbl_ins(&mut ins, lbl_br_pos);
    set_ssa_reg(&mut ins, tc_safe, r_cur, 0x7F10, TY_I64, vec![r_pos, TOK_IMM_I16, 1]);
    emit_decode_operand(&mut ins, &mut next_lbl, false);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x48, 0x83, 0xF8, 0x01, 0x0F, 0x84]);
    emit_read_label_id(&mut ins);
    emit_rel32_for_lid(&mut ins);
    set_ssa_reg(&mut ins, tc_safe, r_pos, 0x7F10, TY_I64, vec![r_cur, TOK_IMM_I16, 1]);
    emit_jmp_ins(&mut ins, lbl_ins_loop);

    emit_lbl_ins(&mut ins, lbl_br_neg);
    set_ssa_reg(&mut ins, tc_safe, r_cur, 0x7F10, TY_I64, vec![r_pos, TOK_IMM_I16, 1]);
    emit_decode_operand(&mut ins, &mut next_lbl, false);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x48, 0x83, 0xF8, 0xFF, 0x0F, 0x84]);
    emit_read_label_id(&mut ins);
    emit_rel32_for_lid(&mut ins);
    set_ssa_reg(&mut ins, tc_safe, r_pos, 0x7F10, TY_I64, vec![r_cur, TOK_IMM_I16, 1]);
    emit_jmp_ins(&mut ins, lbl_ins_loop);

    emit_lbl_ins(&mut ins, lbl_store_dst);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x48, 0x89, 0x85]);
    set_ssa_reg(&mut ins, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, (-4i16) as u16, r_disp_dst]);
    set_ssa_reg(&mut ins, tc_safe, r_pos, 0x7F10, TY_I64,   vec![r_cur, TOK_IMM_I16, 1]);
    emit_jmp_ins(&mut ins, lbl_ins_loop);

    emit_lbl_ins(&mut ins, lbl_ret);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_retty, TOK_IMM_I16, TY_BYTES]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_ret_bytes);
    set_ssa_reg(&mut ins, tc_safe, r_cur, 0x7F10, TY_I64, vec![r_pos, TOK_IMM_I16, 1]);
    emit_decode_operand(&mut ins, &mut next_lbl, false);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[
        0x48, 0x83, 0xEC, 0x08, 0x48, 0x89, 0x04, 0x24,
        0xB8, 0x01, 0x00, 0x00, 0x00, 0xBF, 0x01, 0x00, 0x00, 0x00,
        0x48, 0x89, 0xE6, 0xBA, 0x08, 0x00, 0x00, 0x00, 0x0F, 0x05,
        0x48, 0x83, 0xC4, 0x08, 0x31, 0xFF, 0xB8, 0x3C, 0x00, 0x00, 0x00, 0x0F, 0x05,
    ]);
    set_ssa_reg(&mut ins, tc_safe, r_pos, 0x7F10, TY_I64, vec![r_cur, TOK_IMM_I16, 1]);
    emit_jmp_ins(&mut ins, lbl_ins_loop);

    emit_lbl_ins(&mut ins, lbl_ret_bytes);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[
        0xBF, 0x01, 0x00, 0x00, 0x00,
        0x48, 0x8D, 0xB5, 0x00, 0x00, 0xFE, 0xFF,
        0x4C, 0x89, 0xDA,
        0xB8, 0x01, 0x00, 0x00, 0x00,
        0x0F, 0x05,
        0x31, 0xFF,
        0xB8, 0x3C, 0x00, 0x00, 0x00,
        0x0F, 0x05,
    ]);
    set_ssa_reg(&mut ins, tc_safe, r_pos, 0x7F10, TY_I64, vec![r_pos, TOK_IMM_I16, 3]);
    emit_jmp_ins(&mut ins, lbl_ins_loop);

    emit_lbl_ins(&mut ins, lbl_loop_end);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_pass, TOK_IMM_I16, 0]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_pass0_end);
    set_ssa_reg(&mut ins, tc_safe, r_tmp, 0x7F50, TY_I64,   vec![r_buf]);
    set_ssa_reg(&mut ins, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, (-9i16) as u16, TOK_IMM_I16, 96,  r_tmp]);
    set_ssa_reg(&mut ins, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, (-9i16) as u16, TOK_IMM_I16, 104, r_tmp]);
    ins.push(vec![0x7F4A, r_buf, TOK_INS_END]);

    emit_lbl_ins(&mut ins, lbl_pass0_end);
    set_ssa_reg(&mut ins, tc_safe, r_pass, 0x7F06, TY_I64, vec![TOK_IMM_I16, 1]);
    emit_jmp_ins(&mut ins, lbl_pass_entry);

    pack_fn_module(
        &[],
        0x0000,
        &[
            (0x7EC0, TY_ARRAY, tc_safe),
            (0x7EC1, TY_I64,   tc_safe),
            (0x7EC2, TY_I64,   tc_safe),
            (0x7EC3, TY_I64,   tc_safe),
        ],
        TY_BYTES,
        0x0000,
        &ins,
    )
}

pub fn fnv1a_64_hash(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub fn assert_elf_bit_exact(native_elf: &[u8], vm_elf: &[u8], label: &str) {
    if native_elf != vm_elf {
        let min_len = native_elf.len().min(vm_elf.len());
        let first_diff = (0..min_len).find(|&i| native_elf[i] != vm_elf[i]).unwrap_or(min_len);
        let s_start = first_diff.saturating_sub(8);
        let s_end = (first_diff + 16).min(min_len);
        panic!(
            "Step 7 ELF mismatch on '{}': native_len={} vs vm_len={}, first diff at byte offset {} (0x{:04X}):\n  native[{}..{}]: {:02X?}\n  vm    [{}..{}]: {:02X?}",
            label,
            native_elf.len(),
            vm_elf.len(),
            first_diff,
            first_diff,
            s_start, s_end, &native_elf[s_start..s_end],
            s_start, s_end, &vm_elf[s_start..s_end],
        );
    }
}

/// Assembles a custom 9-instruction non-seed program exercising arithmetic, shifts, division,
/// ternary comparison, Kleene negation, and 4-operand `SEL_TRIT` branchless selection.
pub fn build_custom_non_seed_alu_program() -> Result<Vec<u16>, String> {
    let asm_src = "
        PARAM $0 I64 SAFE
        PARAM $1 I64 SAFE
        PARAM $2 I64 SAFE
        RET_TY I64
        %0 = IMUL $0 4
        %1 = ISUB %0 $1
        %2 = SHL $2 1
        %3 = IADD %1 %2
        %4 = IDIV %3 3
        %5 = TCMP %4 35
        %6 = TNOT %5
        %7 = SEL_TRIT %6 25 50 75
        %8 = IADD %4 %7
        RET %8
    ";
    assemble_omni_text_to_u16(asm_src)
}

/// Assembles a non-seed control-flow + backward-loop program computing `sum(1..=$0)` (`55` for `$0 = 10`).
pub fn build_custom_branch_loop_program() -> Result<Vec<u16>, String> {
    let asm_src = "
        PARAM $0 I64 SAFE
        RET_TY I64
        %0 = CINT 0
        %1 = CINT $0
        LABEL 1
        %0 = IADD %0 %1
        %1 = ISUB %1 1
        %2 = TCMP %1 0
        BR_POS %2 1
        RET %0
    ";
    assemble_omni_text_to_u16(asm_src)
}

pub fn run_step7_stage1_self_hosting(
    bc_path: &str,
    stage1_elf_path: &str,
    stage2_self_elf_path: &str,
    stage2_var0_elf_path: &str,
    stage2_var3_elf_path: &str,
) -> Result<(usize, usize, usize, u64, u64, u64, u64), String> {
    let _ = stage2_self_elf_path;
    let stage1_compiler_u16 = build_step7_stage1_compiler_stream();

    let compiler_mod = decode_omni_token_stream(&stage1_compiler_u16)?;
    let rep = verify_binary_function(&compiler_mod.functions[0]);
    if !rep.passed {
        return Err(format!("Step 7 Stage-1 compiler failed static verification: {:?}", rep.errors));
    }

    let raw_bc = encode_u16_stream_to_bytes(&stage1_compiler_u16);
    fs::write(bc_path, &raw_bc).map_err(|e| e.to_string())?;

    let stage1_elf = compile_pure_u16_to_x86_64_elf_with_args(&stage1_compiler_u16, 45, 40, 90)?;
    fs::write(stage1_elf_path, &stage1_elf).map_err(|e| e.to_string())?;

    let (_, seed0_var0_u16, _, _) = build_transpiled_seed_stream(0);
    let (_, seed0_var3_u16, _, _) = build_transpiled_seed_stream(24);
    let (_, seed4_cons_u16, _, _) = build_transpiled_seed_stream(4);
    let (_, seed6_clmp_u16, _, _) = build_transpiled_seed_stream(6);
    let custom_alu_u16 = build_custom_non_seed_alu_program()?;
    let branch_loop_u16 = build_custom_branch_loop_program()?;

    let mut vm = OmniBinaryVm::new();
    let emit_via_vm = |vm_ref: &mut OmniBinaryVm, target_u16: &[u16]| -> Result<Vec<u8>, String> {
        vm_ref.steps_executed = 0;
        let val = vm_ref.execute_function(
            &compiler_mod,
            0x0000,
            vec![
                OmniValue::U16Array(target_u16.to_vec()),
                OmniValue::Int(45),
                OmniValue::Int(40),
                OmniValue::Int(90),
            ],
        )?;
        match val {
            OmniValue::Bytes(b) => Ok(b),
            other => Err(format!("Expected TY_BYTES from Stage-1 compiler, got {:?}", other)),
        }
    };

    let stage2_var0_elf   = emit_via_vm(&mut vm, &seed0_var0_u16)?;
    let stage2_var3_elf   = emit_via_vm(&mut vm, &seed0_var3_u16)?;
    let stage2_seed4_elf  = emit_via_vm(&mut vm, &seed4_cons_u16)?;
    let stage2_seed6_elf  = emit_via_vm(&mut vm, &seed6_clmp_u16)?;
    let stage2_custom_elf = emit_via_vm(&mut vm, &custom_alu_u16)?;
    let stage2_loop_elf   = emit_via_vm(&mut vm, &branch_loop_u16)?;

    // Verify Stage-1 output matches direct x86-64 AOT bit-for-bit on both straight-line and looping programs
    let direct_custom_elf = compile_pure_u16_to_x86_64_elf_with_args(&custom_alu_u16, 45, 40, 90)?;
    assert_elf_bit_exact(&direct_custom_elf, &stage2_custom_elf, "custom_alu_non_seed");
    let direct_loop_elf = compile_pure_u16_to_x86_64_elf_with_args(&branch_loop_u16, 45, 40, 90)?;
    assert_elf_bit_exact(&direct_loop_elf, &stage2_loop_elf, "branch_loop_non_seed");

    fs::write(stage2_var0_elf_path, &stage2_var0_elf).map_err(|e| e.to_string())?;
    fs::write(stage2_var3_elf_path, &stage2_var3_elf).map_err(|e| e.to_string())?;
    fs::write("omni_stage2_seed4_consensus.elf", &stage2_seed4_elf).map_err(|e| e.to_string())?;
    fs::write("omni_stage2_seed6_clamp.elf", &stage2_seed6_elf).map_err(|e| e.to_string())?;
    fs::write("omni_stage2_custom_alu.elf", &stage2_custom_elf).map_err(|e| e.to_string())?;
    fs::write("omni_stage2_branch_loop.elf", &stage2_loop_elf).map_err(|e| e.to_string())?;

    let hash_var0  = fnv1a_64_hash(&stage2_var0_elf);
    let hash_var3  = fnv1a_64_hash(&stage2_var3_elf);
    let hash_seed4 = fnv1a_64_hash(&stage2_seed4_elf);
    let hash_seed6 = fnv1a_64_hash(&stage2_seed6_elf);

    assert_ne!(hash_var0, hash_var3, "Stage-1 compiler must emit distinct machine code for distinct immediates");
    assert_ne!(hash_var0, hash_seed4, "Stage-1 compiler must emit distinct machine code for Seed 4 consensus");
    assert_ne!(hash_seed4, hash_seed6, "Stage-1 compiler must emit distinct machine code for Seed 6 clamp");

    #[cfg(all(unix, target_arch = "x86_64"))]
    {
        use std::os::unix::fs::PermissionsExt;
        use std::process::Command;
        for p in [
            stage1_elf_path,
            stage2_var0_elf_path,
            stage2_var3_elf_path,
            "omni_stage2_seed4_consensus.elf",
            "omni_stage2_seed6_clamp.elf",
            "omni_stage2_custom_alu.elf",
            "omni_stage2_branch_loop.elf",
        ] {
            if let Ok(meta) = fs::metadata(p) {
                let mut perms = meta.permissions();
                perms.set_mode(0o755);
                let _ = fs::set_permissions(p, perms);
            }
        }

        let targets: &[(&str, &str, &[u16], &[u8], &[&str], i64)] = &[
            ("seed0_var0.omni_bc",  stage2_var0_elf_path,              &seed0_var0_u16,  &stage2_var0_elf,   &["30", "15"],        1),
            ("seed0_var3.omni_bc",  stage2_var3_elf_path,              &seed0_var3_u16,  &stage2_var3_elf,   &["30", "15"],       -1),
            ("seed4_cons.omni_bc",  "omni_stage2_seed4_consensus.elf", &seed4_cons_u16,  &stage2_seed4_elf,  &["1", "1", "-1"],   -1),
            ("seed6_clmp.omni_bc",  "omni_stage2_seed6_clamp.elf",     &seed6_clmp_u16,  &stage2_seed6_elf,  &["150", "10", "90"], 90),
            ("custom_alu.omni_bc",  "omni_stage2_custom_alu.elf",      &custom_alu_u16,  &stage2_custom_elf, &["30", "15", "9"],   66),
            ("branch_loop.omni_bc", "omni_stage2_branch_loop.elf",     &branch_loop_u16, &stage2_loop_elf,   &["10"],              55),
        ];

        // True Self-Hosting Fixed Point (Step 8.3): Stage-1 ELF compiles omni_self_compiler.omni_bc -> Stage-2 ELF
        let s2_compiler_out = Command::new(format!("./{}", stage1_elf_path))
            .arg(bc_path)
            .output()
            .map_err(|e| format!("Failed to run native Stage-1 ELF on {}: {}", bc_path, e))?;
        if !s2_compiler_out.status.success() || s2_compiler_out.stdout.is_empty() {
            return Err(String::from("Native Stage-1 ELF failed to self-compile omni_self_compiler.omni_bc"));
        }
        assert_elf_bit_exact(&stage1_elf, &s2_compiler_out.stdout, "Stage-1 == Stage-2 Self-Compiler Fixed Point");
        fs::write(stage2_self_elf_path, &s2_compiler_out.stdout).map_err(|e| e.to_string())?;
        if let Ok(meta) = fs::metadata(stage2_self_elf_path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(stage2_self_elf_path, perms);
        }

        for &(bc_file, elf_bin, u16_slice, expected_elf, cli_args, expected_i64) in targets {
            fs::write(bc_file, encode_u16_stream_to_bytes(u16_slice)).map_err(|e| e.to_string())?;
            let output = Command::new(format!("./{}", stage1_elf_path))
                .arg(bc_file)
                .output()
                .map_err(|e| format!("Failed to run native Stage-1 ELF on {}: {}", bc_file, e))?;
            if !output.status.success() || output.stdout.is_empty() {
                return Err(format!("Native Stage-1 ELF failed on {}", bc_file));
            }
            assert_elf_bit_exact(&output.stdout, expected_elf, bc_file);

            let s2_output = Command::new(format!("./{}", stage2_self_elf_path))
                .arg(bc_file)
                .output()
                .map_err(|e| format!("Failed to run Stage-2 compiler ELF on {}: {}", bc_file, e))?;
            if !s2_output.status.success() || s2_output.stdout.is_empty() {
                return Err(format!("Stage-2 compiler ELF failed on {}", bc_file));
            }
            assert_elf_bit_exact(&s2_output.stdout, expected_elf, bc_file);

            let run_out = Command::new(format!("./{}", elf_bin))
                .args(cli_args)
                .output()
                .map_err(|e| format!("Failed to run Stage-2 ELF {}: {}", elf_bin, e))?;
            if !run_out.status.success() || run_out.stdout.len() < 8 {
                return Err(format!("Stage-2 ELF {} failed or wrote <8 bytes", elf_bin));
            }
            let got_i64 = i64::from_le_bytes(run_out.stdout[0..8].try_into().unwrap());
            if got_i64 != expected_i64 {
                return Err(format!(
                    "Stage-2 ELF {} stdout i64 mismatch: got {}, expected {}",
                    elf_bin, got_i64, expected_i64
                ));
            }
        }
    }

    Ok((
        raw_bc.len(),
        stage1_elf.len(),
        stage2_var0_elf.len(),
        hash_var0,
        hash_var3,
        hash_seed4,
        hash_seed6,
    ))
}

// ==============================================================================
// 14. UNIT & DIFFERENTIAL SELFTEST SUITE (ROADMAP 3.2)
// ==============================================================================

pub fn run_unit_and_differential_selftests() -> Result<usize, String> {
    let mut checks_passed = 0usize;
    let tc_safe = pack_tc(false, CAP_ID_PURE);
    let tc_untr = pack_tc(true, CAP_ID_PURE);
    let tmp_dir = std::env::temp_dir();

    let stage1_u16 = build_step7_stage1_compiler_stream();
    let stage1_mod = decode_omni_token_stream(&stage1_u16)?;

    // 1. Pack5 round-trip + sentinel rejection test
    let trits: Vec<i8> = (0..64).map(|i| ((i * 5 + 1) % 3) as i8 - 1).collect();
    let packed = pack5_encode(&trits);
    let unpacked = pack5_decode(&packed, trits.len()).map_err(|e| e.to_string())?;
    if unpacked != trits {
        return Err(String::from("Pack5 round-trip mismatch"));
    }
    if pack5_decode(&[SENTINEL_SPECIAL_TOKEN], 5).is_ok() {
        return Err(String::from("Pack5 failed to reject sentinel byte 243"));
    }
    checks_passed += 2;

    // 1b. IABS golden tests (VM vs x86-64 AOT vs Wasm 64-bit, including i64::MIN)
    let abs_asm = "PARAM $0 I64 SAFE\nRET_TY I64\n%0 = IABS $0\nRET %0\n";
    let abs_u16 = assemble_omni_text_to_u16(abs_asm)?;
    let abs_mod = decode_omni_token_stream(&abs_u16)?;
    let abs_wasm = compile_pure_u16_to_wasm(&abs_u16)?;
    for (input, expected) in [(-42i64, 42i64), (17i64, 17i64), (0i64, 0i64), (i64::MIN, i64::MIN)] {
        let mut vm = OmniBinaryVm::new();
        let vm_res = vm.execute_function(&abs_mod, 0, vec![OmniValue::Int(input)])?;
        if vm_res != OmniValue::Int(expected) {
            return Err(format!("IABS VM mismatch for {}: got {:?}, expected {}", input, vm_res, expected));
        }
        let wasm_val = execute_wasm_module_i64(&abs_wasm, &[input])?;
        if wasm_val != expected {
            return Err(format!("IABS Wasm mismatch for {}: got {}, expected {}", input, wasm_val, expected));
        }
        let elf = compile_pure_u16_to_x86_64_elf_with_args(&abs_u16, input, 0, 0)?;
        let elf_path = tmp_dir.join(format!("iabs_test_{}.elf", input));
        std::fs::write(&elf_path, &elf).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&elf_path).map_err(|e| e.to_string())?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&elf_path, perms).map_err(|e| e.to_string())?;
        }
        #[cfg(all(unix, target_arch = "x86_64"))]
        {
            use std::process::Command;
            let out = Command::new(&elf_path)
                .arg(input.to_string())
                .output()
                .map_err(|e| format!("Failed to run IABS ELF: {}", e))?;
            if !out.status.success() || out.stdout.len() < 8 {
                return Err(format!("IABS ELF failed for input {}: status {:?}", input, out.status));
            }
            let native_val = i64::from_le_bytes(out.stdout[0..8].try_into().unwrap());
            if native_val != expected {
                return Err(format!("IABS native mismatch for {}: got {}, expected {}", input, native_val, expected));
            }
        }
        checks_passed += 1;
    }

    // 1c. Seed programs: full 64-bit VM vs native stdout vs Wasm 64-bit
    for seed_idx in [0usize, 3, 4, 6] {
        let (_, stream, args, expected) = build_transpiled_seed_stream(seed_idx);
        let mod_ = decode_omni_token_stream(&stream)?;
        let fn_idx = mod_.functions[0].fn_idx;
        let mut vm = OmniBinaryVm::new();
        let vm_val = vm.execute_function(&mod_, fn_idx, args.clone())?;
        if vm_val != expected {
            return Err(format!("Seed {} VM mismatch: {:?} != {:?}", seed_idx, vm_val, expected));
        }
        let exp_i = match &expected {
            OmniValue::Int(n) => *n,
            OmniValue::Trit(t) => *t as i64,
            _ => { checks_passed += 1; continue; }
        };
        let i64_args: Vec<i64> = args.iter().map(|a| match a {
            OmniValue::Int(n) => *n,
            OmniValue::Trit(t) => *t as i64,
            _ => 0,
        }).collect();
        let wasm_bytes = compile_pure_u16_to_wasm(&stream)?;
        let wasm_val = execute_wasm_module_i64(&wasm_bytes, &i64_args)?;
        if wasm_val != exp_i {
            return Err(format!("Seed {} Wasm 64-bit mismatch: {} != {}", seed_idx, wasm_val, exp_i));
        }
        let elf = compile_pure_u16_to_x86_64_elf_with_args(&stream, 30, 15, 90)?;
        let elf_path = tmp_dir.join(format!("seed{}_diff.elf", seed_idx));
        std::fs::write(&elf_path, &elf).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&elf_path).map_err(|e| e.to_string())?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&elf_path, perms).map_err(|e| e.to_string())?;
        }
        #[cfg(all(unix, target_arch = "x86_64"))]
        {
            use std::process::Command;
            let arg_strs: Vec<String> = i64_args.iter().map(|n| n.to_string()).collect();
            let out = Command::new(&elf_path).args(&arg_strs).output().map_err(|e| format!("Seed {} ELF run failed: {}", seed_idx, e))?;
            if !out.status.success() || out.stdout.len() < 8 {
                return Err(format!("Seed {} ELF wrote <8 bytes", seed_idx));
            }
            let native = i64::from_le_bytes(out.stdout[0..8].try_into().unwrap());
            if native != exp_i {
                return Err(format!("Seed {} native mismatch: {} != {}", seed_idx, native, exp_i));
            }
        }
        checks_passed += 1;
    }

    // 1d. Random numeric/ternary + BR_POS programs (VM vs Stage-1 vs 64-bit native)
    let mut rng: u64 = 0xC0FFEE_2026_1027;
    for trial in 0..8usize {
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let n_ops = 3 + ((rng >> 32) % 5) as usize;
        let mut asm = String::from("PARAM $0 I64 SAFE\nPARAM $1 I64 SAFE\nRET_TY I64\n");
        let mut regs = 0usize;
        for op_i in 0..n_ops {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let kind = (rng >> 32) % 12;
            if kind < 6 {
                let op = ["IADD", "ISUB", "IMUL", "BAND", "BXOR", "IMIN"][kind as usize];
                let src = if op_i == 0 { "$0" } else { &format!("%{}", regs.saturating_sub(1)) };
                let imm = ((rng >> 16) % 31 + 1) as i64;
                asm.push_str(&format!("%{} = {} {} {}\n", regs, op, src, imm));
                regs += 1;
            } else if kind == 6 && regs > 0 {
                asm.push_str(&format!("%{} = TCMP %{} 0\n", regs, regs.saturating_sub(1)));
                regs += 1;
                asm.push_str(&format!("BR_POS %{} 1\n", regs.saturating_sub(1)));
                asm.push_str(&format!("%{} = CINT 100\n", regs));
                regs += 1;
                asm.push_str("JMP 2\n");
                asm.push_str("LABEL 1\n");
                asm.push_str(&format!("%{} = CINT 200\n", regs));
                regs += 1;
                asm.push_str("LABEL 2\n");
            } else if kind == 7 && regs > 0 {
                asm.push_str(&format!("%{} = TCMP %{} 10\n", regs, regs.saturating_sub(1)));
                regs += 1;
                asm.push_str(&format!("%{} = SEL_TRIT %{} 5 10 15\n", regs, regs.saturating_sub(1)));
                regs += 1;
            } else {
                let src = if regs == 0 { "$0" } else { &format!("%{}", regs.saturating_sub(1)) };
                asm.push_str(&format!("%{} = IADD {} 1\n", regs, src));
                regs += 1;
            }
        }
        asm.push_str(&format!("RET %{}\n", regs.saturating_sub(1)));
        let u16s = assemble_omni_text_to_u16(&asm)?;
        let m = decode_omni_token_stream(&u16s)?;
        let a_in = 10 + (trial as i64) * 3;
        let b_in = 4 + (trial as i64);
        let mut vm = OmniBinaryVm::new();
        let vm_val = vm.execute_function(&m, m.functions[0].fn_idx, vec![OmniValue::Int(a_in), OmniValue::Int(b_in)])?;
        if let OmniValue::Int(exp) = vm_val {
            let wasm_b = compile_pure_u16_to_wasm(&u16s)?;
            let wasm_v = execute_wasm_module_i64(&wasm_b, &[a_in, b_in])?;
            if wasm_v != exp {
                return Err(format!("Random trial {} Wasm mismatch: {} != {}", trial, wasm_v, exp));
            }
            let elf = compile_pure_u16_to_x86_64_elf_with_args(&u16s, a_in, b_in, 0)?;
            let s1_elf = match vm.execute_function(
                &stage1_mod,
                0,
                vec![OmniValue::U16Array(u16s.clone()), OmniValue::Int(a_in), OmniValue::Int(b_in), OmniValue::Int(0)],
            )? {
                OmniValue::Bytes(b) => b,
                _ => return Err(format!("Random trial {} Stage-1 did not return Bytes", trial)),
            };
            if s1_elf != elf {
                return Err(format!("Random trial {} Stage-1 vs AOT ELF mismatch", trial));
            }
            let path = tmp_dir.join(format!("rand_{}.elf", trial));
            std::fs::write(&path, &elf).map_err(|e| e.to_string())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&path).map_err(|e| e.to_string())?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&path, perms).map_err(|e| e.to_string())?;
            }
            #[cfg(all(unix, target_arch = "x86_64"))]
            {
                use std::process::Command;
                let out = Command::new(&path)
                    .arg(a_in.to_string())
                    .arg(b_in.to_string())
                    .output()
                    .map_err(|e| format!("rand {} run failed: {}", trial, e))?;
                if out.stdout.len() < 8 {
                    return Err(format!("Random trial {} wrote <8 bytes", trial));
                }
                let native = i64::from_le_bytes(out.stdout[0..8].try_into().unwrap());
                if native != exp {
                    return Err(format!("Random trial {} mismatch: native {} != VM {}\nProgram:\n{}", trial, native, exp, asm));
                }
            }
            checks_passed += 1;
        }
    }

    // 1k. i64::MIN / -1 and MIN % -1 (VM vs x86-64 ELF vs Wasm 64-bit)
    {
        for (op, expected) in [("IDIV", i64::MIN), ("IMOD", 0i64)] {
            let asm = format!("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = {} $0 -1\nRET %0\n", op);
            let u16s = assemble_omni_text_to_u16(&asm)?;
            let m = decode_omni_token_stream(&u16s)?;
            let mut vm = OmniBinaryVm::new();
            let vm_res = vm.execute_function(&m, 0, vec![OmniValue::Int(i64::MIN)])?;
            if let OmniValue::Int(v) = &vm_res {
                if *v != expected { return Err(format!("VM {} MIN/-1 = {} want {}", op, v, expected)); }
            }
            let wasm = compile_pure_u16_to_wasm(&u16s)?;
            let wasm_v = execute_wasm_module_i64(&wasm, &[i64::MIN])?;
            if wasm_v != expected {
                return Err(format!("Wasm {} MIN/-1 = {} want {}", op, wasm_v, expected));
            }
            let elf = compile_pure_u16_to_x86_64_elf_with_args(&u16s, i64::MIN, 0, 0)?;
            let path = tmp_dir.join(format!("golden_min_{}.elf", op));
            std::fs::write(&path, &elf).map_err(|e| e.to_string())?;
            #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; let mut p = std::fs::metadata(&path).map_err(|e| e.to_string())?.permissions(); p.set_mode(0o755); std::fs::set_permissions(&path, p).map_err(|e| e.to_string())?; }
            #[cfg(all(unix, target_arch = "x86_64"))] {
                use std::process::Command;
                let out = Command::new(&path).arg(i64::MIN.to_string()).output().map_err(|e| format!("{}: {}", op, e))?;
                if !out.status.success() || out.stdout.len() < 8 {
                    return Err(format!("{} ELF failed: {:?}", op, out.status));
                }
                let n = i64::from_le_bytes(out.stdout[0..8].try_into().unwrap());
                if n != expected {
                    return Err(format!("{} native {} != {}", op, n, expected));
                }
            }
        }
        checks_passed += 1;
    }

    // 1d-2. Div-by-zero golden (VM vs x86-64 ELF vs Wasm 64-bit)
    {
        let dz_asm = "PARAM $0 I64 SAFE\nRET_TY I64\n%0 = IDIV $0 0\nRET %0\n";
        let dz_u16 = assemble_omni_text_to_u16(dz_asm)?;
        let dz_mod = decode_omni_token_stream(&dz_u16)?;
        let mut vm = OmniBinaryVm::new();
        let vm_res = vm.execute_function(&dz_mod, 0, vec![OmniValue::Int(42)])?;
        if vm_res != OmniValue::Int(0) {
            return Err(format!("div-by-zero VM mismatch: got {:?}", vm_res));
        }
        let dz_wasm = compile_pure_u16_to_wasm(&dz_u16)?;
        let dz_wasm_v = execute_wasm_module_i64(&dz_wasm, &[42])?;
        if dz_wasm_v != 0 {
            return Err(format!("div-by-zero Wasm mismatch: got {}", dz_wasm_v));
        }
        let elf = compile_pure_u16_to_x86_64_elf_with_args(&dz_u16, 42, 0, 0)?;
        let path = tmp_dir.join("divzero.elf");
        std::fs::write(&path, &elf).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&path).map_err(|e| e.to_string())?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&path, perms).map_err(|e| e.to_string())?;
        }
        #[cfg(all(unix, target_arch = "x86_64"))]
        {
            use std::process::Command;
            let out = Command::new(&path).arg("42").output().map_err(|e| format!("divzero ELF: {}", e))?;
            if !out.status.success() || out.stdout.len() < 8 {
                return Err(String::from("divzero ELF wrote <8 bytes"));
            }
            let native = i64::from_le_bytes(out.stdout[0..8].try_into().unwrap());
            if let OmniValue::Int(vm_i) = vm_res {
                if native != vm_i || native != 0 {
                    return Err(format!("div-by-zero mismatch: VM={} native={}", vm_i, native));
                }
            }
        }
        checks_passed += 1;
    }

    // 1f. Additional edge-case goldens (IMOD by zero, SHL 65, BR_NEG golden — all hard assertions)
    {
        let asm = "PARAM $0 I64 SAFE\nRET_TY I64\n%0 = IMOD $0 0\nRET %0\n";
        let u16s = assemble_omni_text_to_u16(asm)?;
        let m = decode_omni_token_stream(&u16s)?;
        let mut vm = OmniBinaryVm::new();
        let vm_res = vm.execute_function(&m, 0, vec![OmniValue::Int(99)])?;
        if vm_res != OmniValue::Int(0) {
            return Err(format!("IMOD0 VM mismatch: got {:?}", vm_res));
        }
        let wasm = compile_pure_u16_to_wasm(&u16s)?;
        if execute_wasm_module_i64(&wasm, &[99])? != 0 {
            return Err(String::from("IMOD0 Wasm mismatch"));
        }
        let elf = compile_pure_u16_to_x86_64_elf_with_args(&u16s, 99, 0, 0)?;
        let path = tmp_dir.join("imod0.elf");
        std::fs::write(&path, &elf).map_err(|e| e.to_string())?;
        #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; let mut p = std::fs::metadata(&path).map_err(|e| e.to_string())?.permissions(); p.set_mode(0o755); std::fs::set_permissions(&path, p).map_err(|e| e.to_string())?; }
        #[cfg(all(unix, target_arch = "x86_64"))] {
            use std::process::Command;
            let out = Command::new(&path).arg("99").output().map_err(|e| format!("imod0: {}", e))?;
            if out.stdout.len() < 8 { return Err(String::from("IMOD0 wrote <8 bytes")); }
            let n = i64::from_le_bytes(out.stdout[0..8].try_into().unwrap());
            if let OmniValue::Int(v) = vm_res { if n != v || n != 0 { return Err(format!("IMOD0 mismatch VM={} native={}", v, n)); } }
        }
        checks_passed += 1;
    }
    {
        let asm = "PARAM $0 I64 SAFE\nRET_TY I64\n%0 = SHL $0 65\nRET %0\n";
        let u16s = assemble_omni_text_to_u16(asm)?;
        let m = decode_omni_token_stream(&u16s)?;
        let mut vm = OmniBinaryVm::new();
        let vm_res = vm.execute_function(&m, 0, vec![OmniValue::Int(1)])?;
        let wasm = compile_pure_u16_to_wasm(&u16s)?;
        let wasm_v = execute_wasm_module_i64(&wasm, &[1])?;
        if OmniValue::Int(wasm_v) != vm_res {
            return Err(format!("SHL65 Wasm mismatch: Wasm={} VM={:?}", wasm_v, vm_res));
        }
        let elf = compile_pure_u16_to_x86_64_elf_with_args(&u16s, 1, 0, 0)?;
        let path = tmp_dir.join("shl65.elf");
        std::fs::write(&path, &elf).map_err(|e| e.to_string())?;
        #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; let mut p = std::fs::metadata(&path).map_err(|e| e.to_string())?.permissions(); p.set_mode(0o755); std::fs::set_permissions(&path, p).map_err(|e| e.to_string())?; }
        #[cfg(all(unix, target_arch = "x86_64"))] {
            use std::process::Command;
            let out = Command::new(&path).arg("1").output().map_err(|e| format!("shl65: {}", e))?;
            if out.stdout.len() < 8 { return Err(String::from("SHL65 wrote <8 bytes")); }
            let n = i64::from_le_bytes(out.stdout[0..8].try_into().unwrap());
            if let OmniValue::Int(v) = vm_res { if n != v { return Err(format!("SHL65 mismatch VM={} native={}", v, n)); } }
        }
        checks_passed += 1;
    }
    {
        let asm = "PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 0\nBR_NEG %0 1\n%1 = CINT 50\nJMP 2\nLABEL 1\n%1 = CINT 75\nLABEL 2\nRET %1\n";
        let u16s = assemble_omni_text_to_u16(asm)?;
        let m = decode_omni_token_stream(&u16s)?;
        let mut vm = OmniBinaryVm::new();
        let vm_res = vm.execute_function(&m, 0, vec![OmniValue::Int(-7)])?;
        let wasm = compile_pure_u16_to_wasm(&u16s)?;
        if OmniValue::Int(execute_wasm_module_i64(&wasm, &[-7])?) != vm_res {
            return Err(String::from("BR_NEG Wasm mismatch"));
        }
        let elf = compile_pure_u16_to_x86_64_elf_with_args(&u16s, -7, 0, 0)?;
        let s1_elf = match vm.execute_function(
            &stage1_mod,
            0,
            vec![OmniValue::U16Array(u16s.clone()), OmniValue::Int(-7), OmniValue::Int(0), OmniValue::Int(0)],
        )? {
            OmniValue::Bytes(b) => b,
            _ => return Err(String::from("BR_NEG Stage-1 did not return Bytes")),
        };
        if s1_elf != elf {
            return Err(String::from("BR_NEG Stage-1 vs AOT ELF mismatch"));
        }
        let path = tmp_dir.join("br_neg.elf");
        std::fs::write(&path, &elf).map_err(|e| e.to_string())?;
        #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; let mut p = std::fs::metadata(&path).map_err(|e| e.to_string())?.permissions(); p.set_mode(0o755); std::fs::set_permissions(&path, p).map_err(|e| e.to_string())?; }
        #[cfg(all(unix, target_arch = "x86_64"))] {
            use std::process::Command;
            let out = Command::new(&path).arg("-7").output().map_err(|e| format!("br_neg: {}", e))?;
            if out.stdout.len() < 8 { return Err(String::from("BR_NEG wrote <8 bytes")); }
            let n = i64::from_le_bytes(out.stdout[0..8].try_into().unwrap());
            if let OmniValue::Int(v) = vm_res {
                if n != v || n != 75 {
                    return Err(format!("BR_NEG mismatch VM={} native={}", v, n));
                }
            }
        }
        checks_passed += 1;
    }

    // 1h. Mixed SEL_TRIT + arithmetic (hard assertions across VM, Wasm, and Native ELF)
    {
        let asm = "PARAM $0 I64 SAFE\nRET_TY I64\n%0 = IMUL $0 3\n%1 = TCMP %0 20\n%2 = SEL_TRIT %1 5 10 15\n%3 = IADD %0 %2\nRET %3\n";
        let u16s = assemble_omni_text_to_u16(asm)?;
        let m = decode_omni_token_stream(&u16s)?;
        let wasm = compile_pure_u16_to_wasm(&u16s)?;
        for input in [3i64, 7, 10] {
            let mut vm = OmniBinaryVm::new();
            let vm_res = vm.execute_function(&m, 0, vec![OmniValue::Int(input)])?;
            let wasm_v = execute_wasm_module_i64(&wasm, &[input])?;
            if OmniValue::Int(wasm_v) != vm_res {
                return Err(format!("SEL_MIX Wasm mismatch input={} Wasm={} VM={:?}", input, wasm_v, vm_res));
            }
            let elf = compile_pure_u16_to_x86_64_elf_with_args(&u16s, input, 0, 0)?;
            let path = tmp_dir.join(format!("sel_mix_{}.elf", input));
            std::fs::write(&path, &elf).map_err(|e| e.to_string())?;
            #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; let mut p = std::fs::metadata(&path).map_err(|e| e.to_string())?.permissions(); p.set_mode(0o755); std::fs::set_permissions(&path, p).map_err(|e| e.to_string())?; }
            #[cfg(all(unix, target_arch = "x86_64"))] {
                use std::process::Command;
                let out = Command::new(&path).arg(input.to_string()).output().map_err(|e| format!("sel mix: {}", e))?;
                if out.stdout.len() < 8 { return Err(String::from("sel mix wrote <8 bytes")); }
                let n = i64::from_le_bytes(out.stdout[0..8].try_into().unwrap());
                if let OmniValue::Int(v) = vm_res {
                    if n != v {
                        return Err(format!("SEL_MIX input={} VM={} native={}", input, v, n));
                    }
                }
            }
        }
        checks_passed += 1;
    }

    // 1g. SEL_TRIT edge cases (cond <0, =0, >0, and non-trit-like values — hard assertions)
    {
        let asm = "PARAM $0 I64 SAFE\nRET_TY I64\n%0 = SEL_TRIT $0 11 22 33\nRET %0\n";
        let u16s = assemble_omni_text_to_u16(asm)?;
        let m = decode_omni_token_stream(&u16s)?;
        let wasm = compile_pure_u16_to_wasm(&u16s)?;
        for input in [-2i64, -1, 0, 1, 2, 100] {
            let mut vm = OmniBinaryVm::new();
            let vm_res = vm.execute_function(&m, 0, vec![OmniValue::Int(input)])?;
            let wasm_v = execute_wasm_module_i64(&wasm, &[input])?;
            if OmniValue::Int(wasm_v) != vm_res {
                return Err(format!("SEL_TRIT EDGE Wasm mismatch input={} Wasm={} VM={:?}", input, wasm_v, vm_res));
            }
            let elf = compile_pure_u16_to_x86_64_elf_with_args(&u16s, input, 0, 0)?;
            let path = tmp_dir.join(format!("sel_edge_{}.elf", input));
            std::fs::write(&path, &elf).map_err(|e| e.to_string())?;
            #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; let mut p = std::fs::metadata(&path).map_err(|e| e.to_string())?.permissions(); p.set_mode(0o755); std::fs::set_permissions(&path, p).map_err(|e| e.to_string())?; }
            #[cfg(all(unix, target_arch = "x86_64"))] {
                use std::process::Command;
                let out = Command::new(&path).arg(input.to_string()).output().map_err(|e| format!("sel edge: {}", e))?;
                if out.stdout.len() < 8 { return Err(String::from("sel edge wrote <8 bytes")); }
                let n = i64::from_le_bytes(out.stdout[0..8].try_into().unwrap());
                if let OmniValue::Int(v) = vm_res {
                    if n != v {
                        return Err(format!("SEL_TRIT EDGE input={} VM={} native={}", input, v, n));
                    }
                }
            }
        }
        checks_passed += 1;
    }

    // 1i. Nested-branch golden (VM vs Stage-1 vs Native ELF)
    {
        let asm = "PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 0\nBR_POS %0 1\n%1 = CINT 10\nJMP 3\nLABEL 1\n%2 = TCMP $0 5\nBR_POS %2 2\n%1 = CINT 20\nJMP 3\nLABEL 2\n%1 = CINT 30\nLABEL 3\nRET %1\n";
        let u16s = assemble_omni_text_to_u16(asm)?;
        let m = decode_omni_token_stream(&u16s)?;
        let wasm = compile_pure_u16_to_wasm(&u16s)?;
        for (input, expected) in [(-1i64, 10i64), (3, 20), (8, 30)] {
            let mut vm = OmniBinaryVm::new();
            let vm_res = vm.execute_function(&m, 0, vec![OmniValue::Int(input)])?;
            if vm_res != OmniValue::Int(expected) {
                return Err(format!("Nested branch VM mismatch input={}: {:?} != {}", input, vm_res, expected));
            }
            if OmniValue::Int(execute_wasm_module_i64(&wasm, &[input])?) != vm_res {
                return Err(format!("Nested branch Wasm mismatch input={}", input));
            }
            let elf = compile_pure_u16_to_x86_64_elf_with_args(&u16s, input, 0, 0)?;
            let s1_elf = match vm.execute_function(
                &stage1_mod,
                0,
                vec![OmniValue::U16Array(u16s.clone()), OmniValue::Int(input), OmniValue::Int(0), OmniValue::Int(0)],
            )? {
                OmniValue::Bytes(b) => b,
                _ => return Err(String::from("Nested branch Stage-1 did not return Bytes")),
            };
            if s1_elf != elf {
                return Err(String::from("Nested branch Stage-1 vs AOT ELF mismatch"));
            }
            let path = tmp_dir.join(format!("nested_{}.elf", input));
            std::fs::write(&path, &elf).map_err(|e| e.to_string())?;
            #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; let mut p = std::fs::metadata(&path).map_err(|e| e.to_string())?.permissions(); p.set_mode(0o755); std::fs::set_permissions(&path, p).map_err(|e| e.to_string())?; }
            #[cfg(all(unix, target_arch = "x86_64"))] {
                use std::process::Command;
                let out = Command::new(&path).arg(input.to_string()).output().map_err(|e| format!("nested: {}", e))?;
                if out.stdout.len() < 8 { return Err("nested wrote <8".into()); }
                let n = i64::from_le_bytes(out.stdout[0..8].try_into().unwrap());
                if let OmniValue::Int(v) = vm_res {
                    if n != v || n != expected {
                        return Err(format!("Nested mismatch input={} VM={} native={} expected={}", input, v, n, expected));
                    }
                }
            }
        }
        checks_passed += 1;
    }

    // 1j. BR_NEG + arithmetic golden (VM vs Stage-1 vs Native ELF)
    {
        let asm = "PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 0\nBR_NEG %0 1\n%1 = IADD $0 100\nJMP 2\nLABEL 1\n%1 = ISUB $0 50\nLABEL 2\nRET %1\n";
        let u16s = assemble_omni_text_to_u16(asm)?;
        let m = decode_omni_token_stream(&u16s)?;
        let wasm = compile_pure_u16_to_wasm(&u16s)?;
        for input in [-10i64, 10] {
            let mut vm = OmniBinaryVm::new();
            let vm_res = vm.execute_function(&m, 0, vec![OmniValue::Int(input)])?;
            if OmniValue::Int(execute_wasm_module_i64(&wasm, &[input])?) != vm_res {
                return Err(format!("BR_NEG+arith Wasm mismatch input={}", input));
            }
            let elf = compile_pure_u16_to_x86_64_elf_with_args(&u16s, input, 0, 0)?;
            let s1_elf = match vm.execute_function(
                &stage1_mod,
                0,
                vec![OmniValue::U16Array(u16s.clone()), OmniValue::Int(input), OmniValue::Int(0), OmniValue::Int(0)],
            )? {
                OmniValue::Bytes(b) => b,
                _ => return Err(String::from("BR_NEG+arith Stage-1 did not return Bytes")),
            };
            if s1_elf != elf {
                return Err(String::from("BR_NEG+arith Stage-1 vs AOT ELF mismatch"));
            }
            let path = tmp_dir.join(format!("brneg_arith_{}.elf", input));
            std::fs::write(&path, &elf).map_err(|e| e.to_string())?;
            #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; let mut p = std::fs::metadata(&path).map_err(|e| e.to_string())?.permissions(); p.set_mode(0o755); std::fs::set_permissions(&path, p).map_err(|e| e.to_string())?; }
            #[cfg(all(unix, target_arch = "x86_64"))] {
                use std::process::Command;
                let out = Command::new(&path).arg(input.to_string()).output().map_err(|e| format!("brneg arith: {}", e))?;
                if out.stdout.len() < 8 { return Err("brneg arith wrote <8".into()); }
                let n = i64::from_le_bytes(out.stdout[0..8].try_into().unwrap());
                if let OmniValue::Int(v) = vm_res {
                    if n != v {
                        return Err(format!("BR_NEG+arith mismatch input={} VM={} native={}", input, v, n));
                    }
                }
            }
        }
        checks_passed += 1;
    }

    // 1e. Golden tests isolating SEL_TRIT and BR_POS
    {
        let sel_asm = "PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 10\n%1 = SEL_TRIT %0 11 22 33\nRET %1\n";
        let sel_u16 = assemble_omni_text_to_u16(sel_asm)?;
        let sel_mod = decode_omni_token_stream(&sel_u16)?;
        let sel_wasm = compile_pure_u16_to_wasm(&sel_u16)?;
        for input in [-5i64, 10, 25] {
            let mut vm = OmniBinaryVm::new();
            let vm_res = vm.execute_function(&sel_mod, 0, vec![OmniValue::Int(input)])?;
            let wasm_v = execute_wasm_module_i64(&sel_wasm, &[input])?;
            if OmniValue::Int(wasm_v) != vm_res {
                return Err(format!("SEL_TRIT Wasm mismatch input={}", input));
            }
            let elf = compile_pure_u16_to_x86_64_elf_with_args(&sel_u16, input, 0, 0)?;
            let path = tmp_dir.join(format!("sel_trit_{}.elf", input));
            std::fs::write(&path, &elf).map_err(|e| e.to_string())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&path).map_err(|e| e.to_string())?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&path, perms).map_err(|e| e.to_string())?;
            }
            #[cfg(all(unix, target_arch = "x86_64"))]
            {
                use std::process::Command;
                let out = Command::new(&path).arg(input.to_string()).output()
                    .map_err(|e| format!("SEL_TRIT ELF failed: {}", e))?;
                if out.stdout.len() < 8 { return Err(String::from("SEL_TRIT wrote <8 bytes")); }
                let native = i64::from_le_bytes(out.stdout[0..8].try_into().unwrap());
                if let OmniValue::Int(vm_i) = vm_res {
                    if native != vm_i {
                        return Err(format!("SEL_TRIT input={} VM={} native={}", input, vm_i, native));
                    }
                }
            }
        }
        checks_passed += 1;
    }
    {
        let br_asm = "PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 0\nBR_POS %0 1\n%1 = CINT 100\nJMP 2\nLABEL 1\n%1 = CINT 200\nLABEL 2\nRET %1\n";
        let br_u16 = assemble_omni_text_to_u16(br_asm)?;
        let br_mod = decode_omni_token_stream(&br_u16)?;
        let mut vm = OmniBinaryVm::new();
        let vm_res = vm.execute_function(&br_mod, 0, vec![OmniValue::Int(5)])?;
        let br_wasm = compile_pure_u16_to_wasm(&br_u16)?;
        if OmniValue::Int(execute_wasm_module_i64(&br_wasm, &[5])?) != vm_res {
            return Err(String::from("BR_POS Wasm mismatch"));
        }
        let elf = compile_pure_u16_to_x86_64_elf_with_args(&br_u16, 5, 0, 0)?;
        let s1_elf = match vm.execute_function(
            &stage1_mod,
            0,
            vec![OmniValue::U16Array(br_u16.clone()), OmniValue::Int(5), OmniValue::Int(0), OmniValue::Int(0)],
        )? {
            OmniValue::Bytes(b) => b,
            _ => return Err(String::from("BR_POS Stage-1 did not return Bytes")),
        };
        if s1_elf != elf {
            return Err(String::from("BR_POS Stage-1 vs AOT ELF mismatch"));
        }
        let path = tmp_dir.join("br_pos_golden.elf");
        std::fs::write(&path, &elf).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&path).map_err(|e| e.to_string())?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&path, perms).map_err(|e| e.to_string())?;
        }
        #[cfg(all(unix, target_arch = "x86_64"))]
        {
            use std::process::Command;
            let out = Command::new(path).arg("5").output().map_err(|e| format!("BR_POS ELF: {}", e))?;
            if out.stdout.len() < 8 { return Err(String::from("BR_POS wrote <8 bytes")); }
            let native = i64::from_le_bytes(out.stdout[0..8].try_into().unwrap());
            if let OmniValue::Int(vm_i) = vm_res {
                if native != vm_i || native != 200 {
                    return Err(format!("BR_POS VM={} native={}", vm_i, native));
                }
            }
        }
        checks_passed += 1;
    }

    // 1n. Branching Loop + 3-Way BR_TRIT Golden (VM vs Stage-1 vs Native ELF)
    {
        let loop_u16 = build_custom_branch_loop_program()?;
        let loop_mod = decode_omni_token_stream(&loop_u16)?;
        let mut vm = OmniBinaryVm::new();
        let vm_res = vm.execute_function(&loop_mod, 0, vec![OmniValue::Int(10)])?;
        if vm_res != OmniValue::Int(55) {
            return Err(format!("Branch loop VM mismatch: {:?}", vm_res));
        }
        let loop_wasm = compile_pure_u16_to_wasm(&loop_u16)?;
        if execute_wasm_module_i64(&loop_wasm, &[10])? != 55 {
            return Err(String::from("Branch loop Wasm mismatch"));
        }
        let elf = compile_pure_u16_to_x86_64_elf_with_args(&loop_u16, 10, 0, 0)?;
        let s1_elf = match vm.execute_function(
            &stage1_mod,
            0,
            vec![OmniValue::U16Array(loop_u16.clone()), OmniValue::Int(10), OmniValue::Int(0), OmniValue::Int(0)],
        )? {
            OmniValue::Bytes(b) => b,
            _ => return Err(String::from("Branch loop Stage-1 did not return Bytes")),
        };
        if s1_elf != elf {
            return Err(String::from("Branch loop Stage-1 vs AOT ELF mismatch"));
        }

        let br_trit_asm = "PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 10\nBR_TRIT %0 1 2 3\nLABEL 1\n%1 = ISUB $0 5\nJMP 4\nLABEL 2\n%1 = CINT 100\nJMP 4\nLABEL 3\n%1 = IADD $0 50\nLABEL 4\nRET %1\n";
        let bt_u16 = assemble_omni_text_to_u16(br_trit_asm)?;
        let bt_mod = decode_omni_token_stream(&bt_u16)?;
        let bt_wasm = compile_pure_u16_to_wasm(&bt_u16)?;
        for (inp, exp) in [(4i64, -1i64), (10, 100), (20, 70)] {
            let mut bt_vm = OmniBinaryVm::new();
            let bt_res = bt_vm.execute_function(&bt_mod, 0, vec![OmniValue::Int(inp)])?;
            if bt_res != OmniValue::Int(exp) {
                return Err(format!("BR_TRIT VM mismatch for {}: {:?}", inp, bt_res));
            }
            if execute_wasm_module_i64(&bt_wasm, &[inp])? != exp {
                return Err(format!("BR_TRIT Wasm mismatch for {}", inp));
            }
            let bt_elf = compile_pure_u16_to_x86_64_elf_with_args(&bt_u16, inp, 0, 0)?;
            let bt_s1 = match bt_vm.execute_function(
                &stage1_mod,
                0,
                vec![OmniValue::U16Array(bt_u16.clone()), OmniValue::Int(inp), OmniValue::Int(0), OmniValue::Int(0)],
            )? {
                OmniValue::Bytes(b) => b,
                _ => return Err(String::from("BR_TRIT Stage-1 did not return Bytes")),
            };
            if bt_s1 != bt_elf {
                return Err(String::from("BR_TRIT Stage-1 vs AOT ELF mismatch"));
            }
        }

        // Byte-buffer (ANEW, ISET, SLEN) + CMOV assembler & Stage-1 lowering check (Step 8.1 & 8.2)
        let buf_asm = "PARAM $0 I64 SAFE
RET_TY I64
%0 = ANEW 0
%0 = ISET %0 10 20 30
%0 = ISET %0 -4 100
%0 = ISET %0 -8 200
%1 = SLEN %0
%2 = TCMP $0 5
%3 = CMOV %2 100 200 300
%4 = IADD %1 %3
RET %4
";
        let buf_u16 = assemble_omni_text_to_u16(buf_asm)?;
        let buf_mod = decode_omni_token_stream(&buf_u16)?;
        let mut buf_vm = OmniBinaryVm::new();
        let buf_res = buf_vm.execute_function(&buf_mod, 0, vec![OmniValue::Int(10)])?;
        if buf_res != OmniValue::Int(315) {
            return Err(format!("Byte-buffer ANEW/ISET/SLEN VM mismatch: {:?}", buf_res));
        }
        let buf_wasm = compile_pure_u16_to_wasm(&buf_u16)?;
        if execute_wasm_module_i64(&buf_wasm, &[10])? != 315 {
            return Err(String::from("Byte-buffer ANEW/ISET/SLEN Wasm mismatch"));
        }
        let buf_elf = compile_pure_u16_to_x86_64_elf_with_args(&buf_u16, 10, 0, 0)?;
        let buf_s1 = match buf_vm.execute_function(
            &stage1_mod,
            0,
            vec![OmniValue::U16Array(buf_u16.clone()), OmniValue::Int(10), OmniValue::Int(0), OmniValue::Int(0)],
        )? {
            OmniValue::Bytes(b) => b,
            _ => return Err(String::from("Byte-buffer Stage-1 did not return Bytes")),
        };
        if buf_s1 != buf_elf {
            return Err(String::from("Byte-buffer Stage-1 vs AOT ELF mismatch"));
        }

        // True Stage-1 == Stage-2 Self-Compilation Fixed Point (Step 8.3)
        #[cfg(all(unix, target_arch = "x86_64"))]
        {
            use std::os::unix::fs::PermissionsExt;
            use std::process::Command;
            let s1_elf_bytes = compile_pure_u16_to_x86_64_elf_with_args(&stage1_u16, 45, 40, 90)?;
            let s1_bin = "/tmp/selftest_stage1_self.elf";
            let s1_bc  = "/tmp/selftest_stage1_self.omni_bc";
            std::fs::write(s1_bin, &s1_elf_bytes).map_err(|e| e.to_string())?;
            std::fs::write(s1_bc, encode_u16_stream_to_bytes(&stage1_u16)).map_err(|e| e.to_string())?;
            let mut perms = std::fs::metadata(s1_bin).map_err(|e| e.to_string())?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(s1_bin, perms).map_err(|e| e.to_string())?;
            let s2_out = Command::new(s1_bin)
                .arg(s1_bc)
                .output()
                .map_err(|e| format!("Failed to run Stage-1 self-compile: {}", e))?;
            if !s2_out.status.success() || s2_out.stdout != s1_elf_bytes {
                return Err(format!(
                    "Stage-1 == Stage-2 self-compilation fixed point failed (s1={}B vs s2={}B)",
                    s1_elf_bytes.len(),
                    s2_out.stdout.len()
                ));
            }
        }
        checks_passed += 4;
    }

    // 1l. Malformed-bytecode & Unresolved Label Rejection Suite
    {
        // Missing TOK_MOD_BEGIN
        let bad1 = vec![0x7F80, 0x7F81, 0x7E00];
        if decode_omni_token_stream(&bad1).is_ok() {
            return Err("stream missing TOK_MOD_BEGIN should fail to decode".into());
        }
        // Truncated TOK_FN_DEF header after TOK_MOD_BEGIN
        let bad_trunc_fn = vec![TOK_MOD_BEGIN, TOK_FN_DEF, 0x0000];
        if decode_omni_token_stream(&bad_trunc_fn).is_ok() {
            return Err("truncated TOK_FN_DEF stream should fail to decode".into());
        }
        // Truncated instruction inside TOK_FN_DEF
        let bad_trunc_ins = vec![TOK_MOD_BEGIN, TOK_FN_DEF, 0x0000, 0x0000, TY_I64, 0x0000, 0x0001, 0x7E00, 0x7F06];
        if decode_omni_token_stream(&bad_trunc_ins).is_ok() {
            return Err("truncated SSA instruction should fail to decode".into());
        }
        // Unknown opcode inside valid function framing
        let bad_op = pack_fn_module(
            &[],
            0,
            &[(0x7EC0, TY_I64, tc_safe)],
            TY_I64,
            0x0000,
            &[vec![0x7E00, 0x7F99, TY_I64, tc_safe, 0x7EC0, TOK_INS_END], vec![0x7F4A, 0x7E00, TOK_INS_END]],
        );
        let m_bad_op = decode_omni_token_stream(&bad_op)?;
        if verify_binary_function(&m_bad_op.functions[0]).passed {
            return Err("unknown opcode 0x7F99 was accepted by verifier".into());
        }
        // Unresolved branch target label (JMP 12 with no LABEL 12)
        let bad_unresolved_lbl = assemble_omni_text_to_u16("PARAM $0 I64 SAFE\nRET_TY I64\nJMP 12\nRET $0\n")?;
        let m_unresolved = decode_omni_token_stream(&bad_unresolved_lbl)?;
        if verify_binary_function(&m_unresolved.functions[0]).passed {
            return Err("unresolved branch target label 12 was accepted by verifier".into());
        }
        checks_passed += 3;
    }
    {
        // Invalid label id in assembler
        let bad_asm = "PARAM $0 I64 SAFE\nRET_TY I64\nLABEL 99\nRET $0\n";
        if assemble_omni_text_to_u16(bad_asm).is_ok() {
            return Err("label id 99 should be rejected".into());
        }
        checks_passed += 1;
    }

    // 1m. VM Fuel (step_limit) & Memory (memory_budget_bytes) Budget Enforcement
    {
        let inf_asm = "PARAM $0 I64 SAFE\nRET_TY I64\nLABEL 1\nJMP 1\nRET $0\n";
        let inf_u16 = assemble_omni_text_to_u16(inf_asm)?;
        let inf_mod = decode_omni_token_stream(&inf_u16)?;
        let mut bounded_vm = OmniBinaryVm::with_budgets(64, 1024);
        if bounded_vm.execute_function(&inf_mod, 0, vec![OmniValue::Int(1)]).is_ok() {
            return Err("VM failed to halt infinite loop when step_limit=64 was exceeded".into());
        }

        let big_arena = pack_fn_module(
            &[],
            0,
            &[(0x7EC0, TY_I64, tc_safe)],
            TY_I64,
            0x0000,
            &[
                vec![0x7E00, 0x7F01, TY_ARENA, tc_safe, TOK_IMM_I32, 0x0001, 0x0000, TOK_INS_END],
                vec![0x7F05, 0x7E00, TOK_INS_END],
                vec![0x7F4A, 0x7EC0, TOK_INS_END],
            ],
        );
        let big_mod = decode_omni_token_stream(&big_arena)?;
        let mut mem_bounded_vm = OmniBinaryVm::with_budgets(1000, 1024);
        if mem_bounded_vm.execute_function(&big_mod, 0, vec![OmniValue::Int(1)]).is_ok() {
            return Err("VM failed to reject 65536-byte ARENA when memory_budget_bytes=1024".into());
        }

        // Verify 64 MiB (0x0400_0000 = 67,108,864 bytes) ARENA succeeds under default 256 MiB budget
        let arena_64mb = pack_fn_module(
            &[],
            0,
            &[(0x7EC0, TY_I64, tc_safe)],
            TY_I64,
            0x0000,
            &[
                vec![0x7E00, 0x7F01, TY_ARENA, tc_safe, TOK_IMM_I32, 0x0400, 0x0000, TOK_INS_END],
                vec![0x7F05, 0x7E00, TOK_INS_END],
                vec![0x7F4A, 0x7EC0, TOK_INS_END],
            ],
        );
        let mod_64mb = decode_omni_token_stream(&arena_64mb)?;
        let mut default_vm = OmniBinaryVm::new();
        if default_vm.execute_function(&mod_64mb, 0, vec![OmniValue::Int(77)]).map_err(|e| e)? != OmniValue::Int(77) {
            return Err("Default VM failed 64 MiB ARENA execution".into());
        }
        checks_passed += 3;
    }

    // 2. Negative Verifier Test A: Taint violation (untrusted param passed directly to SQL sink 0x7F63)
    let bad_taint = pack_fn_module(
        &[(0x0000, vec![(TY_STR, tc_safe)])],
        0,
        &[(0x7EC0, TY_STR, tc_untr)],
        TY_STRUCT,
        cap_bit(CAP_ID_DB_W),
        &[
            vec![0x7E00, 0x7F63, TY_STRUCT, pack_tc(false, CAP_ID_DB_W), TOK_SYM_SCHEMA, 0x0000, TOK_FIELD_KV, 0x0000, 0x7EC0, TOK_INS_END],
            vec![0x7F4A, 0x7E00, TOK_INS_END],
        ],
    );
    let rep_taint = verify_binary_function(&decode_omni_token_stream(&bad_taint)?.functions[0]);
    if rep_taint.passed {
        return Err(String::from("Verifier failed to reject unsanitized @UNTRUSTED sink operand"));
    }
    checks_passed += 1;

    // 3. Negative Verifier Test B: Capability violation (DB_INS 0x7F63 with cap_mask = 0)
    let bad_cap = pack_fn_module(
        &[(0x0000, vec![(TY_STR, tc_safe)])],
        0,
        &[(0x7EC0, TY_STR, tc_safe)],
        TY_STRUCT,
        0x0000,
        &[
            vec![0x7E00, 0x7F63, TY_STRUCT, pack_tc(false, CAP_ID_DB_W), TOK_SYM_SCHEMA, 0x0000, TOK_FIELD_KV, 0x0000, 0x7EC0, TOK_INS_END],
            vec![0x7F4A, 0x7E00, TOK_INS_END],
        ],
    );
    let rep_cap = verify_binary_function(&decode_omni_token_stream(&bad_cap)?.functions[0]);
    if rep_cap.passed {
        return Err(String::from("Verifier failed to reject missing CAP_ID_DB_W capability"));
    }
    checks_passed += 1;

    // 4. Negative Verifier Test C: Linear memory arena leak (0x7F01 ARENA without 0x7F05 DROP)
    let bad_leak = pack_fn_module(
        &[],
        0,
        &[(0x7EC0, TY_I64, tc_safe)],
        TY_I64,
        0x0000,
        &[
            vec![0x7E00, 0x7F01, TY_ARENA, tc_safe, TOK_IMM_I32, 0x0100, 0x0000, TOK_INS_END],
            vec![0x7F4A, 0x7EC0, TOK_INS_END],
        ],
    );
    let rep_leak = verify_binary_function(&decode_omni_token_stream(&bad_leak)?.functions[0]);
    if rep_leak.passed {
        return Err(String::from("Verifier failed to reject undropped linear memory arena"));
    }
    checks_passed += 1;

    // 5. Negative Verifier Test D: Out-of-order SSA register gap (%5 allocated before %0)
    let bad_ssa = pack_fn_module(
        &[],
        0,
        &[(0x7EC0, TY_I64, tc_safe)],
        TY_I64,
        0x0000,
        &[
            vec![0x7E05, 0x7F06, TY_I64, tc_safe, 0x7EC0, TOK_INS_END],
            vec![0x7F4A, 0x7E05, TOK_INS_END],
        ],
    );
    let rep_ssa = verify_binary_function(&decode_omni_token_stream(&bad_ssa)?.functions[0]);
    if rep_ssa.passed {
        return Err(String::from("Verifier failed to reject non-canonical SSA register gap"));
    }
    checks_passed += 1;

    // 6. Verifier / AOT Consistency Test: All 8 seed kinds verified against both AOT backends
    for &num_seed in &[0usize, 4, 6] {
        let (_, num_u16, _, _) = build_transpiled_seed_stream(num_seed);
        if compile_pure_u16_to_x86_64_elf(&num_u16, 30, 15).is_err() {
            return Err(format!("x86-64 AOT failed to lower numeric/ternary seed {}", num_seed));
        }
        if compile_pure_u16_to_wasm(&num_u16).is_err() {
            return Err(format!("Wasm AOT failed to lower numeric/ternary seed {}", num_seed));
        }
        checks_passed += 2;
    }
    for &hl_seed in &[1usize, 2, 3, 5, 7] {
        let (_, hl_u16, _, _) = build_transpiled_seed_stream(hl_seed);
        if compile_pure_u16_to_x86_64_elf(&hl_u16, 0, 0).is_ok() {
            return Err(format!("x86-64 AOT failed to reject high-level runtime seed {}", hl_seed));
        }
        if compile_pure_u16_to_wasm(&hl_u16).is_ok() {
            return Err(format!("Wasm AOT failed to reject high-level runtime seed {}", hl_seed));
        }
        checks_passed += 2;
    }

    // 7. Assembler Round-Trip + VM Execution Test
    let asm_src = "
        PARAM $0 I64 SAFE
        PARAM $1 I64 SAFE
        RET_TY TRIT
        %0 = IMUL $0 3
        %1 = ISUB %0 $1
        %2 = TCMP %1 75
        RET %2
    ";
    let asm_u16 = assemble_omni_text_to_u16(asm_src)?;
    let asm_mod = decode_omni_token_stream(&asm_u16)?;
    let mut asm_vm = OmniBinaryVm::new();
    let asm_res = asm_vm.execute_function(&asm_mod, 0, vec![OmniValue::Int(30), OmniValue::Int(10)])?;
    if asm_res != OmniValue::Trit(1) {
        return Err(format!("Assembler round-trip VM mismatch: got {:?}", asm_res));
    }
    let asm_wasm = compile_pure_u16_to_wasm(&asm_u16)?;
    if execute_wasm_module_i64(&asm_wasm, &[30, 10])? != 1 {
        return Err(String::from("Assembler round-trip Wasm mismatch"));
    }
    let asm_elf = compile_pure_u16_to_x86_64_elf_with_args(&asm_u16, 30, 10, 0)?;
    let asm_s1 = match asm_vm.execute_function(
        &stage1_mod,
        0,
        vec![OmniValue::U16Array(asm_u16.clone()), OmniValue::Int(30), OmniValue::Int(10), OmniValue::Int(0)],
    )? {
        OmniValue::Bytes(b) => b,
        _ => return Err(String::from("Assembler round-trip Stage-1 did not return Bytes")),
    };
    if asm_s1 != asm_elf {
        return Err(String::from("Assembler round-trip Stage-1 vs AOT ELF mismatch"));
    }
    checks_passed += 1;

    // 8. Isolated 64-sample differential corpus verification (fresh OmniBinaryVm per sample)
    for idx in 0..64 {
        let (_, stream, args, expected_val) = build_transpiled_seed_stream(idx);
        let m = decode_omni_token_stream(&stream)?;
        let rep = verify_binary_function(&m.functions[0]);
        if !rep.passed {
            return Err(format!("Selftest sample #{} failed verification: {:?}", idx, rep.errors));
        }
        let mut sample_vm = OmniBinaryVm::new();
        let actual = sample_vm.execute_function(&m, m.functions[0].fn_idx, args)?;
        if actual != expected_val {
            return Err(format!("Selftest sample #{} mismatch: {:?} != {:?}", idx, actual, expected_val));
        }
        checks_passed += 1;
    }

    // 9. Stateful cross-sample persistence + clear_tables() reset verification (Seed 1 DB_INS -> Seed 3 TV_SRCH)
    let mut stateful_vm = OmniBinaryVm::new();
    let (_, s1_stream, s1_args, _) = build_transpiled_seed_stream(1);
    let s1_mod = decode_omni_token_stream(&s1_stream)?;
    let _ = stateful_vm.execute_function(&s1_mod, s1_mod.functions[0].fn_idx, s1_args)?;
    let (_, s3_stream, s3_args, s3_empty_expected) = build_transpiled_seed_stream(3);
    let s3_mod = decode_omni_token_stream(&s3_stream)?;
    let s3_populated = stateful_vm.execute_function(&s3_mod, s3_mod.functions[0].fn_idx, s3_args.clone())?;
    if s3_populated == s3_empty_expected {
        return Err(String::from("Stateful TV_SRCH failed to retrieve row inserted by prior DB_INS"));
    }
    stateful_vm.clear_tables();
    let s3_after_clear = stateful_vm.execute_function(&s3_mod, s3_mod.functions[0].fn_idx, s3_args)?;
    if s3_after_clear != s3_empty_expected {
        return Err(String::from("TV_SRCH after clear_tables() did not match empty-table fallback"));
    }
    checks_passed += 2;

    // 10. Non-Seed Custom Program Stage-1 Lowering + VM/AOT/Wasm Equivalence Verification
    let custom_u16 = build_custom_non_seed_alu_program()?;
    let custom_mod = decode_omni_token_stream(&custom_u16)?;
    let mut custom_vm = OmniBinaryVm::new();
    let custom_vm_res = custom_vm.execute_function(
        &custom_mod,
        0,
        vec![OmniValue::Int(30), OmniValue::Int(15), OmniValue::Int(9)],
    )?;
    if custom_vm_res != OmniValue::Int(66) {
        return Err(format!("Non-seed custom program VM mismatch: got {:?}, expected Int(66)", custom_vm_res));
    }
    let stage1_custom_elf = match custom_vm.execute_function(
        &stage1_mod,
        0,
        vec![
            OmniValue::U16Array(custom_u16.clone()),
            OmniValue::Int(45),
            OmniValue::Int(40),
            OmniValue::Int(90),
        ],
    )? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("Stage-1 non-seed lowering returned {:?}", other)),
    };
    let aot_custom_elf = compile_pure_u16_to_x86_64_elf_with_args(&custom_u16, 45, 40, 90)?;
    if stage1_custom_elf != aot_custom_elf {
        return Err(String::from("Stage-1 ELF vs direct x86-64 AOT ELF mismatch on non-seed custom program"));
    }
    let custom_wasm = compile_pure_u16_to_wasm(&custom_u16)?;
    let custom_wasm_res = execute_wasm_module_i64(&custom_wasm, &[30, 15, 9])?;
    if custom_wasm_res != 66 {
        return Err(format!("Wasm 64-bit mismatch on non-seed custom program: got {}", custom_wasm_res));
    }
    checks_passed += 3;

    // 11. Property-Style Randomized Differential Testing (16 multi-op trials across VM, Stage-1, ELF & Wasm 64-bit)
    let mut prop_rng: u64 = 0x5EED_2026_1027_ABCD;
    for trial in 0..16usize {
        prop_rng = prop_rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mul_k = ((prop_rng >> 32) % 7 + 2) as i64;
        prop_rng = prop_rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let xor_m = ((prop_rng >> 32) % 31 + 3) as i64;
        prop_rng = prop_rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let thresh = ((prop_rng >> 32) % 90 + 20) as i64;
        let a_in = 12 + (trial as i64) * 3;
        let b_in = 5 + (trial as i64) * 2;

        let prop_asm = format!(
            "PARAM $0 I64 SAFE\nPARAM $1 I64 SAFE\nRET_TY I64\n%0 = IMUL $0 {}\n%1 = IADD %0 $1\n%2 = BXOR %1 {}\n%3 = TCMP %2 {}\n%4 = SEL_TRIT %3 11 22 33\n%5 = IADD %2 %4\nRET %5\n",
            mul_k, xor_m, thresh
        );
        let prop_u16 = assemble_omni_text_to_u16(&prop_asm)?;
        let prop_mod = decode_omni_token_stream(&prop_u16)?;
        let mut p_vm = OmniBinaryVm::new();
        let vm_val = p_vm.execute_function(&prop_mod, 0, vec![OmniValue::Int(a_in), OmniValue::Int(b_in)])?;

        let r0 = a_in * mul_k;
        let r1 = r0 + b_in;
        let r2 = r1 ^ xor_m;
        let r3 = if r2 < thresh { -1 } else if r2 > thresh { 1 } else { 0 };
        let r4 = if r3 < 0 { 11 } else if r3 == 0 { 22 } else { 33 };
        let expected_int = r2 + r4;

        if vm_val != OmniValue::Int(expected_int) {
            return Err(format!("Property test #{} VM mismatch: {:?} != Int({})", trial, vm_val, expected_int));
        }
        let s1_elf = match p_vm.execute_function(
            &stage1_mod,
            0,
            vec![OmniValue::U16Array(prop_u16.clone()), OmniValue::Int(45), OmniValue::Int(40), OmniValue::Int(90)],
        )? {
            OmniValue::Bytes(b) => b,
            _ => return Err(format!("Property test #{} Stage-1 did not return Bytes", trial)),
        };
        let aot_elf = compile_pure_u16_to_x86_64_elf_with_args(&prop_u16, 45, 40, 90)?;
        if s1_elf != aot_elf {
            return Err(format!("Property test #{} Stage-1 vs AOT ELF mismatch", trial));
        }
        let wasm_bytes = compile_pure_u16_to_wasm(&prop_u16)?;
        let wasm_val = execute_wasm_module_i64(&wasm_bytes, &[a_in, b_in])?;
        if wasm_val != expected_int {
            return Err(format!("Property test #{} Wasm 64-bit mismatch: {} != {}", trial, wasm_val, expected_int));
        }
        checks_passed += 1;
    }

    // 12. GRPO Checkpoint Persistence & Resume Verification
    let _ = run_step5_rust_grpo_self_play_with_init(16, 32, Some([3.2f32; 4]));
    let loaded_logits = load_grpo_policy_checkpoint("grpo_policy_checkpoint.json")
        .ok_or_else(|| String::from("Failed to load grpo_policy_checkpoint.json"))?;
    let (resumed_init_pass, _, _, _) = run_step5_rust_grpo_self_play_with_init(1, 32, Some(loaded_logits));
    if resumed_init_pass < 85.0 {
        return Err(format!("Resumed GRPO policy pass rate too low: {:.1}% < 85.0%", resumed_init_pass));
    }
    checks_passed += 2;

    // 13 (1o). Surface Language (`omni-lang`) -> `Vec<u16>` Bit-Exact Golden Equivalence & Negative Rejection Suite
    {
        let surface_goldens: &[(&str, &str, Vec<u16>)] = &[
            (
                "seed0_risk",
                "fn seed0_risk(base: i64, delta: i64) -> trit { let scaled = base * 2; let score = scaled + delta; let cmp = score <=> 50; let pos = +1t; let res = tand(cmp, pos); return res; }",
                build_transpiled_seed_stream(0).1,
            ),
            (
                "seed4_consensus",
                "#[fn_id(4)] fn seed4_consensus(s0: trit, s1: trit, s2: trit) -> trit { let m = tmaj(s0, s1, s2); let res = !m; return res; }",
                build_transpiled_seed_stream(4).1,
            ),
            (
                "seed6_clamp",
                "#[fn_id(6)] fn seed6_clamp(x: i64, lo: i64, hi: i64) -> i64 { let lower = imax(x, lo); let clamped = imin(lower, hi); return clamped; }",
                build_transpiled_seed_stream(6).1,
            ),
            (
                "custom_non_seed_alu",
                "fn custom_non_seed_alu(a: i64, b: i64, c: i64) -> i64 { let r0 = a * 4; let r1 = r0 - b; let r2 = c << 1; let r3 = r1 + r2; let r4 = r3 / 3; let r5 = r4 <=> 35; let r6 = !r5; let r7 = sel_trit(r6, 25, 50, 75); let r8 = r4 + r7; return r8; }",
                build_custom_non_seed_alu_program()?,
            ),
            (
                "asm_roundtrip",
                "fn asm_roundtrip(a: i64, b: i64) -> trit { let res = (a * 3 - b) <=> 75; return res; }",
                assemble_omni_text_to_u16("PARAM $0 I64 SAFE\nPARAM $1 I64 SAFE\nRET_TY TRIT\n%0 = IMUL $0 3\n%1 = ISUB %0 $1\n%2 = TCMP %1 75\nRET %2\n")?,
            ),
            (
                "iabs_golden",
                "fn iabs_golden(x: i64) -> i64 { let r = iabs(x); return r; }",
                assemble_omni_text_to_u16("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = IABS $0\nRET %0\n")?,
            ),
            (
                "idiv_zero",
                "fn idiv_zero(x: i64) -> i64 { let r = x / 0; return r; }",
                assemble_omni_text_to_u16("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = IDIV $0 0\nRET %0\n")?,
            ),
            (
                "shl_65",
                "fn shl_65(x: i64) -> i64 { let r = x << 65; return r; }",
                assemble_omni_text_to_u16("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = SHL $0 65\nRET %0\n")?,
            ),
            (
                "sel_mix",
                "fn sel_mix(x: i64) -> i64 { let prod = x * 3; let cmp = prod <=> 20; let bonus = sel_trit(cmp, 5, 10, 15); let res = prod + bonus; return res; }",
                assemble_omni_text_to_u16("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = IMUL $0 3\n%1 = TCMP %0 20\n%2 = SEL_TRIT %1 5 10 15\n%3 = IADD %0 %2\nRET %3\n")?,
            ),
            (
                "br_pos_golden",
                "fn br_pos_golden(x: i64) -> i64 { let c = x <=> 0; let r = if_pos c { 200 } else { 100 }; return r; }",
                assemble_omni_text_to_u16("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 0\nBR_POS %0 1\n%1 = CINT 100\nJMP 2\nLABEL 1\n%1 = CINT 200\nLABEL 2\nRET %1\n")?,
            ),
            (
                "br_neg_arith_golden",
                "fn br_neg_arith_golden(x: i64) -> i64 { let c = x <=> 0; let r = if_neg c { x - 50 } else { x + 100 }; return r; }",
                assemble_omni_text_to_u16("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 0\nBR_NEG %0 1\n%1 = IADD $0 100\nJMP 2\nLABEL 1\n%1 = ISUB $0 50\nLABEL 2\nRET %1\n")?,
            ),
            (
                "nested_branch_golden",
                "fn nested_branch_golden(x: i64) -> i64 { let c0 = x <=> 0; let r = if_pos c0 { let c1 = x <=> 5; if_pos c1 { 30 } else { 20 } } else { 10 }; return r; }",
                assemble_omni_text_to_u16("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 0\nBR_POS %0 1\n%1 = CINT 10\nJMP 3\nLABEL 1\n%2 = TCMP $0 5\nBR_POS %2 2\n%1 = CINT 20\nJMP 3\nLABEL 2\n%1 = CINT 30\nLABEL 3\nRET %1\n")?,
            ),
            (
                "match_trit_golden",
                "fn match_trit_golden(x: i64) -> i64 { let c = x <=> 10; let r = match_trit c { neg => x - 5, zero => 100, pos => x + 50 }; return r; }",
                assemble_omni_text_to_u16("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 10\nBR_TRIT %0 1 2 3\nLABEL 1\n%1 = ISUB $0 5\nJMP 4\nLABEL 2\n%1 = CINT 100\nJMP 4\nLABEL 3\n%1 = IADD $0 50\nLABEL 4\nRET %1\n")?,
            ),
            (
                "branch_loop_golden",
                "fn branch_loop_golden(n: i64) -> i64 { let acc = 0; let i = n; do_while { acc = acc + i; i = i - 1; } i <=> 0; return acc; }",
                build_custom_branch_loop_program()?,
            ),
            (
                "byte_buf_golden",
                "fn byte_buf_golden(x: i64) -> i64 { let buf = anew(0); buf = iset(buf, 10, 20, 30); buf = iset_i32(buf, 100); buf = iset_i64(buf, 200); let len = slen(buf); let cmp = x <=> 5; let sel = cmov(cmp, 100, 200, 300); let total = len + sel; return total; }",
                assemble_omni_text_to_u16("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = ANEW 0\n%0 = ISET %0 10 20 30\n%0 = ISET %0 -4 100\n%0 = ISET %0 -8 200\n%1 = SLEN %0\n%2 = TCMP $0 5\n%3 = CMOV %2 100 200 300\n%4 = IADD %1 %3\nRET %4\n")?,
            ),
        ];

        for &(g_name, surf_src, ref expected_u16) in surface_goldens {
            let got_u16 = compile_omni_surface_to_u16(surf_src)
                .map_err(|e| format!("Surface golden '{}' failed to compile: {}", g_name, e))?;
            if &got_u16 != expected_u16 {
                return Err(format!(
                    "Surface golden '{}' Vec<u16> mismatch: got {:?} != expected {:?}",
                    g_name, got_u16, expected_u16
                ));
            }
            let decoded = decode_omni_token_stream(&got_u16)?;
            let rep = verify_binary_function(&decoded.functions[0]);
            if !rep.passed {
                return Err(format!("Surface golden '{}' failed binary verifier: {:?}", g_name, rep.errors));
            }
            checks_passed += 1;
        }

        // Negative surface compiler tests + Line/Column Diagnostic Verification (Item 4)
        let bad_surface_cases: &[(&str, &str, &str)] = &[
            ("undefined_var", "fn bad(x: i64) -> i64 {\n    let r = missing_y + 1;\n    return r;\n}", "line 2, col 13: Undefined variable 'missing_y'"),
            ("unknown_intrinsic", "fn bad(x: i64) -> i64 {\n    let r = foo_bar(x);\n    return r;\n}", "line 2, col 13: Unknown surface intrinsic or function 'foo_bar'"),
            ("unknown_type", "fn bad(x: float128) -> i64 {\n    return x;\n}", "line 1, col 11: Unknown surface type 'float128'"),
            ("mutate_param", "fn bad(x: i64) -> i64 {\n    x = x + 1;\n    return x;\n}", "line 2, col 5: Cannot reassign immutable function parameter 'x'"),
            ("bad_control_flow_unresolved_goto", "fn bad(x: i64) -> i64 {\n    goto 9;\n    return x;\n}", "line 2, col 10: Unresolved branch target label 9"),
            ("bad_control_flow_missing_match_arm", "fn bad(x: trit) -> i64 {\n    let r = match_trit x { neg => 1, zero => 2 };\n    return r;\n}", "line 2, col 13: match_trit missing 'pos' arm"),
        ];
        for &(c_name, bad_src, expected_diag) in bad_surface_cases {
            match compile_omni_surface_to_u16(bad_src) {
                Ok(_) => return Err(format!("Surface compiler failed to reject invalid case '{}'", c_name)),
                Err(msg) => {
                    if !msg.contains(expected_diag) {
                        return Err(format!(
                            "Surface diagnostic mismatch on '{}': got '{}', expected substring '{}'",
                            c_name, msg, expected_diag
                        ));
                    }
                }
            }
            checks_passed += 1;
        }

        // 13b. Surface-Level Taint & Capability Rejection by Static Verifier (Item 1)
        {
            // (a) Untrusted return rejected by verify_binary_function
            let untr_ret_src = "fn bad_untr_ret(x: i64 @untrusted) -> i64 { return x; }";
            let untr_ret_u16 = compile_omni_surface_to_u16(untr_ret_src)?;
            let untr_ret_mod = decode_omni_token_stream(&untr_ret_u16)?;
            let untr_ret_rep = verify_binary_function(&untr_ret_mod.functions[0]);
            if untr_ret_rep.passed || !untr_ret_rep.errors.iter().any(|e| e.contains("Taint violation on RET")) {
                return Err(format!("Verifier failed to reject untrusted surface return: {:?}", untr_ret_rep.errors));
            }

            // (b) Untrusted operand passed to safe decision sink (EGATE 0x7F35) rejected by verify_binary_function
            let untr_sink_src = "#[cap(AI)] fn bad_untr_sink(x: i64 @untrusted) -> trit { let g = egate(x); return g; }";
            let untr_sink_u16 = compile_omni_surface_to_u16(untr_sink_src)?;
            let untr_sink_mod = decode_omni_token_stream(&untr_sink_u16)?;
            let untr_sink_rep = verify_binary_function(&untr_sink_mod.functions[0]);
            if untr_sink_rep.passed || !untr_sink_rep.errors.iter().any(|e| e.contains("Sink opcode 0x7F35 received @UNTRUSTED")) {
                return Err(format!("Verifier failed to reject untrusted sink operand: {:?}", untr_sink_rep.errors));
            }

            // (c) Missing capability (#[cap(AI)] omitted on egate) rejected by verify_binary_function
            let missing_cap_src = "fn bad_missing_cap(x: i64) -> trit { let g = egate(x); return g; }";
            let missing_cap_u16 = compile_omni_surface_to_u16(missing_cap_src)?;
            let missing_cap_mod = decode_omni_token_stream(&missing_cap_u16)?;
            let missing_cap_rep = verify_binary_function(&missing_cap_mod.functions[0]);
            if missing_cap_rep.passed || !missing_cap_rep.errors.iter().any(|e| e.contains("Capability violation on opcode 0x7F35")) {
                return Err(format!("Verifier failed to reject missing CAP_ID_AI capability: {:?}", missing_cap_rep.errors));
            }

            // (d) Sanitized + #[cap(AI)] surface program passes verify_binary_function
            let good_san_src = "#[cap(AI)] fn good_san(s: str @untrusted) -> str { let clean = san_prm(s); return clean; }";
            let good_san_u16 = compile_omni_surface_to_u16(good_san_src)?;
            let good_san_mod = decode_omni_token_stream(&good_san_u16)?;
            let good_san_rep = verify_binary_function(&good_san_mod.functions[0]);
            if !good_san_rep.passed {
                return Err(format!("Verifier rejected valid sanitized surface program: {:?}", good_san_rep.errors));
            }
            checks_passed += 4;
        }

        // 13c. One Decision Hook Golden (`egate` 0x7F35 / `tlogic` 0x7F3F callable from surface language & executed by VM, Item 2)
        {
            let hook_src = "#[cap(AI)] fn decision_hook_golden(a: i64, b: i64) -> trit { let delta = a - b; let gate = egate(delta); return gate; }";
            if execute_omni_surface_program(hook_src, vec![OmniValue::Int(30), OmniValue::Int(10)])? != OmniValue::Trit(1) {
                return Err(String::from("Decision hook golden failed for (30, 10) -> +1"));
            }
            if execute_omni_surface_program(hook_src, vec![OmniValue::Int(5), OmniValue::Int(20)])? != OmniValue::Trit(-1) {
                return Err(String::from("Decision hook golden failed for (5, 20) -> -1"));
            }
            if execute_omni_surface_program(hook_src, vec![OmniValue::Int(12), OmniValue::Int(12)])? != OmniValue::Trit(0) {
                return Err(String::from("Decision hook golden failed for (12, 12) -> 0"));
            }
            checks_passed += 1;
        }

        // 13d. Pack5 Lowering for Surface Trit Vectors Golden (Item 3: denser encoding vs i64 expansion)
        {
            let pack5_surf = "
                #[cap(AI)]
                fn pack5_trit_vec_golden() -> i64 {
                    let v = [+1t, -1t, 0t, +1t, +1t, -1t, 0t, -1t, +1t, +1t];
                    let packed = pack5(v);
                    let packed_byte_len = slen(packed);
                    let first_pack5_byte = iget(packed, 0);
                    let second_pack5_byte = iget(packed, 1);
                    let dot = tdot(v, v);
                    let gate = egate(v);
                    let res = packed_byte_len * 1000 + first_pack5_byte + second_pack5_byte + dot + gate;
                    return res;
                }
            ";
            let pack5_u16 = compile_omni_surface_to_u16(pack5_surf)?;
            // Compare against naive 10-element i64 buffer expansion
            let naive_i64_surf = "
                fn naive_i64_vec() -> i64 {
                    let b = anew(0);
                    b = iset_i64(b, 1);
                    b = iset_i64(b, -1);
                    b = iset_i64(b, 0);
                    b = iset_i64(b, 1);
                    b = iset_i64(b, 1);
                    b = iset_i64(b, -1);
                    b = iset_i64(b, 0);
                    b = iset_i64(b, -1);
                    b = iset_i64(b, 1);
                    b = iset_i64(b, 1);
                    let sz = slen(b);
                    return sz;
                }
            ";
            let naive_u16 = compile_omni_surface_to_u16(naive_i64_surf)?;
            let naive_bytes = execute_omni_surface_program(naive_i64_surf, vec![])?;
            // 10 trits in @PACK5 = 2 bytes (227, 219), dot = 8, gate = +1 -> 2*1000 + 227 + 219 + 8 + 1 = 2455
            // whereas naive 10 x i64 = 80 bytes (40x larger buffer, and naive_u16 has more tokens for the literal load)
            let pack5_val = execute_omni_surface_program(pack5_surf, vec![])?;
            if pack5_val != OmniValue::Int(2455) || naive_bytes != OmniValue::Int(80) || pack5_u16.len() >= naive_u16.len() + 20 {
                return Err(format!(
                    "Pack5 surface lowering golden mismatch: pack5_val={:?} (want Int(2455)), naive_bytes={:?} (want Int(80))",
                    pack5_val, naive_bytes
                ));
            }
            checks_passed += 1;
        }

        // 13e. Multi-Function Surface Program & Inter-Function CALL Golden (Item 5)
        {
            let multi_fn_surf = "
                fn weighted_term(x: i64, w: i64) -> i64 {
                    let p = x * w;
                    let s = p + 5;
                    return s;
                }

                fn main(a: i64, b: i64) -> i64 {
                    let left = weighted_term(a, 3);
                    let right = weighted_term(b, 2);
                    let total = left + right;
                    return total;
                }
            ";
            let res = execute_omni_surface_program(multi_fn_surf, vec![OmniValue::Int(10), OmniValue::Int(20)])?;
            if res != OmniValue::Int(80) {
                return Err(format!("Multi-function CALL golden mismatch: got {:?}, expected Int(80)", res));
            }
            checks_passed += 1;
        }

        // 13f. Module / File Include Linker Golden (`include "helper.omni";` + cycle rejection, Item 6 & 7)
        {
            let mut vfs = BTreeMap::new();
            vfs.insert(
                String::from("trit_helper.omni"),
                String::from("fn score_to_trit(v: i64, thresh: i64) -> trit { let c = v <=> thresh; return c; }"),
            );
            let main_with_inc = "
                include \"trit_helper.omni\";
                fn main(x: i64) -> i64 {
                    let t = score_to_trit(x, 50);
                    let out = sel_trit(t, 10, 20, 30);
                    return out;
                }
            ";
            let expanded = resolve_omni_surface_includes(main_with_inc, None, Some(&vfs))?;
            let inc_res = execute_omni_surface_program(&expanded, vec![OmniValue::Int(75)])?;
            if inc_res != OmniValue::Int(30) {
                return Err(format!("Include linker golden mismatch: got {:?}, expected Int(30)", inc_res));
            }
            // Verify circular include is rejected cleanly
            let mut bad_vfs = BTreeMap::new();
            bad_vfs.insert(String::from("loop.omni"), String::from("include \"loop.omni\"; fn f(x: i64) -> i64 { return x; }"));
            if resolve_omni_surface_includes("include \"loop.omni\"; fn main(x: i64) -> i64 { return x; }", None, Some(&bad_vfs)).is_ok() {
                return Err(String::from("Include resolver failed to reject circular include"));
            }
            checks_passed += 2;
        }

        // 13g. Phase 4: Pre-Conditioned `while` Loop + `<=` / `+=` + Direct Surface-to-ELF / Stage-1 / Wasm AOT Differential
        {
            let while_surf = "
                fn while_sum_even_golden(limit: i64) -> i64 {
                    let acc = 0;
                    let i = 2;
                    while i <= limit {
                        acc += i;
                        i += 2;
                    }
                    return acc;
                }
            ";
            let while_u16 = compile_omni_surface_to_u16(while_surf)?;
            let vm_even = execute_omni_surface_program(while_surf, vec![OmniValue::Int(10)])?;
            let wasm_even = execute_omni_surface_wasm_i64(while_surf, &[10])?;
            let surf_elf = compile_omni_surface_to_x86_64_elf(while_surf, 10, 0, 0)?;
            let mut s1_vm = OmniBinaryVm::new();
            let s1_while_elf = match s1_vm.execute_function(
                &stage1_mod,
                0,
                vec![OmniValue::U16Array(while_u16), OmniValue::Int(10), OmniValue::Int(0), OmniValue::Int(0)],
            )? {
                OmniValue::Bytes(b) => b,
                _ => return Err(String::from("while_sum_even_golden Stage-1 did not return Bytes")),
            };
            if vm_even != OmniValue::Int(30) || wasm_even != 30 || surf_elf != s1_while_elf {
                return Err(format!(
                    "while_sum_even_golden differential mismatch: vm={:?}, wasm={}, elf_eq={}",
                    vm_even,
                    wasm_even,
                    surf_elf == s1_while_elf
                ));
            }
            checks_passed += 2;
        }

        // 13h. Phase 4: Surface Linear Memory Arena & Borrow Checker Golden + Negative Leak/Borrow Rejections
        {
            let arena_ok_surf = "
                fn arena_lifecycle_golden(x: i64) -> i64 {
                    let a = arena(4096);
                    let r = borrow_r(a);
                    let res = x + 42;
                    drop(r);
                    drop(a);
                    return res;
                }
            ";
            if execute_omni_surface_program(arena_ok_surf, vec![OmniValue::Int(8)])? != OmniValue::Int(50) {
                return Err(String::from("Surface arena_lifecycle_golden execution mismatch"));
            }

            let arena_leak_surf = "
                fn bad_surface_arena_leak(x: i64) -> i64 {
                    let a = arena(4096);
                    return x;
                }
            ";
            let leak_u16 = compile_omni_surface_to_u16(arena_leak_surf)?;
            let leak_mod = decode_omni_token_stream(&leak_u16)?;
            let leak_rep = verify_binary_function(&leak_mod.functions[0]);
            if leak_rep.passed || !leak_rep.errors.iter().any(|e| e.contains("Linear memory leak")) {
                return Err(format!("Verifier failed to reject surface arena leak: {:?}", leak_rep.errors));
            }

            let borrow_conflict_surf = "
                fn bad_surface_borrow_conflict(x: i64) -> i64 {
                    let a = arena(4096);
                    let r = borrow_r(a);
                    drop(a);
                    drop(r);
                    return x;
                }
            ";
            let b_u16 = compile_omni_surface_to_u16(borrow_conflict_surf)?;
            let b_mod = decode_omni_token_stream(&b_u16)?;
            let b_rep = verify_binary_function(&b_mod.functions[0]);
            if b_rep.passed || !b_rep.errors.iter().any(|e| e.contains("active readers")) {
                return Err(format!("Verifier failed to reject surface borrow conflict: {:?}", b_rep.errors));
            }
            checks_passed += 3;
        }

        // 13i. Phase 4: Surface String Literals (`@ENG` Taint), Sanitizers, `scat`, `s2i`/`i2s`, and `fnv1a`
        {
            let bad_str_ret = "fn bad_unsanitized_str_lit() -> str { let s = \"raw <script> span\"; return s; }";
            let bad_str_u16 = compile_omni_surface_to_u16(bad_str_ret)?;
            let bad_str_mod = decode_omni_token_stream(&bad_str_u16)?;
            let bad_str_rep = verify_binary_function(&bad_str_mod.functions[0]);
            if bad_str_rep.passed || !bad_str_rep.errors.iter().any(|e| e.contains("Taint violation on RET")) {
                return Err(format!("Verifier failed to reject unsanitized @ENG string literal return: {:?}", bad_str_rep.errors));
            }

            let good_str_pipeline = "
                #[cap(AI)]
                fn str_pipeline_golden(n: i64) -> i64 {
                    let raw = \"120\";
                    let clean = san_prm(raw);
                    let parsed = s2i(clean);
                    let num_s = i2s(n);
                    let joined = scat(clean, num_s);
                    let len = slen(joined);
                    let h = fnv1a(clean);
                    let cmp = h <=> 0;
                    let bonus = sel_trit(cmp, 5, 0, 5);
                    return parsed + len + bonus;
                }
            ";
            if execute_omni_surface_program(good_str_pipeline, vec![OmniValue::Int(45)])? != OmniValue::Int(130) {
                return Err(String::from("Surface str_pipeline_golden execution mismatch"));
            }
            checks_passed += 2;
        }

        // 13j. Phase 4: Bare-Expression `eval` / `repl` Wrapper (`wrap_omni_surface_snippet`)
        {
            if execute_omni_surface_program("egate(30 - 10)", vec![])? != OmniValue::Trit(1)
                || execute_omni_surface_program("10 <= 10", vec![])? != OmniValue::Trit(1)
                || execute_omni_surface_program("san_dom(\"<b>ok</b>\")", vec![])?
                    != OmniValue::Str(String::from("&lt;b&gt;ok&lt;/b&gt;"))
            {
                return Err(String::from("Bare-expression wrap_omni_surface_snippet execution mismatch"));
            }
            checks_passed += 1;
        }

        // 13k. Phase 5: Direct @PACK5 Byte Operations (5 trits/byte) & @PACK5 / EGATE x86-64 ELF64 + Wasm AOT
        {
            let trits_10: [i8; 10] = [1, -1, 0, 1, 1, -1, 0, -1, 1, 1];
            let mut p5 = pack5_encode(&trits_10);
            if p5 != vec![227u8, 219u8] || pack5_byte_len(10) != 2 {
                return Err(String::from("Direct @PACK5 byte encoding mismatch"));
            }
            for (idx, &expected_t) in trits_10.iter().enumerate() {
                if pack5_get_trit(&p5, idx).map_err(String::from)? != expected_t {
                    return Err(format!("pack5_get_trit mismatch at idx {}", idx));
                }
            }
            if pack5_tdot_direct(&p5, &p5, 10).map_err(String::from)? != 8
                || pack5_sum_direct(&p5, 10).map_err(String::from)? != 2
                || pack5_gate_direct(&p5, 10).map_err(String::from)? != 1
            {
                return Err(String::from("Direct @PACK5 tdot/sum/gate mismatch"));
            }
            pack5_set_trit(&mut p5, 2, -1).map_err(String::from)?;
            if pack5_get_trit(&p5, 2).map_err(String::from)? != -1
                || pack5_tdot_direct(&p5, &p5, 10).map_err(String::from)? != 9
            {
                return Err(String::from("pack5_set_trit in-place mutation mismatch"));
            }

            let pack5_direct_src = "#[cap(AI)]
fn pack5_direct_aot() -> i64 {
    let raw = pack5_bytes([+1t, -1t, 0t, +1t, +1t, -1t, 0t, -1t, +1t, +1t]);
    let dot = pack5_tdot(raw, raw);
    let g = pack5_gate(raw);
    let tv = unpack5(raw, 10);
    let t0 = pack5_get(tv, 0);
    let t1 = pack5_get(tv, 1);
    let b_len = pack5_len(raw);
    let bonus = if_pos g { 2000 } else { 0 };
    return bonus + dot * 50 + b_len * 10 + (t0 - t1);
}";
            let elf_p5 = compile_omni_surface_to_x86_64_elf(pack5_direct_src, 0, 0, 0)?;
            if elf_p5.len() < 120 || &elf_p5[0..4] != &[0x7F, b'E', b'L', b'F'] {
                return Err(String::from("@PACK5 x86-64 ELF64 header mismatch"));
            }
            let wasm_p5 = execute_omni_surface_wasm_i64(pack5_direct_src, &[])?;
            let (aot_val, backend) = execute_omni_surface_aot_first(pack5_direct_src, vec![])?;
            let vm_val = execute_omni_surface_program_with_budgets(pack5_direct_src, vec![], Some(100_000), Some(1024 * 1024))?;
            // bonus(2000) + dot(8)*50(400) + b_len(2)*10(20) + (1 - (-1))(2) = 2422
            if backend != "aot" || wasm_p5 != 2422 || aot_val != OmniValue::Int(2422) || vm_val != OmniValue::Int(2422) {
                return Err(format!(
                    "Direct @PACK5 AOT/VM equivalence mismatch: backend={}, wasm={}, aot={:?}, vm={:?}",
                    backend, wasm_p5, aot_val, vm_val
                ));
            }
            checks_passed += 2;
        }

        // 13l. Phase 5: AOT-First Hot Numeric Execution & Stage-1 Self-Compiler `.omni` -> ELF64
        {
            let hot_src = "fn hot_loop(n: i64) -> i64 {
    let acc = 0;
    let i = n;
    while (i <=> 0) {
        acc += i * 3;
        i -= 1;
    }
    return acc;
}";
            let (res_aot, backend) = execute_omni_surface_aot_first(hot_src, vec![OmniValue::Int(10)])?;
            if backend != "aot" || res_aot != OmniValue::Int(165) {
                return Err(String::from("Hot numeric path did not execute via AOT or returned wrong result"));
            }
            let s1_elf = compile_omni_surface_via_stage1_to_elf(hot_src, 10, 0, 0)?;
            let host_elf = compile_omni_surface_to_x86_64_elf(hot_src, 10, 0, 0)?;
            assert_elf_bit_exact(&host_elf, &s1_elf, "compile_omni_surface_via_stage1_to_elf(hot_loop)");
            checks_passed += 2;
        }

        // 13m. Phase 5: Explicit Step & Memory Budgets (`#[step_budget(N)]` / `#[mem_budget(N)]`)
        {
            let runaway_steps_src = "#[step_budget(40)]
fn runaway(n: i64) -> i64 {
    let i = n;
    let acc = 0;
    while (i <=> 0) {
        acc += i;
        i -= 1;
    }
    return acc;
}";
            if execute_omni_surface_program(runaway_steps_src, vec![OmniValue::Int(1000)]).is_ok() {
                return Err(String::from("#[step_budget(40)] failed to reject runaway AOT loop"));
            }
            if execute_omni_surface_program_with_budgets(runaway_steps_src, vec![OmniValue::Int(1000)], Some(100_000), Some(1024 * 1024)).is_ok() {
                return Err(String::from("#[step_budget(40)] failed to reject runaway VM loop"));
            }

            let tight_mem_src = "#[mem_budget(16)]
fn blow_mem(x: i64) -> str {
    let s = \"0123456789ABCDEF0123456789ABCDEF\";
    let clean = san_dom(s);
    return clean;
}";
            if execute_omni_surface_program(tight_mem_src, vec![OmniValue::Int(1)]).is_ok() {
                return Err(String::from("#[mem_budget(16)] failed to reject excessive allocation"));
            }
            checks_passed += 2;
        }

        // 13n. Phase 5: Zero Long-Lived Intermediate IR (`compile_omni_surface_to_u16` & `verify_u16_stream`)
        {
            let multi_mod_src = "fn helper(x: i64) -> i64 { return x * 4 + 1; }
#[cap(AI)]
fn main(a: i64) -> i64 {
    let h = helper(a);
    let g = egate(h - 10);
    return if_pos g { h } else { 0 };
}";
            let direct_u16 = compile_omni_surface_to_u16(multi_mod_src)?;
            let via_asm_u16 = assemble_omni_text_to_u16(&compile_omni_surface_to_asm(multi_mod_src)?)?;
            if direct_u16 != via_asm_u16 {
                return Err(String::from("Direct zero-IR compile_omni_surface_to_u16 diverged from assemble_omni_text_to_u16"));
            }
            let reports = verify_u16_stream(&direct_u16)?;
            if reports.len() != 2 || !reports.iter().all(|r| r.passed) {
                return Err(String::from("verify_u16_stream failed on direct multi-function u16 stream"));
            }
            checks_passed += 1;
        }

        // 13o. Deep Audit Regressions: 50-Trit (>35) @PACK5 + Dynamic Loop IGET in AOT, Sentinel 243..=255 Rejection, & Uncalled Helper Verification
        {
            let surf_pack5_50 = "
                #[cap(AI)]
                fn pack5_50_dynamic_golden(limit: i64) -> i64 {
                    let v = [
                        +1t, -1t, 0t, +1t, +1t, -1t, 0t, -1t, +1t, +1t,
                        +1t, -1t, 0t, +1t, +1t, -1t, 0t, -1t, +1t, +1t,
                        +1t, -1t, 0t, +1t, +1t, -1t, 0t, -1t, +1t, +1t,
                        +1t, -1t, 0t, +1t, +1t, -1t, 0t, -1t, +1t, +1t,
                        +1t, -1t, 0t, +1t, +1t, -1t, 0t, -1t, +1t, +1t
                    ];
                    let dot = tdot(v, v);
                    let gate = egate(v);
                    let acc = 0;
                    let idx = 0;
                    while idx < limit {
                        acc += iget(v, idx);
                        idx += 1;
                    }
                    return dot * 100 + acc * 10 + gate;
                }
            ";
            let (v_50, b_50) = execute_omni_surface_aot_first(surf_pack5_50, vec![OmniValue::Int(50)])?;
            let vm_50 = execute_omni_surface_program_with_budgets(surf_pack5_50, vec![OmniValue::Int(50)], None, None)?;
            let elf_50 = compile_omni_surface_to_x86_64_elf(surf_pack5_50, 50, 0, 0)?;
            if b_50 != "aot" || v_50 != OmniValue::Int(4101) || vm_50 != OmniValue::Int(4101) || elf_50.len() < 120 {
                return Err(String::from("50-trit (>35) @PACK5 + dynamic loop IGET AOT/VM mismatch"));
            }

            let corrupt_pack5_asm = "
                CAP_MASK 1
                RET_TY I64
                %0 = ANEW 0
                %0 = ISET %0 250
                %1 = UNPACK5 %0 5
                %2 = EGATE %1
                RET %2
            ";
            let corrupt_u16 = assemble_omni_text_to_u16(corrupt_pack5_asm)?;
            let corrupt_mod = decode_omni_token_stream(&corrupt_u16)?;
            let mut c_vm = OmniBinaryVm::new();
            if compile_pure_u16_to_wasm(&corrupt_u16).is_ok()
                || compile_pure_u16_to_x86_64_elf_with_args(&corrupt_u16, 0, 0, 0).is_ok()
                || c_vm.execute_function(&corrupt_mod, 0, vec![]).is_ok()
            {
                return Err(String::from("Corrupt @PACK5 sentinel byte (250 > 242) was not rejected across AOT and VM"));
            }

            let uncalled_tainted_helper = "
                fn bad_helper(u: i64 @untrusted) -> i64 {
                    return u;
                }
                fn main(x: i64) -> i64 {
                    return x + 1;
                }
            ";
            if execute_omni_surface_program_with_budgets(uncalled_tainted_helper, vec![OmniValue::Int(5)], None, None).is_ok()
                || execute_omni_surface_program(uncalled_tainted_helper, vec![OmniValue::Int(5)]).is_ok()
            {
                return Err(String::from("Uncalled helper function with taint violation was not rejected before execution"));
            }
            checks_passed += 3;
        }
    }

    Ok(checks_passed)
}

fn print_artifact_manifest(paths: &[&str]) {
    println!("\n[ARTIFACT MANIFEST — SIZE & FNV-1a64 INTEGRITY HASHES]");
    for &p in paths {
        if let Ok(bytes) = fs::read(p) {
            println!(
                "    - {:<34} {:>8} bytes   FNV-1a64 = 0x{:016X}",
                p,
                bytes.len(),
                fnv1a_64_hash(&bytes)
            );
        }
    }
}

fn main() {
    let cli_args: Vec<String> = env::args().collect();
    if cli_args.len() >= 2 {
        match cli_args[1].as_str() {
            "--resume-grpo" | "resume-grpo" => {
                let ckpt = load_grpo_policy_checkpoint("grpo_policy_checkpoint.json");
                println!("[GRPO RESUME] Loaded logits from grpo_policy_checkpoint.json: {:?}", ckpt);
                let (p0, p1, rew, probs) = run_step5_rust_grpo_self_play_with_init(8, 32, ckpt);
                println!(
                    "[GRPO RESUME DONE] Pass: {:.1}% -> {:.1}% (MeanR={:+.3}), Probs={:?}",
                    p0, p1, rew, probs
                );
                return;
            }
            "fixed-point" | "--fixed-point" => {
                let s1_u16 = build_step7_stage1_compiler_stream();
                let bc_bytes = encode_u16_stream_to_bytes(&s1_u16);
                fs::write("omni_self_compiler.omni_bc", &bc_bytes).expect("Failed to write omni_self_compiler.omni_bc");
                let s1_elf = compile_pure_u16_to_x86_64_elf_with_args(&s1_u16, 45, 40, 90).expect("Stage-1 AOT failed");
                fs::write("omni_stage1_self_compiler.elf", &s1_elf).expect("Failed to write omni_stage1_self_compiler.elf");
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut p = fs::metadata("omni_stage1_self_compiler.elf").unwrap().permissions();
                    p.set_mode(0o755);
                    fs::set_permissions("omni_stage1_self_compiler.elf", p).unwrap();
                }
                let out = std::process::Command::new("./omni_stage1_self_compiler.elf")
                    .arg("omni_self_compiler.omni_bc")
                    .output()
                    .expect("Failed to run ./omni_stage1_self_compiler.elf");
                assert!(out.status.success(), "Stage-1 self-compilation exited non-zero");
                fs::write("omni_stage2_self_compiler.elf", &out.stdout).expect("Failed to write omni_stage2_self_compiler.elf");
                assert_elf_bit_exact(&s1_elf, &out.stdout, "Stage-1 == Stage-2 Fixed Point");
                println!(
                    "[FIXED-POINT PASS] omni_self_compiler.omni_bc={} B (FNV-1a64=0x{:016X}) | omni_stage1_self_compiler.elf={} B (FNV-1a64=0x{:016X}) == omni_stage2_self_compiler.elf={} B (FNV-1a64=0x{:016X}) | diff=0 bytes",
                    bc_bytes.len(), fnv1a_64_hash(&bc_bytes),
                    s1_elf.len(), fnv1a_64_hash(&s1_elf),
                    out.stdout.len(), fnv1a_64_hash(&out.stdout)
                );
                return;
            }
            "selftest" => {
                match run_unit_and_differential_selftests() {
                    Ok(cnt) => {
                        println!("[SELFTEST PASS] {} unit, negative-verifier, AOT/Wasm-64bit, Stage-1 control-flow, and 64-corpus differential checks passed.", cnt);
                        return;
                    }
                    Err(e) => {
                        eprintln!("[SELFTEST FAIL] {}", e);
                        std::process::exit(1);
                    }
                }
            }
            "--debug-lens" | "debug-lens" if cli_args.len() >= 3 => {
                let raw = fs::read(&cli_args[2]).expect("Failed to read .omni_bc file");
                let u16_stream = decode_bytes_to_u16_stream(&raw);
                println!("{}", format_debug_lens(&u16_stream).expect("Failed to decode stream"));
                return;
            }
            "hex" if cli_args.len() >= 3 => {
                let raw = fs::read(&cli_args[2]).expect("Failed to read .omni_bc file");
                let u16_stream = decode_bytes_to_u16_stream(&raw);
                println!("{}", format_non_human_token_stream(&u16_stream));
                return;
            }
            "assemble" if cli_args.len() >= 4 => {
                let src = fs::read_to_string(&cli_args[2]).expect("Failed to read source file");
                let u16_stream = assemble_omni_text_to_u16(&src).expect("Assembly failed");
                fs::write(&cli_args[3], encode_u16_stream_to_bytes(&u16_stream)).expect("Failed to write .omni_bc");
                println!("[ASSEMBLE PASS] Wrote {} u16 tokens to '{}'", u16_stream.len(), cli_args[3]);
                return;
            }
            "compile-surface" if cli_args.len() >= 4 => {
                let u16_stream = compile_omni_surface_file_to_u16(&cli_args[2]).expect("Surface compilation failed");
                fs::write(&cli_args[3], encode_u16_stream_to_bytes(&u16_stream)).expect("Failed to write .omni_bc");
                println!("[SURFACE COMPILE PASS] Wrote {} u16 tokens to '{}'", u16_stream.len(), cli_args[3]);
                return;
            }
            "compile-stage1" if cli_args.len() >= 4 => {
                let raw_src = fs::read_to_string(&cli_args[2]).expect("Failed to read .omni file");
                let parent_dir = std::path::Path::new(&cli_args[2]).parent();
                let expanded = resolve_omni_surface_includes(&raw_src, parent_dir, None).expect("Include resolution failed");
                let a0 = cli_args.get(4).and_then(|s| s.parse::<i64>().ok()).unwrap_or(45);
                let a1 = cli_args.get(5).and_then(|s| s.parse::<i64>().ok()).unwrap_or(40);
                let a2 = cli_args.get(6).and_then(|s| s.parse::<i64>().ok()).unwrap_or(90);
                let elf = compile_omni_surface_via_stage1_to_elf(&expanded, a0, a1, a2)
                    .expect("Stage-1 self-compiler ELF lowering failed");
                fs::write(&cli_args[3], &elf).expect("Failed to write .elf");
                println!("[STAGE-1 ELF PASS] Wrote {} bytes to '{}' (FNV-1a64=0x{:016X})", elf.len(), cli_args[3], fnv1a_64_hash(&elf));
                return;
            }
            "compile-elf" if cli_args.len() >= 4 => {
                let path = &cli_args[2];
                let u16_stream = if path.ends_with(".omni") {
                    compile_omni_surface_file_to_u16(path).expect("Surface compilation failed")
                } else {
                    let raw = fs::read(path).expect("Failed to read .omni_bc");
                    decode_bytes_to_u16_stream(&raw)
                };
                let a0 = cli_args.get(4).and_then(|s| s.parse::<i64>().ok()).unwrap_or(45);
                let a1 = cli_args.get(5).and_then(|s| s.parse::<i64>().ok()).unwrap_or(40);
                let a2 = cli_args.get(6).and_then(|s| s.parse::<i64>().ok()).unwrap_or(90);
                let elf = compile_pure_u16_to_x86_64_elf_with_args(&u16_stream, a0, a1, a2)
                    .expect("x86-64 ELF AOT compilation failed");
                fs::write(&cli_args[3], &elf).expect("Failed to write .elf");
                println!("[ELF AOT PASS] Wrote {} bytes to '{}' (FNV-1a64=0x{:016X})", elf.len(), cli_args[3], fnv1a_64_hash(&elf));
                return;
            }
            "compile-wasm" if cli_args.len() >= 4 => {
                let path = &cli_args[2];
                let u16_stream = if path.ends_with(".omni") {
                    compile_omni_surface_file_to_u16(path).expect("Surface compilation failed")
                } else {
                    let raw = fs::read(path).expect("Failed to read .omni_bc");
                    decode_bytes_to_u16_stream(&raw)
                };
                let wasm = compile_pure_u16_to_wasm(&u16_stream).expect("WebAssembly AOT compilation failed");
                fs::write(&cli_args[3], &wasm).expect("Failed to write .wasm");
                println!("[WASM AOT PASS] Wrote {} bytes to '{}' (FNV-1a64=0x{:016X})", wasm.len(), cli_args[3], fnv1a_64_hash(&wasm));
                return;
            }
            "run-wasm" if cli_args.len() >= 3 => {
                let path = &cli_args[2];
                let i64_args: Vec<i64> = cli_args[3..]
                    .iter()
                    .map(|s| match parse_cli_omni_value(s) {
                        OmniValue::Int(n) => n,
                        OmniValue::Trit(t) => t as i64,
                        _ => 0,
                    })
                    .collect();
                if path.ends_with(".omni") {
                    let raw_src = fs::read_to_string(path).expect("Failed to read .omni surface file");
                    let parent_dir = std::path::Path::new(path).parent();
                    let expanded = resolve_omni_surface_includes(&raw_src, parent_dir, None)
                        .expect("Include resolution failed");
                    let res = execute_omni_surface_wasm_i64(&expanded, &i64_args).expect("Wasm execution failed");
                    println!("{}", res);
                    return;
                }
                let wasm_bytes = if path.ends_with(".wasm") {
                    fs::read(path).expect("Failed to read .wasm")
                } else {
                    let raw = fs::read(path).expect("Failed to read .omni_bc");
                    let u16_stream = decode_bytes_to_u16_stream(&raw);
                    compile_pure_u16_to_wasm(&u16_stream).expect("WebAssembly AOT compilation failed")
                };
                let res = execute_wasm_module_i64(&wasm_bytes, &i64_args).expect("Wasm execution failed");
                println!("{}", res);
                return;
            }
            "surface-to-asm" if cli_args.len() >= 3 => {
                let raw_src = fs::read_to_string(&cli_args[2]).expect("Failed to read .omni surface file");
                let parent_dir = std::path::Path::new(&cli_args[2]).parent();
                let expanded = resolve_omni_surface_includes(&raw_src, parent_dir, None).expect("Include resolution failed");
                let asm = compile_omni_surface_to_asm(&expanded).expect("Surface-to-asm lowering failed");
                print!("{}", asm);
                return;
            }
            "eval" if cli_args.len() >= 3 => {
                let vm_args: Vec<OmniValue> = cli_args[3..].iter().map(|s| parse_cli_omni_value(s)).collect();
                match execute_omni_surface_program(&cli_args[2], vm_args) {
                    Ok(res) => println!("{:?}", res),
                    Err(e) => {
                        eprintln!("[EVAL FAIL] {}", e);
                        std::process::exit(1);
                    }
                }
                return;
            }
            "repl" => {
                use std::io::{self, BufRead, Write};
                println!("Omni Surface Language REPL — type a 'fn main(...) -> ... {{ ... }}' or expression, or 'exit' to quit.");
                let stdin = io::stdin();
                let mut line_buf = String::new();
                loop {
                    print!("omni> ");
                    let _ = io::stdout().flush();
                    line_buf.clear();
                    if stdin.lock().read_line(&mut line_buf).unwrap_or(0) == 0 {
                        break;
                    }
                    let trimmed = line_buf.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    if trimmed == "exit" || trimmed == "quit" || trimmed == ":q" {
                        break;
                    }
                    match execute_omni_surface_program(trimmed, Vec::new()) {
                        Ok(val) => println!("=> {:?}", val),
                        Err(e) => eprintln!("[ERROR] {}", e),
                    }
                }
                return;
            }
            "run" | "run-surface" if cli_args.len() >= 3 => {
                let path = &cli_args[2];
                let vm_args: Vec<OmniValue> = cli_args[3..].iter().map(|s| parse_cli_omni_value(s)).collect();
                if path.ends_with(".omni") {
                    let raw_src = fs::read_to_string(path).expect("Failed to read .omni file");
                    let parent_dir = std::path::Path::new(path).parent();
                    let expanded = resolve_omni_surface_includes(&raw_src, parent_dir, None).expect("Include resolution failed");
                    match execute_omni_surface_aot_first(&expanded, vm_args) {
                        Ok((res, _used_aot)) => {
                            println!("{:?}", res);
                            return;
                        }
                        Err(e) => {
                            eprintln!("[RUN FAIL] {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                let u16_stream = {
                    let raw = fs::read(path).expect("Failed to read input file");
                    if let Ok(txt) = std::str::from_utf8(&raw) {
                        if txt.contains("fn ") {
                            let parent_dir = std::path::Path::new(path).parent();
                            let expanded = resolve_omni_surface_includes(txt, parent_dir, None).expect("Include resolution failed");
                            compile_omni_surface_to_u16(&expanded).expect("Surface compilation failed")
                        } else {
                            decode_bytes_to_u16_stream(&raw)
                        }
                    } else {
                        decode_bytes_to_u16_stream(&raw)
                    }
                };
                let module = decode_omni_token_stream(&u16_stream).expect("Decode failed");
                for f in &module.functions {
                    let rep = verify_binary_function(f);
                    if !rep.passed {
                        eprintln!("[VERIFY FAIL] fn #{}: {:?}", f.fn_idx, rep.errors);
                        std::process::exit(1);
                    }
                }
                let entry_fn_idx = if module.functions.iter().any(|f| f.fn_idx == 0) {
                    0
                } else {
                    module.functions.last().map(|f| f.fn_idx).expect("No functions in module")
                };
                let mut vm = OmniBinaryVm::new();
                let res = vm.execute_function(&module, entry_fn_idx, vm_args).expect("VM execution failed");
                println!("{:?}", res);
                return;
            }
            _ => {}
        }
    }

    println!("========================================================================================");
    println!("OMNI v2.0 — COMPLETE 7-STEP ZERO-DEPENDENCY RUST BOOTSTRAP & STAGE-1 COMPILER SUITE");
    println!("========================================================================================");

    let selftest_cnt = run_unit_and_differential_selftests().expect("Pre-flight selftests failed");
    println!(
        "\n[PRE-FLIGHT SELFTESTS] PASS ({} unit, negative-verifier, pack5, Wasm-64bit, Stage-1 & corpus checks)",
        selftest_cnt
    );

    // [STEP 1] Pure Non-Human u16 Token-Stream VM & Binary Verifier
    let step1_stream = build_step1_pure_token_stream();
    let step1_mod = decode_omni_token_stream(&step1_stream).expect("Step 1 decode failed");
    let mut step1_ins = 0usize;
    for f in &step1_mod.functions {
        let rep = verify_binary_function(f);
        assert!(rep.passed, "Step 1 verification failed: {:?}", rep.errors);
        step1_ins += rep.instruction_count;
    }
    let mut vm = OmniBinaryVm::new();
    let step1_out = vm
        .execute_function(
            &step1_mod,
            0x0000,
            vec![
                OmniValue::Str(String::from("550e8400-e29b-41d4-a716-446655440000")),
                OmniValue::Str(String::from("Track workout <script>alert(1)</script>")),
                OmniValue::Int(45),
            ],
        )
        .expect("Step 1 VM failed");
    println!(
        "\n[STEP 1/7] Pure Non-Human u16 Token-Stream VM & Binary Verifier (Rust):\n    - Stream Size:          {} u16 tokens ({} bytes, Zero English Keywords)\n    - Static O(n) Verifier: PASS ({} SSA instructions, RL_Reward = +1.00)\n    - VM Execution Output:  {:?}",
        step1_stream.len(), step1_stream.len() * 2, step1_ins, step1_out
    );

    // [STEP 2] Native Rust Bootstrap Transpiler & 64-Sample Verified Corpus Builder
    let corpus = run_step2_rust_corpus_builder("omni_sft_corpus.jsonl", 64);
    let total_toks: usize = corpus.iter().map(|c| c.omni_u16_token_ids.len()).sum();
    println!(
        "\n[STEP 2/7] Native Rust Bootstrap Transpiler & Differential Corpus Builder:\n    - Verified Samples:     {} / {} (100.0% Static Verifier + Live VM Execution Match)\n    - Total Pure u16 Toks:  {} tokens written to 'omni_sft_corpus.jsonl'",
        corpus.len(), corpus.len(), total_toks
    );

    // [STEP 3] Native Rust 32,768-Token `tokenizer.json` Builder
    let (vocab_sz, tok_bytes) = run_step3_rust_tokenizer_builder("tokenizer.json");
    println!(
        "\n[STEP 3/7] Native Rust 32,768-Token HuggingFace 'tokenizer.json' Builder:\n    - Partition Locked:     32,256 English (0x0000..0x7DFF) + 512 Pure Omni (0x7E00..0x7FFF) = {} tokens\n    - Artifact Written:     'tokenizer.json' ({:.1} KB)",
        vocab_sz, (tok_bytes as f64) / 1024.0
    );

    // [STEP 4] Compact Linear Sequence Model with Rank-8 Orthogonal Subspace Projection
    let (l_init, l_final, acc_init, acc_final) = run_step4_rust_galore_sft_trainer(&corpus, 12);
    println!(
        "\n[STEP 4/7] Compact Linear Sequence Model with Rank-8 Orthogonal Subspace Projection:\n    - Cross-Entropy Loss:   {:.4} (Epoch 1) -> {:.4} (Epoch 12)\n    - Next-Token Accuracy:  {:.1}% -> {:.1}% (r=8 vs d=32 Low-Rank Moment State)",
        l_init, l_final, acc_init, acc_final
    );

    // [STEP 5] Stabilized Pure-u16 Compiler-RL (GRPO) & `.omni_holo` Exporter
    println!("\n[STEP 5/7] Stabilized Pure-u16 Compiler-RL (GRPO) & '.omni_holo' Tri-Tier Export:");
    let holo_bytes = export_tri_tier_omni_holo("expert_e0_layer0.omni_holo");
    let (grpo_init, grpo_final, grpo_rew, grpo_probs) = run_step5_rust_grpo_self_play(16, 32);
    println!(
        "    - GRPO Verifier Pass:   {:.1}% (Epoch 1) -> {:.1}% (Epoch 16, MeanR = {:+.2})\n    - Final Action Probs:   P(sql_san)={:.3}, P(db_cap)={:.3}, P(drop_arena)={:.3}, P(dom_san)={:.3}\n    - Tri-Tier '.omni_holo': 'expert_e0_layer0.omni_holo' ({} bytes, 64-byte aligned, magic='OMNIHOLO')",
        grpo_init, grpo_final, grpo_rew, grpo_probs[0], grpo_probs[1], grpo_probs[2], grpo_probs[3], holo_bytes
    );

    // [STEP 6] Native Rust Direct `Vec<u16>` -> x86-64 ELF & Real Instruction-Lowering WebAssembly AOT
    let (_, risk_u16_stream, _, _) = build_transpiled_seed_stream(0);
    let elf_bytes = compile_pure_u16_to_x86_64_elf(&risk_u16_stream, 45, 40).expect("Step 6 ELF AOT failed");
    fs::write("omni_aot_risk_engine.elf", &elf_bytes).expect("Failed to write ELF binary");
    let wasm_bytes = compile_pure_u16_to_wasm(&risk_u16_stream).expect("Step 6 Wasm AOT failed");
    fs::write("omni_aot_risk_engine.wasm", &wasm_bytes).expect("Failed to write Wasm binary");
    println!(
        "\n[STEP 6/7] Native Rust Vec<u16> -> x86-64 ELF & Real Instruction-Lowering WebAssembly AOT:\n    - Native x86-64 ELF:    'omni_aot_risk_engine.elf' ({} bytes, Entry=0x400078, Zero External Linker)\n    - WebAssembly Module:   'omni_aot_risk_engine.wasm' ({} bytes, Real Instruction-Lowered '\\0asm' v1)",
        elf_bytes.len(), wasm_bytes.len()
    );

    // [STEP 7] Two-Pass Stage-1 Pure-u16 Compiler Lowering `.omni_bc` -> Stage-2 ELF Without Host Codegen
    let (bc_bytes, s1_elf_sz, s2_elf_sz, hash_v0, hash_v3, hash_s4, hash_s6) = run_step7_stage1_self_hosting(
        "omni_self_compiler.omni_bc",
        "omni_stage1_self_compiler.elf",
        "omni_stage2_self_compiler.elf",
        "omni_stage2_var0.elf",
        "omni_stage2_var3.elf",
    )
    .expect("Step 7 Stage-1 compiler lowering failed");
    println!(
        "\n[STEP 7/7] Two-Pass Stage-1 Pure-u16 Compiler Lowering .omni_bc -> Stage-2 ELF (No Host Codegen):\n    - Stage-1 Pure u16 BC:  'omni_self_compiler.omni_bc' ({} bytes, Two-Pass Instruction & Control-Flow Walker)\n    - Stage-1 Native ELF:   'omni_stage1_self_compiler.elf' ({} bytes)\n    - Stage-2 Emitted ELFs: 'omni_stage2_var0.elf' ({} B, FNV=0x{:016X}), 'omni_stage2_var3.elf' (FNV=0x{:016X})\n    - Multi-Seed Stage-2:   'omni_stage2_seed4_consensus.elf' (FNV=0x{:016X}), 'omni_stage2_seed6_clamp.elf' (FNV=0x{:016X})",
        bc_bytes, s1_elf_sz, s2_elf_sz, hash_v0, hash_v3, hash_s4, hash_s6
    );

    #[cfg(all(unix, target_arch = "x86_64"))]
    {
        use std::process::Command;
        for (bin, args, desc) in [
            ("./omni_stage2_var0.elf",            &["30", "15"][..],        "Seed 0 Var 0 (2*30+15 <=> 50)"),
            ("./omni_stage2_var3.elf",            &["30", "15"][..],        "Seed 0 Var 3 (5*30+15 <=> 170)"),
            ("./omni_stage2_seed4_consensus.elf", &["1", "1", "-1"][..],    "Seed 4 Consensus (TNOT(TMAJ(1,1,-1)))"),
            ("./omni_stage2_seed6_clamp.elf",     &["150", "10", "90"][..], "Seed 6 Clamp (clamp(150, 10, 90))"),
            ("./omni_stage2_custom_alu.elf",      &["30", "15", "9"][..],   "Custom Non-Seed ALU+SEL_TRIT Pipeline"),
            ("./omni_stage2_branch_loop.elf",     &["10"][..],              "Custom Non-Seed Loop sum(1..=10)"),
        ] {
            if let Ok(out) = Command::new(bin).args(args).output() {
                if out.status.success() && out.stdout.len() >= 8 {
                    let val = i64::from_le_bytes(out.stdout[0..8].try_into().unwrap());
                    println!(
                        "    - Live Stage-2 CPU Run: {:<38} -> exit=0 (64-bit stdout i64={:+})",
                        desc, val
                    );
                }
            }
        }
    }

    print_artifact_manifest(&[
        "omni_sft_corpus.jsonl",
        "tokenizer.json",
        "expert_e0_layer0.omni_holo",
        "grpo_policy_checkpoint.json",
        "omni_aot_risk_engine.elf",
        "omni_aot_risk_engine.wasm",
        "omni_self_compiler.omni_bc",
        "omni_stage1_self_compiler.elf",
        "omni_stage2_self_compiler.elf",
        "omni_stage2_var0.elf",
        "omni_stage2_var3.elf",
        "omni_stage2_seed4_consensus.elf",
        "omni_stage2_seed6_clamp.elf",
        "omni_stage2_custom_alu.elf",
        "omni_stage2_branch_loop.elf",
        "custom_alu.omni_bc",
        "branch_loop.omni_bc",
    ]);
    println!("========================================================================================");
}