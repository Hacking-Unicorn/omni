# Omni

Omni is a 16-bit token-stream ISA (128 opcodes, `0x7F00..=0x7F7F`) with a capability / linear-ownership / taint static verifier, a deterministic VM, `@PACK5` balanced-ternary packing, a `.omni` surface language, and native `x86-64` ELF64 and 64-bit WebAssembly backends for a documented opcode subset.

**Status, in one line:** Omni is **self-hosted end-to-end in Pure-Omni across the x86-64 ELF64 backend (M2), the surface `.omni` → `.omni_bc` frontend (M3), the deterministic token-stream VM (M4), and the zero-shim Linux x86-64 syscall emitter (M5)**: `13,134` lines across `pure/**/*.omni` (`pure/m3/`, `pure/m2/` incl. `pure/m2/verify/`, `pure/m4/`, `pure/m5/`, `pure/m7/`, `pure/api/`, `pure/isa/`, `pure/verifier_engine.omni`, and `pure/host_kv.omni`) compile (a) the expanded M3 frontend source bit-for-bit to `pure/m3/m3.omni_bc` (`59,540` words / `119,080` B, `0xC96457590C836700`, `0`-word diff vs Rust), (b) the `41,220`-word M2 backend seed `pure/m2/m2.omni_bc` (`0xB3CA3BEB90A1C98E`, with integrated `pure/m2/verify/mod_check.omni` module verifier) bit-for-bit to `119,371`-B ELF64 (`0x49FF6E9263B9983D`, `S1 == S2 == S3 == V`, `0`-byte diff vs Rust), (c) the `27,819`-word Pure-Omni M4 VM seed `pure/m4/m4.omni_bc` (`0x99567F6F5D0E8F32`) via Pure-Omni M3 (`0`-word diff) and Pure-Omni M2 → `89,772`-B ELF64 (`0xD4B4CE81CFFF4636`, `0`-byte diff vs Rust; `p5-selfhost`), and (d) the `1,300`-line `pure/verifier_engine.omni` end-to-end through Pure-Omni M3 → Pure-Omni M2 → `38,080`-B ELF64 (`0x39E62472D8BA5F74`, `0`-byte diff vs Rust; standalone native `verifier.elf` `37,953` B, `0x9C69BF7364DBB04E`), with multi-function Wasm AOT + `CALL` + depth-64 evaluator (`M2-10`), Pure-Omni multi-AOT module verifier (`M2-14`), opt-in `lockstep_shadow_frontend` (`M3-14`), 5-backend parity (`M3-15` + Pure-M4 column in `backend-diff --suite`), raw `.omni_bc` 3-op thin-shim execution + Wasm-multi VM host (`P4-1..P4-5`), and a standalone `233`-byte zero-shim Linux x86-64 ELF64 (`P5-2`). **Pure-Omni M3 → M2/M4/verifier is the default CLI path** (`compile`, `compile-elf`, `run`, `self-host`) using hash-pinned native `m3.elf`/`m2.elf`/`m4.elf`/`verifier.elf` binaries on Linux x86-64 (`--interpreted` / `OMNI_FORCE_INTERPRETED=1` selects the VM), with Pure-Omni O-5 include expansion (`pure/m3/include.omni`), macros quarantined behind `--rust`/`--oracle`, and the `pure-default-no-rustc` CI job verifying the pure-default path with `rustc`/`cargo` absent from `PATH` (classified Stage-0 seed: `236` lines [`seed0` 134 + `p4_shim` 102]; runtime host Rust: `35,564` lines [`omni_rust_bootstrap.rs` 26,068 + `m2_host/` 8,399 + `host_runtime/pure_default.rs` 995 + `p4_shim/mod.rs` 102]; active Rust total: `35,738` lines with the duplicate 8-crate workspace archived in `oracle/archived-workspace/`); `@PACK5` ops are excluded from M2.

## Design constraints

These limits are intentional invariants (`HANDOFF.md` §2):

- **Zero external crates.** `Cargo.lock` lists only the eight workspace packages.
- **192 SSA registers and 16 labels (`0..=15`) per function** (enforced by both surface compiler and verifier); **64-deep call stack** (enforced by the VM and `m2-lint`).
- **Eager argument evaluation.** No heap structs, hash maps, or GC in core `.omni`; data lives in flat linear byte arenas (compile-time offset constants and arena-backed sorted arrays with binary search are used).
- **No recursion in core `.omni` modules** (iterative state machines + explicit worklists, enforced by `m2-lint` and `m3-gate`).
- **The 128-opcode table (`0x7F00..=0x7F7F`) is never renumbered.**
- **5-backend differential parity** (`vm`, `wasm`, `x86`, `stage1`, `m4`) gated by `backend-diff --suite` (`242` graded: `204` pass, `38` xfail, `0` xpass, `0` fail), where any unexpected `FAIL` or `XPASS` fails the suite.

## What runs where

### Real iterative `.omni` core modules (executed on the VM & differentially gated)

| Module | Lines | What it actually does |
|---|---:|---|
| `pure/verifier_engine.omni` | 1,300 | Full static token-stream verifier (capabilities, taint, linear/borrow state, SSA order, labels, opcodes, per-opcode minimum arity `2012`, `EMPTY_MODULE` `3007`, `CALL` target existence `2010` & arity `2011`). Active on the default Pure-Omni hot path via hash-pinned native `verifier.elf` (`37,953` B, `0x9C69BF7364DBB04E`; `b2-pure-verifier`, 14 checks). |
| `pure/m2/decode.omni` + `pure/m2/util.omni` | 650 | Shared u16 token-stream decoder (`m2d_decode_at`, `m2d_decode_at_ws`) reading opcode metadata from `pure/isa/table.omni`. Differentially tested against Rust's `decode_omni_token_stream`. |
| `pure/isa/table.omni` | 505 | Shared ISA rodata table (128 opcode rows + 81 frozen mnemonic ids `0..=80`), using strict-bounds `ld_u8`/`ld_u16`. Drift-locked against Rust (`13BA`). |
| `pure/host_kv.omni` | 389 | Arena-backed sorted-array KV store (`0x1B0000..0x1EFFFF`, iterative binary search, 8192-key cap, miss = `-1`). Runs in lockstep against the Rust VM KV arms (`13AZ`). |
| `pure/isa/encode.omni` | 296 | Shared u16 packet & module encoder (`M3-04`). Verified bit-for-bit (`0` byte diff) on round-trip over every positive acceptance golden, `pure/verifier_engine.omni` (11,893 words), and `stage1_compiler_stream` (19,731 words). |
| `pure/m2/labels.omni` | 116 | Arena-backed `u32` label table & resolver (`0x20000..0x23FFF`, `code_off + 1` / `0 = undef`, first-definition-wins, LBL `0x7EF0..0x7EFF` disjoint from IMM `0..4079`, IMM ≥ 4080 rejected). |
| `pure/m2/arena.omni` + `pure/m2/elf.omni` | 301 | M2 D-8 arena state/output band (`0x24000..0x7FEFF`), D-7 >128 KiB hard rejection, `m2a_finish_slice` (`ret_slice`), byte-exact 120-B ELF64 header, trap stubs, and x86-64 frame/multi prologues (`m2-elf`). |
| `pure/m2/emit_x86.omni` + `pure/m2/driver.omni` | 748 | Two-pass single-function u16 → x86-64 ELF64 compiler (`M2-12`/`M2-13` + provable `ANEW`/`ISET` buffer 2-op `IGET` readback). Compiles the full **20,443-word Stage-1 compiler stream** to the exact `112,180`-byte ELF64 (`0x62F14B335B52BECC`, `0`-byte diff vs Rust; `m2-compile`). |
| `pure/m2/verify/mod_check.omni` | 387 | Iterative multi-AOT module verifier (`M2-14` + Audit §2.1): function header bounds, unique `fn_idx`, per-opcode minimum operand arity (drift-locked against `pure/verifier_engine.omni`), `CALL` target/arity checks, trailing `RET`, iterative Kahn topological-sort cycle detection (no recursion), and L1–L12 label-key parity via `pure/m2/labels.omni` (`m2-verify`, 10/10 differential checks vs Rust; linked directly into `m2.omni_bc` / `m2.elf`). |
| `pure/m2/emit_multi.omni` + `pure/m2/m2_standalone.omni` | 1,245 | Two-pass multi-function u16 → x86-64 ELF64 compiler (`M2-15`/`M2-16` + Phase 7 compact encodings + `PT_GNU_STACK` + provable buffer 2-op `IGET` readback + Audit §1.2/§2.1 `T_OOB` trap fallback and pre-lowering `m2_verify_module_into` gate). Compiles `pure/compiler.omni`, `pure/verifier_engine.omni`, and its own `41,220`-word seed `pure/m2/m2.omni_bc` (`0xB3CA3BEB90A1C98E`) → `119,371`-B ELF64 (`0x49FF6E9263B9983D`, `S1 == S2 == S3 == V`) with `0`-byte diff vs Rust (`m2-multi-pure`, `m2-fixed-point`). |
| `pure/m3/*.omni` (`arena`, `lex`, `skim`, `expr`, `stmt`, `lower`) | 3,631 | Iterative surface `.omni` → `.omni_bc` frontend (`M3-05..M3-17` + `st_u64` lowering: lexer, Pass S fn/macro/const skimmer, Pratt expression parser, iterative statement parser, full SSA/CFG/opcode lowerer with `if_pos`/`if_neg`, `match_trit`, `StrLit`, loops, labels/gotos, and O(1) scopes). Self-compiles its own expanded source to `pure/m3/m3.omni_bc` (`59,540` words, `0xC96457590C836700`, `0`-word diff vs Rust) and chains with Pure-Omni M2 on `pure/verifier_engine.omni` → `38,080`-B ELF64 (`0x39E62472D8BA5F74`, `0`-byte diff vs Rust; `m3-fixed-point`, `m3-lower`, `m3-gate`). |
| `pure/m4/*.omni` (`arena`, `ops_core`, `vm`) | 1,069 | Phase 4/7 AI-native Pure-Omni deterministic token-stream VM (`P4-1..P4-5` + Phase 7 hot-path tuning + Audit §1.1/§2.2/§5 exact `INT_MIN` divmod, strict `ISET -9` OOB, 5th `backend-diff` column, and fuel-charged decode/label-index loops): executes raw `.omni_bc` directly in a 512 KiB flat arena (64-deep call stack, 224 slots/frame, configurable `--fuel N`, ISA-B byte buffers, built-in 64 KiB binary trace ring `0x70000..0x7FFFF`), backed by a minimal 3-op thin shim (`p4_shim/mod.rs`: `read_bytes`/`write_bytes`/`spawn`) and running on M2-10 Wasm-multi (`p4-vm`, `p4-exec`, `p4-shim`, `p4-wasm`). Deterministic seed `pure/m4/m4.omni_bc` (`27,819` words, `0x99567F6F5D0E8F32`) compiles via Pure-Omni M3 (`0`-word diff) and lowers via Pure-Omni M2 to `89,772`-B ELF64 (`0xD4B4CE81CFFF4636`, `0`-byte diff; `p5-selfhost`). |
| `pure/m5/elf_sys.omni` + `pure/m5/launcher.omni` | 132 | Phase 5 zero-shim Linux x86-64 syscall emitter (`P5-2`, 73 lines -> `233`-B ELF64 `p5-sys` with `PT_GNU_STACK`) + Phase B4 `.omni_bc` `0x7EE0` header validator/entry stub (`pure/m5/launcher.omni`, 59 lines -> `237`-B ELF64 `0x45EE82AAE5008841`, which validates `TOK_MOD_BEGIN` on stdin and exits 0/1 while `b4-pure-launcher` orchestrates the native `m2.elf -> m4.elf` compilation). |
| `pure/m7/judge.omni` | 292 | Phase 7 (`P7-0`/`P7-0a`) balanced-ternary measurement decision layer: noise-band thresholding without division (`p7_cmp_band`), 3/5-run ternary consensus (`tmaj`, `p7_judge_5runs`), subsystem attribution (`p7_attribute_subsystem`), written adopt/reject & no-regression rules, `@PACK5` radix-3 verdict packing/unpacking, and Pure-Omni P4-5 trace-ring verification & 4-class opcode histograms (`p7-measure`, `p7-canary`). |
| `pure/m7/heal.omni` | 118 | Phase 7 (`P7-1`) Pure-Omni self-healing engine: balanced-ternary failure taxonomy (`p7_classify_failure`: `+1t` healable / `0t` OK / `-1t` fatal fail-closed), bounded N-attempt repair state machine (`p7_heal_decide`), content-addressed FNV-1a64 rollback integrity verification, and 16-B binary healing log writer/verifier (`p7_heal_log_push`/`p7_heal_log_verify`; `p7-heal`). |
| `pure/api/ternary.omni` + `pure/api/api_main.omni` | 136 | New-Level API Stage 1: pure ternary capability surface (`api_route_trit`, `api_consensus`, `api_expert_pack5`, `api_cap_narrow`, plus thin wrappers `api_vote5`/`api_pack5_verdict` over `pure/m7/judge.omni`), library-first with zero `pure/m3/` changes; self-compiles via Pure-Omni M3 (`3,131` w, `0`-word diff) and lowers via Pure-Omni M2 (`7,590` B ELF64, `0`-byte diff), backed by Pure-Omni verifier rules `1010..1013` in `pure/verifier_engine.omni` (`api-stage1`). |
| `std/*.omni` (7) + `pure/programs/*.omni` (6) | 518 | Standard library helpers and small production-style programs, served directly from disk via `include_str!` (`M3-16`). |
| `pure/compiler.omni`, `pure/verifier.omni`, `pure/host_runtime.omni`, `pure/tooling.omni`, `pure/selfhost_suite.omni` | 402 | Legacy Phase 7/A–D integer decision kernels (frozen for regression hashes; not the M2/M3 compilers). |

### Runs in the Stage-0 Rust host

| Stage | Notes |
|---|---|
| Surface lexer, parser, includes, macros, lowering (`omni-lang`) | Production `.omni` → `u16` compiler (including `#[strict]`, 16 labels, ISA-B `ld_u8/16/32/64` & `st_u16/32`, and `fe-dump` canonical lock `spec/m3/corpus.tsv`). |
| Static verifier (`omni-verifier`) | Primary verifier, shadowed in lockstep by `pure/verifier_engine.omni`. |
| VM (`OmniBinaryVm`) | Fuel & memory budgets, 64-deep calls, strict-bounds ISA-B loads/stores (`error[OUT_OF_BOUNDS]`), KV store (`DB_QRY`/`DB_GET` miss = `-1`, 8192-key cap), and in-VM host simulation. |
| `x86-64` ELF64 & 64-bit Wasm emitters (`omni-aot`) | Single-function AOT backends for the documented pure numeric/ternary/`@PACK5`/byte-buffer subset (including ISA-B `ld_u8/16/32/64`, `st_u16/32`, and per-buffer Wasm byte storage). Host opcodes (`0x7F64..=0x7F7F`) are rejected by AOT. |
| Stage-1 self-compiler (`omni_self_compiler.omni_bc`) | 19,731-word `u16` compiler stream generated by Rust (`build_step7_stage1_compiler_stream`) that emits its own bit-exact 106,242-byte `x86-64` ELF64 (`stage1 == stage2`). Frozen until the Milestone 2 (`stage2 == stage3`) re-freeze; rejects unsupported opcodes (`DROP`, `IMOD`, ISA-B, `pack5`/AI) up front with `error[UNSUPPORTED]`. |
| Live HTTP/KV host (`omni serve`), harness, agent driver, CLI | Rust host over the capability-gated 10-call `OmniNarrowHost` boundary (path-sandboxed with `error[PATH_ESCAPE]`). |

### Re-frozen regression hashes (pinned in code by `13BF` / `M2-01`, single §5 re-freeze)

All 5 frozen artifacts are asserted by FNV-1a64 hash and exact size in `selftest`. They were re-frozen once deliberately in Audit §5 (`PT_GNU_STACK` non-executable stack header on all generated ELFs, strict `OUT_OF_BOUNDS` on 4-op `ISET -9`, and x86/Wasm/Stage-1 fixes for `IMOD`, trit clamping, and `BR_POS`/`BR_NEG`; legacy 2-op `IGET` is explicitly lenient and returns `0` on OOB across all backends):

| Artifact | Size | FNV-1a64 | What it is |
|---|---:|---|---|
| Phase D kernel `.omni_bc` | 159 words | `0x23B8D17257CCA9C4` | 3-input decision kernel (unchanged) |
| Phase D kernel `.elf` | 880 B | `0x8D5AAAD22399A1EE` | Phase D kernel lowered via host x86-64 and Stage-1 (+56 B `PT_GNU_STACK` + `BR_POS`/`BR_NEG`) |
| 18-module core suite | 14,559 words | `0x78462AE4C81DF408` | 7 `std/*` + 5 legacy `pure/*` kernels + 6 `pure/programs/*` (unchanged) |
| Stage-1 bytecode (`omni_self_compiler.omni_bc`) | 20,443 words (40,886 B) | `0xED4C50AA79563EFF` | Rust-generated Stage-1 self-compiler token stream (`PT_GNU_STACK`, `IMOD`, trit clamp, `BR_POS`/`BR_NEG`) |
| Stage-1 ELF (`stage1.elf == stage2.elf`) | 112,180 B | `0x62F14B335B52BECC` | Bit-exact Stage-1 == Stage-2 fixed-point ELF64 (`diff = 0` bytes) |

## Repository layout

- `omni_rust_bootstrap.rs` — Standalone Stage-0 monolith (`rustc -O omni_rust_bootstrap.rs -o omnic`, zero warnings).
- `host_runtime/pure_default.rs` — Production pure-default host runner (hash-pinned native `m2.elf`/`m3.elf`/`m4.elf`/`verifier.elf`, hardened `0700` cache, `--fuel N`, `--interpreted` fallback).
- `oracle/archived-workspace/` — Archived duplicate 8-crate Cargo workspace (`omni-isa`, `omni-ir`, `omni-verifier`, `omni-vm`, `omni-assembler`, `omni-lang`, `omni-aot`, `omni-cli`; moved from `omni-workspace/` in Audit §6.1).
- `pure/` — Pure-Omni core modules (`pure/verifier_engine.omni`, `pure/isa/`, `pure/m2/`, `pure/m3/`, `pure/m4/`, `pure/m5/`, `pure/host_kv.omni`, `pure/programs/`, and frozen legacy kernels).
- `std/` — Pure-Omni standard library (`std/prelude.omni`, `std/math.omni`, `std/trit.omni`, `std/http.omni`, `std/pack5.omni`, `std/decision.omni`, `std/resource.omni`).
- `m2_host/` — Host-side Milestone 2–5 tooling (`lint.rs` for `m2-lint`, `decode_diff.rs` for M2/M3/P4/P5 differential gates).
- `p4_shim/` — Phase 4 minimal 3-op thin host shim (`p4_shim/mod.rs`: `read_bytes`, `write_bytes`, `spawn`).
- `spec/` — Executable specs & locks: `spec/acceptance/` (41 goldens + 1 documented VFS circular-include fixture), `spec/m2/` (M2 plan, decisions, single-mode byte lock, shape census, label reference, lint fixtures, verifier harness), `spec/m3/` (M3 plan, decisions, `corpus.tsv` frontend lock, Q1–Q8 & negative fixtures), `spec/m4/` (Phase 4 plan & VM/Wasm harnesses), `spec/m5/` (Phase 5 plan & zero-shim syscall harness), `spec/m6/` (Phase 6 consolidation, verification record & merge-ready checklist), `spec/host_runtime/`.
- `docs/archive/` — Historical Nov 6–7 audit/progress snapshots (for current living status, see `HANDOFF.md` on the `docs/handoff` branch, `spec/m2/`, and `spec/m3/`).
- `spec.md` — ISA and language specification (`OMNI_ISA_TABLE` is authoritative where counts differ).
- `LICENSE` — MIT.

## Quick start

### Monolith (`rustc -O`)

```bash
rustc -O omni_rust_bootstrap.rs -o omnic
./omnic selftest              # 1,690 checks on Linux x86-64 (13AY diff5, 13AZ KV, 13BA ISA table, 13BF hash pin, M2/M3/P4/P5/P7/API/A1-B4/S1-S6 locks)
./omnic a1-pure-default       # Seed-Shrink A1: Pure-Omni M3->M2/M4 default CLI path (`compile`, `compile-elf`, `run`, `self-host`; `--rust` oracle)
./omnic a2-pure-include       # Seed-Shrink A2: Pure-Omni O-5 include expander (`pure/m3/include.omni`, 0-byte diff vs Rust, `CIRCULAR_INCLUDE` gate)
./omnic a3-macro-quarantine   # Seed-Shrink A3: quarantine `macro` behind `--rust` (`ORACLE_ONLY_MACRO`) + pure replacement golden (0-diff M3/M2/M4)
./omnic b2-pure-verifier      # Seed-Shrink B2: Pure-Omni `verifier_engine.omni` on default hot path via native `verifier.elf` (14 checks incl. N1 arity/EMPTY_MODULE)
./omnic b4-pure-launcher      # Seed-Shrink B4: Pure-Omni M5 237-B zero-shim .omni_bc validator/entry stub ELF64 (`0x45EE82AAE5008841`) & native M2->M4 chain gate
./omnic s2-security           # Audit §2/N4/L3/L4 Security: M2 standalone verifier + 412B fuel/wall-clock cap + p4-exec verify + hardened native cache + Wasm arity + serve --bind
./omnic s3-perf               # Audit §3/L6 Performance: hash-pinned native m2.elf/m3.elf/m4.elf/verifier.elf default path + latency bounds
./omnic s4-honesty            # Audit §4/L7 Honesty: compile-elf 0/0/0 default args + live wc -l doc drift lock + frozen-5 pin lock (`--print-pins`, `--strict-readme`)
./omnic s5-refreeze           # Audit §5 Re-Freeze: strict ISET -9 OOB + PT_GNU_STACK + 9 burned-down xfails + 5 re-frozen hashes
./omnic s6-cleanup            # Audit §6 Cleanup: host_runtime/pure_default.rs extraction + archived workspace + CI fuzz-smoke + m2-lint dedup
./omnic p7-measure            # Phase 7 (P7-0): 5-benchmark corpus + P4-5 trace ring + pure/m7/judge.omni ternary (+1t/0t/-1t) + @PACK5 verdict gate
./omnic p7-canary             # Phase 7 (P7-0a): 5-run consensus + subsystem attribution (M3/M2/M4) + M2 emitter canary (+1t) & neutral (0t) gate
./omnic p7-heal               # Phase 7 (P7-1): self-healing taxonomy, FNV-1a64 rollback, bounded N-attempt repair & fault-injection gate
./omnic p7-adapt              # Phase 7 (P7-2): live trace workload classifier, cap-bounded policy, hysteresis & closed-loop self-adaptive gate
./omnic p7-learn              # Phase 7 (P7-3): tamper-checked ternary experience table, safety floor, @PACK5 export & self-learning gate
./omnic api-stage1            # New-Level API Stage 1: pure/api/ternary.omni + cap_narrow + Pure-Omni verifier 1010..1013 + 7 goldens (0-diff M3/M2/M4)
./omnic api-stage2            # New-Level API Stage 2 (Part 1): pure/api/reflect.omni (token_inspect/specialize/inspect_trace) + LINEAR_REFLECT 1012 + 3 goldens
./omnic api-stage2-emit       # New-Level API Stage 2 (Part 2): pure/api/emit.omni (token_emit_const/token_emit/re-hosted M2 token_emit_elf) + golden
./omnic api-stage2-5          # New-Level API Stage 2.5: pure/api/adaptive.omni (judge_better, specialize_with_hint, adaptive_step) + golden
./omnic p5-selfhost           # Phase 5 full zero-Rust chain: M3->pure/m4/m4.omni_bc (27,819w)->M2 ELF64 (89,772 B) + M3->M4/M2 loop
./omnic p5-sys                # Phase 5 zero-shim Linux x86-64 ELF64 (233 B, raw 0F 05 sys_read/sys_write/sys_exit; native exec on Linux)
./omnic p4-vm                 # Phase 4 Pure-Omni token-stream VM == Rust OmniBinaryVm (11/11 checks incl. P4-5 trace ring)
./omnic p4-exec <f.omni_bc>   # Phase 4 raw .omni_bc execution via 3-op OmniThinShim (zero source/parser at runtime)
./omnic p4-shim               # Phase 4 minimal 3-op thin host shim (read_bytes, write_bytes, spawn — zero HTTP/shell)
./omnic p4-wasm               # Phase 4 full Pure-Omni VM compiles to Wasm v1 + VM core executes on M2-10 Wasm-multi host
./omnic m3-fixed-point        # Pure-Omni M3 frontend fixed point (59,540 words, 0-word diff) + E2E M3+M2 ELF64 chain (38,080 B)
./omnic m3-4backend           # 5-backend parity (VM == Wasm-multi == Rust-x86-multi == Pure-Omni-M2-ELF64 == Pure-M4; Stage-1 N/A)
./omnic m3-lockstep           # Opt-in lockstep_shadow_frontend (OMNI_M3_LOCKSTEP=1) parity + mismatch hard-fail checks
./omnic m3-lower              # Pure-Omni M3 lowering parity vs Rust (21/21 modules incl. verifier_engine & st_u64, 0-word diff)
./omnic m3-gate               # lints + compiles + verifies all 6 pure/m3/*.omni modules
./omnic m2-fixed-point        # M2 self-hosted compiler drift check + S1 == V (and S1 == S2 == S3 on Linux x86-64: 119,371 B)
./omnic m2-verify             # Pure-Omni multi-AOT module verifier + Kahn cycle check + L1-L12 labels (10/10 vs Rust)
./omnic m2-wasm-multi         # Multi-function Wasm v1 emitter + CALL + shared arena global + depth-64 evaluator (7/7)
./omnic m2-compile            # Pure-Omni 2-pass compiler on 20,443-word Stage-1 anchor (112,180 B, 0-byte diff)
./omnic m2-multi-pure         # Pure-Omni multi-function compiler (7 checks, 0-byte diff vs Rust on verifier_engine, self & 2-op IGET)
./omnic m2-multi              # Rust M2-09 multi-function x86-64 ELF64 reference checks
./omnic m2-elf                # Pure-Omni arena & ELF64 header/stub checks
./omnic m2-lint               # lints all 27 registered pure/m2, pure/m3, pure/m4, pure/m5, pure/m7, pure/api & pure/isa modules (0 gated findings)
./omnic backend-diff --suite  # 5-backend differential suite (242 graded: 204 pass, 38 xfail, 0 FAIL, 0 XPASS)
./omnic acceptance-diff       # 41 executable acceptance goldens
./omnic pure-omni-diff        # 49 Pure-Omni differential checks
./omnic pure-bootstrap        # Phase D & 18-module suite FNV-1a64 hashes
./omnic fixed-point           # Stage-1 == Stage-2 fixed-point ELF64 check (cross-platform)
```

### Zero-Rust-Toolchain Rebuild & Seed-Shrink Classification (Phase C1–C3 / Audit §4)

The **classified Stage-0 seed** is **`236` lines** (`seed0/omni_seed0.rs` 134 lines + `p4_shim/mod.rs` 102 lines — a **99.51% classified seed reduction** from the 48,590-line `main@bc2711e6` baseline; see `oracle/README.md`). At runtime, **`35,564` lines of host Rust** (`omni_rust_bootstrap.rs` 26,068 + `m2_host/*.rs` 8,399 + `host_runtime/pure_default.rs` 995 + `p4_shim/mod.rs` 102; out of `35,738` active Rust lines including `40`-line `omni_dev_host.rs` and `134`-line `seed0/omni_seed0.rs`) orchestrate the CLI, while on Linux x86-64 the default path executes hash-pinned native `m3.elf`/`m2.elf`/`m4.elf`/`verifier.elf` binaries directly (`scripts/zero_rust_rebuild.sh` proves **no Rust toolchain (`rustc`/`cargo`) on `PATH`**):

```bash
bash scripts/zero_rust_rebuild.sh          # C1: strips rustc/cargo from PATH, verifies all 3 seeds, 237-B M5 validator/entry stub (0x45EE82AAE5008841), native m2.elf->m4.elf (89,772 B, 0xD4B4CE81CFFF4636), M2/M3 fixed points & P7 gates
python3 scripts/external_verifier_check.py # C2: independent zero-dependency Python 3 checker over spec/verifier_traces/pure_verifier_golden_traces.json
```

### Archived 8-Crate Workspace (`oracle/archived-workspace/`)

```bash
cargo run --release --manifest-path oracle/archived-workspace/Cargo.toml -p omni-cli -- selftest         # 707 checks on Linux x86-64
cargo run --release --manifest-path oracle/archived-workspace/Cargo.toml -p omni-cli -- acceptance-diff
cargo run --release --manifest-path oracle/archived-workspace/Cargo.toml -p omni-cli -- pure-omni-diff
cargo test --release --manifest-path oracle/archived-workspace/Cargo.toml --workspace -- --test-threads=1 # 13/13 tests on Linux x86-64
```
