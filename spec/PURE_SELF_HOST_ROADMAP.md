# Omni Pure Self-Hosting Roadmap & Verification Report (`omni-pure-selfhost-v1.0.0`)

**Status:** Phases A–E, Pure-Core Completion Phases 1–5 & Strengthening Phases S1–S4 Complete & Verified (`378` Monolith / `339` Windows / `340` Linux Workspace Differential Checks)

---

## 1. Completed Self-Hosting & Pure-Core Completion Phases

| Phase | Contract Version | Surface / Implementation | Status |
|---|---|---|---|
| **Phase 7** (Verifier Kernel) | `omni-pure-kernel-v1.0.0` | `PHASE7_PURE_OMNI_VERIFIER_KERNEL_SURFACE` + `verify_module_via_phase7_pure_omni_kernel` | **Complete & Verified** |
| **Phase A** (Surface Frontend) | `omni-pure-frontend-v1.0.0` | `spec/pure_self_host/frontend.omni` + `compile_omni_surface_via_pure_omni_frontend_with_sourcemap` | **Complete & Verified** |
| **Phase B** (Pure-Omni Driver) | `omni-pure-driver-v1.0.0` | `spec/pure_self_host/driver.omni` + `run_pure_omni_driver_json` (`omni pure-driver`) | **Complete & Verified** |
| **Phase C** (Pure-Omni Host Runtime) | `omni-pure-host-v1.0.0` | `spec/pure_self_host/host_runtime.omni` + `serve_omni_http_via_pure_omni_host` over `OmniNarrowHost` | **Complete & Verified** |
| **Phase D** (Self-Hosting Bootstrap) | `omni-pure-selfhost-v1.0.0` | `spec/pure_self_host/omni_selfhost_compiler.omni` + `run_phase_d_pure_omni_selfhost_bootstrap` (`omni pure-bootstrap`) | **Complete & Verified (`gen1 == gen2`)** |
| **Phase E** (Differential Lock) | `omni-acceptance-suite-v1.0.0` | `run_pure_omni_acceptance_diff` (`omni pure-omni-diff` + `omni-harness`) | **Complete & Verified** |
| **Pure-Core Phase 1** (Full Frontend Coverage) | `omni-pure-selfhost-v1.2.0` | `pure/compiler.omni` (`pure_check_include_graph`, `pure_check_macro_hygiene`, `pure_check_call_and_label`) + `13AJ` | **Complete & Verified** |
| **Pure-Core Phase 2** (Pure-Omni Tooling) | `omni-pure-tooling-v1.1.0` | `pure/tooling.omni` (`pure_explain_error_stage`, `pure_bootstrap_fixed_point_check`) + `inspect_omni_surface_via_pure_omni` + `13AK` | **Complete & Verified** |
| **Pure-Core Phase 3** (Primary Host Runtime) | `omni-pure-host-v1.1.0` | `pure/host_runtime.omni` (`omni_pure_host_check_taint_sink`, `omni_pure_host_kv_apply`) + primary `OmniNarrowHost` routing + `13AL` | **Complete & Verified** |
| **Pure-Core Phase 4** (Expanded Fixed Point) | `omni-pure-selfhost-v1.2.0` | Closed 9-module `std/*` + `pure/*` suite (`pure/selfhost_suite.omni`, `build_pure_omni_core_suite_stream`) + `13AM` | **Complete & Verified (`6,115` words `gen1 == gen2`)** |
| **Pure-Core Phase 5** (Default Workflows) | `omni-pure-selfhost-v1.2.0` | Pure-Omni default across `run`, `eval`, `repl`, `serve`, `compile-surface`, `inspect`, `harness`, `agent-driver` + `13AN` | **Complete & Verified** |
| **Strengthening S1** (Real Programs) | `omni-pure-selfhost-v1.2.0` | 5 real programs in `pure/programs/` + `13AO` | **Complete & Verified** |
| **Strengthening S2** (Deeper Tooling) | `omni-pure-tooling-v1.2.0` | `pure_inspect_module_metrics`, `pure_multi_step_repair_budget`, multi-function `LINEAR_LEAK` repair + `13AP` | **Complete & Verified** |
| **Strengthening S3** (Unique Primitives) | `omni-pure-selfhost-v1.2.0` | `std/pack5.omni`, `std/decision.omni`, `std/resource.omni`, `pure/programs/pack5_decision_controller.omni` + `13AQ` | **Complete & Verified** |
| **Strengthening S4** (Hardening & Metrics) | `omni-pure-selfhost-v1.2.0` | `pure-bootstrap` health metrics (derived from `PURE_OMNI_CORE_SUITE_MODULES`), `TAINT_SINK` repair for `tlogic`/`db_get`/`kv_put`, `MISSING_CAP` inference for `tlogic`/`san_dom`/`san_sql` + `13AR` | **Complete & Verified** |

### Real Pure-Omni Programs (`pure/programs/`)

| Program | Exercises | Verified Result |
|---|---|---|
| `ternary_neural_router.omni` | `@PACK5` dot products, macros, `egate`, `std/math` + `std/trit` | `Int(574)` |
| `arena_audit_pipeline.omni` | Linear arenas, borrows, budgets | `Int(210)` |
| `policy_consensus_engine.omni` | Ternary quorum, `std/http` packing | `Int(200_000_220)` |
| `kv_ledger_service.omni` | `route` + KV over `OmniNarrowHost` | `POST` → `850`, `GET` → `1700` |
| `compiler_self_audit.omni` | Links all `pure/*` core modules | `Int(10771)` |
| `pack5_decision_controller.omni` | `std/pack5` + `std/decision` + `std/resource` | `Int(419)` |

### Health Metrics (`omni pure-bootstrap`)

`stdlib_modules = 7`, `core_modules = 5`, `real_programs = 6`, `total_pure_modules = 18`, `suite_words = 14,559`, `pure-omni-diff = 47` checks.

> [!NOTE]
> The Pure-Omni path is still a **subset** of the Rust bootstrap. The self-host compiler kernel is `159` words; the Rust bootstrap (`omni_rust_bootstrap.rs`) remains the Gen-0 compiler and is still required to build everything. "Gen-1 == Gen-2" proves deterministic agreement between the Pure-Omni frontend and the Rust reference on these modules, not independence from Rust.

---

## 2. Generation-1 == Generation-2 Self-Hosting Fixed-Point Hashes

| Artifact | Size | FNV-1a64 Hash | Fixed-Point Status |
|---|---|---|---|
| `gen1_compiler.omni_bc` == `gen2_compiler.omni_bc` | `159` `u16` words (`318` bytes) | `0x23B8D17257CCA9C4` | **Bit-Identical (`true`)** |
| `gen1_compiler.elf` == `gen2_compiler.elf` | `824` bytes | `0xE221085777B8AC47` | **Bit-Identical (`true`)** |
| Closed 18-Module Pure-Omni Suite (`std/*` + `pure/*` + `pure/programs/*`, `Gen-1 == Gen-2`) | `14,559` `u16` words (`29,118` bytes) | `0x78462AE4C81DF408` | **Bit-Identical (`true`)** |
| `stage1_compiler.elf` == `stage2_compiler.elf` | `19,731` `u16` words / `106,242` bytes | `0xEAFC81F46B482FF8` | **Bit-Identical (`true`)** |

---

## 3. Reproducible Verification Commands

```bash
# 1. Monolith Selftest (378 checks)
rustc -O omni_rust_bootstrap.rs -o omni_boot_bin
./omni_boot_bin selftest

# 2. Pure-Omni Self-Hosting Bootstrap Fixed Point (gen1 == gen2: 159 words + 14,559-word 18-module suite + health metrics)
./omni_boot_bin pure-bootstrap

# 3. Full Pure-Omni Differential Lock (pure-omni-diff, 47 checks)
./omni_boot_bin pure-omni-diff

# 4. Workspace 8-Crate Differential Selftest (339 Windows / 340 Linux checks)
cd omni-workspace/omni-workspace
cargo run -p omni-cli -- selftest
cargo run -p omni-cli -- pure-omni-diff
```
