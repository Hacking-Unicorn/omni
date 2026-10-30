# Omni Bootstrap & Surface Language Progress Checklist (`OMNI_PROGRESS_CHECKLIST.md`)

## Phase 1: Core Bootstrap & Fixed-Point Verification (Complete)
- [x] **1. Re-prove the Stage-1 fixed point outside the selftest**:
  - Built `omni_self_compiler.omni_bc` (`19,731` `u16` tokens = `39,462` bytes, `FNV-1a64 = 0x677F57AE71D28FED`).
  - Emitted `omni_stage1_self_compiler.elf` (`106,242` bytes, `FNV-1a64 = 0xEAFC81F46B482FF8`).
  - Executed `./omni_stage1_self_compiler.elf omni_self_compiler.omni_bc > omni_stage2_self_compiler.elf` (`106,242` bytes, `FNV-1a64 = 0xEAFC81F46B482FF8`, `0` byte differences).
- [x] **2. Rebuild the workspace from the current monolith and prove equivalence**:
  - Extracted and synchronized all 8 Cargo crates (`omni-isa`, `omni-ir`, `omni-verifier`, `omni-vm`, `omni-assembler`, `omni-lang`, `omni-aot`, `omni-cli`).
  - Verified `cargo build --workspace` and `cargo run -p omni-cli -- selftest` pass with 0 errors and 0 warnings.
- [x] **3. Make the extracted verifier a complete clone**:
  - Full parity on capability bitmask, linear ownership/borrow state machine, arena leak detection, SSA monotonic allocation, branch target resolution, and taint/sanitizer rules (`assert_verifier_clone_eq`).
- [x] **4. Make the extracted VM a complete clone for the supported subset**:
  - Full parity on control flow (`LABEL`, `JMP`, `BR_POS`, `BR_NEG`, `BR_TRIT`), byte buffers (`ANEW`, `IGET`, `ISET`, `SLEN`), ternary decision hooks (`EGATE`, `TLOGIC`), inter-function calls (`CALL`), and fuel/memory budgets (`DEFAULT_VM_MEMORY_BUDGET_BYTES = 256 MiB`).
- [x] **5. Put Wasm under the same differential for every golden that the `x86-64` path covers**:
  - `compile_pure_u16_to_wasm` and `execute_wasm_module_i64` verified across all 31 `x86-64` goldens.
- [x] **6. Add an explicit "not supported" list in `SPEC.md`**:
  - Documented all VM-only runtime opcodes rejected by standalone AOT in `SPEC.md` Section 6.
- [x] **7. Freeze the contract in `SPEC.md`**:
  - Updated `SPEC.md` so every behavior asserted by the selftest is written as a normative rule.

## Phase 2: Minimal Omni Surface Language (`omni-lang`) Foundation (Complete)
- [x] **Lexer, Parser & Lowering Pass (`compile_omni_surface_to_asm` / `compile_omni_surface_to_u16`)**:
  - Lowers human-readable `.omni` syntax (`fn`, `let`, reassignment, arithmetic/bitwise/ternary expressions, `if_pos`/`if_neg`, `match_trit`, `do_while`/`while`, byte buffers) to canonical `.omni_asm` and `Vec<u16>` with zero new opcodes.
  - Verified bit-exact `Vec<u16>` identity across all 15 `SPEC.md` goldens (`seed0_risk` through `byte_buf_golden`).

## Phase 3: Surface Language Properties & Usability (`omni-lang` v2 — Complete)
- [x] **1. Surface-Level Taint & Capability Rejection**:
  - Added `#[cap(AI, DB_W, ...)]` function capability attributes and `x: <ty> @untrusted` parameter taint annotations, plus surface sanitizers (`san_sql`, `san_dom`, `san_prm`).
  - Added 4 selftest checks proving `verify_binary_function` rejects untrusted return (`bad_untr_ret`), untrusted operand at a safe decision sink (`bad_untr_sink`), and missing capability (`bad_missing_cap`), while accepting sanitized flows (`good_san`).
- [x] **2. One Decision Hook (`egate` / `tlogic`)**:
  - Wired `0x7F35 (OP_7F35_EGATE)` (`egate(x)` / `decision_hook(x)`) and `0x7F3F (OP_7F3F_TLOGIC)` (`tlogic(x)`) from `.omni` surface syntax through the assembler and `OmniBinaryVm`.
  - Added `decision_hook_golden` verifying `+1`, `-1`, and `0` ternary decisions in both the monolith and workspace differential selftests.
- [x] **3. `@PACK5` Lowering for Surface Trit Values**:
  - Lowered surface trit vector literals `[+1t, -1t, 0t, +1t, +1t, ...]` through compile-time `pack5_encode` (`ANEW 0` + `ISET <packed_bytes>` + `UNPACK5 <len>` and `pack5_bytes([...])`) instead of expanding to `i64`.
  - Added `pack5_trit_vec_golden` proving 10 trits pack into `2` bytes (`[227, 219]`) vs `80` bytes for 10 `i64`s (`40x` denser byte buffer), with `tdot(v, v) == 8` and `egate(v) == +1` (`2455`).
- [x] **4. Error Messages with Line & Column**:
  - Added 1-based `line <L>, col <C>: ...` source coordinates to all surface tokens, AST nodes, parameters, and label references.
  - Added 6 diagnostic checks verifying exact `line/col` reporting on undefined variables, unknown intrinsics, bad types, immutable parameter reassignment, unresolved `goto` labels, and missing `match_trit` arms.
- [x] **5. Multi-Function `.omni` Programs**:
  - Added multi-function module parsing and lowering (`FN_DEF <id>`) with inter-function calls lowered to `0x7F48 (OP_7F48_CALL)` (`TOK_SYM_FN`).
  - Added multi-function golden (`weighted_term` + `main` -> `80`).
- [x] **6. Module / File Includes (`include "other.omni";`)**:
  - Added `resolve_omni_surface_includes`, `compile_omni_surface_file_to_u16`, and `link_omni_surface_modules` with relative path resolution and circular-include detection (`0` new opcodes).
  - Added positive multi-file include golden (`trit_helper.omni` + `main` -> `30`) and circular include rejection check.
- [x] **7. Single-File Runner, Evaluator & REPL**:
  - Upgraded `omni run <file.omni> [args...]` (and `cargo run -p omni-cli -- run <file.omni> [args...]`), plus `eval` and `repl` subcommands, to resolve includes, compile, verify all functions in the module, and run in one step.

## Phase 4: Audit Fixes, Surface Arenas, `@ENG` String Literals & Direct Surface-to-AOT (Complete)
- [x] **1. Audit & Cross-Platform Hardening**:
  - Eliminated all 7 compiler warnings across `omni_rust_bootstrap.rs`, `omni_rust_bootstrap_verified.rs`, and the 8-crate Cargo workspace (`0` warnings, `0` errors).
  - Replaced hardcoded `/tmp/...` paths in `run_unit_and_differential_selftests` with `std::env::temp_dir().join(...)` for full Windows/Linux/macOS portability.
  - Added unconditional `OmniBinaryVm` assertion coverage across all platforms for `1d-2` (`divzero`), `1f` (`IMOD0`), and `1i` (`Nested-branch golden`).
  - Reconciled `SPEC.md` Section 1 token partition hex ranges (`0x7DFD..=0x7EFE`) with the normative `omni-isa` constants.
- [x] **2. Surface Linear Ownership & Arena Verification**:
  - Exposed linear memory/arena intrinsics (`arena(cap)`, `borrow_r(x)`, `borrow_w(x)`, `move_val(x)`, `drop(x)`) in `omni-lang` lowering to existing opcodes `0x7F01..=0x7F05`.
  - Added selftests (`13g` / `6h`) verifying `verify_binary_function` rejects use-after-move (`Use of Moved register`) and unreleased arenas (`Unclosed Arena(s) at function exit`) while accepting properly dropped arenas (`Int(42)`).
- [x] **3. Surface `@ENG` String Literals, String Intrinsics & Taint Tracking**:
  - Lowered `"..."` surface string literals to `CSTR@UNTRUSTED @str:<hex>` (`0x7F08` + `TOK_ENG_SPAN 0x7EEA`) and exposed `s2i` (`0x7F55`), `i2s` (`0x7F56`), and `scat`/`sconc` (`0x7F51`).
  - Added selftests (`13h` / `6i`) proving untrusted string literals are rejected at unsanitized `RET` or `egate` sinks until sanitized via `san_sql`/`san_dom`/`san_prm`.
- [x] **4. Compound Assignments, Inclusive Comparisons & Snippet Wrapper**:
  - Added compound assignment statements (`+=`, `-=`, `*=`) and inclusive relational operators (`<=` -> `TNOT(CGT)`, `>=` -> `TNOT(CLT)`).
  - Unified `wrap_omni_surface_snippet` in `omni-lang` so multi-statement snippets and bare expressions work seamlessly in `eval` and `repl`.
- [x] **5. Direct `.omni` Surface-to-AOT (`x86-64` ELF64 & WebAssembly) Pipeline**:
  - Added `compile_omni_surface_to_x86_64_elf`, `compile_omni_surface_to_wasm`, and `execute_omni_surface_wasm_i64` in `omni-lang` and the monolith, plus `compile-elf`, `compile-wasm`, and `run-wasm` CLI subcommands (`186` monolith checks / `145` workspace differential checks passing).

## Phase 5: Direct `@PACK5` Operations, AOT-First Hot Paths, Explicit Budgets & Zero Long-Lived IR (Complete)
- [x] **1. `@PACK5` for Trit Data Instead of 64-Bit Integers (`5 trits per byte`)**:
  - Added zero-allocation `@PACK5` byte primitives (`pack5_byte_len`, `pack5_get_trit`, `pack5_set_trit`, `pack5_tdot_direct`, `pack5_sum_direct`, `pack5_gate_direct`) in `omni-isa` and wired `OmniBinaryVm` (`IGET`, `TDOT`, `EGATE`, `TLOGIC`, `TV_SRCH`) directly to `@PACK5` bytes.
  - Lowered `0x7F01 (ARENA)`, `0x7F05 (DROP)`, `0x7F30 (PACK5)`, `0x7F31 (UNPACK5)`, `0x7F34 (TDOT)`, `0x7F35 (EGATE)`, `0x7F3F (TLOGIC)`, and `@PACK5` `0x7F0E (IGET)` in both `compile_pure_u16_to_x86_64_elf_with_args` and `compile_pure_u16_to_wasm`, plus surface intrinsics `pack5_tdot`, `pack5_gate`, `pack5_get`, and `pack5_len`.
- [x] **2. AOT / Stage-1 for Hot Numeric Paths Instead of the Interpreter**:
  - Added `is_module_aot_hot_path` and `execute_omni_surface_aot_first` so `execute_omni_surface_program` (`run`, `eval`, `repl`) automatically dispatches hot numeric, ternary, `@PACK5`, control-flow, and arena functions through the 64-bit AOT path (`backend == "aot"`).
  - Added `compile_omni_surface_via_stage1_to_elf` and the `compile-stage1` CLI subcommand to lower `.omni` programs through `omni_self_compiler.omni_bc` into bit-exact `x86-64` ELF64 binaries without host codegen.
- [x] **3. Explicit Step & Memory Budgets (`#[step_budget(N)]` / `#[mem_budget(N)]`)**:
  - Enforced `OmniBinaryVm::charge_memory` across all allocating opcodes (`ARENA`, `CSTR`, `ANEW`, `ISET`, `PACK5`, `UNPACK5`, `SCONC`, `I2S`, `DB_INS`, `DOM_NODE`, `EMBED`), call-depth cap `MAX_VM_CALL_DEPTH = 64`, and `execute_wasm_module_i64_with_budgets` in AOT.
  - Added `#[step_budget(N)]` / `#[fuel(N)]` and `#[mem_budget(N)]` / `#[memory_budget(N)]` surface function attributes and `execute_omni_surface_program_with_budgets`.
- [x] **4. No Long-Lived Intermediate IR & Deep Audit Hardening**:
  - Refactored `compile_omni_surface_to_u16` (`SurfaceLowerCtx.packets`) to emit binary `u16` instruction packets directly from `SurfaceFnDef` and patch `IfBranch` join targets directly in binary without retaining `ctx.lines` or building an `.omni_asm` string, and added direct `verify_u16_stream(&[u16])`.
  - Removed the 35-trit AOT `@PACK5` limit so arbitrary-length `@PACK5` vectors (`> 35` trits, e.g., 50 trits) and dynamic-index `iget(v, i)` loops execute natively in both `x86-64` ELF64 AOT and 64-bit WebAssembly AOT (`pack5_50_dynamic_golden(50) == 4101`).
  - Added strict `@PACK5` sentinel/corrupt byte (`243..=255`) rejection (`validate_func_pack5_ops` in AOT + `Err("PACK5_SENTINEL_OR_CORRUPT_BYTE")` propagation in `OmniBinaryVm`) and full multi-function verification across all execution entrypoints (`196` monolith checks / `157` workspace differential checks passing with `0` warnings).
