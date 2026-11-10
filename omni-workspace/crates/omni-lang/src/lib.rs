#![allow(dead_code, unused_imports)]
//! Omni surface language, host, repair agent, acceptance suite, and Pure-Omni wrappers.
//!
//! The public API is re-exported from the crate root so existing `omni_lang::`
//! callers stay unchanged. Submodules are the port boundaries for replacing
//! each Rust unit with a Pure-Omni twin.

pub mod compiler;
pub mod ternary;
pub mod verifier;
pub mod host;
pub mod repair_agent;
pub mod harness;
pub mod inspect;
pub mod acceptance;
pub mod pure_omni;

pub use compiler::*;
pub use ternary::*;
pub use verifier::*;
pub use host::*;
pub use repair_agent::*;
pub use harness::*;
pub use inspect::*;
pub use acceptance::*;
pub use pure_omni::*;
