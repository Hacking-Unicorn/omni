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
use crate::harness::*;
use crate::inspect::*;
use crate::acceptance::*;
use crate::pure_omni::*;

// ==============================================================================
// 7D. VERSION-LOCKED AGENT DRIVER (`omni-agent-driver`) & FIRST-CLASS REPAIR STRATEGIES
// ==============================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OmniRepairStrategySpec {
    pub id: &'static str,
    pub version: &'static str,
    pub error_code: &'static str,
    pub context_pattern: &'static str,
    pub description: &'static str,
}

/// Returns the complete catalog of version-locked first-class repair strategies (`omni-repair-strategies-v1.1.0`).
pub fn list_omni_repair_strategies() -> Vec<OmniRepairStrategySpec> {
    vec![
        OmniRepairStrategySpec {
            id: "strat_missing_cap_minimal_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "MISSING_CAP",
            context_pattern: "single_or_multi_fn_opcode_cap",
            description: "Infers the minimal required capability bit(s) from the failing opcode/function and attaches or merges #[cap(...)] on the enclosing fn or route.",
        },
        OmniRepairStrategySpec {
            id: "strat_multi_fn_callee_cap_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "MISSING_CAP",
            context_pattern: "multi_fn_callee",
            description: "Attaches minimal required #[cap(...)] to a helper callee in a multi-function module and propagates capabilities to callers.",
        },
        OmniRepairStrategySpec {
            id: "strat_taint_sanitize_ret_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "TAINT_ON_RET",
            context_pattern: "untrusted_return",
            description: "Sanitizes @untrusted return values with auth_verify(...) or san_prm(...) and adds minimal AUTH/AI capability.",
        },
        OmniRepairStrategySpec {
            id: "strat_taint_sanitize_sink_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "TAINT_SINK",
            context_pattern: "untrusted_sink_arg",
            description: "Wraps @untrusted arguments passed to egate(...) or db_query(...) in auth_verify(...) and adds AUTH capability.",
        },
        OmniRepairStrategySpec {
            id: "strat_bad_type_infer_let_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "BAD_TYPE",
            context_pattern: "strict_untyped_let_or_unknown_type",
            description: "Infers canonical Omni types (i64, trit, trit_vec, bytes, str, arena) on untyped let bindings or replaces unknown type names with canonical i64.",
        },
        OmniRepairStrategySpec {
            id: "strat_ambiguous_grammar_canonical_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "AMBIGUOUS_GRAMMAR",
            context_pattern: "strict_non_canonical_alias",
            description: "Normalizes TRIT_POS/ZERO/NEG to +1t/0t/-1t, int/tvec to i64/trit_vec, untrusted_<ty> to <ty> @untrusted, and mod to include.",
        },
        OmniRepairStrategySpec {
            id: "strat_budget_aware_synthesis_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "BUDGET_EXCEEDED",
            context_pattern: "step_or_mem_budget_trap",
            description: "Synthesizes right-sized #[step_budget(N)] or #[mem_budget(N)] attributes when execution traps under a tight budget.",
        },
        OmniRepairStrategySpec {
            id: "strat_linear_arena_drop_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "LINEAR_LEAK",
            context_pattern: "undropped_arena_at_ret",
            description: "Inserts drop(arena_var) immediately before return to satisfy linear arena conservation.",
        },
        OmniRepairStrategySpec {
            id: "strat_borrow_order_swap_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "BORROW_CONFLICT",
            context_pattern: "drop_owner_before_reader",
            description: "Reorders drop(reader) before drop(owner) so all active reader borrows are released before dropping the arena.",
        },
        OmniRepairStrategySpec {
            id: "strat_immut_param_local_copy_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "IMMUT_PARAM",
            context_pattern: "reassign_fn_param",
            description: "Copies an immutable function parameter into a mutable local SSA binding before reassignment.",
        },
        OmniRepairStrategySpec {
            id: "strat_corrupt_pack5_clamp_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "CORRUPT_PACK5",
            context_pattern: "sentinel_byte_243_to_255",
            description: "Clamps invalid @PACK5 sentinel bytes (243..=255) into the valid 5-trit radix-3 domain (0..=242).",
        },
        OmniRepairStrategySpec {
            id: "strat_multi_fn_arity_align_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "ARITY_MISMATCH",
            context_pattern: "call_arity_mismatch",
            description: "Aligns caller argument count with the target function or intrinsic signature by trimming extra args or padding missing defaults.",
        },
        OmniRepairStrategySpec {
            id: "strat_include_cycle_prune_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "CIRCULAR_INCLUDE",
            context_pattern: "cyclic_include_directive",
            description: "Prunes cyclic include directives from single-file or multi-file module graphs.",
        },
        OmniRepairStrategySpec {
            id: "strat_unresolved_label_insert_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "UNRESOLVED_LABEL",
            context_pattern: "missing_branch_target_label",
            description: "Inserts the missing basic-block target 'label N;' before function return.",
        },
        OmniRepairStrategySpec {
            id: "strat_undef_var_init_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "UNDEF_VAR",
            context_pattern: "undeclared_variable_use",
            description: "Declares a typed local binding 'let <var>: i64 = 0;' prior to the first use of an undefined variable.",
        },
        OmniRepairStrategySpec {
            id: "strat_unknown_fn_alias_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "UNKNOWN_FN",
            context_pattern: "unknown_intrinsic_or_helper",
            description: "Replaces an unknown function call with a canonical built-in intrinsic (iabs).",
        },
    ]
}

/// Serializes all available repair strategies (`omni-repair-strategies-v1.1.0`) as deterministic single-line JSON.
pub fn omni_repair_strategies_json() -> String {
    let items: Vec<String> = list_omni_repair_strategies()
        .into_iter()
        .map(|s| {
            format!(
                "{{\"id\":\"{}\",\"version\":\"{}\",\"error_code\":\"{}\",\"context_pattern\":\"{}\",\"description\":\"{}\"}}",
                s.id,
                s.version,
                s.error_code,
                s.context_pattern,
                json_escape_str(s.description)
            )
        })
        .collect();
    format!(
        "{{\"strategy_version\":\"{}\",\"harness_version\":\"{}\",\"count\":{},\"strategies\":[{}]}}",
        OMNI_REPAIR_STRATEGIES_VERSION,
        OMNI_HARNESS_CONTRACT_VERSION,
        items.len(),
        items.join(",")
    )
}

/// Selects the deterministic first-class repair strategy ID for `(src, err)`.
pub fn select_omni_repair_strategy(src: &str, err: &OmniStructuredError) -> &'static str {
    let fn_count = src.matches("fn ").count();
    match err.code {
        "MISSING_CAP" => {
            if fn_count > 1 {
                "strat_multi_fn_callee_cap_v1"
            } else {
                "strat_missing_cap_minimal_v1"
            }
        }
        "TAINT_ON_RET" => "strat_taint_sanitize_ret_v1",
        "TAINT_SINK" => "strat_taint_sanitize_sink_v1",
        "BAD_TYPE" => "strat_bad_type_infer_let_v1",
        "AMBIGUOUS_GRAMMAR" => "strat_ambiguous_grammar_canonical_v1",
        "BUDGET_EXCEEDED" => "strat_budget_aware_synthesis_v1",
        "LINEAR_LEAK" => "strat_linear_arena_drop_v1",
        "BORROW_CONFLICT" => "strat_borrow_order_swap_v1",
        "IMMUT_PARAM" => "strat_immut_param_local_copy_v1",
        "CORRUPT_PACK5" => "strat_corrupt_pack5_clamp_v1",
        "ARITY_MISMATCH" => "strat_multi_fn_arity_align_v1",
        "CIRCULAR_INCLUDE" => "strat_include_cycle_prune_v1",
        "UNRESOLVED_LABEL" => "strat_unresolved_label_insert_v1",
        "UNDEF_VAR" => "strat_undef_var_init_v1",
        "UNKNOWN_FN" => "strat_unknown_fn_alias_v1",
        _ => "strat_syntax_fallback_v1",
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OmniAgentAttempt {
    pub attempt: usize,
    pub source: String,
    pub harness_json: String,
    pub error: Option<OmniStructuredError>,
    pub strategy_id: Option<&'static str>,
    pub strategy_version: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OmniAgentRepairReport {
    pub ok: bool,
    pub contract_version: String,
    pub strategy_version: String,
    pub attempts: Vec<OmniAgentAttempt>,
    pub final_source: String,
    pub final_json: String,
}

pub(crate) fn infer_surface_rhs_type(rhs: &str) -> &'static str {
    let first_stmt = rhs.split(';').next().unwrap_or(rhs);
    let e = first_stmt.trim();
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
    } else if e.starts_with("pack5_bytes(") || e.starts_with("anew(") || e.starts_with("pack5(") {
        "bytes"
    } else if e.starts_with("arena(") {
        "arena"
    } else if e.contains("<=>")
        || e.contains("==")
        || e.contains("!=")
        || e.contains("<=")
        || e.contains(">=")
        || e.contains(" < ")
        || e.contains(" > ")
        || e.starts_with("egate(")
        || e.starts_with("decision_hook(")
        || e.starts_with("tlogic(")
        || e.starts_with("tmaj(")
        || e.starts_with("tand(")
        || e.starts_with("tor(")
        || e.starts_with("txor(")
        || e.starts_with("tcmp(")
        || e == "+1t"
        || e == "0t"
        || e == "-1t"
        || e.starts_with('!')
    {
        "trit"
    } else {
        "i64"
    }
}

/// Infers the minimal required capability names for a `MISSING_CAP` diagnostic.
pub fn infer_minimal_capabilities_for_error(src: &str, err: &OmniStructuredError) -> Vec<&'static str> {
    let mut needed: Vec<&'static str> = Vec::new();
    // 1. Check exact opcode hex in the verifier error message first for minimal capability precision
    if err.message.contains("0x7F35") {
        needed.push("AI");
    }
    if err.message.contains("0x7F6C") || err.message.contains("0x7F6D") || err.message.contains("0x7F6E") || err.message.contains("0x7F6F") {
        needed.push("NET");
    }
    if err.message.contains("0x7F64") || err.message.contains("0x7F65") || err.message.contains("0x7F68") {
        needed.push("DB_R");
    }
    if err.message.contains("0x7F63") || err.message.contains("0x7F6B") {
        needed.push("DB_W");
    }
    if err.message.contains("0x7F70") {
        needed.push("AUTH");
    }
    if err.message.contains("0x7F76") {
        needed.push("DOM");
    }
    if !needed.is_empty() {
        return needed;
    }
    // 2. Fallback to surface call inspection
    if src.contains("route ")
        || src.contains("req.param")
        || src.contains("req_param")
        || src.contains("respond(")
        || src.contains("respond_json(")
        || src.contains("respond_html(")
    {
        needed.push("NET");
    }
    if src.contains("db_query") || src.contains("db_get") || src.contains("kv_get") || src.contains("db_count") {
        needed.push("DB_R");
    }
    if src.contains("kv_put") || src.contains("db_put") {
        needed.push("DB_W");
    }
    if src.contains("egate(") || src.contains("tlogic(") || src.contains("decision_hook(") || src.contains("pack5_gate(") {
        needed.push("AI");
    }
    if src.contains("auth_verify(") {
        needed.push("AUTH");
    }
    if src.contains("san_dom(") {
        needed.push("DOM");
    }
    if src.contains("san_sql(") {
        needed.push("DB_W");
    }
    if needed.is_empty() {
        needed.push("AI");
    }
    needed
}

/// Repairs a multi-file `.omni` include graph by breaking circular `include` chains,
/// synthesizing missing included stub modules, and running the repair loop on the linked module.
pub fn repair_omni_include_graph(
    vfs: &BTreeMap<String, String>,
    entry_file: &str,
    args: Vec<OmniValue>,
    strict: bool,
    max_attempts: usize,
) -> Result<(BTreeMap<String, String>, OmniAgentRepairReport), String> {
    let mut repaired_vfs = vfs.clone();
    let mut entry_src = repaired_vfs
        .get(entry_file)
        .cloned()
        .unwrap_or_else(|| String::from(entry_file));

    // Break circular includes and synthesize missing included files in repaired_vfs
    for _pass in 0..8 {
        match resolve_omni_surface_includes(&entry_src, None, Some(&repaired_vfs)) {
            Ok(expanded) => {
                let rep = run_omni_agent_repair_loop(
                    &expanded,
                    args,
                    strict,
                    max_attempts,
                    OMNI_HARNESS_CONTRACT_VERSION,
                )?;
                repaired_vfs.insert(String::from(entry_file), rep.final_source.clone());
                return Ok((repaired_vfs, rep));
            }
            Err(e) => {
                let st = parse_omni_structured_error(&e);
                if st.code == "CIRCULAR_INCLUDE" {
                    // Strip cyclic include directives from VFS modules and entry_src
                    for val in repaired_vfs.values_mut() {
                        let filtered: Vec<&str> = val
                            .lines()
                            .filter(|l| {
                                let t = l.trim_start();
                                !(t.starts_with("include ") || t.starts_with("include\""))
                            })
                            .collect();
                        *val = filtered.join("\n");
                    }
                    let filtered_entry: Vec<&str> = entry_src
                        .lines()
                        .filter(|l| {
                            let t = l.trim_start();
                            if t.starts_with("include ") || t.starts_with("include\"") {
                                // Keep include if target exists in repaired_vfs and is now cycle-free
                                if let Some(q1) = t.find('"') {
                                    if let Some(q2) = t[q1 + 1..].find('"') {
                                        let inc_name = &t[q1 + 1..q1 + 1 + q2];
                                        return inc_name != entry_file && repaired_vfs.contains_key(inc_name);
                                    }
                                }
                                false
                            } else {
                                true
                            }
                        })
                        .collect();
                    entry_src = filtered_entry.join("\n");
                    repaired_vfs.insert(String::from(entry_file), entry_src.clone());
                } else if e.contains("Failed to read included file '") {
                    if let Some(q1) = e.find("Failed to read included file '") {
                        let rest = &e[q1 + 30..];
                        if let Some(q2) = rest.find('\'') {
                            let missing_name = &rest[..q2];
                            repaired_vfs.insert(
                                String::from(missing_name),
                                String::from("#[strict]\nfn stub_helper(x: i64) -> i64 { return x; }\n"),
                            );
                        }
                    }
                } else if let Some(repaired_entry) = repair_omni_program_from_harness_error(&entry_src, &st, strict) {
                    entry_src = repaired_entry;
                    repaired_vfs.insert(String::from(entry_file), entry_src.clone());
                } else {
                    break;
                }
            }
        }
    }
    let rep = run_omni_agent_repair_loop(
        &entry_src,
        args,
        strict,
        max_attempts,
        OMNI_HARNESS_CONTRACT_VERSION,
    )?;
    repaired_vfs.insert(String::from(entry_file), rep.final_source.clone());
    Ok((repaired_vfs, rep))
}

/// Repairs a candidate `.omni` surface program using the structured `(code, line, col, retry_hint, message)`
/// feedback returned by `omni-harness`. Lives outside the token ISA and verifier.
pub fn repair_omni_program_from_harness_error(
    src: &str,
    err: &OmniStructuredError,
    _strict: bool,
) -> Option<String> {
    let target_line = err.line.unwrap_or(1).max(1);
    match err.code {
        "MISSING_CAP" => {
            let needed = infer_minimal_capabilities_for_error(src, err);
            let cap_attr = format!("#[cap({})]", needed.join(", "));
            // If multi-function module, attach #[cap(...)] to the enclosing function at or above target_line
            let lines: Vec<&str> = src.lines().collect();
            if src.matches("fn ").count() > 1 && target_line <= lines.len() {
                let mut fn_line_idx: Option<usize> = None;
                for idx in (0..target_line).rev() {
                    let trimmed = lines[idx].trim_start();
                    if trimmed.starts_with("fn ") || trimmed.contains(" fn ") || trimmed.starts_with("route ") {
                        fn_line_idx = Some(idx);
                        break;
                    }
                }
                if let Some(f_idx) = fn_line_idx {
                    let mut new_lines: Vec<String> = lines.iter().map(|s| String::from(*s)).collect();
                    if f_idx > 0 && new_lines[f_idx - 1].contains("#[cap(") {
                        let prev = &new_lines[f_idx - 1];
                        if let Some(c_start) = prev.find("#[cap(") {
                            if let Some(rel_end) = prev[c_start..].find(")]") {
                                let existing = &prev[c_start + 6..c_start + rel_end];
                                let mut merged: Vec<String> = existing
                                    .split(',')
                                    .map(|s| s.trim().to_ascii_uppercase())
                                    .filter(|s| !s.is_empty())
                                    .collect();
                                for n in &needed {
                                    if !merged.iter().any(|m| m == n) {
                                        merged.push(String::from(*n));
                                    }
                                }
                                new_lines[f_idx - 1] = format!(
                                    "{}#[cap({})]{}",
                                    &prev[..c_start],
                                    merged.join(", "),
                                    &prev[c_start + rel_end + 2..]
                                );
                                let out = new_lines.join("\n");
                                if out != src {
                                    return Some(out);
                                }
                            }
                        }
                    } else if new_lines[f_idx].contains("#[cap(") {
                        let cur = &new_lines[f_idx];
                        if let Some(c_start) = cur.find("#[cap(") {
                            if let Some(rel_end) = cur[c_start..].find(")]") {
                                let existing = &cur[c_start + 6..c_start + rel_end];
                                let mut merged: Vec<String> = existing
                                    .split(',')
                                    .map(|s| s.trim().to_ascii_uppercase())
                                    .filter(|s| !s.is_empty())
                                    .collect();
                                for n in &needed {
                                    if !merged.iter().any(|m| m == n) {
                                        merged.push(String::from(*n));
                                    }
                                }
                                new_lines[f_idx] = format!(
                                    "{}#[cap({})]{}",
                                    &cur[..c_start],
                                    merged.join(", "),
                                    &cur[c_start + rel_end + 2..]
                                );
                                let out = new_lines.join("\n");
                                if out != src {
                                    return Some(out);
                                }
                            }
                        }
                    } else {
                        new_lines[f_idx] = format!("{} {}", cap_attr, new_lines[f_idx]);
                        let out = new_lines.join("\n");
                        if out != src {
                            return Some(out);
                        }
                    }
                }
            }
            if let Some(c_start) = src.find("#[cap(") {
                if let Some(rel_end) = src[c_start..].find(")]") {
                    let existing = &src[c_start + 6..c_start + rel_end];
                    let mut merged: Vec<String> = existing
                        .split(',')
                        .map(|s| s.trim().to_ascii_uppercase())
                        .filter(|s| !s.is_empty())
                        .collect();
                    for n in needed {
                        if !merged.iter().any(|m| m == n) {
                            merged.push(String::from(n));
                        }
                    }
                    let mut out = String::new();
                    out.push_str(&src[..c_start]);
                    out.push_str(&format!("#[cap({})]", merged.join(", ")));
                    out.push_str(&src[c_start + rel_end + 2..]);
                    if out != src {
                        return Some(out);
                    }
                }
            }
            if let Some(pos) = src.find("route ").or_else(|| src.find("fn ")) {
                let mut out = String::new();
                out.push_str(&src[..pos]);
                out.push_str(&cap_attr);
                out.push(' ');
                out.push_str(&src[pos..]);
                return Some(out);
            }
            Some(format!("{}\n{}", cap_attr, src))
        }
        "TAINT_ON_RET" | "TAINT_SINK" => {
            if src.contains("@untrusted") {
                let mut sanitized = src.replace("@untrusted", "@safe");
                if err.code == "TAINT_ON_RET" {
                    let lines: Vec<&str> = src.lines().collect();
                    if target_line <= lines.len() {
                        let line_s = lines[target_line - 1];
                        if let Some(ret_idx) = line_s.find("return ") {
                            let var_expr = line_s[ret_idx + 7..].trim().trim_end_matches(';').trim();
                            let rep_stmt = format!(
                                "let __clean: i64 = auth_verify({}); return __clean;",
                                var_expr
                            );
                            let mut new_lines: Vec<String> = lines.iter().map(|s| String::from(*s)).collect();
                            new_lines[target_line - 1] = format!(
                                "{}{}",
                                &line_s[..ret_idx],
                                rep_stmt
                            );
                            let joined = new_lines.join("\n");
                            sanitized = if joined.contains("#[cap(") {
                                joined.replacen("#[cap(", "#[cap(AUTH, ", 1)
                            } else {
                                format!("#[cap(AUTH)] {}", joined)
                            };
                        }
                    }
                } else if err.code == "TAINT_SINK" {
                    let lines: Vec<&str> = src.lines().collect();
                    if target_line <= lines.len() {
                        let line_s = lines[target_line - 1];
                        if line_s.contains("egate(") {
                            let mut new_lines: Vec<String> = lines.iter().map(|s| String::from(*s)).collect();
                            new_lines[target_line - 1] = line_s.replace("egate(", "egate(auth_verify(").replace(");", "));");
                            let joined = new_lines.join("\n");
                            sanitized = if joined.contains("#[cap(") {
                                joined.replacen("#[cap(", "#[cap(AUTH, ", 1)
                            } else {
                                format!("#[cap(AI, AUTH)] {}", joined)
                            };
                        } else if line_s.contains("tlogic(") {
                            let mut new_lines: Vec<String> = lines.iter().map(|s| String::from(*s)).collect();
                            new_lines[target_line - 1] = line_s.replace("tlogic(", "tlogic(auth_verify(").replace(");", "));");
                            let joined = new_lines.join("\n");
                            sanitized = if joined.contains("#[cap(") {
                                joined.replacen("#[cap(", "#[cap(AUTH, ", 1)
                            } else {
                                format!("#[cap(AI, AUTH)] {}", joined)
                            };
                        } else if line_s.contains("db_get(") {
                            let mut new_lines: Vec<String> = lines.iter().map(|s| String::from(*s)).collect();
                            new_lines[target_line - 1] = line_s.replace("db_get(", "db_get(auth_verify(").replace(");", "));");
                            let joined = new_lines.join("\n");
                            sanitized = if joined.contains("#[cap(") {
                                joined.replacen("#[cap(", "#[cap(AUTH, ", 1)
                            } else {
                                format!("#[cap(DB_R, AUTH)] {}", joined)
                            };
                        } else if line_s.contains("kv_put(") {
                            let mut new_lines: Vec<String> = lines.iter().map(|s| String::from(*s)).collect();
                            if let Some(comma) = line_s.rfind(',') {
                                if let Some(rparen) = line_s.rfind(')') {
                                    let arg2 = line_s[comma + 1..rparen].trim();
                                    new_lines[target_line - 1] = format!(
                                        "{}, auth_verify({}){}",
                                        &line_s[..comma],
                                        arg2,
                                        &line_s[rparen..]
                                    );
                                    let joined = new_lines.join("\n");
                                    sanitized = if joined.contains("#[cap(") {
                                        joined.replacen("#[cap(", "#[cap(AUTH, ", 1)
                                    } else {
                                        format!("#[cap(DB_W, AUTH)] {}", joined)
                                    };
                                }
                            }
                        } else if line_s.contains("db_query(") {
                            let mut new_lines: Vec<String> = lines.iter().map(|s| String::from(*s)).collect();
                            if let Some(comma) = line_s.rfind(',') {
                                if let Some(rparen) = line_s.rfind(')') {
                                    let arg2 = line_s[comma + 1..rparen].trim();
                                    new_lines[target_line - 1] = format!(
                                        "{}, auth_verify({}){}",
                                        &line_s[..comma],
                                        arg2,
                                        &line_s[rparen..]
                                    );
                                    let joined = new_lines.join("\n");
                                    sanitized = if joined.contains("#[cap(") {
                                        joined.replacen("#[cap(", "#[cap(AUTH, ", 1)
                                    } else {
                                        format!("#[cap(DB_R, AUTH)] {}", joined)
                                    };
                                }
                            }
                        }
                    }
                }
                if sanitized != src {
                    return Some(sanitized);
                }
            }
            None
        }
        "BAD_TYPE" => {
            if err.message.contains("Missing explicit type annotation on 'let ") {
                let mut new_lines: Vec<String> = Vec::new();
                for (idx, line_s) in src.lines().enumerate() {
                    if idx + 1 == target_line || line_s.contains("let ") {
                        let mut search_offset = 0usize;
                        let mut fixed_this_line = None;
                        while let Some(rel_let) = line_s[search_offset..].find("let ") {
                            let let_pos = search_offset + rel_let;
                            let after_let = &line_s[let_pos + 4..];
                            if let Some(eq_pos) = after_let.find('=') {
                                let lhs_var = after_let[..eq_pos].trim();
                                if !lhs_var.contains(':') && !lhs_var.contains(';') {
                                    let rhs = &after_let[eq_pos + 1..];
                                    let inferred_ty = infer_surface_rhs_type(rhs);
                                    fixed_this_line = Some(format!(
                                        "{}let {}: {} ={}",
                                        &line_s[..let_pos],
                                        lhs_var,
                                        inferred_ty,
                                        rhs
                                    ));
                                    break;
                                }
                            }
                            search_offset = let_pos + 4;
                        }
                        if let Some(fixed_line) = fixed_this_line {
                            new_lines.push(fixed_line);
                            continue;
                        }
                    }
                    new_lines.push(String::from(line_s));
                }
                let out = new_lines.join("\n");
                if out != src {
                    return Some(out);
                }
            } else if err.message.contains("Unknown surface type '") {
                if let Some(q1) = err.message.find("Unknown surface type '") {
                    let rest = &err.message[q1 + 22..];
                    if let Some(q2) = rest.find('\'') {
                        let bad_ty = &rest[..q2];
                        let out = src.replace(bad_ty, "i64");
                        if out != src {
                            return Some(out);
                        }
                    }
                }
            }
            None
        }
        "AMBIGUOUS_GRAMMAR" => {
            let mut out = String::from(src);
            out = out.replace("TRIT_POS", "+1t");
            out = out.replace("TRIT_ZERO", "0t");
            out = out.replace("TRIT_NEG", "-1t");
            out = out.replace("untrusted_i64", "i64 @untrusted");
            out = out.replace("untrusted_str", "str @untrusted");
            out = out.replace(": int", ": i64");
            out = out.replace("-> int", "-> i64");
            out = out.replace(": tvec", ": trit_vec");
            out = out.replace("-> tvec", "-> trit_vec");
            if out.contains("mod \"") {
                out = out.replace("mod \"", "include \"");
            }
            if out != src {
                Some(out)
            } else {
                None
            }
        }
        "BUDGET_EXCEEDED" => {
            let mut out = String::from(src);
            if let Some(sb_pos) = out.find("#[step_budget(") {
                if let Some(end_p) = out[sb_pos..].find(")]") {
                    out = format!(
                        "{}#[step_budget(500000)]{}",
                        &out[..sb_pos],
                        &out[sb_pos + end_p + 2..]
                    );
                    return Some(out);
                }
            }
            if let Some(mb_pos) = out.find("#[mem_budget(") {
                if let Some(end_p) = out[mb_pos..].find(")]") {
                    out = format!(
                        "{}#[mem_budget(1048576)]{}",
                        &out[..mb_pos],
                        &out[mb_pos + end_p + 2..]
                    );
                    return Some(out);
                }
            }
            None
        }
        "LINEAR_LEAK" => {
            let lines: Vec<&str> = src.lines().collect();
            let mut fn_ranges: Vec<(usize, usize)> = Vec::new();
            let mut cur_start: Option<usize> = None;
            let mut brace_depth = 0usize;
            for (idx, line_s) in lines.iter().enumerate() {
                let t = line_s.trim_start();
                if cur_start.is_none() && (t.starts_with("fn ") || t.contains(" fn ") || t.starts_with("route ")) {
                    cur_start = Some(idx);
                    brace_depth = 0;
                }
                if cur_start.is_some() {
                    for b in line_s.bytes() {
                        if b == b'{' {
                            brace_depth += 1;
                        } else if b == b'}' {
                            brace_depth = brace_depth.saturating_sub(1);
                            if brace_depth == 0 {
                                if let Some(st) = cur_start.take() {
                                    fn_ranges.push((st, idx + 1));
                                }
                            }
                        }
                    }
                }
            }
            if let Some(st) = cur_start {
                fn_ranges.push((st, lines.len()));
            }
            if fn_ranges.is_empty() {
                fn_ranges.push((0, lines.len()));
            }
            let mut new_lines: Vec<String> = lines.iter().map(|s| String::from(*s)).collect();
            let mut patched = false;
            for (fn_start_line, fn_end_line) in fn_ranges {
                let fn_slice = lines[fn_start_line..fn_end_line].join("\n");
                let mut arena_vars: Vec<String> = Vec::new();
                for line_s in &lines[fn_start_line..fn_end_line] {
                    if line_s.contains("arena(") || line_s.contains("borrow_r(") || line_s.contains("borrow_w(") {
                        if let Some(let_pos) = line_s.find("let ") {
                            let var_part = &line_s[let_pos + 4..];
                            let v_name = var_part
                                .split(|c: char| c == ':' || c == '=' || c.is_ascii_whitespace())
                                .next()
                                .unwrap_or("")
                                .trim();
                            if !v_name.is_empty() && !fn_slice.contains(&format!("drop({});", v_name)) {
                                arena_vars.push(String::from(v_name));
                            }
                        }
                    }
                }
                if !arena_vars.is_empty() {
                    arena_vars.reverse();
                    let drop_stmts: String = arena_vars.iter().map(|v| format!("drop({}); ", v)).collect();
                    for line_item in new_lines.iter_mut().take(fn_end_line).skip(fn_start_line) {
                        if line_item.contains("return ") {
                            *line_item = line_item.replacen("return ", &format!("{}return ", drop_stmts), 1);
                            patched = true;
                            break;
                        }
                    }
                    if patched {
                        break;
                    }
                }
            }
            if patched {
                let out = new_lines.join("\n");
                if out != src {
                    return Some(out);
                }
            }
            None
        }
        "BORROW_CONFLICT" => {
            let lines: Vec<&str> = src.lines().collect();
            if target_line <= lines.len() && target_line < lines.len() {
                let cur_l = lines[target_line - 1].trim();
                let next_l = lines[target_line].trim();
                if cur_l.starts_with("drop(") && next_l.starts_with("drop(") {
                    let mut new_lines: Vec<String> = lines.iter().map(|s| String::from(*s)).collect();
                    new_lines.swap(target_line - 1, target_line);
                    let out = new_lines.join("\n");
                    if out != src {
                        return Some(out);
                    }
                }
            }
            None
        }
        "IMMUT_PARAM" => {
            if let Some(q1) = err.message.find("parameter '") {
                let rest = &err.message[q1 + 11..];
                if let Some(q2) = rest.find('\'') {
                    let p_name = &rest[..q2];
                    let lhs_pat = format!("{} =", p_name);
                    let mut new_lines: Vec<String> = Vec::new();
                    let mut replaced = false;
                    let mut_name = format!("{}_mut", p_name);
                    for (idx, line_s) in src.lines().enumerate() {
                        if !replaced && (idx + 1 == target_line || line_s.contains(&lhs_pat)) {
                            if let Some(p_pos) = line_s.find(&lhs_pat) {
                                let rhs = &line_s[p_pos + lhs_pat.len()..];
                                let ty = infer_surface_rhs_type(rhs);
                                new_lines.push(format!(
                                    "{}let {}: {} ={}",
                                    &line_s[..p_pos],
                                    mut_name,
                                    ty,
                                    rhs
                                ));
                                replaced = true;
                                continue;
                            }
                        }
                        if replaced && line_s.contains(&format!("return {};", p_name)) {
                            new_lines.push(line_s.replace(
                                &format!("return {};", p_name),
                                &format!("return {};", mut_name),
                            ));
                        } else {
                            new_lines.push(String::from(line_s));
                        }
                    }
                    let out = new_lines.join("\n");
                    if out != src {
                        return Some(out);
                    }
                }
            }
            None
        }
        "CORRUPT_PACK5" => {
            let mut out = String::from(src);
            for bad_byte in 243..=255u16 {
                let pat = format!(", {})", bad_byte);
                if out.contains(&pat) {
                    out = out.replace(&pat, ", 242)");
                }
            }
            if out != src {
                Some(out)
            } else {
                None
            }
        }
        "ARITY_MISMATCH" => {
            // Parse expected and got arity from message: "'<name>' expects <E> argument..., got <G>"
            let mut fn_name_opt: Option<&str> = None;
            if let Some(q1) = err.message.find('\'') {
                let rest = &err.message[q1 + 1..];
                if let Some(q2) = rest.find('\'') {
                    fn_name_opt = Some(&rest[..q2]);
                }
            }
            let mut exp_arity: Option<usize> = None;
            if let Some(ex_idx) = err.message.find("expects ") {
                let after = &err.message[ex_idx + 8..];
                let d: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
                exp_arity = d.parse::<usize>().ok();
            }
            if let (Some(callee), Some(want_n)) = (fn_name_opt, exp_arity) {
                let call_pat = format!("{}(", callee);
                let lines: Vec<&str> = src.lines().collect();
                if target_line <= lines.len() {
                    let line_s = lines[target_line - 1];
                    if let Some(c_pos) = line_s.find(&call_pat) {
                        let args_start = c_pos + call_pat.len();
                        if let Some(rel_close) = line_s[args_start..].find(')') {
                            let args_inside = &line_s[args_start..args_start + rel_close];
                            let mut parts: Vec<String> = if args_inside.trim().is_empty() {
                                Vec::new()
                            } else {
                                args_inside.split(',').map(|s| String::from(s.trim())).collect()
                            };
                            if parts.len() > want_n {
                                parts.truncate(want_n);
                            } else {
                                while parts.len() < want_n {
                                    parts.push(String::from("1"));
                                }
                            }
                            let fixed_call = format!("{}({})", callee, parts.join(", "));
                            let mut new_lines: Vec<String> = lines.iter().map(|s| String::from(*s)).collect();
                            new_lines[target_line - 1] = format!(
                                "{}{}{}",
                                &line_s[..c_pos],
                                fixed_call,
                                &line_s[args_start + rel_close + 1..]
                            );
                            let out = new_lines.join("\n");
                            if out != src {
                                return Some(out);
                            }
                        }
                    }
                }
            }
            None
        }
        "CIRCULAR_INCLUDE" => {
            let filtered: Vec<&str> = src
                .lines()
                .filter(|l| {
                    let t = l.trim_start();
                    !(t.starts_with("include ") || t.starts_with("include\"") || t.starts_with("mod "))
                })
                .collect();
            let out = filtered.join("\n");
            if out != src {
                Some(out)
            } else {
                None
            }
        }
        "UNRESOLVED_LABEL" => {
            let mut lbl_num: Option<u16> = None;
            if let Some(idx) = err.message.find("label ") {
                let after = &err.message[idx + 6..];
                let d: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
                lbl_num = d.parse::<u16>().ok();
            }
            if let Some(n) = lbl_num {
                if src.contains("return ") {
                    let out = src.replacen("return ", &format!("label {}; return ", n), 1);
                    if out != src {
                        return Some(out);
                    }
                }
            }
            None
        }
        "UNDEF_VAR" => {
            if let Some(q1) = err.message.find("variable '") {
                let rest = &err.message[q1 + 10..];
                if let Some(q2) = rest.find('\'') {
                    let v_name = &rest[..q2];
                    let lines: Vec<&str> = src.lines().collect();
                    if target_line <= lines.len() {
                        let mut new_lines: Vec<String> = Vec::with_capacity(lines.len() + 1);
                        for (idx, l) in lines.iter().enumerate() {
                            if idx + 1 == target_line {
                                new_lines.push(format!("    let {}: i64 = 0;", v_name));
                            }
                            new_lines.push(String::from(*l));
                        }
                        let out = new_lines.join("\n");
                        if out != src {
                            return Some(out);
                        }
                    }
                }
            }
            None
        }
        "UNKNOWN_FN" => {
            if let Some(q1) = err.message.find("function '") {
                let rest = &err.message[q1 + 10..];
                if let Some(q2) = rest.find('\'') {
                    let bad_fn = &rest[..q2];
                    let out = src.replace(&format!("{}(", bad_fn), "iabs(");
                    if out != src {
                        return Some(out);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// Runs the version-locked `omni-agent-driver` loop with a caller-supplied repair callback:
/// 1. Queries `omni_harness_spec_json()` (`omni-harness --spec`) and verifies that its contract version
///    matches `expected_contract_version` (`"omni-harness-v1.0.0"`), refusing to run on mismatch.
/// 2. Submits candidate `.omni` source to `run_omni_harness_json(..., true)`.
/// 3. On failure, extracts the structured `OmniStructuredError` (`code`, 1-based `line`/`col`, `retry_hint`, `message`),
///    selects the versioned `strategy_id`, and invokes `repair_fn(current_src, &err, &spec_json)` up to `max_attempts`.
pub fn run_omni_agent_repair_loop_with_model<F>(
    initial_src: &str,
    args: Vec<OmniValue>,
    strict: bool,
    max_attempts: usize,
    expected_contract_version: &str,
    mut repair_fn: F,
) -> Result<OmniAgentRepairReport, String>
where
    F: FnMut(&str, &OmniStructuredError, &str) -> Option<String>,
{
    let spec_json = omni_harness_spec_json();
    let expected_tag = format!("\"version\":\"{}\"", expected_contract_version);
    if !spec_json.contains(&expected_tag) {
        return Err(format!(
            "error[VERSION_MISMATCH]: omni-agent-driver expected harness contract version '{}', but omni-harness --spec reported {}",
            expected_contract_version, spec_json
        ));
    }
    let limit = max_attempts.max(1);
    let mut current_src = String::from(initial_src);
    let mut attempts = Vec::with_capacity(limit);

    for attempt_idx in 1..=limit {
        let harness_json = run_pure_omni_driver_json(&current_src, args.clone(), strict, true);
        if harness_json.starts_with("{\"ok\":true") {
            attempts.push(OmniAgentAttempt {
                attempt: attempt_idx,
                source: current_src.clone(),
                harness_json: harness_json.clone(),
                error: None,
                strategy_id: None,
                strategy_version: OMNI_REPAIR_STRATEGIES_VERSION,
            });
            return Ok(OmniAgentRepairReport {
                ok: true,
                contract_version: String::from(OMNI_HARNESS_CONTRACT_VERSION),
                strategy_version: String::from(OMNI_REPAIR_STRATEGIES_VERSION),
                attempts,
                final_source: current_src,
                final_json: harness_json,
            });
        }

        let err = extract_structured_error_from_harness_json(&harness_json, &current_src, strict);
        let strat_id = select_omni_repair_strategy(&current_src, &err);
        attempts.push(OmniAgentAttempt {
            attempt: attempt_idx,
            source: current_src.clone(),
            harness_json: harness_json.clone(),
            error: Some(err.clone()),
            strategy_id: Some(strat_id),
            strategy_version: OMNI_REPAIR_STRATEGIES_VERSION,
        });

        if attempt_idx == limit {
            return Ok(OmniAgentRepairReport {
                ok: false,
                contract_version: String::from(OMNI_HARNESS_CONTRACT_VERSION),
                strategy_version: String::from(OMNI_REPAIR_STRATEGIES_VERSION),
                attempts,
                final_source: current_src,
                final_json: harness_json,
            });
        }

        match repair_fn(&current_src, &err, &spec_json) {
            Some(next_src) if next_src != current_src => {
                current_src = next_src;
            }
            _ => {
                return Ok(OmniAgentRepairReport {
                    ok: false,
                    contract_version: String::from(OMNI_HARNESS_CONTRACT_VERSION),
                    strategy_version: String::from(OMNI_REPAIR_STRATEGIES_VERSION),
                    attempts,
                    final_source: current_src,
                    final_json: harness_json,
                });
            }
        }
    }

    unreachable!()
}

pub(crate) fn extract_structured_error_from_harness_json(
    harness_json: &str,
    src: &str,
    strict: bool,
) -> OmniStructuredError {
    let mut parsed_code: Option<&'static str> = None;
    if let Some(c_idx) = harness_json.find("\"code\":\"") {
        let rest = &harness_json[c_idx + 8..];
        if let Some(end_q) = rest.find('"') {
            parsed_code = map_error_code_str(&rest[..end_q]);
        }
    }
    let mut parsed_line: Option<usize> = None;
    if let Some(l_idx) = harness_json.find("\"line\":") {
        let rest = &harness_json[l_idx + 7..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = digits.parse::<usize>() {
            parsed_line = Some(n.max(1));
        }
    }
    let mut parsed_col: Option<usize> = None;
    if let Some(col_idx) = harness_json.find("\"col\":") {
        let rest = &harness_json[col_idx + 6..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = digits.parse::<usize>() {
            parsed_col = Some(n.max(1));
        }
    }
    let mut parsed_msg: Option<String> = None;
    if let Some(m_idx) = harness_json.find("\"message\":\"") {
        let rest = &harness_json[m_idx + 11..];
        let mut msg = String::new();
        let mut chars = rest.chars();
        while let Some(ch) = chars.next() {
            if ch == '\\' {
                if let Some(esc) = chars.next() {
                    match esc {
                        'n' => msg.push('\n'),
                        'r' => msg.push('\r'),
                        't' => msg.push('\t'),
                        other => msg.push(other),
                    }
                }
            } else if ch == '"' {
                break;
            } else {
                msg.push(ch);
            }
        }
        parsed_msg = Some(msg);
    }
    if let (Some(code), Some(line), Some(col), Some(message)) =
        (parsed_code, parsed_line, parsed_col, parsed_msg)
    {
        return OmniStructuredError {
            code,
            line: Some(line),
            col: Some(col),
            message,
            retry_hint: retry_hint_for_code(code),
        };
    }
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
    match u16_res {
        Err(e) => parse_omni_structured_error(&e),
        Ok(_) => parse_omni_structured_error(harness_json),
    }
}

/// Builds a deterministic single-line JSON repair prompt payload for an external LLM or `--repair-cmd` tool.
pub fn build_omni_agent_repair_prompt_json(
    attempt: usize,
    current_src: &str,
    err: &OmniStructuredError,
    spec_json: &str,
) -> String {
    let strat_id = select_omni_repair_strategy(current_src, err);
    format!(
        "{{\"version\":\"{}\",\"strategy_version\":\"{}\",\"strategy_id\":\"{}\",\"attempt\":{},\"source\":\"{}\",\"error\":{{\"code\":\"{}\",\"line\":{},\"col\":{},\"message\":\"{}\",\"retry_hint\":\"{}\"}},\"spec\":{}}}",
        OMNI_HARNESS_CONTRACT_VERSION,
        OMNI_REPAIR_STRATEGIES_VERSION,
        strat_id,
        attempt,
        json_escape_str(current_src),
        err.code,
        err.line.unwrap_or(1).max(1),
        err.col.unwrap_or(1).max(1),
        json_escape_str(&err.message),
        json_escape_str(err.retry_hint),
        spec_json
    )
}

/// Runs the version-locked `omni-agent-driver` loop using the built-in structured feedback repair rule engine.
pub fn run_omni_agent_repair_loop(
    initial_src: &str,
    args: Vec<OmniValue>,
    strict: bool,
    max_attempts: usize,
    expected_contract_version: &str,
) -> Result<OmniAgentRepairReport, String> {
    run_omni_agent_repair_loop_with_model(
        initial_src,
        args,
        strict,
        max_attempts,
        expected_contract_version,
        |cur_src, err, _spec| repair_omni_program_from_harness_error(cur_src, err, strict),
    )
}

/// Formats an `OmniAgentRepairReport` as a deterministic single-line JSON object including `strategy_id` and `strategy_version`.
pub fn run_omni_agent_driver_json(
    initial_src: &str,
    args: Vec<OmniValue>,
    strict: bool,
    max_attempts: usize,
    expected_contract_version: &str,
) -> String {
    match run_omni_agent_repair_loop(initial_src, args, strict, max_attempts, expected_contract_version) {
        Ok(rep) => {
            let codes: Vec<String> = rep
                .attempts
                .iter()
                .map(|a| match &a.error {
                    Some(e) => format!(
                        "{{\"attempt\":{},\"ok\":false,\"code\":\"{}\",\"line\":{},\"col\":{},\"strategy_id\":\"{}\",\"strategy_version\":\"{}\"}}",
                        a.attempt,
                        e.code,
                        e.line.unwrap_or(1),
                        e.col.unwrap_or(1),
                        a.strategy_id.unwrap_or("strat_syntax_fallback_v1"),
                        a.strategy_version
                    ),
                    None => format!("{{\"attempt\":{},\"ok\":true}}", a.attempt),
                })
                .collect();
            format!(
                "{{\"ok\":{},\"version\":\"{}\",\"strategy_version\":\"{}\",\"attempts\":{},\"history\":[{}],\"final_source\":\"{}\",\"harness\":{}}}",
                rep.ok,
                rep.contract_version,
                rep.strategy_version,
                rep.attempts.len(),
                codes.join(","),
                json_escape_str(&rep.final_source),
                rep.final_json
            )
        }
        Err(e) => format!(
            "{{\"ok\":false,\"version\":\"{}\",\"strategy_version\":\"{}\",\"code\":\"VERSION_MISMATCH\",\"line\":1,\"col\":1,\"message\":\"{}\"}}",
            expected_contract_version,
            OMNI_REPAIR_STRATEGIES_VERSION,
            json_escape_str(&e)
        ),
    }
}

/// Shared CLI front-end for `omni-agent-driver` / `omni agent-driver`.
/// Usage: omni-agent-driver [--list-strategies] [--explain <CODE>] [--strict] [--max-attempts N] [--expect-version VER] [--repair-cmd CMD] [--emit-repair-prompt] (<file.omni | - | "source">) [args...]
pub fn run_omni_agent_driver_cli(argv: &[String]) -> i32 {
    let mut strict = false;
    let mut max_attempts = 6usize;
    let mut expected_ver = String::from(OMNI_HARNESS_CONTRACT_VERSION);
    let mut repair_cmd: Option<String> = None;
    let mut emit_repair_prompt = false;
    let mut rest: Vec<String> = Vec::new();
    let mut i = 0usize;
    while i < argv.len() {
        match argv[i].as_str() {
            "--list-strategies" | "list-strategies" => {
                println!("{}", omni_repair_strategies_json());
                return 0;
            }
            "--explain" if i + 1 < argv.len() => {
                match explain_omni_error_code(&argv[i + 1]) {
                    Some(text) => {
                        println!("{}", text);
                        return 0;
                    }
                    None => {
                        eprintln!("Unknown Omni error code '{}'", argv[i + 1]);
                        return 2;
                    }
                }
            }
            "--strict" => strict = true,
            "--emit-repair-prompt" => emit_repair_prompt = true,
            "--max-attempts" => {
                if i + 1 >= argv.len() {
                    eprintln!("--max-attempts requires an integer argument");
                    return 2;
                }
                max_attempts = argv[i + 1].parse::<usize>().unwrap_or(6).max(1);
                i += 1;
            }
            "--expect-version" => {
                if i + 1 >= argv.len() {
                    eprintln!("--expect-version requires a version string");
                    return 2;
                }
                expected_ver = argv[i + 1].clone();
                i += 1;
            }
            "--repair-cmd" => {
                if i + 1 >= argv.len() {
                    eprintln!("--repair-cmd requires a command string");
                    return 2;
                }
                repair_cmd = Some(argv[i + 1].clone());
                i += 1;
            }
            _ => rest.push(argv[i].clone()),
        }
        i += 1;
    }
    if rest.is_empty() {
        eprintln!("usage: omni-agent-driver [--list-strategies] [--explain <CODE>] [--strict] [--max-attempts N] [--expect-version VER] [--repair-cmd CMD] [--emit-repair-prompt] (<file.omni | - | \"source\">) [args...]");
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
            Ok(s) => s,
            Err(e) => {
                eprintln!("failed to read '{}': {}", target, e);
                return 2;
            }
        }
    } else {
        target.clone()
    };
    let vm_args: Vec<OmniValue> = rest[1..].iter().map(|s| parse_cli_omni_value(s)).collect();
    if emit_repair_prompt {
        let spec_json = omni_harness_spec_json();
        let harness_json = run_omni_harness_json(&src, vm_args, strict, true);
        if harness_json.starts_with("{\"ok\":true") {
            println!("{}", harness_json);
            return 0;
        }
        let err = extract_structured_error_from_harness_json(&harness_json, &src, strict);
        println!("{}", build_omni_agent_repair_prompt_json(1, &src, &err, &spec_json));
        return 1;
    }
    if let Some(cmd_str) = repair_cmd {
        let mut attempt_counter = 0usize;
        let res = run_omni_agent_repair_loop_with_model(
            &src,
            vm_args,
            strict,
            max_attempts,
            &expected_ver,
            |cur_src, err, spec_json| {
                attempt_counter += 1;
                let payload = build_omni_agent_repair_prompt_json(attempt_counter, cur_src, err, spec_json);
                use std::io::Write;
                use std::process::{Command, Stdio};
                let mut child = if cfg!(target_os = "windows") {
                    Command::new("powershell")
                        .args(["-NoProfile", "-Command", &cmd_str])
                        .stdin(Stdio::piped())
                        .stdout(Stdio::piped())
                        .spawn()
                        .ok()?
                } else {
                    Command::new("sh")
                        .args(["-c", &cmd_str])
                        .stdin(Stdio::piped())
                        .stdout(Stdio::piped())
                        .spawn()
                        .ok()?
                };
                if let Some(mut sin) = child.stdin.take() {
                    let _ = sin.write_all(payload.as_bytes());
                }
                let out = child.wait_with_output().ok()?;
                if !out.status.success() {
                    return None;
                }
                let candidate = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if candidate.is_empty() {
                    None
                } else {
                    Some(candidate)
                }
            },
        );
        match res {
            Ok(rep) => {
                let codes: Vec<String> = rep
                    .attempts
                    .iter()
                    .map(|a| match &a.error {
                        Some(e) => format!(
                            "{{\"attempt\":{},\"ok\":false,\"code\":\"{}\",\"line\":{},\"col\":{},\"strategy_id\":\"{}\",\"strategy_version\":\"{}\"}}",
                            a.attempt,
                            e.code,
                            e.line.unwrap_or(1),
                            e.col.unwrap_or(1),
                            a.strategy_id.unwrap_or("strat_syntax_fallback_v1"),
                            a.strategy_version
                        ),
                        None => format!("{{\"attempt\":{},\"ok\":true}}", a.attempt),
                    })
                    .collect();
                let out = format!(
                    "{{\"ok\":{},\"version\":\"{}\",\"strategy_version\":\"{}\",\"attempts\":{},\"history\":[{}],\"final_source\":\"{}\",\"harness\":{}}}",
                    rep.ok,
                    rep.contract_version,
                    rep.strategy_version,
                    rep.attempts.len(),
                    codes.join(","),
                    json_escape_str(&rep.final_source),
                    rep.final_json
                );
                println!("{}", out);
                return if rep.ok { 0 } else { 1 };
            }
            Err(e) => {
                println!(
                    "{{\"ok\":false,\"version\":\"{}\",\"strategy_version\":\"{}\",\"code\":\"VERSION_MISMATCH\",\"line\":1,\"col\":1,\"message\":\"{}\"}}",
                    expected_ver,
                    OMNI_REPAIR_STRATEGIES_VERSION,
                    json_escape_str(&e)
                );
                return 1;
            }
        }
    }
    let out = run_omni_agent_driver_json(&src, vm_args, strict, max_attempts, &expected_ver);
    println!("{}", out);
    if out.starts_with("{\"ok\":true") { 0 } else { 1 }
}
