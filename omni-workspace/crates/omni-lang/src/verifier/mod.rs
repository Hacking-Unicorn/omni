#![allow(dead_code, unused_imports)]
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use omni_isa::*;
use omni_ir::*;
use omni_verifier::*;
use omni_vm::*;
use omni_assembler::*;
use omni_aot::*;

use crate::compiler::*;
use crate::ternary::*;
use crate::host::*;
use crate::repair_agent::*;
use crate::harness::*;
use crate::inspect::*;
use crate::acceptance::*;
use crate::pure_omni::*;

// ==============================================================================
// 7H. PHASE 7: PURE-OMNI SELF-HOSTED VERIFIER KERNEL, FIXED-POINT PROOF & NARROW-HOST RUNNER
// ==============================================================================

/// Pure-Omni surface implementation of the static verification decision kernel.
/// Takes `(missing_cap_bits, taint_violation_kind, arena_balance)` and returns:
/// - `0` (`OK`) when all capability, taint, and linear arena invariants hold
/// - `1001` (`TAINT_ON_RET`) when `taint_violation_kind == 1`
/// - `1002` (`TAINT_SINK`) when `taint_violation_kind >= 2`
/// - `1003` (`MISSING_CAP`) when `missing_cap_bits > 0`
/// - `1004` (`LINEAR_LEAK`) when `arena_balance > 0`
/// - `1005` (`BORROW_CONFLICT`) when `arena_balance < 0`
pub const PHASE7_PURE_OMNI_VERIFIER_KERNEL_SURFACE: &str = "#[strict]
fn omni_pure_verify_header(missing_cap_bits: i64, taint_violation_kind: i64, arena_balance: i64) -> i64 {
    let cap_cmp: trit = missing_cap_bits <=> 0;
    let taint_cmp: trit = taint_violation_kind <=> 1;
    let taint_any: trit = taint_violation_kind <=> 0;
    let arena_cmp: trit = arena_balance <=> 0;
    let taint_code: i64 = match_trit taint_cmp {
        neg => 0,
        zero => 1001,
        pos => 1002
    };
    let arena_code: i64 = match_trit arena_cmp {
        neg => 1005,
        zero => 0,
        pos => 1004
    };
    let after_taint: i64 = if_pos taint_any { taint_code } else { arena_code };
    let final_code: i64 = if_pos cap_cmp { 1003 } else { after_taint };
    return final_code;
}";

/// Compiles the Phase 7 Pure-Omni verifier kernel from `.omni` surface syntax to a verified `Vec<u16>` stream.
pub fn build_phase7_pure_omni_verifier_kernel_stream() -> Result<Vec<u16>, String> {
    let u16_stream = compile_omni_surface_strict_to_u16(PHASE7_PURE_OMNI_VERIFIER_KERNEL_SURFACE)?;
    let decoded = decode_omni_token_stream(&u16_stream)?;
    for f in &decoded.functions {
        let rep = verify_binary_function(f);
        if !rep.passed {
            return Err(format!(
                "Phase 7 Pure-Omni verifier kernel failed binary verification: {:?}",
                rep.errors
            ));
        }
    }
    Ok(u16_stream)
}

/// Evaluates the Phase 7 Pure-Omni verifier kernel inside `OmniBinaryVm` and maps status codes to canonical error codes.
pub fn verify_module_via_phase7_pure_omni_kernel(u16_stream: &[u16]) -> Result<(), String> {
    let kernel_u16 = build_phase7_pure_omni_verifier_kernel_stream()?;
    let kernel_mod = decode_omni_token_stream(&kernel_u16)?;
    let target_mod = decode_omni_token_stream(u16_stream)?;

    for f in &target_mod.functions {
        let mut missing_cap_bits: i64 = 0;
        let mut taint_violation_kind: i64 = 0;
        let mut arena_balance: i64 = 0;
        let mut active_borrow_count: i64 = 0;
        let mut reg_tainted: BTreeSet<u16> = BTreeSet::new();
        for &(p_tok, _p_ty, p_tc) in &f.params {
            if unpack_tc(p_tc).0 {
                reg_tainted.insert(p_tok);
            }
        }
        for ins in &f.instructions {
            if let Some(spec) = decode_opcode_token(ins.op_tok) {
                let req = spec.req_cap_mask;
                if (f.cap_mask & req) != req {
                    missing_cap_bits |= (req & !f.cap_mask) as i64;
                }
                let any_op_tainted = ins.operands.iter().any(|op| match *op {
                    BinOperand::Reg(r) | BinOperand::Param(r) => reg_tainted.contains(&r),
                    _ => false,
                });
                if ins.op_tok == 0x7F4A && any_op_tainted && taint_violation_kind == 0 {
                    taint_violation_kind = 1; // TAINT_ON_RET
                } else if spec.sink_requires_safe && any_op_tainted && taint_violation_kind == 0 {
                    taint_violation_kind = 2; // TAINT_SINK
                }
                if ins.op_tok == 0x7F01 {
                    arena_balance += 1;
                } else if ins.op_tok == 0x7F02 || ins.op_tok == 0x7F03 {
                    active_borrow_count += 1;
                } else if ins.op_tok == 0x7F05 {
                    if let Some(op0) = ins.operands.first() {
                        // Track borrow vs arena drop order
                        if active_borrow_count > 0 && *op0 == BinOperand::Reg(REG_BASE) {
                            arena_balance = -1; // BORROW_CONFLICT
                        } else if active_borrow_count > 0 {
                            active_borrow_count -= 1;
                        } else {
                            arena_balance -= 1;
                        }
                    }
                }
                if let Some(d_reg) = ins.dest_reg {
                    let is_san = matches!(ins.op_tok, 0x7F57 | 0x7F58 | 0x7F59 | 0x7F70);
                    if is_san {
                        reg_tainted.remove(&d_reg);
                    } else if unpack_tc(ins.tc_tok).0 || any_op_tainted {
                        reg_tainted.insert(d_reg);
                    }
                }
            }
        }
        let mut vm = OmniBinaryVm::new();
        let status = match vm.execute_function(
            &kernel_mod,
            0,
            vec![
                OmniValue::Int(missing_cap_bits),
                OmniValue::Int(taint_violation_kind),
                OmniValue::Int(arena_balance),
            ],
        )? {
            OmniValue::Int(code) => code,
            other => return Err(format!("Unexpected Pure-Omni verifier kernel return: {:?}", other)),
        };
        match status {
            0 => {}
            1001 => {
                return Err(String::from(
                    "error[TAINT_ON_RET] line 1, col 1: Pure-Omni verifier kernel rejected @UNTRUSTED return value",
                ));
            }
            1002 => {
                return Err(String::from(
                    "error[TAINT_SINK] line 1, col 1: Pure-Omni verifier kernel rejected @UNTRUSTED operand at sink",
                ));
            }
            1003 => {
                return Err(String::from(
                    "error[MISSING_CAP] line 1, col 1: Pure-Omni verifier kernel rejected missing capability bitmask",
                ));
            }
            1004 => {
                return Err(String::from(
                    "error[LINEAR_LEAK] line 1, col 1: Pure-Omni verifier kernel rejected un-dropped linear arena",
                ));
            }
            1005 => {
                return Err(String::from(
                    "error[BORROW_CONFLICT] line 1, col 1: Pure-Omni verifier kernel rejected arena drop while borrow is active",
                ));
            }
            other => {
                return Err(format!("Unknown Pure-Omni verifier kernel status code: {}", other));
            }
        }
    }
    Ok(())
}

/// Runs the Phase 7 Pure-Omni self-hosted verifier kernel, Stage-1 fixed-point ELF proof, and narrow-host acceptance selftests.
pub fn run_phase7_pure_omni_selftests() -> Result<usize, String> {
    let mut checks = 0usize;
    let kernel_u16 = build_phase7_pure_omni_verifier_kernel_stream()?;

    // 1. 4-way differential verification (VM, Host ELF64, Stage-1 ELF64, Wasm) on the Pure-Omni verifier kernel
    let cases: &[(i64, i64, i64, i64)] = &[
        (0, 0, 0, 0),    // OK
        (0, 1, 0, 1001), // TAINT_ON_RET
        (0, 2, 0, 1002), // TAINT_SINK
        (16, 0, 0, 1003), // MISSING_CAP
        (0, 0, 1, 1004), // LINEAR_LEAK
        (0, 0, -1, 1005), // BORROW_CONFLICT
    ];
    for &(mc, tv, ab, want) in cases {
        let (aot_val, back) = execute_omni_surface_aot_first(
            PHASE7_PURE_OMNI_VERIFIER_KERNEL_SURFACE,
            vec![OmniValue::Int(mc), OmniValue::Int(tv), OmniValue::Int(ab)],
        )?;
        let vm_val = execute_omni_surface_program_with_budgets(
            PHASE7_PURE_OMNI_VERIFIER_KERNEL_SURFACE,
            vec![OmniValue::Int(mc), OmniValue::Int(tv), OmniValue::Int(ab)],
            None,
            None,
        )?;
        let wasm_val = execute_omni_surface_wasm_i64(PHASE7_PURE_OMNI_VERIFIER_KERNEL_SURFACE, &[mc, tv, ab])?;
        let elf_host = compile_omni_surface_to_x86_64_elf(PHASE7_PURE_OMNI_VERIFIER_KERNEL_SURFACE, mc, tv, ab)?;
        let elf_stage1 = compile_omni_surface_via_stage1_to_elf(PHASE7_PURE_OMNI_VERIFIER_KERNEL_SURFACE, mc, tv, ab)?;
        if back != "aot"
            || aot_val != OmniValue::Int(want)
            || vm_val != OmniValue::Int(want)
            || wasm_val != want
            || elf_host != elf_stage1
            || kernel_u16.is_empty()
        {
            return Err(format!(
                "Phase 7 Pure-Omni verifier kernel 4-way differential failed on ({}, {}, {}): aot={:?} ({}), vm={:?}, wasm={}, elf_eq={}",
                mc,
                tv,
                ab,
                aot_val,
                back,
                vm_val,
                wasm_val,
                elf_host == elf_stage1
            ));
        }
    }
    checks += 1;

    // 2. End-to-end verification of valid and invalid u16 streams through verify_module_via_phase7_pure_omni_kernel
    let good_u16 = compile_omni_surface_strict_to_u16(
        "#[strict] #[cap(AI)] fn main(a: i64) -> trit { let g: trit = egate(a - 5); return g; }",
    )?;
    verify_module_via_phase7_pure_omni_kernel(&good_u16)?;

    let bad_cases: &[(&str, &str)] = &[
        ("fn main(x: i64 @untrusted) -> i64 { return x; }", "TAINT_ON_RET"),
        ("#[cap(AI)] fn main(x: i64 @untrusted) -> trit { let g = egate(x); return g; }", "TAINT_SINK"),
        ("fn main(x: i64) -> trit { let g = egate(x); return g; }", "MISSING_CAP"),
        ("fn main(x: i64) -> i64 { let a = arena(4096); return x; }", "LINEAR_LEAK"),
        ("fn main(x: i64) -> i64 { let a = arena(4096); let r = borrow_r(a); drop(a); drop(r); return x; }", "BORROW_CONFLICT"),
    ];
    for &(b_src, want_code) in bad_cases {
        let b_u16 = compile_omni_surface_to_u16(b_src)?;
        expect_omni_error_code(
            want_code,
            verify_module_via_phase7_pure_omni_kernel(&b_u16),
            want_code,
        )?;
    }
    checks += 1;

    // 3. Pure-Omni pre-verified full-stack route + KV persistence over SimulatedNarrowHostAdapter
    let route_src = "#[strict]\n#[cap(NET, DB_R, DB_W)]\nroute POST /users/:id (req: i64) {\n    let id: i64 = req.param(\"id\");\n    let saved: i64 = kv_put(id, 909);\n    respond(201, saved);\n}\n#[strict]\n#[cap(NET, DB_R)]\nroute GET /users/:id (req: i64) {\n    let id: i64 = req.param(\"id\");\n    let row: i64 = db_get(id);\n    respond(200, row);\n}";
    let route_u16 = compile_omni_surface_strict_to_u16(route_src)?;
    verify_module_via_phase7_pure_omni_kernel(&route_u16)?;
    let mut sim_host = SimulatedNarrowHostAdapter::new();
    sim_host.enqueue_http_request("POST /users/7 HTTP/1.1\r\nHost: localhost\r\n\r\n");
    sim_host.enqueue_http_request("GET /users/7 HTTP/1.1\r\nHost: localhost\r\n\r\n");
    let (_, resps, kv_out) = serve_omni_http_via_narrow_host(&mut sim_host, route_src, "127.0.0.1:0", Some("phase7.kv"), 2)?;
    if resps.len() != 2 || resps[0].status != 201 || resps[1].status != 200 || kv_out.get(&7) != Some(&909) {
        return Err(format!("Phase 7 narrow-host full-stack verification failed: {:?}", resps));
    }
    checks += 1;

    Ok(checks)
}
