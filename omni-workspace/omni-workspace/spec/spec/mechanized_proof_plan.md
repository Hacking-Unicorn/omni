# Mechanized Proof Plan for the Omni Verifier & Core AOT (`spec/mechanized_proof_plan.md`)

**Status:** Design & Formalization Roadmap (Section 4.3 of `OMNI_IMPROVEMENT_INSTRUCTIONS.md`)

## 1. Objective

Provide a realistic, incremental roadmap to mechanize the soundness of Omni's Trusted Computing Base (`spec/trusted_core.md`) in **Lean 4** (or Coq / Iris), covering:
1. The static binary verifier (`verify_binary_function`): Taint Non-Interference, Capability Safety, and Linear Arena Conservation.
2. Observational equivalence (CompCert-style simulation) between `OmniBinaryVm` and the pure numeric/ternary/`@PACK5` `x86-64` / Wasm AOT backends.

## 2. Target Theorems

### Theorem 1 — Static Verifier Soundness (Progress & Preservation)
For any decoded `BinFunction` $F$, if `verify_binary_function(F).passed == true`:
- **Capability Safety:** No step of `OmniBinaryVm::execute_function(F)` invokes an opcode or `OmniNarrowHost` syscall whose `req_cap_mask` is not a subset of `F.cap_mask`.
- **Taint Non-Interference:** Treating `@SAFE` inputs as Low ($L$) and `@UNTRUSTED` inputs as High ($H$), no unsanitized $H$-dependent value is ever passed to an opcode with `sink_requires_safe == true` or returned by `RET` (`0x7F4A`).
- **Linear Resource Conservation:** Every `ARENA` (`0x7F01`) handle allocated along any terminating path is dropped exactly once (`0x7F05`) with zero active readers/writers (`0x7F02`/`0x7F03`), yielding `allocated_bytes == 0` upon function return.

### Theorem 2 — AOT Forward Simulation (VM $\approx$ x86-64 ELF64 $\approx$ Wasm)
For any `BinFunction` $F$ where `verify_binary_function(F).passed == true` and `is_module_aot_hot_path(F) == true`, and for all 64-bit inputs $(a, b, c) \in \mathbb{Z}_{2^{64}}^3$ and step budget $B$:
- `execute_wasm_module_i64(compile_pure_u16_to_wasm(F), [a, b, c])` and the `x86-64` ELF64 code emitted by `compile_pure_u16_to_x86_64_elf_with_args(F, a, b, c)` produce the exact same 64-bit two's-complement result (including totalized division/modulo by zero and shift-amount masking) as `OmniBinaryVm`.

## 3. Principal Obstacles & Mitigation Strategy

| Obstacle | Why It Is Hard | Mechanization Strategy |
|---|---|---|
| **1. Base-3 `@PACK5` Arithmetic (`3^5 = 243`)** | Bitwise `i64` arithmetic in SMT/Lean is standard, whereas 5-trit-per-byte base-3 division/modulo requires nonlinear arithmetic lemmas. | Prove a finite lookup-table bijection on `u8` (`0..=242` $\leftrightarrow \{-1, 0, +1\}^5$) by exhaustive `decide` / `bv_decide` reflection in Lean 4. |
| **2. Linear Arena Borrow State Machine** | Aliased register copies could invalidate simple lexical drop checking. | Restrict `ARENA` handles in `verify_binary_function` to single-assignment SSA registers with affine move/borrow transitions modeled via a lightweight Iris resource algebra. |
| **3. Host Boundary (`OmniNarrowHost`)** | External OS sockets and files have nondeterministic failures. | Parameterize the operational semantics over an abstract trace monad implementing the 10-syscall `OmniNarrowHost` interface (`spec/host_interface.md`). |

## 4. Phased Execution Milestones

- **Milestone M1:** Mechanize `OMNI_ISA_TABLE` (`0x7F00..=0x7F7F`) and the `u8` `@PACK5` bijection in Lean 4 via `bv_decide`.
- **Milestone M2:** Mechanize the Kildall fixed-point taint & linear-arena dataflow lattice of `verify_binary_function` and prove Theorem 1.
- **Milestone M3:** Mechanize the straight-line and structured-loop simulation between `OmniBinaryVm` and the Wasm / `x86-64` AOT emitters (Theorem 2).
