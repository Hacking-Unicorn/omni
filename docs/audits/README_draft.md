# Omni

Omni is an AI-native 16-bit token-stream ISA (`0x7F00..=0x7F7F`, 128 opcodes) with a linear-ownership, capability, and taint verifier, a deterministic VM, `@PACK5` balanced-ternary packing, and four backends (VM, x86-64 ELF64, Stage-1 ELF64, Wasm).

This repository is a **Stage-0 Rust bootstrap** plus a growing set of **Pure-Omni** modules. The long-term goal is to replace the Rust implementation with Omni, leaving Rust only as a thin Stage-0 seed (compiler driver, syscalls, and a reference verifier). That replacement is **not done**. The sections below say exactly what still runs in Rust and what already runs as `.omni`.

**Hard constraints (intentional, not temporary gaps):**

- Zero external crates in the core workspace (sovereignty constraint).
- 192 SSA registers and 16 labels per function, 64-deep calls, eager evaluation.
- No heap structs, maps, or GC in core `.omni`.
- No recursion in core `.omni` modules.
- Never overclaim: a locked hash is not a self-hosting proof.

## What runs where (Stage-0 Rust vs Stage-1 Omni)

| Stage | Runs in | Status on `isa/multibyte-loads` (`8160842`) |
|---|---|---|
| Surface lexer, parser, include graph, macros, lowering to `u16` | **Rust** (`omni-lang` `compiler/`, and the monolith) | Production path. `pure/compiler.omni` and the Phase A kernel only map pre-digested integer flags to error codes; they do not tokenize or parse source. Functions named `compile_omni_surface_via_pure_omni_frontend*` still call `compile_omni_surface_to_u16_with_sourcemap`. |
| Production static verifier | **Rust** (`omni-verifier`, `verify_binary_function` / `verify_u16_stream`) | Default for compile, run, and serve. |
| Phase 7 "Pure-Omni verifier kernel" (`pure/verifier.omni`, 51 lines) | Omni kernel + **Rust analysis** | Rust walks the token stream, computes `(missing_cap, taint_kind, arena_balance)`, then the Omni kernel maps those three integers to `error[<CODE>]`. Useful as a 4-backend oracle; not a stream verifier. |
| Omni verifier engine (`pure/verifier_engine.omni`, 1,034 lines, 58 functions, loops only) | **Omni guest** (executed by the Rust VM) | Real u16-stream walker, differentially checked against the Rust twin (77/77 cases in `run_verifier_engine_diff_selftests`). **Not** the default path; lockstep shadow mode on compile/run/serve is not wired yet. Intentionally excluded from the 18-module hash suite. |
| VM (`OmniBinaryVm`) | **Rust** | Must remain in the Stage-0 seed until a native backend can host the compiler. |
| x86-64 ELF64 and Wasm AOT emitters | **Rust** (`omni-aot`) | Cover the documented numeric / ternary / control-flow / `@PACK5` / arena opcode subset (`is_x86_64_aot_supported_opcode`, `is_wasm_aot_supported_opcode`, 52 opcodes, host opcodes rejected). Wasm `IGET`/`ISET` do not yet store or reload general byte-buffer contents (unhandled `IGET` yields 0; `ISET` updates length only). `byte_buf_golden` checks length, not bytes. |
| Stage-1 / Stage-2 compiler stream (`build_step7_stage1_compiler_stream`) | A **Rust-generated** `u16` program that emits ELF64 | A real (subset) token-to-ELF64 emitter, locked at 19,731 `u16` words / 106,242 ELF bytes, with a Stage-1 == Stage-2 fixed point. This is not `pure/compiler.omni` and is not a surface-language compiler. |
| Live host (`omni serve`: TCP HTTP/1.1, file-backed DB/KV) | **Rust** `OmniNarrowHost` (10 syscalls) plus Rust HTTP/KV/route code | Syscalls stay in the Stage-0 seed. `pure/host_runtime.omni` is a cap/taint/status-packing kernel, not a TCP stack. FS/DB paths are canonicalized into a sandbox root and rejected with `error[PATH_ESCAPE]`. |
| In-VM host simulation | **Rust** VM | Socket-free tests; `respond` returns `Int(status * 1_000_000 + body)`. |
| Repair agent, harness, inspect, acceptance runner, CLI | **Rust** | `pure/tooling.omni` provides small decision kernels (strategy id, error stage, fixed-point compare). |
| `std/*.omni` and `pure/programs/*.omni` | **Omni guest** | Real (small) library and demo programs, compiled and run by the Rust pipeline. |

`run`, `eval`, `repl`, `serve`, `compile-surface`, `inspect`, `harness`, and `agent-driver` do call functions with `via_pure_omni` in the name by default (`--legacy-rust` opts out of that wrapper on the harness). That wrapper is not a Pure-Omni compiler: it runs a tiny Omni kernel, then the Rust compiler and verifier.

### About the locked "self-host" hashes

These numbers are real regression locks. They are **not** proof that Omni compiles Omni independently of Rust.

| Artifact | Size | FNV-1a64 (locked) | What it actually is |
|---|---|---|---|
| Phase D kernel `.omni_bc` (`gen1 == gen2`) | 159 `u16` words | `0x23B8D17257CCA9C4` | The 34-line `omni_selfhost_compiler_main` decision kernel, compiled twice by the Rust surface compiler. Gen-1 does not emit Gen-2. |
| Phase D kernel `.elf` | 824 bytes | `0xE221085777B8AC47` | The same kernel lowered by the Rust x86-64 / Stage-1 emitters. |
| Closed 18-module suite (`std/*` + `pure/*` excluding `verifier_engine.omni` + `pure/programs/*`) | 14,559 `u16` words | `0x78462AE4C81DF408` | Combined token stream of those modules, produced by the Rust compiler. "Gen-1 == Gen-2" means the "Pure-Omni frontend" wrapper and the Rust compiler agree, because the wrapper *is* the Rust compiler. |
| Stage-1 compiler stream == Stage-2 | 19,731 `u16` words / 106,242 ELF bytes | `0xEAFC81F46B482FF8` | Fixed point of the Rust-generated u16-to-ELF64 emitter. Stronger than the 159-word kernel; still generated and hosted by Rust. |

## Repository layout

- [`pure/`](./pure/) — Pure-Omni modules. Kernels (`compiler.omni`, `verifier.omni`, `host_runtime.omni`, `tooling.omni`, `selfhost_suite.omni`) plus the real stream-walking [`pure/verifier_engine.omni`](./pure/verifier_engine.omni) and six programs in [`pure/programs/`](./pure/programs/).
- [`std/`](./std/) — Pure-Omni standard library: `prelude`, `math`, `trit`, `http`, `pack5`, `decision`, `resource`.
- [`omni_rust_bootstrap.rs`](./omni_rust_bootstrap.rs) — Standalone single-file Stage-0 monolith (expected **416** self-test checks on this branch). Builds with `rustc -O omni_rust_bootstrap.rs -o omni`.
- [`omni-workspace/`](./omni-workspace/) — 8-crate Cargo workspace (expected **378** Linux differential checks on this branch; Windows may differ by the Unix-only symlink sandbox case):
  - `crates/omni-isa` — 128-opcode table, token ranges, contract versions, zero-allocation `@PACK5` codec, FNV-1a64.
  - `crates/omni-ir` — `u16` token-stream decoder and IR types.
  - `crates/omni-verifier` — Production O(n) capability, linear ownership, SSA, and taint verifier.
  - `crates/omni-vm` — Deterministic `OmniBinaryVm` (fuel, memory budget, 64-deep calls, in-VM host simulation).
  - `crates/omni-assembler` — Instruction packet encoder and `.omni_asm` text assembler.
  - `crates/omni-lang` — Surface compiler, host, repair agent, harness, inspect, acceptance, Pure-Omni wrappers, and the Omni verifier-engine driver.
  - `crates/omni-aot` — Rust x86-64 ELF64 compiler, Wasm compiler and evaluator, Stage-1 stream builder.
  - `crates/omni-cli` — `omnic` plus `omni-harness` and `omni-agent-driver`.
- [`spec/`](./spec/) — Acceptance goldens (`spec/acceptance/`, 24 positive + 15 negative in the manifest; 40 `.omni` files on disk), host interface, trusted-core and proof-plan notes, and roadmaps.
- [`spec.md`](./spec.md) — Normative specification. Some check counts and a few Section 6 opcode names are stale relative to this branch; prefer the ISA table in `omni-isa` when they disagree.
- [`LICENSE`](./LICENSE) — MIT, Copyright (c) 2026 Caleb Peacock.

## Quick start

Confirm the check counts below with a fresh `selftest` before treating them as frozen. Arithmetic on this branch: previous `main` baseline 378 monolith / 340 Linux workspace, plus 4 sandbox checks, 1 verifier-engine gate (covering 77 differential cases), and 33 `if_pos` join-slot checks.

### Linux / macOS

```bash
# Monolith (expected 416 checks)
rustc -O omni_rust_bootstrap.rs -o omni
./omni selftest
./omni pure-bootstrap
./omni pure-omni-diff
./omni acceptance-diff
./omni inspect spec/acceptance/goldens/decision_hook_golden.omni
./omni --explain TAINT_SINK

# 8-crate workspace (expected 378 Linux checks)
cd omni-workspace
CARGO_INCREMENTAL=0 cargo build --workspace
cargo run -p omni-cli -- selftest
cargo run -p omni-cli -- pure-omni-diff
cargo test --workspace
```

### Windows (PowerShell)

```powershell
rustc -O omni_rust_bootstrap.rs -o omni.exe
.\omni.exe selftest
.\omni.exe pure-bootstrap
.\omni.exe pure-omni-diff

cd omni-workspace
$env:CARGO_INCREMENTAL=0; cargo build --workspace
cargo run -p omni-cli -- selftest
cargo run -p omni-cli -- pure-omni-diff
```

### Harness, agent driver, eval, and serve

```bash
cargo run -p omni-cli --bin omni-harness -- --spec
cargo run -p omni-cli --bin omni-harness -- --strict "fn main(a: i64) -> i64 { let b: i64 = a * 2; return b; }" 21
cargo run -p omni-cli --bin omni-agent-driver -- --strict "fn main(a: int) -> trit { let d = a - 10; let g = egate(d); return g; }" 25

cargo run -p omni-cli -- eval "#[cap(AI)] fn main(a: i64, b: i64) -> trit { let g = egate(a - b); return g; }" 30 10
cargo run -p omni-cli -- serve app.omni 127.0.0.1:8080 --max-requests 10 --db omni_host.kv
cargo run -p omni-cli -- repl
```

`omni-harness --legacy-rust` forces the Rust-only compile wrapper. The default `--pure-omni` path still uses the Rust compiler; it only adds the small Omni kernels described above.

## Roadmap

Work stays on feature branches. `main` is not updated until the suites and locked hashes say so. Zero external crates, and every commit re-runs the selftests.

1. **Keep the README honest** (this draft). Update `spec.md`, `spec/PURE_SELF_HOST_ROADMAP.md`, `omni_full_status_audit.md`, and CI job titles so they stop advertising 378/339/340 and "Pure-Omni by default" as if the compiler had moved.
2. **Finish the verifier port the honest way.** Wire `pure/verifier_engine.omni` into `compile`, `run`, and `serve` as lockstep shadow mode against the Rust reference (verdict, error code, and span parity; hard-error on mismatch). Do not flip it to the default until lockstep is boring. Do not raise SSA/label/call limits or add recursion to make this easier.
3. **Multi-byte loads and stores, no new opcodes.** Lower surface `ld_u16`/`ld_u32`/`ld_u64` and `st_u16`/`st_u32`/`st_u64` to new operand *shapes* of existing `IGET`/`ISET` (modes `-2`/`-4`/`-8`, with `st_u64` keeping `-9` behavior unchanged). Then implement those shapes on all four backends, including real Wasm memory traffic, and add a byte-content golden that actually reads the bytes back. After that, slim `verifier_engine.omni` onto the helpers.
4. **Modularize the remaining Stage-0 tree.** The `lib.rs` split and MIT license are already done on the hygiene branch. Still open: move the VM-facing code into a clear `vm/` boundary in `omni-lang` if anything remains there, and keep the monolith and workspace clones in sync.
5. **Port the next modules after the verifier** (see `audits/port_map.md`):
   1. Token-stream decoder (`omni-ir`) — already started inside `verifier_engine.omni`.
   2. `@PACK5` / ISA helpers (`omni-isa`, building on `std/pack5.omni`).
   3. Surface frontend (lexer / parser / lowering in `compiler/mod.rs`), as many small loop-only functions that stay inside 192 SSA / 16 labels / 64-deep calls, with an arena-encoded AST instead of heap structs.
6. **Milestone 2: a true two-pass Omni u16-to-ELF64 emitter** in `pure/compiler.omni`, with a real `stage2 == stage3` fixed point that starts from tokens (and, eventually, from Omni source tokenized in Omni). Do not treat the 159-word Phase D kernel as that emitter.
7. **Thin the Stage-0 seed.** Once the Omni compiler, verifier, and assembler can bootstrap, Rust should shrink to: `rustc` entry, `OmniNarrowHost` syscalls, the reference verifier twin, and whatever host loader is still required to launch Stage-1. The VM stays in the seed until AOT can host the compiler.

HTTP, DB, and the repair agent belong in core on purpose. Their *syscall* edge stays in Rust; policy and protocol logic can move to Omni when the frontend can express them.
