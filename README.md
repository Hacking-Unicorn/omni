# Omni

Omni is a 16-bit token-stream ISA (128 opcodes, `0x7F00..=0x7F7F`) with a capability / linear-ownership / taint static verifier, a deterministic VM, `@PACK5` balanced-ternary packing, a `.omni` surface language, and native `x86-64` ELF64 and 64-bit WebAssembly backends for a documented opcode subset.

**Status, in one line:** Omni is **token-level self-hosted for x86-64 ELF64** (Milestone 2 complete: **~4,300 lines of real, differentially-tested iterative `.omni` core modules** in `pure/m2/` + `pure/isa/` compile both the 19,731-word Stage-1 compiler bit-for-bit to `106,242` B and their own 32,674-word multi-function seed `pure/m2/m2.omni_bc` bit-for-bit to `136,841` B with `0`-byte diff vs Rust). The Stage-0 Rust host still performs surface `.omni` lexing/parsing/lowering (Milestone 3, `spec/m3/`), VM execution, and Wasm AOT (`M2-10` multi-function Wasm `CALL` pending); `@PACK5` ops are excluded from M2.

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
| `pure/m2/arena.omni` + `pure/m2/elf.omni` | 290 | M2 D-8 arena state/output band (`0x24000..0x7FEFF`), D-7 >128 KiB hard rejection, `m2a_finish_slice` (`ret_slice`), byte-exact 120-B ELF64 header, trap stubs, and x86-64 frame/multi prologues (`m2-elf`). |
| `pure/m2/emit_x86.omni` + `pure/m2/driver.omni` | 485 | Two-pass single-function u16 → x86-64 ELF64 compiler (`M2-12`/`M2-13`). Compiles the full **19,731-word Stage-1 compiler stream** to the exact `106,242`-byte ELF64 (`0xEAFC81F46B482FF8`, `0`-byte diff vs Rust; `m2-compile`). |
| `pure/m2/emit_multi.omni` + `pure/m2/m2_standalone.omni` | 620 | Two-pass multi-function u16 → x86-64 ELF64 compiler (`M2-15`/`M2-16`: D-4 stack args `[rbp+16+8k]`, 1 MiB `rbx` arena, `CALL`, 3-op `RET`, ISA-B `IGET`/`ISET`, trap stubs). Compiles `pure/compiler.omni`, `pure/verifier_engine.omni`, and its own 32,674-word seed `pure/m2/m2.omni_bc` → `136,841`-B ELF64 (`0x272A39311643CD65`) with `0`-byte diff vs Rust (`m2-multi-pure`, `m2-fixed-point`). |
| `std/*.omni` (7) + `pure/programs/*.omni` (6) | ~1,000 | Standard library helpers and small production-style programs, served directly from disk via `include_str!` (`M3-16`). |
| `pure/compiler.omni`, `pure/verifier.omni`, `pure/host_runtime.omni`, `pure/tooling.omni`, `pure/selfhost_suite.omni` | ~400 | Legacy Phase 7/A–D integer decision kernels (frozen for regression hashes; not the M2/M3 compilers). |

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
- `pure/` — Pure-Omni core modules (`pure/verifier_engine.omni`, `pure/isa/`, `pure/m2/`, `pure/host_kv.omni`, `pure/programs/`, and frozen legacy kernels).
- `std/` — Pure-Omni standard library (`std/prelude.omni`, `std/math.omni`, `std/trit.omni`, `std/http.omni`, `std/pack5.omni`, `std/decision.omni`, `std/resource.omni`).
- `m2_host/` — Host-side Milestone 2 tooling (`lint.rs` for `m2-lint`, `decode_diff.rs` for decoder/encoder/label differential tests).
- `spec/` — Executable specs & locks: `spec/acceptance/` (41 goldens), `spec/m2/` (M2 plan, decisions, single-mode byte lock, shape census, label reference, lint fixtures), `spec/m3/` (M3 plan, decisions, `corpus.tsv` frontend lock, Q1–Q8 & negative fixtures), `spec/host_runtime/`.
- `docs/archive/` — Historical Nov 6–7 audit/progress snapshots (for current living status, see `HANDOFF.md` on the `docs/handoff` branch, `spec/m2/`, and `spec/m3/`).
- `spec.md` — ISA and language specification (`OMNI_ISA_TABLE` is authoritative where counts differ).
- `LICENSE` — MIT.

## Quick start

### Monolith (`rustc -O`)

```bash
rustc -O omni_rust_bootstrap.rs -o omnic
./omnic selftest              # 1176+ checks (13AY diff4, 13AZ KV, 13BA ISA table, 13BF hash pin, M2/M3 locks)
./omnic m2-fixed-point        # M2 self-hosted compiler drift check + S1 == V (and S1 == S2 == S3 on Linux x86-64)
./omnic m2-compile            # Pure-Omni 2-pass compiler on 19,731-word Stage-1 anchor (106,242 B, 0-byte diff)
./omnic m2-multi-pure         # Pure-Omni multi-function compiler (0-byte diff vs Rust on verifier_engine & self)
./omnic m2-multi              # Rust M2-09 multi-function x86-64 ELF64 reference checks
./omnic m2-elf                # Pure-Omni arena & ELF64 header/stub checks
./omnic m2-lint               # lints all 7 registered pure/m2 & pure/isa modules (0 gated findings)
./omnic backend-diff --suite  # 4-backend differential suite (0 FAIL, 0 XPASS required)
./omnic acceptance-diff       # 41 executable acceptance goldens
./omnic pure-omni-diff        # 49 Pure-Omni differential checks
./omnic pure-bootstrap        # Phase D & 18-module suite FNV-1a64 hashes
./omnic fixed-point           # Stage-1 == Stage-2 fixed-point ELF64 check (cross-platform)
```

### 8-Crate Workspace (`cargo`)

```bash
cargo run --release --manifest-path omni-workspace/Cargo.toml -p omni-cli -- selftest
cargo run --release --manifest-path omni-workspace/Cargo.toml -p omni-cli -- acceptance-diff
cargo run --release --manifest-path omni-workspace/Cargo.toml -p omni-cli -- pure-omni-diff
cargo test --release --manifest-path omni-workspace/Cargo.toml --workspace
```
