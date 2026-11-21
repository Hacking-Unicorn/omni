# Omni Pure Self-Host Instructions
**Target: Pure-Omni Surface Compiler + Host Runtime (No Rust)**

**Document Version:** 1.0  
**Date:** 2026-11-06  
**Target Audience:** External AI agent / implementer  
**Purpose:** Precise, ordered instructions to complete the remaining Phase 7 goal — a pure-Omni implementation of the surface language compiler, verifier integration, and host runtime — while preserving every existing verification, fixed-point, differential, and error-contract guarantee.

**Current Foundation (already complete and must not be broken):**
- Phases 1–6 complete
- Acceptance suite + error contract + `acceptance-diff`
- Narrow host interface (`OmniNarrowHost`)
- Deepened repair strategies (`omni-repair-strategies-v1.1.0`)
- Compositional invariants + trusted core
- Hygienic macros + `std/prelude.omni`
- Pure-Omni **verifier kernel** with 4-way differential equivalence
- 333 monolith / 294–295 workspace checks

**Guiding Principle:**  
The pure-Omni port is driven by the existing executable acceptance suite and differential harness. Every step must keep the 4-way equivalence (VM == Host x86-64 ELF == Stage-1 ELF bit-exact == Wasm) and the machine-readable error contract intact.

---

## 0. Non-Negotiable Constraints

1. All existing differential verification (monolith vs workspace, VM vs Stage-1 vs x86-64 ELF AOT vs Wasm) must continue to pass.
2. Golden size regression gates and exact token-count locks remain mandatory.
3. Machine-readable `error[<CODE>]` with non-null 1-based `(line, col)` spans is permanent.
4. Standalone AOT backends continue to explicitly reject host opcodes.
5. Zero external crate dependencies in the core.
6. Every new pure-Omni component must be accompanied by goldens that exercise it through the acceptance suite.
7. The pure-Omni surface compiler must eventually be able to compile itself (or a sufficiently complete subset that can bootstrap the rest).
8. The pure-Omni host runtime must target the already-defined `OmniNarrowHost` interface; it must not invent a wider syscall surface.
9. Prefer incremental, differentially verified steps over a big-bang rewrite.

---

## 1. Overall Strategy

1. **Phase A** — Pure-Omni surface language frontend (`omni-pure-frontend-v1.0.0`: lexer → parser → AST → lowering to `u16` token stream + source-map generation)
2. **Phase B** — Pure-Omni driver & integration with the existing pure-Omni verifier kernel (`omni-pure-driver-v1.0.0`)
3. **Phase C** — Pure-Omni host runtime implementing `OmniNarrowHost` (`omni-pure-host-v1.0.0`)
4. **Phase D** — Self-hosting bootstrap (pure-Omni compiler compiling the pure-Omni compiler to a bit-exact Gen-1 == Gen-2 fixed point, `omni-pure-selfhost-v1.0.0`)
5. **Phase E** — Full differential lock (`pure-omni-diff`), agent-driver/harness integration, and documentation
