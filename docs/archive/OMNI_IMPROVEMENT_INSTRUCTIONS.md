# Omni Improvement & Evolution Instructions

**Document Version:** 1.0  
**Target Audience:** External AI agent / implementer  
**Purpose:** Precise, actionable instructions to evolve Omni from its current high-rigor state (Phases 1–6 complete) into a stronger, more scalable, and potentially genius-level system.  
**Guiding Principle:** Preserve and extend the existing verification density, fixed-point proofs, differential checks, machine-readable error contracts, zero-dependency purity, and AI-first surface. Never dilute existing guarantees.

---

## 0. Non-Negotiable Constraints (Read First)

1. All new work must maintain or improve the existing differential verification story (monolith vs workspace, VM vs Stage-1 vs x86-64 ELF AOT vs Wasm AOT).
2. Golden size regression gates and exact `Vec<u16>` / token-count locks must continue to pass.
3. Machine-readable `error[<CODE>]` format with non-null 1-based `(line, col)` spans is a permanent contract.
4. Standalone AOT backends must continue to explicitly reject host opcodes (Section 6 of SPEC.md).
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
   - Store this suite in a dedicated directory: `spec/acceptance/` with a machine-readable manifest (`acceptance_manifest.json` or equivalent).

2. **Make the error contract the source of truth**
   - Document every `error[<CODE>]` currently emitted by lexer, parser, lowering, verifier, PACK5 validation, and budget traps.
   - For each code provide:
     - Exact string format.
     - When it is emitted.
     - Guaranteed presence of non-null `line` and `col`.
     - Whether a retry hint is produced by the harness.
   - Add a selftest that parses every structured error and asserts the contract.

3. **Continuous differential harness for the pure-Omni port**
   - Before writing significant pure-Omni surface code, implement a differential runner that:
     - Feeds the same acceptance suite to the current Rust surface compiler.
     - Feeds the same suite to the emerging pure-Omni surface compiler.
     - Asserts bit-identical (or semantically equivalent under a defined mapping) token streams and identical structured errors.
   - Fail the build if any golden diverges.

4. **Acceptance criteria for this section**
   - `spec/acceptance/` exists and contains a complete, versioned snapshot of current goldens.
   - A new selftest (`acceptance_suite_selftest`) passes and is part of both monolith and workspace selftests.
   - The differential runner exists and can be invoked as `omni acceptance-diff` or equivalent.

---

## 2. Deepen the AI Repair Loop (Harness + Agent Driver)

### Goal
Move the repair loop from single-error, single-function fixes to multi-step, multi-function, capability-aware, budget-aware, and include-aware repair strategies. Make the strategies themselves first-class and versioned.

### Implementation Requirements

1. **Expand error coverage in the repair loop**
   - Current loop already handles core codes (TAINT_ON_RET, TAINT_SINK, MISSING_CAP, LINEAR_LEAK, BORROW_CONFLICT, BAD_TYPE, etc.).
   - Add explicit multi-step repair strategies for:
     - Multi-function modules (error in callee, error in caller, arity mismatches across functions).
     - `include` / module graphs (circular includes, missing files, name collisions).
     - Capability inference and minimal capability set suggestion.
     - Budget violations (`BUDGET_EXCEEDED`) with concrete suggestions (reduce allocation, add `#[step_budget]`, etc.).
     - Strict-mode violations (`AMBIGUOUS_GRAMMAR`, missing type annotations).

2. **Make repair strategies first-class and versioned**
   - Introduce a `RepairStrategy` abstraction (or pure data description) that can be serialized.
   - Version the strategy set (e.g., `omni-repair-strategies-v1.1.0`).
   - The agent driver must be able to list available strategies and select them by error code + context.
   - Record which strategy was used in the harness JSON output.

3. **Capability-aware and budget-aware synthesis**
   - When a `MISSING_CAP` error occurs, the repair prompt / strategy must propose the minimal additional capabilities required.
   - When a budget error occurs, the strategy must propose concrete attribute values or code changes that stay under budget.
   - Add goldens that demonstrate successful multi-step repair of these classes.

4. **Multi-file and include-aware repair**
   - The repair loop must be able to receive a set of related `.omni` files (or an include graph) and propose coordinated fixes across files.
   - Negative tests for circular includes and missing modules must produce actionable repair prompts.

5. **Acceptance criteria**
   - New selftests cover at least 8 distinct multi-step repair scenarios (including multi-function, include, capability, and budget).
   - Harness JSON output includes `strategy_id` and `strategy_version`.
   - `omni-agent-driver --list-strategies` (or equivalent) works.
   - All existing single-error repair goldens continue to pass.

---

## 3. Define and Freeze a Narrow Capability-Gated Host Interface

### Goal
Resolve the host purity vs. practicality tension before the pure-Omni port begins. Create a minimal, explicit, capability-gated boundary that the pure Omni runtime can target.

### Implementation Requirements

1. **Design the narrow host interface**
   - List the absolute minimum operations required for the current live host features:
     - TCP listen / accept / read / write / close
     - File open / read / write / close (for the KV/DB backing store)
     - High-resolution time (if used by budgets or logging)
   - Map each operation to existing or new capability bits.
   - Define the exact binary or surface-level calling convention that pure Omni code will use to invoke these operations.
   - Explicitly state which current host opcodes (`0x7F64..=0x7F7F`) will be implemented by calling into this narrow interface versus being reimplemented in pure Omni.

2. **Document the trust boundary**
   - Create `spec/host_interface.md` (or a new section in SPEC.md) that states:
     - Which operations are trusted (implemented outside pure Omni).
     - Which capabilities gate them.
     - What the pure Omni runtime is allowed to assume about their behavior.
     - What is explicitly *not* part of the trusted interface (everything else must be pure Omni).

3. **Implement a thin adapter in the current Rust host**
   - Refactor the existing `omni serve` / DB/KV path so that all syscalls go through the documented narrow interface.
   - This adapter becomes the reference implementation that the pure Omni host will eventually match or call.

4. **Acceptance criteria**
   - `spec/host_interface.md` exists and is referenced from SPEC.md.
   - All current live host selftests (`13v` / `6w` and related) still pass after the refactor.
   - No new host opcodes are introduced without an explicit decision and SPEC update.
   - The interface is small enough that a pure Omni implementation (or a pure Omni + thin syscall layer) is realistic.

---

## 4. Strengthen Verification Toward Compositionality (Longer-Term)

### Goal
Reduce reliance on ever-growing golden suites and move toward a more compositional verification story that can survive surface and host growth.

### Implementation Requirements (Incremental)

1. **Identify a trusted core**
   - Select the smallest set of components whose correctness is load-bearing:
     - Token stream decoder
     - Static verifier (capability, linear, taint, SSA, branch targets)
     - Core AOT lowering for the pure numeric/ternary/@PACK5 subset
   - Document why this core is trusted and what would be required to raise confidence further (e.g., model checking, interactive proofs, or exhaustive state exploration on small programs).

2. **Introduce compositional lemmas / invariants**
   - For the verifier: state and test invariants such as “if `verify_binary_function` accepts a function, then every execution path under the VM respects linear ownership and taint rules.”
   - For AOT: “if a module is accepted by `is_module_aot_hot_path` and compiles, its observable behavior matches the VM on all goldens.”
   - Encode as many of these as possible as additional selftests or property checks.

3. **Plan (do not yet fully implement) a path to machine-checkable proofs**
   - Write a short design note outlining what a CompCert-style or Iris-style proof of the verifier + core AOT would require.
   - Identify the largest obstacles (memory model, host interface, ternary packing).
   - This note is planning only; full mechanized proof is a later phase.

4. **Acceptance criteria for this increment**
   - Trusted-core document exists.
   - At least 5 new compositional / invariant selftests are added and passing.
   - Design note on mechanized proof path exists in `docs/` or `spec/`.

---

## 5. Plan the Next Abstraction Layer (Without Breaking Verification)

### Goal
Raise the expressiveness ceiling in a controlled way so that AI agents and humans can write larger programs without forcing growth of the core ISA or loss of verification density.

### Implementation Requirements

1. **Define the next layer of abstraction**
   - Candidates (choose one primary direction and document why):
     - Hygienic macros / syntax extensions that expand to the existing surface before verification.
     - A small module system with explicit exports and capability requirements.
     - A pure Omni standard library of data structures (vectors, maps, etc.) implemented entirely in the current surface language and verified under the existing rules.
   - Whatever is chosen must lower to the existing ISA with zero new opcodes and must be fully subject to the existing verifier.

2. **Verification requirements for the new layer**
   - Every abstraction must be expandable / inlinable to a form that the current goldens and differential suite can check.
   - Capability and taint must be tracked through the abstraction.
   - Linear resources must not be hideable behind the abstraction.

3. **Acceptance criteria**
   - A design document (`spec/next_abstraction.md`) exists that states the chosen direction, the lowering rules, and the verification strategy.
   - At least one non-trivial example is implemented and passes the full differential suite.
   - No changes to the core ISA or to the standalone AOT rejection list are required.

---

## 6. Tooling & Human Diagnostics Improvements

### Goal
Improve inspectability and debuggability of binary token streams and surface programs without compromising the AI-first / verification-first design.

### Implementation Requirements

1. **Binary token stream inspection**
   - Enhance or add a command (`omni disasm` or `omni inspect`) that pretty-prints a `.omni_bc` or `Vec<u16>` stream with:
     - Opcode names
     - Register / parameter / label decoding
     - Source map annotations when available
     - Capability and taint annotations
   - Output must be deterministic and suitable for both humans and further agent consumption.

2. **Improved diagnostic surface**
   - Ensure every structured error can be rendered in a human-readable multi-line form while still producing the machine-readable single-line form for the harness.
   - Add an optional `--explain` flag to the harness / agent driver that expands the error code into a short explanation + suggested repair patterns.

3. **Acceptance criteria**
   - `omni inspect` (or equivalent) exists and is covered by selftests.
   - Human-readable error rendering does not break any existing harness JSON consumers.
   - At least three new selftests demonstrate the inspection and explanation features.

---

## 7. Heterogeneous Execution (Design Only — Do Not Implement Yet)

### Goal
Produce a rigorous design for CPU/GPU/NPU execution that preserves Omni’s static guarantees. Implementation is explicitly deferred until after Phase 7 and the host interface are complete.

### Implementation Requirements (Design Document Only)

1. Create `spec/heterogeneous_design.md` covering:
   - Memory model differences and data movement.
   - How capability, taint, and linear ownership are tracked across devices.
   - What the static verifier can and cannot prove about multi-device programs.
   - Placement / scheduling annotations (surface-level).
   - Explicit “supported subset” fences per backend (analogous to current AOT rejection list).
   - Budget model across devices.
   - Failure modes and structured error codes for device-related faults.

2. The design must state clear non-goals for the first version (e.g., no dynamic kernel generation, no arbitrary pointer sharing, etc.).

3. **Acceptance criteria**
   - The design document exists, is internally consistent, and is referenced from the main roadmap / SPEC.
   - No implementation code for GPU/NPU paths is added in this phase.

---

## 8. Overall Execution Order for the External AI

Execute in this sequence unless a dependency forces a change:

1. Section 1 — Acceptance suite + differential harness (foundation for everything else).
2. Section 3 — Narrow host interface (required before pure host work).
3. Section 2 — Deepen repair loop (can proceed in parallel with 1 and 3 after the acceptance suite exists).
4. Section 6 — Tooling & diagnostics (low risk, high usability).
5. Section 5 — Next abstraction layer design + one example.
6. Section 4 — Compositional verification increments.
7. Section 7 — Heterogeneous design document only.
8. Only after the above are stable: begin the actual pure-Omni surface + verifier + host port (Phase 7), driven by the acceptance suite from Section 1.

---

## 9. Definition of Done for This Instruction Set

- All listed design documents and acceptance artifacts exist in the repository.
- New selftests for every implemented feature are present and passing in both monolith and workspace.
- Existing Phase 1–6 guarantees (fixed-point, differential counts, golden sizes, error contract) remain intact.
- The external AI produces a short status report after each major section listing:
  - What was implemented.
  - Which selftests were added or updated.
  - Any deviations from these instructions and why.
  - Remaining risks.

---

**End of Instructions**

These instructions deliberately prioritize verification integrity, executable specifications, and controlled growth over feature velocity. Follow them strictly. The quality of Omni depends on it.
