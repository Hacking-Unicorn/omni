# M3-01: canonical frontend dumps and the M3 frontend lock

`omnic fe-dump` (workspace) and `omni_rust_bootstrap fe-dump` (monolith) record what the Rust frontend produces today. The future Omni frontend in `pure/m3/` must reproduce it exactly (M3-06..M3-13).

```
fe-dump lock                                              # prints the rows of spec/m3/corpus.tsv
fe-dump --tokens  <file.omni> [out.bin] [--strict]
fe-dump --ast     <file.omni> [out.bin] [--strict]
fe-dump --packets <file.omni> [out.bin] [--strict]
fe-dump --stream  <file.omni> [out.bin] [--strict]        # with out.bin, also writes <out.bin>.sm
```

The mode words also work without dashes (`tokens`, `ast`, ...). Every dump prints `words=N:FNV-1a64` and, with `out.bin`, writes little-endian u16 words. Includes resolve exactly as an in-memory compile does: built-in modules first, then paths relative to the process cwd.

## Formats

All integers are u16 words. `u32` = `[lo, hi]`. `i64` = 4 words, little-endian. `str` = `[nbytes, byte, byte, ...]`, one word per byte. `span` = `line u32, col u32`.

- **Tokens** (`0x4D33`): `[magic, count u32]`, then per token `[kind, line u32, col u32, value i64, text str]`.
  - `kind`: 1 Ident, 2 Int, 3 Trit, 4 StrLit, 5 AtIdent, 6 Sym, 7 Op, 8 Hash.
  - `value`: the Int value or the Trit value, else 0. `text` is the token text as the Rust lexer stores it (Q2: `+1t` stores `"+1"`, `-1t` stores `"-1t"`).
- **AST** (`0x4D41`): `parse_module` output after macro substitution and include expansion, in preorder. `[magic, nfns u32]`, then per fn:
  `[0xF0, span, name str, has_fn_id + fn_id, cap_mask, has_step + i64, has_mem + i64, has_budget_span + span, strict_attr, is_route, has_method + str, has_path + str, nparams, (name str, ty str, untrusted, span)*, ret_ty str, ret span, body]`.
  - `body` / any stmt list = `[n u32, stmt*]`.
  - Stmts: `0x10 Let(span, name, has_ty [+ ty str + span], expr)`, `0x11 Assign(span, name, expr)`, `0x12 DoWhile(span, body, cond)`, `0x13 While(span, cond, body)`, `0x14 Label(span, id)`, `0x15 Goto(span, id)`, `0x16 Ret(span, expr)`, `0x17 Expr(span, expr)`.
  - Exprs: `0x20 Int(span, i64)`, `0x21 Trit(span, text, value)`, `0x22 Str(span, str)`, `0x23 TritVec(span, n u32, trit*)`, `0x24 Var(span, name)`, `0x25 UnOp(span, op, a)`, `0x26 BinOp(span, op, a, b)`, `0x27 Call(span, name, n u32, args)`, `0x28 If(span, kind str, cond, then, else)`, `0x29 Match(span, scrut, neg, zero, pos)`.
  - Trit values are stored as i16 in a u16 word (`-1` = `0xFFFF`).
- **Packets** (`0x4D50`): `[magic, nfns u32]`, then per fn `[fid, nparams, (reg, ty, tc)*, ret_ty, cap_mask, npkts u32]`, then per packet `[span, len, words...]`. These are exactly the `SurfaceLowerCtx` packets and spans. The dumper re-packs them and fails with `FE_DUMP_DIVERGENCE` unless the result equals the compiler's stream word for word.
- **Stream**: the exact `compile_omni_surface_to_u16_with_sourcemap` output.
- **Sourcemap** (`0x4D53`): `[magic, n u32]`, then per entry `[off u32, line u32, col u32]`, then budget span, default span, and a pack5 flag (+ span).
- **Errors**: recorded as `CLASS:line:col`. Message text is not locked.

## The lock: `spec/m3/corpus.tsv`

One row per corpus entry: `name, strict, tok=words:fnv:tokv=words:fnv, ast=words:fnv, pk=words:fnv, out=words:fnv:sm=words:fnv`. Any column can be `ERR:CLASS:line:col`. `tokv` is the token dump with every line/col zeroed, so it compares kinds, values and texts only. The monolith selftest (13M3a), the workspace monolith-vs-workspace check, and `cargo test` (`m3_fe_lock`) all assert it row for row. They also assert that each negative fixture below fails with its class.

Corpus (84 rows):
- the 18 closed-suite modules from `lookup_omni_stdlib_module`, compiled strict as the suite does;
- `pure/verifier_engine.omni`;
- the 39 acceptance goldens under `spec/acceptance/goldens/` (excluding `neg_circular_include.omni`, see below);
- 15 quirk fixtures in `spec/m3/fixtures/q*.omni` (Q1-Q8, `spec/m3/PLAN.md` §1);
- 11 negative fixtures in `spec/m3/fixtures/neg_*.omni`, one per frontend error class.

| Class | Fixture | Locked result |
|---|---|---|
| SYNTAX_ERROR | `neg_syntax_error` | `3:16` |
| ARITY_MISMATCH | `neg_arity_mismatch` (user fn) | `6:13` |
| UNRESOLVED_LABEL | `neg_unresolved_label` | `3:10` |
| BAD_TYPE | `neg_bad_type` | `2:12` |
| AMBIGUOUS_GRAMMAR | `neg_ambiguous_grammar` (`#[strict]` + `int`) | `3:12` |
| UNDEF_VAR | `neg_undef_var` | `4:12` |
| BUDGET_EXCEEDED | `neg_budget_exceeded` (`step_budget(0)`) | `2:15` |
| UNKNOWN_FN | `neg_unknown_fn` | `3:13` |
| MISSING_CAP | `neg_missing_cap` (`#[cap(TELEPORT)]`) | `2:7` |
| IMMUT_PARAM | `neg_immut_param` | `3:5` |
| CIRCULAR_INCLUDE | `neg_circular_include` (includes itself) | `2:1` |

`neg_circular_include` names its own repo-root-relative path, so a normal compile only finds the cycle when run from the repo root. The lock therefore resolves it through the include resolver's in-memory VFS (an `inc=` row), which gives the same answer without depending on the cwd. The acceptance golden of the same name is excluded for the same reason.

## Observations the lock records (Rust behaviour, not endorsements)

- **Label cap (Q8, C4).** The cap is **16** labels per function. The emitter and verifier already accept ids 0..15 (Vera, `vera_harness.md` §3). The source compiler numbers labels from 1 and rejects the 16th (`omni_rust_bootstrap.rs:3587`, `:3646-3656`; workspace `compiler/mod.rs:1585`, `:1644-1654`), so frontend-compiled functions get **15** until C4 lands:
  - `q8c_label_ok` (14 labels) and `q8d_label_15` (15) compile;
  - `q8b_label_limit` (16) fails `BUDGET_EXCEEDED:19:14`. **C4 must flip this row** to a compiled stream, regenerated in the C4 commit;
  - `q8e_label_17` (17) fails `BUDGET_EXCEEDED:20:14` and must still fail after C4.
- `q8a_ssa_limit` (> 192 SSA) fails `BUDGET_EXCEEDED:195:21`.
- `q6_include_twice` fails `SYNTAX_ERROR` (`58:43` since M3-16; `6:53` before): the textual include duplicates function definitions (no de-duplication). The coordinate is inside the second pasted copy of `std/math.omni`, so it moved when the embedded literal was replaced by the on-disk file.
- `q5_macro_order`: a macro used before its definition stays a call, and fails `UNKNOWN_FN:3:12`.
- `q1_strict_substring` is rejected (`AMBIGUOUS_GRAMMAR:3:13`) only because a comment contains the strict marker.

If an intentional frontend change alters a row, regenerate with `fe-dump lock > spec/m3/corpus.tsv` in the same commit and justify the change in the commit message.
