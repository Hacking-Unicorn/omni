# Milestone 3 plan: the lexer, parser and surface frontend in iterative Omni

Status: **plan, not implemented.** Copied from Scarlett Truth's M3 design review.
- The `FE:` / `M:` line citations below were taken at `74a215c`.
- This plan is committed on `scarlett/m3-groundwork`, branched from `d2d6c13`. That commit adds 154 lines to `compiler/mod.rs`, so citations past FE:~1900 may be shifted by a few dozen lines. Re-check them before relying on them.
- Decisions and open items are in `spec/m3/DECISIONS.md`.

## 0. Settled decisions vs open items

| # | Decision (from Boss Lady / Caleb, treated as fixed) | Consequence here |
|---|---|---|
| S-1 | M3 lives in **`pure/m3/`**. `pure/m2/` belongs to M2. `pure/compiler.omni` and the 18-module / 14,559-word suite are frozen. | No edits to `pure/compiler.omni`, `PURE_OMNI_CORE_SUITE_MODULES` (`pure_omni/mod.rs:1106-1125`), or any module in it. M3 only **reads** `pure/m2/*.omni` via `include`. |
| S-2 | The 16-label cap applies everywhere. The one named legacy exemption is the Rust-built Stage-1 stream. | M3 takes **no** exemption. |
| S-3 | Per Nyx's measurement, **15 labels per function** is the usable limit until the emitter question is decided. `alloc_lbl` starts at `next_lbl: 1` (FE:1585) and errors at `next_lbl > 15` (FE:1644-1654). | Every M3 function is designed for **≤ 15 labels** (target ≤ 12, M2 lint), and ≤ 192 SSA (target ≤ 120). See O-1. |
| S-4 | No recursion in core `.omni`. Call depth ≤ 64. Eager args. No heap structs, maps or GC. The opcode table 0x7F00-0x7F7F is never renumbered. Frozen hashes don't move. | §3, §4 |

Open items are in §9.

## 1. What is being ported (Rust inventory)

| Rust piece | FE lines @74a215c | M mirror | Role |
|---|---|---|---|
| Token / AST types `SurfaceTok`, `SpannedSurfaceTok`, `SurfaceFnDef`, `SurfaceExpr`, `SurfaceStmt` | FE:26-129 | M:2028-2131 | Heap `String`/`Box`/`Vec`. Becomes flat arena records (§3). |
| `tokenize_omni_surface_inner` | FE:132-325 (194) | M:2134 | Lexer |
| `check_macro_body_hygiene` / `substitute_macro_expr` | FE:352-402 / 405-515 | M:2354 / 2407 | Macros. **Self-recursive.** |
| `SurfaceParser` helpers (`peek`, `next_spanned`, `expect_*`, `consume_at_annotations`, `parse_route_path`) | FE:517-671 | M:2519- | Parser support. Lookahead is at most 2 tokens (FE:1120 `tokens.get(self.pos + 1)`), with no backtracking. |
| `parse_module` | FE:673-761 (89) | M:2675 | Top level: `include`/`mod`, `macro`, `fn` |
| `parse_fn_def` | FE:763-1015 (253) | M:2765 | Attributes, route, params, ret, body |
| `parse_block` / `parse_stmt` | FE:1017-1025 / 1027-1170 (144) | M:3029 | Statements |
| `parse_expr` → `parse_cmp` → `parse_bitor` → `parse_bitxor` → `parse_bitand` → `parse_shift` → `parse_add` → `parse_mul` → `parse_unary` | FE:1172-1315 | M:3174- | 8-level precedence chain, all left-associative. `parse_unary` is self-recursive. |
| `parse_primary` | FE:1317-1510 (194) | M:3319 | Literals, calls, `if_pos`-family, `match_trit`, parens, macro use. It recurses back to `parse_expr`/`parse_block`. |
| `map_omni_surface_type_inner`, `fnv1a_32_positive` | FE:1513-1546 | | Type names, name hash |
| `SurfaceLowerCtx` + `emit`, `alloc_reg`, `alloc_lbl` | FE:1548-1654 | M:3550- | Lowering state. `BTreeMap` var map + taint set (FE:1549-1550). |
| `eval_to_operand`, `lower_trit_vec_pack5`, `lower_call_rhs`, `ensure_contiguous_join_slot`, `lower_block_into_target`, `lower_expr_into`, `lower_stmt`, `check_unresolved_labels` | FE:1656-2419 | M:3658-4421 | Lowering. **Mutually recursive.** |
| `lookup_omni_stdlib_module` (embedded sources) | FE:2423-2980 (558) | M:4425 | 22 keys / 20 bodies, two of which are `spec/pure_self_host` aliases (FE:2467, FE:2596) |
| `resolve_omni_surface_includes` / `resolve_inner` | FE:2983-3052 | M:4985 | Textual include expansion. **Self-recursive** DFS. |
| `compile_omni_surface_to_asm_inner` / `compile_omni_surface_to_u16_with_sourcemap` | FE:3054-3135 / 3199-3346 | M:5201 | Drivers, which duplicate each other |
| `encode_asm_ins_packet_u16` | `omni-assembler/src/lib.rs:27` (mnemonic table :29-97) | M:1762 | asm text line → u16 packet. Called from `emit` (FE:1597-1630). |

Cycles that must become explicit stacks:
- parse_expr → … → parse_primary → parse_expr / parse_block → parse_stmt → parse_expr.
- lower_expr_into ↔ eval_to_operand ↔ lower_call_rhs.
- lower_expr_into → lower_block_into_target → lower_stmt → lower_expr_into.
- substitute_macro_expr, check_macro_body_hygiene and resolve_inner, each self-recursive.

Error classes the frontend emits (FE:132-3346): `SYNTAX_ERROR`×27, `ARITY_MISMATCH`×19, `UNRESOLVED_LABEL`×7, `BAD_TYPE`×7, `AMBIGUOUS_GRAMMAR`×6, `UNDEF_VAR`×5, `BUDGET_EXCEEDED`×4, and one each of `UNKNOWN_FN`, `MISSING_CAP`, `IMMUT_PARAM`, `CIRCULAR_INCLUDE`.

Behaviours parity must reproduce exactly (each one is a test case in M3-01):

| # | Quirk | Where |
|---|---|---|
| Q1 | `effective_strict = strict \|\| src.contains("#[strict]")` is a raw substring test, so it also fires inside comments and strings | FE:3203, FE:3055 |
| Q2 | `+1t` yields `Trit("+1",1)`, but `-1t` yields `Trit("-1t",-1)`. `0t` is a trit only if it isn't followed by an identifier char. | FE:228-246 |
| Q3 | Unary `-` on an `Int` result folds with `wrapping_neg`, including through parens and `- -5` | FE:1290-1296 |
| Q4 | `<=` / `>=` desugar to `TNOT(CGT)` / `TNOT(CLT)` with the operator's span | FE:1195-1206 |
| Q5 | Macros are inserted when the definition is parsed, so a use before the definition stays a call | FE:705-748 |
| Q6 | Includes are textual. There's no de-duplication (a diamond include is included twice). Lines whose `trim_start()` starts with `include ` / `include"` / `mod ` are dropped, and the result is re-lexed and re-parsed from scratch. | FE:2988-3048, FE:3207-3211 |
| Q7 | fn id: the attr if present, else 0 for single-fn or the last fn, else idx+1 | FE:3217-3223 |
| Q8 | `next_reg > 192` / `next_lbl > 15` errors with `BUDGET_EXCEEDED` at the current span | FE:1632-1654 |

## 2. Rust → new `pure/m3/` modules

Concatenation order puts `m3_main` last, because the last-declared function is the entry (FE:3069-3075 rule, as in M2).

| New module | Replaces (Rust) | Contents |
|---|---|---|
| `pure/m3/arena.omni` | (new) | `m3_init`, region constants, `g_get/g_set`, bounds checks, pool append/compare, error set |
| `pure/m3/isa.omni` | `omni-isa` table + assembler mnemonic table (`omni-assembler/src/lib.rs:29-97`) | rodata builder: mnemonic id → (opcode, result type), operand-kind table. **Reads** `pure/m2/decode.omni` / `util.omni` via `include` and doesn't edit them. |
| `pure/m3/packet.omni` | `encode_asm_ins_packet_u16` (`omni-assembler/src/lib.rs:27-`) | Structured packet encoder: (dest, mnemonic id, operand records) → u16 tokens. No text. |
| `pure/m3/lex.omni`, `lex_tok.omni` | FE:132-325 | Streaming lexer: a 4-slot token ring, class-table dispatch |
| `pure/m3/skim.omni` | `parse_module` FE:673-761, `parse_fn_def` header part FE:763-1015, `consume_at_annotations` FE:611-634, `parse_route_path` FE:636-671, signature loop FE:3216-3237 | Pass S: include list, macro defs, fn headers → fn table; body skip by brace depth |
| `pure/m3/expr.omni` | FE:1172-1315 | Explicit-stack precedence parser (operator/operand stacks) |
| `pure/m3/primary.omni` | FE:1317-1510 | Primary forms as parse-stack states |
| `pure/m3/stmt.omni` | FE:1017-1170 | Statement/block states |
| `pure/m3/macro.omni` | FE:352-515 | Iterative hygiene check + iterative AST clone/substitute |
| `pure/m3/symtab.omni` | `BTreeMap`s in `SurfaceLowerCtx` (FE:1549-1551) and fn_sigs | Linear-scan tables (fnv32 used as a compare key, no buckets), with insert-overwrite semantics to match `BTreeMap::insert` |
| `pure/m3/lower_expr.omni` | FE:1656-1713, 1976-2239 | Work-stack post-order lowering |
| `pure/m3/lower_call.omni` | FE:1715-1973 | Table-driven intrinsics (~110 names) + user calls |
| `pure/m3/lower_stmt.omni` | FE:2241-2419 | Statement lowering, labels/goto, unresolved-label check |
| `pure/m3/pack.omni` | `pack_fn_module` (omni-ir) + FE:3240-3346 | Module/function headers, back-patched lengths, sourcemap |
| `pure/m3/driver.omni` | FE:3199-3346 | Passes L/S/P/B, `m3_main` (last), output slice |

Rust stays in Stage-0 for now:
- **Include resolution** (FE:2983-3052): `pure` code has no filesystem. The host expands includes and hands M3 one source buffer, which is exactly what Rust does internally today (Q6). Porting the resolver over a "source bundle" is a later milestone (O-5).
- The `emit_asm` text mode for `omnic asm`.

## 3. Arena layout and handoff with M2

**Handoff (recommended default): M3 and M2 are separate programs with separate arenas.**
- M3's input is source bytes. M3's output is a u16 token stream plus a sourcemap, returned with M2-06's `RET buf, off, len`.
- The host (Stage-0 Rust, later an Omni driver) passes M3's stream slice as M2's param-0 input. M2's input logical `0x00000-0x0FFFF` is then exactly M3's output words.
- Nothing is shared, so nothing can overlap.
- Input:
  - VM: `Bytes(source)`, the same way the verifier engine takes `Bytes` (`verifier/mod.rs:474-500`).
  - Native: the M2-09 input window, as u16 words, 2 source bytes per word, padded with NUL to even length (O-4).

**Non-overlap even if they're fused later.** M3's logical addresses use their own band, **`0x80000-0xFFFFF`**, which is disjoint from every M2 region:
- tokens/input `0x00000`
- labels `0x10000-0x13FFF`
- fn tables / register state / globals `0x14000-0x17FFF`
- output `0x18000-`

In handoff mode, physical = logical − `0x80000`. If the two are ever fused into one arena, M2's output must stay below `0x80000`, which caps 2N ≤ `0x68000`. That decision belongs to a future fusion, not M3.

M3 band (offsets are relative to `0x80000`). Every mutable field is u16/u32/u64 because the patch shapes are only `-2`/`-4` (4-op, C1, planned) and `-9` (shipped). There is **no byte patch**, so byte tables (char classes) are built by append-only init.

| Rel offset | Size | Contents | Bound |
|---|---|---|---|
| `0x00000-0x007FF` | 2 KiB | globals: 256 × i64 (pass, src cursor, line/col, strict, fn index, next_reg, next_lbl, error code/line/col, pool tops, stack tops, out_len) | fixed |
| `0x00800-0x00FFF` | 2 KiB | rodata: 256-B char-class table, keyword ids, type-name table | init |
| `0x01000-0x01FFF` | 4 KiB | rodata: 128 × 8 B opcode table + 81 real mnemonics × 16 B (`pure/isa/table.omni` at `raven/isa-core-port` `bf7f212`; ids 0..80 frozen, new ids only appended at 81+; 81-row deviation from the original 128 × 16 B plan, approved by Boss Lady) + intrinsic table (111 slots × 16 B available, floor 110: name ref, mnemonic id, arity, class). Free room: (4096 − 81×16 − 128×8) / 16 = 111. | init |
| `0x02000-0x02FFF` | 4 KiB | rodata string pool (keywords, intrinsic names) | init |
| `0x03000-0x04FFF` | 8 KiB | **parse stack**: 512 frames × 16 B `{state u16, prec u16, node u32, aux u32, line u16, col u16}` | depth ≤ 512, else `NEST_LIMIT` (O-6) |
| `0x05000-0x06FFF` | 8 KiB | **lowering work stack**: 512 × 16 B `{node u32, phase u16, dest u16, aux u32, lbl u16, pad}` | ≤ 512 |
| `0x07000-0x07DFF` | 3.5 KiB | per-fn var table: 224 × 16 B `{name ref u32, fnv32 u32, operand u16, taint u16, pad}` (192 REG + 32 PAR) | reset per fn |
| `0x07E00-0x07FFF` | 512 B | per-fn labels: defined bitmask (u16), used list 63 × `{id,line,col}` u16 | reset per fn |
| `0x08000-0x0BFFF` | 16 KiB | fn table: 1024 × 16 B `{name ref u32, fid u16, arity u16, ret ty u16, src off u32, flags u16}` | ≤ 1024 (M2's cap) |
| `0x0C000-0x0DFFF` | 8 KiB | macro table: 128 × 64 B `{name ref, def tok offset, nparams, params[8], body root}` | ≤ 128 |
| `0x0E000-0x0E0FF` | 256 B | token ring: 4 × 16 B `{kind u16, sub u16, line u16, col u16, val i64}` + spare | fixed |
| `0x10000-0x17FFF` | 32 KiB | ident/string pool (persistent part: fn/macro names; per-fn part truncated after each fn) | ≤ 32 KiB |
| `0x18000-0x1BFFF` | 16 KiB | per-fn AST node pool: 1024 × 16 B `{kind u8+op u8 packed in u16, line u16, col u16, k0,k1,k2,k3 u16, next u16}`. Int payloads go in `k0..k3` as an i64. | reset per fn |
| `0x1C000-0x1DFFF` | 8 KiB | macro AST pool (persistent): 512 nodes | ≤ 512 |
| `0x1E000-0x1FFFF` | 8 KiB | sourcemap staging: 2048 × `{line u16, col u16}` per pkt | per module |
| `0x20000-…` | grows | **output**: header `{stream_words u32, sm_entries u32}`, then the stream, then the sourcemap. Append-only; fn/module length words back-patched with 4-op `ISET -2/-4`. | `0x20000 + out ≤ cap`, checked on every append |


**Intrinsic headroom.** The floor of 110 leaves only 1 slot of headroom. M3-11 must count its exact intrinsic names before coding. If there are more than 111, the overflow moves to a new rodata block, decided in DECISIONS.

**13BA coverage.** Only the workspace copy of 13BA reads the real `omni-isa` / `omni-assembler` crates; the monolith copy checks against mirror tables inside the monolith.

The pre-filled prefix is 128 KiB. That was chosen on purpose: Nyx measured that a 256 KiB VM fill takes about 0.72 s, because the VM clones on every register read (§8 R-2).

**Why streaming and four passes.** M3's own source will likely be about 3-5k lines, more than 64 KiB, so a whole-module token buffer won't fit (verifier_engine.omni alone is 32,942 B ≈ 9k tokens, by a rough regex count). So the lexer streams. To keep Rust's **error order**, which is all lex errors, then all parse errors, then signature checks, then lowering:

| Pass | What | Stores |
|---|---|---|
| L | Lex the whole source, discard tokens. Also does the raw `#[strict]` scan (Q1). | nothing |
| S | Skim: includes (error if any: the host must have expanded them), macro defs (with def offset, for Q5), fn headers → fn table. Bodies are skipped by brace depth. | fn + macro tables |
| P | Parse every body into the per-fn pool and discard it. This reports parse errors in source order. | nothing |
| — | Signature checks: types, duplicates, fn ids (FE:3216-3237) | fn table |
| B | For each fn: re-lex from its `src off`, parse into the pool, lower, encode, pack, reset the pool | output |

## 4. Recursion → flat-arena stacks, call depth ≤ 64

- **Expressions** use a precedence-table parser with an operand stack (node ids) and an operator stack (op id, prec, span) inside the parse stack.
  - Precedences: cmp 1, bitor 2, bitxor 3, bitand 4, shift 5, add 6, mul 7, unary-prefix 8.
  - All binary levels are left-associative (each `parse_*` is a `while` loop), so "reduce while top.prec ≥ incoming.prec" gives the same tree as FE:1178-1287.
  - Q3/Q4 are applied at reduce time. Prefix ops are pushed as unary frames and reduced when their operand completes.
- **Primaries that contain blocks** (`if_pos`-family, `match_trit`, call args, parens) push a frame with a state code:
  - `CALL_ARGS(n)`, `PAREN`
  - `IF_COND/IF_THEN/IF_ELSE`
  - `MATCH_SCRUT/ARM_NEG/ARM_ZERO/ARM_POS`
  - `BLOCK(stmt list tail)`
  - `STMT_LET/ASSIGN/WHILE_COND/WHILE_BODY/DO_BODY/DO_COND/RET`

  The parser is one `while` loop in `parse_run`: read the top frame, then call the one handler that state class needs, via class dispatch (§5). Handlers return the new stack top and never call each other recursively.
- **Lowering** is an explicit post-order walk on the work stack, with `{node, phase}` frames.
  - The phase order must reproduce Rust's `alloc_reg`/`alloc_lbl`/`emit` order **exactly**: a BinOp evaluates lhs, then rhs, then emits; `if_pos` allocates its labels in Rust's order before the condition…
  - The packet-level differential (M3-10..12) proves it.
- **Macros:** hygiene is an iterative tree walk. Substitution is an iterative clone from the macro pool into the per-fn pool with an arg map.
- **Call depth:** the static call graph is acyclic by construction. The deepest expected chain is about 12: `m3_main → pass_b → fn_loop → parse_run → stmt_class → stmt_let → expr_reduce → ast_new → pool_put → st_u16`. This is enforced by running M2-08's module check (acyclic, ≤ 64 frames) and `m2-lint` on `pure/m3/*.omni` (M3-02).
- **No `sel_trit` over side effects.** Args are eager, so any arm that writes the arena uses `if_pos`/`match_trit` and pays for the labels.

## 5. Function splits: ≤ 15 labels, ≤ 192 SSA (target ≤ 12 / ≤ 120)

Label costs (Nyx, measured; M2 plan §5.1): value `if_pos` 2, `match_trit` 4, `while` 3, `do_while` 1, `sel_trit` 0. A three-level 3-way dispatch needs 4 `match_trit` = 16 labels, which is over the limit. So dispatch is always `class = table[x]`, then at most 2 `match_trit` levels per function (≤ 12 labels), with deeper fan-out done by calling separate functions.

"Naive" means a 1:1 port of the Rust control flow, estimated as 2·`if` + 3·loops + 4·3-way `match`. SSA is estimated from `let` count plus about 3 per branch. These are **estimates** until `m2-lint` reports real numbers.

| Rust fn (FE @74a215c) | Lines | if / loop / match | Naive labels / SSA | M3 split | Each (est.) |
|---|---:|---|---|---|---|
| `tokenize_omni_surface_inner` 132-325 | 194 | 25 / 7 / 0 | ~71 / ~150 | `lex_next` (class dispatch), `lex_skip_ws_comment`, `lex_hash_attr`, `lex_string`, `lex_at_ident`, `lex_op3_trit` (`<=>`,`+1t`,`-1t`,`0t`), `lex_op2`, `lex_sym_op1`, `lex_int`, `lex_ident_kw` (+ strict `TRIT_*`) — 10 fns | ≤ 9 lbl, ≤ 60 SSA |
| `parse_fn_def` 763-1015 | 253 | 24 / 4 / 2 | ~66 / ~170 | `hdr_attrs`, `attr_fn_id`, `attr_budget`, `attr_caps`, `hdr_route`, `hdr_param_step`, `hdr_ret`, `skip_body` — 8 | ≤ 10 / ≤ 70 |
| `parse_stmt` 1027-1170 | 144 | 22 / 1 / 3 (14 arms) | ~60 / ~140 | `stmt_class`, `st_let`, `st_assign_compound`, `st_while`, `st_do_while`, `st_label_goto`, `st_ret` — 7 | ≤ 10 / ≤ 60 |
| `parse_primary` 1317-1510 | 194 | 22 / 5 / 3 (17 arms) | ~65 / ~150 | `prim_class`, `prim_lit`, `prim_tritvec`, `prim_ident_call`, `prim_call_args`, `prim_if_family`, `prim_match_trit`, `prim_macro_use` — 8 | ≤ 10 / ≤ 70 |
| `parse_cmp`…`parse_unary` 1178-1315 | 122 (7 fns) | recursive chain | n/a | `ex_operand`, `ex_on_op` (prec from table: 0 lbl), `ex_reduce_one`, `ex_finish` — 4 | ≤ 8 / ≤ 50 |
| `parse_module` 673-761 | 89 | 13 / 2 / 1 | ~35 / ~90 | `top_step` (include/mod/macro/fn), `macro_def` — 2 | ≤ 10 / ≤ 60 |
| `substitute_macro_expr` 405-515 | 111 | self-recursive | n/a | `mac_clone_step`, `mac_map_arg` — 2 | ≤ 8 / ≤ 50 |
| `check_macro_body_hygiene` 352-402 | 51 | self-recursive | n/a | `hyg_step` — 1 | ≤ 8 / ≤ 40 |
| `lower_call_rhs` 1715-1973 | 259 | 26 / 2 / 5 (59 string arms, ~110 names) | ~80 / ~180 | `lc_lookup` (linear scan of intrinsic table), `lc_user_call` (arity, `CALL_*`), `lc_class` (2-level), `lc_unary`, `lc_binary`, `lc_host_args` (`eval_host_arg_to_operand` FE:1669), `lc_bytes` (iset/ld/st), `lc_pack5` — 8 | ≤ 12 / ≤ 90 |
| `lower_expr_into` 2052-2239 | 188 | 11 / 3 / 10 (28 arms) | ~70 / ~160 | `lx_enter` (kind class), `lx_binop_post`, `lx_unop_post`, `lx_if_pre/mid/post`, `lx_match_pre/arm/post`, `lx_tritvec`, `lx_join_slot` (FE:2000-2024) — 11 | ≤ 10 / ≤ 70 |
| `lower_stmt` 2241-2407 | 167 | 14 / 6 / 1 | ~50 / ~130 | `ls_class`, `ls_let`, `ls_assign`, `ls_while_pre/post`, `ls_do_while`, `ls_label_goto`, `ls_ret`, `ls_check_labels` (FE:2409) — 9 | ≤ 10 / ≤ 60 |
| `compile_omni_surface_to_u16_with_sourcemap` 3199-3346 | 148 | 11 / 10 / 1 | ~55 / ~140 | `m3_main` (straight-line: init, L, S, sig, P, B, ret_slice: 0 lbl), `pass_*` (1 `while` each = 3), `sig_check_step`, `pack_fn_hdr`, `pack_mod` — about 8 | ≤ 6 / ≤ 60 |
| `encode_asm_ins_packet_u16` (assembler :27-~216) | ~190 | big `match` + operand parse | ~40 / ~120 | `pk_dest`, `pk_op` (table), `pk_operand` (kind class: reg/par/fn/str/imm16/imm32/lbl), `pk_str_span`, `pk_end` — 5 | ≤ 10 / ≤ 60 |

Estimated size: about 80-100 Omni functions and roughly 3-5k lines. That's well within M2's 1024-function cap.

## 6. Cutover from embedded literals to on-disk sources without moving a frozen hash

Today `lookup_omni_stdlib_module` (FE:2423-2980) returns Rust string literals. The include resolver prefers them over disk (FE:3014). The suite builder compiles them (`pure_omni/mod.rs:1127-1150`). The frozen values hash **bytecode**, not source (`pure_omni/mod.rs:872-874`), so a source swap is safe exactly when the token stream is identical.

Plan (M3-16, Rust only, separate commit, no `.omni` edits):
1. **Pre-lock.** Add a selftest that records, for all 22 keys, `(words, FNV-64)` of the strict compile of the literal, plus suite 14,559 w `0x78462AE4C81DF408`, Phase D 159 w `0x23B8D17257CCA9C4`, 824 B `0xE221085777B8AC47`, and Stage-1 106,242 B `0xEAFC81F46B482FF8`. This reuses M2-01's guard.
2. **Equivalence proof.** For each key, the disk file must give the same token kinds and values as the literal (spans may differ) **and** the same u16 stream. It must also give the same result for Q1: a `#[strict]` appearing in a disk comment would flip non-strict includes. I showed in task 1 that the disk copies differ only in comments and whitespace. The token-level identity is **unverified** until this test runs.
3. **Swap per module.** Replace the literal with `include_str!` of the disk path, one module per key, in the workspace and in M (`M:4425`, relative path from repo root).
   - The two aliases keep pointing at the **`pure/`** files (FE:2467, FE:2596), not the drifted `spec/pure_self_host/*.omni`.
   - The `PHASE_A..D` consts in `pure_omni/mod.rs:31,209,586,782` are out of scope and stay.
   - Any module that fails step 2 keeps its literal and is listed as an open item. No `.omni` file is edited to make it pass.
4. **Post-check.** All step-1 values are byte-identical. Only sourcemap line/col values for code inside stdlib includes change, and that's documented.
5. After this, M3's corpus reads the same bytes Rust compiles, so there's a single source of truth.

## 7. Parity tests

Corpus:
- the 40 files in `spec/acceptance/goldens/`
- the 18 suite modules via the lookup (strict)
- `pure/verifier_engine.omni` (1,034 lines)
- the Phase A-D surface consts
- the `if_pos` join-slot selftests (FE:3799-3893)
- `pure/m2/*.omni` once it exists
- `pure/m3/*.omni` itself
- a negative corpus with one file per error class (§1) plus Q1-Q8

Ladder:
1. **Canonical dumps from Rust (M3-01):** `omnic fe-dump --tokens|--ast|--packets|--stream` with fixed binary formats (token record = the §3 ring record; AST = preorder node records; packets = u16 lists + span). Manifest: `spec/m3/corpus.tsv` (name, stage, words, FNV-64).
2. **Stage diffs on the VM:** M3 tokens == Rust tokens (M3-06), AST == AST (M3-08/09), packets == packets (M3-04 on Rust's own asm lines; M3-10..12 from source), final stream + sourcemap == `compile_omni_surface_to_u16_with_sourcemap` (M3-13). Errors are compared as `(class, line, col)`. Message text is out of scope (O-7).
3. **Lockstep in the product (M3-14):** a `lockstep_shadow_frontend(path, src)` modeled on `lockstep_shadow_verify` (`verifier/mod.rs:680-703`): Rust compiles, M3 compiles on the VM, and any disagreement is an error. It's opt-in first, then default once the corpus passes. It has the same suspend/inject hooks as the verifier's.
4. **4 backends (M3-15):** the M3 image itself runs on (a) the VM, (b) the Rust x86 multi-function AOT (M2-09), (c) Wasm multi (M2-10), and (d) the native image M2 emits (M2-15/16). All four produce byte-identical output on the full corpus. Stage-1 is **N/A** (frozen, single-function, the named legacy exemption) and is listed as an exclusion, as M2-06 does.
5. **Fixed point (M3-17):** `M3_vm(pure/m3/*) == RustFE(pure/m3/*) == committed pure/m3/m3.omni_bc`, with a drift check, as in M2-16.

Every commit also passes M2's global guard G (cargo test, selftest, pure-bootstrap, fixed-point, pure-omni-diff, 77/77, 4-backend goldens, frozen-hash lock).

## 8. Milestones (each one commit)

The port map's order is: ISA table/decoder → packet encoder → lexer → parser → lowering. M2 runs in parallel.
- Gates from M2: M2-03 (lint), M2-04 (decode), C1 + M2-05 (ld/st intrinsics), M2-06 (`RET` slice), M2-08 (module check), M2-09/10/15/16 (backends).

| ID | Kind | What | Needs |
|---|---|---|---|
| M3-00 | docs | `spec/m3/PLAN.md`, decisions S-1..S-4, open items O-1..O-9 | — |
| M3-01 | Rust | `fe-dump` canonical serializers + corpus manifest + Q1-Q8 and negative fixtures (Rust behaviour locked) | M2-01 |
| M3-02 | Rust | Point `m2-lint` (≤ 15 hard / ≤ 12 target labels, ≤ 192 / 120 SSA, no recursion) and the M2-08 depth check at `pure/m3/` | M2-03, M2-08 |
| M3-03 | Omni | `pure/m3/isa.omni` mnemonic/opcode rodata + drift test against `omni-isa` and assembler :29-97 | M2-04 |
| M3-04 | Omni | `pure/m3/packet.omni` encoder; diff against `encode_asm_ins_packet_u16` on every asm line the corpus produces | M3-03, C1, M2-05 |
| M3-05 | Omni | `pure/m3/arena.omni` (`m3_init`, §3 layout asserts, pool, error slot) | M2-05 |
| M3-06 | Omni | `lex.omni`, `lex_tok.omni` (pass L); token parity incl. strict and Q1/Q2 | M3-05 |
| M3-07 | Omni | `skim.omni` (pass S + signature checks, Q5/Q7) | M3-06 |
| M3-08 | Omni | `expr.omni`, `primary.omni` explicit-stack parser; AST parity (Q3/Q4) | M3-07 |
| M3-09 | Omni | `stmt.omni`, `macro.omni` (pass P complete); AST parity on the full corpus | M3-08 |
| M3-10 | Omni | `symtab.omni`, `lower_expr.omni`; packet parity for expressions | M3-04, M3-09 |
| M3-11 | Omni | `lower_call.omni` (intrinsic table) | M3-10 |
| M3-12 | Omni | `lower_stmt.omni` (labels, loops, ret, Q8) | M3-11 |
| M3-13 | Omni | `pack.omni`, `driver.omni` (pass B, `m3_main` last, output + sourcemap); full-stream parity on the VM | M3-12, M2-06 |
| M3-14 | Rust | `lockstep_shadow_frontend` (opt-in, then default) | M3-13 |
| M3-15 | Rust test | 4-backend parity of the M3 image (VM, x86 multi, Wasm, M2-native); Stage-1 exclusion listed | M2-09, M2-10, M2-16 |
| M3-16 | Rust | Embedded-literal → `include_str!` cutover with pre/post hash lock (§6) | M3-01 (any time after) |
| M3-17 | Rust + seed | Commit `pure/m3/m3.omni_bc` + drift check; `omnic m3-fixed-point` | M3-15 |
| M3-18 | docs | README/spec honesty: "frontend runs in Omni under lockstep; Rust frontend still present" | M3-17 |

Deleting the Rust frontend is **not** in M3. It needs M3-14 in default mode for a period and Caleb's decision.

## 9. Risks and open questions

**Top risks**

| # | Risk | Mitigation |
|---|---|---|
| R-1 | **Order-exact lowering.** Register/label numbering and packet order must match Rust's recursive evaluation order exactly. A 1-off difference changes every downstream hash. | Packet-level differential per construct (M3-10..12). Phase tables written from the Rust code paths, not from intent. Q-list fixtures. |
| R-2 | **VM cost / input size.** The VM clones on every register read (Nyx), and M3's own source probably exceeds M2's 64 KiB input window (O-3 in M2). | 128 KiB prefill, streaming lexer, per-fn pools. Possibly a VM perf fix with no semantic change (a Rust change, Vera's beat). Raise the input-window choice with M2's O-3. |
| R-3 | **Dependency stack.** M3-04+ waits on C1 (multibyte ISET/IGET; 4-op `ISET -2/-4` and 3-op `IGET -2/-4/-8` are not implemented at `74a215c`), M2-05/06, and M2-09/10/16 for the 4-backend gate. | Start with M3-00..M3-03 and M3-16 now. Keep the M3 encoder off `pure/m2` internals except read-only `decode`/`util`. |
| R-4 | Error-order parity (lex → parse → sig → lower) under streaming | The four-pass design (§3). Negative corpus per class. |
| R-5 | The cutover moves a hash through a hidden source dependency (Q1 substring, aliases) | §6 per-module proof. Leave the literal in place on any failure. |

### R-2 measurement: compiled size vs the M2 input window (O-3), measured 2026-11-10

These are real Rust-compiler outputs at `74a215c`. Release build of the workspace CLI:

```
cd /workspace/audit-scarlett-review/omni-workspace
CARGO_TARGET_DIR=/tmp/ws-review/target cargo build --release -p omni-cli
cd ..
for f in $(git ls-files 'std/*.omni' 'pure/*.omni' 'pure/**/*.omni'); do
  /tmp/ws-review/target/release/omnic compile-surface "$f" /tmp/m3meas/$(echo $f | tr / _).bc
  echo "$f $(wc -l <$f) $(wc -c <$f)"; done
```

`compile-surface` (`omni-cli/src/main.rs:1021-1025`) is `compile_omni_surface_file_to_u16`, which writes 2 bytes per word. Files that `include` others are measured **after include expansion**, marked (inc).

| Module | Lines | Bytes | u16 words | words/line | words/KB |
|---|---:|---:|---:|---:|---:|
| pure/verifier_engine.omni | 1,034 | 32,944 | 11,893 | 11.50 | 369.7 |
| pure/compiler.omni | 117 | 5,271 | 890 | 7.61 | 172.9 |
| pure/host_runtime.omni | 70 | 2,914 | 507 | 7.24 | 178.2 |
| pure/tooling.omni | 99 | 4,352 | 662 | 6.69 | 155.8 |
| pure/verifier.omni | 51 | 2,107 | 378 | 7.41 | 183.7 |
| pure/selfhost_suite.omni (inc) | 65 | 3,328 | 3,371 | 51.86 | 1037.2 |
| pure/programs/arena_audit_pipeline.omni (inc) | 54 | 1,765 | 613 | 11.35 | 355.6 |
| pure/programs/compiler_self_audit.omni (inc) | 44 | 2,180 | 3,272 | 74.36 | 1536.9 |
| pure/programs/kv_ledger_service.omni (inc) | 37 | 1,567 | 553 | 14.95 | 361.4 |
| pure/programs/pack5_decision_controller.omni (inc) | 41 | 2,200 | 1,039 | 25.34 | 483.6 |
| pure/programs/policy_consensus_engine.omni (inc) | 48 | 2,011 | 1,170 | 24.38 | 595.8 |
| pure/programs/ternary_neural_router.omni (inc) | 59 | 2,303 | 665 | 11.27 | 295.7 |
| std/decision.omni | 57 | 1,938 | 336 | 5.89 | 177.5 |
| std/http.omni | 33 | 1,135 | 198 | 6.00 | 178.6 |
| std/math.omni | 47 | 1,227 | 219 | 4.66 | 182.8 |
| std/pack5.omni | 43 | 1,459 | 240 | 5.58 | 168.4 |
| std/prelude.omni | 34 | 1,088 | 118 | 3.47 | 111.1 |
| std/resource.omni | 38 | 1,394 | 174 | 4.58 | 127.8 |
| std/trit.omni | 34 | 1,148 | 154 | 4.53 | 137.4 |

| Ratio | min | median | max |
|---|---:|---:|---:|
| words/line, all 19 (incl. expanded) | 3.47 | 7.41 | 74.36 |
| words/KB, all 19 | 111.1 | 182.8 | 1536.9 |
| words/line, 12 standalone (no includes) | 3.47 | 5.95 | 11.50 |
| words/KB, 12 standalone | 111.1 | 175.2 | 369.7 |

The include-expanded ratios measure the included code, not the line density, so they don't apply to M3's own lines. The best proxy for M3 is `verifier_engine.omni`, the only real algorithmic Omni program. It's also the standalone maximum: 11.50 words/line.

**Projected M3 image** (64 KiB window = 32,768 words; 128 KiB = 65,536 words):

| Assumed density | 3,000 lines | 4,000 lines | 5,000 lines | Fits 64 KiB? | Fits 128 KiB? |
|---|---|---|---|---|---|
| standalone median 5.95 w/line | 17,842 w / 34.8 KiB | 23,789 w / 46.5 KiB | 29,737 w / 58.1 KiB | yes (all) | yes |
| **verifier_engine 11.50 w/line** | **34,500 w / 67.4 KiB** | **46,000 w / 89.8 KiB** | **57,500 w / 112.3 KiB** | **no (all three)** | **yes (all three)** |

Break-even at 11.50 w/line: about 2,850 lines for 64 KiB and about 5,700 lines for 128 KiB.

**Conclusion for O-3:** at verifier-engine density, a 3-5k-line M3 does **not** fit in the 0x10000 window but **does** fit in 0x20000. The margin at 5k lines is about 12%. Above about 5,700 lines even 128 KiB overflows, so an input-size guard and a "split into compilation units" fallback are still needed.

**Open questions** (for Boss Lady / Caleb; none are decided here)

| # | Question |
|---|---|
| O-1 | **Label limit:** is the usable cap 15 (ids 1..15, Nyx's emitter measurement) or 16? M3 is designed for ≤ 15 either way. If 16 is confirmed, nothing in M3 changes. |
| O-2 | Should shared ISA/decoder code be hoisted to a common dir (for example `pure/common/`) instead of M3 including `pure/m2/` read-only? That would touch M2-16's drift-checked file set. |
| O-3 | Do linear-scan symbol tables with fnv32 compare keys satisfy "no hash maps"? Bucketed open addressing is avoided on purpose. |
| O-4 | Source input encoding: `Bytes` on the VM vs u16 words natively (NUL-padded to even length; M2's `mf_input_odd` traps on odd bytes) |
| O-5 | When to port include resolution (Q6) to Omni over a host-provided source bundle. Until then the host expands includes. |
| O-6 | Nesting limit (512 frames) and other caps (1024 fns, 128 macros, 32 KiB pool, 1024 AST nodes per fn): Rust has no such limits, so these are new documented errors. Is that acceptable? |
| O-7 | Error message text parity: compare `(class, line, col)` only, or also have Omni render Rust's exact strings? |
| O-8 | Whether `emit_asm` text output (`omnic asm`) is ever ported or stays as Rust tooling |
| O-9 | The monolith mirror: every M3 Rust commit (M3-01/02/14/16/17) must be mirrored in M with the A≡M diff, as in M2. Is retiring the monolith before M3-14 an option? |
