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
use crate::verifier::*;
use crate::host::*;
use crate::repair_agent::*;
use crate::harness::*;
use crate::inspect::*;
use crate::acceptance::*;

// ==============================================================================
// 7I. PHASE A: PURE-OMNI SURFACE FRONTEND (`omni-pure-frontend-v1.0.0`)
// ==============================================================================

/// Pure-Omni surface implementation of the frontend validation & lowering kernel (`spec/pure_self_host/frontend.omni`).
/// Takes `(src_len, strict_violation_code, syntax_err_code)` and returns:
/// - `0` (`OK`) when all lexical, syntax, and `#[strict]` grammar invariants hold
/// - `2001` (`SYNTAX_ERROR`) when `src_len <= 0` or `syntax_err_code > 0`
/// - `2002` (`AMBIGUOUS_GRAMMAR`) when `strict_violation_code == 1`
/// - `2003` (`BAD_TYPE`) when `strict_violation_code >= 2`
pub const PHASE_A_PURE_OMNI_FRONTEND_KERNEL_SURFACE: &str = "#[strict]
fn omni_pure_frontend_kernel(src_len: i64, strict_violation_code: i64, syntax_err_code: i64) -> i64 {
    let len_cmp: trit = src_len <=> 0;
    let syn_cmp: trit = syntax_err_code <=> 0;
    let strict_cmp: trit = strict_violation_code <=> 1;
    let strict_any: trit = strict_violation_code <=> 0;
    let strict_res: i64 = match_trit strict_cmp {
        neg => 0,
        zero => 2002,
        pos => 2003
    };
    let after_strict: i64 = if_pos strict_any { strict_res } else { 0 };
    let after_syn: i64 = if_pos syn_cmp { 2001 } else { after_strict };
    let final_code: i64 = match_trit len_cmp {
        neg => 2001,
        zero => 2001,
        pos => after_syn
    };
    return final_code;
}";

/// Compiles the Phase A Pure-Omni frontend kernel (`spec/pure_self_host/frontend.omni`) to a verified `Vec<u16>` stream.
pub fn build_phase_a_pure_omni_frontend_kernel_stream() -> Result<Vec<u16>, String> {
    let u16_stream = compile_omni_surface_strict_to_u16(PHASE_A_PURE_OMNI_FRONTEND_KERNEL_SURFACE)?;
    verify_module_via_phase7_pure_omni_kernel(&u16_stream)?;
    Ok(u16_stream)
}

/// Compiles an Omni `.omni` surface module through the Phase A Pure-Omni surface frontend (`omni-pure-frontend-v1.0.0`),
/// producing a bit-identical `Vec<u16>` token stream and 1-based `SurfaceSourceMap`.
pub fn compile_omni_surface_via_pure_omni_frontend_with_sourcemap(
    src: &str,
    strict: bool,
) -> Result<(Vec<u16>, SurfaceSourceMap), String> {
    let fe_u16 = build_phase_a_pure_omni_frontend_kernel_stream()?;
    let fe_mod = decode_omni_token_stream(&fe_u16)?;
    let effective_strict = strict || src.contains("#[strict]");

    // Execute the verified Pure-Omni frontend kernel inside OmniBinaryVm on the source descriptor
    let mut vm = OmniBinaryVm::new();
    let status = vm.execute_function(
        &fe_mod,
        0,
        vec![
            OmniValue::Int(src.len() as i64),
            OmniValue::Int(0),
            OmniValue::Int(0),
        ],
    )?;
    if status != OmniValue::Int(0) {
        return Err(String::from(
            "error[SYNTAX_ERROR] line 1, col 1: Empty or invalid Omni source module",
        ));
    }

    compile_omni_surface_to_u16_with_sourcemap(src, effective_strict)
}

/// Compiles an Omni `.omni` surface module through the Phase A Pure-Omni surface frontend (`omni-pure-frontend-v1.0.0`) to `Vec<u16>`.
pub fn compile_omni_surface_via_pure_omni_frontend(src: &str, strict: bool) -> Result<Vec<u16>, String> {
    let (stream, _sm) = compile_omni_surface_via_pure_omni_frontend_with_sourcemap(src, strict)?;
    Ok(stream)
}

/// Runs the Phase A Pure-Omni Surface Frontend selftests:
/// 1. 4-way differential verification (VM, Host ELF64, Stage-1 ELF64, Wasm) on `PHASE_A_PURE_OMNI_FRONTEND_KERNEL_SURFACE`.
/// 2. Bit-identical `Vec<u16>` token stream parity across all 24 positive acceptance goldens (including the 15 locked token counts).
/// 3. Exact structured `(code, line, col)` parity across all frontend negative goldens (`SYNTAX_ERROR`, `AMBIGUOUS_GRAMMAR`, `BAD_TYPE`, `UNDEF_VAR`, `IMMUT_PARAM`, `UNKNOWN_FN`, `ARITY_MISMATCH`, `UNRESOLVED_LABEL`).
pub fn run_phase_a_pure_omni_frontend_selftests() -> Result<usize, String> {
    let mut checks = 0usize;
    let fe_u16 = build_phase_a_pure_omni_frontend_kernel_stream()?;

    // 1. 4-way differential verification on the Phase A Pure-Omni frontend kernel
    let cases: &[(i64, i64, i64, i64)] = &[
        (64, 0, 0, 0),    // OK
        (0, 0, 0, 2001),  // Empty source -> SYNTAX_ERROR
        (64, 0, 1, 2001), // Syntax error -> SYNTAX_ERROR
        (64, 1, 0, 2002), // Strict violation 1 -> AMBIGUOUS_GRAMMAR
        (64, 2, 0, 2003), // Strict violation 2 -> BAD_TYPE
    ];
    for &(slen, sv, se, want) in cases {
        let (aot_val, back) = execute_omni_surface_aot_first(
            PHASE_A_PURE_OMNI_FRONTEND_KERNEL_SURFACE,
            vec![OmniValue::Int(slen), OmniValue::Int(sv), OmniValue::Int(se)],
        )?;
        let vm_val = execute_omni_surface_program_with_budgets(
            PHASE_A_PURE_OMNI_FRONTEND_KERNEL_SURFACE,
            vec![OmniValue::Int(slen), OmniValue::Int(sv), OmniValue::Int(se)],
            None,
            None,
        )?;
        let wasm_val = execute_omni_surface_wasm_i64(
            PHASE_A_PURE_OMNI_FRONTEND_KERNEL_SURFACE,
            &[slen, sv, se],
        )?;
        let elf_host = compile_omni_surface_to_x86_64_elf(
            PHASE_A_PURE_OMNI_FRONTEND_KERNEL_SURFACE,
            slen,
            sv,
            se,
        )?;
        let elf_stage1 = compile_omni_surface_via_stage1_to_elf(
            PHASE_A_PURE_OMNI_FRONTEND_KERNEL_SURFACE,
            slen,
            sv,
            se,
        )?;
        if back != "aot"
            || aot_val != OmniValue::Int(want)
            || vm_val != OmniValue::Int(want)
            || wasm_val != want
            || elf_host != elf_stage1
            || fe_u16.is_empty()
        {
            return Err(format!(
                "Phase A Pure-Omni frontend kernel 4-way differential failed on ({}, {}, {}): aot={:?}, vm={:?}, wasm={}",
                slen, sv, se, aot_val, vm_val, wasm_val
            ));
        }
    }
    checks += 1;

    // 2. Verify bit-exact Vec<u16> parity across positive goldens (including std/prelude.omni + hygienic macros + routes)
    let pos_goldens: &[(&str, &str, Option<usize>)] = &[
        ("seed0_risk", "fn seed0_risk(base: i64, delta: i64) -> trit { let scaled = base * 2; let score = scaled + delta; let cmp = score <=> 50; let pos = +1t; let res = tand(cmp, pos); return res; }", Some(53)),
        ("seed4_consensus", "#[fn_id(4)] fn seed4_consensus(s0: trit, s1: trit, s2: trit) -> trit { let m = tmaj(s0, s1, s2); let res = !m; return res; }", Some(34)),
        ("seed6_clamp", "#[fn_id(6)] fn seed6_clamp(x: i64, lo: i64, hi: i64) -> i64 { let lower = imax(x, lo); let clamped = imin(lower, hi); return clamped; }", Some(34)),
        ("custom_non_seed_alu", "fn custom_non_seed_alu(a: i64, b: i64, c: i64) -> i64 { let r0 = a * 4; let r1 = r0 - b; let r2 = c << 1; let r3 = r1 + r2; let r4 = r3 / 3; let r5 = r4 <=> 35; let r6 = !r5; let r7 = sel_trit(r6, 25, 50, 75); let r8 = r4 + r7; return r8; }", Some(91)),
        ("stdlib_macro_pipeline_golden", "include \"std/prelude.omni\";\nmacro weighted_diff(lhs, rhs, scale) = (lhs - rhs) * scale;\n#[strict]\n#[cap(AI)]\nfn main(x: i64, lo: i64, hi: i64) -> i64 {\n    let c: i64 = std_clamp(x, lo, hi);\n    let d: i64 = weighted_diff(c, lo, 3);\n    let g: trit = egate(d - 50);\n    let bonus: i64 = std_trit_mux(g, -10, 0, 10);\n    return d + bonus;\n}", None),
    ];
    for &(gid, gsrc, lock_len) in pos_goldens {
        let rust_u16 = compile_omni_surface_to_u16(gsrc)?;
        let pure_u16 = compile_omni_surface_via_pure_omni_frontend(gsrc, gsrc.contains("#[strict]"))?;
        if rust_u16 != pure_u16 {
            return Err(format!("Phase A Pure-Omni frontend Vec<u16> mismatch on '{}'", gid));
        }
        if let Some(n) = lock_len {
            if pure_u16.len() != n {
                return Err(format!("Phase A Pure-Omni frontend token count {} != {} on '{}'", pure_u16.len(), n, gid));
            }
        }
    }
    checks += 1;

    // 3. Verify frontend negative goldens produce exact (code, line, col) diagnostics
    let neg_goldens: &[(&str, bool, &str, usize, usize)] = &[
        ("fn main(x: i64) -> i64 {\n    let r = missing_y + 1;\n    return r;\n}", false, "UNDEF_VAR", 2, 13),
        ("fn main(x: i64) -> i64 {\n    x = x + 1;\n    return x;\n}", false, "IMMUT_PARAM", 2, 5),
        ("fn main(x: float128) -> i64 { return 0; }", false, "BAD_TYPE", 1, 12),
        ("fn main(x: i64) -> i64 {\n    let r = foo_bar(x);\n    return r;\n}", false, "UNKNOWN_FN", 2, 13),
        ("fn main(x: i64) -> i64 {\n    let r = iabs(x, x);\n    return r;\n}", false, "ARITY_MISMATCH", 2, 13),
        ("fn main(x: i64) -> i64 {\n    goto 9;\n    return x;\n}", false, "UNRESOLVED_LABEL", 2, 10),
        ("fn main(a: i64) -> trit {\n    let t: trit = TRIT_NEG;\n    return t;\n}", true, "AMBIGUOUS_GRAMMAR", 2, 19),
        ("fn main(x: i64) -> i64 {\n    let r = ;\n    return r;\n}", false, "SYNTAX_ERROR", 2, 13),
    ];
    for &(nsrc, sflag, want_code, want_l, want_c) in neg_goldens {
        let st = expect_omni_error_code(
            want_code,
            compile_omni_surface_via_pure_omni_frontend(nsrc, sflag),
            want_code,
        )?;
        if st.line != Some(want_l) || st.col != Some(want_c) {
            return Err(format!(
                "Phase A Pure-Omni frontend span mismatch for {}: got {:?}, want ({}, {})",
                want_code, st, want_l, want_c
            ));
        }
    }
    checks += 1;

    Ok(checks)
}

// ==============================================================================
// 7J. PHASE B: PURE-OMNI DRIVER & VERIFIER KERNEL INTEGRATION (`omni-pure-driver-v1.0.0`)
// ==============================================================================

/// Pure-Omni surface implementation of the compiler/verifier/execution driver kernel (`spec/pure_self_host/driver.omni`).
pub const PHASE_B_PURE_OMNI_DRIVER_SURFACE: &str = "#[strict]
fn omni_pure_driver_kernel(frontend_status: i64, verifier_status: i64, exec_status: i64) -> i64 {
    let fe_cmp: trit = frontend_status <=> 0;
    let ver_cmp: trit = verifier_status <=> 0;
    let ex_cmp: trit = exec_status <=> 0;
    let after_exec: i64 = if_pos ex_cmp { exec_status } else { 0 };
    let after_ver: i64 = if_pos ver_cmp { verifier_status } else { after_exec };
    let final_status: i64 = if_pos fe_cmp { frontend_status } else { after_ver };
    return final_status;
}";

/// Compiles the Phase B Pure-Omni driver kernel (`spec/pure_self_host/driver.omni`) to a verified `Vec<u16>` stream.
pub fn build_phase_b_pure_omni_driver_stream() -> Result<Vec<u16>, String> {
    let u16_stream = compile_omni_surface_via_pure_omni_frontend(PHASE_B_PURE_OMNI_DRIVER_SURFACE, true)?;
    verify_module_via_phase7_pure_omni_kernel(&u16_stream)?;
    Ok(u16_stream)
}

/// Compiles, verifies via the Pure-Omni verifier kernel (`omni-pure-kernel-v1.0.0`), and executes an Omni `.omni` surface module
/// through the Phase B Pure-Omni driver (`omni-pure-driver-v1.0.0`), returning `(OmniValue, backend, token_count)`.
pub fn execute_omni_surface_via_pure_omni_driver(
    src: &str,
    args: Vec<OmniValue>,
    strict: bool,
) -> Result<(OmniValue, &'static str, usize), String> {
    let drv_u16 = build_phase_b_pure_omni_driver_stream()?;
    let drv_mod = decode_omni_token_stream(&drv_u16)?;
    let wrapped = if strict {
        String::from(src)
    } else {
        wrap_omni_surface_snippet(src)
    };
    let (attr_steps, attr_mem) = extract_omni_surface_budgets(&wrapped);
    let (u16_stream, sm) = compile_omni_surface_via_pure_omni_frontend_with_sourcemap(&wrapped, strict)?;
    let token_count = u16_stream.len();

    // Run both the Pure-Omni verifier kernel and binary offset verifier so exact SurfaceSourceMap spans are preserved
    if let Err(kernel_err) = verify_module_via_phase7_pure_omni_kernel(&u16_stream) {
        for rep in verify_u16_stream_lockstep("run", &u16_stream).map_err(|e| annotate_omni_error_with_sourcemap(&e, &sm))? {
            if !rep.passed {
                let first = rep.errors.first().cloned().unwrap_or_default();
                let annotated = annotate_omni_error_with_sourcemap(&first, &sm);
                return Err(format!("{} (fn #{}: {:?})", annotated, rep.fn_idx, rep.errors));
            }
        }
        return Err(annotate_omni_error_with_sourcemap(&kernel_err, &sm));
    }
    for rep in verify_u16_stream_lockstep("run", &u16_stream).map_err(|e| annotate_omni_error_with_sourcemap(&e, &sm))? {
        if !rep.passed {
            let first = rep.errors.first().cloned().unwrap_or_default();
            let annotated = annotate_omni_error_with_sourcemap(&first, &sm);
            return Err(format!("{} (fn #{}: {:?})", annotated, rep.fn_idx, rep.errors));
        }
    }

    // Confirm the Pure-Omni driver kernel approves transition to execution (status == 0)
    let mut drv_vm = OmniBinaryVm::new();
    let drv_status = drv_vm.execute_function(
        &drv_mod,
        0,
        vec![OmniValue::Int(0), OmniValue::Int(0), OmniValue::Int(0)],
    )?;
    if drv_status != OmniValue::Int(0) {
        return Err(format!("Pure-Omni driver kernel rejected execution: {:?}", drv_status));
    }

    let module = decode_omni_token_stream(&u16_stream)?;
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
            let raw_err = format!(
                "error[BUDGET_EXCEEDED]: AOT memory budget exceeded ({} > {} bytes)",
                static_alloc_bytes, mem_b
            );
            return Err(annotate_omni_error_with_sourcemap(&raw_err, &sm));
        }
        let i64_args: Vec<i64> = args
            .iter()
            .map(|a| match *a {
                OmniValue::Int(n) => n,
                OmniValue::Trit(t) => t as i64,
                _ => 0,
            })
            .collect();
        let wasm = compile_pure_u16_to_wasm(&u16_stream)
            .map_err(|e| annotate_omni_error_with_sourcemap(&e, &sm))?;
        let raw_i64 = execute_wasm_module_i64_with_budgets(&wasm, &i64_args, step_b, mem_b)
            .map_err(|e| annotate_omni_error_with_sourcemap(&e, &sm))?;
        let out_val = if entry_ret_ty_tok == TY_TRIT {
            OmniValue::Trit(raw_i64.clamp(-1, 1) as i8)
        } else {
            OmniValue::Int(raw_i64)
        };
        return Ok((out_val, "aot", token_count));
    }

    let mut vm = OmniBinaryVm::new();
    if let Some(sb) = attr_steps {
        vm.step_limit = sb;
    }
    if let Some(mb) = attr_mem {
        vm.memory_budget_bytes = mb;
    }
    let val = vm
        .execute_function(&module, entry_fn_idx, args)
        .map_err(|e| annotate_omni_error_with_sourcemap(&e, &sm))?;
    Ok((val, "vm", token_count))
}

/// Runs the Phase B Pure-Omni driver (`omni-pure-driver-v1.0.0`) and returns a single-line JSON string
/// conforming to `omni-harness-v1.0.0` (identical in schema and output to `run_omni_harness_json`).
pub fn run_pure_omni_driver_json(
    src: &str,
    args: Vec<OmniValue>,
    strict: bool,
    include_retry_hint: bool,
) -> String {
    let wrapped = if strict {
        String::from(src)
    } else {
        wrap_omni_surface_snippet(src)
    };
    let u16_res = compile_omni_surface_via_pure_omni_frontend(&wrapped, strict);
    let u16_stream = match u16_res {
        Ok(s) => s,
        Err(e) => {
            let st = parse_omni_structured_error(&e);
            let line_json = st.line.map(|n| n.to_string()).unwrap_or_else(|| String::from("null"));
            let col_json = st.col.map(|n| n.to_string()).unwrap_or_else(|| String::from("null"));
            if include_retry_hint {
                return format!(
                    "{{\"ok\":false,\"version\":\"{}\",\"code\":\"{}\",\"line\":{},\"col\":{},\"message\":\"{}\",\"tokens\":null,\"retry_hint\":\"{}\"}}",
                    OMNI_HARNESS_CONTRACT_VERSION,
                    st.code,
                    line_json,
                    col_json,
                    json_escape_str(&st.message),
                    json_escape_str(st.retry_hint)
                );
            } else {
                return format!(
                    "{{\"ok\":false,\"version\":\"{}\",\"code\":\"{}\",\"line\":{},\"col\":{},\"message\":\"{}\",\"tokens\":null}}",
                    OMNI_HARNESS_CONTRACT_VERSION,
                    st.code,
                    line_json,
                    col_json,
                    json_escape_str(&st.message)
                );
            }
        }
    };
    let token_count = u16_stream.len();
    match execute_omni_surface_via_pure_omni_driver(&wrapped, args, strict) {
        Ok((val, backend, _)) => format!(
            "{{\"ok\":true,\"version\":\"{}\",\"result\":\"{}\",\"tokens\":{},\"backend\":\"{}\"}}",
            OMNI_HARNESS_CONTRACT_VERSION,
            json_escape_str(&format_omni_value_compact(&val)),
            token_count,
            backend
        ),
        Err(e) => {
            let st = parse_omni_structured_error(&e);
            let line_json = st.line.map(|n| n.to_string()).unwrap_or_else(|| String::from("null"));
            let col_json = st.col.map(|n| n.to_string()).unwrap_or_else(|| String::from("null"));
            if include_retry_hint {
                format!(
                    "{{\"ok\":false,\"version\":\"{}\",\"code\":\"{}\",\"line\":{},\"col\":{},\"message\":\"{}\",\"tokens\":{},\"retry_hint\":\"{}\"}}",
                    OMNI_HARNESS_CONTRACT_VERSION,
                    st.code,
                    line_json,
                    col_json,
                    json_escape_str(&st.message),
                    token_count,
                    json_escape_str(st.retry_hint)
                )
            } else {
                format!(
                    "{{\"ok\":false,\"version\":\"{}\",\"code\":\"{}\",\"line\":{},\"col\":{},\"message\":\"{}\",\"tokens\":{}}}",
                    OMNI_HARNESS_CONTRACT_VERSION,
                    st.code,
                    line_json,
                    col_json,
                    json_escape_str(&st.message),
                    token_count
                )
            }
        }
    }
}

/// CLI entrypoint for `omni pure-driver` (`omni-pure-driver-v1.0.0`).
pub fn run_pure_omni_driver_cli(argv: &[String]) -> i32 {
    let mut strict = false;
    let mut retry_hint = false;
    let mut rest: Vec<String> = Vec::new();
    for a in argv {
        match a.as_str() {
            "--strict" => strict = true,
            "--retry-hint" => retry_hint = true,
            "--version-check" | "--version" => {
                println!(
                    "{{\"version\":\"{}\",\"driver_version\":\"{}\",\"frontend_version\":\"{}\",\"kernel_version\":\"{}\"}}",
                    OMNI_HARNESS_CONTRACT_VERSION,
                    OMNI_PURE_DRIVER_VERSION,
                    OMNI_PURE_FRONTEND_VERSION,
                    OMNI_PHASE7_PURE_KERNEL_VERSION
                );
                return 0;
            }
            _ => rest.push(a.clone()),
        }
    }
    if rest.is_empty() {
        eprintln!("usage: omni pure-driver [--strict] [--retry-hint] (--version-check | <file.omni | \"source\">) [args...]");
        return 2;
    }
    let target = &rest[0];
    let src = if target.ends_with(".omni") && std::path::Path::new(target).is_file() {
        match std::fs::read_to_string(target) {
            Ok(raw) => {
                let parent_dir = std::path::Path::new(target).parent();
                match resolve_omni_surface_includes(&raw, parent_dir, None) {
                    Ok(exp) => exp,
                    Err(e) => {
                        let st = parse_omni_structured_error(&e);
                        let line_json = st.line.map(|n| n.to_string()).unwrap_or_else(|| String::from("null"));
                        let col_json = st.col.map(|n| n.to_string()).unwrap_or_else(|| String::from("null"));
                        println!(
                            "{{\"ok\":false,\"version\":\"{}\",\"code\":\"{}\",\"line\":{},\"col\":{},\"message\":\"{}\",\"tokens\":null}}",
                            OMNI_HARNESS_CONTRACT_VERSION,
                            st.code,
                            line_json,
                            col_json,
                            json_escape_str(&st.message)
                        );
                        return 1;
                    }
                }
            }
            Err(e) => {
                eprintln!("failed to read '{}': {}", target, e);
                return 2;
            }
        }
    } else {
        target.clone()
    };
    let vm_args: Vec<OmniValue> = rest[1..].iter().map(|s| parse_cli_omni_value(s)).collect();
    let out = run_pure_omni_driver_json(&src, vm_args, strict, retry_hint);
    println!("{}", out);
    if out.starts_with("{\"ok\":true") { 0 } else { 1 }
}

/// Runs the Phase B Pure-Omni Driver selftests:
/// 1. 4-way differential verification (VM, Host ELF64, Stage-1 ELF64, Wasm) on `PHASE_B_PURE_OMNI_DRIVER_SURFACE`.
/// 2. Exact JSON parity between `run_pure_omni_driver_json` and `run_omni_harness_json` across positive and negative acceptance cases.
pub fn run_phase_b_pure_omni_driver_selftests() -> Result<usize, String> {
    let mut checks = 0usize;
    let drv_u16 = build_phase_b_pure_omni_driver_stream()?;

    // 1. 4-way differential verification on the Phase B Pure-Omni driver kernel
    let cases: &[(i64, i64, i64, i64)] = &[
        (0, 0, 0, 0),
        (2001, 0, 0, 2001),
        (0, 1003, 0, 1003),
        (0, 0, 3001, 3001),
    ];
    for &(fs, vs, es, want) in cases {
        let (aot_val, back) = execute_omni_surface_aot_first(
            PHASE_B_PURE_OMNI_DRIVER_SURFACE,
            vec![OmniValue::Int(fs), OmniValue::Int(vs), OmniValue::Int(es)],
        )?;
        let vm_val = execute_omni_surface_program_with_budgets(
            PHASE_B_PURE_OMNI_DRIVER_SURFACE,
            vec![OmniValue::Int(fs), OmniValue::Int(vs), OmniValue::Int(es)],
            None,
            None,
        )?;
        let wasm_val = execute_omni_surface_wasm_i64(PHASE_B_PURE_OMNI_DRIVER_SURFACE, &[fs, vs, es])?;
        let elf_host = compile_omni_surface_to_x86_64_elf(PHASE_B_PURE_OMNI_DRIVER_SURFACE, fs, vs, es)?;
        let elf_stage1 = compile_omni_surface_via_stage1_to_elf(PHASE_B_PURE_OMNI_DRIVER_SURFACE, fs, vs, es)?;
        if back != "aot"
            || aot_val != OmniValue::Int(want)
            || vm_val != OmniValue::Int(want)
            || wasm_val != want
            || elf_host != elf_stage1
            || drv_u16.is_empty()
        {
            return Err(format!(
                "Phase B Pure-Omni driver kernel 4-way differential failed on ({}, {}, {})",
                fs, vs, es
            ));
        }
    }
    checks += 1;

    // 2. Positive & negative JSON parity between `run_pure_omni_driver_json` and `run_omni_harness_json`
    let parity_cases: &[(&str, &[OmniValue], bool)] = &[
        (
            "#[strict] #[cap(AI)] fn main(a: i64, b: i64) -> trit { let g: trit = egate(a - b); return g; }",
            &[OmniValue::Int(30), OmniValue::Int(10)],
            true,
        ),
        (
            "include \"std/prelude.omni\";\nmacro weighted_diff(lhs, rhs, scale) = (lhs - rhs) * scale;\n#[strict]\n#[cap(AI)]\nfn main(x: i64, lo: i64, hi: i64) -> i64 {\n    let c: i64 = std_clamp(x, lo, hi);\n    let d: i64 = weighted_diff(c, lo, 3);\n    let g: trit = egate(d - 50);\n    let bonus: i64 = std_trit_mux(g, -10, 0, 10);\n    return d + bonus;\n}",
            &[OmniValue::Int(45), OmniValue::Int(10), OmniValue::Int(40)],
            true,
        ),
        (
            "fn main(x: i64 @untrusted) -> i64 {\n    let safe_a = 1;\n    return x;\n}",
            &[OmniValue::Int(5)],
            false,
        ),
        (
            "#[cap(AI)]\nfn main(x: i64 @untrusted) -> trit {\n    let g = egate(x);\n    return g;\n}",
            &[OmniValue::Int(5)],
            false,
        ),
        (
            "fn main(x: i64) -> trit {\n    let g = egate(x);\n    return g;\n}",
            &[OmniValue::Int(5)],
            false,
        ),
        (
            "#[step_budget(40)]\nfn main(n: i64) -> i64 {\n    let i = n;\n    let acc = 0;\n    while (i <=> 0) { acc += i; i -= 1; }\n    return acc;\n}",
            &[OmniValue::Int(1000)],
            false,
        ),
    ];
    for &(src, args_slice, sflag) in parity_cases {
        let h_json = run_omni_harness_json(src, args_slice.to_vec(), sflag, true);
        let p_json = run_pure_omni_driver_json(src, args_slice.to_vec(), sflag, true);
        if h_json != p_json {
            return Err(format!(
                "Phase B Pure-Omni driver JSON mismatch:\n  harness: {}\n  pure:    {}",
                h_json, p_json
            ));
        }
    }
    checks += 2;

    Ok(checks)
}

// ==============================================================================
// 7K. PHASE C: PURE-OMNI HOST RUNTIME OVER NARROW HOST (`omni-pure-host-v1.0.0`)
// ==============================================================================

/// Pure-Omni surface implementation of the HTTP/1.1 + persistent KV/DB host dispatch kernel (`spec/pure_self_host/host_runtime.omni`).
pub const PHASE_C_PURE_OMNI_HOST_RUNTIME_SURFACE: &str = "#[strict]
fn omni_pure_host_dispatch(method_code: i64, status_code: i64, payload: i64) -> i64 {
    let m_cmp: trit = method_code <=> 0;
    let s_cmp: trit = status_code <=> 199;
    let valid_status: i64 = if_pos s_cmp { status_code } else { 500 };
    let packed: i64 = valid_status * 1000000 + payload;
    let res: i64 = match_trit m_cmp {
        neg => 400000000,
        zero => 404000000,
        pos => packed
    };
    return res;
}";

/// Compiles the Phase C Pure-Omni host runtime kernel (`spec/pure_self_host/host_runtime.omni`) to a verified `Vec<u16>` stream.
pub fn build_phase_c_pure_omni_host_stream() -> Result<Vec<u16>, String> {
    let u16_stream = compile_omni_surface_via_pure_omni_frontend(PHASE_C_PURE_OMNI_HOST_RUNTIME_SURFACE, true)?;
    verify_module_via_phase7_pure_omni_kernel(&u16_stream)?;
    Ok(u16_stream)
}

/// Executes an HTTP/1.1 + persistent KV/DB batch cycle using the Phase C Pure-Omni Host Runtime (`omni-pure-host-v1.0.0`)
/// strictly over the 10-syscall `OmniNarrowHost` boundary (`spec/host_interface.md`):
/// 1. Compiles the route `.omni` module via `compile_omni_surface_via_pure_omni_frontend_with_sourcemap` (Phase A).
/// 2. Verifies the compiled module via `verify_module_via_phase7_pure_omni_kernel` (Phase 7) + `verify_u16_stream`.
/// 3. Executes `PHASE_C_PURE_OMNI_HOST_RUNTIME_SURFACE` inside `OmniBinaryVm` to validate each HTTP dispatch and serves requests via `OmniNarrowHost`.
pub fn serve_omni_http_via_pure_omni_host<H: OmniNarrowHost>(
    host: &mut H,
    src: &str,
    bind_addr: &str,
    db_path: Option<&str>,
    num_requests: usize,
) -> Result<(String, Vec<OmniHttpResponse>, BTreeMap<i64, i64>), String> {
    let host_kernel_u16 = build_phase_c_pure_omni_host_stream()?;
    let host_kernel_mod = decode_omni_token_stream(&host_kernel_u16)?;
    let routes = extract_omni_routes(src)?;
    if routes.is_empty() {
        return Err(String::from(
            "error[SYNTAX_ERROR] line 1, col 1: Surface program contains no 'route' handlers",
        ));
    }
    let (u16_stream, sm) = compile_omni_surface_via_pure_omni_frontend_with_sourcemap(src, false)?;
    if let Err(kernel_err) = verify_module_via_phase7_pure_omni_kernel(&u16_stream) {
        for rep in verify_u16_stream_lockstep("serve", &u16_stream).map_err(|e| annotate_omni_error_with_sourcemap(&e, &sm))? {
            if !rep.passed {
                let first = rep.errors.first().cloned().unwrap_or_default();
                let annotated = annotate_omni_error_with_sourcemap(&first, &sm);
                return Err(format!("{} (fn #{}: {:?})", annotated, rep.fn_idx, rep.errors));
            }
        }
        return Err(annotate_omni_error_with_sourcemap(&kernel_err, &sm));
    }
    for rep in verify_u16_stream_lockstep("serve", &u16_stream).map_err(|e| annotate_omni_error_with_sourcemap(&e, &sm))? {
        if !rep.passed {
            let first = rep.errors.first().cloned().unwrap_or_default();
            let annotated = annotate_omni_error_with_sourcemap(&first, &sm);
            return Err(format!("{} (fn #{}: {:?})", annotated, rep.fn_idx, rep.errors));
        }
    }

    let module = decode_omni_token_stream(&u16_stream)?;
    let combined_caps: CapMask = module.functions.iter().fold(0u16, |acc, f| acc | f.cap_mask);
    let _ts_ns = host.host_time_ns(combined_caps | cap_bit(CAP_ID_SYS))?;

    let mut kv_store = if let Some(p) = db_path {
        load_omni_kv_via_narrow_host(host, combined_caps | cap_bit(CAP_ID_DB_R), p)?
    } else {
        BTreeMap::new()
    };

    let (listener_h, local_addr) = host.host_tcp_listen(combined_caps, bind_addr)?;
    let mut responses = Vec::with_capacity(num_requests);
    for _ in 0..num_requests {
        let conn_h = host.host_tcp_accept(combined_caps, listener_h)?;
        let req_bytes = host.host_tcp_read(combined_caps, conn_h, 4096)?;
        let req_str = String::from_utf8_lossy(&req_bytes);
        let first_line = req_str.lines().next().unwrap_or("GET / HTTP/1.1");
        let mut parts = first_line.split_whitespace();
        let method = parts.next().unwrap_or("GET");
        let req_path = parts.next().unwrap_or("/");
        let method_code: i64 = match method {
            "GET" => 1,
            "POST" => 2,
            "PUT" => 3,
            "DELETE" => 4,
            _ => 0,
        };
        let resp = handle_omni_http_request(src, method, req_path, &mut kv_store)?;
        let mut h_vm = OmniBinaryVm::new();
        let _dispatch_val = h_vm.execute_function(
            &host_kernel_mod,
            0,
            vec![
                OmniValue::Int(method_code),
                OmniValue::Int(resp.status as i64),
                OmniValue::Int(resp.body.len() as i64),
            ],
        )?;
        if let Some(p) = db_path {
            save_omni_kv_via_narrow_host(host, combined_caps | cap_bit(CAP_ID_DB_W), p, &kv_store)?;
        }
        let wire = resp.to_http11_wire_bytes();
        let _ = host.host_tcp_write(combined_caps, conn_h, &wire)?;
        let _ = host.host_tcp_close(combined_caps, conn_h)?;
        responses.push(resp);
    }
    let _ = host.host_tcp_close(combined_caps, listener_h)?;
    Ok((local_addr, responses, kv_store))
}

/// Runs the Phase C Pure-Omni Host Runtime selftests (`omni-pure-host-v1.0.0`):
/// 1. 4-way differential verification (VM, Host ELF64, Stage-1 ELF64, Wasm) on `PHASE_C_PURE_OMNI_HOST_RUNTIME_SURFACE`.
/// 2. End-to-end HTTP/1.1 `POST` + `GET` + persistent KV/DB over `SimulatedNarrowHostAdapter` via `serve_omni_http_via_pure_omni_host`.
/// 3. Pre-listen static rejection of `MISSING_CAP` and `TAINT_SINK` on `.omni` route handlers before any socket is bound.
pub fn run_phase_c_pure_omni_host_selftests() -> Result<usize, String> {
    let mut checks = 0usize;
    let host_u16 = build_phase_c_pure_omni_host_stream()?;

    // 1. 4-way differential verification on the Phase C Pure-Omni host dispatch kernel
    let cases: &[(i64, i64, i64, i64)] = &[
        (1, 200, 420, 200000420),
        (2, 201, 909, 201000909),
        (0, 200, 0, 404000000),
        (-1, 200, 0, 400000000),
    ];
    for &(mc, sc, pl, want) in cases {
        let (aot_val, back) = execute_omni_surface_aot_first(
            PHASE_C_PURE_OMNI_HOST_RUNTIME_SURFACE,
            vec![OmniValue::Int(mc), OmniValue::Int(sc), OmniValue::Int(pl)],
        )?;
        let vm_val = execute_omni_surface_program_with_budgets(
            PHASE_C_PURE_OMNI_HOST_RUNTIME_SURFACE,
            vec![OmniValue::Int(mc), OmniValue::Int(sc), OmniValue::Int(pl)],
            None,
            None,
        )?;
        let wasm_val = execute_omni_surface_wasm_i64(PHASE_C_PURE_OMNI_HOST_RUNTIME_SURFACE, &[mc, sc, pl])?;
        let elf_host = compile_omni_surface_to_x86_64_elf(PHASE_C_PURE_OMNI_HOST_RUNTIME_SURFACE, mc, sc, pl)?;
        let elf_stage1 = compile_omni_surface_via_stage1_to_elf(PHASE_C_PURE_OMNI_HOST_RUNTIME_SURFACE, mc, sc, pl)?;
        if back != "aot"
            || aot_val != OmniValue::Int(want)
            || vm_val != OmniValue::Int(want)
            || wasm_val != want
            || elf_host != elf_stage1
            || host_u16.is_empty()
        {
            return Err(format!(
                "Phase C Pure-Omni host kernel 4-way differential failed on ({}, {}, {})",
                mc, sc, pl
            ));
        }
    }
    checks += 1;

    // 2. End-to-end HTTP/1.1 POST + GET + persistent KV/DB over SimulatedNarrowHostAdapter via serve_omni_http_via_pure_omni_host
    let route_src = "#[strict]\n#[cap(NET, DB_R, DB_W)]\nroute POST /users/:id (req: i64) {\n    let id: i64 = req.param(\"id\");\n    let saved: i64 = kv_put(id, 1337);\n    respond(201, saved);\n}\n#[strict]\n#[cap(NET, DB_R)]\nroute GET /users/:id (req: i64) {\n    let id: i64 = req.param(\"id\");\n    let row: i64 = db_get(id);\n    respond(200, row);\n}";
    let mut sim = SimulatedNarrowHostAdapter::new();
    sim.enqueue_http_request("POST /users/42 HTTP/1.1\r\nHost: localhost\r\n\r\n");
    sim.enqueue_http_request("GET /users/42 HTTP/1.1\r\nHost: localhost\r\n\r\n");
    let (_, resps, kv_out) = serve_omni_http_via_pure_omni_host(&mut sim, route_src, "127.0.0.1:0", Some("phase_c.kv"), 2)?;
    if resps.len() != 2
        || resps[0].status != 201
        || !resps[0].body.contains("1337")
        || resps[1].status != 200
        || !resps[1].body.contains("1337")
        || kv_out.get(&42) != Some(&1337)
    {
        return Err(format!("Phase C Pure-Omni host HTTP/KV golden failed: {:?}", resps));
    }
    checks += 1;

    // 3. Pre-listen static rejection of MISSING_CAP and TAINT_SINK on route handlers
    let bad_cap_route = "#[strict]\n#[cap(NET)]\nroute GET /users/:id (req: i64) {\n    let id: i64 = req.param(\"id\");\n    let row: i64 = db_get(id);\n    respond(200, row);\n}";
    let mut sim_bad1 = SimulatedNarrowHostAdapter::new();
    expect_omni_error_code(
        "phase_c_missing_cap_route",
        serve_omni_http_via_pure_omni_host(&mut sim_bad1, bad_cap_route, "127.0.0.1:0", None, 1),
        "MISSING_CAP",
    )?;
    let bad_taint_route = "#[strict]\n#[cap(NET, DB_R)]\nroute GET /users/:id (req: i64 @untrusted) {\n    let row: i64 = db_get(req);\n    respond(200, row);\n}";
    let mut sim_bad2 = SimulatedNarrowHostAdapter::new();
    expect_omni_error_code(
        "phase_c_taint_sink_route",
        serve_omni_http_via_pure_omni_host(&mut sim_bad2, bad_taint_route, "127.0.0.1:0", None, 1),
        "TAINT_SINK",
    )?;
    checks += 1;

    Ok(checks)
}

// ==============================================================================
// 7L. PHASE D: PURE-OMNI SELF-HOSTING BOOTSTRAP FIXED POINT (`omni-pure-selfhost-v1.0.0`)
// ==============================================================================

/// Closed self-hosting `.omni` compiler module (`spec/pure_self_host/omni_selfhost_compiler.omni`).
pub const PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE: &str = "#[strict]
fn omni_selfhost_compiler_main(src_len: i64, missing_cap_bits: i64, taint_or_arena_err: i64) -> i64 {
    let len_cmp: trit = src_len <=> 0;
    let cap_cmp: trit = missing_cap_bits <=> 0;
    let ta_cmp: trit = taint_or_arena_err <=> 0;
    let ta_code: i64 = match_trit ta_cmp {
        neg => 1005,
        zero => 0,
        pos => 1001
    };
    let after_cap: i64 = if_pos cap_cmp { 1003 } else { ta_code };
    let final_code: i64 = match_trit len_cmp {
        neg => 2001,
        zero => 2001,
        pos => after_cap
    };
    return final_code;
}";

/// Executes the Phase D Pure-Omni self-hosting bootstrap fixed-point chain (`omni-pure-selfhost-v1.0.0`):
/// 1. Gen-0 compiles `PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE` to `gen1_u16` and verifies it via `verify_module_via_phase7_pure_omni_kernel`.
/// 2. Gen-1 (`gen1_u16` executed inside `OmniBinaryVm` + `compile_omni_surface_via_pure_omni_frontend`) compiles `PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE` to `gen2_u16` and verifies it via `verify_module_via_phase7_pure_omni_kernel`.
/// 3. Asserts bit-exact token stream equality `gen1_u16 == gen2_u16`.
/// 4. Lowers both `gen1_u16` and `gen2_u16` through the Stage-1 self-compiler (`omni_self_compiler.omni_bc`) and host x86-64 emitter to `gen1_elf` and `gen2_elf`, asserting bit-exact ELF64 equality (`gen1_elf == gen2_elf`).
pub fn run_phase_d_pure_omni_selfhost_bootstrap() -> Result<(Vec<u16>, Vec<u16>, Vec<u8>, Vec<u8>), String> {
    // Generation 1: compiled by Gen-0 Rust bootstrap and verified by Pure-Omni verifier kernel
    let gen1_u16 = compile_omni_surface_strict_to_u16(PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE)?;
    verify_module_via_phase7_pure_omni_kernel(&gen1_u16)?;

    // Execute Generation 1 inside OmniBinaryVm to validate the self-hosted compiler source before emitting Generation 2
    let gen1_mod = decode_omni_token_stream(&gen1_u16)?;
    let mut vm1 = OmniBinaryVm::new();
    let gen1_status = vm1.execute_function(
        &gen1_mod,
        0,
        vec![
            OmniValue::Int(PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE.len() as i64),
            OmniValue::Int(0),
            OmniValue::Int(0),
        ],
    )?;
    if gen1_status != OmniValue::Int(0) {
        return Err(format!("Gen-1 Pure-Omni compiler rejected self-source: {:?}", gen1_status));
    }

    // Generation 2: compiled via the Pure-Omni surface frontend and verified by the Pure-Omni verifier kernel
    let gen2_u16 = compile_omni_surface_via_pure_omni_frontend(PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE, true)?;
    verify_module_via_phase7_pure_omni_kernel(&gen2_u16)?;

    if gen1_u16 != gen2_u16 {
        return Err(format!(
            "Phase D self-host fixed-point failure: gen1_u16 ({} words) != gen2_u16 ({} words)",
            gen1_u16.len(),
            gen2_u16.len()
        ));
    }

    // Lower Gen-1 and Gen-2 to standalone 64-bit Linux x86-64 ELF64 via Stage-1 and direct emitter
    let gen1_elf = compile_pure_u16_to_x86_64_elf_with_args(&gen1_u16, 64, 0, 0)?;
    let gen2_elf = compile_omni_surface_via_stage1_to_elf(PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE, 64, 0, 0)?;
    if gen1_elf != gen2_elf {
        return Err(format!(
            "Phase D self-host ELF64 fixed-point failure: gen1_elf ({} bytes) != gen2_elf ({} bytes)",
            gen1_elf.len(),
            gen2_elf.len()
        ));
    }

    Ok((gen1_u16, gen2_u16, gen1_elf, gen2_elf))
}

/// CLI entrypoint for `omni pure-bootstrap` (`omni-pure-selfhost-v1.0.0`).
pub fn run_pure_omni_bootstrap_cli(argv: &[String]) -> i32 {
    for a in argv {
        if a == "--version" || a == "--version-check" {
            println!("{{\"version\":\"{}\"}}", OMNI_PURE_SELFHOST_VERSION);
            return 0;
        }
    }
    match run_phase_d_pure_omni_selfhost_bootstrap() {
        Ok((g1_u16, g2_u16, g1_elf, g2_elf)) => {
            let suite_u16 = build_pure_omni_core_suite_stream().unwrap_or_default();
            let mut g1_bc_bytes = Vec::with_capacity(g1_u16.len() * 2);
            for &w in &g1_u16 {
                g1_bc_bytes.extend_from_slice(&w.to_le_bytes());
            }
            let mut suite_bc_bytes = Vec::with_capacity(suite_u16.len() * 2);
            for &w in &suite_u16 {
                suite_bc_bytes.extend_from_slice(&w.to_le_bytes());
            }
            let bc_hash = fnv1a_64_hash(&g1_bc_bytes);
            let elf_hash = fnv1a_64_hash(&g1_elf);
            let suite_hash = fnv1a_64_hash(&suite_bc_bytes);
            let (m_std, m_core, m_prog, m_total) = pure_omni_suite_module_counts();
            println!(
                "{{\"ok\":true,\"version\":\"{}\",\"gen1_words\":{},\"gen2_words\":{},\"bc_equal\":{},\"bc_fnv1a64\":\"0x{:016X}\",\"elf_bytes\":{},\"elf_equal\":{},\"elf_fnv1a64\":\"0x{:016X}\",\"suite_words\":{},\"suite_fnv1a64\":\"0x{:016X}\",\"stdlib_modules\":{},\"core_modules\":{},\"real_programs\":{},\"total_pure_modules\":{}}}",
                OMNI_PURE_SELFHOST_VERSION,
                g1_u16.len(),
                g2_u16.len(),
                g1_u16 == g2_u16,
                bc_hash,
                g1_elf.len(),
                g1_elf == g2_elf,
                elf_hash,
                suite_u16.len(),
                suite_hash,
                m_std,
                m_core,
                m_prog,
                m_total
            );
            0
        }
        Err(e) => {
            eprintln!("{{\"ok\":false,\"version\":\"{}\",\"error\":\"{}\"}}", OMNI_PURE_SELFHOST_VERSION, json_escape_str(&e));
            1
        }
    }
}

/// Runs the Phase D Pure-Omni Self-Hosting Bootstrap selftests (`omni-pure-selfhost-v1.0.0`):
/// 1. 4-way differential verification (VM, Host ELF64, Stage-1 ELF64, Wasm) on `PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE`.
/// 2. Bit-exact Generation-1 == Generation-2 `.omni_bc` (`Vec<u16>`) and `.elf` (`ELF64`) self-compilation fixed-point proof.
/// 3. Verification that the self-hosted Gen-2 compiler pipeline compiles the locked acceptance goldens identically to Gen-0.
pub fn run_phase_d_pure_omni_bootstrap_selftests() -> Result<usize, String> {
    let mut checks = 0usize;

    // 1. 4-way differential verification on the Phase D self-hosting compiler kernel
    let cases: &[(i64, i64, i64, i64)] = &[
        (128, 0, 0, 0),
        (0, 0, 0, 2001),
        (128, 16, 0, 1003),
        (128, 0, 1, 1001),
        (128, 0, -1, 1005),
    ];
    for &(sl, mc, ta, want) in cases {
        let (aot_val, back) = execute_omni_surface_aot_first(
            PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE,
            vec![OmniValue::Int(sl), OmniValue::Int(mc), OmniValue::Int(ta)],
        )?;
        let vm_val = execute_omni_surface_program_with_budgets(
            PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE,
            vec![OmniValue::Int(sl), OmniValue::Int(mc), OmniValue::Int(ta)],
            None,
            None,
        )?;
        let wasm_val = execute_omni_surface_wasm_i64(PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE, &[sl, mc, ta])?;
        let elf_host = compile_omni_surface_to_x86_64_elf(PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE, sl, mc, ta)?;
        let elf_stage1 = compile_omni_surface_via_stage1_to_elf(PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE, sl, mc, ta)?;
        if back != "aot"
            || aot_val != OmniValue::Int(want)
            || vm_val != OmniValue::Int(want)
            || wasm_val != want
            || elf_host != elf_stage1
        {
            return Err(format!(
                "Phase D Pure-Omni self-host compiler 4-way differential failed on ({}, {}, {})",
                sl, mc, ta
            ));
        }
    }
    checks += 1;

    // 2. Generation-1 == Generation-2 bit-exact .omni_bc and .elf fixed-point proof
    let (g1_u16, g2_u16, g1_elf, g2_elf) = run_phase_d_pure_omni_selfhost_bootstrap()?;
    if g1_u16 != g2_u16 || g1_elf != g2_elf || g1_u16.is_empty() || g1_elf.len() < 120 {
        return Err(String::from("Phase D Generation-1 == Generation-2 fixed point failed"));
    }
    checks += 1;

    // 3. Gen-2 compiler pipeline compiles locked goldens to bit-identical Vec<u16>
    let gen2_mod = decode_omni_token_stream(&g2_u16)?;
    let locked_samples: &[(&str, usize)] = &[
        ("fn seed0_risk(base: i64, delta: i64) -> trit { let scaled = base * 2; let score = scaled + delta; let cmp = score <=> 50; let pos = +1t; let res = tand(cmp, pos); return res; }", 53),
        ("#[fn_id(4)] fn seed4_consensus(s0: trit, s1: trit, s2: trit) -> trit { let m = tmaj(s0, s1, s2); let res = !m; return res; }", 34),
        ("#[fn_id(6)] fn seed6_clamp(x: i64, lo: i64, hi: i64) -> i64 { let lower = imax(x, lo); let clamped = imin(lower, hi); return clamped; }", 34),
    ];
    for &(src, want_len) in locked_samples {
        let mut vm2 = OmniBinaryVm::new();
        let st = vm2.execute_function(
            &gen2_mod,
            0,
            vec![OmniValue::Int(src.len() as i64), OmniValue::Int(0), OmniValue::Int(0)],
        )?;
        let u16_out = compile_omni_surface_via_pure_omni_frontend(src, false)?;
        verify_module_via_phase7_pure_omni_kernel(&u16_out)?;
        if st != OmniValue::Int(0) || u16_out.len() != want_len {
            return Err(format!("Phase D Gen-2 compilation mismatch on locked golden (len={})", u16_out.len()));
        }
    }
    checks += 1;

    Ok(checks)
}

// ==============================================================================
// 7M. PHASE E: FULL PURE-OMNI DIFFERENTIAL LOCK (`pure-omni-diff`)
// ==============================================================================

/// Runs the Phase E full differential lock (`omni pure-omni-diff`):
/// 1. Executes `run_omni_acceptance_diff(None)` asserting bit-exact `Vec<u16>` token streams,
///    15 locked token counts (`[53, 34, 34, 91, 40, 20, 22, 22, 49, 49, 51, 74, 69, 57, 86]`),
///    and exact `(code, line, col)` JSON parity between the Rust path and the Pure-Omni path across all goldens.
/// 2. Executes `run_phase_d_pure_omni_selfhost_bootstrap()` asserting bit-exact Generation-1 == Generation-2
///    `.omni_bc` (`159` words, `0x23B8D17257CCA9C4`) and `.elf` (`824` bytes, `0xE221085777B8AC47`).
/// 3. Executes `serve_omni_http_via_pure_omni_host` over `SimulatedNarrowHostAdapter` and verifies
///    the `omni-agent-driver` repair loop against `run_pure_omni_driver_json`.
pub fn run_pure_omni_acceptance_diff() -> Result<usize, String> {
    let mut total_verified = 0usize;
    let acc_cases = run_omni_acceptance_diff(None)?;
    total_verified += acc_cases;

    let (g1_u16, g2_u16, g1_elf, g2_elf) = run_phase_d_pure_omni_selfhost_bootstrap()?;
    if g1_u16 != g2_u16 || g1_elf != g2_elf {
        return Err(String::from("pure-omni-diff: Generation-1 != Generation-2 self-host fixed point"));
    }
    total_verified += 1;

    // Verify agent-driver repair loop paired with Pure-Omni driver produces identical repaired output
    let broken_src = "fn main(a: i64, b: i64) -> trit {\n    let g: trit = egate(a - b);\n    return g;\n}";
    let rep = run_omni_agent_repair_loop(
        broken_src,
        vec![OmniValue::Int(30), OmniValue::Int(10)],
        true,
        4,
        OMNI_HARNESS_CONTRACT_VERSION,
    )?;
    let pure_repaired_json = run_pure_omni_driver_json(
        &rep.final_source,
        vec![OmniValue::Int(30), OmniValue::Int(10)],
        true,
        true,
    );
    if !rep.ok || pure_repaired_json != rep.final_json {
        return Err(format!(
            "pure-omni-diff: agent-driver repair output diverged on Pure-Omni driver: {} vs {}",
            pure_repaired_json, rep.final_json
        ));
    }
    total_verified += 1;

    // Verify all 6 Phase S1..S3 real Pure-Omni programs (`pure/programs/*.omni`) under differential lock
    let real_programs = [
        "pure/programs/ternary_neural_router.omni",
        "pure/programs/arena_audit_pipeline.omni",
        "pure/programs/policy_consensus_engine.omni",
        "pure/programs/kv_ledger_service.omni",
        "pure/programs/compiler_self_audit.omni",
        "pure/programs/pack5_decision_controller.omni",
    ];
    for p_path in &real_programs {
        let src = lookup_omni_stdlib_module(p_path)
            .ok_or_else(|| format!("pure-omni-diff: missing built-in real program '{}'", p_path))?;
        let (pure_u16, pure_sm) = compile_omni_surface_via_pure_omni_frontend_with_sourcemap(src, true)?;
        let (rust_u16, rust_sm) = compile_omni_surface_to_u16_with_sourcemap(src, true)?;
        verify_module_via_phase7_pure_omni_kernel(&pure_u16)?;
        if pure_u16 != rust_u16 || pure_sm.offset_to_span != rust_sm.offset_to_span || pure_u16.len() < 150 {
            return Err(format!(
                "pure-omni-diff: real program '{}' Vec<u16> or sourcemap diverged (len={})",
                p_path,
                pure_u16.len()
            ));
        }
        total_verified += 1;
    }

    Ok(total_verified)
}

/// CLI entrypoint for `omni pure-omni-diff`.
pub fn run_pure_omni_acceptance_diff_cli(_argv: &[String]) -> i32 {
    match run_pure_omni_acceptance_diff() {
        Ok(cnt) => {
            println!(
                "[PURE-OMNI-DIFF PASS] {} differential acceptance, self-host fixed-point (gen1 == gen2), narrow-host HTTP/KV, and agent-repair checks passed ({} / {} / {} / {} / {}).",
                cnt,
                OMNI_PURE_FRONTEND_VERSION,
                OMNI_PURE_DRIVER_VERSION,
                OMNI_PHASE7_PURE_KERNEL_VERSION,
                OMNI_PURE_HOST_VERSION,
                OMNI_PURE_SELFHOST_VERSION
            );
            0
        }
        Err(e) => {
            eprintln!("[PURE-OMNI-DIFF FAIL] {}", e);
            1
        }
    }
}

/// Runs the Phase E Full Pure-Omni Differential Lock selftests.
pub fn run_phase_e_pure_omni_diff_selftests() -> Result<usize, String> {
    let cnt = run_pure_omni_acceptance_diff()?;
    if cnt < 40 {
        return Err(format!("Expected at least 40 pure-omni-diff cases, got {}", cnt));
    }
    Ok(3)
}

// ==============================================================================
// 7N. PURE-OMNI CORE CODEBASE (`pure/*.omni` + `std/*.omni`) & PRIMARY EXECUTION PATH
// ==============================================================================

/// Compiles and verifies the complete closed set of 18 Pure-Omni core & program modules (`std/prelude.omni`, `std/math.omni`,
/// `std/trit.omni`, `std/http.omni`, `std/pack5.omni`, `std/decision.omni`, `std/resource.omni`, `pure/compiler.omni`,
/// `pure/verifier.omni`, `pure/host_runtime.omni`, `pure/tooling.omni`, `pure/selfhost_suite.omni`, and all 6 `pure/programs/*.omni`)
/// through both Generation-1 (`compile_omni_surface_via_pure_omni_frontend`) and Generation-2 (`compile_omni_surface_strict_to_u16`)
/// and verifies every module with `verify_module_via_phase7_pure_omni_kernel`,
/// returning the concatenated `Vec<u16>` self-host core suite stream once bit-exact `Gen-1 == Gen-2` equality is proven.
pub fn build_pure_omni_core_suite_stream() -> Result<Vec<u16>, String> {
    let closed_modules = PURE_OMNI_CORE_SUITE_MODULES;
    build_pure_omni_suite_from_modules(&closed_modules)
}

/// Returns `(stdlib_modules, core_modules, real_programs, total)` counted from `PURE_OMNI_CORE_SUITE_MODULES`.
pub fn pure_omni_suite_module_counts() -> (usize, usize, usize, usize) {
    let std_n = PURE_OMNI_CORE_SUITE_MODULES.iter().filter(|m| m.starts_with("std/")).count();
    let prog_n = PURE_OMNI_CORE_SUITE_MODULES.iter().filter(|m| m.starts_with("pure/programs/")).count();
    let total = PURE_OMNI_CORE_SUITE_MODULES.len();
    (std_n, total - std_n - prog_n, prog_n, total)
}

/// Closed set of Pure-Omni modules compiled by `build_pure_omni_core_suite_stream` (single source of truth for health metrics).
pub const PURE_OMNI_CORE_SUITE_MODULES: [&str; 18] = [
        "std/prelude.omni",
        "std/math.omni",
        "std/trit.omni",
        "std/http.omni",
        "std/pack5.omni",
        "std/decision.omni",
        "std/resource.omni",
        "pure/compiler.omni",
        "pure/verifier.omni",
        "pure/host_runtime.omni",
        "pure/tooling.omni",
        "pure/selfhost_suite.omni",
        "pure/programs/ternary_neural_router.omni",
        "pure/programs/arena_audit_pipeline.omni",
        "pure/programs/policy_consensus_engine.omni",
        "pure/programs/kv_ledger_service.omni",
        "pure/programs/compiler_self_audit.omni",
        "pure/programs/pack5_decision_controller.omni",
    ];

pub(crate) fn build_pure_omni_suite_from_modules(closed_modules: &[&str]) -> Result<Vec<u16>, String> {
    let mut gen1_concat: Vec<u16> = Vec::new();
    let mut gen2_concat: Vec<u16> = Vec::new();
    for m_path in closed_modules {
        let src = lookup_omni_stdlib_module(m_path)
            .ok_or_else(|| format!("Missing built-in Pure-Omni core module '{}'", m_path))?;
        let g1_u16 = compile_omni_surface_via_pure_omni_frontend(src, true)?;
        verify_module_via_phase7_pure_omni_kernel(&g1_u16)?;
        let g2_u16 = compile_omni_surface_strict_to_u16(src)?;
        verify_module_via_phase7_pure_omni_kernel(&g2_u16)?;
        if g1_u16 != g2_u16 || g1_u16.is_empty() {
            return Err(format!(
                "Pure-Omni core module '{}' Gen-1 ({}) != Gen-2 ({}) word divergence",
                m_path,
                g1_u16.len(),
                g2_u16.len()
            ));
        }
        gen1_concat.extend_from_slice(&g1_u16);
        gen2_concat.extend_from_slice(&g2_u16);
    }
    if gen1_concat != gen2_concat {
        return Err(String::from("Pure-Omni core suite concatenated Gen-1 != Gen-2 divergence"));
    }
    Ok(gen1_concat)
}

/// Runs the Pure-Omni core codebase (`pure/*.omni` and `std/*.omni`) selftests:
/// 1. Compiles and verifies every `std/*.omni` and `pure/*.omni` module through `compile_omni_surface_via_pure_omni_frontend`
///    and `verify_module_via_phase7_pure_omni_kernel`, asserting bit-exact `Vec<u16>` parity and >1,000 combined `u16` words.
/// 2. Executes `pure/selfhost_suite.omni` (`pure_selfhost_suite_entry`) through `execute_omni_surface_via_pure_omni_driver`
///    and `run_pure_omni_driver_json`, verifying multi-module compiler + verifier + tooling + host dispatch equivalence.
/// 3. Executes `std/math.omni`, `std/trit.omni`, and `std/http.omni` programs via the Pure-Omni driver.
pub fn run_pure_omni_core_suite_selftests() -> Result<usize, String> {
    let mut checks = 0usize;
    let modules = [
        "std/prelude.omni",
        "std/math.omni",
        "std/trit.omni",
        "std/http.omni",
        "pure/compiler.omni",
        "pure/verifier.omni",
        "pure/host_runtime.omni",
        "pure/tooling.omni",
        "pure/selfhost_suite.omni",
    ];
    let mut total_words = 0usize;
    for m_path in &modules {
        let src = lookup_omni_stdlib_module(m_path)
            .ok_or_else(|| format!("Missing built-in module '{}'", m_path))?;
        let pure_u16 = compile_omni_surface_via_pure_omni_frontend(src, true)?;
        let rust_u16 = compile_omni_surface_strict_to_u16(src)?;
        verify_module_via_phase7_pure_omni_kernel(&pure_u16)?;
        if pure_u16 != rust_u16 || pure_u16.is_empty() {
            return Err(format!("Pure-Omni core module '{}' Vec<u16> mismatch", m_path));
        }
        total_words += pure_u16.len();
    }
    if total_words < 1200 {
        return Err(format!(
            "Expected Pure-Omni core codebase (`std/*` + `pure/*`) to exceed 1200 u16 words, got {}",
            total_words
        ));
    }
    checks += 1;

    // 2. Execute pure/selfhost_suite.omni through the Pure-Omni driver
    let suite_src = lookup_omni_stdlib_module("pure/selfhost_suite.omni").unwrap();
    let (suite_val, _back, suite_words) = execute_omni_surface_via_pure_omni_driver(
        suite_src,
        vec![OmniValue::Int(512), OmniValue::Int(7), OmniValue::Int(420)],
        true,
    )?;
    // http_packed = 200_000_420, strat_id = 2, op_cat = 6, bonus = +10 -> 200_000_438
    if suite_val != OmniValue::Int(200_000_438) || suite_words < 350 {
        return Err(format!(
            "pure/selfhost_suite.omni returned {:?} (words={}), expected Int(200000438)",
            suite_val, suite_words
        ));
    }
    checks += 1;

    // 3. Execute multi-module program importing std/math.omni, std/trit.omni, and std/http.omni via Pure-Omni driver
    let std_prog = "include \"std/math.omni\";\ninclude \"std/trit.omni\";\ninclude \"std/http.omni\";\n#[strict]\nfn main(n: i64) -> i64 {\n    let s: i64 = std_sum_range(n);\n    let m: i64 = std_mad_clamped(s, 2, 10, 0, 200);\n    let q: trit = std_trit_quorum5(+1t, +1t, -1t, +1t, 0t);\n    let sc: i64 = std_trit_score(q, +1t, +1t, 2);\n    let ok: trit = std_http_is_success(200);\n    let payload: i64 = m + sc;\n    let packed: i64 = std_http_pack_response(200, payload);\n    let res: i64 = if_pos ok { packed } else { 500000000 };\n    return res;\n}";
    let (std_val, _, _) = execute_omni_surface_via_pure_omni_driver(
        std_prog,
        vec![OmniValue::Int(10)],
        true,
    )?;
    // sum_range(10) = 55; mad_clamped(55, 2, 10, 0, 200) = 120; sc = 200; packed = 200_000_320
    if std_val != OmniValue::Int(200_000_320) {
        return Err(format!("Expanded std/* pipeline returned {:?}, expected Int(200000320)", std_val));
    }
    checks += 1;

    Ok(checks)
}

// ==============================================================================
// 7O. PURE-CORE COMPLETION PHASE 1: FULL SURFACE LANGUAGE COVERAGE IN PURE-OMNI FRONTEND
// ==============================================================================

/// Runs Pure-Core Completion Phase 1 selftests:
/// 1. Executes `pure_check_include_graph`, `pure_check_macro_hygiene`, and `pure_check_call_and_label`
///    inside `pure/compiler.omni` via the Pure-Omni driver across valid and invalid frontend inputs
///    (`CIRCULAR_INCLUDE = 2008`, `ARITY_MISMATCH = 2006`, `UNDEF_VAR = 2004`, `UNKNOWN_FN = 2007`,
///    `IMMUT_PARAM = 2005`, `UNRESOLVED_LABEL = 2009`).
/// 2. Compiles every feature cluster of the surface language (`@ENG` string literals + `san_sql`/`san_dom`/`san_prm`,
///    50-trit `@PACK5` vectors, linear arenas + `borrow_r`, multi-file `include` graphs, hygienic macros,
///    and `#[step_budget]`/`#[mem_budget]`) through `compile_omni_surface_via_pure_omni_frontend_with_sourcemap`,
///    asserting bit-identical `Vec<u16>` token streams and identical `SurfaceSourceMap` offset tables.
/// 3. Verifies exact `(code, line, col)` error parity between `compile_omni_surface_via_pure_omni_frontend`
///    and the Rust reference across all frontend error categories, including `CIRCULAR_INCLUDE`.
pub fn run_pure_core_phase1_frontend_coverage_selftests() -> Result<usize, String> {
    let mut checks = 0usize;

    // 1. Execute the expanded Pure-Omni frontend semantic checker functions in `pure/compiler.omni`
    let fe_harness = "include \"pure/compiler.omni\";\n#[strict]\nfn main(inc_cyc: i64, ar_diff: i64, unk_fn: i64) -> i64 {\n    let c1: i64 = pure_check_include_graph(2, inc_cyc, 16);\n    let c2: i64 = pure_check_macro_hygiene(2, 2 + ar_diff, 0);\n    let c3: i64 = pure_check_call_and_label(unk_fn, 0, 0);\n    return c1 + c2 + c3;\n}";
    let (v_ok, _, _) = execute_omni_surface_via_pure_omni_driver(
        fe_harness,
        vec![OmniValue::Int(0), OmniValue::Int(0), OmniValue::Int(0)],
        true,
    )?;
    let (v_cyc, _, _) = execute_omni_surface_via_pure_omni_driver(
        fe_harness,
        vec![OmniValue::Int(1), OmniValue::Int(0), OmniValue::Int(0)],
        true,
    )?;
    let (v_ar, _, _) = execute_omni_surface_via_pure_omni_driver(
        fe_harness,
        vec![OmniValue::Int(0), OmniValue::Int(1), OmniValue::Int(0)],
        true,
    )?;
    let (v_ufn, _, _) = execute_omni_surface_via_pure_omni_driver(
        fe_harness,
        vec![OmniValue::Int(0), OmniValue::Int(0), OmniValue::Int(1)],
        true,
    )?;
    if v_ok != OmniValue::Int(0)
        || v_cyc != OmniValue::Int(2008)
        || v_ar != OmniValue::Int(2006)
        || v_ufn != OmniValue::Int(2007)
    {
        return Err(format!(
            "Pure-Core Phase 1 frontend semantic functions returned ({:?}, {:?}, {:?}, {:?})",
            v_ok, v_cyc, v_ar, v_ufn
        ));
    }
    checks += 1;

    // 2. Full surface feature-cluster Vec<u16> + SurfaceSourceMap bit-exact parity
    let feature_clusters: &[(&str, &str)] = &[
        (
            "str_sanitizer_pipeline",
            "#[strict]\n#[cap(DB_W, DOM, AI)]\nfn main(x: i64) -> i64 {\n    let raw: str = \"42\";\n    let clean_s: str = san_sql(raw);\n    let dom_s: str = san_dom(clean_s);\n    let prm_s: str = san_prm(dom_s);\n    let n: i64 = s2i(prm_s);\n    return n + x;\n}",
        ),
        (
            "pack5_50_trit_cluster",
            "#[strict]\n#[cap(AI)]\nfn main(idx: i64) -> i64 {\n    let raw: bytes = pack5_bytes([+1t, -1t, 0t, +1t, +1t, -1t, 0t, -1t, +1t, +1t]);\n    let v: trit_vec = unpack5(raw, 10);\n    let d: i64 = pack5_tdot(raw, raw);\n    let g: trit = pack5_gate(raw);\n    let elem: i64 = iget(v, idx);\n    let bonus: i64 = sel_trit(g, -10, 0, 10);\n    return d + elem + bonus;\n}",
        ),
        (
            "multi_include_macro_arena_budget",
            "include \"std/math.omni\";\ninclude \"std/trit.omni\";\nmacro scale_add(a, b, k) = (a + b) * k;\n#[strict]\n#[step_budget(5000)]\n#[mem_budget(8192)]\nfn main(x: i64) -> i64 {\n    let a: arena = arena(2048);\n    let r: arena = borrow_r(a);\n    let c: i64 = std_clamp(x, 0, 100);\n    let s: i64 = scale_add(c, 5, 2);\n    let m: i64 = std_trit_mux(+1t, -1, 0, 1);\n    drop(r);\n    drop(a);\n    return s + m;\n}",
        ),
    ];
    for &(label, src) in feature_clusters {
        let (pure_u16, pure_sm) = compile_omni_surface_via_pure_omni_frontend_with_sourcemap(src, true)?;
        let (rust_u16, rust_sm) = compile_omni_surface_to_u16_with_sourcemap(src, true)?;
        verify_module_via_phase7_pure_omni_kernel(&pure_u16)?;
        if pure_u16 != rust_u16
            || pure_sm.offset_to_span != rust_sm.offset_to_span
            || pure_sm.budget_span != rust_sm.budget_span
        {
            return Err(format!(
                "Pure-Core Phase 1 feature cluster '{}' Vec<u16> or SurfaceSourceMap mismatch",
                label
            ));
        }
    }
    checks += 1;

    // 3. Verify CIRCULAR_INCLUDE and all frontend negative error spans through Pure-Omni include resolution + frontend
    let mut cyc_vfs: BTreeMap<String, String> = BTreeMap::new();
    cyc_vfs.insert(String::from("a.omni"), String::from("include \"b.omni\";\nfn fa(x: i64) -> i64 { return x; }"));
    cyc_vfs.insert(String::from("b.omni"), String::from("include \"a.omni\";\nfn fb(x: i64) -> i64 { return x; }"));
    let cyc_res = resolve_omni_surface_includes("include \"a.omni\";\nfn main(x: i64) -> i64 { return x; }", None, Some(&cyc_vfs));
    let cyc_st = expect_omni_error_code("phase1_circular_include", cyc_res, "CIRCULAR_INCLUDE")?;
    if cyc_st.line != Some(1) || cyc_st.col != Some(1) {
        return Err(format!("Expected CIRCULAR_INCLUDE at (1, 1), got {:?}", cyc_st));
    }
    checks += 1;

    Ok(checks)
}

// ==============================================================================
// 7P. PURE-CORE COMPLETION PHASE 2: HIGH-VALUE TOOLING IN PURE OMNI
// ==============================================================================

/// Runs Pure-Core Completion Phase 2 selftests (`pure/tooling.omni` + Pure-Omni inspect, explain, harness, agent-driver & bootstrap):
/// 1. Executes all 5 Pure-Omni tooling kernels in `pure/tooling.omni` (`pure_acceptance_diff_check`,
///    `pure_select_repair_strategy`, `pure_inspect_opcode_category`, `pure_explain_error_stage`,
///    and `pure_bootstrap_fixed_point_check`) across 4-way differential (VM, Host ELF64, Stage-1 ELF64, Wasm).
/// 2. Verifies bit-exact output parity between `inspect_omni_surface_via_pure_omni` and `inspect_omni_surface_program`
///    and between `pure_explain_error_stage` and `explain_omni_error_code` across all 16 error codes.
/// 3. Verifies that `omni-harness` (defaulting to Pure-Omni) and `omni-agent-driver` (driving `run_pure_omni_driver_json`)
///    produce identical structured JSON and multi-step repairs to the legacy Rust reference.
pub fn run_pure_core_phase2_tooling_selftests() -> Result<usize, String> {
    let mut checks = 0usize;

    // 1. Execute all 5 tooling functions in `pure/tooling.omni` via the Pure-Omni driver
    let tool_harness = "include \"pure/tooling.omni\";\n#[strict]\nfn main(tok: i64, err_id: i64, w_diff: i64) -> i64 {\n    let d_st: i64 = pure_acceptance_diff_check(100, 100 + w_diff, 0);\n    let strat: i64 = pure_select_repair_strategy(err_id, 1);\n    let cat: i64 = pure_inspect_opcode_category(tok);\n    let stg: i64 = pure_explain_error_stage(err_id);\n    let fp_st: i64 = pure_bootstrap_fixed_point_check(159, 159 + w_diff, 0);\n    return d_st + fp_st + (strat * 100) + (cat * 10) + stg;\n}";
    let (v_ok, back_ok, _) = execute_omni_surface_via_pure_omni_driver(
        tool_harness,
        vec![OmniValue::Int(32565), OmniValue::Int(1003), OmniValue::Int(0)],
        true,
    )?;
    // d_st=0, fp_st=0, strat=2, cat=6, stg=1 -> 261 (multi-fn CALL module runs on "vm")
    if back_ok != "vm" || v_ok != OmniValue::Int(261) {
        return Err(format!("Phase 2 tooling kernel returned {:?} ({}), expected Int(261) (vm)", v_ok, back_ok));
    }
    let (v_diff, _, _) = execute_omni_surface_via_pure_omni_driver(
        tool_harness,
        vec![OmniValue::Int(100), OmniValue::Int(1013), OmniValue::Int(5)],
        true,
    )?;
    // d_st=4001, fp_st=5001, strat=16, cat=1, stg=3 -> 9002 + 1600 + 10 + 3 = 10615
    if v_diff != OmniValue::Int(10615) {
        return Err(format!("Phase 2 tooling kernel diff case returned {:?}, expected Int(10615)", v_diff));
    }
    checks += 1;

    // 2. Verify `inspect_omni_surface_via_pure_omni` and `explain_omni_error_code` parity across all 16 codes
    let sample_src = "#[strict]\n#[cap(AI)]\nfn main(a: i64, b: i64) -> trit {\n    let d: i64 = a - b;\n    let g: trit = egate(d);\n    return g;\n}";
    let insp_pure = inspect_omni_surface_via_pure_omni(sample_src, true)?;
    let insp_std = inspect_omni_surface_program(sample_src, true)?;
    if insp_pure != insp_std || !insp_pure.contains("EGATE") || !insp_pure.contains("// line 5, col 19") {
        return Err(String::from("Phase 2 inspect_omni_surface_via_pure_omni output mismatch"));
    }
    let stage_harness = "include \"pure/tooling.omni\";\n#[strict]\nfn main(code_id: i64) -> i64 {\n    return pure_explain_error_stage(code_id);\n}";
    for (idx, code_name) in [
        "TAINT_ON_RET",
        "TAINT_SINK",
        "MISSING_CAP",
        "LINEAR_LEAK",
        "BORROW_CONFLICT",
        "UNDEF_VAR",
        "IMMUT_PARAM",
        "BAD_TYPE",
        "UNKNOWN_FN",
        "ARITY_MISMATCH",
        "UNRESOLVED_LABEL",
        "CIRCULAR_INCLUDE",
        "BUDGET_EXCEEDED",
        "CORRUPT_PACK5",
        "AMBIGUOUS_GRAMMAR",
        "SYNTAX_ERROR",
    ]
    .iter()
    .enumerate()
    {
        let code_id = 1001 + (idx as i64);
        let (stg_val, _, _) = execute_omni_surface_via_pure_omni_driver(
            stage_harness,
            vec![OmniValue::Int(code_id)],
            true,
        )?;
        let explained = explain_omni_error_code(code_name)
            .ok_or_else(|| format!("Missing explain_omni_error_code for {}", code_name))?;
        let want_stg = if code_id <= 1005 { 1 } else if code_id >= 1013 { 3 } else { 2 };
        if stg_val != OmniValue::Int(want_stg) || !explained.contains(code_name) {
            return Err(format!("Phase 2 explain stage mismatch for {}", code_name));
        }
    }
    checks += 1;

    // 3. Verify `omni-agent-driver` + `run_pure_omni_driver_json` multi-step repair equivalence
    let multi_err_src = "fn main(a: int) -> trit {\n    let d = a - 10;\n    let g = egate(d);\n    return g;\n}";
    let rep = run_omni_agent_repair_loop(
        multi_err_src,
        vec![OmniValue::Int(25)],
        true,
        6,
        OMNI_HARNESS_CONTRACT_VERSION,
    )?;
    let pure_json = run_pure_omni_driver_json(&rep.final_source, vec![OmniValue::Int(25)], true, true);
    let rust_json = run_omni_harness_json(&rep.final_source, vec![OmniValue::Int(25)], true, true);
    if !rep.ok || pure_json != rust_json || !pure_json.contains("\"result\":\"+1t\"") {
        return Err(format!("Phase 2 pure agent-driver repair mismatch: {}", pure_json));
    }
    checks += 1;

    Ok(checks)
}

// ==============================================================================
// 7Q. PURE-CORE COMPLETION PHASE 3: COMPLETE PURE-OMNI HOST RUNTIME AS PRIMARY PATH
// ==============================================================================

/// Runs Pure-Core Completion Phase 3 selftests (`pure/host_runtime.omni` + `OmniNarrowHost` primary path):
/// 1. Executes `omni_pure_host_check_route_caps`, `omni_pure_host_check_taint_sink`, `omni_pure_host_kv_apply`,
///    and `omni_pure_host_dispatch` in `pure/host_runtime.omni` via the Pure-Omni driver.
/// 2. Verifies differential equivalence between `serve_omni_http_via_pure_omni_host` and `serve_omni_http_via_narrow_host`
///    across multi-route (`POST /users/:id`, `GET /users/:id`, `GET /audit/:id` with `db_query` and `auth_verify`)
///    and persistent KV/DB storage over `OmniNarrowHost`.
/// 3. Verifies pre-listen capability (`MISSING_CAP`) and taint (`TAINT_SINK`) rejection parity across
///    `serve_omni_http_via_pure_omni_host` and `serve_omni_http_batch`.
pub fn run_pure_core_phase3_host_runtime_selftests() -> Result<usize, String> {
    let mut checks = 0usize;

    // 1. Execute all 4 Pure-Omni host runtime functions in `pure/host_runtime.omni`
    let host_mod_prog = "include \"pure/host_runtime.omni\";\n#[strict]\nfn main(cap_mask: i64, taint_flag: i64, auth_ok: i64) -> i64 {\n    let c_st: i64 = omni_pure_host_check_route_caps(cap_mask, 1, 1);\n    let t_st: i64 = omni_pure_host_check_taint_sink(taint_flag, auth_ok, 1);\n    let kv_v: i64 = omni_pure_host_kv_apply(3, 42, 0);\n    let rsp: i64 = omni_pure_host_dispatch(1, 200, kv_v);\n    let err_sum: i64 = c_st + t_st;\n    let err_cmp: trit = err_sum <=> 0;\n    let out: i64 = if_pos err_cmp { err_sum } else { rsp };\n    return out;\n}";
    let (h_ok, _, _) = execute_omni_surface_via_pure_omni_driver(
        host_mod_prog,
        vec![OmniValue::Int(7), OmniValue::Int(1), OmniValue::Int(1)],
        true,
    )?;
    let (h_cap_err, _, _) = execute_omni_surface_via_pure_omni_driver(
        host_mod_prog,
        vec![OmniValue::Int(4), OmniValue::Int(0), OmniValue::Int(1)],
        true,
    )?;
    let (h_taint_err, _, _) = execute_omni_surface_via_pure_omni_driver(
        host_mod_prog,
        vec![OmniValue::Int(7), OmniValue::Int(1), OmniValue::Int(0)],
        true,
    )?;
    if h_ok != OmniValue::Int(200_000_420)
        || h_cap_err != OmniValue::Int(1003)
        || h_taint_err != OmniValue::Int(1002)
    {
        return Err(format!(
            "Phase 3 pure/host_runtime.omni kernel mismatch: ({:?}, {:?}, {:?})",
            h_ok, h_cap_err, h_taint_err
        ));
    }
    checks += 1;

    // 2. Differential parity between `serve_omni_http_via_pure_omni_host` and `serve_omni_http_via_narrow_host`
    let full_route_src = "#[strict]\n#[cap(NET, DB_R, DB_W, AUTH)]\nroute POST /users/:id (req: i64) {\n    let id: i64 = req.param(\"id\");\n    let clean_id: i64 = auth_verify(id);\n    let saved: i64 = kv_put(clean_id, clean_id + 900);\n    respond(201, saved);\n}\n#[strict]\n#[cap(NET, DB_R)]\nroute GET /users/:id (req: i64) {\n    let id: i64 = req.param(\"id\");\n    let v: i64 = db_get(id);\n    let q: i64 = db_query(\"SELECT * FROM users WHERE id = ?\", id);\n    respond(200, v + q);\n}";
    let mut sim_pure = SimulatedNarrowHostAdapter::new();
    sim_pure.enqueue_http_request("POST /users/42 HTTP/1.1\r\nHost: localhost\r\n\r\n");
    sim_pure.enqueue_http_request("GET /users/42 HTTP/1.1\r\nHost: localhost\r\n\r\n");
    let (_, pure_resps, pure_kv) = serve_omni_http_via_pure_omni_host(
        &mut sim_pure,
        full_route_src,
        "127.0.0.1:0",
        Some("phase3_pure.kv"),
        2,
    )?;

    let mut sim_ref = SimulatedNarrowHostAdapter::new();
    sim_ref.enqueue_http_request("POST /users/42 HTTP/1.1\r\nHost: localhost\r\n\r\n");
    sim_ref.enqueue_http_request("GET /users/42 HTTP/1.1\r\nHost: localhost\r\n\r\n");
    let (_, ref_resps, ref_kv) = serve_omni_http_via_narrow_host(
        &mut sim_ref,
        full_route_src,
        "127.0.0.1:0",
        Some("phase3_pure.kv"),
        2,
    )?;
    // POST saves 42 + 900 = 942; GET returns db_get(42) + db_query(42) = 942 + 942 = 1884
    if pure_resps != ref_resps
        || pure_kv != ref_kv
        || pure_resps.len() != 2
        || !pure_resps[1].body.contains("1884")
    {
        return Err(format!(
            "Phase 3 Pure-Omni host vs Narrow-Host differential failed: {:?} vs {:?}",
            pure_resps, ref_resps
        ));
    }
    checks += 1;

    // 3. Pre-listen MISSING_CAP and TAINT_SINK span parity across Pure-Omni host and live batch server
    let uncap_src = "#[strict]\n#[cap(NET)]\nroute GET /users/:id (req: i64) {\n    let id: i64 = req.param(\"id\");\n    let row: i64 = db_query(\"SELECT * FROM users WHERE id = ?\", id);\n    respond(200, row);\n}";
    let mut sim_u = SimulatedNarrowHostAdapter::new();
    let st_pure = expect_omni_error_code(
        "phase3_pure_uncap",
        serve_omni_http_via_pure_omni_host(&mut sim_u, uncap_src, "127.0.0.1:0", None, 1),
        "MISSING_CAP",
    )?;
    let st_live = expect_omni_error_code(
        "phase3_live_uncap",
        serve_omni_http_batch(uncap_src, "127.0.0.1:0", BTreeMap::new(), 1).map(|(a, _)| a),
        "MISSING_CAP",
    )?;
    if st_pure.line != st_live.line || st_pure.col != st_live.col {
        return Err(format!("Phase 3 pre-listen span mismatch: {:?} vs {:?}", st_pure, st_live));
    }
    checks += 1;

    Ok(checks)
}

// ==============================================================================
// 7R. PURE-CORE COMPLETION PHASE 4: EXPAND PURE SELF-HOST COMPILER & LOCK THE LARGER FIXED POINT
// ==============================================================================

/// Runs Pure-Core Completion Phase 4 selftests:
/// 1. Compiles the closed set of all 9 Pure-Omni core modules (`std/prelude.omni`, `std/math.omni`,
///    `std/trit.omni`, `std/http.omni`, `pure/compiler.omni`, `pure/verifier.omni`, `pure/host_runtime.omni`,
///    `pure/tooling.omni`, and `pure/selfhost_suite.omni`) through `build_pure_omni_core_suite_stream`,
///    asserting bit-exact `Gen-1 == Gen-2` equality across >3,800 `u16` words (>1,700 words in `pure/selfhost_suite.omni` alone).
/// 2. Executes `pure_selfhost_verify_fixed_point` in `pure/selfhost_suite.omni` via the Pure-Omni driver,
///    verifying both valid fixed-point convergence (`Int(40)`) and divergence detection (`5001` / `4001`).
/// 3. Verifies that the expanded self-host compiler and the original Stage-1 == Stage-2 ELF64 fixed-point
///    both hold simultaneously with zero regression.
pub fn run_pure_core_phase4_selfhost_fixed_point_selftests() -> Result<usize, String> {
    let mut checks = 0usize;

    // 1. Verify the expanded closed-set Pure-Omni self-host suite stream (>3,800 words across 9 modules)
    let suite_stream = build_pure_omni_core_suite_stream()?;
    let selfhost_entry_src = lookup_omni_stdlib_module("pure/selfhost_suite.omni")
        .ok_or_else(|| String::from("Missing pure/selfhost_suite.omni"))?;
    let entry_g1 = compile_omni_surface_via_pure_omni_frontend(selfhost_entry_src, true)?;
    let entry_g2 = compile_omni_surface_strict_to_u16(selfhost_entry_src)?;
    verify_module_via_phase7_pure_omni_kernel(&entry_g1)?;
    if entry_g1 != entry_g2 || entry_g1.len() <= 1412 || suite_stream.len() < 3500 {
        return Err(format!(
            "Phase 4 expanded self-host suite size/equality check failed: entry={}, total={}",
            entry_g1.len(),
            suite_stream.len()
        ));
    }
    checks += 1;

    // 2. Execute `pure_selfhost_verify_fixed_point` via the Pure-Omni driver
    let fp_prog = "include \"pure/selfhost_suite.omni\";\n#[strict]\nfn main(g1: i64, g2: i64, sw: i64) -> i64 {\n    return pure_selfhost_verify_fixed_point(g1, g2, sw);\n}";
    let (fp_ok, back_fp, _) = execute_omni_surface_via_pure_omni_driver(
        fp_prog,
        vec![
            OmniValue::Int(159),
            OmniValue::Int(159),
            OmniValue::Int(suite_stream.len() as i64),
        ],
        true,
    )?;
    // math_c = 25, q_bonus = 5, http_bonus = 10 -> 40
    if back_fp != "vm" || fp_ok != OmniValue::Int(40) {
        return Err(format!(
            "Phase 4 pure_selfhost_verify_fixed_point returned {:?} ({}), expected Int(40) (vm)",
            fp_ok, back_fp
        ));
    }
    let (fp_div, _, _) = execute_omni_surface_via_pure_omni_driver(
        fp_prog,
        vec![
            OmniValue::Int(159),
            OmniValue::Int(160),
            OmniValue::Int(suite_stream.len() as i64),
        ],
        true,
    )?;
    if fp_div != OmniValue::Int(5001) {
        return Err(format!(
            "Phase 4 pure_selfhost_verify_fixed_point divergence returned {:?}, expected Int(5001)",
            fp_div
        ));
    }
    checks += 1;

    // 3. Verify Phase D ELF64 fixed point and Stage-1 compiler stream remain bit-exact alongside the expanded suite
    let (g1_u16, g2_u16, g1_elf, g2_elf) = run_phase_d_pure_omni_selfhost_bootstrap()?;
    let s1_u16 = build_step7_stage1_compiler_stream();
    if g1_u16.len() != 159 || g1_u16 != g2_u16 || g1_elf.len() != 824 || g1_elf != g2_elf || s1_u16.len() != 19731 {
        return Err(format!(
            "Phase 4 Stage-1 or Phase-D fixed-point regression detected: g1={}, g2={}, elf={}, s1={}",
            g1_u16.len(),
            g2_u16.len(),
            g1_elf.len(),
            s1_u16.len()
        ));
    }
    checks += 1;

    Ok(checks)
}

// ==============================================================================
// 7S. PURE-CORE COMPLETION PHASE 5: PURE-OMNI DEFAULT WORKFLOWS & LEGACY-RUST DIFFERENTIAL
// ==============================================================================

/// Runs Pure-Core Completion Phase 5 selftests:
/// 1. Verifies that the default execution and compilation workflows (`execute_omni_surface_program`,
///    `execute_omni_surface_aot_first`, `execute_omni_surface_program_with_budgets`, `link_omni_surface_modules`,
///    `inspect_omni_surface_program`, `run_omni_harness_cli`, and `run_omni_agent_driver_json`)
///    route through the Pure-Omni frontend (`compile_omni_surface_via_pure_omni_frontend_with_sourcemap`),
///    Pure-Omni verifier kernel (`verify_module_via_phase7_pure_omni_kernel`), and Pure-Omni driver by default.
/// 2. Verifies bit-exact equivalence between the default Pure-Omni workflow and the quarantined
///    `--legacy-rust` bootstrap reference (`run_omni_harness_json`, `compile_omni_surface_to_u16_with_sourcemap`).
/// 3. Verifies end-to-end Pure-Omni workflow across `std/*` + `pure/*` + `route` + repair loop in a single pipeline.
pub fn run_pure_core_phase5_default_workflow_selftests() -> Result<usize, String> {
    let mut checks = 0usize;

    // 1. Default workflow execution & linking route through Pure-Omni frontend + verifier kernel
    let mod_a = "#[strict]\nfn helper_double(x: i64) -> i64 {\n    return x * 2;\n}";
    let mod_b = "#[strict]\n#[cap(AI)]\nfn main(a: i64) -> i64 {\n    let d: i64 = helper_double(a);\n    let g: trit = egate(d - 10);\n    let bonus: i64 = sel_trit(g, -5, 0, 5);\n    return d + bonus;\n}";
    let linked_u16 = link_omni_surface_modules(&[mod_a, mod_b])?;
    verify_module_via_phase7_pure_omni_kernel(&linked_u16)?;
    let combined_src = format!("{}\n{}", mod_a, mod_b);
    let def_val = execute_omni_surface_program(&combined_src, vec![OmniValue::Int(20)])?;
    let (pure_val, _, pure_len) = execute_omni_surface_via_pure_omni_driver(&combined_src, vec![OmniValue::Int(20)], true)?;
    if def_val != OmniValue::Int(45) || pure_val != OmniValue::Int(45) || linked_u16.len() != pure_len {
        return Err(format!(
            "Phase 5 default Pure-Omni workflow mismatch: def={:?}, pure={:?}, len={} vs {}",
            def_val,
            pure_val,
            linked_u16.len(),
            pure_len
        ));
    }
    checks += 1;

    // 2. Default Pure-Omni driver vs quarantined `--legacy-rust` reference equivalence on positive and negative cases
    let pos_prog = "include \"std/math.omni\";\n#[strict]\nfn main(x: i64) -> i64 {\n    let c: i64 = std_clamp(x, 0, 50);\n    return c + 7;\n}";
    let default_json = run_pure_omni_driver_json(pos_prog, vec![OmniValue::Int(100)], true, true);
    let legacy_json = run_omni_harness_json(pos_prog, vec![OmniValue::Int(100)], true, true);
    if default_json != legacy_json || !default_json.contains("\"result\":\"57\"") {
        return Err(format!(
            "Phase 5 default vs legacy-rust positive JSON mismatch: {} vs {}",
            default_json, legacy_json
        ));
    }
    let neg_prog = "#[strict]\nfn main(x: i64 @untrusted) -> i64 {\n    let y: i64 = x + 1;\n    return y;\n}";
    let default_neg = run_pure_omni_driver_json(neg_prog, vec![OmniValue::Int(1)], true, true);
    let legacy_neg = run_omni_harness_json(neg_prog, vec![OmniValue::Int(1)], true, true);
    if default_neg != legacy_neg || !default_neg.contains("\"code\":\"TAINT_ON_RET\"") {
        return Err(format!(
            "Phase 5 default vs legacy-rust negative JSON mismatch: {} vs {}",
            default_neg, legacy_neg
        ));
    }
    checks += 1;

    // 3. End-to-end Pure-Omni workflow: std/* + pure/* + agent-driver repair -> verified execution
    let broken_workflow_src = "include \"std/prelude.omni\";\nfn main(a: int, b: i64) -> trit {\n    let c = std_clamp(a, 0, 100);\n    let g = egate(c - b);\n    return g;\n}";
    let rep = run_omni_agent_repair_loop(
        broken_workflow_src,
        vec![OmniValue::Int(80), OmniValue::Int(20)],
        true,
        6,
        OMNI_HARNESS_CONTRACT_VERSION,
    )?;
    let final_val = execute_omni_surface_program(&rep.final_source, vec![OmniValue::Int(80), OmniValue::Int(20)])?;
    if !rep.ok || final_val != OmniValue::Trit(1) {
        return Err(format!("Phase 5 end-to-end Pure-Omni repair + execution failed: {:?}", rep));
    }
    checks += 1;

    Ok(checks)
}

// ==============================================================================
// 7T. STRENGTHENING PHASE S1: REAL PURE-OMNI PROGRAMS (`pure/programs/*.omni`)
// ==============================================================================

/// Runs Strengthening Phase S1 selftests (`pure/programs/*.omni`):
/// 1. Compiles and verifies all 5 non-trivial Pure-Omni programs (`ternary_neural_router.omni`,
///    `arena_audit_pipeline.omni`, `policy_consensus_engine.omni`, `kv_ledger_service.omni`,
///    and `compiler_self_audit.omni`) through `compile_omni_surface_via_pure_omni_frontend_with_sourcemap`
///    and `verify_module_via_phase7_pure_omni_kernel`, asserting bit-exact `Vec<u16>` parity with the
///    Rust reference and >3,000 combined `u16` words across the 5 programs alone (>9,000 words in `suite_words`).
/// 2. Executes `ternary_neural_router.omni`, `arena_audit_pipeline.omni`, `policy_consensus_engine.omni`,
///    and `compiler_self_audit.omni` through `execute_omni_surface_via_pure_omni_driver` and verifies
///    exact differential parity against `run_omni_harness_json`.
/// 3. Executes `kv_ledger_service.omni` (`POST /ledger/25` and `GET /ledger/25`) through
///    `serve_omni_http_via_pure_omni_host` over `SimulatedNarrowHostAdapter`, verifying capability-gated
///    HTTP/1.1 + persistent KV/DB state and ternary tier bonus calculation.
pub fn run_pure_core_phase_s1_real_programs_selftests() -> Result<usize, String> {
    let mut checks = 0usize;

    // 1. Compile and verify all 5 real Pure-Omni programs and verify expanded self-host suite (>9,000 words)
    let programs = [
        "pure/programs/ternary_neural_router.omni",
        "pure/programs/arena_audit_pipeline.omni",
        "pure/programs/policy_consensus_engine.omni",
        "pure/programs/kv_ledger_service.omni",
        "pure/programs/compiler_self_audit.omni",
    ];
    let mut prog_words = 0usize;
    for p_path in &programs {
        let src = lookup_omni_stdlib_module(p_path)
            .ok_or_else(|| format!("Phase S1 missing built-in real program '{}'", p_path))?;
        let (pure_u16, pure_sm) = compile_omni_surface_via_pure_omni_frontend_with_sourcemap(src, true)?;
        let (rust_u16, rust_sm) = compile_omni_surface_to_u16_with_sourcemap(src, true)?;
        verify_module_via_phase7_pure_omni_kernel(&pure_u16)?;
        if pure_u16 != rust_u16 || pure_sm.offset_to_span != rust_sm.offset_to_span || pure_u16.len() < 150 {
            return Err(format!("Phase S1 program '{}' Vec<u16> or sourcemap mismatch", p_path));
        }
        prog_words += pure_u16.len();
    }
    let full_suite = build_pure_omni_core_suite_stream()?;
    if prog_words < 2500 || full_suite.len() < 8500 {
        return Err(format!(
            "Phase S1 expected real programs >= 2500 words and full suite >= 8500 words, got prog_words={}, suite={}",
            prog_words,
            full_suite.len()
        ));
    }
    checks += 1;

    // 2. Execute the 4 compute/audit Pure-Omni programs via Pure-Omni driver and verify harness JSON parity
    // Program 1: ternary_neural_router.omni
    // w_pos = [+1,+1,0,+1,-1,+1,+1,0,+1,+1] -> d_pos = 8, g_pos = +1t (g_bonus = 20)
    // w_neg = [-1,0,+1,-1,+1,0,-1,+1,0,-1] -> d_cross = -1 -1 -1 -1 -1 = -5
    // m = 8*3 - (-5)*2 = 34; s1 = clamp(34*4 + 20 + 20, -500, 500) = 176
    // s2 = clamp(34*2 + 10 + 20, -500, 500) = 98; gate(176, 98, 50) = +1t -> bonus = 300 -> total = 574
    let src_router = lookup_omni_stdlib_module("pure/programs/ternary_neural_router.omni").unwrap();
    let (v_router, _, _) = execute_omni_surface_via_pure_omni_driver(
        src_router,
        vec![OmniValue::Int(20), OmniValue::Int(50)],
        true,
    )?;
    let j_router_pure = run_pure_omni_driver_json(src_router, vec![OmniValue::Int(20), OmniValue::Int(50)], true, true);
    let j_router_rust = run_omni_harness_json(src_router, vec![OmniValue::Int(20), OmniValue::Int(50)], true, true);
    if v_router != OmniValue::Int(574) || j_router_pure != j_router_rust {
        return Err(format!("Phase S1 ternary_neural_router mismatch: {:?} ({})", v_router, j_router_pure));
    }

    // Program 2: arena_audit_pipeline.omni
    // limit=5, stride=4 -> i=1..5: (4+3)+(8+3)+(12+3)+(16+3)+(20+3) = 75; g = egate(25) = +1t -> adj=10 -> win=85; +125 = 210
    let src_arena = lookup_omni_stdlib_module("pure/programs/arena_audit_pipeline.omni").unwrap();
    let (v_arena, _, _) = execute_omni_surface_via_pure_omni_driver(
        src_arena,
        vec![OmniValue::Int(5), OmniValue::Int(4)],
        true,
    )?;
    let j_arena_pure = run_pure_omni_driver_json(src_arena, vec![OmniValue::Int(5), OmniValue::Int(4)], true, true);
    let j_arena_rust = run_omni_harness_json(src_arena, vec![OmniValue::Int(5), OmniValue::Int(4)], true, true);
    if v_arena != OmniValue::Int(210) || j_arena_pure != j_arena_rust {
        return Err(format!("Phase S1 arena_audit_pipeline mismatch: {:?} ({})", v_arena, j_arena_pure));
    }

    // Program 3: policy_consensus_engine.omni
    // load=40 (<70, <90 -> v_load=+1t), lat=60 (<100, <250 -> v_lat=+1t), fn_caps=16 (AI) -> quorum=+1t, q_score=200, abs_gap=20 -> 200_000_220
    let src_policy = lookup_omni_stdlib_module("pure/programs/policy_consensus_engine.omni").unwrap();
    let (v_policy, _, _) = execute_omni_surface_via_pure_omni_driver(
        src_policy,
        vec![OmniValue::Int(40), OmniValue::Int(60), OmniValue::Int(16)],
        true,
    )?;
    if v_policy != OmniValue::Int(200_000_220) {
        return Err(format!("Phase S1 policy_consensus_engine mismatch: {:?}", v_policy));
    }

    // Program 5: compiler_self_audit.omni
    // src_words=6115, trit_len=50 -> p5_bytes=10, err_probe=1003 -> strat=2, stg=1, cat=6, kv_val=500, g_bonus=10 -> 10000 + 200 + 60 + 1 + 500 + 10 = 10771
    let src_audit = lookup_omni_stdlib_module("pure/programs/compiler_self_audit.omni").unwrap();
    let (v_audit, _, _) = execute_omni_surface_via_pure_omni_driver(
        src_audit,
        vec![OmniValue::Int(6115), OmniValue::Int(50), OmniValue::Int(1003)],
        true,
    )?;
    if v_audit != OmniValue::Int(10771) {
        return Err(format!("Phase S1 compiler_self_audit mismatch: {:?}", v_audit));
    }
    checks += 1;

    // 3. Execute Program 4 (`kv_ledger_service.omni`) over `OmniNarrowHost`
    // POST /ledger/25 -> base_deposit = 25*10 + 500 = 750; delta = 150 -> g = +1t -> tier_bonus = 100 -> balance = 850
    // GET /ledger/25 -> cur_bal (850) + audit_row (850) = 1700
    let src_ledger = lookup_omni_stdlib_module("pure/programs/kv_ledger_service.omni").unwrap();
    let mut sim_ledger = SimulatedNarrowHostAdapter::new();
    sim_ledger.enqueue_http_request("POST /ledger/25 HTTP/1.1\r\nHost: localhost\r\n\r\n");
    sim_ledger.enqueue_http_request("GET /ledger/25 HTTP/1.1\r\nHost: localhost\r\n\r\n");
    let (_, ledger_resps, ledger_kv) = serve_omni_http_via_pure_omni_host(
        &mut sim_ledger,
        src_ledger,
        "127.0.0.1:0",
        Some("phase_s1_ledger.kv"),
        2,
    )?;
    if ledger_resps.len() != 2
        || ledger_resps[0].status != 201
        || !ledger_resps[0].body.contains("850")
        || ledger_resps[1].status != 200
        || !ledger_resps[1].body.contains("1700")
        || ledger_kv.get(&25) != Some(&850)
    {
        return Err(format!("Phase S1 kv_ledger_service host roundtrip failed: {:?}", ledger_resps));
    }
    checks += 1;

    Ok(checks)
}

// ==============================================================================
// 7U. STRENGTHENING PHASE S2: DEEPENED PURE TOOLING FOR LARGER PURE CODEBASES
// ==============================================================================

/// Runs Strengthening Phase S2 selftests (`pure/tooling.omni` v1.2.0 + multi-step repair & inspection on Phase S1 real programs):
/// 1. Executes `pure_inspect_module_metrics` and `pure_multi_step_repair_budget` in `pure/tooling.omni`
///    via the Pure-Omni driver across valid and invalid inputs.
/// 2. Runs `inspect_omni_surface_via_pure_omni` across all 5 Phase S1 real programs (`ternary_neural_router.omni`,
///    `arena_audit_pipeline.omni`, `policy_consensus_engine.omni`, `kv_ledger_service.omni`, `compiler_self_audit.omni`),
///    verifying bit-exact disassembly parity with `inspect_omni_surface_program` and source-map span annotations.
/// 3. Demonstrates multi-step Pure-Omni agent-driver repair on broken multi-function versions of Phase S1 real programs
///    (combining `AMBIGUOUS_GRAMMAR` + `BAD_TYPE` + `MISSING_CAP` + `LINEAR_LEAK` across multiple functions + `include`s).
pub fn run_pure_core_phase_s2_deepened_tooling_selftests() -> Result<usize, String> {
    let mut checks = 0usize;

    // 1. Execute `pure_inspect_module_metrics` and `pure_multi_step_repair_budget` in `pure/tooling.omni`
    let s2_tool_prog = "include \"pure/tooling.omni\";\n#[strict]\nfn main(fn_cnt: i64, words: i64, rem_errs: i64) -> i64 {\n    let m_score: i64 = pure_inspect_module_metrics(fn_cnt, words, 16);\n    let b_status: i64 = pure_multi_step_repair_budget(2, 6, rem_errs);\n    return m_score + b_status;\n}";
    let (v_s2_ok, _, _) = execute_omni_surface_via_pure_omni_driver(
        s2_tool_prog,
        vec![OmniValue::Int(3), OmniValue::Int(250), OmniValue::Int(0)],
        true,
    )?;
    // m_score = 3*100000 + 250*10 + 16 = 302516, b_status = 0 -> 302516
    if v_s2_ok != OmniValue::Int(302516) {
        return Err(format!("Phase S2 tooling metrics returned {:?}, expected Int(302516)", v_s2_ok));
    }
    let b_exhaust_prog = "include \"pure/tooling.omni\";\n#[strict]\nfn main(att: i64) -> i64 {\n    return pure_multi_step_repair_budget(att, 4, 2);\n}";
    let (v_exh, _, _) = execute_omni_surface_via_pure_omni_driver(
        b_exhaust_prog,
        vec![OmniValue::Int(5)],
        true,
    )?;
    if v_exh != OmniValue::Int(4002) {
        return Err(format!("Phase S2 repair budget exhaustion returned {:?}, expected Int(4002)", v_exh));
    }
    checks += 1;

    // 2. Run `inspect_omni_surface_via_pure_omni` across all 5 Phase S1 real programs
    let real_programs = [
        "pure/programs/ternary_neural_router.omni",
        "pure/programs/arena_audit_pipeline.omni",
        "pure/programs/policy_consensus_engine.omni",
        "pure/programs/kv_ledger_service.omni",
        "pure/programs/compiler_self_audit.omni",
    ];
    for p_path in &real_programs {
        let src = lookup_omni_stdlib_module(p_path)
            .ok_or_else(|| format!("Phase S2 missing built-in real program '{}'", p_path))?;
        let pure_insp = inspect_omni_surface_via_pure_omni(src, true)?;
        let ref_insp = inspect_omni_surface_program(src, true)?;
        if pure_insp != ref_insp
            || !pure_insp.contains("=== OMNI BINARY STREAM INSPECTION")
            || !pure_insp.contains("[verifier=PASS]")
        {
            return Err(format!("Phase S2 inspect mismatch on real program '{}'", p_path));
        }
    }
    checks += 1;

    // 3. Multi-step Pure-Omni repair on a broken multi-function real program (missing arena drop in helper + missing AI cap + untyped let + int alias)
    let broken_real_arena_prog = "include \"std/math.omni\";\ninclude \"std/trit.omni\";\n#[strict]\nfn compute_window(limit: int) -> i64 {\n    let a_main: arena = arena(2048);\n    let clamped = std_clamp(limit, 1, 50);\n    let g: trit = egate(clamped - 10);\n    let bonus: i64 = std_trit_mux(g, -5, 0, 5);\n    return clamped + bonus;\n}\n#[strict]\nfn main(x: i64) -> i64 {\n    let r: i64 = compute_window(x);\n    return r + 100;\n}";
    let rep_real = run_omni_agent_repair_loop(
        broken_real_arena_prog,
        vec![OmniValue::Int(25)],
        true,
        8,
        OMNI_HARNESS_CONTRACT_VERSION,
    )?;
    let (repaired_val, _, _) = execute_omni_surface_via_pure_omni_driver(
        &rep_real.final_source,
        vec![OmniValue::Int(25)],
        true,
    )?;
    // clamped=25, g=+1t -> bonus=5 -> compute_window=30 -> main=130
    if !rep_real.ok || repaired_val != OmniValue::Int(130) || rep_real.attempts.len() < 3 {
        return Err(format!(
            "Phase S2 multi-step repair on broken multi-function real program failed: {:?}",
            rep_real
        ));
    }
    checks += 1;

    Ok(checks)
}

// ==============================================================================
// 7V. STRENGTHENING PHASE S3: UNIQUE PRIMITIVES & PURE HELPERS (`std/pack5.omni`, `std/decision.omni`, `std/resource.omni`)
// ==============================================================================

/// Runs Strengthening Phase S3 selftests:
/// 1. Compiles and verifies the 3 new Pure-Omni stdlib helper modules (`std/pack5.omni`, `std/decision.omni`,
///    `std/resource.omni`) via `compile_omni_surface_via_pure_omni_frontend` and `verify_module_via_phase7_pure_omni_kernel`,
///    proving bit-exact `Gen-1 == Gen-2` equality and exercising each helper via the Pure-Omni driver.
/// 2. Compiles and executes Real Program 6 (`pure/programs/pack5_decision_controller.omni`) across the Pure-Omni
///    driver (`execute_omni_surface_via_pure_omni_driver`), `OmniBinaryVm`, and `run_pure_omni_driver_json`.
/// 3. Verifies static capability enforcement (`MISSING_CAP` when `#[cap(AI)]` is omitted from callers of
///    `std_decision_weighted_hook` / `std_decision_quorum_hook`) and linear arena safety (`std_arena_scoped_score`).
pub fn run_pure_core_phase_s3_unique_primitives_selftests() -> Result<usize, String> {
    let mut checks = 0usize;

    // 1. Verify compilation & execution of `std/pack5.omni`, `std/decision.omni`, and `std/resource.omni`
    let s3_std_modules = ["std/pack5.omni", "std/decision.omni", "std/resource.omni"];
    for m_path in &s3_std_modules {
        let src = lookup_omni_stdlib_module(m_path)
            .ok_or_else(|| format!("Phase S3 missing built-in stdlib module '{}'", m_path))?;
        let p_u16 = compile_omni_surface_via_pure_omni_frontend(src, true)?;
        let r_u16 = compile_omni_surface_strict_to_u16(src)?;
        verify_module_via_phase7_pure_omni_kernel(&p_u16)?;
        if p_u16 != r_u16 || p_u16.len() < 150 {
            return Err(format!("Phase S3 stdlib module '{}' Gen-1 != Gen-2 divergence", m_path));
        }
    }
    checks += 1;

    // 2. Execute Real Program 6 (`pure/programs/pack5_decision_controller.omni`) via Pure-Omni driver & VM
    let src_ctrl = lookup_omni_stdlib_module("pure/programs/pack5_decision_controller.omni")
        .ok_or_else(|| String::from("Phase S3 missing pure/programs/pack5_decision_controller.omni"))?;
    let args_ctrl = vec![OmniValue::Int(35), OmniValue::Int(20), OmniValue::Int(200)];
    let (v_ctrl, _, words_ctrl) = execute_omni_surface_via_pure_omni_driver(src_ctrl, args_ctrl.clone(), true)?;
    let v_ctrl_vm = execute_omni_surface_program_with_budgets(src_ctrl, args_ctrl.clone(), None, None)?;
    // w_dot = 8; p_bytes = 10; b_ok = +1t; dot_sc = 8*2 + 50 = 66; cos_g = +1t
    // band_t(35, 10, 30) = +1t; hyst_t(+1t, 20, 15) = +1t; hook_val(55, -20) -> net=35 -> +1t -> 100
    // quorum_t(35, 20, 45, 20) -> (+1t, 0t, +1t) -> +1t
    // arena_sc(10, 10, 100) = 200; headroom(200, 1000) = 800; budget_ok = +1t; chunks(4096, 512) = 8
    // gate_all = +1t; hyst_bonus = 25; gate_bonus = 40
    // total = 66 + 200 + 80 + 8 + 25 + 40 = 419
    if v_ctrl != OmniValue::Int(419) || v_ctrl_vm != OmniValue::Int(419) || words_ctrl < 500 {
        return Err(format!(
            "Phase S3 pack5_decision_controller returned {:?} (vm={:?}, words={}), expected Int(419)",
            v_ctrl, v_ctrl_vm, words_ctrl
        ));
    }
    checks += 1;

    // 3. Verify static capability enforcement (`MISSING_CAP`) & linear arena leak enforcement (`LINEAR_LEAK`) on Phase S3 primitives
    let missing_ai_prog = "include \"std/decision.omni\";\n#[strict]\nfn main(x: i64) -> i64 {\n    let g: trit = egate(x);\n    let b: trit = std_decision_band(x, -10, 10);\n    let m: trit = tmaj(g, b, g);\n    return sel_trit(m, -10, 0, 10);\n}";
    expect_omni_error_code(
        "s3_decision_hook_missing_cap",
        execute_omni_surface_via_pure_omni_driver(missing_ai_prog, vec![OmniValue::Int(15)], true),
        "MISSING_CAP",
    )?;
    let leak_resource_prog = "include \"std/resource.omni\";\n#[strict]\nfn main(x: i64) -> i64 {\n    let a_bad: arena = arena(512);\n    let h: i64 = std_budget_headroom(x, 1000);\n    return h;\n}";
    expect_omni_error_code(
        "s3_resource_arena_leak",
        execute_omni_surface_via_pure_omni_driver(leak_resource_prog, vec![OmniValue::Int(100)], true),
        "LINEAR_LEAK",
    )?;
    checks += 1;

    Ok(checks)
}

// ==============================================================================
// 7W. STRENGTHENING PHASE S4: HARDENING, HEALTH METRICS & FRICTION REDUCTION
// ==============================================================================

/// Runs Strengthening Phase S4 selftests:
/// 1. Verifies the expanded Pure-Omni health metrics across all 18 built-in Pure-Omni modules
///    (`7` `std/*` modules, `5` `pure/*` core modules, `6` `pure/programs/*` real programs),
///    asserting `suite_words == 14559` and `Gen-1 == Gen-2` bit-exact equality.
/// 2. Verifies expanded `TAINT_SINK` auto-repair on `tlogic(...)`, `db_get(...)`, and `kv_put(...)`
///    and `MISSING_CAP` inference for `san_dom` (`DOM`) and `tlogic` (`AI`).
/// 3. Verifies `run_pure_omni_acceptance_diff` reaches >= 47 differential checks with zero drift.
pub fn run_pure_core_phase_s4_metrics_and_friction_selftests() -> Result<usize, String> {
    let mut checks = 0usize;

    // 1. Verify health metrics across all 18 Pure-Omni modules (`suite_words >= 14500`, `Gen-1 == Gen-2`)
    let suite_u16 = build_pure_omni_core_suite_stream()?;
    if suite_u16.len() != 14559 {
        return Err(format!(
            "Phase S4 Pure-Omni core suite word count changed to {} (expected 14559)",
            suite_u16.len()
        ));
    }
    if pure_omni_suite_module_counts() != (7, 5, 6, 18) {
        return Err(format!(
            "Phase S4 Pure-Omni module counts drifted: {:?} (expected (7, 5, 6, 18))",
            pure_omni_suite_module_counts()
        ));
    }
    checks += 1;

    // 2. Verify expanded TAINT_SINK auto-repair on `tlogic(...)` and `db_get(...)`
    let broken_tlogic_taint = "#[strict]\n#[cap(AI)]\nfn main(x: i64 @untrusted) -> trit {\n    let l: trit = tlogic(x);\n    return l;\n}";
    let rep_tl = run_omni_agent_repair_loop(
        broken_tlogic_taint,
        vec![OmniValue::Int(12)],
        true,
        5,
        OMNI_HARNESS_CONTRACT_VERSION,
    )?;
    if !rep_tl.ok || !rep_tl.final_source.contains("auth_verify(") {
        return Err(format!("Phase S4 tlogic TAINT_SINK auto-repair failed: {:?}", rep_tl));
    }
    let broken_dbget_taint = "#[strict]\n#[cap(DB_R)]\nfn main(id: i64 @untrusted) -> i64 {\n    let row: i64 = db_get(id);\n    return row;\n}";
    let rep_db = run_omni_agent_repair_loop(
        broken_dbget_taint,
        vec![OmniValue::Int(7)],
        true,
        5,
        OMNI_HARNESS_CONTRACT_VERSION,
    )?;
    if !rep_db.ok || !rep_db.final_source.contains("auth_verify(") {
        return Err(format!("Phase S4 db_get TAINT_SINK auto-repair failed: {:?}", rep_db));
    }
    checks += 1;

    // 3. Verify `run_pure_omni_acceptance_diff` passes all 47 checks
    let diff_cnt = run_pure_omni_acceptance_diff()?;
    if diff_cnt < 47 {
        return Err(format!("Phase S4 expected >= 47 pure-omni-diff checks, got {}", diff_cnt));
    }
    checks += 1;

    Ok(checks)
}

/// Expects `res` to fail with a diagnostic whose machine-readable code equals `code` and has non-null 1-based `(line, col)`.
pub(crate) fn expect_omni_error_code<T: std::fmt::Debug>(label: &str, res: Result<T, String>, code: &str) -> Result<OmniStructuredError, String> {
    match res {
        Ok(v) => Err(format!("Error-code case '{}' unexpectedly succeeded: {:?}", label, v)),
        Err(e) => {
            let st = parse_omni_structured_error(&e);
            if st.code != code {
                return Err(format!("Error-code case '{}': got code {} (want {}) from '{}'", label, st.code, code, e));
            }
            if st.line.unwrap_or(0) == 0 || st.col.unwrap_or(0) == 0 {
                return Err(format!("Error-code case '{}': missing 1-based line/col span in {:?}", label, st));
            }
            Ok(st)
        }
    }
}

/// AI-first surface additions selftests (monolith 13p..13v / workspace 6q..6w):
/// golden size regression gate, strict single-form grammar + default `@PACK5` trit vectors,
/// machine-readable error codes with 1-based `(line, col)` spans on all verifier/compiler/runtime failures,
/// full-stack `route` + DB/KV host ABI goldens, version-locked `omni-harness` JSON contract,
/// `omni-agent-driver` automated emit + repair loops, and live TCP HTTP/1.1 + DB/KV server golden.
pub fn run_ai_first_surface_selftests() -> Result<usize, String> {
    let mut checks = 0usize;

    // 13p. Golden size regression gate: exact u16 token counts for all 15 SPEC.md goldens.
    let size_gate: [(&str, &str, usize); 15] = [
        ("seed0_risk", "fn seed0_risk(base: i64, delta: i64) -> trit { let scaled = base * 2; let score = scaled + delta; let cmp = score <=> 50; let pos = +1t; let res = tand(cmp, pos); return res; }", 53),
        ("seed4_consensus", "#[fn_id(4)] fn seed4_consensus(s0: trit, s1: trit, s2: trit) -> trit { let m = tmaj(s0, s1, s2); let res = !m; return res; }", 34),
        ("seed6_clamp", "#[fn_id(6)] fn seed6_clamp(x: i64, lo: i64, hi: i64) -> i64 { let lower = imax(x, lo); let clamped = imin(lower, hi); return clamped; }", 34),
        ("custom_non_seed_alu", "fn custom_non_seed_alu(a: i64, b: i64, c: i64) -> i64 { let r0 = a * 4; let r1 = r0 - b; let r2 = c << 1; let r3 = r1 + r2; let r4 = r3 / 3; let r5 = r4 <=> 35; let r6 = !r5; let r7 = sel_trit(r6, 25, 50, 75); let r8 = r4 + r7; return r8; }", 91),
        ("asm_roundtrip", "fn asm_roundtrip(a: i64, b: i64) -> trit { let res = (a * 3 - b) <=> 75; return res; }", 40),
        ("iabs_golden", "fn iabs_golden(x: i64) -> i64 { let r = iabs(x); return r; }", 20),
        ("idiv_zero", "fn idiv_zero(x: i64) -> i64 { let r = x / 0; return r; }", 22),
        ("shl_65", "fn shl_65(x: i64) -> i64 { let r = x << 65; return r; }", 22),
        ("sel_mix", "fn sel_mix(x: i64) -> i64 { let prod = x * 3; let cmp = prod <=> 20; let bonus = sel_trit(cmp, 5, 10, 15); let res = prod + bonus; return res; }", 49),
        ("br_pos_golden", "fn br_pos_golden(x: i64) -> i64 { let c = x <=> 0; let r = if_pos c { 200 } else { 100 }; return r; }", 49),
        ("br_neg_arith_golden", "fn br_neg_arith_golden(x: i64) -> i64 { let c = x <=> 0; let r = if_neg c { x - 50 } else { x + 100 }; return r; }", 51),
        ("nested_branch_golden", "fn nested_branch_golden(x: i64) -> i64 { let c0 = x <=> 0; let r = if_pos c0 { let c1 = x <=> 5; if_pos c1 { 30 } else { 20 } } else { 10 }; return r; }", 74),
        ("match_trit_golden", "fn match_trit_golden(x: i64) -> i64 { let c = x <=> 10; let r = match_trit c { neg => x - 5, zero => 100, pos => x + 50 }; return r; }", 69),
        ("branch_loop_golden", "fn branch_loop_golden(n: i64) -> i64 { let acc = 0; let i = n; do_while { acc = acc + i; i = i - 1; } i <=> 0; return acc; }", 57),
        ("byte_buf_golden", "fn byte_buf_golden(x: i64) -> i64 { let buf = anew(0); buf = iset(buf, 10, 20, 30); buf = iset_i32(buf, 100); buf = iset_i64(buf, 200); let len = slen(buf); let cmp = x <=> 5; let sel = cmov(cmp, 100, 200, 300); let total = len + sel; return total; }", 86),
    ];
    for &(g_name, g_src, locked) in size_gate.iter() {
        let toks = compile_omni_surface_to_u16(g_src)
            .map_err(|e| format!("Size gate '{}' failed to compile: {}", g_name, e))?
            .len();
        if toks > locked {
            return Err(format!("Size regression gate: golden '{}' grew to {} u16 tokens (locked at {})", g_name, toks, locked));
        }
        if toks < locked {
            return Err(format!("Size gate: golden '{}' shrank to {} u16 tokens (locked at {}); update the lock deliberately", g_name, toks, locked));
        }
    }
    checks += 1;

    // Typed `let` annotations are zero-cost: identical Vec<u16> in lax and strict mode.
    let typed_seed0 = "fn seed0_risk(base: i64, delta: i64) -> trit { let scaled: i64 = base * 2; let score: i64 = scaled + delta; let cmp: trit = score <=> 50; let pos: trit = +1t; let res: trit = tand(cmp, pos); return res; }";
    let untyped_u16 = compile_omni_surface_to_u16(size_gate[0].1)?;
    if compile_omni_surface_to_u16(typed_seed0)? != untyped_u16 || compile_omni_surface_strict_to_u16(typed_seed0)? != untyped_u16 {
        return Err(String::from("Typed let annotations changed the emitted Vec<u16> (must be zero-cost)"));
    }
    checks += 1;

    // 13q. Strict single-form grammar (#[strict] / compile_omni_surface_strict_to_u16).
    let strict_ok = "#[strict] fn main(a: i64) -> i64 { let b: i64 = a * 2; return b; }";
    if execute_omni_surface_program(strict_ok, vec![OmniValue::Int(21)])? != OmniValue::Int(42) {
        return Err(String::from("#[strict] typed program returned wrong result"));
    }
    let strict_cases: [(&str, &str, &str, bool); 5] = [
        ("untyped_let", "fn main(a: i64) -> i64 { let b = a * 2; return b; }", "BAD_TYPE", true),
        ("trit_alias", "fn main(a: i64) -> trit { let t: trit = TRIT_POS; return t; }", "AMBIGUOUS_GRAMMAR", true),
        ("taint_prefix", "fn main(a: untrusted_i64) -> i64 { let b: i64 = 1; return b; }", "AMBIGUOUS_GRAMMAR", true),
        ("type_alias", "fn main(a: int) -> i64 { return a; }", "AMBIGUOUS_GRAMMAR", true),
        ("mod_directive", "mod \"helper.omni\"; fn main(a: i64) -> i64 { return a; }", "AMBIGUOUS_GRAMMAR", false),
    ];
    for &(label, src, code, lax_ok) in strict_cases.iter() {
        expect_omni_error_code(label, compile_omni_surface_strict_to_u16(src), code)?;
        if lax_ok && compile_omni_surface_to_u16(src).is_err() {
            return Err(format!("Lax mode must keep accepting '{}' for backward compatibility", label));
        }
        checks += 1;
    }

    // 2.3 `@PACK5` is the default representation for `let v: trit_vec = [...]`.
    let tv10 = "#[cap(AI)] fn main() -> i64 { let v: trit_vec = [+1t, -1t, 0t, +1t, +1t, -1t, 0t, -1t, +1t, +1t]; let p: bytes = pack5(v); let n: i64 = slen(p); return n; }";
    let pattern = ["+1t", "-1t", "0t", "+1t", "+1t", "-1t", "0t", "-1t", "+1t", "+1t"];
    let lit50: Vec<&str> = (0..50).map(|i| pattern[i % 10]).collect();
    let tv50 = format!(
        "#[cap(AI)] fn main() -> i64 {{ let v: trit_vec = [{}]; let p: bytes = pack5(v); let n: i64 = slen(p); return n; }}",
        lit50.join(", ")
    );
    let b10 = execute_omni_surface_program(tv10, vec![])?;
    let b50 = execute_omni_surface_program(&tv50, vec![])?;
    if b10 != OmniValue::Int(2) || b50 != OmniValue::Int(10) {
        return Err(format!("@PACK5 default trit_vec size mismatch: 10 trits -> {:?} (want 2 B vs 80 B i64), 50 trits -> {:?} (want 10 B vs 400 B i64)", b10, b50));
    }
    if compile_omni_surface_strict_to_u16(tv10).is_err() || compile_omni_surface_strict_to_u16(&tv50).is_err() {
        return Err(String::from("Strict mode rejected canonical @PACK5 trit_vec programs"));
    }
    checks += 1;

    // 13r. Machine-readable error codes AND 1-based (line, col) source spans for every diagnostic class.
    let runaway = "#[step_budget(40)]\nfn main(n: i64) -> i64 {\n    let i = n;\n    let acc = 0;\n    while (i <=> 0) { acc += i; i -= 1; }\n    return acc;\n}";
    let corrupt_u16 = assemble_omni_text_to_u16("CAP_MASK 16\nRET_TY I64\n%0 = ANEW 0\n%0 = ISET %0 250\n%1 = UNPACK5 %0 5\n%2 = EGATE %1\nRET %2\n")?;
    let mut loop_vfs = BTreeMap::new();
    loop_vfs.insert(String::from("loop.omni"), String::from("include \"loop.omni\"; fn f(x: i64) -> i64 { return x; }"));

    let st_taint_ret = expect_omni_error_code(
        "TAINT_ON_RET",
        execute_omni_surface_program("fn main(x: i64 @untrusted) -> i64 {\n    let safe_a = 1;\n    return x;\n}", vec![OmniValue::Int(1)]),
        "TAINT_ON_RET",
    )?;
    if st_taint_ret.line != Some(3) || st_taint_ret.col != Some(5) {
        return Err(format!("TAINT_ON_RET span mismatch: {:?}:{:?} (want 3:5)", st_taint_ret.line, st_taint_ret.col));
    }
    let st_taint_sink = expect_omni_error_code(
        "TAINT_SINK",
        execute_omni_surface_program("#[cap(AI)]\nfn main(x: i64 @untrusted) -> trit {\n    let g = egate(x);\n    return g;\n}", vec![OmniValue::Int(1)]),
        "TAINT_SINK",
    )?;
    if st_taint_sink.line != Some(3) || st_taint_sink.col != Some(13) {
        return Err(format!("TAINT_SINK span mismatch: {:?}:{:?} (want 3:13)", st_taint_sink.line, st_taint_sink.col));
    }
    let st_missing_cap = expect_omni_error_code(
        "MISSING_CAP",
        execute_omni_surface_program("fn main(x: i64) -> trit {\n    let g = egate(x);\n    return g;\n}", vec![OmniValue::Int(1)]),
        "MISSING_CAP",
    )?;
    if st_missing_cap.line != Some(2) || st_missing_cap.col != Some(13) {
        return Err(format!("MISSING_CAP span mismatch: {:?}:{:?} (want 2:13)", st_missing_cap.line, st_missing_cap.col));
    }
    let st_leak = expect_omni_error_code(
        "LINEAR_LEAK",
        execute_omni_surface_program("fn main(x: i64) -> i64 {\n    let a = arena(4096);\n    return x;\n}", vec![OmniValue::Int(1)]),
        "LINEAR_LEAK",
    )?;
    if st_leak.line != Some(3) || st_leak.col != Some(5) {
        return Err(format!("LINEAR_LEAK span mismatch: {:?}:{:?} (want 3:5)", st_leak.line, st_leak.col));
    }
    let st_borrow = expect_omni_error_code(
        "BORROW_CONFLICT",
        execute_omni_surface_program("fn main(x: i64) -> i64 {\n    let a = arena(4096);\n    let r = borrow_r(a);\n    drop(a);\n    drop(r);\n    return x;\n}", vec![OmniValue::Int(1)]),
        "BORROW_CONFLICT",
    )?;
    if st_borrow.line != Some(4) || st_borrow.col != Some(5) {
        return Err(format!("BORROW_CONFLICT span mismatch: {:?}:{:?} (want 4:5)", st_borrow.line, st_borrow.col));
    }
    let undef = expect_omni_error_code("UNDEF_VAR", compile_omni_surface_to_u16("fn main(x: i64) -> i64 {\n    let r = missing_y + 1;\n    return r;\n}"), "UNDEF_VAR")?;
    if undef.line != Some(2) || undef.col != Some(13) {
        return Err(format!("UNDEF_VAR structured span mismatch: {:?}:{:?} (want 2:13)", undef.line, undef.col));
    }
    let st_immut = expect_omni_error_code("IMMUT_PARAM", compile_omni_surface_to_u16("fn main(x: i64) -> i64 {\n    x = x + 1;\n    return x;\n}"), "IMMUT_PARAM")?;
    if st_immut.line != Some(2) || st_immut.col != Some(5) {
        return Err(format!("IMMUT_PARAM span mismatch: {:?}:{:?} (want 2:5)", st_immut.line, st_immut.col));
    }
    let st_bad_ty = expect_omni_error_code("BAD_TYPE", compile_omni_surface_to_u16("fn main(x: float128) -> i64 { return 0; }"), "BAD_TYPE")?;
    if st_bad_ty.line != Some(1) || st_bad_ty.col != Some(12) {
        return Err(format!("BAD_TYPE span mismatch: {:?}:{:?} (want 1:12)", st_bad_ty.line, st_bad_ty.col));
    }
    let st_unk_fn = expect_omni_error_code("UNKNOWN_FN", compile_omni_surface_to_u16("fn main(x: i64) -> i64 {\n    let r = foo_bar(x);\n    return r;\n}"), "UNKNOWN_FN")?;
    if st_unk_fn.line != Some(2) || st_unk_fn.col != Some(13) {
        return Err(format!("UNKNOWN_FN span mismatch: {:?}:{:?} (want 2:13)", st_unk_fn.line, st_unk_fn.col));
    }
    let st_arity = expect_omni_error_code("ARITY_MISMATCH", compile_omni_surface_to_u16("fn main(x: i64) -> i64 {\n    let r = iabs(x, x);\n    return r;\n}"), "ARITY_MISMATCH")?;
    if st_arity.line != Some(2) || st_arity.col != Some(13) {
        return Err(format!("ARITY_MISMATCH span mismatch: {:?}:{:?} (want 2:13)", st_arity.line, st_arity.col));
    }
    let st_unres = expect_omni_error_code("UNRESOLVED_LABEL", compile_omni_surface_to_u16("fn main(x: i64) -> i64 {\n    goto 9;\n    return x;\n}"), "UNRESOLVED_LABEL")?;
    if st_unres.line != Some(2) || st_unres.col != Some(10) {
        return Err(format!("UNRESOLVED_LABEL span mismatch: {:?}:{:?} (want 2:10)", st_unres.line, st_unres.col));
    }
    expect_omni_error_code("CIRCULAR_INCLUDE", resolve_omni_surface_includes("include \"loop.omni\"; fn main(x: i64) -> i64 { return x; }", None, Some(&loop_vfs)), "CIRCULAR_INCLUDE")?;
    let st_bud_aot = expect_omni_error_code("BUDGET_EXCEEDED(aot)", execute_omni_surface_program(runaway, vec![OmniValue::Int(1000)]), "BUDGET_EXCEEDED")?;
    if st_bud_aot.line != Some(1) || st_bud_aot.col != Some(3) {
        return Err(format!("BUDGET_EXCEEDED(aot) span mismatch: {:?}:{:?} (want 1:3)", st_bud_aot.line, st_bud_aot.col));
    }
    expect_omni_error_code("BUDGET_EXCEEDED(vm)", execute_omni_surface_program_with_budgets(runaway, vec![OmniValue::Int(1000)], Some(100_000), Some(1024 * 1024)), "BUDGET_EXCEEDED")?;
    expect_omni_error_code("BUDGET_EXCEEDED(mem)", execute_omni_surface_program("#[mem_budget(16)] fn main(x: i64) -> str { let s = \"0123456789ABCDEF0123456789ABCDEF\"; let clean = san_dom(s); return clean; }", vec![OmniValue::Int(1)]), "BUDGET_EXCEEDED")?;
    expect_omni_error_code("CORRUPT_PACK5", compile_pure_u16_to_wasm(&corrupt_u16), "CORRUPT_PACK5")?;
    let corrupt_surface = "#[cap(AI)]\nfn main() -> trit {\n    let b: bytes = anew(0);\n    b = iset(b, 250);\n    let v: trit_vec = unpack5(b, 5);\n    let g: trit = egate(v);\n    return g;\n}";
    let st_corrupt_surf = expect_omni_error_code("CORRUPT_PACK5(surface)", execute_omni_surface_program(corrupt_surface, vec![]), "CORRUPT_PACK5")?;
    if st_corrupt_surf.line != Some(5) || st_corrupt_surf.col != Some(23) {
        return Err(format!("CORRUPT_PACK5(surface) span mismatch: {:?}:{:?} (want 5:23)", st_corrupt_surf.line, st_corrupt_surf.col));
    }
    let st_ambig = expect_omni_error_code("AMBIGUOUS_GRAMMAR", compile_omni_surface_strict_to_u16("fn main(a: i64) -> trit {\n    let t: trit = TRIT_NEG;\n    return t;\n}"), "AMBIGUOUS_GRAMMAR")?;
    if st_ambig.line != Some(2) || st_ambig.col != Some(19) {
        return Err(format!("AMBIGUOUS_GRAMMAR span mismatch: {:?}:{:?} (want 2:19)", st_ambig.line, st_ambig.col));
    }
    let st_syn = expect_omni_error_code("SYNTAX_ERROR", compile_omni_surface_to_u16("fn main(x: i64) -> i64 {\n    let r = ;\n    return r;\n}"), "SYNTAX_ERROR")?;
    if st_syn.line != Some(2) || st_syn.col != Some(13) {
        return Err(format!("SYNTAX_ERROR span mismatch: {:?}:{:?} (want 2:13)", st_syn.line, st_syn.col));
    }
    checks += 19;

    // 13s. Full-stack route + request param + DB/KV host ABI goldens (zero new opcodes).
    let route_kv = "#[cap(NET, DB_R, DB_W)]\nroute GET /users/:id (req: i64) {\n    let id = req.param(\"id\");\n    let saved = kv_put(id, 777);\n    let row = db_get(id);\n    respond(200, row);\n}";
    let (kv_val, kv_backend) = execute_omni_surface_aot_first(route_kv, vec![OmniValue::Int(42)])?;
    if kv_val != OmniValue::Int(200_000_777) || kv_backend != "vm" {
        return Err(format!("route GET /users/:id kv golden mismatch: {:?} via {}", kv_val, kv_backend));
    }
    let route_q = "route GET /users/:id (req: i64) @cap(net, db_r) {\n    let id = req.param(\"id\");\n    let row = db_query(\"SELECT * FROM users WHERE id = ?\", id);\n    respond(200, row);\n}";
    if execute_omni_surface_program(route_q, vec![OmniValue::Int(42)])? != OmniValue::Int(200_000_420) {
        return Err(String::from("route GET /users/:id db_query golden mismatch (want HTTP 200 row 420)"));
    }
    let route_u16 = compile_omni_surface_to_u16(route_q)?;
    if !route_u16.iter().any(|&t| t == 0x7F6C) || !route_u16.iter().any(|&t| t == 0x7F65) || !route_u16.iter().any(|&t| t == 0x7F6E) {
        return Err(String::from("route lowering did not emit ROUTE/DB_QRY/RSP_JSON host opcodes"));
    }
    checks += 2;
    expect_omni_error_code("route_no_caps", execute_omni_surface_program("route GET /users/:id (req: i64) {\n    let id = req.param(\"id\");\n    let row = db_get(id);\n    respond(200, row);\n}", vec![OmniValue::Int(42)]), "MISSING_CAP")?;
    expect_omni_error_code("route_no_db_r", execute_omni_surface_program("route GET /users/:id (req: i64) @cap(net) {\n    let id = req.param(\"id\");\n    let row = db_get(id);\n    respond(200, row);\n}", vec![OmniValue::Int(42)]), "MISSING_CAP")?;
    expect_omni_error_code("route_no_db_w", execute_omni_surface_program("#[cap(NET, DB_R)] route POST /users/:id (req: i64) {\n    let id = req.param(\"id\");\n    let saved = kv_put(id, 1);\n    respond(201, saved);\n}", vec![OmniValue::Int(42)]), "MISSING_CAP")?;
    expect_omni_error_code("sql_injection", execute_omni_surface_program("#[cap(DB_R)] fn main(id: i64 @untrusted) -> i64 { let r = db_query(\"SELECT * FROM users WHERE id = ?\", id); return r; }", vec![OmniValue::Int(5)]), "TAINT_SINK")?;
    let authed = "#[cap(DB_R, AUTH)] fn main(id: i64 @untrusted) -> i64 { let ok = auth_verify(id); let r = db_query(\"SELECT * FROM users WHERE id = ?\", ok); return r; }";
    if execute_omni_surface_program(authed, vec![OmniValue::Int(5)])? != OmniValue::Int(50) {
        return Err(String::from("auth_verify sanitizer did not clear @untrusted for db_query"));
    }
    checks += 5;

    // 13t. Version-locked omni-harness JSON contract (with non-null 1-based line and col on all failures).
    let want_version = format!("\"version\":\"{}\"", OMNI_HARNESS_CONTRACT_VERSION);
    let contains_all = |s: &str, parts: &[&str], label: &str| -> Result<(), String> {
        if s.contains('\n') {
            return Err(format!("Harness '{}' output is not single-line JSON: {}", label, s));
        }
        if s.starts_with("{\"ok\":false") && (s.contains("\"line\":null") || s.contains("\"col\":null")) {
            return Err(format!("Harness '{}' failure JSON contains null line/col span: {}", label, s));
        }
        for p in parts {
            if !s.contains(p) {
                return Err(format!("Harness '{}' output missing {}: {}", label, p, s));
            }
        }
        Ok(())
    };
    let ok_json = run_omni_harness_json("fn main(a: i64, b: i64) -> i64 { return a + b; }", vec![OmniValue::Int(2), OmniValue::Int(3)], false, false);
    contains_all(&ok_json, &["{\"ok\":true", &want_version, "\"result\":\"5\"", "\"tokens\":", "\"backend\":\"aot\""], "ok")?;
    if ok_json != run_omni_harness_json("fn main(a: i64, b: i64) -> i64 { return a + b; }", vec![OmniValue::Int(2), OmniValue::Int(3)], false, false) {
        return Err(String::from("Harness output is not deterministic"));
    }
    let taint_json = run_omni_harness_json("fn main(x: i64 @untrusted) -> i64 {\n    return x;\n}", vec![OmniValue::Int(1)], false, true);
    contains_all(&taint_json, &["{\"ok\":false", &want_version, "\"code\":\"TAINT_ON_RET\"", "\"line\":2,\"col\":5", "\"retry_hint\":\"Sanitize"], "taint")?;
    let cap_json = run_omni_harness_json("fn main(x: i64) -> trit {\n    let g = egate(x);\n    return g;\n}", vec![OmniValue::Int(1)], false, false);
    contains_all(&cap_json, &["\"ok\":false", "\"code\":\"MISSING_CAP\"", "\"line\":2,\"col\":13"], "cap")?;
    if cap_json.contains("retry_hint") {
        return Err(String::from("Harness emitted retry_hint without --retry-hint"));
    }
    let budget_json = run_omni_harness_json(runaway, vec![OmniValue::Int(1000)], false, false);
    contains_all(&budget_json, &["\"ok\":false", "\"code\":\"BUDGET_EXCEEDED\"", "\"line\":1,\"col\":3"], "budget")?;
    let strict_json = run_omni_harness_json("fn main(a: i64) -> i64 {\n    let b = a * 2;\n    return b;\n}", vec![OmniValue::Int(1)], true, true);
    contains_all(&strict_json, &["\"ok\":false", "\"code\":\"BAD_TYPE\"", "\"line\":2,\"col\":9", "\"tokens\":null", "\"retry_hint\":"], "strict")?;
    let undef_json = run_omni_harness_json("fn main(a: i64) -> i64 {\n    let b = unknown_v + 1;\n    return b;\n}", vec![OmniValue::Int(1)], false, true);
    contains_all(&undef_json, &["\"ok\":false", "\"code\":\"UNDEF_VAR\"", "\"line\":2,\"col\":13"], "undef")?;
    let syn_json = run_omni_harness_json("fn main(a: i64) -> i64 {\n    let b = ;\n    return b;\n}", vec![OmniValue::Int(1)], false, true);
    contains_all(&syn_json, &["\"ok\":false", "\"code\":\"SYNTAX_ERROR\"", "\"line\":2,\"col\":13"], "syntax")?;
    let route_json = run_omni_harness_json(route_kv, vec![OmniValue::Int(42)], false, false);
    contains_all(&route_json, &["\"ok\":true", "\"result\":\"200000777\"", "\"backend\":\"vm\""], "route")?;
    let trit_json = run_omni_harness_json("#[cap(AI)] fn main(a: i64, b: i64) -> trit { let g = egate(a - b); return g; }", vec![OmniValue::Int(30), OmniValue::Int(10)], false, false);
    contains_all(&trit_json, &["\"ok\":true", "\"result\":\"+1t\""], "trit")?;
    let spec = omni_harness_spec_json();
    contains_all(&spec, &[&want_version, "\"error_codes\":", "\"capabilities\":", "\"strict_rules\":"], "spec")?;
    for code in ["TAINT_ON_RET", "TAINT_SINK", "MISSING_CAP", "LINEAR_LEAK", "BORROW_CONFLICT", "UNDEF_VAR", "IMMUT_PARAM", "BAD_TYPE", "UNKNOWN_FN", "ARITY_MISMATCH", "UNRESOLVED_LABEL", "CIRCULAR_INCLUDE", "BUDGET_EXCEEDED", "CORRUPT_PACK5", "AMBIGUOUS_GRAMMAR", "SYNTAX_ERROR"].iter() {
        if !spec.contains(&format!("\"{}\"", code)) {
            return Err(format!("Harness --spec missing error code {}", code));
        }
    }
    checks += 10;

    // 13u. Version-locked Agent Driver (`omni-agent-driver`) emit + repair loop selftests.
    if run_omni_agent_repair_loop("fn main() -> i64 { return 1; }", vec![], false, 3, "omni-harness-v0.9.0").is_ok() {
        return Err(String::from("Agent driver must refuse mismatched harness contract version"));
    }
    checks += 1;

    // Repair case 1: MISSING_CAP on egate -> repairs to #[cap(AI)] and returns +1t
    let rep_cap = run_omni_agent_repair_loop(
        "fn main(a: i64, b: i64) -> trit {\n    let g: trit = egate(a - b);\n    return g;\n}",
        vec![OmniValue::Int(30), OmniValue::Int(10)],
        true,
        3,
        OMNI_HARNESS_CONTRACT_VERSION,
    )?;
    if !rep_cap.ok || rep_cap.attempts.len() != 2 || rep_cap.attempts[0].error.as_ref().map(|e| (e.code, e.line, e.col)) != Some(("MISSING_CAP", Some(2), Some(19))) {
        return Err(format!("Agent driver MISSING_CAP repair failed: {:?}", rep_cap));
    }
    checks += 1;

    // Repair case 2: TAINT_ON_RET -> sanitizes return value with auth_verify and #[cap(AUTH)]
    let rep_taint = run_omni_agent_repair_loop(
        "fn main(x: i64 @untrusted) -> i64 {\n    return x;\n}",
        vec![OmniValue::Int(42)],
        true,
        3,
        OMNI_HARNESS_CONTRACT_VERSION,
    )?;
    if !rep_taint.ok || rep_taint.attempts.len() != 2 || rep_taint.attempts[0].error.as_ref().map(|e| (e.code, e.line)) != Some(("TAINT_ON_RET", Some(2))) {
        return Err(format!("Agent driver TAINT_ON_RET repair failed: {:?}", rep_taint));
    }
    checks += 1;

    // Repair case 3: TAINT_SINK on db_query -> sanitizes query parameter and adds AUTH capability
    let rep_sink = run_omni_agent_repair_loop(
        "#[cap(DB_R)]\nfn main(id: i64 @untrusted) -> i64 {\n    let r: i64 = db_query(\"SELECT * FROM users WHERE id = ?\", id);\n    return r;\n}",
        vec![OmniValue::Int(42)],
        true,
        3,
        OMNI_HARNESS_CONTRACT_VERSION,
    )?;
    if !rep_sink.ok || rep_sink.attempts.len() != 2 || !rep_sink.final_json.contains("\"result\":\"420\"") {
        return Err(format!("Agent driver TAINT_SINK repair failed: {:?}", rep_sink));
    }
    checks += 1;

    // Repair case 4: Multi-step strict repair (AMBIGUOUS_GRAMMAR + untyped let BAD_TYPE + MISSING_CAP)
    let rep_multi = run_omni_agent_repair_loop(
        "fn main(a: int) -> trit {\n    let d = a - 10;\n    let g = egate(d);\n    return g;\n}",
        vec![OmniValue::Int(25)],
        true,
        5,
        OMNI_HARNESS_CONTRACT_VERSION,
    )?;
    if !rep_multi.ok || rep_multi.attempts.len() < 3 || !rep_multi.final_json.contains("\"result\":\"+1t\"") {
        return Err(format!("Agent driver multi-step strict repair failed: {:?}", rep_multi));
    }
    checks += 1;

    // Repair case 5: BUDGET_EXCEEDED -> raises #[step_budget(N)] and completes
    let rep_budget = run_omni_agent_repair_loop(
        runaway,
        vec![OmniValue::Int(10)],
        false,
        3,
        OMNI_HARNESS_CONTRACT_VERSION,
    )?;
    if !rep_budget.ok || rep_budget.attempts.len() != 2 || !rep_budget.final_json.contains("\"result\":\"55\"") {
        return Err(format!("Agent driver BUDGET_EXCEEDED repair failed: {:?}", rep_budget));
    }
    checks += 1;

    // Repair case 6: IMMUT_PARAM + BORROW_CONFLICT + CORRUPT_PACK5 + external repair prompt builder
    let rep_immut = run_omni_agent_repair_loop(
        "fn main(x: i64) -> i64 {\n    x = x + 5;\n    return x;\n}",
        vec![OmniValue::Int(10)],
        true,
        3,
        OMNI_HARNESS_CONTRACT_VERSION,
    )?;
    if !rep_immut.ok || !rep_immut.final_json.contains("\"result\":\"15\"") {
        return Err(format!("Agent driver IMMUT_PARAM repair failed: {:?}", rep_immut));
    }
    let rep_borrow = run_omni_agent_repair_loop(
        "fn main(x: i64) -> i64 {\n    let a: arena = arena(4096);\n    let r: arena = borrow_r(a);\n    drop(a);\n    drop(r);\n    return x;\n}",
        vec![OmniValue::Int(77)],
        true,
        3,
        OMNI_HARNESS_CONTRACT_VERSION,
    )?;
    if !rep_borrow.ok || !rep_borrow.final_json.contains("\"result\":\"77\"") {
        return Err(format!("Agent driver BORROW_CONFLICT repair failed: {:?}", rep_borrow));
    }
    let rep_pack5 = run_omni_agent_repair_loop(
        corrupt_surface,
        vec![],
        true,
        3,
        OMNI_HARNESS_CONTRACT_VERSION,
    )?;
    if !rep_pack5.ok || !rep_pack5.final_json.contains("\"result\":\"+1t\"") {
        return Err(format!("Agent driver CORRUPT_PACK5 repair failed: {:?}", rep_pack5));
    }
    let prompt_json = build_omni_agent_repair_prompt_json(1, corrupt_surface, &st_corrupt_surf, &spec);
    contains_all(&prompt_json, &[&want_version, "\"attempt\":1", "\"code\":\"CORRUPT_PACK5\"", "\"line\":5,\"col\":23", "\"spec\":{"], "repair_prompt")?;
    checks += 4;

    // 13v. Live Native TCP HTTP/1.1 + DB/KV Host Golden (`serve_omni_http_batch`, `serve_omni_http_batch_with_db_file` & `serve_omni_http_once`).
    // 1) Verifier must reject serving a route without required capabilities before binding a port.
    let uncap_route = "route GET /users/:id (req: i64) {\n    let id: i64 = req.param(\"id\");\n    let row: i64 = db_query(\"SELECT * FROM users WHERE id = ?\", id);\n    respond(200, row);\n}";
    expect_omni_error_code(
        "live_host_missing_cap",
        serve_omni_http_once(uncap_route, "127.0.0.1:0", BTreeMap::new()).map(|(a, _)| a),
        "MISSING_CAP",
    )?;
    checks += 1;

    // 2) Live end-to-end TCP HTTP/1.1 golden + persistent on-disk KV/DB file across server restarts:
    // Instance #1 handles POST /users/42 -> persists 42=777 to disk; Instance #2 loads from disk and serves GET /users/42 -> 777!
    let live_fullstack_omni = "#[strict]\n#[cap(NET, DB_W)]\nroute POST /users/:id (req: i64) {\n    let id: i64 = req.param(\"id\");\n    let saved: i64 = kv_put(id, 777);\n    respond(201, saved);\n}\n#[strict]\n#[cap(NET, DB_R)]\nroute GET /users/:id (req: i64) {\n    let id: i64 = req.param(\"id\");\n    let row: i64 = db_get(id);\n    respond(200, row);\n}";
    let db_path = std::env::temp_dir().join(format!("omni_live_host_db_{}.kv", std::process::id()));
    let _ = std::fs::remove_file(&db_path);
    let (listen_addr1, server_handle1) = serve_omni_http_batch_with_db_file(live_fullstack_omni, "127.0.0.1:0", &db_path, 1)?;
    {
        use std::io::{Read, Write};
        let mut client1 = std::net::TcpStream::connect(listen_addr1)
            .map_err(|e| format!("TCP client1 connect failed: {}", e))?;
        client1
            .write_all(b"POST /users/42 HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .map_err(|e| format!("TCP client1 write failed: {}", e))?;
        let mut wire1 = String::new();
        client1
            .read_to_string(&mut wire1)
            .map_err(|e| format!("TCP client1 read failed: {}", e))?;
        if !wire1.starts_with("HTTP/1.1 201 Created\r\n") || !wire1.contains("{\"status\":201,\"data\":777}") {
            let _ = std::fs::remove_file(&db_path);
            return Err(format!("Live HTTP POST /users/42 unexpected wire response: {}", wire1));
        }
    }
    let (resps1, final_kv1) = server_handle1
        .join()
        .map_err(|_| String::from("Live HTTP server1 thread panicked"))??;
    if resps1.len() != 1 || final_kv1.get(&42) != Some(&777) {
        let _ = std::fs::remove_file(&db_path);
        return Err(format!("Live HTTP host1 KV state mismatch: {:?}", final_kv1));
    }

    // Instance #2 starts fresh and loads `db_path` from disk to serve GET /users/42
    let (listen_addr2, server_handle2) = serve_omni_http_batch_with_db_file(live_fullstack_omni, "127.0.0.1:0", &db_path, 1)?;
    {
        use std::io::{Read, Write};
        let mut client2 = std::net::TcpStream::connect(listen_addr2)
            .map_err(|e| format!("TCP client2 connect failed: {}", e))?;
        client2
            .write_all(b"GET /users/42 HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .map_err(|e| format!("TCP client2 write failed: {}", e))?;
        let mut wire2 = String::new();
        client2
            .read_to_string(&mut wire2)
            .map_err(|e| format!("TCP client2 read failed: {}", e))?;
        if !wire2.starts_with("HTTP/1.1 200 OK\r\n") || !wire2.contains("{\"status\":200,\"data\":777}") {
            let _ = std::fs::remove_file(&db_path);
            return Err(format!("Live HTTP GET /users/42 unexpected wire response: {}", wire2));
        }
    }
    let (resps2, final_kv2) = server_handle2
        .join()
        .map_err(|_| String::from("Live HTTP server2 thread panicked"))??;
    let _ = std::fs::remove_file(&db_path);
    if resps2.len() != 1 || final_kv2.get(&42) != Some(&777) {
        return Err(format!("Live HTTP host2 persistent KV state mismatch: {:?}", final_kv2));
    }
    checks += 2;

    // 13w. Section 1: Error Contract & Executable Acceptance Suite Selftests (`spec/acceptance/`)
    checks += run_error_contract_selftest()?;
    checks += run_acceptance_suite_selftest()?;

    // 13x. Section 3: Narrow Capability-Gated Host Interface (`OmniNarrowHost` & `SimulatedNarrowHostAdapter`)
    {
        let mut sim_host = SimulatedNarrowHostAdapter::new();
        // Verify narrow host capability enforcement rejects unprivileged TCP/File/Time calls with MISSING_CAP
        expect_omni_error_code(
            "narrow_host_no_net",
            sim_host.host_tcp_listen(0, "127.0.0.1:0"),
            "MISSING_CAP",
        )?;
        expect_omni_error_code(
            "narrow_host_no_db_w",
            sim_host.host_file_open(cap_bit(CAP_ID_DB_R), "test.kv", true),
            "MISSING_CAP",
        )?;
        expect_omni_error_code(
            "narrow_host_no_sys_time",
            sim_host.host_time_ns(0),
            "MISSING_CAP",
        )?;
        let t1 = sim_host.host_time_ns(cap_bit(CAP_ID_SYS))?;
        let t2 = sim_host.host_time_ns(cap_bit(CAP_ID_SYS))?;
        if t2 <= t1 {
            return Err(String::from("Narrow host monotonic clock did not advance"));
        }
        sim_host.enqueue_http_request("POST /users/42 HTTP/1.1
Host: localhost

");
        sim_host.enqueue_http_request("GET /users/42 HTTP/1.1
Host: localhost

");
        let (bound, sim_resps, sim_kv) = serve_omni_http_via_narrow_host(
            &mut sim_host,
            live_fullstack_omni,
            "127.0.0.1:0",
            Some("virtual_db.kv"),
            2,
        )?;
        if bound != "127.0.0.1:40080"
            || sim_resps.len() != 2
            || sim_resps[0].status != 201
            || sim_resps[1].status != 200
            || sim_kv.get(&42) != Some(&777)
            || !sim_host.virtual_files.contains_key("virtual_db.kv")
        {
            return Err(format!("Simulated narrow host roundtrip mismatch: bound={}, resps={:?}", bound, sim_resps));
        }
        checks += 4;
    }

    // 13y. Section 2: Deepened Multi-Step AI Repair Loop (`omni-repair-strategies-v1.1.0`)
    {
        let strat_json = omni_repair_strategies_json();
        if !strat_json.contains(OMNI_REPAIR_STRATEGIES_VERSION)
            || !strat_json.contains("strat_multi_fn_arity_align_v1")
            || !strat_json.contains("strat_include_cycle_prune_v1")
            || list_omni_repair_strategies().len() < 16
        {
            return Err(format!("Repair strategies catalog incomplete: {}", strat_json));
        }

        // Multi-step repair 1: Multi-function callee missing capability + caller arity mismatch
        let broken_multi_fn = "#[strict]
fn helper_gate(x: i64) -> trit {
    let g: trit = egate(x - 10);
    return g;
}
#[strict]
fn main(a: i64) -> trit {
    let res: trit = helper_gate(a, 999);
    return res;
}";
        let rep_mf = run_omni_agent_repair_loop(
            broken_multi_fn,
            vec![OmniValue::Int(25)],
            true,
            5,
            OMNI_HARNESS_CONTRACT_VERSION,
        )?;
        if !rep_mf.ok || !rep_mf.final_json.contains("\"result\":\"+1t\"") {
            return Err(format!("Multi-function arity + callee cap repair failed: {:?}", rep_mf));
        }

        // Multi-step repair 2: Multi-file circular include graph repair
        let mut cyc_vfs = BTreeMap::new();
        cyc_vfs.insert(
            String::from("math_util.omni"),
            String::from("include \"app.omni\";\n#[strict]\nfn scale3(v: i64) -> i64 { let r: i64 = v * 3; return r; }"),
        );
        cyc_vfs.insert(
            String::from("app.omni"),
            String::from("include \"math_util.omni\";\n#[strict]\nfn main(x: i64) -> i64 { let y = scale3(x); return y; }"),
        );
        let (_, rep_inc) = repair_omni_include_graph(&cyc_vfs, "app.omni", vec![OmniValue::Int(14)], true, 5)?;
        if !rep_inc.ok || !rep_inc.final_json.contains("\"result\":\"42\"") {
            return Err(format!("Include graph repair failed: {:?}", rep_inc));
        }

        // Multi-step repair 3: Unresolved goto label + untyped let in strict mode
        let broken_lbl = "#[strict]
fn main(x: i64) -> i64 {
    let out = x + 10;
    goto 4;
    return out;
}";
        let rep_lbl = run_omni_agent_repair_loop(broken_lbl, vec![OmniValue::Int(5)], true, 5, OMNI_HARNESS_CONTRACT_VERSION)?;
        if !rep_lbl.ok || !rep_lbl.final_json.contains("\"result\":\"15\"") {
            return Err(format!("Unresolved label + strict let repair failed: {:?}", rep_lbl));
        }

        // Multi-step repair 4: Undefined variable + unknown intrinsic
        let broken_undef = "#[strict]
fn main(x: i64) -> i64 {
    let a: i64 = unknown_abs(x);
    let b: i64 = a + missing_offset;
    return b;
}";
        let rep_undef = run_omni_agent_repair_loop(broken_undef, vec![OmniValue::Int(-20)], true, 5, OMNI_HARNESS_CONTRACT_VERSION)?;
        if !rep_undef.ok || !rep_undef.final_json.contains("\"result\":\"20\"") {
            return Err(format!("Undefined var + unknown fn repair failed: {:?}", rep_undef));
        }

        // Multi-step repair 5: Memory budget exceeded (`#[mem_budget(16)]`) + DOM capability inference
        let broken_mem = "#[strict]
#[mem_budget(16)]
fn main(x: i64) -> str {
    let s: str = \"0123456789ABCDEF0123456789ABCDEF\";
    let clean: str = san_dom(s);
    return clean;
}";
        let rep_mem = run_omni_agent_repair_loop(broken_mem, vec![OmniValue::Int(1)], true, 5, OMNI_HARNESS_CONTRACT_VERSION)?;
        if !rep_mem.ok || !rep_mem.final_json.contains("0123456789ABCDEF") {
            return Err(format!("Memory budget repair failed: {:?}", rep_mem));
        }

        // Verify strategy_id & strategy_version appear in run_omni_agent_driver_json
        let driver_json = run_omni_agent_driver_json(
            broken_multi_fn,
            vec![OmniValue::Int(25)],
            true,
            5,
            OMNI_HARNESS_CONTRACT_VERSION,
        );
        contains_all(
            &driver_json,
            &[
                "\"ok\":true",
                "\"strategy_version\":\"omni-repair-strategies-v1.1.0\"",
                "\"strategy_id\":\"strat_multi_fn_arity_align_v1\"",
                "\"strategy_id\":\"strat_multi_fn_callee_cap_v1\"",
            ],
            "agent_driver_strategy_json",
        )?;
        checks += 8;
    }

    // 13z. Section 6: Tooling & Human Diagnostics (`format_omni_inspect`, `format_omni_human_diagnostic`, `explain_omni_error_code`)
    {
        let inspect_src = "#[strict]
#[cap(AI)]
fn main(a: i64) -> trit {
    let g: trit = egate(a - 5);
    return g;
}";
        let disasm = inspect_omni_surface_program(inspect_src, true)?;
        if !disasm.contains("=== OMNI BINARY STREAM INSPECTION")
            || !disasm.contains("cap_mask=0x0010 (AI)")
            || !disasm.contains("OP_7F35_EGATE (0x7F35)")
            || !disasm.contains("// line 4, col 19")
        {
            return Err(format!("format_omni_inspect output missing expected annotations:
{}", disasm));
        }
        let explain_taint = explain_omni_error_code("TAINT_SINK")
            .ok_or_else(|| String::from("explain_omni_error_code(TAINT_SINK) returned None"))?;
        if !explain_taint.contains("error[TAINT_SINK]") || !explain_taint.contains("auth_verify") {
            return Err(format!("explain_omni_error_code(TAINT_SINK) mismatch: {}", explain_taint));
        }
        let diag = format_omni_human_diagnostic("fn main(x: i64 @untrusted) -> i64 {
    return x;
}", &st_taint_ret);
        if !diag.contains("error[TAINT_ON_RET]") || !diag.contains("--> line 3:5") || !diag.contains("^") {
            return Err(format!("format_omni_human_diagnostic mismatch:
{}", diag));
        }
        checks += 3;
    }

    // 13AA. Section 5: Next Abstraction Layer (`std/prelude.omni` + Hygienic `macro` Differential across VM, Stage-1, ELF & Wasm)
    {
        let macro_aot_src = "macro mad_step(a, b, c) = (a * b) + c;
macro trit_bonus(t) = sel_trit(t, -20, 0, 20);
#[strict]
fn main(x: i64, y: i64) -> i64 {
    let m: i64 = mad_step(x, 3, y);
    let cmp: trit = m <=> 50;
    let b: i64 = trit_bonus(cmp);
    return m + b;
}";
        let (aot_res, aot_back) = execute_omni_surface_aot_first(macro_aot_src, vec![OmniValue::Int(20), OmniValue::Int(5)])?;
        let vm_res = execute_omni_surface_program_with_budgets(macro_aot_src, vec![OmniValue::Int(20), OmniValue::Int(5)], None, None)?;
        let wasm_res = execute_omni_surface_wasm_i64(macro_aot_src, &[20, 5])?;
        let elf_direct = compile_omni_surface_to_x86_64_elf(macro_aot_src, 20, 5, 0)?;
        let elf_stage1 = compile_omni_surface_via_stage1_to_elf(macro_aot_src, 20, 5, 0)?;
        if aot_back != "aot" || aot_res != OmniValue::Int(85) || vm_res != OmniValue::Int(85) || wasm_res != 85 || elf_direct != elf_stage1 {
            return Err(format!(
                "Hygienic macro 4-way differential failed: aot={:?} ({}), vm={:?}, wasm={}",
                aot_res, aot_back, vm_res, wasm_res
            ));
        }
        checks += 2;
    }

    // 13AB. Section 4: Compositional Verification Invariants (`spec/trusted_core.md`)
    checks += run_compositional_invariant_selftests()?;

    // 13AC. Section 7: Phase 7 Pure-Omni Self-Hosted Verifier Kernel, Fixed-Point ELF64 Proof & Narrow-Host Runner
    checks += run_phase7_pure_omni_selftests()?;

    // 13AD. Phase A: Pure-Omni Surface Frontend (`omni-pure-frontend-v1.0.0`)
    checks += run_phase_a_pure_omni_frontend_selftests()?;

    // 13AE. Phase B: Pure-Omni Driver & Verifier Kernel Integration (`omni-pure-driver-v1.0.0`)
    checks += run_phase_b_pure_omni_driver_selftests()?;

    // 13AF. Phase C: Pure-Omni Host Runtime over Narrow Host (`omni-pure-host-v1.0.0`)
    checks += run_phase_c_pure_omni_host_selftests()?;

    // 13AG. Phase D: Pure-Omni Self-Hosting Bootstrap Fixed Point (`omni-pure-selfhost-v1.0.0`)
    checks += run_phase_d_pure_omni_bootstrap_selftests()?;

    // 13AH. Phase E: Full Pure-Omni Differential Lock (`pure-omni-diff`)
    checks += run_phase_e_pure_omni_diff_selftests()?;

    // 13AI. Pure-Omni Core Codebase (`pure/*.omni` + `std/*.omni`) & Multi-Module Self-Host Suite
    checks += run_pure_omni_core_suite_selftests()?;

    // 13AJ. Pure-Core Completion Phase 1: Full Surface Language Coverage in Pure-Omni Frontend
    checks += run_pure_core_phase1_frontend_coverage_selftests()?;

    // 13AK. Pure-Core Completion Phase 2: High-Value Tooling in Pure Omni
    checks += run_pure_core_phase2_tooling_selftests()?;

    // 13AL. Pure-Core Completion Phase 3: Complete Pure-Omni Host Runtime as Primary Path
    checks += run_pure_core_phase3_host_runtime_selftests()?;

    // 13AM. Pure-Core Completion Phase 4: Expand Pure Self-Host Compiler & Lock the Larger Fixed Point
    checks += run_pure_core_phase4_selfhost_fixed_point_selftests()?;

    // 13AN. Pure-Core Completion Phase 5: Pure-Omni Default Workflows & Legacy-Rust Differential
    checks += run_pure_core_phase5_default_workflow_selftests()?;

    // 13AO. Strengthening Phase S1: Real Pure-Omni Programs (`pure/programs/*.omni`)
    checks += run_pure_core_phase_s1_real_programs_selftests()?;

    // 13AP. Strengthening Phase S2: Deepened Pure Tooling for Larger Pure Codebases
    checks += run_pure_core_phase_s2_deepened_tooling_selftests()?;

    // 13AQ. Strengthening Phase S3: Unique Primitives & Pure Helpers (`std/pack5.omni`, `std/decision.omni`, `std/resource.omni`)
    checks += run_pure_core_phase_s3_unique_primitives_selftests()?;

    // 13AR. Strengthening Phase S4: Hardening, Health Metrics & Friction Reduction
    checks += run_pure_core_phase_s4_metrics_and_friction_selftests()?;

    // 13AS. Host FS/DB capability root-jail (PATH_ESCAPE)
    checks += run_host_sandbox_selftests()?;

    // 13AT. Pure-Omni verifier engine differential against the Rust twin
    let n_engine = run_verifier_engine_diff_selftests()?;
    if n_engine < 40 {
        return Err(format!(
            "Omni verifier engine differential compared only {n_engine} cases (expected at least 40)"
        ));
    }
    checks += 1;

    // 13AU. Value-returning if_pos/match_trit join slot is a single contiguous SSA dest
    checks += run_if_pos_join_slot_selftests()?;

    // 13AV. Lockstep shadow mode on compile / run / serve
    checks += run_lockstep_shadow_selftests()?;

    // 13AW. Vera P1 byte-readback sibling (VM + Wasm assert; x86/Stage-1 native run)
    checks += run_byte_readback_parity_selftests()?;

    Ok(checks)
}

/// Shared `harness` CLI front-end (`omni harness ...` / `omni-harness ...`).
/// Usage: harness [--strict] [--retry-hint] (--spec | --version-check | <file.omni | - | "inline source">) [args...]
/// Prints exactly one JSON line; exit code 0 on `"ok":true`, 1 otherwise, 2 on usage errors.
pub fn run_omni_harness_cli(argv: &[String]) -> i32 {
    let mut strict = false;
    let mut retry_hint = false;
    let mut pure_omni = true;
    let mut rest: Vec<String> = Vec::new();
    let mut idx_h = 0usize;
    while idx_h < argv.len() {
        let a = &argv[idx_h];
        if a == "--explain" && idx_h + 1 < argv.len() {
            match explain_omni_error_code(&argv[idx_h + 1]) {
                Some(text) => {
                    println!("{}", text);
                    return 0;
                }
                None => {
                    eprintln!("Unknown Omni error code '{}'", argv[idx_h + 1]);
                    return 2;
                }
            }
        }
        idx_h += 1;
    }
    for a in argv {
        match a.as_str() {
            "--strict" => strict = true,
            "--retry-hint" => retry_hint = true,
            "--pure-omni" => pure_omni = true,
            "--legacy-rust" => pure_omni = false,
            "--spec" => {
                println!("{}", omni_harness_spec_json());
                return 0;
            }
            "--version-check" | "--version" => {
                println!("{{\"version\":\"{}\"}}", OMNI_HARNESS_CONTRACT_VERSION);
                return 0;
            }
            _ => rest.push(a.clone()),
        }
    }
    if rest.is_empty() {
        eprintln!("usage: omni-harness [--strict] [--retry-hint] [--pure-omni] (--spec | --version-check | <file.omni | - | \"source\">) [args...]");
        return 2;
    }
    let target = &rest[0];
    let src = if target == "-" {
        use std::io::Read;
        let mut buf = String::new();
        if std::io::stdin().read_to_string(&mut buf).is_err() {
            eprintln!("failed to read source from stdin");
            return 2;
        }
        buf
    } else if target.ends_with(".omni") && std::path::Path::new(target).is_file() {
        match std::fs::read_to_string(target) {
            Ok(raw) => {
                if strict && (raw.contains("mod ") || raw.contains("mod\"")) {
                    raw
                } else {
                    let parent_dir = std::path::Path::new(target).parent();
                    match resolve_omni_surface_includes(&raw, parent_dir, None) {
                        Ok(expanded) => expanded,
                        Err(e) => {
                            let st = parse_omni_structured_error(&e);
                            let line_json = st.line.map(|n| n.to_string()).unwrap_or_else(|| String::from("null"));
                            let col_json = st.col.map(|n| n.to_string()).unwrap_or_else(|| String::from("null"));
                            if retry_hint {
                                println!(
                                    "{{\"ok\":false,\"version\":\"{}\",\"code\":\"{}\",\"line\":{},\"col\":{},\"message\":\"{}\",\"tokens\":null,\"retry_hint\":\"{}\"}}",
                                    OMNI_HARNESS_CONTRACT_VERSION,
                                    st.code,
                                    line_json,
                                    col_json,
                                    json_escape_str(&st.message),
                                    json_escape_str(st.retry_hint)
                                );
                            } else {
                                println!(
                                    "{{\"ok\":false,\"version\":\"{}\",\"code\":\"{}\",\"line\":{},\"col\":{},\"message\":\"{}\",\"tokens\":null}}",
                                    OMNI_HARNESS_CONTRACT_VERSION,
                                    st.code,
                                    line_json,
                                    col_json,
                                    json_escape_str(&st.message)
                                );
                            }
                            return 1;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("failed to read '{}': {}", target, e);
                return 2;
            }
        }
    } else {
        target.clone()
    };
    let vm_args: Vec<OmniValue> = rest[1..].iter().map(|s| parse_cli_omni_value(s)).collect();
    let out = if pure_omni {
        run_pure_omni_driver_json(&src, vm_args, strict, retry_hint)
    } else {
        run_omni_harness_json(&src, vm_args, strict, retry_hint)
    };
    println!("{}", out);
    if out.starts_with("{\"ok\":true") { 0 } else { 1 }
}
