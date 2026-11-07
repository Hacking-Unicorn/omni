# Omni Language, ISA, Verifier, VM, AOT Compiler & Model Harness

> **Workspace Path Note:** The 8-crate Cargo workspace lives at [`omni-workspace/omni-workspace/`](./omni-workspace/omni-workspace/), while the standalone single-file reference compiler lives at the repository root in [`omni_rust_bootstrap.rs`](./omni_rust_bootstrap.rs) (compiles directly with `rustc -O omni_rust_bootstrap.rs -o omni` with no flags required).

Zero-dependency Rust implementation of the **Omni** 16-bit binary token ISA (`0x7F00..=0x7F7F`), static linear/capability/taint verifier with machine-readable error codes (`error[<CODE>]`) and 1-based `(line, col)` source spans, deterministic ternary & numeric VM (`OmniBinaryVm`) with both an in-VM host contract simulator and a live native TCP HTTP/1.1 + file-backed DB/KV host (`omni serve`: `route`, `req.param`, `db_query`, `db_get`, `kv_put`, `auth_verify`, `respond`), self-hosting Stage-1/Stage-2 `x86-64` ELF64 and 64-bit WebAssembly AOT backends (verified for the pure numeric/ternary/`@PACK5`/arena subset), `.omni` surface language compiler (`omni-lang`) with strict single-form mode (`#[strict]`), the version-locked model driver (`omni-harness`, contract `omni-harness-v1.0.0`), and the autonomous emit-and-repair agent driver (`omni-agent-driver`).

> **Scope & Honesty Note:**
> - **AOT Backends (`x86-64` ELF64 & 64-bit Wasm)** cover the verified numeric, ternary, control-flow, byte-buffer, `@PACK5`, and linear arena subset; host opcodes (`0x7F64..=0x7F7F`) are explicitly rejected by standalone AOT (`SPEC.md` Section 6).
> - **Host ABI (`route` / DB / KV)** has two explicit modes (`SPEC.md` Section 7.9 & `spec/host_interface.md`): (1) **in-VM host contract simulation** (`execute_omni_surface_program` returning `Int(status * 1_000_000 + body)` for socket-free unit tests) and (2) **live native TCP HTTP/1.1 + file-backed DB/KV host** (`omni serve <file.omni> [addr] [--db <path>]`) built over the 10-syscall `OmniNarrowHost` capability-gated interface (`omni-narrow-host-v1.0.0`).
> - **Pure Omni Self-Hosting (`omni-pure-selfhost-v1.0.0`)**: Stage-1/Stage-2 self-hosts the binary token-to-ELF64 backend (`omni_self_compiler.omni_bc`), the Phase 7 Pure-Omni static verifier kernel (`omni-pure-kernel-v1.0.0`), the Phase A Pure-Omni surface frontend (`omni-pure-frontend-v1.0.0`), the Phase B Pure-Omni driver (`omni-pure-driver-v1.0.0`), the Phase C Pure-Omni host runtime (`omni-pure-host-v1.0.0`), the Phase D self-hosting bootstrap fixed point (`gen1 == gen2`: `159` words `0x23B8D17257CCA9C4`, `824` ELF64 bytes `0xE221085777B8AC47`), the multi-module Pure-Omni Core & Standard Library Suite (`pure/*.omni` + `std/*.omni`: `1,412` `u16` words `0x187D5EDEDC2F4E37`, `gen1 == gen2` fixed point), and the Phase E full differential lock (`omni pure-omni-diff`) are verified across the 4-way VM/Host-ELF64/Stage-1-ELF64/Wasm differential and `OmniNarrowHost`, with `run`, `eval`, `repl`, and `serve` routing through the Pure-Omni driver and host runtime by default ([`spec/PURE_SELF_HOST_ROADMAP.md`](./spec/PURE_SELF_HOST_ROADMAP.md)).

## Repository Layout

- [`pure/`](./pure/) — Pure-Omni core compiler, static verifier, host runtime, tooling kernels, and multi-module self-host suite (`pure/compiler.omni`, `pure/verifier.omni`, `pure/host_runtime.omni`, `pure/tooling.omni`, `pure/selfhost_suite.omni`; `1,412` `u16` words `Gen-1 == Gen-2`).
- [`std/`](./std/) — Pure-Omni standard library modules (`std/prelude.omni`, `std/math.omni`, `std/trit.omni`, `std/http.omni`).
- [`omni_rust_bootstrap.rs`](./omni_rust_bootstrap.rs) — Standalone single-file reference monolith (`351` self-test checks, `0` warnings).
- [`omni-workspace/omni-workspace/`](./omni-workspace/omni-workspace/) — 8-crate Cargo workspace (`312` Windows / `313` Linux `x86-64` monolith-vs-workspace differential checks, `0` warnings):
  - `crates/omni-isa` — 128-opcode table (`0x7F00..=0x7F7F`), token ranges, contract version constants (`OMNI_HARNESS_CONTRACT_VERSION`, `OMNI_REPAIR_STRATEGIES_VERSION`, `OMNI_NARROW_HOST_VERSION`, `OMNI_ACCEPTANCE_SUITE_VERSION`, `OMNI_ERROR_CONTRACT_VERSION`, `OMNI_PHASE7_PURE_KERNEL_VERSION`), zero-allocation `@PACK5` (`5 trits/byte`) codec & direct ops, FNV-1a64 hashing.
  - `crates/omni-ir` — Binary token stream decoder (`decode_omni_token_stream`) and IR types.
  - `crates/omni-verifier` — Static capability bitmask, linear ownership/borrow/arena state machine, SSA, and `@UNTRUSTED` taint verifier (`verify_binary_function`, `verify_u16_stream`) emitting machine-readable `error[<CODE>]` diagnostics.
  - `crates/omni-vm` — Deterministic execution engine (`OmniBinaryVm`) with explicit step/fuel and memory budgets and full-stack host ABI execution (`0x7F64..=0x7F70`).
  - `crates/omni-assembler` — Direct binary instruction packet encoder (`encode_asm_ins_packet_u16`) and `.omni_asm` text assembler (`assemble_omni_text_to_u16`).
  - `crates/omni-lang` — `.omni` surface language lexer, parser, strict single-form mode (`#[strict]` / `compile_omni_surface_strict_to_u16`), hygienic expression macros (`macro`), built-in verified stdlib prelude (`include "std/prelude.omni";`), binary offset sourcemap (`SurfaceSourceMap`), `OmniNarrowHost` 10-syscall boundary, live TCP HTTP/1.1 + file-backed DB/KV server (`serve_omni_http_once`, `serve_omni_http_batch`, `serve_omni_http_batch_with_db_file`, `serve_omni_http_via_narrow_host`), versioned repair strategies (`omni-repair-strategies-v1.1.0`), executable acceptance runner (`run_omni_acceptance_diff`), binary stream inspector (`format_omni_inspect`), and Phase 7 Pure-Omni verifier kernel (`verify_module_via_phase7_pure_omni_kernel`).
  - `crates/omni-aot` — Two-pass `x86-64` Linux ELF64 compiler, 64-bit WebAssembly (`\0asm`) compiler & budgeted evaluator (`execute_wasm_module_i64_with_budgets`), and Stage-1/Stage-2 self-compiler (`omni_self_compiler.omni_bc`).
  - `crates/omni-cli` — CLI runner (`omnic`: `selftest`, `run`, `eval`, `repl`, `harness`, `agent-driver`, `serve`, `inspect`/`disasm`, `acceptance-diff`, `--explain`, `compile-surface`, `surface-to-asm`, `compile-elf`, `compile-stage1`, `compile-wasm`, `run-wasm`) and standalone `omni-harness` and `omni-agent-driver` binary targets.
- [`spec/`](./spec/) — Executable acceptance suite (`spec/acceptance/acceptance_manifest.json` + 40 `.omni` goldens), `spec/IMPROVEMENT_ROADMAP.md`, `spec/error_contract.md`, `spec/host_interface.md`, `spec/next_abstraction.md`, `spec/trusted_core.md`, `spec/mechanized_proof_plan.md`, and `spec/heterogeneous_design.md`.
- [`spec.md`](./spec.md) — Normative specification (`SPEC.md`).
- [`omni_full_status_audit.md`](./omni_full_status_audit.md) — Full architectural and verification status audit.
- [`omni_progress_checklist.md`](./omni_progress_checklist.md) — Phase 1–7 progress checklist.

## Quick Start

### 1. Run the 8-Crate Workspace Selftest (`312` / `313` Checks)
```powershell
cd omni-workspace/omni-workspace
$env:CARGO_INCREMENTAL=0; cargo build --workspace
cargo run -p omni-cli -- selftest
cargo run -p omni-cli -- pure-omni-diff
```

### 2. Run the Standalone Monolith Selftest (`351` Checks), Self-Host Bootstrap & Acceptance Differential Suite
```powershell
rustc -O omni_rust_bootstrap.rs -o omni.exe
.\omni.exe selftest
.\omni.exe pure-bootstrap
.\omni.exe pure-omni-diff
.\omni.exe acceptance-diff
.\omni.exe inspect spec/acceptance/goldens/decision_hook_golden.omni
.\omni.exe --explain TAINT_SINK
```

### 3. Run the Version-Locked Model Harness (`omni-harness`) & Agent Driver (`omni-agent-driver`)
```powershell
# Inspect the version-locked grammar, capability, and error-code contract:
cargo run -p omni-cli --bin omni-harness -- --spec

# Execute a strict single-form .omni program and receive single-line JSON:
cargo run -p omni-cli --bin omni-harness -- --strict "fn main(a: i64) -> i64 { let b: i64 = a * 2; return b; }" 21

# Receive a structured machine-readable error code + non-null 1-based line/col + retry hint on failure:
cargo run -p omni-cli --bin omni-harness -- --strict --retry-hint "fn main(x: i64 @untrusted) -> i64 { let safe_a: i64 = 1; return x; }" 1

# Run the autonomous emit-and-repair agent driver on a broken candidate .omni program (or pass --repair-cmd "<model_cmd>"):
cargo run -p omni-cli --bin omni-agent-driver -- --strict "fn main(a: int) -> trit { let d = a - 10; let g = egate(d); return g; }" 25
```

### 4. Evaluate, Run, or Serve Live Full-Stack `.omni` Programs (`omni serve`)
```powershell
cargo run -p omni-cli -- eval "#[cap(AI)] fn main(a: i64, b: i64) -> trit { let g = egate(a - b); return g; }" 30 10
cargo run -p omni-cli -- eval "route GET /users/:id (req: i64) @cap(net, db_r) { let id = req.param(\"id\"); let row = db_query(\"SELECT * FROM users WHERE id = ?\", id); respond(200, row); }" 42
cargo run -p omni-cli -- serve app.omni 127.0.0.1:8080 --max-requests 10 --db omni_host.kv
cargo run -p omni-cli -- repl
```
