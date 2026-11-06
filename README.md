# Omni Language, ISA, Verifier, VM, AOT Compiler & Model Harness

> **Workspace Path Note:** The 8-crate Cargo workspace lives at [`omni-workspace/omni-workspace/`](./omni-workspace/omni-workspace/), while the standalone single-file reference compiler lives at the repository root in [`omni_rust_bootstrap.rs`](./omni_rust_bootstrap.rs) (compiles directly with `rustc -O omni_rust_bootstrap.rs -o omni` with no flags required).

Zero-dependency Rust implementation of the **Omni** 16-bit binary token ISA (`0x7F00..=0x7F7F`), static linear/capability/taint verifier with machine-readable error codes (`error[<CODE>]`), deterministic ternary & numeric VM (`OmniBinaryVm`) with host ABI (`route`, `req.param`, `db_query`, `db_get`, `kv_put`, `auth_verify`, `respond`), self-hosting Stage-1/Stage-2 `x86-64` ELF64 and 64-bit WebAssembly AOT backends, `.omni` surface language compiler (`omni-lang`) with strict single-form mode (`#[strict]`), and the version-locked model driver (`omni-harness`, contract `omni-harness-v1.0.0`).

## Repository Layout

- [`omni_rust_bootstrap.rs`](./omni_rust_bootstrap.rs) — Standalone single-file reference monolith (`237` self-test checks, `0` warnings).
- [`omni-workspace/omni-workspace/`](./omni-workspace/omni-workspace/) — 8-crate Cargo workspace (`198` Windows / `199` Linux `x86-64` monolith-vs-workspace differential checks, `0` warnings):
  - `crates/omni-isa` — 128-opcode table (`0x7F00..=0x7F7F`), token ranges, `OMNI_HARNESS_CONTRACT_VERSION`, zero-allocation `@PACK5` (`5 trits/byte`) codec & direct ops, FNV-1a64 hashing.
  - `crates/omni-ir` — Binary token stream decoder (`decode_omni_token_stream`) and IR types.
  - `crates/omni-verifier` — Static capability bitmask, linear ownership/borrow/arena state machine, SSA, and `@UNTRUSTED` taint verifier (`verify_binary_function`, `verify_u16_stream`) emitting machine-readable `error[<CODE>]` diagnostics.
  - `crates/omni-vm` — Deterministic execution engine (`OmniBinaryVm`) with explicit step/fuel and memory budgets and full-stack host ABI execution (`0x7F64..=0x7F70`).
  - `crates/omni-assembler` — Direct binary instruction packet encoder (`encode_asm_ins_packet_u16`) and `.omni_asm` text assembler (`assemble_omni_text_to_u16`).
  - `crates/omni-lang` — `.omni` surface language lexer, parser, strict single-form mode (`#[strict]` / `compile_omni_surface_strict_to_u16`), full-stack `route` + DB/KV lowering, multi-file `include` linker, direct zero-IR `Vec<u16>` emitter (`compile_omni_surface_to_u16`), `#[step_budget(N)]`/`#[mem_budget(N)]` attributes, AOT-first dispatcher (`execute_omni_surface_aot_first`), and version-locked JSON harness (`run_omni_harness_json`).
  - `crates/omni-aot` — Two-pass `x86-64` Linux ELF64 compiler, 64-bit WebAssembly (`\0asm`) compiler & budgeted evaluator (`execute_wasm_module_i64_with_budgets`), and Stage-1/Stage-2 self-compiler (`omni_self_compiler.omni_bc`).
  - `crates/omni-cli` — CLI runner (`omnic`: `selftest`, `run`, `eval`, `repl`, `harness`, `compile-surface`, `surface-to-asm`, `compile-elf`, `compile-stage1`, `compile-wasm`, `run-wasm`) and standalone `omni-harness` binary target.
- [`spec.md`](./spec.md) — Normative specification (`SPEC.md`).
- [`omni_full_status_audit.md`](./omni_full_status_audit.md) — Full architectural and verification status audit.
- [`omni_progress_checklist.md`](./omni_progress_checklist.md) — Phase 1–6 progress checklist.

## Quick Start

### 1. Run the 8-Crate Workspace Selftest (`198` / `199` Checks)
```powershell
cd omni-workspace/omni-workspace
$env:CARGO_INCREMENTAL=0; cargo build --workspace
cargo run -p omni-cli -- selftest
```

### 2. Run the Standalone Monolith Selftest (`237` Checks)
```powershell
rustc -O omni_rust_bootstrap.rs -o omni.exe
.\omni.exe selftest
```

### 3. Run the Version-Locked Model Harness (`omni-harness`)
```powershell
# Inspect the version-locked grammar, capability, and error-code contract:
cargo run -p omni-cli --bin omni-harness -- --spec

# Execute a strict single-form .omni program and receive single-line JSON:
cargo run -p omni-cli --bin omni-harness -- --strict "fn main(a: i64) -> i64 { let b: i64 = a * 2; return b; }" 21

# Receive a structured machine-readable error code + retry hint on failure:
cargo run -p omni-cli --bin omni-harness -- --strict --retry-hint "fn main(x: i64 @untrusted) -> i64 { return x; }" 1
```

### 4. Evaluate or Run `.omni` Surface Programs (Numeric, `@PACK5` & Full-Stack Routes)
```powershell
cargo run -p omni-cli -- eval "#[cap(AI)] fn main(a: i64, b: i64) -> trit { let g = egate(a - b); return g; }" 30 10
cargo run -p omni-cli -- eval "route GET /users/:id (req: i64) @cap(net, db_r) { let id = req.param(\"id\"); let row = db_query(\"SELECT * FROM users WHERE id = ?\", id); respond(200, row); }" 42
cargo run -p omni-cli -- repl
```
