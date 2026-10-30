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

Verified coverage (`196` non-vacuous checks = `148` core bootstrap checks + `48` `omni-lang` surface language, Phase 5 & deep-audit checks, `0` compiler warnings):
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

Verified workspace build & differential results (`157` checks on Windows, `158` checks on Linux `x86-64`, `0` compiler warnings):
- **8-Crate Workspace (`omni-isa`, `omni-ir`, `omni-verifier`, `omni-vm`, `omni-assembler`, `omni-lang`, `omni-aot`, `omni-cli`)**: Compiles cleanly with zero external dependencies and runs `assert_verifier_clone_eq`, `assert_golden_monolith_vs_workspace`, all 64 corpus samples, and all 50 surface-language, Phase 5 & deep-audit differential checks against `mod monolith`.

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

Verified surface language properties & usability checks (`48` checks in Section `13` / `50` checks in `omni-cli`, `0` new opcodes):

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
