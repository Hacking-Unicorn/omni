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
use crate::pure_omni::*;

// ==============================================================================
// 7G. EXECUTABLE ACCEPTANCE SUITE, ERROR CONTRACT, CONTINUOUS DIFF & COMPOSITIONAL INVARIANTS
// ==============================================================================

/// Verifies the normative 16-code machine-readable error contract (`spec/error_contract.md`):
/// asserts that every code has a non-empty retry hint, appears in `omni-harness --spec`,
/// renders cleanly via `explain_omni_error_code` and `format_omni_human_diagnostic`, and carries non-null 1-based `(line, col)`.
pub fn run_error_contract_selftest() -> Result<usize, String> {
    let all_codes: [&str; 16] = [
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
    ];
    let spec_json = omni_harness_spec_json();
    for &code in &all_codes {
        if map_error_code_str(code) != Some(code) {
            return Err(format!("Error contract table missing map_error_code_str({})", code));
        }
        let hint = retry_hint_for_code(code);
        if hint.is_empty() || hint.starts_with("Check surface syntax") && code != "SYNTAX_ERROR" {
            return Err(format!("Error code {} lacks dedicated retry_hint", code));
        }
        if !spec_json.contains(&format!("\"{}\"", code)) {
            return Err(format!("omni_harness_spec_json missing error code {}", code));
        }
        let explained = explain_omni_error_code(code)
            .ok_or_else(|| format!("explain_omni_error_code returned None for {}", code))?;
        if !explained.contains(&format!("error[{}]", code)) || !explained.contains("Repaired:") {
            return Err(format!("explain_omni_error_code({}) malformed: {}", code, explained));
        }
        let sample_err = format!("error[{}] line 2, col 5: sample diagnostic for {}", code, code);
        let parsed = parse_omni_structured_error(&sample_err);
        if parsed.code != code || parsed.line != Some(2) || parsed.col != Some(5) || parsed.retry_hint != hint {
            return Err(format!("parse_omni_structured_error failed roundtrip for {}: {:?}", code, parsed));
        }
        let human = format_omni_human_diagnostic("fn main() -> i64 {\n    return 0;\n}", &parsed);
        if !human.contains(&format!("error[{}]", code)) || !human.contains("--> line 2:5") || !human.contains("= help:") {
            return Err(format!("format_omni_human_diagnostic failed for {}: {}", code, human));
        }
    }
    Ok(all_codes.len())
}

/// Runs the continuous differential acceptance harness (`omni acceptance-diff`):
/// feeds the frozen positive and negative acceptance suite through both the direct zero-IR compiler
/// (`compile_omni_surface_to_u16` / `compile_omni_surface_strict_to_u16` + `run_omni_harness_json`)
/// and the reference `.omni_asm` lowering + assembler + VM/AOT pipeline (or an optional external candidate command),
/// asserting bit-identical `Vec<u16>` token streams, exact locked token counts, and identical structured `(code, line, col)` errors.
pub fn run_omni_acceptance_diff(candidate_cmd: Option<&str>) -> Result<usize, String> {
    let mut verified_cases = 0usize;

    let positive_suite: &[(&str, &str, Option<usize>, &[&str], &str, &str)] = &[
        ("seed0_risk", "fn seed0_risk(base: i64, delta: i64) -> trit { let scaled = base * 2; let score = scaled + delta; let cmp = score <=> 50; let pos = +1t; let res = tand(cmp, pos); return res; }", Some(53), &["30", "15"], "+1t", "aot"),
        ("seed4_consensus", "#[fn_id(4)] fn seed4_consensus(s0: trit, s1: trit, s2: trit) -> trit { let m = tmaj(s0, s1, s2); let res = !m; return res; }", Some(34), &["+1t", "+1t", "-1t"], "-1t", "aot"),
        ("seed6_clamp", "#[fn_id(6)] fn seed6_clamp(x: i64, lo: i64, hi: i64) -> i64 { let lower = imax(x, lo); let clamped = imin(lower, hi); return clamped; }", Some(34), &["150", "10", "90"], "90", "aot"),
        ("custom_non_seed_alu", "fn custom_non_seed_alu(a: i64, b: i64, c: i64) -> i64 { let r0 = a * 4; let r1 = r0 - b; let r2 = c << 1; let r3 = r1 + r2; let r4 = r3 / 3; let r5 = r4 <=> 35; let r6 = !r5; let r7 = sel_trit(r6, 25, 50, 75); let r8 = r4 + r7; return r8; }", Some(91), &["30", "15", "9"], "66", "aot"),
        ("asm_roundtrip", "fn asm_roundtrip(a: i64, b: i64) -> trit { let res = (a * 3 - b) <=> 75; return res; }", Some(40), &["30", "10"], "+1t", "aot"),
        ("iabs_golden", "fn iabs_golden(x: i64) -> i64 { let r = iabs(x); return r; }", Some(20), &["-42"], "42", "aot"),
        ("idiv_zero", "fn idiv_zero(x: i64) -> i64 { let r = x / 0; return r; }", Some(22), &["42"], "0", "aot"),
        ("shl_65", "fn shl_65(x: i64) -> i64 { let r = x << 65; return r; }", Some(22), &["1"], "2", "aot"),
        ("sel_mix", "fn sel_mix(x: i64) -> i64 { let prod = x * 3; let cmp = prod <=> 20; let bonus = sel_trit(cmp, 5, 10, 15); let res = prod + bonus; return res; }", Some(49), &["7"], "36", "aot"),
        ("br_pos_golden", "fn br_pos_golden(x: i64) -> i64 { let c = x <=> 0; let r = if_pos c { 200 } else { 100 }; return r; }", Some(49), &["5"], "200", "aot"),
        ("br_neg_arith_golden", "fn br_neg_arith_golden(x: i64) -> i64 { let c = x <=> 0; let r = if_neg c { x - 50 } else { x + 100 }; return r; }", Some(51), &["-10"], "-60", "aot"),
        ("nested_branch_golden", "fn nested_branch_golden(x: i64) -> i64 { let c0 = x <=> 0; let r = if_pos c0 { let c1 = x <=> 5; if_pos c1 { 30 } else { 20 } } else { 10 }; return r; }", Some(74), &["8"], "30", "aot"),
        ("match_trit_golden", "fn match_trit_golden(x: i64) -> i64 { let c = x <=> 10; let r = match_trit c { neg => x - 5, zero => 100, pos => x + 50 }; return r; }", Some(69), &["20"], "70", "aot"),
        ("branch_loop_golden", "fn branch_loop_golden(n: i64) -> i64 { let acc = 0; let i = n; do_while { acc = acc + i; i = i - 1; } i <=> 0; return acc; }", Some(57), &["10"], "55", "aot"),
        ("byte_buf_golden", "fn byte_buf_golden(x: i64) -> i64 { let buf = anew(0); buf = iset(buf, 10, 20, 30); buf = iset_i32(buf, 100); buf = iset_i64(buf, 200); let len = slen(buf); let cmp = x <=> 5; let sel = cmov(cmp, 100, 200, 300); let total = len + sel; return total; }", Some(86), &["10"], "315", "aot"),
        ("decision_hook_golden", "#[strict]\n#[cap(AI)]\nfn decision_hook_golden(a: i64, b: i64) -> trit {\n    let delta: i64 = a - b;\n    let gate: trit = egate(delta);\n    return gate;\n}", None, &["30", "10"], "+1t", "aot"),
        ("pack5_trit_vec_golden", "#[strict]\n#[cap(AI)]\nfn pack5_trit_vec_golden() -> i64 {\n    let v: trit_vec = [+1t, -1t, 0t, +1t, +1t, -1t, 0t, -1t, +1t, +1t];\n    let packed: bytes = pack5(v);\n    let packed_byte_len: i64 = slen(packed);\n    let first_pack5_byte: i64 = iget(packed, 0);\n    let second_pack5_byte: i64 = iget(packed, 1);\n    let dot: i64 = tdot(v, v);\n    let gate: trit = egate(v);\n    let res: i64 = packed_byte_len * 1000 + first_pack5_byte + second_pack5_byte + dot + gate;\n    return res;\n}", None, &[], "2455", "aot"),
        ("multi_fn_weighted_term", "#[strict]\nfn weighted_term(x: i64, w: i64) -> i64 {\n    let p: i64 = x * w;\n    let s: i64 = p + 5;\n    return s;\n}\n#[strict]\nfn main(a: i64, b: i64) -> i64 {\n    let left: i64 = weighted_term(a, 3);\n    let right: i64 = weighted_term(b, 2);\n    let total: i64 = left + right;\n    return total;\n}", None, &["10", "20"], "80", "vm"),
        ("while_sum_even_golden", "#[strict]\nfn while_sum_even_golden(limit: i64) -> i64 {\n    let acc: i64 = 0;\n    let i: i64 = 2;\n    while i <= limit {\n        acc += i;\n        i += 2;\n    }\n    return acc;\n}", None, &["10"], "30", "aot"),
        ("arena_lifecycle_golden", "#[strict]\nfn arena_lifecycle_golden(x: i64) -> i64 {\n    let a: arena = arena(4096);\n    let r: arena = borrow_r(a);\n    let res: i64 = x + 42;\n    drop(r);\n    drop(a);\n    return res;\n}", None, &["8"], "50", "aot"),
        ("pack5_direct_aot", "#[strict]\n#[cap(AI)]\nfn pack5_direct_aot() -> i64 {\n    let raw: bytes = pack5_bytes([+1t, -1t, 0t, +1t, +1t, -1t, 0t, -1t, +1t, +1t]);\n    let dot: i64 = pack5_tdot(raw, raw);\n    let g: trit = pack5_gate(raw);\n    let tv: trit_vec = unpack5(raw, 10);\n    let t0: i64 = pack5_get(tv, 0);\n    let t1: i64 = pack5_get(tv, 1);\n    let b_len: i64 = pack5_len(raw);\n    let bonus: i64 = if_pos g { 2000 } else { 0 };\n    return bonus + dot * 50 + b_len * 10 + (t0 - t1);\n}", None, &[], "2422", "aot"),
        ("route_kv_golden", "#[strict]\n#[cap(NET, DB_R, DB_W)]\nroute GET /users/:id (req: i64) {\n    let id: i64 = req.param(\"id\");\n    let saved: i64 = kv_put(id, 777);\n    let row: i64 = db_get(id);\n    respond(200, row);\n}", None, &["42"], "200000777", "vm"),
        ("route_db_query_golden", "#[strict]\n#[cap(NET, DB_R)]\nroute GET /users/:id (req: i64) {\n    let id: i64 = req.param(\"id\");\n    let row: i64 = db_query(\"SELECT * FROM users WHERE id = ?\", id);\n    respond(200, row);\n}", None, &["42"], "200000420", "vm"),
        ("stdlib_macro_pipeline_golden", "include \"std/prelude.omni\";\nmacro weighted_diff(lhs, rhs, scale) = (lhs - rhs) * scale;\n#[strict]\n#[cap(AI)]\nfn main(x: i64, lo: i64, hi: i64) -> i64 {\n    let c: i64 = std_clamp(x, lo, hi);\n    let d: i64 = weighted_diff(c, lo, 3);\n    let g: trit = egate(d - 50);\n    let bonus: i64 = std_trit_mux(g, -10, 0, 10);\n    return d + bonus;\n}", None, &["45", "10", "40"], "100", "vm"),
    ];

    for &(gid, gsrc, locked_toks, arg_strs, want_res, want_backend) in positive_suite {
        let direct_u16 = compile_omni_surface_to_u16(gsrc)
            .map_err(|e| format!("Acceptance golden '{}' failed direct compile: {}", gid, e))?;
        let via_asm_u16 = assemble_omni_text_to_u16(&compile_omni_surface_to_asm(gsrc)?)
            .map_err(|e| format!("Acceptance golden '{}' failed asm roundtrip: {}", gid, e))?;
        if direct_u16 != via_asm_u16 {
            return Err(format!("Acceptance golden '{}' direct vs asm Vec<u16> divergence", gid));
        }
        if let Some(lock_n) = locked_toks {
            if direct_u16.len() != lock_n {
                return Err(format!(
                    "Acceptance golden '{}' token count {} != locked {}",
                    gid,
                    direct_u16.len(),
                    lock_n
                ));
            }
        }
        let vm_args: Vec<OmniValue> = arg_strs.iter().map(|s| parse_cli_omni_value(s)).collect();
        let h_json = run_omni_harness_json(gsrc, vm_args.clone(), gsrc.contains("#[strict]"), true);
        let pure_u16 = compile_omni_surface_via_pure_omni_frontend(gsrc, gsrc.contains("#[strict]"))
            .map_err(|e| format!("Acceptance golden '{}' failed Pure-Omni frontend compile: {}", gid, e))?;
        verify_module_via_phase7_pure_omni_kernel(&pure_u16)
            .map_err(|e| format!("Acceptance golden '{}' failed Pure-Omni verifier kernel: {}", gid, e))?;
        let pure_json = run_pure_omni_driver_json(gsrc, vm_args, gsrc.contains("#[strict]"), true);
        if pure_u16 != direct_u16 || pure_json != h_json {
            return Err(format!(
                "Acceptance golden '{}' Pure-Omni vs Rust divergence: pure_json={}, rust_json={}",
                gid, pure_json, h_json
            ));
        }
        let want_res_tag = format!("\"result\":\"{}\"", want_res);
        let want_back_tag = format!("\"backend\":\"{}\"", want_backend);
        if !h_json.starts_with("{\"ok\":true") || !h_json.contains(&want_res_tag) || !h_json.contains(&want_back_tag) {
            return Err(format!(
                "Acceptance golden '{}' harness mismatch: got {}, expected {} & {}",
                gid, h_json, want_res_tag, want_back_tag
            ));
        }
        if let Some(cmd) = candidate_cmd {
            let out = std::process::Command::new(cmd)
                .arg(gsrc)
                .args(arg_strs)
                .output()
                .map_err(|e| format!("Failed to invoke candidate_cmd '{}': {}", cmd, e))?;
            let cand_json = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if cand_json != h_json {
                return Err(format!(
                    "Acceptance diff mismatch on '{}': rust={} vs candidate={}",
                    gid, h_json, cand_json
                ));
            }
        }
        verified_cases += 1;
    }

    let negative_suite: &[(&str, &str, bool, &str, usize, usize)] = &[
        ("neg_taint_on_ret", "fn main(x: i64 @untrusted) -> i64 {\n    let safe_a = 1;\n    return x;\n}", false, "TAINT_ON_RET", 3, 5),
        ("neg_taint_sink", "#[cap(AI)]\nfn main(x: i64 @untrusted) -> trit {\n    let g = egate(x);\n    return g;\n}", false, "TAINT_SINK", 3, 13),
        ("neg_missing_cap", "fn main(x: i64) -> trit {\n    let g = egate(x);\n    return g;\n}", false, "MISSING_CAP", 2, 13),
        ("neg_linear_leak", "fn main(x: i64) -> i64 {\n    let a = arena(4096);\n    return x;\n}", false, "LINEAR_LEAK", 3, 5),
        ("neg_borrow_conflict", "fn main(x: i64) -> i64 {\n    let a = arena(4096);\n    let r = borrow_r(a);\n    drop(a);\n    drop(r);\n    return x;\n}", false, "BORROW_CONFLICT", 4, 5),
        ("neg_undef_var", "fn main(x: i64) -> i64 {\n    let r = missing_y + 1;\n    return r;\n}", false, "UNDEF_VAR", 2, 13),
        ("neg_immut_param", "fn main(x: i64) -> i64 {\n    x = x + 1;\n    return x;\n}", false, "IMMUT_PARAM", 2, 5),
        ("neg_bad_type", "fn main(x: float128) -> i64 { return 0; }", false, "BAD_TYPE", 1, 12),
        ("neg_unknown_fn", "fn main(x: i64) -> i64 {\n    let r = foo_bar(x);\n    return r;\n}", false, "UNKNOWN_FN", 2, 13),
        ("neg_arity_mismatch", "fn main(x: i64) -> i64 {\n    let r = iabs(x, x);\n    return r;\n}", false, "ARITY_MISMATCH", 2, 13),
        ("neg_unresolved_label", "fn main(x: i64) -> i64 {\n    goto 9;\n    return x;\n}", false, "UNRESOLVED_LABEL", 2, 10),
        ("neg_budget_exceeded", "#[step_budget(40)]\nfn main(n: i64) -> i64 {\n    let i = n;\n    let acc = 0;\n    while (i <=> 0) { acc += i; i -= 1; }\n    return acc;\n}", false, "BUDGET_EXCEEDED", 1, 3),
        ("neg_corrupt_pack5", "#[cap(AI)]\nfn main() -> trit {\n    let b: bytes = anew(0);\n    b = iset(b, 250);\n    let v: trit_vec = unpack5(b, 5);\n    let g: trit = egate(v);\n    return g;\n}", true, "CORRUPT_PACK5", 5, 23),
        ("neg_ambiguous_grammar", "fn main(a: i64) -> trit {\n    let t: trit = TRIT_NEG;\n    return t;\n}", true, "AMBIGUOUS_GRAMMAR", 2, 19),
        ("neg_syntax_error", "fn main(x: i64) -> i64 {\n    let r = ;\n    return r;\n}", false, "SYNTAX_ERROR", 2, 13),
    ];

    for &(nid, nsrc, strict_flag, want_code, want_l, want_c) in negative_suite {
        let h_json = run_omni_harness_json(nsrc, vec![OmniValue::Int(1000)], strict_flag, true);
        let pure_json = run_pure_omni_driver_json(nsrc, vec![OmniValue::Int(1000)], strict_flag, true);
        let want_tag = format!("\"code\":\"{}\",\"line\":{},\"col\":{}", want_code, want_l, want_c);
        if !h_json.starts_with("{\"ok\":false") || !h_json.contains(&want_tag) || pure_json != h_json {
            return Err(format!(
                "Acceptance negative golden '{}' mismatch: got {}, pure={}, expected {}",
                nid, h_json, pure_json, want_tag
            ));
        }
        verified_cases += 1;
    }

    Ok(verified_cases)
}

/// Validates both the in-memory acceptance differential suite and on-disk `spec/acceptance/acceptance_manifest.json` when present.
pub fn run_acceptance_suite_selftest() -> Result<usize, String> {
    let cnt = run_omni_acceptance_diff(None)?;
    let candidate_paths = [
        std::path::Path::new("spec/acceptance/acceptance_manifest.json"),
        std::path::Path::new("../spec/acceptance/acceptance_manifest.json"),
        std::path::Path::new("../../spec/acceptance/acceptance_manifest.json"),
    ];
    for p in &candidate_paths {
        if p.is_file() {
            let raw = std::fs::read_to_string(p).map_err(|e| e.to_string())?;
            if !raw.contains("\"manifest_version\": \"omni-acceptance-suite-v1.0.0\"")
                || !raw.contains("\"seed0_risk\"")
                || !raw.contains("\"stdlib_macro_pipeline_golden\"")
                || !raw.contains("\"neg_taint_on_ret\"")
            {
                return Err(format!("Malformed acceptance_manifest.json at '{}'", p.display()));
            }
            break;
        }
    }
    Ok(cnt)
}

/// CLI entry point for `omni acceptance-diff [--candidate-cmd <cmd>]`.
pub fn run_omni_acceptance_diff_cli(argv: &[String]) -> i32 {
    let mut cand_cmd: Option<&str> = None;
    let mut i = 0usize;
    while i < argv.len() {
        if argv[i] == "--candidate-cmd" && i + 1 < argv.len() {
            cand_cmd = Some(argv[i + 1].as_str());
            i += 2;
        } else {
            i += 1;
        }
    }
    match run_omni_acceptance_diff(cand_cmd) {
        Ok(cnt) => {
            println!(
                "[ACCEPTANCE-DIFF PASS] {} positive and negative executable specification goldens matched bit-for-bit ({}).",
                cnt, OMNI_ACCEPTANCE_SUITE_VERSION
            );
            0
        }
        Err(e) => {
            eprintln!("[ACCEPTANCE-DIFF FAIL] {}", e);
            1
        }
    }
}

/// Tests the 5 compositional verification invariants defined in `spec/trusted_core.md` (Section 4 of `OMNI_IMPROVEMENT_INSTRUCTIONS.md`).
pub fn run_compositional_invariant_selftests() -> Result<usize, String> {
    let mut checks = 0usize;

    // Invariant 1 (Taint Non-Interference Across Branched Control Flow):
    // Any path returning an unsanitized @untrusted value is rejected; sanitizing on the dominating path is accepted.
    let branched_taint_bad = "#[strict]\nfn main(x: i64 @untrusted, flag: i64) -> i64 {\n    let c: trit = flag <=> 0;\n    let r: i64 = if_pos c { x + 10 } else { 42 };\n    return r;\n}";
    expect_omni_error_code(
        "lemma1_branched_taint_bad",
        execute_omni_surface_program(branched_taint_bad, vec![OmniValue::Int(7), OmniValue::Int(1)]),
        "TAINT_ON_RET",
    )?;
    let branched_taint_good = "#[strict]\n#[cap(AUTH)]\nfn main(x: i64 @untrusted, flag: i64) -> i64 {\n    let clean: i64 = auth_verify(x);\n    let c: trit = flag <=> 0;\n    let r: i64 = if_pos c { clean + 10 } else { 42 };\n    return r;\n}";
    if execute_omni_surface_program(branched_taint_good, vec![OmniValue::Int(7), OmniValue::Int(1)])? != OmniValue::Int(17) {
        return Err(String::from("Lemma 1 sanitized branched control-flow returned wrong value"));
    }
    checks += 1;

    // Invariant 2 (Linear Arena Conservation & Zero Post-Execution Allocation):
    // Every accepted arena program finishes with allocated_bytes == 0, while multiple arenas with partial drop are rejected.
    let two_arenas_partial_leak = "#[strict]\nfn main(x: i64) -> i64 {\n    let a1: arena = arena(1024);\n    let a2: arena = arena(2048);\n    drop(a1);\n    return x;\n}";
    expect_omni_error_code(
        "lemma2_two_arenas_partial_leak",
        execute_omni_surface_program(two_arenas_partial_leak, vec![OmniValue::Int(9)]),
        "LINEAR_LEAK",
    )?;
    let two_arenas_conserved = "#[strict]\nfn main(x: i64) -> i64 {\n    let a1: arena = arena(1024);\n    let a2: arena = arena(2048);\n    let r1: arena = borrow_r(a1);\n    drop(r1);\n    drop(a2);\n    drop(a1);\n    return x + 1;\n}";
    let u16_conserved = compile_omni_surface_strict_to_u16(two_arenas_conserved)?;
    let mod_conserved = decode_omni_token_stream(&u16_conserved)?;
    let mut vm_conserved = OmniBinaryVm::new();
    let res_conserved = vm_conserved.execute_function(&mod_conserved, 0, vec![OmniValue::Int(41)])?;
    if res_conserved != OmniValue::Int(42) || vm_conserved.allocated_bytes != 0 {
        return Err(format!(
            "Lemma 2 arena conservation violated: res={:?}, allocated_bytes={}",
            res_conserved, vm_conserved.allocated_bytes
        ));
    }
    checks += 1;

    // Invariant 3 (Capability Monotonicity Across All Capability-Gated Opcodes in OMNI_ISA_TABLE):
    // Every opcode in 0x7F00..=0x7F7F with req_cap_mask != 0 is rejected by verify_binary_function when cap_mask == 0.
    let tc_safe = pack_tc(false, CAP_ID_PURE);
    let mut gated_opcodes_checked = 0usize;
    for op_tok in OP_BASE..=OP_MAX {
        if let Some(spec) = decode_opcode_token(op_tok) {
            if spec.req_cap_mask != 0 {
                let pkt = if spec.has_dest_reg {
                    vec![REG_BASE, op_tok, TY_I64, tc_safe, PAR_BASE, TOK_INS_END]
                } else {
                    vec![op_tok, PAR_BASE, TOK_INS_END]
                };
                let uncap_stream = pack_fn_module(
                    &[],
                    0,
                    &[(PAR_BASE, TY_I64, tc_safe)],
                    TY_I64,
                    0x0000,
                    &[pkt, vec![0x7F4A, PAR_BASE, TOK_INS_END]],
                );
                let uncap_mod = decode_omni_token_stream(&uncap_stream)?;
                let rep = verify_binary_function(&uncap_mod.functions[0]);
                if rep.passed || !rep.errors.iter().any(|e| e.contains("MISSING_CAP")) {
                    return Err(format!(
                        "Lemma 3 capability monotonicity failed on opcode 0x{:04X} ({})",
                        op_tok, spec.debug_label
                    ));
                }
                gated_opcodes_checked += 1;
            }
        }
    }
    if gated_opcodes_checked < 20 {
        return Err(format!("Expected at least 20 capability-gated opcodes, found {}", gated_opcodes_checked));
    }
    checks += 1;

    // Invariant 4 (Hot-Path AOT Observational Equivalence Across 64-Bit Boundary Inputs):
    let boundary_prog = "#[strict]\nfn main(a: i64, b: i64) -> i64 {\n    let q: i64 = a / b;\n    let r: i64 = a % b;\n    let c: trit = q <=> r;\n    let s: i64 = sel_trit(c, -10, 0, 10);\n    return (q + r) + s;\n}";
    let b_u16 = compile_omni_surface_strict_to_u16(boundary_prog)?;
    let b_mod = decode_omni_token_stream(&b_u16)?;
    if !is_module_aot_hot_path(&b_mod) {
        return Err(String::from("Lemma 4 boundary_prog must qualify as AOT hot path"));
    }
    let boundary_inputs: &[(i64, i64)] = &[
        (0, 0),
        (42, 0),
        (i64::MIN, -1),
        (i64::MAX, 1),
        (-99, 7),
    ];
    for &(a, b) in boundary_inputs {
        let (aot_v, back) = execute_omni_surface_aot_first(boundary_prog, vec![OmniValue::Int(a), OmniValue::Int(b)])?;
        let vm_v = execute_omni_surface_program_with_budgets(boundary_prog, vec![OmniValue::Int(a), OmniValue::Int(b)], None, None)?;
        if back != "aot" || aot_v != vm_v {
            return Err(format!("Lemma 4 AOT vs VM boundary divergence on ({}, {}): {:?} vs {:?}", a, b, aot_v, vm_v));
        }
    }
    checks += 1;

    // Invariant 5 (Module Composition & Hygienic Macro Taint/Capability Transparency):
    // A hygienic macro cannot hide an @untrusted sink violation or missing capability.
    let macro_taint_leak = "macro gate_macro(v) = egate(v);\n#[strict]\n#[cap(AI)]\nfn main(u: i64 @untrusted) -> trit {\n    let g: trit = gate_macro(u);\n    return g;\n}";
    expect_omni_error_code(
        "lemma5_macro_taint_transparency",
        execute_omni_surface_program(macro_taint_leak, vec![OmniValue::Int(10)]),
        "TAINT_SINK",
    )?;
    let macro_unbound_var = "macro bad_hygiene(a) = a + secret_caller_var;\n#[strict]\nfn main(x: i64) -> i64 {\n    let secret_caller_var: i64 = 100;\n    let r: i64 = bad_hygiene(x);\n    return r;\n}";
    expect_omni_error_code(
        "lemma5_macro_hygiene_unbound",
        compile_omni_surface_strict_to_u16(macro_unbound_var),
        "UNDEF_VAR",
    )?;
    checks += 1;

    Ok(checks)
}
