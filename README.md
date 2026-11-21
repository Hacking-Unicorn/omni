# Omni

Omni is a 16-bit token-stream ISA (128 opcodes, `0x7F00..=0x7F7F`) with a capability / linear-ownership / taint static verifier, a deterministic VM, `@PACK5` balanced-ternary packing, a `.omni` surface language, and native `x86-64` ELF64 and 64-bit WebAssembly backends for a documented opcode subset.

**Status, in one line:** Omni is **self-hosted end-to-end in Pure-Omni across the x86-64 ELF64 backend (M2), the surface `.omni` → `.omni_bc` frontend (M3), the deterministic token-stream VM (M4), and the zero-shim Linux x86-64 syscall emitter (M5)**: 10,050 lines of real, differentially-tested iterative `.omni` compiler/verifier/VM modules in `pure/m3/`, `pure/m2/`, `pure/m4/`, `pure/m5/`, `pure/isa/`, `pure/verifier_engine.omni`, and `pure/host_kv.omni` compile (a) the 164,632-byte expanded M3 frontend source bit-for-bit to `pure/m3/m3.omni_bc` (`59,474` words / `118,948` B, `0`-word diff vs Rust), (b) the 33,551-word M2 backend seed `pure/m2/m2.omni_bc` bit-for-bit to `93,561`-B ELF64 (`0`-byte diff vs Rust), (c) the 26,076-word Pure-Omni M4 VM seed `pure/m4/m4.omni_bc` via Pure-Omni M3 (`0`-word diff) and Pure-Omni M2 → `78,020`-B ELF64 (`0`-byte diff vs Rust; `p5-selfhost`), and (d) the 1,034-line `pure/verifier_engine.omni` end-to-end through Pure-Omni M3 → Pure-Omni M2 → `30,063`-B ELF64 (`0`-byte diff vs Rust), with multi-function Wasm AOT + `CALL` + depth-64 evaluator (`M2-10`), Pure-Omni multi-AOT module verifier (`M2-14`), opt-in `lockstep_shadow_frontend` (`M3-14`), 4-backend parity (`M3-15`), raw `.omni_bc` 3-op thin-shim execution + Wasm-multi VM host (`P4-1..P4-5`), and a standalone `177`-byte zero-shim Linux x86-64 ELF64 (`P5-2`). The Stage-0 Rust host still handles `include` file expansion, macro-using legacy fixtures, and the live HTTP/KV dev tooling; `@PACK5` ops are excluded from M2.

## Design constraints

These limits are intentional invariants (`HANDOFF.md` §2):

- **Zero external crates.** `Cargo.lock` lists only the eight workspace packages.
- **192 SSA registers and 16 labels (`0..=15`) per function** (enforced by both surface compiler and verifier); **64-deep call stack** (enforced by the VM and `m2-lint`).
- **Eager argument evaluation.** No heap structs, hash maps, or GC in core `.omni`; data lives in flat linear byte arenas (compile-time offset constants and arena-backed sorted arrays with binary search are used).
- **No recursion in core `.omni` modules** (iterative state machines + explicit worklists, enforced by `m2-lint` and `m3-gate`).
- **The 128-opcode table (`0x7F00..=0x7F7F`) is never renumbered.**
- **4-backend differential parity** (`vm`, `wasm`, `x86`, `stage1`) gated by `backend-diff --suite` (`diff4`), where any unexpected `FAIL` or `XPASS` fails the suite.

## What runs where

### Real iterative `.omni` core modules (executed on the VM & differentially gated)

| Module | Lines | What it actually does |
|---|---:|---|
| `pure/verifier_engine.omni` | 1,034 | Full static token-stream verifier (capabilities, taint, linear/borrow state, SSA order, labels, opcodes). Runs in **lockstep shadow mode** against the Rust verifier on `compile` / `run` / `serve` and passes the differential suite. |
| `pure/m2/decode.omni` + `pure/m2/util.omni` | 650 | Shared u16 token-stream decoder (`m2d_decode_at`, `m2d_decode_at_ws`) reading opcode metadata from `pure/isa/table.omni`. Differentially tested against Rust's `decode_omni_token_stream`. |
| `pure/isa/table.omni` | 505 | Shared ISA rodata table (128 opcode rows + 81 frozen mnemonic ids `0..=80`), using strict-bounds `ld_u8`/`ld_u16`. Drift-locked against Rust (`13BA`). |
| `pure/host_kv.omni` | 389 | Arena-backed sorted-array KV store (`0x1B0000..0x1EFFFF`, iterative binary search, 8192-key cap, miss = `-1`). Runs in lockstep against the Rust VM KV arms (`13AZ`). |
| `pure/isa/encode.omni` | 296 | Shared u16 packet & module encoder (`M3-04`). Verified bit-for-bit (`0` byte diff) on round-trip over every positive acceptance golden, `pure/verifier_engine.omni` (11,893 words), and `stage1_compiler_stream` (19,731 words). |
| `pure/m2/labels.omni` | 116 | Arena-backed `u32` label table & resolver (`0x20000..0x23FFF`, `code_off + 1` / `0 = undef`, first-definition-wins, LBL `0x7EF0..0x7EFF` disjoint from IMM `0..4079`, IMM ≥ 4080 rejected). |
| `pure/m2/arena.omni` + `pure/m2/elf.omni` | 301 | M2 D-8 arena state/output band (`0x24000..0x7FEFF`), D-7 >128 KiB hard rejection, `m2a_finish_slice` (`ret_slice`), byte-exact 120-B ELF64 header, trap stubs, and x86-64 frame/multi prologues (`m2-elf`). |
| `pure/m2/emit_x86.omni` + `pure/m2/driver.omni` | 668 | Two-pass single-function u16 → x86-64 ELF64 compiler (`M2-12`/`M2-13`). Compiles the full **19,731-word Stage-1 compiler stream** to the exact `106,242`-byte ELF64 (`0xEAFC81F46B482FF8`, `0`-byte diff vs Rust; `m2-compile`). |
| `pure/m2/verify/mod_check.omni` | 334 | Iterative multi-AOT module verifier (`M2-14`): unique `fn_idx`, `CALL` target/arity checks, iterative Kahn topological-sort cycle detection (no recursion), and L1–L12 label-key parity via `pure/m2/labels.omni` (`m2-verify`, 10/10 differential checks vs Rust). |
| `pure/m2/emit_multi.omni` + `pure/m2/m2_standalone.omni` | 1,029 | Two-pass multi-function u16 → x86-64 ELF64 compiler (`M2-15`/`M2-16` + Phase 7 compact encodings: `disp8` frame loads/stores, 3-tier compact immediates, `sub/add rsp, imm8`, 0-arg elision, D-4 stack args `[rbp+16+8k]`, 1 MiB `rbx` arena, `CALL`, 3-op `RET`, ISA-B `IGET`/`ISET`, trap stubs). Compiles `pure/compiler.omni`, `pure/verifier_engine.omni`, and its own 33,551-word seed `pure/m2/m2.omni_bc` (`0xC30924711C8C95E9`) → `93,561`-B ELF64 (`0x763478CC11DB8026`, −31.8% vs 137,246 B) with `0`-byte diff vs Rust (`m2-multi-pure`, `m2-fixed-point`). |
| `pure/m3/*.omni` (`arena`, `lex`, `skim`, `expr`, `stmt`, `lower`) | 3,621 | Iterative surface `.omni` → `.omni_bc` frontend (`M3-05..M3-17`: lexer, Pass S fn/macro/const skimmer, Pratt expression parser, iterative statement parser, full SSA/CFG/opcode lowerer with `if_pos`/`if_neg`, `match_trit`, `StrLit`, loops, labels/gotos, and O(1) scopes). Self-compiles its own 164,632-B expanded source to `pure/m3/m3.omni_bc` (`59,474` words, `0x0668919FB4378D88`, `0`-word diff vs Rust) and chains with Pure-Omni M2 on `pure/verifier_engine.omni` → `30,063`-B ELF64 (`0xA9290A98CF42DD6B`, −27.2% vs 41,285 B, `0`-byte diff vs Rust; `m3-fixed-point`, `m3-lower`, `m3-gate`). |
| `pure/m4/*.omni` (`arena`, `ops_core`, `vm`) | 962 | Phase 4/7 AI-native Pure-Omni deterministic token-stream VM (`P4-1..P4-5` + Phase 7 hot-path tuning: O(1) pre-indexed label cache in `0x20000..0x23FFF`, guarded opcode-family dispatch, `LABEL`/`NOP` fast path, single `m2d_ins_scan` per step): executes raw `.omni_bc` directly in a 512 KiB flat arena (64-deep call stack, 224 slots/frame, fuel cap, ISA-B byte buffers, built-in 64 KiB binary trace ring `0x70000..0x7FFFF`), backed by a minimal 3-op thin shim (`p4_shim/mod.rs`: `read_bytes`/`write_bytes`/`spawn`) and running on M2-10 Wasm-multi (`p4-vm`, `p4-exec`, `p4-shim`, `p4-wasm`). Deterministic seed `pure/m4/m4.omni_bc` (`26,076` words, `0x00299F8C0BB3F88A`) compiles via Pure-Omni M3 (`0`-word diff) and lowers via Pure-Omni M2 to `78,020`-B ELF64 (`0xF66A7081E2B43319`, `0`-byte diff; `p5-selfhost`). |
| `pure/m5/elf_sys.omni` | 73 | Phase 5 zero-shim Linux x86-64 syscall emitter (`P5-2`): emits raw `0F 05` syscalls (`sys_read`, `sys_write`, `sys_exit`) + `_start` stack-arena entry stub into a standalone `177`-byte Linux x86-64 ELF64 binary with zero Rust, zero C, and zero shim (`p5-sys`). |
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

### Frozen regression hashes (pinned in code by `13BF` / `M2-01`)

All 5 frozen artifacts are asserted by FNV-1a64 hash and exact size in `selftest` and are intentionally kept frozen until the planned single re-freeze after Milestone 2 (`stage2 == stage3`):

| Artifact | Size | FNV-1a64 | What it is |
|---|---:|---|---|
| Phase D kernel `.omni_bc` | 159 words | `0x23B8D17257CCA9C4` | 3-input decision kernel (regression lock, not a full self-hosted compiler) |
| Phase D kernel `.elf` | 824 B | `0xE221085777B8AC47` | Phase D kernel lowered via host x86-64 and Stage-1 |
| 18-module core suite | 14,559 words | `0x78462AE4C81DF408` | 7 `std/*` + 5 legacy `pure/*` kernels + 6 `pure/programs/*` |
| Stage-1 bytecode (`omni_self_compiler.omni_bc`) | 19,731 words (39,462 B) | `0x677F57AE71D28FED` | Rust-generated Stage-1 self-compiler token stream |
| Stage-1 ELF (`stage1.elf == stage2.elf`) | 106,242 B | `0xEAFC81F46B482FF8` | Bit-exact Stage-1 == Stage-2 fixed-point ELF64 |

## Repository layout

- `omni_rust_bootstrap.rs` — Standalone Stage-0 monolith (`rustc -O omni_rust_bootstrap.rs -o omnic`, zero warnings).
- `omni-workspace/` — 8-crate Cargo workspace (`omni-isa`, `omni-ir`, `omni-verifier`, `omni-vm`, `omni-assembler`, `omni-lang`, `omni-aot`, `omni-cli`).
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
./omnic selftest              # 1,416 checks on Windows / 1,532 on Linux x86-64 (13AY diff4, 13AZ KV, 13BA ISA table, 13BF hash pin, M2/M3/P4/P5/P7/API locks)
./omnic p7-measure            # Phase 7 (P7-0): 5-benchmark corpus + P4-5 trace ring + pure/m7/judge.omni ternary (+1t/0t/-1t) + @PACK5 verdict gate
./omnic p7-canary             # Phase 7 (P7-0a): 5-run consensus + subsystem attribution (M3/M2/M4) + M2 emitter canary (+1t) & neutral (0t) gate
./omnic p7-heal               # Phase 7 (P7-1): self-healing taxonomy, FNV-1a64 rollback, bounded N-attempt repair & fault-injection gate
./omnic api-stage1            # New-Level API Stage 1: pure/api/ternary.omni + cap_narrow + Pure-Omni verifier 1010..1013 + 7 goldens (0-diff M3/M2/M4)
./omnic p5-selfhost           # Phase 5 full zero-Rust chain: M3->pure/m4/m4.omni_bc (26,076w)->M2 ELF64 (78,020 B) + M3->M4/M2 loop
./omnic p5-sys                # Phase 5 zero-shim Linux x86-64 ELF64 (177 B, raw 0F 05 sys_read/sys_write/sys_exit; native exec on Linux)
./omnic p4-vm                 # Phase 4 Pure-Omni token-stream VM == Rust OmniBinaryVm (11/11 checks incl. P4-5 trace ring)
./omnic p4-exec <f.omni_bc>   # Phase 4 raw .omni_bc execution via 3-op OmniThinShim (zero source/parser at runtime)
./omnic p4-shim               # Phase 4 minimal 3-op thin host shim (read_bytes, write_bytes, spawn — zero HTTP/shell)
./omnic p4-wasm               # Phase 4 full Pure-Omni VM compiles to Wasm v1 + VM core executes on M2-10 Wasm-multi host
./omnic m3-fixed-point        # Pure-Omni M3 frontend fixed point (59,474 words, 0-word diff) + E2E M3+M2 ELF64 chain
./omnic m3-4backend           # 4-backend parity (VM == Wasm-multi == Rust-x86-multi == Pure-Omni-M2-ELF64; Stage-1 N/A)
./omnic m3-lockstep           # Opt-in lockstep_shadow_frontend (OMNI_M3_LOCKSTEP=1) parity + mismatch hard-fail checks
./omnic m3-lower              # Pure-Omni M3 lowering parity vs Rust (20/20 modules incl. verifier_engine, 0-word diff)
./omnic m3-gate               # lints + compiles + verifies all 6 pure/m3/*.omni modules
./omnic m2-fixed-point        # M2 self-hosted compiler drift check + S1 == V (and S1 == S2 == S3 on Linux x86-64)
./omnic m2-verify             # Pure-Omni multi-AOT module verifier + Kahn cycle check + L1-L12 labels (10/10 vs Rust)
./omnic m2-wasm-multi         # Multi-function Wasm v1 emitter + CALL + shared arena global + depth-64 evaluator (7/7)
./omnic m2-compile            # Pure-Omni 2-pass compiler on 19,731-word Stage-1 anchor (106,242 B, 0-byte diff)
./omnic m2-multi-pure         # Pure-Omni multi-function compiler (0-byte diff vs Rust on verifier_engine & self)
./omnic m2-multi              # Rust M2-09 multi-function x86-64 ELF64 reference checks
./omnic m2-elf                # Pure-Omni arena & ELF64 header/stub checks
./omnic m2-lint               # lints all 17 registered pure/m2, pure/m3, pure/m4, pure/m5, pure/m7 & pure/isa modules (0 gated findings)
./omnic backend-diff --suite  # 4-backend differential suite (0 FAIL, 0 XPASS required)
./omnic acceptance-diff       # 41 executable acceptance goldens
./omnic pure-omni-diff        # 49 Pure-Omni differential checks
./omnic pure-bootstrap        # Phase D & 18-module suite FNV-1a64 hashes
./omnic fixed-point           # Stage-1 == Stage-2 fixed-point ELF64 check (cross-platform)
```

### 8-Crate Workspace (`cargo`)

```bash
cargo run --release --manifest-path omni-workspace/Cargo.toml -p omni-cli -- selftest         # 667 checks on Windows / 707 on Linux x86-64
cargo run --release --manifest-path omni-workspace/Cargo.toml -p omni-cli -- acceptance-diff
cargo run --release --manifest-path omni-workspace/Cargo.toml -p omni-cli -- pure-omni-diff
cargo test --release --manifest-path omni-workspace/Cargo.toml --workspace
```
