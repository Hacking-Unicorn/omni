# Omni Language, ISA, Verifier, VM & AOT Compiler

Zero-dependency Rust implementation of the **Omni** 16-bit binary token ISA (`0x7F00..=0x7F7F`), static linear/capability/taint verifier, deterministic ternary & numeric VM (`OmniBinaryVm`), self-hosting Stage-1/Stage-2 `x86-64` ELF64 and 64-bit WebAssembly AOT backends, and `.omni` surface language compiler (`omni-lang`).

## Repository Layout

- [`omni_rust_bootstrap.rs`](./omni_rust_bootstrap.rs) — Standalone single-file reference monolith (`193` self-test checks).
- [`omni-workspace/omni-workspace/`](./omni-workspace/omni-workspace/) — 8-crate Cargo workspace (`154` monolith-vs-workspace differential checks):
  - `crates/omni-isa` — 128-opcode table (`0x7F00..=0x7F7F`), token ranges, zero-allocation `@PACK5` (`5 trits/byte`) codec & direct ops, FNV-1a64 hashing.
  - `crates/omni-ir` — Binary token stream decoder (`decode_omni_token_stream`) and IR types.
  - `crates/omni-verifier` — Static capability bitmask, linear ownership/borrow/arena state machine, SSA, and `@UNTRUSTED` taint verifier (`verify_binary_function`, `verify_u16_stream`).
  - `crates/omni-vm` — Deterministic execution engine (`OmniBinaryVm`) with explicit step/fuel and memory budgets across all allocating opcodes.
  - `crates/omni-assembler` — Direct binary instruction packet encoder (`encode_asm_ins_packet_u16`) and `.omni_asm` text assembler (`assemble_omni_text_to_u16`).
  - `crates/omni-lang` — `.omni` surface language lexer, parser, multi-file `include` linker, direct zero-IR `Vec<u16>` emitter (`compile_omni_surface_to_u16`), `#[step_budget(N)]`/`#[mem_budget(N)]` attributes, and AOT-first dispatcher (`execute_omni_surface_aot_first`).
  - `crates/omni-aot` — Two-pass `x86-64` Linux ELF64 compiler, 64-bit WebAssembly (`\0asm`) compiler & budgeted evaluator (`execute_wasm_module_i64_with_budgets`), and Stage-1/Stage-2 self-compiler (`omni_self_compiler.omni_bc`).
  - `crates/omni-cli` — CLI runner (`selftest`, `run`, `eval`, `repl`, `compile-surface`, `surface-to-asm`, `compile-elf`, `compile-stage1`, `compile-wasm`, `run-wasm`).
- [`spec.md`](./spec.md) — Normative specification (`SPEC.md`).
- [`omni_full_status_audit.md`](./omni_full_status_audit.md) — Full architectural and verification status audit.
- [`omni_progress_checklist.md`](./omni_progress_checklist.md) — Phase 1–5 progress checklist.

## Quick Start

### 1. Run the 8-Crate Workspace Selftest
```powershell
cd omni-workspace/omni-workspace
$env:CARGO_INCREMENTAL=0; cargo build --workspace
cargo run -p omni-cli -- selftest
```

### 2. Run the Standalone Monolith Selftest
```powershell
rustc -O omni_rust_bootstrap.rs -o omni.exe
.\omni.exe selftest
```

### 3. Evaluate or Run `.omni` Surface Programs
```powershell
cargo run -p omni-cli -- eval "#[cap(AI)] fn main(a: i64, b: i64) -> trit { let g = egate(a - b); return g; }" 30 10
cargo run -p omni-cli -- eval "let acc = 0; let i = 10; while i >= 1 { acc += i; i -= 1; } acc"
cargo run -p omni-cli -- repl
```
