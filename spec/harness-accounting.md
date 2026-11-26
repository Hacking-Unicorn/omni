# Omni Step 8b — Selftest Harness Accounting & Triage (1,823 Checks)

Machine-measured breakdown of `run_unit_and_differential_selftests()` in `omni_rust_bootstrap.rs` on `feat/step8-pure-harness` (post-PR #47 `6bb8ba0`). Every check in the `1,823`-check Rust selftest matrix has an explicit category and disposition before Step 9 deletes the Rust bootstrap. Nothing is silently dropped.

## Category Summary (Sums to 1,823)

| Category | Suites | Checks | Disposition |
|---|---:|---:|---|
| **PORTED** (Step 8) | 7 | 118 | Ported to Pure-Omni ELFs in Step 8 (`pure/harness/*.omni`), full-text `cmp` byte-identical |
| **PORT-NOW → PORTED** (Step 8b) | 5 | 29 | Ported to Pure-Omni ELFs in Step 8b (`pure/harness/extra_gates.omni`), full-text `cmp` byte-identical |
| **ORACLE** | 21 | 970 | Compares Pure-Omni against the Rust bootstrap/host mirror or tests Rust-only CLI (`serve`, `wc -l`); retires at Step 9 when referent is deleted |
| **WASM** | 2 | 14 | Wasmtime/Wasm backend checks (`m2_10_wasm_multi`, `p4_4_wasm_host`); retires at Step 9 per Linux x86-64-first scope |
| **PURE-LOGIC** (covered) | 22 | 692 | Tests Omni artifacts/properties directly; explicit surviving-anchor coverage mechanism documented below |
| **TOTAL** | **57** | **1,823** | `118 + 29 + 970 + 14 + 692 = 1,823` |

---

## 1. Complete Per-Suite Accounting Table

### A. Top-Level Suites in `run_unit_and_differential_selftests()` (989 checks)

| Suite | Checks | Category | Disposition / Surviving Anchor |
|---|---:|---|---|
| `core_inline_1_to_13o` | 200 | **PURE-LOGIC** | Covered by `audit-view` (16 goldens + B6), `b2-pure-verifier`, `m2-fixed-point`, `verify_step7_driver.sh` (mechanism §2.1) |
| `ai_first_surface` (26 sub-suites + 142 inline) | 360 | **PURE-LOGIC** | Covered by `m2-frozen`, `m2-fixed-point`, `m3-modular`, `m2-lint`, `b2-pure-verifier`, `audit-view` (mechanism §2.2) |
| `cross_backend_effect_golden` | 242 | **ORACLE** | Compares Rust VM vs Rust Wasm vs Rust x86-64 AOT across effect goldens; retires at Step 9 (native x86-64 path covered by `omnic_pure.elf run` + `verify_step7_driver.sh`) |
| `if_pos_join_slot_stage1` | 16 | **PURE-LOGIC** | Covered by `m2-fixed-point` + `audit-view` B6 + `omnic_pure.elf` self-compile fixed point (mechanism §2.3) |
| `join_live_dest` | 63 | **PURE-LOGIC** | Covered by `m2-fixed-point` + `audit-view` B6 + `omnic_pure.elf` self-compile fixed point (mechanism §2.3) |
| `host_kv_lockstep` | 9 | **ORACLE** | Compares `pure/host_kv.omni` against Rust `OmniBinaryVm` KV store mirror; retires at Step 9 |
| `m3_fe_lock` | 95 | **ORACLE** | Compares Rust frontend against `pure/m3/` frontend across 95 surface cases; retires at Step 9 |
| `m3_gate` | 4 | **ORACLE** | Locks Rust frontend vs `pure/m3/` gate invariants; retires at Step 9 |

*Subtotal A:* `200 + 360 + 242 + 16 + 63 + 9 + 95 + 4 = 989`.

### B. `run_m2_foundation_selftests()` Suites (50 suites, 834 checks)

| Suite (`m2_host/decode_diff.rs` / `lint.rs` / `audit_view.rs` / `proposer_loop.rs`) | Checks | Category | Disposition / Pure-Omni Module |
|---|---:|---|---|
| `m2_frozen_hashes` | 4 | **PORTED** | `pure/harness/core_gates.omni` (`omnic_pure.elf m2-frozen`, full-text 0-byte diff) |
| `m2_single_mode_byte_lock` | 2 | **ORACLE** | Locks Rust single-mode x86-64/Wasm emitters against `spec/m2/single_mode_lock.tsv`; retires at Step 9 |
| `m2_shape_census` | 1 | **PURE-LOGIC** | Covered by `m2-frozen` + `m2-fixed-point` (mechanism §2.4) |
| `m2_label_reference` | 9 | **PURE-LOGIC** | Covered by `m2-lint` + `m2-fixed-point` + `audit-view` (mechanism §2.5) |
| `m2_lint` | 51 | **PORTED** | `pure/harness/m2_lint.omni` (`omnic_pure.elf m2-lint`, full-text 0-byte diff TSV + 15/15 fixtures) |
| `m2_decode_diff` | 306 | **ORACLE** | Differential `pure/m2/decode.omni` vs Rust `decode_omni_token_stream`; retires at Step 9 |
| `m3_isa_encode_roundtrip` | 34 | **PURE-LOGIC** | Covered by `m3-modular` + `audit-view` round-trip + `omnic_pure.elf` self-compile (mechanism §2.6) |
| `m2_labels_table` | 8 | **PURE-LOGIC** | Covered by `m2-fixed-point` + `m3-modular` (mechanism §2.5) |
| `m2_08_multi_aot_precheck` | 8 | **ORACLE** | Rust multi-AOT precheck helper; retires at Step 9 |
| `m2_09_multi_aot` | 8 | **ORACLE** | Rust multi-function x86-64 AOT emitter tests; retires at Step 9 |
| `m2_11_arena_elf` | 4 | **PURE-LOGIC** | Covered by `m2-fixed-point` + `s5-refreeze` `PT_GNU_STACK` + `m3-modular` (mechanism §2.7) |
| `m2_13_stage1_anchor` | 4 | **PURE-LOGIC** | Covered by `m2-frozen` (`stage1_elf_bytes` `112180 B` `0x62F14B335B52BECC`) + `m2-fixed-point` (mechanism §2.8) |
| `m2_15_pure_multi_compiler` | 7 | **PURE-LOGIC** | Covered by `m2-fixed-point` + `verify_step7_driver.sh` `compile-elf` (mechanism §2.7) |
| `m2_16_fixed_point` | 3 | **PORTED** | `pure/harness/core_gates.omni` (`omnic_pure.elf m2-fixed-point`, full-text 0-byte diff) |
| `m3_06_lex_diff` | 47 | **ORACLE** | Differential `pure/m3/lex.omni` vs Rust lexer; retires at Step 9 |
| `m3_07_skim_diff` | 50 | **ORACLE** | Differential `pure/m3/skim.omni` vs Rust skimmer; retires at Step 9 |
| `m3_08_expr_diff` | 24 | **ORACLE** | Differential `pure/m3/expr.omni` vs Rust expr parser; retires at Step 9 |
| `m3_09_stmt_diff` | 14 | **ORACLE** | Differential `pure/m3/stmt.omni` vs Rust stmt parser; retires at Step 9 |
| `m3_10_lower_diff` | 25 | **ORACLE** | Differential `pure/m3/lower.omni` vs Rust lowerer; retires at Step 9 |
| `m2_10_wasm_multi` | 11 | **WASM** | Wasm multi-function emitter tests; retires at Step 9 |
| `m2_14_verify_diff` | 10 | **ORACLE** | Differential `pure/m2/verify/mod_check.omni` vs Rust verifier; retires at Step 9 |
| `m3_15_four_backend_parity` | 4 | **ORACLE** | Differential Rust VM vs Wasm vs x86-64 vs Pure-M2; retires at Step 9 |
| `p4_1_vm_diff` | 11 | **ORACLE** | Differential `pure/m4/` VM vs Rust `OmniBinaryVm`; retires at Step 9 |
| `p4_3_thin_shim` | 3 | **ORACLE** | Tests Rust `p4_shim/mod.rs`; retires at Step 9 |
| `p4_4_wasm_host` | 3 | **WASM** | Tests Wasmtime host execution; retires at Step 9 |
| `p5_2_zero_shim_sys` | 3 | **PORT-NOW → PORTED** | `pure/harness/extra_gates.omni` (`omnic_pure.elf p5-sys`, full-text 0-byte diff) |
| `p7_0_measure` | 10 | **PURE-LOGIC** | Covered by `proposer-loop` + `m2-fixed-point` + `verify_step7_driver.sh` (mechanism §2.9) |
| `p7_0a_canary` | 7 | **PURE-LOGIC** | Covered by `proposer-loop` Stage 2/3 ternary `+1t`/`0t`/`-1t` verdicts (mechanism §2.9) |
| `p7_1_self_heal` | 6 | **PURE-LOGIC** | Covered by `proposer-loop` Stage 1 CEGIS rejection + `m2-lint` (mechanism §2.9) |
| `api_stage1` | 5 | **PURE-LOGIC** | Covered by `m2-lint` + `m2-frozen` + `audit-view` (mechanism §2.10) |
| `api_stage2` | 5 | **PURE-LOGIC** | Covered by `m2-lint` + `b2-pure-verifier` (mechanism §2.10) |
| `api_stage2_emit` | 5 | **PURE-LOGIC** | Covered by `m2-lint` + `m2-fixed-point` (mechanism §2.10) |
| `api_stage2_5` | 5 | **PURE-LOGIC** | Covered by `m2-lint` + `b2-pure-verifier` (mechanism §2.10) |
| `p7_adapt` | 6 | **PURE-LOGIC** | Covered by `m2-lint` + `proposer-loop` (mechanism §2.9) |
| `p7_learn` | 6 | **PURE-LOGIC** | Covered by `m2-lint` + `proposer-loop` (mechanism §2.9) |
| `a1_pure_default` | 6 | **ORACLE** | Differential Pure-Omni default path vs `--rust` oracle; retires at Step 9 |
| `a2_pure_include` | 5 | **PURE-LOGIC** | Covered by `omnic_pure.elf` self-compile + `verify_step7_driver.sh` `compile` on multi-include programs (mechanism §2.11) |
| `a3_macro_quarantine` | 4 | **PORT-NOW → PORTED** | `pure/harness/extra_gates.omni` (`omnic_pure.elf a3-macro-quarantine`, full-text 0-byte diff) |
| `b2_pure_verifier` | 16 | **PORTED** | `pure/harness/core_gates.omni` (`omnic_pure.elf b2-pure-verifier`, full-text 0-byte diff) |
| `s2_security` | 9 | **ORACLE** | Includes Rust `serve --bind`, Rust `p4-exec`, and Rust temp-ELF `0700` checks; Pure-Omni security invariants (`PT_GNU_STACK`, `ERR_MISSING_CAP=1003`, OOB trap `rc=1`) are covered by `s5-refreeze`, `b2-pure-verifier`, and `p2a-syscalls` |
| `s4_honesty` | 3 | **ORACLE** | Locks Rust `wc -l` in `README.md` and Rust `compile-elf` arg defaults; retires at Step 9 |
| `s3_perf` | 5 | **PURE-LOGIC** | Covered by `proposer-loop` sequential timing + `m2-fixed-point` ELF size pin (`125,539 B`) (mechanism §2.12) |
| `m3_modular` | 13 | **PORT-NOW → PORTED** | `pure/harness/extra_gates.omni` (`omnic_pure.elf m3-modular`, full-text 0-byte diff) |
| `audit_view` | 27 | **PORTED** | `pure/harness/audit_view_gate.omni` (`omnic_pure.elf audit-view`, full-text 0-byte diff) |
| `proposer_loop` | 10 | **PORTED** | `pure/harness/proposer_loop.omni` (`omnic_pure.elf proposer-loop`, full-text 0-byte diff) |
| `p2a_syscalls` | 7 | **PORTED** | `pure/harness/core_gates.omni` (`omnic_pure.elf p2a-syscalls`, full-text 0-byte diff) |
| `s5_refreeze` | 4 | **PORT-NOW → PORTED** | `pure/harness/extra_gates.omni` (`omnic_pure.elf s5-refreeze`, full-text 0-byte diff) |
| `s6_cleanup` | 5 | **ORACLE** | Checks Rust workspace/bootstrap cleanup invariants; retires at Step 9 |
| `b4_pure_launcher` | 5 | **PORT-NOW → PORTED** | `pure/harness/extra_gates.omni` (`omnic_pure.elf b4-pure-launcher`, full-text 0-byte diff) |
| `m3_14_lockstep_frontend` | 6 | **ORACLE** | Toggles Rust global M3 lockstep shadow frontend; retires at Step 9 |

*Subtotal B:* `4+2+1+9+51+306+34+8+8+8+4+4+7+3+47+50+24+14+25+11+10+4+11+3+3+3+10+7+6+5+5+5+5+6+6+6+5+4+16+9+3+5+13+27+10+7+4+5+5+6 = 834`.
*Grand Total:* `989 + 834 = 1,823`.

---

## 2. Coverage Mechanisms for Every Retained PURE-LOGIC Suite (692 Checks)

Every PURE-LOGIC suite not marked `PORT-NOW` carries an explicit mechanism showing which surviving Pure-Omni anchor fails if that property regresses:

- **§2.1 `core_inline_1_to_13o` (200):** Tests arithmetic/edge-case goldens (`IABS`, `IDIV 0`, `SHL 65`, `SEL_TRIT`, `BR_POS`, `BR_NEG`, `BR_TRIT`, `do_while`, `pack5`), negative verifier checks (taint/cap/linear leak/SSA gap), and multi-function calls. **Mechanism:** All 16 surface goldens are compiled, decompiled, and recompiled with 0-word diff in `audit_view_gate.elf` (`audit-view`); any opcode encoding or lowering regression breaks the 0-word round-trip. Negative verifier rules (`ERR_MISSING_CAP=1003`, truncated/unframed streams) fail closed in `b2-pure-verifier` and `audit_view_gate.elf`. Native x86-64 execution parity is checked on `kv_ledger_service`, `compiler_self_audit`, and `pack5_decision_controller` in `verify_step7_driver.sh`.
- **§2.2 `ai_first_surface` (360):** Enforces strict surface grammar, `pure/isa/table.omni` metadata alignment, multi-byte `ld_u8`/`st_u16`/`iset_patch_i64` lowering, 16-label cap, and frozen hashes. **Mechanism:** `m2.elf` (`125,539 B`) and `m3.elf` (`206,946 B`) themselves use `ld_u8`, `st_u16`, `iset_patch_i64`, `pure/isa/table.omni`, and up to 16 labels per function; any mis-lowering or ISA table drift immediately breaks `m2-fixed-point` (`S1 != S2`) or changes `m2.elf`/`m3.elf` hashes, while any label-cap violation fails `m2-lint` (`HARD_CAP`).
- **§2.3 `if_pos_join_slot_stage1` (16) & `join_live_dest` (63):** Guards against B6 register clobbering (`v0 = 0` inside an `if_pos` arm clobbering a live join destination). **Mechanism:** `audit_view_gate.elf` Check 18 explicitly constructs the B6 clobber `.omni_bc`, verifies `v0 = 0;` visibility, and recompiles it; additionally, `omnic_pure.elf` (`48,828 B` → `53,416 B`) and `m2.elf` contain dozens of nested `if_pos` join-expressions whose miscompilation breaks `omnic_pure.elf` self-compile fixed point (`cmp` 0-byte diff) and `m2-fixed-point`.
- **§2.4 `m2_shape_census` (1):** Counts instruction shapes in the frozen corpus. **Mechanism:** Covered by `m2-frozen` (all 4 frozen `.omni_bc` hashes + `stage1_elf_bytes` pin); any corpus shape change alters the FNV-1a64 hash of the frozen seed.
- **§2.5 `m2_label_reference` (9) & `m2_labels_table` (8):** Verifies label relocation table encoding and 16-label bounds. **Mechanism:** `m2.elf` uses `pure/m2/labels.omni` to relocate every forward/backward branch when lowering `m2.omni_bc` (`43,669 w`) into `125,539 B` `m2.elf` (`0x0A5CD5B55707AA0D`); any relocation offset error breaks `S1 == S2` in `m2-fixed-point` or crashes `S1` when compiling `S2`.
- **§2.6 `m3_isa_encode_roundtrip` (34):** Verifies `pure/isa/encode.omni` u16 token emission round-trip. **Mechanism:** `m3.elf` (linked by `m3-modular`) emits every `.omni_bc` token through the M3 encoder, and `audit_view_gate.elf` round-trips all 16 goldens (`compile -> decompile -> recompile` with 0-word diff); any encoding mismatch breaks the 0-word round-trip and `omnic_pure.elf` self-compile fixed point.
- **§2.7 `m2_11_arena_elf` (4) & `m2_15_pure_multi_compiler` (7):** Verifies ELF64 header/PHDR emission, 1 MiB arena setup, and multi-function call frames. **Mechanism:** `m2-fixed-point` (`S1 == S2 == S3` = `125,539 B`), `s5-refreeze` (`PT_GNU_STACK` non-exec `p_flags == 6`), and `m3-modular` (`206,946 B` linked `m3.elf`) execute the emitted ELF64 headers and multi-function call frames directly on the Linux x86-64 kernel.
- **§2.8 `m2_13_stage1_anchor` (4):** Pins the Stage-1 self-compiler anchor (`112,180 B`). **Mechanism:** Directly checked by `m2-frozen` (`stage1_elf_bytes 112180 0x62F14B335B52BECC OK`).
- **§2.9 `p7_0_measure` (10), `p7_0a_canary` (7), `p7_1_self_heal` (6), `p7_adapt` (6), `p7_learn` (6):** Verifies benchmark measurement, canary ruler (`+1t`/`0t`/`-1t`), CEGIS rejection, and M7 modules. **Mechanism:** `proposer_loop.elf` (`omnic_pure.elf proposer-loop`) directly executes Stage 0 deny-list lock, Stage 1 CEGIS rejection (`43 != 42`), Stage 3 Tier-1 peephole identity elimination (`+1t` ADOPT / `0t` HOLD / `-1t` REJECT), and Stage 4 audit view; any regression in `pure/m7/*.omni` lint budgets also fails `m2-lint`.
- **§2.10 `api_stage1` (5), `api_stage2` (5), `api_stage2_emit` (5), `api_stage2_5` (5):** Verifies `pure/api/*.omni` reflection/emission modules. **Mechanism:** All `pure/api/*.omni` modules are gated by `m2-lint` (SSA/label/linear-bytes budgets) and `verifier.elf` (linear reflection & capability soundness).
- **§2.11 `a2_pure_include` (5):** Verifies recursive `include "..."` resolution and cycle rejection. **Mechanism:** `scripts/build_pure_driver.sh` and `omnic_pure.elf compile` expand multi-file `include` trees (`pack5_decision_controller.omni` → `std/{pack5,decision,resource}.omni`; `omnic_driver.omni` → `../link/elf_link.omni` + `sys_util.omni`) and verify 0-byte diff in `verify_step7_driver.sh` (including self-compile fixed point).
- **§2.12 `s3_perf` (5):** Locks ELF size and sequential timing discipline. **Mechanism:** `m2-fixed-point` pins `m2.elf` at exact `125,539 B` and `m3-modular` pins `m3.elf` at exact `206,946 B`; any code-size regression moves those byte-exact pins immediately.
