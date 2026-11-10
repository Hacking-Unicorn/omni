# Milestone 2 — Decision Record

Status: **recorded 2026-11-10** (Caleb Peacock, relayed by Boss Lady); **updated 2026-11-14** to match
`HANDOFF.md` §6 on `docs/handoff` (O-2/O-3/O-4/O-5/O-7 decided, approved multi-mode arena layout,
label cap 16, M2-10 owner). Where this file and `HANDOFF.md` disagree, `HANDOFF.md` wins.
Scope: the `pure/m2/` two-pass u16 → ELF64 compiler written in Omni.
Companion plan: [`PLAN.md`](./PLAN.md).

## Decided (D-1 .. D-12)

| # | Decision | Where it lands |
|---|---|---|
| D-1 | 3-operand `RET buf, off, len` (byte-slice return) | M2-06 (VM, x86-64, Wasm, both verifiers, frontend `ret_slice`) |
| D-2 | `IGET` byte mode `-1` | C1 shape set (`isa/multibyte-loads`); M2 consumes it via `ld_u8` (M2-05) |
| D-3 | u32 label-table entries (`code_off + 1`; `0` = undefined) | M2-09 / M2-11 / M2-14 |
| D-4 | Native multi-function ABI, 1 MiB stack arena (`rbx` = arena base, stack args at `[rbp+16+8k]`) | M2-09 |
| D-5 | `pure/compiler.omni` stays frozen; the new compiler lives in `pure/m2/` | all commits; `PURE_OMNI_CORE_SUITE_MODULES` untouched |
| D-6 | Wasm `CALL` required for M2 sign-off | M2-10 (**owner: Ember Runtime**, starts after ISA-A), fixed-point witness W |
| D-7 (was O-3) | Multi-mode input window **128 KiB** (0x20000 B = 65,536 words) | M2-09 stub, M2-11 `m2_init` |
| D-8 | Multi-mode arena layout (approved): tokens `0x00000-0x1FFFF` (128 KiB), label table `0x20000-0x23FFF` (4096 × u32), register state `0x24000-0x27FFF`, output `0x28000` up to **below `0x80000`** (M3 band `0x80000-0xFFFFF`; host runtime `0x100000-0x17FFFF`). **Single mode is unchanged byte-for-byte** (`selftest_m2_single_mode_byte_lock`). | M2-04 constants (`pure/m2/decode.omni` `m2l_*`), M2-11 |
| D-9 (was O-5) | Native IMOD stale-`rdx`: template left untouched, M2 rejects it via lint L-7 (no Stage-1 hash bump) | M2-03 |
| D-10 (was O-7) | Multi-function modules only through the new explicit entry point; no silent auto-route of `functions.len() > 1` | M2-09 |
| D-11 (was O-4) | Label ids `>= 4080` (outside `0x7EF0..0x7EFF`) are rejected in M2 now; a Rust spec'd reject comes later | M2-12+ |
| D-12 (was O-2) | Wasm bytes result is checked via an in-guest hash (FNV-1a64 + len) | M2-10 |

## Still open — options only, NOT decided

O-2, O-3, O-4, O-5 and O-7 were decided after M2-00 was first committed (see D-7..D-12 above).

| # | Question | Options |
|---|---|---|
| O-1 | Native trap exit codes | per-kind codes (`T_TRUNC`, `T_OOB`, `T_CAP`) vs one code |
| O-6 | Pack5 dynamic-index bloat | leave vs fix with golden byte bump |
| O-8 | `validate_func_pack5_ops` monolith/workspace divergence | pick one semantics in M2-07 |

## Standing invariants (every M2 commit)

- 192 SSA registers / **16 labels** per function (ids 0..15). The emitter, both verifiers and all four
  backends accept 16 labels in assembly-built functions (Vera, `lbl16_asm_ids_0_15`). The Rust
  **source compiler** still numbers labels from 1 (`omni_rust_bootstrap.rs:3587`, `next_lbl: 1`) and
  rejects the 16th (`:3646-3656`), so **frontend-compiled functions are capped at 15 until main-line
  C4 lands**. The M2-03 budget report prints both caps. M2 code targets ≤ 120 SSA / ≤ 12 labels.
- Call depth ≤ 64 frames; no recursion in `.omni`; eager argument evaluation.
- No heap structs, maps, or GC in `.omni`.
- The 128-entry opcode table (`0x7F00..0x7F7F`) is never renumbered.
- Frozen bootstrap hashes unchanged (enforced by `selftest_m2_frozen_hashes`, M2-01):
  - Phase D `omni_self_compiler.omni_bc`: 159 words, FNV-1a64 `0x23B8D17257CCA9C4`
  - Phase D ELF: 824 B, `0xE221085777B8AC47`
  - 18-module pure core suite stream: 14,559 words, `0x78462AE4C81DF408`
  - Stage-1 self-compiler ELF: 106,242 B, `0xEAFC81F46B482FF8`
  - Stage-1 bytecode `omni_self_compiler.omni_bc`: 19,731 words, `0x677F57AE71D28FED` (checked by `fixed-point`)
- Zero external crates.
