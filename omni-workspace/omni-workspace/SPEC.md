# Omni Binary Token-Stream ISA, Verifier, VM & AOT Normative Specification (`SPEC.md`)

**Contract Status:** Normative — every behavior asserted by `./omni selftest` (`186` checks in `omni_rust_bootstrap.rs`) and `cargo run -p omni-cli -- selftest` (`145`/`146` differential checks across the 8-crate Cargo workspace) is specified below. Features not yet implemented in standalone AOT are explicitly listed in Section 6.

## 1. 16-Bit Binary Token Partition (`0x0000..=0x7FFF`)

Every Omni program is a sequence of 16-bit unsigned words (`u16`, serialized in little-endian byte order for `.omni_bc` files). The 32,768-token (`0x0000..=0x7FFF`) vocabulary is partitioned into disjoint ranges:

| Token Range | Symbolic Constant | Meaning |
|-------------|-------------------|---------|
| `0x0000..=0x7DFF` | `0x0000..=ENGLISH_MAX_TOKEN` | Natural English Lexical Bank (`@ENG`) — hardware-clamped (`tok & 0x7FFF < 0x7E00`) and tagged `@UNTRUSTED` |
| `0x7E00..=0x7EBF` | `REG_BASE..=REG_MAX` | 192 SSA Virtual Destination/Operand Registers (`%0..=%191`) |
| `0x7EC0..=0x7EDF` | `PAR_BASE..=PAR_MAX` | 32 Function Parameter Registers (`$0..=$31`) |
| `0x7EE0` | `TOK_MOD_BEGIN` | Module stream start sentinel |
| `0x7EE1` | `TOK_SCHEMA_DEF` | Relational/struct schema header (`schema_idx`, `field_count`, `(ty, tc)` pairs) |
| `0x7EE2` | `TOK_EXPERT_DEF` | `@PACK5` ternary expert header (`expert_id`, `rows`, `cols`, `packed_len`, `@PACK5` words) |
| `0x7EE3` | `TOK_FN_DEF` | Function header (`fn_idx`, `param_count`, params, `ret_ty`, `cap_mask`, `ins_count`) |
| `0x7EE4` | `TOK_INS_END` | Instruction packet terminator |
| `0x7EE5` | `TOK_IMM_I16` | Signed 16-bit immediate (`1` word follows, sign-extended to `i64`) |
| `0x7EE6` | `TOK_IMM_I32` | Signed 32-bit immediate (`2` words follow: `(hi16 << 16) \| lo16`, sign-extended to `i64`) |
| `0x7EE7..=0x7EE9` | `TOK_TRIT_NEG..=TOK_TRIT_POS` | Balanced ternary immediate constants (`-1=0x7EE7`, `0=0x7EE8`, `+1=0x7EE9`) |
| `0x7EEA` | `TOK_ENG_SPAN` | Untrusted English literal span header (`word_len` followed by `word_len` words in `0x0000..=0x7DFF`) |
| `0x7EEB` | `TOK_SYM_SCHEMA` | Schema symbol reference (`schema_idx` word follows) |
| `0x7EEC` | `TOK_SYM_FN` | Function symbol reference (`fn_idx` word follows) |
| `0x7EED` | `TOK_SYM_SKILL` | Skill/expert symbol reference (`skill_id` word follows) |
| `0x7EEE` | `TOK_FIELD_KV` | Key-value field pair (`field_id` word followed by nested operand) |
| `0x7EEF` | `TOK_MOD_END` | Module stream end sentinel |
| `0x7EF0..=0x7EFF` | `LBL_BASE..=LBL_MAX` | 16 Basic-Block Branch Target Labels (`LBL_BASE + label_id`, `label_id` in `0..=15`) |
| `0x7F00..=0x7F7F` | `OP_BASE..=OP_MAX` | 128 Opcodes (indexed in `OMNI_ISA_TABLE[op_tok - 0x7F00]`) |
| `0x7F80..=0x7FBF` | `TY_VOID..=TY_ERR` | 64 Binary Type Tags (`TY_VOID=0x7F80`, `TY_I64=0x7F81`, `TY_TRIT=0x7F83`, `TY_STR=0x7F84`, `TY_BYTES=0x7F86`, `TY_ARRAY=0x7F87`, `TY_TRIT_VEC=0x7F88`, `TY_STRUCT=0x7F89`, `TY_ARENA=0x7F8A`, `TY_DOM=0x7F8C`) |
| `0x7FC0..=0x7FFF` | `TC_BASE..=0x7FFF` | Packed Taint (`Bit 5 = 0x0020 = @UNTRUSTED`) + Capability ID (`Bits 0..=4`) |
| `0..=242` (`u8`) | `@PACK5` byte lane | 5 balanced trits per byte ($\sum_{k=0}^4 (t_k + 1)3^k \in [0, 242]$); `SENTINEL_SPECIAL_TOKEN = 243` (`0xF3`) is rejected by `pack5_decode` |

## 2. Wrapping 64-Bit Integer & Bitwise ALU (`0x7F06`, `0x7F10..=0x7F23`)

All integer values in the Omni VM, standalone `x86-64` ELF AOT compiler, Stage-1 self-hosting compiler, and standalone WebAssembly v1 AOT compiler are two's-complement signed 64-bit integers (`i64`) with identical wrapping/trap-free semantics across all four execution engines:

- **`CINT / MOV (0x7F06)`**: Loads a 64-bit integer register, parameter, or immediate into destination register `%dst`.
- **`IADD (0x7F10)`, `ISUB (0x7F11)`, `IMUL (0x7F12)`**: 64-bit wrapping addition (`wrapping_add`), subtraction (`wrapping_sub`), and multiplication (`wrapping_mul`).
- **`IDIV (0x7F13)` and `IMOD (0x7F14)` (Total Trap-Free Division/Remainder)**:
  - If `divisor == 0`: `IDIV(a, 0) == 0` and `IMOD(a, 0) == 0` (never traps with `SIGFPE` or Wasm `unreachable`).
  - If `dividend == i64::MIN` and `divisor == -1`: `IDIV(i64::MIN, -1) == i64::MIN` (`wrapping_div`) and `IMOD(i64::MIN, -1) == 0` (`wrapping_rem`).
  - Otherwise: standard signed 64-bit truncated quotient and remainder.
- **`INEG (0x7F15)` and `IABS (0x7F16)`**:
  - `INEG(x) = x.wrapping_neg()` (`INEG(i64::MIN) == i64::MIN`).
  - `IABS(x) = x.wrapping_abs()` (`IABS(-42) == 42`, `IABS(i64::MIN) == i64::MIN`).
- **`IMIN (0x7F17)` and `IMAX (0x7F18)`**: Signed 64-bit minimum (`a.min(b)`) and maximum (`a.max(b)`).
- **`BAND (0x7F1E)`, `BOR (0x7F1F)`, `BXOR (0x7F20)`, `BNOT (0x7F21)`**: 64-bit bitwise AND, OR, XOR, and one's complement (`!a`).
- **`SHL (0x7F22)` and `SHR (0x7F23)`**: Shift amount is masked to `b & 63` (`0..=63`); `SHL` performs 64-bit logical left shift (`wrapping_shl`), and `SHR` performs 64-bit arithmetic right shift (`wrapping_shr`).

## 3. Balanced Ternary Algebra, Ternary Decision Hooks & `SEL_TRIT` (`0x7F09`, `0x7F24..=0x7F3F`, `0x7F45`)

Balanced trits take values in $\{-1, 0, +1\}$. Any integer operand consumed as a trit is clamped via `v.clamp(-1, 1)`.

- **`CTRIT (0x7F09)`**: Loads a balanced trit constant (`-1`, `0`, `+1`) or register.
- **Relational Comparisons (`CEQ 0x7F24`, `CNE 0x7F25`, `CLT 0x7F26`, `CGT 0x7F27`)**: Compare two signed `i64` operands and return `+1` if true, `-1` if false.
- **Kleene & Consensus Logic (`TNOT 0x7F28`, `TAND 0x7F29`, `TOR 0x7F2A`, `TXOR 0x7F2B`, `TMAJ 0x7F2C`)**:
  - `TNOT(a) = -a`
  - `TAND(a, b) = min(a, b)`
  - `TOR(a, b) = max(a, b)`
  - `TXOR(a, b) = -(a * b)`
  - `TMAJ(a, b, c) = signum(a + b + c)` (`+1` if sum $> 0$, `-1` if sum $< 0$, `0` if sum $== 0$).
- **Three-Way Spaceship Comparison (`TCMP 0x7F2D`)**: `TCMP(a, b)` returns `-1` if $a < b$, `0` if $a == b$, and `+1` if $a > b$.
- **`@PACK5` Ternary Vector Codec & Dot Product (`PACK5 0x7F30`, `UNPACK5 0x7F31`, `TDOT 0x7F34`)**:
  - `PACK5(tvec)`: Encodes 5 balanced trits per byte (`0..242` using radix-3 $\sum_{k=0}^4 (t_k + 1)3^k$).
  - `UNPACK5(bytes, len)`: Unpacks `len` balanced trits in $\{-1, 0, +1\}$ from a `@PACK5` byte buffer.
  - `TDOT(v_a, v_b)`: Computes the exact ternary dot product $\sum_i a_i b_i$ as `i64`.
- **Ternary Decision Hooks (`EGATE 0x7F35`, `TLOGIC 0x7F3F`, `LOOKAHEAD 0x7F36`)**:
  - `EGATE (0x7F35)` (`egate(x)` / `decision_hook(x)`): Capability-gated (`CAP_ID_AI`) and taint-checked (`sink_requires_safe = true`) ternary decision hook. Accepts a `TritVec`, a `@PACK5` `Bytes` buffer, an `i64`, or a `trit`, sums its elements/value, and returns `OmniValue::Trit(+1)` if sum $> 0$, `OmniValue::Trit(-1)` if sum $< 0$, or `OmniValue::Trit(0)` if sum $== 0$.
  - `TLOGIC (0x7F3F)` (`tlogic(x)` / `decide_trit(x)`): Pure (`CAP_ID_PURE`) ternary decision hook with the same sign/majority reduction over `TritVec`, `@PACK5` `Bytes`, `i64`, or `trit`.
- **4-Operand Ternary Conditional Select (`SEL_TRIT / CMOV 0x7F45`)**:
  - Packet layout: `[%dst, 0x7F45, ty, tc, cond, val_neg, val_zero, val_pos, 0x7EE4]`.
  - Evaluates `c = cond.clamp(-1, 1)` and returns `val_neg` if $c < 0$, `val_zero` if $c == 0$, and `val_pos` if $c > 0$.

## 4. Control Flow, Inter-Function Calls, Byte Buffers & Stage-1 Self-Hosting (`0x7F40..=0x7F48`, `0x7F0D..=0x7F0F`, `0x7F50`)

- **Labels & Branches (`0x7F40..=0x7F44`)**:
  - `LABEL (0x7F40)`: Defines a basic-block target label `0x7FF0 + id` (`id` in `0..=15`) or `ImmInt(id)`.
  - `JMP (0x7F41)`: Unconditional jump to target label.
  - `BR_TRIT (0x7F42)`: 3-way branch `[0x7F42, cond, lbl_neg, lbl_zero, lbl_pos, 0x7EE4]` jumping to `lbl_neg` ($c < 0$), `lbl_zero` ($c == 0$), or `lbl_pos` ($c > 0$).
  - `BR_POS (0x7F43)`: Conditional jump to target label if `cond > 0` (`+1`).
  - `BR_NEG (0x7F44)`: Conditional jump to target label if `cond < 0` (`-1`).
  - Lowered by `OmniBinaryVm` (via $O(1)$ precomputed `label_pcs`), `compile_pure_u16_to_x86_64_elf_with_args` (two-pass `rel32`), `build_step7_stage1_compiler_stream` (two-pass `rel32`), and `compile_pure_u16_to_wasm` + `execute_wasm_module_i64` (structured Wasm `loop` + `if` + `br 1` dispatch on `pc_local`).
- **Inter-Function Calls (`CALL 0x7F48`)**:
  - Packet layout: `[%dst, 0x7F48, ret_ty, tc, TOK_SYM_FN, callee_fn_idx, arg0, arg1, ..., 0x7EE4]`.
  - Executed by `OmniBinaryVm` across multi-function modules (`BinModule.functions`), verifying each callee before execution and returning the callee's result into `%dst`.
- **Byte Buffers (`0x7F0D`, `0x7F0E`, `0x7F0F`, `0x7F50`)**:
  - `ANEW (0x7F0D)` (`anew(cap)`): Initializes an empty byte buffer (resets length counter to `0`).
  - `IGET (0x7F0E)` (`iget(buf, idx)`): Reads element at index from a `U16Array`, `Bytes` buffer, or `TritVec`.
  - `ISET (0x7F0F)` (`iset(buf, b0, ...)`, `iset_i32(buf, val)`, `iset_i64(buf, val)`, `iset_patch_i64(buf, off, val)`): Appends raw byte operands (`1..N` bytes), little-endian `i32` (`mode == -4`), or little-endian `i64` (`mode == -8`), or patches an existing little-endian `i64` at byte offset (`mode == -9`).
  - `SLEN (0x7F50)` (`slen(buf)`): Returns current byte/element length as `i64`.
- **Stage-1 == Stage-2 Self-Hosting Fixed Point**:
  - Compiling `omni_self_compiler.omni_bc` (`19,731` `u16` tokens = `39,462` bytes) via `compile_pure_u16_to_x86_64_elf_with_args` produces `omni_stage1_self_compiler.elf` (`106,242` bytes).
  - Executing `./omni_stage1_self_compiler.elf omni_self_compiler.omni_bc > omni_stage2_self_compiler.elf` produces an identical `106,242`-byte binary (`0` byte differences, `FNV-1a64 = 0xEAFC81F46B482FF8`).

## 5. Static Verifier, Resource Budgets & Malformed Stream Rejection

- **Bounds-Checked Decoder (`decode_omni_token_stream`)**: Rejects streams missing `0x7EE0 (TOK_MOD_BEGIN)`, truncated `TOK_SCHEMA_DEF`/`TOK_EXPERT_DEF`/`TOK_FN_DEF` headers, truncated SSA instruction headers, truncated immediate/span payloads, or missing `0x7EE4 (TOK_INS_END)` without panicking.
- **Static Verifier (`verify_binary_function`)**: Enforces in a single $O(n)$ pass:
  1. Valid opcode token in `0x7F00..=0x7F7F`.
  2. Canonical monotonic SSA destination register allocation (`%0, %1, ..., %k` with no gaps).
  3. Capability bitmask check: `(func.cap_mask & spec.req_cap_mask) == spec.req_cap_mask`.
  4. Linear ownership state machine (`Uninit`, `Owned`, `BorrowedR(n)`, `BorrowedW`, `Moved`, `Dropped`): rejects use-after-move/drop, dropping a handle with active reader borrows (`BorrowedR(n) > 0`), and returning at `0x7F4A (RET)` while any `0x7F01 (ARENA)` handle remains undropped.
  5. Taint analysis: parameters and English spans (`0x0000..=0x7DFF`) tagged `@UNTRUSTED` propagate taint to destination registers unless cleared by a sanitizer opcode (`0x7F57 SAN_SQL`, `0x7F58 SAN_HTML`, `0x7F59 SAN_SHELL`, `0x7F70 AUTH_VER`); any privilege sink (`spec.sink_requires_safe == true`, including `0x7F35 EGATE` and `0x7F63 DB_INS`) or function return (`0x7F4A RET`) receiving an `@UNTRUSTED` operand is rejected.
  6. Branch target resolution: any `JMP`, `BR_TRIT`, `BR_POS`, or `BR_NEG` targeting a label ID not defined by a `LABEL (0x7F40)` instruction in the same function is rejected.
- **VM Fuel & Memory Budgets (`OmniBinaryVm`)**:
  - Default step limit: `DEFAULT_VM_STEP_LIMIT = 50_000_000` instructions.
  - Default memory budget: `DEFAULT_VM_MEMORY_BUDGET_BYTES = 256 * 1024 * 1024` (`268,435,456` bytes), accommodating `64 MiB` (`0x0400_0000` = `67,108,864` bytes) zero-copy arenas in Step 1 and Corpus Seeds 1 & 2.
  - Custom budgets via `OmniBinaryVm::with_budgets(step_limit, memory_budget_bytes)` deterministically return `Err` when either budget is exceeded.

## 6. Explicit "Not Supported in Standalone AOT" List (VM-Only Opcodes)

The following high-level runtime opcodes (used in Corpus Seeds `1, 2, 3, 5, 7`) require host runtime services (in-memory relational tables, vector indexes, DOM trees, HTTP parsers, or skill registries). They are executed by `OmniBinaryVm` (including stateful cross-call `DB_INS -> TV_SRCH` persistence and `clear_tables()` reset) and are **explicitly rejected with `Err(...)` by standalone `x86-64` ELF AOT (`compile_pure_u16_to_x86_64_elf_with_args`) and standalone WebAssembly AOT (`compile_pure_u16_to_wasm`)**:

| Token Range | Opcodes | Standalone AOT Status |
|-------------|---------|-----------------------|
| `0x7F01`, `0x7F05` | `ARENA`, `DROP` | VM-only (linear heap arena lifetime simulation; rejected by standalone AOT) |
| `0x7F30..=0x7F3F` | `TV_PACK5`, `TV_UNPACK5`, `TV_DOT`, `EGATE`, `LOOKAHEAD`, `TLOGIC` | VM-only (`@PACK5` vector codec and ternary decision hooks; rejected by standalone AOT) |
| `0x7F46..=0x7F49`, `0x7F4B..=0x7F4F` | `CALL`, `TCALL`, `ITER`, `NEXT`, `TRY_T`, `YIELD`, `SPAWN`, `AWAIT`, `CHAN` | VM-only (multi-function/async control flow; rejected by standalone AOT) |
| `0x7F51..=0x7F56`, `0x7F5A..=0x7F5F` | `SCONC`, `SSLIC`, `SHASH`, `UUID4`, `JENC`, `JDEC`, `B64E`, `B64D`, `REGEX` | VM-only (string/JSON/crypto helpers; rejected by standalone AOT) |
| `0x7F60..=0x7F6B` | `T_MATMUL`, `T_RMSN`, `T_ROPE`, `DB_INS`, `DB_QRY`, `DB_UPD`, `DB_DEL`, `TV_SRCH`, `KV_GET`, `KV_SET`, `TX_BEG`, `TX_COM` | VM-only (database, KV, and `@PACK5` vector table ops; rejected by standalone AOT) |
| `0x7F6C..=0x7F7F` | `HTTP_SRV`, `REQ_PAR`, `RSP_JSON`, `RSP_HTML`, `AUTH_VER`, `RATELIM`, `TIME_MS`, `LOG`, `ENV_GET`, `FS_READ`, `DOM_NODE`, `DOM_ATTR`, `DOM_TEXT`, `DOM_EVT`, `DOM_MNT`, `WGPU`, `WAUDIO`, `EMBED`, `SK_LOAD`, `SK_EXEC` | VM-only (HTTP, auth, filesystem, DOM/WebGPU, and skill runtime ops; rejected by standalone AOT) |

## 7. Omni Surface Language (`omni-lang`) Lowering & Usability Contract

`compile_omni_surface_to_asm`, `compile_omni_surface_to_u16`, `compile_omni_surface_file_to_u16`, `compile_omni_surface_to_x86_64_elf`, `compile_omni_surface_to_wasm`, `execute_omni_surface_wasm_i64`, `resolve_omni_surface_includes`, `wrap_omni_surface_snippet`, and `execute_omni_surface_program` (implemented identically in `omni_rust_bootstrap.rs` and `crates/omni-lang`) compile human-readable `.omni` surface programs into verified `Vec<u16>` token streams with **zero new opcodes**:

1. **Function Headers, Capabilities & Taint Annotations**:
   - Optional attributes: `#[fn_id(N)]` (`N` in `0..=15`) sets the `TOK_FN_DEF` function ID; `#[cap(AI, DB_W, ...)]` sets the function's 16-bit `CapMask` (`PURE`, `DB_R`, `DB_W`, `NET`, `DOM`, `AI`, `VISION`/`SKILL`, `AUDIO`, `AUTH`, `FS`/`FS_R`/`FS_W`, `SYS`/`ENV`/`PROC`/`TIME`/`RAND`/`CRYPTO`).
   - Parameter taint annotations: `<name>: <ty> @untrusted` (or `untrusted_<ty>`) marks the parameter with `pack_tc(true, CAP_ID_PURE)`. Returning an unsanitized `@untrusted` value or passing it to a safe sink (`egate(x)`) is rejected by `verify_binary_function` (`Taint violation on RET` / `Sink opcode 0x7F35 received @UNTRUSTED`). Calling `egate(x)` without `#[cap(AI)]` is rejected by `verify_binary_function` (`Capability violation on opcode 0x7F35`).
   - Sanitizers: `san_sql(s)` (`0x7F57`), `san_dom(s)` (`0x7F58`), and `san_prm(s)` (`0x7F59`) clear `@untrusted` taint.
2. **Surface Decision Hook & `@PACK5` Trit-Vector Lowering**:
   - **Decision Hook**: `egate(x)` / `decision_hook(x)` lowers to `0x7F35 (OP_7F35_EGATE)` (requires `#[cap(AI)]` and safe operand) and `tlogic(x)` / `decide_trit(x)` lowers to `0x7F3F (OP_7F3F_TLOGIC)` (pure). Both evaluate a `trit_vec`, `@PACK5` byte buffer, `i64`, or `trit` in `OmniBinaryVm` and return a balanced `trit` in `{-1, 0, +1}`.
   - **Compile-Time `@PACK5` Lowering for Trit Vectors**: Surface trit vector literals `[+1t, -1t, 0t, +1t, +1t, ...]` pack 5 trits per byte at compile time via `pack5_encode` and lower to `ANEW 0` + `ISET <packed_bytes>` + `UNPACK5 <len>` (or `pack5_bytes([...])` for raw `@PACK5` bytes). A 10-trit literal occupies `2` `@PACK5` bytes (`[227, 219]`) instead of `80` bytes (`40x` denser than 10 `i64` words), and supports `tdot(v, w)`, `pack5(v)`, `unpack5(b, len)`, and `egate(v)`.
3. **Line & Column Error Diagnostics**:
   - Every token, expression, statement, and parameter carries 1-based `(line, col)` source coordinates.
   - The compiler reports `line <L>, col <C>: ...` for undefined variables, unknown intrinsics/functions, unknown types, immutable parameter mutation, unresolved `goto`/`br_*` label targets, and malformed `match_trit` arms.
4. **Multi-Function Programs & Module/File `include` Linker**:
   - Multiple `fn` definitions per `.omni` file lower to multi-function `FN_DEF <id>` blocks in a single `BinModule`, with inter-function calls `callee(a, b, ...)` lowered to `0x7F48 (OP_7F48_CALL)` (`CALL_I64`, `CALL_TRIT`, `CALL_BYTES`, `CALL_TRIT_VEC`, `CALL_STR` with `@fn<id>`).
   - `include "other.omni";` / `mod "other.omni";` recursively resolves and links external `.omni` files relative to the parent directory (or in-memory VFS) with deterministic circular-include rejection.
5. **Phase 4 Surface Ergonomics, Linear Memory Arenas, String Literals & Direct AOT**:
   - **Compound Assignment & Inclusive Comparisons**: `x += e`, `x -= e`, `x *= e` desugar to in-place SSA register mutation (`IADD`, `ISUB`, `IMUL`); `a <= b` lowers to `TNOT(CGT(a, b))` and `a >= b` lowers to `TNOT(CLT(a, b))`.
   - **Pre-Conditioned `while` Loop AOT Equivalence**: `while <cond> { <body> }` lowers to `LABEL`/`BR_POS`/`JMP` and executes identically across `OmniBinaryVm`, standalone `x86-64` ELF AOT, Stage-1 Self-Hosting Compiler, and 64-bit WebAssembly (`while_sum_even_golden(10) == 30`).
   - **Surface Linear Memory Arena & Borrow Checker**: `arena(cap)` (`0x7F01`), `borrow_r(a)` (`0x7F02`), `borrow_w(a)` (`0x7F03`), `move_val(a)` (`0x7F04`), and `drop(a)` (`0x7F05`) expose the linear ownership state machine directly to `.omni` programs, statically rejecting arena leaks (`bad_surface_arena_leak`) and active-borrow drops (`bad_surface_borrow_conflict`).
   - **Surface String Literals & `@ENG` Hardware Taint**: String literals `"..."` lower to `CSTR@UNTRUSTED @str:<hex>` (`0x7F08` + `TOK_ENG_SPAN 0x7EEA` with tokens clamped to `0x0000..=0x7DFF` and tagged `@UNTRUSTED`). Unsanitized string literal returns are statically rejected (`bad_unsanitized_str_lit`), while sanitized pipelines (`san_prm` -> `s2i` / `i2s` / `scat` / `slen` / `fnv1a`) pass verification and execute in `OmniBinaryVm` (`str_pipeline_golden(45) == 130`).
   - **Direct Surface-to-AOT & Runner Subcommands**: `compile-elf`, `compile-wasm`, `run-wasm`, `run`, `eval` (with automatic return-type inference for bare expressions via `wrap_omni_surface_snippet`), and `repl`.
6. **Bit-Exact `Vec<u16>` Equivalence**:
   - Across all 15 original goldens (`seed0_risk` through `byte_buf_golden`), `compile_omni_surface_to_u16` continues to emit the **exact same `Vec<u16>`** as `assemble_omni_text_to_u16`.
