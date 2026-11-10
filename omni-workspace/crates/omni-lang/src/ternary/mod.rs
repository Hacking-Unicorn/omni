//! @PACK5 balanced-ternary codec boundary.
//!
//! The zero-allocation `@PACK5` (5 trits/byte, radix-3, valid lane `0..=242`)
//! codec already lives in `omni-isa`. This module re-exports that codec as the
//! port boundary for a future Pure-Omni implementation. Surface-language
//! lowering of trit-vector literals (`pack5`, `pack5_bytes`, `unpack5`, `tdot`)
//! remains in `crate::compiler` because those helpers are methods on the
//! Stage-0 AST walker and cannot be moved without a behavior-risk refactor.

pub use omni_isa::{
    pack5_byte_len, pack5_decode, pack5_encode, pack5_gate_direct, pack5_get_trit,
    pack5_set_trit, pack5_sum_direct, pack5_tdot_direct, PACK5_MAX_VALID,
    SENTINEL_SPARSE_RUN_ZERO, SENTINEL_SPECIAL_TOKEN,
};
