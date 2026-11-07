# Omni — Re-Proven Status Note (`OMNI_FULL_STATUS_AUDIT.md`)

This one-page status note lists only what was re-proven with reproducible commands and verified outputs across the monolith (`omni_rust_bootstrap.rs`), the Stage-1/Stage-2 self-hosting compiler binaries, the 8-crate Cargo workspace (`omni-workspace.tar.gz`), and the usable Omni surface language (`omni-lang`).

## 1. Stage-1 == Stage-2 Fixed Point Outside Selftest (Item 1)

Reproducible command sequence executed on live `x86-64` Linux hardware:

```bash
rustc -O omni_rust_bootstrap.rs -o omni
./omni fixed-point
./omni_stage1_self_compiler.elf omni_self_compiler.omni_bc > omni_stage2_self_compiler.elf
cmp -l omni_stage1_self_compiler.elf omni_stage2_self_compiler.elf
diff -s omni_stage1_self_compiler.elf omni_stage2_self_compiler.elf
```

Verified outputs & integrity hashes:
- `cmp -l omni_stage1_self_compiler.elf omni_stage2_self_compiler.elf`: empty output (`exit code = 0`, `0` byte differences).
- `diff -s omni_stage1_self_compiler.elf omni_stage2_self_compiler.elf`: `Files omni_stage1_self_compiler.elf and omni_stage2_self_compiler.elf are identical`.
- Recorded artifact sizes, `FNV-1a64`, and `SHA-256` digests:

| Artifact | Size (Bytes) | `FNV-1a64` | `SHA-256` |
|----------|--------------|------------|-----------|
| `omni_self_compiler.omni_bc` | `39,462` (`19,731` `u16` tokens) | `0x677F57AE71D28FED` | `5f62934a6ebef01790aa8a4115355c35e2d41e77702efca4c876254628613f25` |
| `omni_stage1_self_compiler.elf` | `106,242` | `0xEAFC81F46B482FF8` | `4f363619517736db9da07d475072f90996df6bc10d8b51f115a5ae571fa6becb` |
| `omni_stage2_self_compiler.elf` | `106,242` | `0xEAFC81F46B482FF8` | `4f363619517736db9da07d475072f90996df6bc10d8b51f115a5ae571fa6becb` |

## 2. Monolith Selftest & Full Wasm Differential (Items 5 & 6)

Reproducible command:

```bash
rustc -O omni_rust_bootstrap.rs -o omni && ./omni selftest
```

Verified coverage (`351` non-vacuous checks = `148` core bootstrap checks + `203` `omni-lang` surface language, Phase 5, deep-audit, Phase 6, Phase 7 & Phases A–E Pure-Omni self-hosting & core codebase checks, `0` compiler warnings):
- **Full Golden Differential (`OmniBinaryVm` == `Stage-1` == `x86-64` ELF == `execute_wasm_module_i64`)**: `compile_pure_u16_to_wasm` and `execute_wasm_module_i64` lower `LABEL (0x7F40)`, `JMP (0x7F41)`, `BR_TRIT (0x7F42)`, `BR_POS (0x7F43)`, `BR_NEG (0x7F44)`, `ANEW (0x7F0D)`, `IGET (0x7F0E)`, `ISET (0x7F0F)`, `SLEN (0x7F50)`, `ARENA (0x7F01)`, `DROP (0x7F05)`, `PACK5 (0x7F30)`, `UNPACK5 (0x7F31)`, `TDOT (0x7F34)`, `EGATE (0x7F35)`, and `TLOGIC (0x7F3F)` in addition to all numeric/bitwise/ternary opcodes. Every golden covered by the `x86-64` path is verified through `execute_wasm_module_i64` and `OmniBinaryVm` on both Windows and Linux.
- **Explicit Standalone AOT Rejection (Check 6)**: Both `compile_pure_u16_to_x86_64_elf` and `compile_pure_u16_to_wasm` reject high-level VM-only runtime seeds (`1, 2, 3, 5, 7`) with `Err(...)`.

## 3. Multi-Crate Workspace Build & In-Process Monolith Differential (Items 2, 3 & 4)

Reproducible commands (Windows PowerShell & Linux/macOS Bash):

```powershell
# Windows PowerShell
tar -xzf omni-workspace.tar.gz
cd omni-workspace
$env:CARGO_INCREMENTAL=0; cargo build --workspace
cargo run -p omni-cli -- selftest
```

```bash
# Linux / macOS Bash
tar -xzf omni-workspace.tar.gz
cd omni-workspace
CARGO_INCREMENTAL=0 cargo build --workspace
cargo run -p omni-cli -- selftest
```

Verified workspace build & differential results (`312` checks on Windows, `313` checks on Linux `x86-64`, `0` compiler warnings):
- **8-Crate Workspace (`omni-isa`, `omni-ir`, `omni-verifier`, `omni-vm`, `omni-assembler`, `omni-lang`, `omni-aot`, `omni-cli`)**: Compiles cleanly with zero external dependencies and runs `assert_verifier_clone_eq`, `assert_golden_monolith_vs_workspace`, all 64 corpus samples, and all 205 surface-language, Phase 5, deep-audit, Phase 6, Phase 7 & Phases A–E Pure-Omni self-hosting differential checks against `mod monolith`.

## 4. Usable Omni Surface Language (`omni-lang`) — Properties & Goldens

Reproducible CLI commands (single-file runner, evaluator, REPL, and AOT/Stage-1 compiler):

```bash
# Single-step compile + verify + AOT-first run of a .omni file:
cargo run -p omni-cli -- run program.omni 30 15
# Evaluate an inline .omni program or bare expression directly (AOT-first):
cargo run -p omni-cli -- eval "#[cap(AI)] fn main(a: i64, b: i64) -> trit { let g = egate(a - b); return g; }" 30 10
cargo run -p omni-cli -- eval "egate(30 - 10)"
# Interactive REPL:
cargo run -p omni-cli -- repl
# Inspect lowered .omni_asm or emit binary .omni_bc / .elf / Stage-1 .elf / .wasm:
cargo run -p omni-cli -- surface-to-asm program.omni
cargo run -p omni-cli -- compile-surface program.omni program.omni_bc
cargo run -p omni-cli -- compile-elf program.omni program.elf
cargo run -p omni-cli -- compile-stage1 program.omni program_stage1.elf
cargo run -p omni-cli -- compile-wasm program.omni program.wasm
cargo run -p omni-cli -- run-wasm program.omni 30 15
```

Verified surface language properties, full-stack host ABI, live TCP HTTP/1.1 + file-backed DB/KV server & version-locked harness/agent-driver checks (`105` checks in Section `13` / `107` checks in `omni-cli`, `0` new opcodes):

| Surface Property / Feature | Lowering Mechanism (Existing Opcodes Only) | Verified Golden / Rejection Proof |
|----------------------------|--------------------------------------------|-----------------------------------|
| **1. Surface Taint & Capability Rejection** | `x: <ty> @untrusted` -> `PARAM $i <TY> UNTRUSTED`; `#[cap(AI)]` -> `CAP_MASK`; `san_sql`/`san_dom`/`san_prm` -> `0x7F57..0x7F59` | `bad_untr_ret` (`Taint violation on RET`), `bad_untr_sink` (`Sink opcode 0x7F35 received @UNTRUSTED`), and `bad_missing_cap` (`Capability violation on opcode 0x7F35`) rejected by `verify_binary_function`; `good_san` passes |
| **2. Decision Hook (`egate` / `tlogic`)** | `egate(x)` -> `0x7F35 (OP_7F35_EGATE)`; `tlogic(x)` -> `0x7F3F (OP_7F3F_TLOGIC)` | `decision_hook_golden(30, 10) == +1`, `decision_hook_golden(5, 20) == -1`, `decision_hook_golden(12, 12) == 0` |
| **3. `@PACK5` Lowering for Trit Vectors** | `[+1t, -1t, 0t, ...]` packed at compile time via `pack5_encode` -> `ANEW 0` (`0x7F0D`) + `ISET` (`0x7F0F`) + `UNPACK5` (`0x7F31`) | `pack5_trit_vec_golden`: 10 trits pack into `2` bytes (`[227, 219]`) vs `80` bytes for 10 `i64`s (`40x` denser), `tdot(v, v) == 8`, `egate(v) == +1` -> returns `2455` |
| **4. Line & Column Error Messages** | 1-based `(line, col)` spans on all tokens, AST nodes, and label references | Reports `line 2, col 13: Undefined variable 'missing_y'`, `line 1, col 11: Unknown surface type 'float128'`, `line 2, col 5: Cannot reassign immutable function parameter 'x'`, `line 2, col 10: Unresolved branch target label 9`, and `line 2, col 13: match_trit missing 'pos' arm` |
| **5. Multi-Function `.omni` Programs** | Multiple `fn` definitions -> `FN_DEF <id>` blocks; inter-function calls -> `0x7F48 (OP_7F48_CALL)` (`TOK_SYM_FN`) | `weighted_term(x, w)` + `main(10, 20)` -> verifies both functions and returns `Int(80)` |
| **6. Module / File `include` Linker** | `include "helper.omni";` / `mod "helper.omni";` preprocessor + circular-include detector | Links `trit_helper.omni` (`score_to_trit`) into `main(75)` -> returns `Int(30)`; rejects circular `include "loop.omni";` with `line/col` error |
| **7. Single-File Runner, Eval & REPL** | `omni run file.omni args...`, `omni eval "..." args...`, `omni repl` (`wrap_omni_surface_snippet`) | Compiles `.omni` -> `Vec<u16>`, verifies all functions via `verify_binary_function`, and executes via AOT-first dispatcher without manual intermediate steps |
| **8. All 15 Original Surface Goldens** | `seed0_risk` through `byte_buf_golden` | 100% bit-exact `Vec<u16>` match (`0` byte diff) against `assemble_omni_text_to_u16` |
| **9. Pre-Conditioned `while` + `<=`/`>=` + `+=`/`-=`/`*=` + Direct Surface-to-AOT** | `<=` -> `TNOT(CGT)`, `>=` -> `TNOT(CLT)`, `+=` -> `IADD`; `compile_omni_surface_to_x86_64_elf` & `compile_omni_surface_to_wasm` | `while_sum_even_golden(10) == 30` verified across `OmniBinaryVm`, Stage-1 Self-Compiler, `x86-64` ELF AOT (`0` byte diff), and 64-bit WebAssembly |
| **10. Surface Linear Memory Arena & Borrow Checker** | `arena(cap)` (`0x7F01`), `borrow_r(a)` (`0x7F02`), `borrow_w(a)` (`0x7F03`), `move_val(a)` (`0x7F04`), `drop(a)` (`0x7F05`) | `arena_lifecycle_golden(8) == 50` passes; `bad_surface_arena_leak` (`Linear memory leak`) and `bad_surface_borrow_conflict` (`active readers`) rejected by `verify_binary_function` |
| **11. Surface String Literals (`@ENG` Taint), Sanitizers & String Ops** | `"..."` -> `CSTR@UNTRUSTED @str:<hex>` (`0x7F08` + `0x7EEA`); `scat` (`0x7F51`), `s2i` (`0x7F55`), `i2s` (`0x7F56`), `fnv1a` (`0x7F5D`) | `bad_unsanitized_str_lit` rejected (`Taint violation on RET`); `str_pipeline_golden(45) == 130` passes verification and executes in `OmniBinaryVm` |
| **12. Direct `@PACK5` Operations & `@PACK5` / `EGATE` AOT (`x86-64` ELF64 + Wasm)** | `pack5_tdot_direct`, `pack5_sum_direct`, `pack5_gate_direct`, `pack5_get_trit`, `pack5_set_trit`; AOT lowering of `0x7F30..=0x7F3F` | `pack5_direct_aot` (`2422`) and `pack5_direct_golden` (`281`) execute identically across `x86-64` ELF64 AOT, 64-bit WebAssembly AOT, and `OmniBinaryVm` |
| **13. AOT-First Hot Numeric Dispatcher & Stage-1 `.omni` -> ELF64** | `execute_omni_surface_aot_first` (`is_module_aot_hot_path`) + `compile_omni_surface_via_stage1_to_elf` (`compile-stage1`) | Dispatches hot numeric/ternary/`@PACK5`/arena paths to AOT (`backend == "aot"`); Stage-1 ELF matches direct `x86-64` ELF64 bit-for-bit (`0` byte diff) |
| **14. Explicit Step & Memory Budgets (`#[step_budget(N)]`, `#[mem_budget(N)]`)** | `OmniBinaryVm::charge_memory` across all allocating opcodes + `execute_wasm_module_i64_with_budgets` | Deterministically halts runaway loops (`#[step_budget(40)]` / `#[step_budget(150)]`) and oversized allocations (`#[mem_budget(16)]` / `#[mem_budget(256)]` / `#[mem_budget(512)]`) in both AOT and VM |
| **15. Zero Long-Lived Intermediate IR (`compile_omni_surface_to_u16`)** | Direct AST -> `SurfaceLowerCtx.packets: Vec<Vec<u16>>` -> `pack_fn_module` + direct `verify_u16_stream(&[u16])` | Emits `Vec<u16>` packets on the fly with zero intermediate `.omni_asm` string retention; 100% bit-identical to `assemble_omni_text_to_u16` across all 15 goldens and multi-function modules |
| **16. Unbounded `@PACK5` Vectors (`> 35` Trits), Dynamic `IGET` & Corrupt Sentinel (`243..=255`) Rejection** | Direct byte-lane `@PACK5` extraction + branchless dynamic `IGET` in AOT (`x86-64` & Wasm) + `validate_func_pack5_ops` | `pack5_50_dynamic_golden(50) == 4101` (50 trits = 10 packed bytes + dynamic loop `iget(v, idx)`) matches across `x86-64` ELF64 AOT, 64-bit WebAssembly AOT, and `OmniBinaryVm`; corrupt byte `250 > 242` and uncalled tainted helpers are deterministically rejected across AOT and VM |
| **17. Golden Size Regression Gate (`13p` / `6q`)** | Asserts locked `u16` token counts `[53, 34, 34, 91, 40, 20, 22, 22, 49, 49, 51, 74, 69, 57, 86]` across all 15 `SPEC.md` goldens | Fails selftest if any golden grows by even `1` `u16` token; verifies zero-cost `: <ty>` on `let` (`0` extra tokens) |
| **18. Stricter AI-First Single-Form Grammar (`#[strict]` / `--strict`, `13q` / `6r`)** | `compile_omni_surface_strict_to_u16` enforcing `: <ty>` on every `let`, canonical `+1t`/`0t`/`-1t`, `@untrusted`, and `include "..."` | Accepts canonical typed programs and `let v: trit_vec = [...]` (`@PACK5` `2` B vs `80` B for 10 trits, `10` B vs `400` B for 50 trits); rejects untyped `let` (`BAD_TYPE`), `TRIT_POS`, `untrusted_i64`, `int`, and `mod "..."` (`AMBIGUOUS_GRAMMAR`) |
| **19. Machine-Readable Error Codes & Non-Null 1-Based Spans (`error[<CODE>]`, `13r` / `6s`)** | `compile_omni_surface_to_u16_with_sourcemap` (`SurfaceSourceMap`) + `annotate_omni_error_with_sourcemap` + `parse_omni_structured_error` | Verifies all 16 codes with non-null 1-based `(line, col)` spans: `TAINT_ON_RET` (`3, 5`), `TAINT_SINK` (`2, 13`), `MISSING_CAP` (`2, 13`), `LINEAR_LEAK` (`3, 5`), `BORROW_CONFLICT` (`4, 14`), `UNDEF_VAR` (`2, 13`), `IMMUT_PARAM` (`2, 5`), `BAD_TYPE` (`1, 11`), `UNKNOWN_FN` (`2, 13`), `ARITY_MISMATCH` (`2, 13`), `UNRESOLVED_LABEL` (`2, 10`), `CIRCULAR_INCLUDE` (`1, 1`), `BUDGET_EXCEEDED` (`1, 1`), `CORRUPT_PACK5`, `AMBIGUOUS_GRAMMAR`, `SYNTAX_ERROR` |
| **20. Full-Stack `route` + `req.param` + DB/KV Host ABI (`13s` / `6t`)** | `route GET /users/:id` -> `0x7F6C ROUTE`, `req.param` -> `0x7F6D REQ_PAR`, `db_get`/`db_query` -> `0x7F64`/`0x7F65`, `kv_put` -> `0x7F6B`, `auth_verify` -> `0x7F70`, `respond` -> `0x7F6E` | `route GET /users/:id` with `kv_put`/`db_get` returns `Int(200_000_777)` and `db_query` returns `Int(200_000_420)`; missing `@cap(net)` / `@cap(db_r)` / `@cap(db_w)` rejected with `MISSING_CAP`; unsanitized `@untrusted` query parameter rejected with `TAINT_SINK` until sanitized via `auth_verify` |
| **21. Version-Locked Model Harness (`omni-harness`, `13t` / `6u`)** | `OMNI_HARNESS_CONTRACT_VERSION = "omni-harness-v1.0.0"`, `run_omni_harness_json`, `omni_harness_spec_json`, and `omni-harness` binary | Single-line JSON output verified for `"ok":true` (`"backend":"aot"` and `"backend":"vm"`), `"ok":false` (`TAINT_ON_RET` `"line":3,"col":5`, `MISSING_CAP` `"line":2,"col":13`, `BUDGET_EXCEEDED` `"line":1,"col":1`, `BAD_TYPE` `"line":1,"col":26` with `--retry-hint`), and `--spec` |
| **22. Autonomous Agent Emit-and-Repair Driver (`omni-agent-driver`, `13u` / `6v`)** | `run_omni_agent_repair_loop_with_model`, `run_omni_agent_repair_loop`, `run_omni_agent_driver_json`, and `omni-agent-driver` binary | Verifies version lock (`VERSION_MISMATCH` on wrong version) and 6 multi-step repair loops (`MISSING_CAP`, `TAINT_ON_RET`, `TAINT_SINK`, `BAD_TYPE` + `AMBIGUOUS_GRAMMAR` + `MISSING_CAP`, `LINEAR_LEAK`, `BUDGET_EXCEEDED`) repairing broken `.omni` candidates to `"ok":true` |
| **23. Live Native TCP HTTP/1.1 + DB/KV Server (`omni serve`, `13v` / `6w`)** | `serve_omni_http_batch`, `serve_omni_http_once`, `handle_omni_http_request`, and `omni serve` over `std::net::TcpListener` | Rejects missing `@cap(net, db_r)` before binding with `MISSING_CAP`; binds `127.0.0.1:0` and serves live TCP `POST /users/42` (`HTTP/1.1 201 Created`, `{"status":201,"data":777}`) and `GET /users/42` (`HTTP/1.1 200 OK`, `{"status":200,"data":777}`) against persistent host KV/DB store |

| **24. Executable Acceptance Suite & Error Contract (`13w` / `6x`)** | `spec/acceptance/acceptance_manifest.json` (`24` positive + `16` negative goldens), `spec/error_contract.md`, `run_error_contract_selftest`, `run_omni_acceptance_diff` (`omni acceptance-diff`) | Verifies all 16 `error[<CODE>]` contracts and runs continuous differential verification across all positive and negative acceptance goldens (`55` checks) |
| **25. Narrow Capability-Gated Host Interface (`13x` / `6y`)** | `spec/host_interface.md`, `OmniNarrowHost` (10 syscalls), `StdNarrowHostAdapter`, `SimulatedNarrowHostAdapter`, `serve_omni_http_via_narrow_host` | Rejects unprivileged TCP/File/Time calls with `MISSING_CAP`; verifies full HTTP/1.1 `POST` + `GET` + persistent KV/DB roundtrip through `SimulatedNarrowHostAdapter` and `StdNarrowHostAdapter` (`4` checks) |
| **26. Deepened Multi-Step AI Repair Loop (`13y` / `6z`)** | `omni-repair-strategies-v1.1.0` (`16` strategies), `list_omni_repair_strategies` (`--list-strategies`), `repair_omni_include_graph` | Repairs multi-function callee cap + arity mismatches, multi-file circular include graphs, unresolved `goto` labels + untyped strict `let`, undefined vars + unknown intrinsics, and memory budgets, recording `strategy_id` and `strategy_version` in JSON (`8` checks) |
| **27. Binary Stream Inspector & Diagnostic Explainer (`13z` / `6AA`)** | `format_omni_inspect` (`omni inspect` / `omni disasm`), `format_omni_human_diagnostic`, `explain_omni_error_code` (`--explain <CODE>`) | Verifies deterministic disassembly with source map spans (`// line 4, col 19`), capability/taint tags, and human multi-line error rendering (`3` checks) |
| **28. Hygienic Surface Macros & Pure-Omni Stdlib Prelude (`13AA` / `6AB`)** | `spec/next_abstraction.md`, `macro <name>(...) = <expr>;`, `include "std/prelude.omni";` (`lookup_omni_stdlib_module`) | Zero new opcodes; verifies 4-way equivalence across `OmniBinaryVm`, Stage-1 Self-Compiler, `x86-64` ELF64 AOT (`0` byte diff), and 64-bit WebAssembly AOT (`2` checks) |
| **29. Compositional Verification Invariants & Heterogeneous Design (`13AB` / `6AC`)** | `spec/trusted_core.md`, `spec/mechanized_proof_plan.md`, `spec/heterogeneous_design.md`, `run_compositional_invariant_selftests` | Verifies all 5 compositional soundness lemmas: branched taint non-interference, multi-arena conservation, capability monotonicity across all gated opcodes, AOT boundary equivalence, and macro taint/hygiene transparency (`5` checks) |
| **30. Phase 7 Pure-Omni Self-Hosted Verifier Kernel (`13AC` / `6AD`)** | `omni-pure-kernel-v1.0.0`, `PHASE7_PURE_OMNI_VERIFIER_KERNEL_SURFACE`, `build_phase7_pure_omni_verifier_kernel_stream`, `verify_module_via_phase7_pure_omni_kernel` | Verifies 4-way equivalence (`OmniBinaryVm` == Host `x86-64` ELF64 == Stage-1 `.omni_bc` ELF64 bit-exact fixed point == 64-bit Wasm) across all 6 verification states (`0`, `1001..=1005`), negative rejection (`TAINT_ON_RET`, `TAINT_SINK`, `MISSING_CAP`, `LINEAR_LEAK`, `BORROW_CONFLICT`), and pre-verified full-stack route + KV execution over `OmniNarrowHost` (`3` checks) |
| **31. Phase A Pure-Omni Surface Frontend (`13AD` / `6AE`)** | `omni-pure-frontend-v1.0.0`, `spec/pure_self_host/frontend.omni`, `compile_omni_surface_via_pure_omni_frontend_with_sourcemap` | Verifies 4-way equivalence on the frontend kernel, bit-identical `Vec<u16>` parity on positive goldens, and exact `(code, line, col)` parity on negative goldens (`3` checks) |
| **32. Phase B Pure-Omni Driver & Verifier Integration (`13AE` / `6AF`)** | `omni-pure-driver-v1.0.0`, `spec/pure_self_host/driver.omni`, `run_pure_omni_driver_json`, `omni pure-driver` | Verifies 4-way equivalence on the driver kernel and exact single-line JSON parity against `run_omni_harness_json` (`3` checks) |
| **33. Phase C Pure-Omni Host Runtime (`13AF` / `6AG`)** | `omni-pure-host-v1.0.0`, `spec/pure_self_host/host_runtime.omni`, `serve_omni_http_via_pure_omni_host` over `OmniNarrowHost` | Verifies 4-way equivalence on the host dispatch kernel, live HTTP/1.1 `POST` + `GET` + persistent KV/DB over `OmniNarrowHost`, and pre-listen `MISSING_CAP` / `TAINT_SINK` rejection (`3` checks) |
| **34. Phase D Self-Hosting Bootstrap Fixed Point (`13AG` / `6AH`)** | `omni-pure-selfhost-v1.0.0`, `spec/pure_self_host/omni_selfhost_compiler.omni`, `run_phase_d_pure_omni_selfhost_bootstrap` (`omni pure-bootstrap`) | Proves bit-exact Generation-1 == Generation-2 equality on both `.omni_bc` (`159` words, `0x23B8D17257CCA9C4`) and `.elf` (`824` bytes, `0xE221085777B8AC47`, `3` checks) |
| **35. Phase E Full Pure-Omni Differential Lock (`13AH` / `6AI`)** | `run_pure_omni_acceptance_diff` (`omni pure-omni-diff`), `omni-harness --pure-omni` | Verifies 100% differential parity across all positive/negative acceptance goldens, Generation-1 == Generation-2 self-host fixed point, and `omni-agent-driver` repair (`3` checks) |
| **36. Pure-Omni Core Codebase (`pure/*.omni`) & Standard Library (`std/*.omni`, `13AI` / `6AJ`)** | `pure/compiler.omni`, `pure/verifier.omni`, `pure/host_runtime.omni`, `pure/tooling.omni`, `pure/selfhost_suite.omni`, `std/prelude.omni`, `std/math.omni`, `std/trit.omni`, `std/http.omni`, `build_pure_omni_core_suite_stream`, `run_pure_omni_core_suite_selftests` | Compiles and verifies all 9 `std/*` + `pure/*` modules (`1,412` `u16` words, `FNV-1a64 = 0x187D5EDEDC2F4E37`) with `Gen-1 == Gen-2` bit-exact parity, routes `run`/`eval`/`repl`/`serve` through Pure-Omni by default, and executes `pure/selfhost_suite.omni` (`Int(200_000_438)`) and multi-module `std/*` (`Int(200_000_320)`, `3` checks) |
