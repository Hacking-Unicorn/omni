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
pub mod isa_table;

pub use compiler::*;
pub use ternary::*;
pub use verifier::*;
pub use host::*;
pub use repair_agent::*;
pub use harness::*;
pub use inspect::*;
pub use acceptance::*;
pub use pure_omni::*;
pub use isa_table::*;

#[cfg(test)]
mod wasm_aot_budget_parity_tests {
    use super::*;
    use omni_aot::{
        compile_pure_u16_to_wasm, compile_pure_u16_to_x86_64_elf_multi, execute_wasm_module_i64,
    };
    use omni_vm::OmniValue;

    #[test]
    fn workspace_fix2_fix3_and_x86_entry_cap_parity() {
        let ok_src = "#[strict]\n#[mem_budget(8192)]\nfn main(cur: i64, step: i64) -> i64 { let a: arena = arena(4096); drop(a); return cur + step; }\n";
        let got = execute_omni_surface_aot_first(ok_src, vec![OmniValue::Int(5), OmniValue::Int(1)])
            .expect("workspace #[mem_budget(8192)] arena(4096) should succeed on Wasm AOT");
        assert_eq!(got, (OmniValue::Int(6), "aot"));

        let neg_src = "#[strict]\n#[mem_budget(8192)]\nfn main(cur: i64, step: i64) -> i64 { let a: arena = arena(20000); drop(a); return cur + step; }\n";
        let err = execute_omni_surface_aot_first(neg_src, vec![OmniValue::Int(5), OmniValue::Int(1)])
            .expect_err("workspace #[mem_budget(8192)] arena(20000) must fail");
        assert!(err.contains("BUDGET_EXCEEDED"), "unexpected error: {err}");

        let mf5_src = "#[strict]\nfn helper_first(x: i64) -> i64 { return x * 100; }\n#[strict]\nfn entry_last(a: i64, b: i64, c: i64, d: i64, e: i64) -> i64 { return helper_first(d) + e + a * 0 + b * 0 + c * 0; }\n";
        let stream = compile_omni_surface_strict_to_u16(mf5_src).expect("compile mf5_src");
        let wasm = compile_pure_u16_to_wasm(&stream).expect("multi wasm dispatch");
        assert_eq!(execute_wasm_module_i64(&wasm, &[1, 2, 3, 4, 5]).unwrap(), 405);
        let x86_elf = compile_pure_u16_to_x86_64_elf_multi(&stream, 0, 0, 0)
            .expect("x86-64 multi-AOT supports up to 32 entry params");
        assert!(x86_elf.starts_with(b"\x7FELF"));
    }
}
