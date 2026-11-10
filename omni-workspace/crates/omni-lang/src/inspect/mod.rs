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
use crate::acceptance::*;
use crate::pure_omni::*;

// ==============================================================================
// 7F. BINARY STREAM INSPECTOR (`omni inspect` / `omni disasm`) & DIAGNOSTIC EXPLAINER
// ==============================================================================

pub(crate) fn decode_cap_mask_names(mask: CapMask) -> String {
    if mask == 0 {
        return String::from("PURE");
    }
    let table: &[(u16, &str)] = &[
        (CAP_ID_DB_R, "DB_R"),
        (CAP_ID_DB_W, "DB_W"),
        (CAP_ID_NET, "NET"),
        (CAP_ID_DOM, "DOM"),
        (CAP_ID_AI, "AI"),
        (CAP_ID_VISION, "VISION"),
        (CAP_ID_AUDIO, "AUDIO"),
        (CAP_ID_AUTH, "AUTH"),
        (CAP_ID_FS, "FS"),
        (CAP_ID_SYS, "SYS"),
    ];
    let mut names = Vec::new();
    for &(cid, cname) in table {
        if (mask & cap_bit(cid)) != 0 {
            names.push(cname);
        }
    }
    if names.is_empty() {
        format!("0x{:04X}", mask)
    } else {
        names.join(" | ")
    }
}

pub(crate) fn decode_ty_tok_name(ty_tok: u16) -> &'static str {
    match ty_tok {
        TY_VOID => "VOID",
        TY_I64 => "I64",
        TY_F64 => "F64",
        TY_TRIT => "TRIT",
        TY_STR => "STR",
        TY_UUID => "UUID",
        TY_BYTES => "BYTES",
        TY_TRIT_VEC => "TRIT_VEC",
        TY_FLT_VEC => "FLT_VEC",
        TY_ARRAY => "ARRAY",
        TY_ARENA => "ARENA",
        TY_MODEL_T => "MODEL_T",
        TY_DOM => "DOM",
        TY_STRUCT => "STRUCT",
        TY_ERR => "ERR",
        _ => "UNKNOWN_TY",
    }
}

pub(crate) fn format_inspect_operand(op: &BinOperand) -> String {
    match op {
        BinOperand::Reg(t) => format!("%{}", t.saturating_sub(REG_BASE)),
        BinOperand::Param(t) => format!("${}", t.saturating_sub(PAR_BASE)),
        BinOperand::ImmInt(v) => v.to_string(),
        BinOperand::ImmTrit(t) => {
            if *t > 0 {
                String::from("+1t")
            } else if *t < 0 {
                String::from("-1t")
            } else {
                String::from("0t")
            }
        }
        BinOperand::EnglishSpan(toks) => format!("@ENG[{} words]", toks.len()),
        BinOperand::SchemaRef(s) => format!("@schema{}", s),
        BinOperand::FnRef(f) => format!("@fn{}", f),
        BinOperand::SkillRef(sk) => format!("@skill{}", sk),
        BinOperand::Label(l) => format!("label({})", l.saturating_sub(LBL_BASE)),
        BinOperand::FieldKV(fid, inner) => format!("field#{}={}", fid, format_inspect_operand(inner)),
    }
}

/// Deterministic binary token stream disassembler & inspector (`omni inspect` / `omni disasm`)
/// displaying decoded opcodes, register/parameter/label operands, capability/taint tags, and source map coordinates.
pub fn format_omni_inspect(
    u16_stream: &[u16],
    sourcemap: Option<&SurfaceSourceMap>,
) -> Result<String, String> {
    let module = decode_omni_token_stream(u16_stream)?;
    let mut raw_bytes = Vec::with_capacity(u16_stream.len() * 2);
    for &w in u16_stream {
        raw_bytes.extend_from_slice(&w.to_le_bytes());
    }
    let mut out = String::new();
    out.push_str(&format!(
        "=== OMNI BINARY STREAM INSPECTION (words={}, bytes={}, fnv1a64=0x{:016X}) ===\n",
        u16_stream.len(),
        raw_bytes.len(),
        fnv1a_64_hash(&raw_bytes)
    ));
    out.push_str(&format!(
        "schemas: {}, experts: {}, functions: {}\n",
        module.schemas.len(),
        module.experts.len(),
        module.functions.len()
    ));
    for func in &module.functions {
        let rep = verify_binary_function(func);
        out.push_str(&format!(
            "\nfn #{} -> {} [cap_mask=0x{:04X} ({})] [verifier={}]\n",
            func.fn_idx,
            decode_ty_tok_name(func.ret_ty_tok),
            func.cap_mask,
            decode_cap_mask_names(func.cap_mask),
            if rep.passed { "PASS" } else { "FAIL" }
        ));
        for (p_idx, &(p_tok, p_ty, p_tc)) in func.params.iter().enumerate() {
            let (untr, cap_id, _) = unpack_tc(p_tc);
            out.push_str(&format!(
                "  param ${} (0x{:04X}) : {} [taint={}, cap_id={}]\n",
                p_idx,
                p_tok,
                decode_ty_tok_name(p_ty),
                if untr { "@UNTRUSTED" } else { "SAFE" },
                cap_id
            ));
        }
        for (ins_idx, ins) in func.instructions.iter().enumerate() {
            let op_name = decode_opcode_token(ins.op_tok)
                .map(|s| s.debug_label)
                .unwrap_or("INVALID_OP");
            let dst_part = match ins.dest_reg {
                Some(r) => {
                    let (untr, cap_id, _) = unpack_tc(ins.tc_tok);
                    format!(
                        "%{} : {} [{}, cap={}] = ",
                        r.saturating_sub(REG_BASE),
                        decode_ty_tok_name(ins.ty_tok),
                        if untr { "@UNTRUSTED" } else { "SAFE" },
                        cap_id
                    )
                }
                None => String::new(),
            };
            let ops_str: Vec<String> = ins.operands.iter().map(format_inspect_operand).collect();
            let loc_comment = sourcemap
                .and_then(|sm| sm.offset_to_span.get(&ins.offset))
                .map(|&(l, c)| format!(" // line {}, col {}", l, c))
                .unwrap_or_default();
            out.push_str(&format!(
                "  [{:03} @0x{:04X}] {}{} (0x{:04X}) {}{}\n",
                ins_idx,
                ins.offset,
                dst_part,
                op_name,
                ins.op_tok,
                ops_str.join(", "),
                loc_comment
            ));
        }
    }
    Ok(out)
}

/// Compiles a `.omni` surface program with its `SurfaceSourceMap` through the Pure-Omni frontend and returns the deterministic `format_omni_inspect` output.
pub fn inspect_omni_surface_program(src: &str, strict: bool) -> Result<String, String> {
    let (u16_stream, sm) = compile_omni_surface_via_pure_omni_frontend_with_sourcemap(src, strict)?;
    format_omni_inspect(&u16_stream, Some(&sm))
}

/// Compiles a `.omni` surface program via the Pure-Omni frontend, verifies it with the Pure-Omni verifier kernel, and returns `format_omni_inspect`.
pub fn inspect_omni_surface_via_pure_omni(src: &str, strict: bool) -> Result<String, String> {
    let (u16_stream, sm) = compile_omni_surface_via_pure_omni_frontend_with_sourcemap(src, strict)?;
    verify_module_via_phase7_pure_omni_kernel(&u16_stream)?;
    format_omni_inspect(&u16_stream, Some(&sm))
}

/// Renders a structured `OmniStructuredError` in a human-readable multi-line diagnostic format with source line and caret underline.
pub fn format_omni_human_diagnostic(src: &str, err: &OmniStructuredError) -> String {
    let line_no = err.line.unwrap_or(1).max(1);
    let col_no = err.col.unwrap_or(1).max(1);
    let src_line = src.lines().nth(line_no - 1).unwrap_or("");
    let caret_pad = " ".repeat(col_no.saturating_sub(1));
    format!(
        "error[{}]: {}\n  --> line {}:{}\n   |\n{:>2} | {}\n   | {}^\n   = help: {}",
        err.code, err.message, line_no, col_no, line_no, src_line, caret_pad, err.retry_hint
    )
}

/// Expands any of the 16 canonical `error[<CODE>]` identifiers into a normative explanation and repair pattern (`--explain <CODE>`).
pub fn explain_omni_error_code(code: &str) -> Option<String> {
    let norm = code.trim().to_ascii_uppercase();
    let (stage, summary, bad_ex, good_ex) = match norm.as_str() {
        "TAINT_ON_RET" => (
            "Static Verifier (0x7F4A RET)",
            "An @untrusted parameter, @ENG string literal, or register derived from untrusted input reached return without sanitization.",
            "fn main(x: i64 @untrusted) -> i64 { return x; }",
            "#[cap(AUTH)] fn main(x: i64 @untrusted) -> i64 { let clean: i64 = auth_verify(x); return clean; }",
        ),
        "TAINT_SINK" => (
            "Static Verifier (Privilege Sinks)",
            "An @untrusted operand was passed directly to a security-sensitive sink (egate, db_query, db_get, kv_put, respond).",
            "#[cap(AI)] fn main(x: i64 @untrusted) -> trit { return egate(x); }",
            "#[cap(AI, AUTH)] fn main(x: i64 @untrusted) -> trit { let c: i64 = auth_verify(x); return egate(c); }",
        ),
        "MISSING_CAP" => (
            "Static Verifier (Capability Bitmask)",
            "A function or route executed an opcode whose required capability bit (AI, NET, DB_R, DB_W, AUTH, DOM) is absent from #[cap(...)].",
            "fn main(x: i64) -> trit { return egate(x); }",
            "#[cap(AI)] fn main(x: i64) -> trit { return egate(x); }",
        ),
        "LINEAR_LEAK" => (
            "Static Verifier (Linear Arena Ownership)",
            "A linear arena allocated with arena(cap) was not consumed by drop(a) before function return.",
            "fn main(x: i64) -> i64 { let a: arena = arena(4096); return x; }",
            "fn main(x: i64) -> i64 { let a: arena = arena(4096); drop(a); return x; }",
        ),
        "BORROW_CONFLICT" => (
            "Static Verifier (Borrow State Machine)",
            "A linear arena was dropped or mutably borrowed while an active reader borrow (borrow_r) was still live.",
            "let a: arena = arena(4096); let r: arena = borrow_r(a); drop(a); drop(r);",
            "let a: arena = arena(4096); let r: arena = borrow_r(a); drop(r); drop(a);",
        ),
        "UNDEF_VAR" => (
            "Surface Lowering / Macro Hygiene",
            "An identifier was referenced or reassigned before being bound with 'let <name>: <ty> = <expr>;', or a macro referenced a free variable.",
            "fn main(x: i64) -> i64 { return missing_v + 1; }",
            "fn main(x: i64) -> i64 { let missing_v: i64 = 0; return missing_v + 1; }",
        ),
        "IMMUT_PARAM" => (
            "Surface Lowering",
            "Function parameters ($0..=$31) are immutable SSA inputs and cannot be reassigned directly.",
            "fn main(x: i64) -> i64 { x = x + 1; return x; }",
            "fn main(x: i64) -> i64 { let x_mut: i64 = x + 1; return x_mut; }",
        ),
        "BAD_TYPE" => (
            "Parser / Strict Single-Form Checker",
            "An unknown type name was used, or a 'let' binding omitted its explicit ': <ty>' annotation in #[strict] mode.",
            "#[strict] fn main(a: i64) -> i64 { let b = a * 2; return b; }",
            "#[strict] fn main(a: i64) -> i64 { let b: i64 = a * 2; return b; }",
        ),
        "UNKNOWN_FN" => (
            "Surface Lowering",
            "Called an identifier that is neither a declared function/macro in the module nor a built-in Omni intrinsic.",
            "fn main(x: i64) -> i64 { return unknown_op(x); }",
            "fn main(x: i64) -> i64 { return iabs(x); }",
        ),
        "ARITY_MISMATCH" => (
            "Surface Lowering",
            "The number of arguments passed to a function, hygienic macro, or built-in intrinsic did not match its signature.",
            "fn main(x: i64) -> i64 { return iabs(x, 2); }",
            "fn main(x: i64) -> i64 { return iabs(x); }",
        ),
        "UNRESOLVED_LABEL" => (
            "Surface Lowering / Static Verifier",
            "A 'goto N;' or conditional branch targeted a basic-block label N (0..=15) not defined by 'label N;' in the same function.",
            "fn main(x: i64) -> i64 { goto 3; return x; }",
            "fn main(x: i64) -> i64 { goto 3; label 3; return x; }",
        ),
        "CIRCULAR_INCLUDE" => (
            "Include Preprocessor",
            "A cycle was detected in 'include \"...\";' directives across modules.",
            "include \"self.omni\"; fn main(x: i64) -> i64 { return x; }",
            "include \"std/prelude.omni\"; fn main(x: i64) -> i64 { return x; }",
        ),
        "BUDGET_EXCEEDED" => (
            "VM / Wasm AOT Runtime",
            "Execution exceeded the function's #[step_budget(N)] instruction cap or #[mem_budget(N)] byte allocation cap.",
            "#[step_budget(10)] fn main(n: i64) -> i64 { ... }",
            "#[step_budget(500000)] fn main(n: i64) -> i64 { ... }",
        ),
        "CORRUPT_PACK5" => (
            "@PACK5 Validator / Decoder",
            "A @PACK5 byte buffer contained a sentinel or out-of-range byte in 243..=255 (valid 5-trit radix-3 bytes are 0..=242).",
            "b = iset(b, 250); let v: trit_vec = unpack5(b, 5);",
            "b = iset(b, 242); let v: trit_vec = unpack5(b, 5);",
        ),
        "AMBIGUOUS_GRAMMAR" => (
            "Strict Single-Form Lexer / Parser",
            "Non-canonical surface aliases (TRIT_POS, int, tvec, untrusted_i64, mod) are forbidden in #[strict] mode.",
            "#[strict] fn main(a: int) -> trit { return TRIT_POS; }",
            "#[strict] fn main(a: i64) -> trit { return +1t; }",
        ),
        "SYNTAX_ERROR" => (
            "Surface Lexer / Parser",
            "Malformed expression, unbalanced delimiters, missing match_trit arm, or empty module.",
            "fn main(x: i64) -> i64 { let r = ; return r; }",
            "fn main(x: i64) -> i64 { let r: i64 = x + 1; return r; }",
        ),
        _ => return None,
    };
    Some(format!(
        "=== OMNI ERROR CODE EXPLANATION: error[{}] ===\nContract: {}\nStage:    {}\nSummary:  {}\nHint:     {}\nBad:      {}\nRepaired: {}",
        norm,
        OMNI_ERROR_CONTRACT_VERSION,
        stage,
        summary,
        retry_hint_for_code(&norm),
        bad_ex,
        good_ex
    ))
}

/// CLI handler for `omni inspect <file.omni | file.omni_bc | "inline source">` and `omni disasm ...`.
pub fn run_omni_inspect_cli(argv: &[String]) -> i32 {
    if argv.is_empty() {
        eprintln!("usage: omni inspect <file.omni | file.omni_bc | \"source\">");
        return 2;
    }
    let target = &argv[0];
    if target.ends_with(".omni_bc") && std::path::Path::new(target).is_file() {
        let raw = match std::fs::read(target) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("failed to read '{}': {}", target, e);
                return 2;
            }
        };
        let u16s = decode_bytes_to_u16_stream(&raw);
        match format_omni_inspect(&u16s, None) {
            Ok(text) => {
                print!("{}", text);
                return 0;
            }
            Err(e) => {
                eprintln!("{}", e);
                return 1;
            }
        }
    }
    let src = if target.ends_with(".omni") && std::path::Path::new(target).is_file() {
        match std::fs::read_to_string(target) {
            Ok(raw) => {
                let parent = std::path::Path::new(target).parent();
                match resolve_omni_surface_includes(&raw, parent, None) {
                    Ok(exp) => exp,
                    Err(e) => {
                        eprintln!("{}", e);
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
    match inspect_omni_surface_program(&src, false) {
        Ok(text) => {
            print!("{}", text);
            0
        }
        Err(e) => {
            eprintln!("{}", e);
            1
        }
    }
}
