# Omni Next Abstraction Layer (`spec/next_abstraction.md`)

**Status:** Implemented & Verified (`13AA` / `6AC`, `stdlib_macro_pipeline_golden`)  
**Core ISA Impact:** Zero new opcodes (`0x7F00..=0x7F7F` unchanged)

## 1. Chosen Direction & Rationale

Omni combines two complementary zero-opcode abstractions that expand cleanly into the existing verified surface before binary lowering:

1. **Hygienic Expression Macros (`macro <name>(<p1>, <p2>, ...) = <expr>;`)**
   - Checked for lexical hygiene at declaration time (`check_macro_body_hygiene`): a macro body may only reference its formal parameters, preventing accidental capture of caller variables (`error[UNDEF_VAR]`).
   - Checked for exact arity at expansion time (`error[ARITY_MISMATCH]`).
   - Expanded AST nodes inherit the call-site source coordinates `(call_line, call_col)` so any downstream static verifier error (`TAINT_SINK`, `MISSING_CAP`, `BAD_TYPE`) points directly to the macro invocation in user code.

2. **Verified Built-In Standard Library Prelude (`include "std/prelude.omni";`)**
   - Resolved in-memory by `lookup_omni_stdlib_module` (`"std/prelude.omni"`, `"std/math.omni"`, `"std/trit.omni"`) without requiring external filesystem paths.
   - Provides `#[strict]` verified pure-Omni building blocks:
     - `std_clamp(x: i64, lo: i64, hi: i64) -> i64`
     - `std_abs_diff(a: i64, b: i64) -> i64`
     - `std_trit_mux(cond: trit, neg_v: i64, zero_v: i64, pos_v: i64) -> i64`
     - `std_consensus3(s0: trit, s1: trit, s2: trit) -> trit`

## 2. Verification & Soundness Guarantees

- **No Hidden Taint or Capabilities:** Because both `macro` expansion and `include "std/prelude.omni";` lower to standard `Vec<u16>` instruction packets prior to `verify_binary_function`, an `@untrusted` argument passed through a macro into `egate(...)` is still rejected with `error[TAINT_SINK]` (verified in Compositional Invariant Lemma 5).
- **4-Way Backend Equivalence:** Single-function hygienic macro programs qualify for the hot numeric AOT path and produce identical results across `OmniBinaryVm`, Host `x86-64` ELF64, Stage-1 `.omni_bc` ELF64 (bit-identical), and Wasm 64-bit AOT (`13AA`).
