# Omni Language, ISA, Verifier, VM, AOT Compiler & Model Harness

> **Workspace Path Note:** The 8-crate Cargo workspace lives at [`omni-workspace/omni-workspace/`](./omni-workspace/omni-workspace/), while the standalone single-file reference compiler lives at the repository root in [`omni_rust_bootstrap.rs`](./omni_rust_bootstrap.rs) (compiles directly with `rustc -O omni_rust_bootstrap.rs -o omni` with no flags required).

Zero-dependency Rust implementation of the **Omni** 16-bit binary token ISA (`0x7F00..=0x7F7F`), static linear/capability/taint verifier with machine-readable error codes (`error[<CODE>]`) and 1-based `(line, col)` source spans, deterministic ternary & numeric VM (`OmniBinaryVm`) with both in-VM host simulation and a live native TCP HTTP/1.1 + DB/KV host runtime (`omni serve`: `route`, `req.param`, `db_query`, `db_get`, `kv_put`, `auth_verify`, `respond`), self-hosting Stage-1/Stage-2 `x86-64` ELF64 and 64-bit WebAssembly AOT backends, `.omni` surface language compiler (`omni-lang`) with strict single-form mode (`#[strict]`), the version-locked model driver (`omni-harness`, contract `omni-harness-v1.0.0`), and the autonomous emit-and-repair agent driver (`omni-agent-driver`).

## Repository Layout

- [`omni_rust_bootstrap.rs`](./omni_rust_bootstrap.rs) — Standalone single-file reference monolith (`248` self-test checks, `0` warnings).
- [`omni-workspace/omni-workspace/`](./omni-workspace/omni-workspace/) — 8-crate Cargo workspace (`209` Windows / `210` Linux `x86-64` monolith-vs-workspace differential checks, `0` warnings):
  - `crates/omni-isa` — 128-opcode table (`0x7F00..=0x7F7F`), token ranges, `OMNI_HARNESS_CONTRACT_VERSION`, zero-allocation `@PACK5` (`5 trits/byte`) codec & direct ops, FNV-1a64 hashing.
  - `crates/omni-ir` — Binary token stream decoder (`decode_omni_token_stream`) and IR types.
  - `crates/omni-verifier` — Static capability bitmask, linear ownership/borrow/arena state machine, SSA, and `@UNTRUSTED` taint verifier (`verify_binary_function`, `verify_u16_stream`) emitting machine-readable `error[<CODE>]` diagnostics.
  - `crates/omni-vm` — Deterministic execution engine (`OmniBinaryVm`) with explicit step/fuel and memory budgets and full-stack host ABI execution (`0x7F64..=0x7F70`).
  - `crates/omni-assembler` — Direct binary instruction packet encoder (`encode_asm_ins_packet_u16`) and `.omni_asm` text assembler (`assemble_omni_text_to_u16`).
  - `crates/omni-lang` — `.omni` surface language lexer, parser, strict single-form mode (`#[strict]` / `compile_omni_surface_strict_to_u16`), binary offset sourcemap (`SurfaceSourceMap`), full-stack `route` + DB/KV lowering and live TCP HTTP/1.1 server (`serve_omni_http_once`, `serve_omni_http_batch`), multi-file `include` linker, direct zero-IR `Vec<u16>` emitter (`compile_omni_surface_to_u16`), `#[step_budget(N)]`/`#[mem_budget(N)]` attributes, AOT-first dispatcher (`execute_omni_surface_aot_first`), version-locked JSON harness (`run_omni_harness_json`), and emit-and-repair driver (`run_omni_agent_repair_loop`).
  - `crates/omni-aot` — Two-pass `x86-64` Linux ELF64 compiler, 64-bit WebAssembly (`\0asm`) compiler & budgeted evaluator (`execute_wasm_module_i64_with_budgets`), and Stage-1/Stage-2 self-compiler (`omni_self_compiler.omni_bc`).
  - `crates/omni-cli` — CLI runner (`omnic`: `selftest`, `run`, `eval`, `repl`, `harness`, `agent-driver`, `serve`, `compile-surface`, `surface-to-asm`, `compile-elf`, `compile-stage1`, `compile-wasm`, `run-wasm`) and standalone `omni-harness` and `omni-agent-driver` binary targets.
- [`spec.md`](./spec.md) — Normative specification (`SPEC.md`).
- [`omni_full_status_audit.md`](./omni_full_status_audit.md) — Full architectural and verification status audit.
- [`omni_progress_checklist.md`](./omni_progress_checklist.md) — Phase 1–6 progress checklist.

## Quick Start

### 1. Run the 8-Crate Workspace Selftest (`209` / `210` Checks)
```powershell
cd omni-workspace/omni-workspace
$env:CARGO_INCREMENTAL=0; cargo build --workspace
cargo run -p omni-cli -- selftest
```

### 2. Run the Standalone Monolith Selftest (`248` Checks)
```powershell
rustc -O omni_rust_bootstrap.rs -o omni.exe
.\omni.exe selftest
```

### 3. Run the Version-Locked Model Harness (`omni-harness`) & Agent Driver (`omni-agent-driver`)
```powershell
# Inspect the version-locked grammar, capability, and error-code contract:
cargo run -p omni-cli --bin omni-harness -- --spec

# Execute a strict single-form .omni program and receive single-line JSON:
cargo run -p omni-cli --bin omni-harness -- --strict "fn main(a: i64) -> i64 { let b: i64 = a * 2; return b; }" 21

# Receive a structured machine-readable error code + non-null 1-based line/col + retry hint on failure:
cargo run -p omni-cli --bin omni-harness -- --strict --retry-hint "fn main(x: i64 @untrusted) -> i64 { let safe_a: i64 = 1; return x; }" 1

# Run the autonomous emit-and-repair agent driver on a broken candidate .omni program:
cargo run -p omni-cli --bin omni-agent-driver -- --strict "fn main(a: int) -> trit { let d = a - 10; let g = egate(d); return g; }" 25
```

### 4. Evaluate, Run, or Serve Live Full-Stack `.omni` Programs (`omni serve`)
```powershell
cargo run -p omni-cli -- eval "#[cap(AI)] fn main(a: i64, b: i64) -> trit { let g = egate(a - b); return g; }" 30 10
cargo run -p omni-cli -- eval "route GET /users/:id (req: i64) @cap(net, db_r) { let id = req.param(\"id\"); let row = db_query(\"SELECT * FROM users WHERE id = ?\", id); respond(200, row); }" 42
cargo run -p omni-cli -- serve app.omni 127.0.0.1:8080 --requests 10
cargo run -p omni-cli -- repl
```
