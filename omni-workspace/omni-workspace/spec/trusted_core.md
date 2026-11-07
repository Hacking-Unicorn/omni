# Omni Trusted Computing Base & Compositional Invariants (`spec/trusted_core.md`)

**Status:** Implemented & Verified (`run_compositional_invariant_selftests`, `13AB` / `6AD`)

## 1. The Minimal Trusted Core (TCB)

Omni's end-to-end safety guarantees rest on three small, zero-dependency components:

1. **Binary Token Stream Decoder (`decode_omni_token_stream`)**
   - Parses raw `u16` streams (`0x0000..=0x7FFF`), enforcing partition boundaries (`TOK_FN_DEF`, `TOK_FN_END`, `TOK_INS_END`, `@PACK5` sentinel range `0..=242`, and `@ENG` span bounds).
2. **Static Binary Verifier (`verify_binary_function` / `verify_u16_stream`)**
   - Enforces SSA register initialization, branch label resolution, capability bitmask inclusion (`(fn.cap_mask & spec.req_cap_mask) == spec.req_cap_mask`), linear memory arena ownership & borrow lifetimes (`0x7F01..=0x7F05`), and fixed-point dataflow taint tracking across all control-flow edges (`@UNTRUSTED` vs `@SAFE`).
3. **Core AOT Lowering (`compile_pure_u16_to_x86_64_elf_with_args`, `compile_pure_u16_to_wasm`, and Stage-1 `.omni_bc`)**
   - Lowers verified pure numeric/ternary/`@PACK5` functions to native `x86-64` ELF64 and WebAssembly while rejecting any host opcode (`0x7F64..=0x7F70`).

Everything outside this core—including `omni-agent-driver` repair heuristics, `format_omni_inspect`, and surface syntactic sugar—is **untrusted** because its output must pass `decode_omni_token_stream` and `verify_binary_function` before execution.

## 2. Five Compositional Soundness Lemmas (Verified in `run_compositional_invariant_selftests`)

1. **Lemma 1 — Branched Taint Non-Interference:**
   - For any function with branched control flow (`if_pos`, `if_neg`, `match_trit`, `while`), if any reachable branch assigns an `@untrusted` expression to a register `%r` that reaches `RET`, the fixed-point taint lattice joins `%r` as tainted and rejects the function with `error[TAINT_ON_RET]`. Conversely, dominating sanitization (`auth_verify`, `san_*`) clears taint along all dominated paths.
2. **Lemma 2 — Multi-Arena Linear Conservation:**
   - Every function accepted by `verify_binary_function` that allocates $k \ge 1$ linear arenas (`arena(N)`) must release all $k$ arenas after releasing any active borrows (`borrow_r` / `borrow_w`), terminating with `vm.allocated_bytes == 0`. Partial drops are rejected with `error[LINEAR_LEAK]`.
3. **Lemma 3 — Capability Monotonicity Across `OMNI_ISA_TABLE`:**
   - For every opcode `op_tok` in `0x7F00..=0x7F7F` where `spec.req_cap_mask != 0` (25 capability-gated opcodes), a function with `cap_mask == 0` containing `op_tok` is unconditionally rejected with `error[MISSING_CAP]`.
4. **Lemma 4 — Hot-Path AOT Observational Equivalence on 64-Bit Boundaries:**
   - For any single-function pure numeric/ternary module accepted by `is_module_aot_hot_path`, execution via `x86-64`/Wasm AOT and `OmniBinaryVm` produces identical results across boundary inputs `(0, 0)`, `(42, 0)` (totalized division/modulo by zero), `(i64::MIN, -1)` (wrapping overflow), `(i64::MAX, 1)`, and `(-99, 7)`.
5. **Lemma 5 — Module & Hygienic Macro Transparency:**
   - Hygienic macros cannot capture caller-scope variables (`error[UNDEF_VAR]`) and cannot conceal `@untrusted` sink violations (`error[TAINT_SINK]`) or missing capabilities (`error[MISSING_CAP]`).
