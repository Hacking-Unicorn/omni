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
use crate::inspect::*;
use crate::acceptance::*;
use crate::pure_omni::*;

// ==============================================================================
// 7C. VERSION-LOCKED MODEL HARNESS (`omni-harness`) & STRUCTURED DIAGNOSTICS
// ==============================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OmniStructuredError {
    pub code: &'static str,
    pub line: Option<usize>,
    pub col: Option<usize>,
    pub message: String,
    pub retry_hint: &'static str,
}

pub(crate) fn map_error_code_str(tag: &str) -> Option<&'static str> {
    match tag {
        "TAINT_ON_RET" => Some("TAINT_ON_RET"),
        "TAINT_SINK" => Some("TAINT_SINK"),
        "MISSING_CAP" => Some("MISSING_CAP"),
        "LINEAR_LEAK" => Some("LINEAR_LEAK"),
        "BORROW_CONFLICT" => Some("BORROW_CONFLICT"),
        "UNDEF_VAR" => Some("UNDEF_VAR"),
        "IMMUT_PARAM" => Some("IMMUT_PARAM"),
        "BAD_TYPE" => Some("BAD_TYPE"),
        "UNKNOWN_FN" => Some("UNKNOWN_FN"),
        "ARITY_MISMATCH" => Some("ARITY_MISMATCH"),
        "UNRESOLVED_LABEL" => Some("UNRESOLVED_LABEL"),
        "CIRCULAR_INCLUDE" => Some("CIRCULAR_INCLUDE"),
        "BUDGET_EXCEEDED" => Some("BUDGET_EXCEEDED"),
        "CORRUPT_PACK5" => Some("CORRUPT_PACK5"),
        "AMBIGUOUS_GRAMMAR" => Some("AMBIGUOUS_GRAMMAR"),
        "SYNTAX_ERROR" => Some("SYNTAX_ERROR"),
        _ => None,
    }
}

pub(crate) fn retry_hint_for_code(code: &str) -> &'static str {
    match code {
        "TAINT_ON_RET" => "Sanitize @untrusted data with san_sql(x), san_dom(x), san_prm(x), or auth_verify(x) before returning.",
        "TAINT_SINK" => "Pass @untrusted values through san_sql(x), san_dom(x), san_prm(x), or auth_verify(x) before invoking a safe-sink opcode.",
        "MISSING_CAP" => "Add the required capability to the function or route header, e.g. #[cap(AI, NET, DB_R, DB_W, AUTH)] or @cap(net, db_r).",
        "LINEAR_LEAK" => "Ensure every allocated linear register or arena is consumed by drop(...) or returned before RET.",
        "BORROW_CONFLICT" => "Release active borrows before taking a conflicting mutable borrow_w(...) or moving the register.",
        "UNDEF_VAR" => "Declare the variable with 'let <name>: <ty> = <expr>;' before referencing or reassigning it.",
        "IMMUT_PARAM" => "Copy the immutable parameter into a local variable ('let tmp: i64 = param;') before reassigning.",
        "BAD_TYPE" => "Use a valid canonical type ('i64', 'trit', 'bytes', 'array', 'trit_vec', 'str', 'arena') and annotate every 'let' in #[strict] mode.",
        "UNKNOWN_FN" => "Use a built-in intrinsic (e.g. iabs, imin, imax, tmaj, sel_trit, egate, tdot, db_query, req.param, respond) or define the helper fn.",
        "ARITY_MISMATCH" => "Check the expected argument count of the intrinsic or user-defined function.",
        "UNRESOLVED_LABEL" => "Ensure every 'goto N;' or 'br_pos(c, N);' has a matching 'label N;' (0..=15) in the same function.",
        "CIRCULAR_INCLUDE" => "Remove the cyclic 'include \"...\";' reference between modules.",
        "BUDGET_EXCEEDED" => "Reduce loop iterations or allocations, or raise #[step_budget(N)] / #[mem_budget(N)].",
        "CORRUPT_PACK5" => "Ensure all @PACK5 bytes are in 0..=242 (valid 5-trit encodings) without sentinel bytes 243..=255.",
        "AMBIGUOUS_GRAMMAR" => "In #[strict] mode, use canonical '+1t'/'0t'/'-1t' trits, ': <ty>' on every let, '@untrusted' suffix, and 'include \"...\"'.",
        _ => "Check surface syntax, balanced braces, and function/route signatures.",
    }
}

/// Parses any compiler, verifier, or runtime diagnostic into a structured machine-readable `OmniStructuredError`.
pub fn parse_omni_structured_error(err: &str) -> OmniStructuredError {
    let mut code: &'static str = "SYNTAX_ERROR";
    if let Some(start) = err.find("error[") {
        let rest = &err[start + 6..];
        if let Some(end) = rest.find(']') {
            if let Some(mapped) = map_error_code_str(&rest[..end]) {
                code = mapped;
            }
        }
    } else if err.contains("Taint violation on RET") {
        code = "TAINT_ON_RET";
    } else if err.contains("Sink opcode") && err.contains("@UNTRUSTED") {
        code = "TAINT_SINK";
    } else if err.contains("Capability violation") {
        code = "MISSING_CAP";
    } else if err.contains("Linear value leak") {
        code = "LINEAR_LEAK";
    } else if err.contains("Borrow violation") {
        code = "BORROW_CONFLICT";
    } else if err.contains("budget exceeded") {
        code = "BUDGET_EXCEEDED";
    } else if err.contains("PACK5_SENTINEL_OR_CORRUPT_BYTE") {
        code = "CORRUPT_PACK5";
    } else if err.contains("Undefined variable") || err.contains("undeclared variable") {
        code = "UNDEF_VAR";
    } else if err.contains("immutable function parameter") {
        code = "IMMUT_PARAM";
    } else if err.contains("Unknown surface type") {
        code = "BAD_TYPE";
    } else if err.contains("Unknown surface intrinsic") {
        code = "UNKNOWN_FN";
    } else if err.contains("Unresolved branch target label") || err.contains("Unresolved jump target") {
        code = "UNRESOLVED_LABEL";
    } else if err.contains("Circular include") {
        code = "CIRCULAR_INCLUDE";
    }

    let mut line_num: Option<usize> = None;
    let mut col_num: Option<usize> = None;
    if let Some(l_idx) = err.find("line ") {
        let after_l = &err[l_idx + 5..];
        let digits: String = after_l.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = digits.parse::<usize>() {
            line_num = Some(n);
        }
        if let Some(c_idx) = after_l.find("col ") {
            let after_c = &after_l[c_idx + 4..];
            let c_digits: String = after_c.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(cn) = c_digits.parse::<usize>() {
                col_num = Some(cn);
            }
        }
    }

    OmniStructuredError {
        code,
        line: Some(line_num.unwrap_or(1).max(1)),
        col: Some(col_num.unwrap_or(1).max(1)),
        message: String::from(err),
        retry_hint: retry_hint_for_code(code),
    }
}

pub(crate) fn json_escape_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

pub(crate) fn format_omni_value_compact(v: &OmniValue) -> String {
    match v {
        OmniValue::Int(n) => n.to_string(),
        OmniValue::Trit(t) => {
            if *t > 0 {
                String::from("+1t")
            } else if *t < 0 {
                String::from("-1t")
            } else {
                String::from("0t")
            }
        }
        OmniValue::Str(s) => s.clone(),
        OmniValue::Bytes(b) => format!("bytes[{}]", b.len()),
        OmniValue::TritVec(tv) => format!("trit_vec[{}]", tv.len()),
        OmniValue::U16Array(a) => format!("u16[{}]", a.len()),
        OmniValue::Struct(m) => {
            if let (Some(OmniValue::Int(status)), Some(body)) = (m.get(&0), m.get(&1)) {
                format!("HTTP {} {}", status, format_omni_value_compact(body))
            } else {
                format!("{:?}", m)
            }
        }
        other => format!("{:?}", other),
    }
}

/// Returns the version-locked JSON specification of the `omni-harness` grammar, capabilities, and error codes.
pub fn omni_harness_spec_json() -> String {
    format!(
        "{{\"version\":\"{}\",\"isa_version\":\"0x{:04X}\",\"types\":[\"i64\",\"trit\",\"bytes\",\"array\",\"trit_vec\",\"str\",\"arena\"],\"capabilities\":[\"PURE\",\"DB_R\",\"DB_W\",\"NET\",\"DOM\",\"AI\",\"VISION\",\"AUDIO\",\"AUTH\",\"FS\",\"SYS\"],\"error_codes\":[\"TAINT_ON_RET\",\"TAINT_SINK\",\"MISSING_CAP\",\"LINEAR_LEAK\",\"BORROW_CONFLICT\",\"UNDEF_VAR\",\"IMMUT_PARAM\",\"BAD_TYPE\",\"UNKNOWN_FN\",\"ARITY_MISMATCH\",\"UNRESOLVED_LABEL\",\"CIRCULAR_INCLUDE\",\"BUDGET_EXCEEDED\",\"CORRUPT_PACK5\",\"AMBIGUOUS_GRAMMAR\",\"SYNTAX_ERROR\"],\"strict_rules\":[\"Explicit : <ty> on every let\",\"Canonical trit literals (+1t, 0t, -1t)\",\"Canonical @untrusted suffix\",\"Canonical include directive\"]}}",
        OMNI_HARNESS_CONTRACT_VERSION, OP_BASE
    )
}

/// Executes the version-locked `omni-harness` pipeline (compile -> static verify -> AOT/VM execute under budgets)
/// and returns a deterministic single-line JSON response locked to `OMNI_HARNESS_CONTRACT_VERSION`.
pub fn run_omni_harness_json(
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
    let u16_res = if strict {
        compile_omni_surface_strict_to_u16(&wrapped)
    } else {
        compile_omni_surface_to_u16(&wrapped)
    };
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
    match execute_omni_surface_aot_first(&wrapped, args) {
        Ok((val, backend)) => {
            format!(
                "{{\"ok\":true,\"version\":\"{}\",\"result\":\"{}\",\"tokens\":{},\"backend\":\"{}\"}}",
                OMNI_HARNESS_CONTRACT_VERSION,
                json_escape_str(&format_omni_value_compact(&val)),
                token_count,
                backend
            )
        }
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
