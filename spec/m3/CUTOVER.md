# M3-16: embedded stdlib literals → `include_str!` of the on-disk sources

`lookup_omni_stdlib_module` (workspace `omni-lang/src/compiler/mod.rs`, monolith mirror in `omni_rust_bootstrap.rs`) used to return Rust string literals. The literals were one-line or compacted copies of the `.omni` files, without their comments. They now return `include_str!` of the files on disk, so the Rust frontend, the suite builder and M3's corpus read the same bytes. The two `spec/pure_self_host/*` alias keys still resolve to the `pure/` files, not the drifted `spec/pure_self_host/*.omni` copies. The `PHASE_A..D` consts in `pure_omni/mod.rs` are out of scope and unchanged.

## Method: one module per step

For each of the 18 modules, in this order, before accepting it:
1. **Pre-proof (old binary, before swapping):** strict compile of the disk file vs the literal. The span-free token dump (`tokv`) and the u16 stream must be identical. Also, the disk file must contain the raw `#[strict]` marker exactly when the literal does (Q1: the marker flips strictness for every includer).
2. **Swap** that one arm in both mirrors and rebuild the monolith.
3. **Post-check:** recompute every row of `spec/m3/corpus.tsv` (84 rows: suite modules, verifier engine, acceptance goldens, Q1-Q8 and negative fixtures). For every row, `tokv` and the u16 stream must be unchanged. Only span-bearing columns may move: `tok` (with spans), `ast`, `pk` spans and `sm`. The 14,559-word suite must stay `0x78462AE4C81DF408` (`pure-bootstrap`).
4. A module that failed any check would be reverted and left embedded.

Step log: `/workspace/audits/scarlett_m3-16_steps.log` (on the shared box; summarised below).

## Result: all 18 modules cut over

| Module | Pre-proof (`tokv`, stream, marker) | Rows whose span columns moved |
|---|---|---|
| `std/prelude.omni` | identical, identical, both present | its lookup row; `golden/stdlib_macro_pipeline_golden`, `m3fix/q6_include_once` (includers) |
| `std/math.omni` | identical, identical, both present | its lookup row + 6 includers (`selfhost_suite` + 5 `pure/programs/*`); `m3fix/q6_include_twice`: **error coordinate** `SYNTAX_ERROR 6:53 → 58:43`, see below |
| `std/trit.omni` | identical, identical, both present | its lookup row + 6 includers (`selfhost_suite` + 5 `pure/programs/*`) |
| `std/http.omni` | identical, identical, both present | its lookup row + 3 includers |
| `std/pack5.omni`, `std/decision.omni`, `std/resource.omni` | identical, identical, both present | lookup row + `pack5_decision_controller` |
| `pure/compiler.omni` (+ alias) | identical, identical, both present | lookup row + 2 includers |
| `pure/verifier.omni` | identical, identical, both present | lookup row + 3 includers |
| `pure/host_runtime.omni` (+ alias), `pure/tooling.omni` | identical, identical, both present | lookup row + 2 includers each |
| `pure/selfhost_suite.omni` and the 6 `pure/programs/*` | identical, identical, both present | own lookup row only |

**`std/math.omni`, the one judgement call.** The automated step flagged it, because `q6_include_twice` (which deliberately includes `std/math.omni` twice) still fails with the same class, `SYNTAX_ERROR`, but at a different coordinate. The duplicate-definition error points into the second pasted copy of the module. That copy is now the commented disk file, so the line number is the file's real line. Every token stream and every u16 stream is identical. This is the "spans inside stdlib includes change" case that `PLAN.md` §6 step 4 allows, so it was cut over by hand after review and the lock row was regenerated. If reviewers prefer, reverting just that arm restores `6:53`.

No `.omni` file was edited. No module had to stay embedded.

## What changed for users

- Streams, bytecode and all five frozen hashes are unchanged.
- Sourcemap line/col for code that comes from a stdlib include, and error coordinates that point inside included stdlib text, now match the real file lines.
- The binaries now need `std/` and `pure/` present at build time (both are tracked in the repo).
