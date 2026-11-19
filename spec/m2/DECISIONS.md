# Milestone 2 — Decision Record

Status: **recorded 2026-11-10** (Caleb Peacock, relayed by Boss Lady); **updated 2026-11-14** to match
`HANDOFF.md` §6 on `docs/handoff` (O-2/O-3/O-4/O-5/O-7 decided, approved multi-mode arena layout,
label cap 16, M2-10 owner). Where this file and `HANDOFF.md` disagree, `HANDOFF.md` wins.
Scope: the `pure/m2/` two-pass u16 → ELF64 compiler written in Omni.
Companion plan: [`PLAN.md`](./PLAN.md).

## Decided (D-1 .. D-14)

| # | Decision | Where it lands |
|---|---|---|
| D-1 | 3-operand `RET buf, off, len` (byte-slice return) | M2-06 (VM, x86-64, Wasm, both verifiers, frontend `ret_slice`) |
| D-2 | `IGET` byte mode `-1` | C1 shape set (`isa/multibyte-loads`); M2 consumes it via `ld_u8` (M2-05) |
| D-3 | u32 label-table entries (`code_off + 1`; `0` = undefined) | M2-09 / M2-11 / M2-14 |
| D-4 | Native multi-function ABI, 1 MiB stack arena (`rbx` = arena base, stack args at `[rbp+16+8k]`) | M2-09 |
| D-5 | `pure/compiler.omni` stays frozen; the new compiler lives in `pure/m2/` | all commits; `PURE_OMNI_CORE_SUITE_MODULES` untouched |
| D-6 | Wasm `CALL` required for M2 sign-off | M2-10 (**owner: Ember Runtime**, starts after ISA-A), fixed-point witness W |
| D-7 (was O-3) | Multi-mode input window **128 KiB** (0x20000 B = 65,536 words). Input larger than the window is a **hard reject: nonzero exit plus a clear error, never silent truncation** (today's single-mode path truncates silently and exits 0; that is the failure this guards). Sizing: Scarlett measured 11.5 u16 words per source line (`verifier_engine.omni` 11,893 w / 1,034 lines), so an M3 past ~5,700 lines overflows 128 KiB and must be rejected, not cut. | M2-04 `m2d_window_ok(n)` (predicate only); enforced in M2-09 (native: `T_TRUNC` exit, no output; golden `mf_input_trunc` = 128 KiB + 1) and M2-11 `m2_init` (guest: error, nonzero exit) |
| D-8 | Multi-mode arena layout (approved): tokens `0x00000-0x1FFFF` (128 KiB), label table `0x20000-0x23FFF` (4096 × u32), register state `0x24000-0x27FFF`, output `0x28000-0x7FEFF`, decoder scratch `0x7FF00-0x7FFFF` (D-14) (M3 band `0x80000-0xFFFFF`; host runtime `0x100000-0x2FFFFF`). The scratch page is carved from the top of the output region, so output ends below `0x7FF00` rather than `0x80000`. **Single mode is unchanged byte-for-byte** (`selftest_m2_single_mode_byte_lock`). | M2-04 constants (`pure/m2/decode.omni` `m2l_*`), M2-11 |
| D-9 (was O-5) | Native IMOD stale-`rdx`: template left untouched, M2 rejects it via lint L-7 (no Stage-1 hash bump) | M2-03 |
| D-10 (was O-7) | Multi-function modules only through the new explicit entry point; no silent auto-route of `functions.len() > 1` | M2-09 |
| D-11 (was O-4) | Label ids `>= 4080` (outside `0x7EF0..0x7EFF`) are rejected in M2 now; a Rust spec'd reject comes later | M2-12+ |
| D-12 (was O-2) | Wasm bytes result is checked via an in-guest hash (FNV-1a64 + len) | M2-10 |
| D-13 | **Opcode metadata has exactly one copy: `pure/isa/table.omni`** (Raven Encoder, `f600102`, rodata at logical `0x81000` = `isa_band_base + isa_rodata_rel`, M3 band, read-only for M2). **`m2/groundwork` depends on `f600102`** (merged with `--no-ff`; it is based on `d2d6c13` like M2). M2 keeps no metadata copy. The decoder's accept/reject stays exactly the Rust decoder's (`decode_omni_token_stream` does not consult opcode metadata); `m2d_decode_at_ws` publishes the table's record for the opcode as a meta word. Caveat: the table's byte accessors use 2-op `IGET` on bytes, which lint L-4 forbids because native backends read the input there (A:345-347); so the meta word is **VM-only until C1's `IGET -1` (D-2) lands**. `m2-lint` reports `pure/isa/table.omni` functions as foreign notes (not gated; the table has its own budget lock, selftest 13BA). **TODO (on C1):** move the table accessors to `IGET -1`, then gate them; M2-09+ native paths must not rely on the meta word before that. | M2-04 (`m2d_op_meta`, `M2D_SCRATCH_META`); C1 follow-up with Raven |
| D-14 | Shared decoder interface (for Raven's encoder, M3-04): `pure/m2/decode.omni` + `pure/m2/util.omni`; link order util, `pure/isa/table.omni`, decode. Entry `m2d_decode_at(arr, n, off) -> next` (pure; next word offset or `-(code·2^20 + off)`), and `m2d_decode_at_ws(arr, n, off, ws) -> ws`, which writes i64 LE words at the fixed scratch offsets **`M2D_SCRATCH_OP = 0x7FF00`** (op token), **`M2D_SCRATCH_NOPS = 0x7FF08`** (operand count, -1 on error), **`M2D_SCRATCH_NEXT = 0x7FF10`** (next offset or error), **`M2D_SCRATCH_META = 0x7FF18`** (table `flags \| cap << 8`, or -1); `0x7FF20-0x7FFFF` reserved. Compile-time constants `m2l_scratch_op/nops/next/meta()` (Omni) and `M2D_SCRATCH_*` (`m2_host/decode_diff.rs`). Operand kind codes `m2k_reg..m2k_field_kv()` = 1 REG, 2 PARAM, 3 LABEL, 4 TRIT, 5 IMM, 6 ENG_SPAN, 7 SCHEMA_REF, 8 FN_REF, 9 SKILL_REF, 10 FIELD_KV; `f600102` defines no operand kind codes (its records are mnemonics and opcodes only, `pure/isa/table.omni:1-2`), so there is nothing to match. Sign helpers `m2u_sext16`, `m2u_sext32`. Names carry the `m2d_`/`m2u_`/`m2k_` prefix (not bare `decode_at`) so concatenated modules can't collide. | M2-04 |

## Still open — options only, NOT decided

O-2, O-3, O-4, O-5 and O-7 were decided after M2-00 was first committed (see D-7..D-12 above).

| # | Question | Options |
|---|---|---|
| O-1 | Native trap exit codes | per-kind codes (`T_TRUNC`, `T_OOB`, `T_CAP`) vs one code |
| O-6 | Pack5 dynamic-index bloat | leave vs fix with golden byte bump |
| O-8 | `validate_func_pack5_ops` monolith/workspace divergence | pick one semantics in M2-07 |

## Standing invariants (every M2 commit)

- 192 SSA registers / **16 labels** per function (ids 0..15; both assembly-built and frontend-compiled after C4). M2 code targets ≤ 120 SSA / ≤ 12 labels.
- Call depth ≤ 64 frames; no recursion in `.omni`; eager argument evaluation.
- No heap structs, maps, or GC in `.omni`.
- The 128-entry opcode table (`0x7F00..0x7F7F`) is never renumbered.
- Frozen bootstrap hashes unchanged (enforced by `selftest_m2_frozen_hashes`, M2-01, and `13BF` C1):
  - Phase D `omni_self_compiler.omni_bc`: 159 words, FNV-1a64 `0x23B8D17257CCA9C4`
  - Phase D ELF: 824 B, `0xE221085777B8AC47`
  - 18-module pure core suite stream: 14,559 words, `0x78462AE4C81DF408`
  - Stage-1 self-compiler ELF: 106,242 B, `0xEAFC81F46B482FF8`
  - Stage-1 bytecode `omni_self_compiler.omni_bc`: 19,731 words / 39,462 B, `0x677F57AE71D28FED`
- Zero external crates.
