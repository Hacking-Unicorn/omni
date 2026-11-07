# Omni Machine-Readable Error Contract (`omni-error-contract-v1.0.0`)

**Contract Version:** `omni-error-contract-v1.0.0`  
**Harness Contract:** `omni-harness-v1.0.0`  
**Repair Strategy Set:** `omni-repair-strategies-v1.1.0`

## 1. Canonical Format & Non-Null Span Guarantee

Every compiler, static verifier, `@PACK5` validator, and runtime budget failure emits a deterministic diagnostic matching:

```text
error[<CODE>] line <L>, col <C>: <message>
```

When formatted by `omni-harness` (`run_omni_harness_json`), every failure is serialized as a single-line JSON object where `"line"` and `"col"` are **guaranteed non-null 1-based integers** (`>= 1`):

```json
{"ok":false,"version":"omni-harness-v1.0.0","code":"<CODE>","line":2,"col":13,"message":"error[<CODE>] line 2, col 13: ...","tokens":null,"retry_hint":"..."}
```

## 2. Complete 16-Code Table

| # | Code | Stage | Format / Trigger | Non-Null `(line, col)` | Default Strategy (`omni-repair-strategies-v1.1.0`) |
|---|---|---|---|---|---|
| 1 | `TAINT_ON_RET` | Static Verifier | `@UNTRUSTED` value reaches `RET` (`0x7F4A`) without sanitizer | Yes (`SurfaceSourceMap::pkt_spans`) | `strat_taint_ret_sanitize_v1` |
| 2 | `TAINT_SINK` | Static Verifier | `@UNTRUSTED` operand reaches safe-only sink (`egate`, `db_query`, `kv_put`, `respond_html`) | Yes (`SurfaceSourceMap::pkt_spans`) | `strat_taint_sink_sanitize_v1` |
| 3 | `MISSING_CAP` | Static Verifier / Narrow Host | Missing `#[cap(...)]` bitmask for capability-gated opcode or `OmniNarrowHost` syscall | Yes (`SurfaceSourceMap::pkt_spans`) | `strat_missing_cap_minimal_v1` / `strat_multi_fn_callee_cap_v1` |
| 4 | `LINEAR_LEAK` | Static Verifier | Linear `arena(N)` (`0x7F01`) not released via `drop(a)` (`0x7F05`) before `RET` | Yes (`SurfaceSourceMap::pkt_spans`) | `strat_linear_leak_drop_v1` |
| 5 | `BORROW_CONFLICT` | Static Verifier | `drop(a)` executed while `borrow_r(a)` or `borrow_w(a)` is still active | Yes (`SurfaceSourceMap::pkt_spans`) | `strat_borrow_conflict_reorder_v1` |
| 6 | `UNDEF_VAR` | Surface Lowering / Macro Hygiene | Reference to undeclared identifier or uncaptured variable inside a hygienic `macro` | Yes (Lexer/Parser span) | `strat_undef_var_bind_zero_v1` |
| 7 | `IMMUT_PARAM` | Surface Lowering | Reassignment (`x = ...`) of an immutable function parameter | Yes (Statement span) | `strat_immut_param_shadow_v1` |
| 8 | `BAD_TYPE` | Surface Parser / Lowering | Unknown type name or missing explicit `: <ty>` on `let` in `#[strict]` mode | Yes (`let` / type token span) | `strat_bad_type_infer_let_v1` |
| 9 | `UNKNOWN_FN` | Surface Lowering | Call to unknown surface intrinsic or undefined function | Yes (Call expression span) | `strat_unknown_fn_intrinsic_v1` |
| 10 | `ARITY_MISMATCH` | Surface Lowering | Function, intrinsic, or hygienic `macro` called with wrong argument count | Yes (Call expression span) | `strat_multi_fn_arity_align_v1` |
| 11 | `UNRESOLVED_LABEL` | Surface Lowering | `goto N;` targets a basic-block label `N` that was never defined with `label N;` | Yes (`goto` span) | `strat_unresolved_label_insert_v1` |
| 12 | `CIRCULAR_INCLUDE` | Include Linker | Cycle detected in `include "file.omni";` graph | Yes (`include` directive span) | `strat_circular_include_prune_v1` |
| 13 | `BUDGET_EXCEEDED` | VM / AOT Runtime / Lowering | `#[step_budget(N)]`, `#[mem_budget(N)]`, register (`192`), or label (`15`) limit exceeded | Yes (`SurfaceSourceMap::budget_span`) | `strat_budget_step_raise_v1` / `strat_budget_mem_raise_v1` |
| 14 | `CORRUPT_PACK5` | `@PACK5` Decoder (VM / AOT) | Packed `@PACK5` byte in sentinel range `243..=255` (`> 3^5 - 1`) | Yes (`SurfaceSourceMap::pack5_span`) | `strat_corrupt_pack5_clamp_v1` |
| 15 | `AMBIGUOUS_GRAMMAR` | Strict Lexer / Parser | Non-canonical alias (`TRIT_POS`, `int`, `tvec`, `untrusted_i64`, `mod`) in `#[strict]` mode | Yes (Token span) | `strat_strict_canonicalize_v1` |
| 16 | `SYNTAX_ERROR` | Surface Lexer / Parser | Malformed token, unbalanced delimiter, or missing included file | Yes (Token span) | `strat_syntax_fallback_v1` |
