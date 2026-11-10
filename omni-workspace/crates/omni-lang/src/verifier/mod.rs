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


// ==============================================================================
// PURE-OMNI VERIFIER ENGINE (real token-stream verifier, not a scalar kernel)
// ==============================================================================

pub const OMNI_VERIFIER_ENGINE_VERSION: &str = "omni-verifier-engine-v1.0.0";
pub const OMNI_VERIFIER_ENGINE_SURFACE: &str =
    include_str!("../../../../../pure/verifier_engine.omni");

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OmniEngineError {
    pub code: i64,
    pub name: &'static str,
    pub offset: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OmniEngineFnReport {
    pub fn_idx: u16,
    pub passed: bool,
    pub errors: Vec<OmniEngineError>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OmniEngineModuleReport {
    pub decode_error: Option<(i64, usize)>,
    pub functions: Vec<OmniEngineFnReport>,
}

pub fn verifier_engine_error_name(code: i64) -> &'static str {
    match code {
        1001 => "TAINT_ON_RET",
        1002 => "TAINT_SINK",
        1003 => "MISSING_CAP",
        1004 => "LINEAR_LEAK",
        1005 => "BORROW_CONFLICT",
        2009 => "UNRESOLVED_LABEL",
        3001 => "UNKNOWN_OPCODE",
        3003 => "SSA_NONCANONICAL",
        3004 => "ARENA_OVERWRITE",
        3005 => "REG_REASSIGN",
        3006 => "DECODE_ERROR",
        other => {
            let _ = other;
            "UNKNOWN"
        }
    }
}

pub fn classify_rust_verifier_error(err: &str) -> (i64, usize) {
    let mut offset = 0usize;
    if let Some(idx) = err.find("Offset ") {
        let digits: String = err[idx + 7..].chars().take_while(|c| c.is_ascii_digit()).collect();
        offset = digits.parse().unwrap_or(0);
    } else if let Some(idx) = err.find("at offset ") {
        let digits: String = err[idx + 10..].chars().take_while(|c| c.is_ascii_digit()).collect();
        offset = digits.parse().unwrap_or(0);
    } else if let Some(idx) = err.find("instruction at ") {
        let digits: String = err[idx + 15..].chars().take_while(|c| c.is_ascii_digit()).collect();
        offset = digits.parse().unwrap_or(0);
    }
    let code = if let Some(s) = err.find("error[") {
        if let Some(e) = err[s + 6..].find(']') {
            match &err[s + 6..s + 6 + e] {
                "TAINT_ON_RET" => 1001,
                "TAINT_SINK" => 1002,
                "MISSING_CAP" => 1003,
                "LINEAR_LEAK" => 1004,
                "BORROW_CONFLICT" => 1005,
                "UNRESOLVED_LABEL" => 2009,
                _ => 3006,
            }
        } else {
            3006
        }
    } else if err.contains("Unknown opcode") {
        3001
    } else if err.contains("Non-canonical SSA") {
        3003
    } else if err.contains("Cannot overwrite active heap arena") {
        3004
    } else if err.contains("Cannot reassign register") {
        3005
    } else {
        3006
    };
    (code, offset)
}

fn parse_engine_result_bytes(buf: &[u8]) -> Result<OmniEngineModuleReport, String> {
    if buf.len() < 24 || buf.len() % 8 != 0 {
        return Err(format!(
            "Omni verifier engine returned a truncated result ({} bytes)",
            buf.len()
        ));
    }
    let words: Vec<i64> = buf
        .chunks_exact(8)
        .map(|c| i64::from_le_bytes(c.try_into().unwrap()))
        .collect();
    let status = words[0];
    let decode_off = words[1] as usize;
    let n_fns = words[2];
    if n_fns < 0 {
        return Err(format!("Omni verifier engine returned negative fn count {n_fns}"));
    }
    let mut i = 3usize;
    let mut functions = Vec::with_capacity(n_fns as usize);
    for _ in 0..n_fns {
        if i + 3 > words.len() {
            return Err(String::from("Omni verifier engine result truncated in function header"));
        }
        let fn_idx = words[i] as u16;
        let passed = words[i + 1] == 1;
        let n_err = words[i + 2];
        i += 3;
        if n_err < 0 {
            return Err(format!("Omni verifier engine returned negative error count {n_err}"));
        }
        let mut errors = Vec::with_capacity(n_err as usize);
        for _ in 0..n_err {
            if i + 2 > words.len() {
                return Err(String::from("Omni verifier engine result truncated in error list"));
            }
            let code = words[i];
            let offset = words[i + 1] as usize;
            errors.push(OmniEngineError {
                code,
                name: verifier_engine_error_name(code),
                offset,
            });
            i += 2;
        }
        functions.push(OmniEngineFnReport {
            fn_idx,
            passed: passed && errors.is_empty(),
            errors,
        });
    }
    let decode_error = if status == 3006 {
        Some((3006, decode_off))
    } else {
        None
    };
    Ok(OmniEngineModuleReport {
        decode_error,
        functions,
    })
}

pub fn rust_verify_as_engine_report(stream: &[u16]) -> OmniEngineModuleReport {
    match decode_omni_token_stream(stream) {
        Err(e) => {
            let (code, offset) = classify_rust_verifier_error(&e);
            OmniEngineModuleReport {
                decode_error: Some((code, offset)),
                functions: Vec::new(),
            }
        }
        Ok(module) => {
            let functions = module
                .functions
                .iter()
                .map(|f| {
                    let rep = verify_binary_function(f);
                    let errors = rep
                        .errors
                        .iter()
                        .map(|e| {
                            let (code, offset) = classify_rust_verifier_error(e);
                            OmniEngineError {
                                code,
                                name: verifier_engine_error_name(code),
                                offset,
                            }
                        })
                        .collect::<Vec<_>>();
                    OmniEngineFnReport {
                        fn_idx: f.fn_idx,
                        passed: rep.passed && errors.is_empty(),
                        errors,
                    }
                })
                .collect();
            OmniEngineModuleReport {
                decode_error: None,
                functions,
            }
        }
    }
}

fn omni_verifier_engine_module() -> Result<omni_ir::BinModule, String> {
    with_lockstep_suspended(|| {
        let u16_stream = compile_omni_surface_strict_to_u16(OMNI_VERIFIER_ENGINE_SURFACE)?;
        let decoded = decode_omni_token_stream(&u16_stream)?;
        for f in &decoded.functions {
            let rep = verify_binary_function(f);
            if !rep.passed {
                return Err(format!(
                    "Omni verifier engine failed Rust binary verification (fn #{}): {:?}",
                    f.fn_idx, rep.errors
                ));
            }
        }
        Ok(decoded)
    })
}

pub fn verify_u16_stream_via_omni_engine(stream: &[u16]) -> Result<OmniEngineModuleReport, String> {
    use std::sync::OnceLock;
    static ENGINE: OnceLock<Result<omni_ir::BinModule, String>> = OnceLock::new();
    let engine = match ENGINE.get_or_init(omni_verifier_engine_module) {
        Ok(m) => m,
        Err(e) => return Err(e.clone()),
    };
    let entry = if engine.functions.iter().any(|f| f.fn_idx == 0) {
        0
    } else {
        engine
            .functions
            .last()
            .map(|f| f.fn_idx)
            .ok_or_else(|| String::from("Omni verifier engine has no functions"))?
    };
    let mut vm = OmniBinaryVm::new();
    let bytes = encode_u16_stream_to_bytes(stream);
    let val = vm.execute_function(engine, entry, vec![OmniValue::Bytes(bytes)])?;
    match val {
        OmniValue::Bytes(buf) => parse_engine_result_bytes(&buf),
        other => Err(format!(
            "Omni verifier engine returned a non-bytes value: {:?}",
            other
        )),
    }
}

pub fn compare_rust_and_omni_verifier(label: &str, stream: &[u16]) -> Result<(), String> {
    let rust = rust_verify_as_engine_report(stream);
    let omni = verify_u16_stream_via_omni_engine(stream)?;
    if rust.decode_error.is_some() != omni.decode_error.is_some() {
        return Err(format!(
            "verifier-diff '{label}': decode-error mismatch rust={:?} omni={:?}",
            rust.decode_error, omni.decode_error
        ));
    }
    if let (Some((rc, ro)), Some((oc, oo))) = (rust.decode_error, omni.decode_error) {
        if rc != oc {
            return Err(format!(
                "verifier-diff '{label}': decode error code rust={rc} omni={oc} offsets rust={ro} omni={oo}"
            ));
        }
        // Offsets should match when the Rust decoder reports one.
        if ro != 0 && ro != oo {
            let rust_raw = match decode_omni_token_stream(stream) {
                Err(e) => e,
                Ok(_) => String::from("<rust decode unexpectedly succeeded>"),
            };
            return Err(format!(
                "verifier-diff '{label}': decode error span rust={ro} omni={oo} (code {rc}); rust_msg={rust_raw}; stream={:04X?}",
                stream
            ));
        }
        return Ok(());
    }
    if rust.functions.len() != omni.functions.len() {
        return Err(format!(
            "verifier-diff '{label}': function count rust={} omni={}",
            rust.functions.len(),
            omni.functions.len()
        ));
    }
    for (rf, of) in rust.functions.iter().zip(omni.functions.iter()) {
        if rf.fn_idx != of.fn_idx || rf.passed != of.passed {
            return Err(format!(
                "verifier-diff '{label}': fn #{} verdict rust passed={} errors={:?} vs omni passed={} errors={:?}",
                rf.fn_idx, rf.passed, rf.errors, of.passed, of.errors
            ));
        }
        if rf.errors.len() != of.errors.len() {
            return Err(format!(
                "verifier-diff '{label}': fn #{} error-count rust={} {:?} vs omni={} {:?}",
                rf.fn_idx,
                rf.errors.len(),
                rf.errors,
                of.errors.len(),
                of.errors
            ));
        }
        for (re, oe) in rf.errors.iter().zip(of.errors.iter()) {
            if re.code != oe.code || re.offset != oe.offset {
                return Err(format!(
                    "verifier-diff '{label}': fn #{} error rust=({} {}, off {}) vs omni=({} {}, off {})",
                    rf.fn_idx, re.code, re.name, re.offset, oe.code, oe.name, oe.offset
                ));
            }
        }
    }
    Ok(())
}


// ==============================================================================
// LOCKSTEP SHADOW MODE
// Every compile / run / serve invocation runs the Rust verifier and the Omni
// verifier engine and hard-errors on any verdict, error-code, or span mismatch.
// The engine's own compile is excluded (reentrancy / OnceLock init).
// ==============================================================================

use std::cell::Cell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

static LOCKSTEP_COMPILE_COUNT: AtomicU64 = AtomicU64::new(0);
static LOCKSTEP_RUN_COUNT: AtomicU64 = AtomicU64::new(0);
static LOCKSTEP_SERVE_COUNT: AtomicU64 = AtomicU64::new(0);
static LOCKSTEP_LAST_RUST_NS: AtomicU64 = AtomicU64::new(0);
static LOCKSTEP_LAST_SHADOW_NS: AtomicU64 = AtomicU64::new(0);

thread_local! {
    static LOCKSTEP_SUSPENDED: Cell<bool> = Cell::new(false);
    static LOCKSTEP_INJECT_PATH: Cell<Option<&'static str>> = Cell::new(None);
}

fn lockstep_counter(path: &str) -> &'static AtomicU64 {
    match path {
        "compile" => &LOCKSTEP_COMPILE_COUNT,
        "run" => &LOCKSTEP_RUN_COUNT,
        "serve" => &LOCKSTEP_SERVE_COUNT,
        _ => &LOCKSTEP_RUN_COUNT,
    }
}

pub fn reset_lockstep_counters() {
    LOCKSTEP_COMPILE_COUNT.store(0, Ordering::SeqCst);
    LOCKSTEP_RUN_COUNT.store(0, Ordering::SeqCst);
    LOCKSTEP_SERVE_COUNT.store(0, Ordering::SeqCst);
}

pub fn lockstep_invocation_count(path: &str) -> u64 {
    lockstep_counter(path).load(Ordering::SeqCst)
}

pub fn set_lockstep_inject_path(path: Option<&'static str>) {
    LOCKSTEP_INJECT_PATH.with(|c| c.set(path));
}

/// `(rust_only_ns, rust_plus_omni_ns)` averaged in the lockstep selftest, or last call.
pub fn last_lockstep_overhead_ns() -> (u64, u64) {
    (
        LOCKSTEP_LAST_RUST_NS.load(Ordering::SeqCst),
        LOCKSTEP_LAST_SHADOW_NS.load(Ordering::SeqCst),
    )
}

fn with_lockstep_suspended<T>(f: impl FnOnce() -> T) -> T {
    LOCKSTEP_SUSPENDED.with(|s| {
        let prev = s.get();
        s.set(true);
        let out = f();
        s.set(prev);
        out
    })
}

fn format_lockstep_mismatch(path: &str, rust: &OmniEngineModuleReport, omni: &OmniEngineModuleReport) -> String {
    format!(
        "error[LOCKSTEP_MISMATCH] line 1, col 1: path={path}: Rust and Omni verifiers disagree rust_decode={:?} omni_decode={:?} rust_fns={:?} omni_fns={:?}",
        rust.decode_error, omni.decode_error, rust.functions, omni.functions
    )
}

fn reports_agree(rust: &OmniEngineModuleReport, omni: &OmniEngineModuleReport) -> bool {
    if rust.decode_error.is_some() != omni.decode_error.is_some() {
        return false;
    }
    if let (Some((rc, ro)), Some((oc, oo))) = (rust.decode_error, omni.decode_error) {
        return rc == oc && (ro == 0 || ro == oo);
    }
    if rust.functions.len() != omni.functions.len() {
        return false;
    }
    rust.functions.iter().zip(omni.functions.iter()).all(|(rf, of)| {
        rf.fn_idx == of.fn_idx
            && rf.passed == of.passed
            && rf.errors.len() == of.errors.len()
            && rf.errors.iter().zip(of.errors.iter()).all(|(re, oe)| re.code == oe.code && re.offset == oe.offset)
    })
}

fn inject_disagreement(report: &OmniEngineModuleReport) -> OmniEngineModuleReport {
    let mut out = report.clone();
    if let Some((code, off)) = out.decode_error {
        out.decode_error = Some((if code == 3006 { 3001 } else { 3006 }, off));
        return out;
    }
    if let Some(f) = out.functions.first_mut() {
        f.passed = !f.passed;
        if f.errors.is_empty() {
            f.errors.push(OmniEngineError {
                code: 9999,
                name: "UNKNOWN",
                offset: 0,
            });
        } else {
            f.errors.clear();
        }
        return out;
    }
    out.decode_error = Some((3006, 0));
    out
}

/// Run Rust and Omni verifiers in lockstep. Hard-error on any disagreement.
/// Skipped while the Omni engine module itself is being compiled.
pub fn lockstep_shadow_verify(path: &str, stream: &[u16]) -> Result<(), String> {
    if LOCKSTEP_SUSPENDED.with(|s| s.get()) {
        return Ok(());
    }
    let t_rust = Instant::now();
    let rust = rust_verify_as_engine_report(stream);
    let rust_ns = t_rust.elapsed().as_nanos() as u64;
    let t_omni = Instant::now();
    let mut omni = verify_u16_stream_via_omni_engine(stream)?;
    let omni_ns = t_omni.elapsed().as_nanos() as u64;
    let shadow_ns = rust_ns.saturating_add(omni_ns);
    LOCKSTEP_LAST_RUST_NS.store(rust_ns, Ordering::SeqCst);
    LOCKSTEP_LAST_SHADOW_NS.store(shadow_ns, Ordering::SeqCst);

    let inject = LOCKSTEP_INJECT_PATH.with(|c| c.get()) == Some(path);
    if inject {
        omni = inject_disagreement(&omni);
    }
    lockstep_counter(path).fetch_add(1, Ordering::SeqCst);
    if !reports_agree(&rust, &omni) {
        return Err(format_lockstep_mismatch(path, &rust, &omni));
    }
    Ok(())
}

/// `verify_u16_stream` after a lockstep agreement check (used by run and serve).
pub fn verify_u16_stream_lockstep(path: &str, stream: &[u16]) -> Result<Vec<BinVerificationReport>, String> {
    lockstep_shadow_verify(path, stream)?;
    verify_u16_stream(stream)
}

pub fn run_lockstep_shadow_selftests() -> Result<usize, String> {
    let mut checks = 0usize;
    let ok_src = "#[strict] fn main(a: i64) -> i64 { let b: i64 = a + 1; return b; }";
    let route_src = "#[strict]\n#[cap(NET)]\nroute GET /ping/:x (req: i64) {\n    let x: i64 = req.param(\"x\");\n    respond(200, x + 1);\n}";

    // Warm the engine so overhead numbers are steady-state, not first-compile.
    let _ = compile_omni_surface_to_u16(ok_src)?;

    reset_lockstep_counters();
    set_lockstep_inject_path(None);
    let _stream = compile_omni_surface_to_u16(ok_src)
        .map_err(|e| format!("lockstep compile path failed: {e}"))?;
    if lockstep_invocation_count("compile") == 0 {
        return Err(String::from("lockstep was not invoked on the compile path"));
    }
    checks += 1;

    reset_lockstep_counters();
    let got = execute_omni_surface_program(ok_src, vec![OmniValue::Int(41)])
        .map_err(|e| format!("lockstep run path failed: {e}"))?;
    if got != OmniValue::Int(42) {
        return Err(format!("lockstep run returned {got:?}, want Int(42)"));
    }
    if lockstep_invocation_count("run") == 0 {
        return Err(String::from("lockstep was not invoked on the run path"));
    }
    checks += 1;

    reset_lockstep_counters();
    let mut kv = BTreeMap::new();
    let resp = handle_omni_http_request(route_src, "GET", "/ping/5", &mut kv)
        .map_err(|e| format!("lockstep serve path failed: {e}"))?;
    if resp.status != 200 {
        return Err(format!("lockstep serve returned status {}, want 200 (body={})", resp.status, resp.body));
    }
    if lockstep_invocation_count("serve") == 0 {
        return Err(String::from("lockstep was not invoked on the serve path"));
    }
    checks += 1;

    for path in ["compile", "run", "serve"] {
        set_lockstep_inject_path(Some(path));
        let err = match path {
            "compile" => compile_omni_surface_to_u16(ok_src).err(),
            "run" => execute_omni_surface_program(ok_src, vec![OmniValue::Int(1)]).err(),
            _ => handle_omni_http_request(route_src, "GET", "/ping/1", &mut BTreeMap::new()).err(),
        };
        set_lockstep_inject_path(None);
        let err = err.ok_or_else(|| format!("lockstep inject on {path} did not fail"))?;
        if !err.contains("error[LOCKSTEP_MISMATCH]") || !err.contains(&format!("path={path}")) {
            return Err(format!("lockstep inject on {path} produced the wrong diagnostic: {err}"));
        }
        checks += 1;
    }

    set_lockstep_inject_path(None);
    let stream = compile_omni_surface_to_u16(ok_src)?;
    let warmup = 5usize;
    let iters = 25usize;
    for _ in 0..warmup {
        let _ = rust_verify_as_engine_report(&stream);
        lockstep_shadow_verify("compile", &stream)?;
    }
    let t0 = Instant::now();
    for _ in 0..iters {
        let _ = rust_verify_as_engine_report(&stream);
    }
    let rust_ns = t0.elapsed().as_nanos() as u64 / iters as u64;
    let t1 = Instant::now();
    for _ in 0..iters {
        lockstep_shadow_verify("compile", &stream)?;
    }
    let shadow_ns = t1.elapsed().as_nanos() as u64 / iters as u64;
    LOCKSTEP_LAST_RUST_NS.store(rust_ns, Ordering::SeqCst);
    LOCKSTEP_LAST_SHADOW_NS.store(shadow_ns, Ordering::SeqCst);
    if shadow_ns == 0 {
        return Err(String::from("lockstep overhead measurement produced a zero shadow time"));
    }
    checks += 1;
    Ok(checks)
}

/// Differential suite: every existing verifier corpus / negative case, plus fuzzed streams.
/// Returns the number of compared cases on success.
pub fn run_verifier_engine_diff_selftests() -> Result<usize, String> {
    let mut cases: Vec<(String, Vec<u16>)> = Vec::new();

    let surfaces: &[(&str, &str)] = &[
        ("ok_add", "#[strict] fn main(a: i64) -> i64 { let b: i64 = a + 1; return b; }"),
        ("ok_egate", "#[strict] #[cap(AI)] fn main(a: i64) -> trit { let g: trit = egate(a - 5); return g; }"),
        ("taint_on_ret", "fn main(x: i64 @untrusted) -> i64 { return x; }"),
        ("taint_sink", "#[cap(AI)] fn main(x: i64 @untrusted) -> trit { let g = egate(x); return g; }"),
        ("missing_cap", "fn main(x: i64) -> trit { let g = egate(x); return g; }"),
        ("linear_leak", "fn main(x: i64) -> i64 { let a = arena(4096); return x; }"),
        ("borrow_conflict", "fn main(x: i64) -> i64 { let a = arena(4096); let r = borrow_r(a); drop(a); drop(r); return x; }"),
        ("ok_drop_arena", "fn main(x: i64) -> i64 { let a = arena(4096); drop(a); return x; }"),
        ("sanitized_sink", "#[cap(AI)] fn main(x: i64 @untrusted) -> trit { let s = san_prm(x); let g = egate(s); return g; }"),
        ("taint_upgrade", "fn main(x: i64 @untrusted) -> i64 { let y: i64 = x + 1; return y; }"),
        ("ok_two_fn", "fn helper(a: i64) -> i64 { let b: i64 = a * 2; return b; }\nfn main(x: i64) -> i64 { let y: i64 = helper(x); return y; }"),
        ("branch_ok", "fn main(x: i64) -> i64 { let c = x <=> 0; let r = if_pos c { 200 } else { 100 }; return r; }"),
        ("loop_ok", "fn main(n: i64) -> i64 { let acc = 0; let i = n; do_while { acc = acc + i; i = i - 1; } i <=> 0; return acc; }"),
        ("sql_taint", "#[cap(DB_R)] fn main(id: i64 @untrusted) -> i64 { let r = db_query(\"SELECT * FROM users WHERE id = ?\", id); return r; }"),
        ("uncalled_tainted_helper", "fn bad_helper(u: i64 @untrusted) -> i64 { return u; }\nfn main(x: i64) -> i64 { return x + 1; }"),
        ("ok_bytes", "fn main(x: i64) -> i64 { let b: bytes = anew(0); b = iset(b, 1); let n: i64 = slen(b); return n + x; }"),
    ];
    for &(name, src) in surfaces {
        match compile_omni_surface_to_u16(src) {
            Ok(stream) => cases.push((String::from(name), stream)),
            Err(e) => {
                return Err(format!(
                    "verifier-diff corpus '{name}' failed to compile (needed as a verifier input): {e}"
                ));
            }
        }
    }

    let goldens: &[(&str, &str)] = &[
        ("seed0_risk", "fn seed0_risk(base: i64, delta: i64) -> trit { let scaled = base * 2; let score = scaled + delta; let cmp = score <=> 50; let pos = +1t; let res = tand(cmp, pos); return res; }"),
        ("iabs_golden", "fn iabs_golden(x: i64) -> i64 { let r = iabs(x); return r; }"),
        ("sel_mix", "fn sel_mix(x: i64) -> i64 { let prod = x * 3; let cmp = prod <=> 20; let bonus = sel_trit(cmp, 5, 10, 15); let res = prod + bonus; return res; }"),
        ("idiv_zero", "fn idiv_zero(x: i64) -> i64 { let r = x / 0; return r; }"),
        ("nested_branch", "fn nested_branch_golden(x: i64) -> i64 { let c0 = x <=> 0; let r = if_pos c0 { let c1 = x <=> 5; if_pos c1 { 30 } else { 20 } } else { 10 }; return r; }"),
        ("byte_buf", "fn byte_buf_golden(x: i64) -> i64 { let buf = anew(0); buf = iset(buf, 10, 20, 30); buf = iset_i32(buf, 100); buf = iset_i64(buf, 200); let len = slen(buf); let cmp = x <=> 5; let sel = cmov(cmp, 100, 200, 300); let total = len + sel; return total; }"),
    ];
    for &(name, src) in goldens {
        match compile_omni_surface_to_u16(src) {
            Ok(stream) => cases.push((String::from(name), stream)),
            Err(e) => return Err(format!("verifier-diff golden '{name}' failed to compile: {e}")),
        }
    }

    let tc_safe = pack_tc(false, CAP_ID_PURE);
    let tc_untr = pack_tc(true, CAP_ID_PURE);
    cases.push((
        String::from("raw_bad_taint_sink"),
        pack_fn_module(
            &[(0x0000, vec![(TY_STR, tc_safe)])],
            0,
            &[(0x7EC0, TY_STR, tc_untr)],
            TY_STRUCT,
            cap_bit(CAP_ID_DB_W),
            &[
                vec![0x7E00, 0x7F63, TY_STRUCT, pack_tc(false, CAP_ID_DB_W), TOK_SYM_SCHEMA, 0x0000, TOK_FIELD_KV, 0x0000, 0x7EC0, TOK_INS_END],
                vec![0x7F4A, 0x7E00, TOK_INS_END],
            ],
        ),
    ));
    cases.push((
        String::from("raw_bad_cap"),
        pack_fn_module(
            &[(0x0000, vec![(TY_STR, tc_safe)])],
            0,
            &[(0x7EC0, TY_STR, tc_safe)],
            TY_STRUCT,
            0x0000,
            &[
                vec![0x7E00, 0x7F63, TY_STRUCT, pack_tc(false, CAP_ID_DB_W), TOK_SYM_SCHEMA, 0x0000, TOK_FIELD_KV, 0x0000, 0x7EC0, TOK_INS_END],
                vec![0x7F4A, 0x7E00, TOK_INS_END],
            ],
        ),
    ));
    cases.push((
        String::from("raw_unknown_op"),
        pack_fn_module(
            &[],
            0,
            &[(0x7EC0, TY_I64, tc_safe)],
            TY_I64,
            0,
            &[
                vec![0x7E00, 0x7EFF, TY_I64, tc_safe, 0x7EC0, TOK_INS_END],
                vec![0x7F4A, 0x7E00, TOK_INS_END],
            ],
        ),
    ));

    // Deterministic fuzz: truncated, garbage, and mutated valid streams.
    let seed_ok = cases
        .iter()
        .find(|(n, _)| n == "ok_add")
        .map(|(_, s)| s.clone())
        .unwrap_or_default();
    cases.push((String::from("fuzz_empty"), Vec::new()));
    cases.push((String::from("fuzz_only_mod_begin"), vec![0x7EE0]));
    cases.push((String::from("fuzz_begin_end"), vec![0x7EE0, 0x7EEF]));
    cases.push((String::from("fuzz_bad_tag"), vec![0x7EE0, 0x1234, 0x7EEF]));
    let mut lcg: u32 = 0xC0FFEE11;
    let mut next = || {
        lcg = lcg.wrapping_mul(1664525).wrapping_add(1013904223);
        lcg
    };
    for i in 0..32 {
        let len = 1 + (next() as usize % 24);
        let mut s = Vec::with_capacity(len);
        s.push(0x7EE0);
        for _ in 1..len {
            s.push((next() as u16) & 0x7FFF);
        }
        cases.push((format!("fuzz_rand_{i}"), s));
    }
    for i in 0..16 {
        if seed_ok.len() < 2 {
            break;
        }
        let mut s = seed_ok.clone();
        let idx = (next() as usize) % s.len();
        s[idx] = s[idx].wrapping_add(1 + (next() as u16 % 7));
        cases.push((format!("fuzz_mut_ok_add_{i}"), s));
    }

    let mut matched = 0usize;
    for (name, stream) in &cases {
        compare_rust_and_omni_verifier(name, stream)?;
        matched += 1;
    }
    if matched != cases.len() {
        return Err(format!(
            "verifier-diff internal count mismatch: {matched} != {}",
            cases.len()
        ));
    }
    Ok(matched)
}

#[cfg(test)]
mod verifier_engine_tests {
    use super::*;

    #[test]
    fn verifier_engine_compiles_and_passes_rust_verify() {
        omni_verifier_engine_module().expect("engine must compile and pass the Rust verifier");
    }

    #[test]
    fn verifier_engine_matches_rust_on_core_cases() {
        let n = run_verifier_engine_diff_selftests().expect("differential suite");
        assert!(n >= 40, "expected at least 40 verifier-diff cases, got {n}");
    }

    #[test]
    fn lockstep_shadow_mode_regression() {
        let n = run_lockstep_shadow_selftests().expect("lockstep shadow mode");
        assert!(n >= 7, "expected at least 7 lockstep checks, got {n}");
        let (rust_ns, shadow_ns) = last_lockstep_overhead_ns();
        eprintln!(
            "lockstep checks={n}; overhead rust-only={:.3}ms rust+omni={:.3}ms delta={:.3}ms",
            rust_ns as f64 / 1_000_000.0,
            shadow_ns as f64 / 1_000_000.0,
            shadow_ns.saturating_sub(rust_ns) as f64 / 1_000_000.0
        );
    }

}
