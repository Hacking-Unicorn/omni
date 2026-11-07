# Omni Improvement & Evolution Roadmap (Phase 7 Complete)

**Document Version:** 1.1 (Phase 7 Preparation Sections 1–7 + Phase 7 Pure-Omni Kernel Port Verified)  
**Target Audience:** External AI agent / implementer / auditor  
**Purpose:** Precise, actionable instructions and verified status for evolving Omni from Phases 1–6 through Phase 7 (`omni-acceptance-suite-v1.0.0`, `omni-error-contract-v1.0.0`, `omni-repair-strategies-v1.1.0`, `omni-narrow-host-v1.0.0`, `omni-pure-kernel-v1.0.0`).  
**Guiding Principle:** Preserve and extend the existing verification density, fixed-point proofs, differential checks, machine-readable error contracts, zero-dependency purity, and AI-first surface. Never dilute existing guarantees.

---

## Executive Implementation Status (All Sections Complete & Verified)

| Section | Deliverable | Version / Artifact | Status |
|---|---|---|---|
| **0. Non-Negotiable Constraints** | 4-way differential (VM / Stage-1 / ELF64 / Wasm), 15 golden size locks (`[53, 34, 34, 91, 40, 20, 22, 22, 49, 49, 51, 74, 69, 57, 86]`), zero external deps | `333` monolith / `294` Windows (`295` Linux) workspace checks | **COMPLETE** |
| **1. Executable Acceptance & Error Contract** | `24` positive + `16` negative goldens, 16-code error contract selftest, `omni acceptance-diff` | `omni-acceptance-suite-v1.0.0`, `omni-error-contract-v1.0.0`, `spec/acceptance/` | **COMPLETE** |
| **2. First-Class Versioned AI Repair Loop** | 16 versioned `OmniRepairStrategySpec` strategies (`--list-strategies`), multi-function/include/cap/budget repair | `omni-repair-strategies-v1.1.0` | **COMPLETE** |
| **3. Narrow Capability-Gated Host Interface** | 10-syscall `OmniNarrowHost` trait, `StdNarrowHostAdapter`, `SimulatedNarrowHostAdapter` | `omni-narrow-host-v1.0.0`, `spec/host_interface.md` | **COMPLETE** |
| **4. Compositional Verification & Trusted Core** | 5 compositional soundness lemmas (`13AB`) + mechanized proof plan | `spec/trusted_core.md`, `spec/mechanized_proof_plan.md` | **COMPLETE** |
| **5. Next Abstraction Layer** | Hygienic `macro <name>(...) = <expr>;` + verified `include "std/prelude.omni";` (`13AA`) | `spec/next_abstraction.md` | **COMPLETE** |
| **6. Tooling & Human Diagnostics** | `omni inspect` / `omni disasm`, `format_omni_human_diagnostic`, `--explain <CODE>` (`13z`) | `format_omni_inspect`, `explain_omni_error_code` | **COMPLETE** |
| **7. Heterogeneous Execution Design** | CPU/GPU/NPU placement, linear buffer transfer, per-device budget & fence specification | `spec/heterogeneous_design.md` | **COMPLETE** |
| **8. Phase 7 Pure-Omni Kernel Port** | Self-hosted Pure-Omni static verifier kernel (`omni_pure_verify_header`), 4-way differential + Stage-1 ELF64 fixed point + Narrow-Host full-stack execution (`13AC`) | `omni-pure-kernel-v1.0.0` | **COMPLETE** |

---

## 0. Non-Negotiable Constraints (Read First)

1. All new work must maintain or improve the existing differential verification story (monolith vs workspace, VM vs Stage-1 vs x86-64 ELF AOT vs Wasm AOT).
2. Golden size regression gates and exact `Vec<u16>` / token-count locks must continue to pass.
3. Machine-readable `error[<CODE>]` format with non-null 1-based `(line, col)` spans is a permanent contract.
4. Standalone AOT backends must continue to explicitly reject host opcodes (Section 6 of `SPEC.md`).
5. Zero external crate dependencies in the core workspace.
6. Any change to the surface language, verifier, or host must be accompanied by new or updated goldens that exercise the change.
7. The version-locked harness contract (`omni-harness-v1.0.0` and successors) must remain stable or be explicitly version-bumped with migration notes.

---

## 1. Treat Goldens + Error Contract as the Primary Executable Specification (Phase 7 Preparation)

### Goal
Make the pure-Omni surface + verifier + host port (Phase 7) driven by an executable specification rather than a manual rewrite of the Rust code. This reduces semantic drift risk and enables continuous differential checking from the first day of the port.

### Implementation Requirements

1. **Extract and freeze the current acceptance suite**
   - Collect every golden program used in selftests (the 15 original SPEC goldens + all later surface, arena, PACK5, budget, host, strict-mode, multi-function, and include goldens).
   - For each golden record:
     - Source `.omni` text (or the exact snippet used in the test).
     - Expected `Vec<u16>` token stream (or its FNV-1a64 / exact length).
     - Expected execution result under VM and under AOT (where applicable).
     - Expected structured error (code + line + col) for negative tests.
   - Store this suite in a dedicated directory: `spec/acceptance/` with a machine-readable manifest (`acceptance_manifest.json`).

2. **Make the error contract the source of truth**
   - Document every `error[<CODE>]` currently emitted by lexer, parser, lowering, verifier, PACK5 validation, and budget traps (`spec/error_contract.md` and `spec/acceptance/error_contract.json`).
   - For each code provide:
     - Exact string format.
     - When it is emitted.
     - Guaranteed presence of non-null `line` and `col`.
     - Whether a retry hint is produced by the harness.
   - Add a selftest (`run_error_contract_selftest`) that parses every structured error and asserts the contract.

3. **Continuous differential harness for the pure-Omni port**
   - Implement a differential runner (`run_omni_acceptance_diff` / `omni acceptance-diff`) that:
     - Feeds the same acceptance suite to the current Rust surface compiler.
     - Feeds the same suite to the candidate/pure-Omni compiler (`--candidate-cmd`).
     - Asserts bit-identical token streams and identical structured errors.
   - Fail the build if any golden diverges.

---

## 2. Deepen the AI Repair Loop (Harness + Agent Driver)

### Goal
Move the repair loop from single-error, single-function fixes to multi-step, multi-function, capability-aware, budget-aware, and include-aware repair strategies. Make the strategies themselves first-class and versioned (`omni-repair-strategies-v1.1.0`).

### Implementation Requirements

1. **Expand error coverage in the repair loop**
   - Explicit multi-step repair strategies for:
     - Multi-function modules (error in callee, error in caller, arity mismatches across functions).
     - `include` / module graphs (`repair_omni_include_graph`: circular includes, missing files, strict-mode fixes).
     - Capability inference and minimal capability set suggestion (`infer_minimal_capabilities_for_error`).
     - Budget violations (`BUDGET_EXCEEDED`) with concrete suggestions (`#[step_budget]`, `#[mem_budget]`).
     - Strict-mode violations (`AMBIGUOUS_GRAMMAR`, missing type annotations).

2. **Make repair strategies first-class and versioned**
   - `OmniRepairStrategySpec` (`list_omni_repair_strategies()`, `omni_repair_strategies_json()`).
   - Versioned strategy set: `omni-repair-strategies-v1.1.0`.
   - `omni-agent-driver --list-strategies` and `strategy_id` / `strategy_version` in JSON output.

---

## 3. Define and Freeze a Narrow Capability-Gated Host Interface

### Goal
Resolve the host purity vs. practicality tension. Create a minimal, explicit, capability-gated 10-syscall boundary (`OmniNarrowHost`, `omni-narrow-host-v1.0.0`) that the pure Omni runtime targets.

### Implementation Requirements

1. **Narrow host interface (`OmniNarrowHost`)**
   - 10 capability-gated operations:
     - `host_tcp_listen`, `host_tcp_accept`, `host_tcp_read`, `host_tcp_write`, `host_tcp_close` (`CAP_ID_NET = 2`)
     - `host_file_open`, `host_file_read`, `host_file_write`, `host_file_close` (`CAP_ID_DB_R = 0` / `CAP_ID_DB_W = 1`)
     - `host_time_ns` (`CAP_ID_SYS = 9`)
   - Reference adapters: `StdNarrowHostAdapter` (production TCP/filesystem) and `SimulatedNarrowHostAdapter` (deterministic socket-free test adapter).
   - Documented in `spec/host_interface.md`.

---

## 4. Strengthen Verification Toward Compositionality

### Goal
Complement the golden suite with compositional verification lemmas (`run_compositional_invariant_selftests`) and a mechanized proof roadmap.

### Implementation Requirements

1. **Trusted Core (`spec/trusted_core.md`)**: Token stream decoder, static verifier, and core AOT lowering.
2. **5 Compositional Soundness Lemmas (`13AB`)**:
   - Lemma 1: Branched taint non-interference (`if_pos` / `if_neg` / `match_trit` join points).
   - Lemma 2: Multi-arena linear conservation (`allocated_bytes == 0` at exit).
   - Lemma 3: Capability monotonicity across all gated opcodes in `OMNI_ISA_TABLE`.
   - Lemma 4: Hot-path AOT observational equivalence across 64-bit boundary inputs (`0`, `i64::MIN`, `i64::MAX`, `-1`).
   - Lemma 5: Hygienic macro taint & scope transparency.
3. **Mechanized Proof Plan (`spec/mechanized_proof_plan.md`)**: Lean 4 / Coq / Iris mechanization roadmap.

---

## 5. Next Abstraction Layer (Without Breaking Verification)

### Goal
Raise expressiveness via zero-opcode hygienic macros (`macro <name>(...) = <expr>;`) and a verified pure-Omni standard library prelude (`include "std/prelude.omni";`: `std_clamp`, `std_abs_diff`, `std_trit_mux`, `std_consensus3`). Documented in `spec/next_abstraction.md`.

---

## 6. Tooling & Human Diagnostics Improvements

### Goal
- `omni inspect` / `omni disasm` (`format_omni_inspect`, `inspect_omni_surface_program`) with opcode names, register/parameter/label decoding, `// line L, col C` source-map annotations, and capability/taint flags.
- `format_omni_human_diagnostic` for multi-line rustc-style caret diagnostics alongside single-line harness JSON.
- `omni --explain <CODE>` (`explain_omni_error_code`) for all 16 error codes.

---

## 7. Heterogeneous Execution (Design Only)

### Goal
Rigorous design in `spec/heterogeneous_design.md` covering CPU/GPU/NPU placement (`#[device(cpu | gpu | npu)]`), linear arena device transfer (`device_move`), per-device capability/taint/supported-subset fences, and unified step/memory budgets.

---

## 8. Phase 7 Pure-Omni Self-Hosted Verifier & Surface Kernel Port

### Goal
Implement and verify the Pure-Omni static verification decision kernel (`PHASE7_PURE_OMNI_VERIFIER_KERNEL_SURFACE`, `omni-pure-kernel-v1.0.0`), prove its 4-way differential equivalence across `OmniBinaryVm`, Host `x86-64` ELF64, Stage-1 `.omni_bc` ELF64 (bit-identical fixed point), and Wasm 64-bit AOT, and execute pre-verified full-stack `.omni` routes over `OmniNarrowHost`.
