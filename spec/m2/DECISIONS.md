# Milestone 2 — Decision Record

Status: **recorded 2026-11-10** (Caleb Peacock, relayed by Boss Lady).
Scope: the `pure/m2/` two-pass u16 → ELF64 compiler written in Omni.
Companion plan: [`PLAN.md`](./PLAN.md).

## Decided (D-1 .. D-6)

| # | Decision | Where it lands |
|---|---|---|
| D-1 | 3-operand `RET buf, off, len` (byte-slice return) | M2-06 (VM, x86-64, Wasm, both verifiers, frontend `ret_slice`) |
| D-2 | `IGET` byte mode `-1` | C1 shape set (`isa/multibyte-loads`); M2 consumes it via `ld_u8` (M2-05) |
| D-3 | u32 label-table entries (`code_off + 1`; `0` = undefined) | M2-09 / M2-11 / M2-14 |
| D-4 | Native multi-function ABI, 1 MiB stack arena (`rbx` = arena base, stack args at `[rbp+16+8k]`) | M2-09 |
| D-5 | `pure/compiler.omni` stays frozen; the new compiler lives in `pure/m2/` | all commits; `PURE_OMNI_CORE_SUITE_MODULES` untouched |
| D-6 | Wasm `CALL` required for M2 sign-off | M2-10, fixed-point witness W |

## Still open (O-1 .. O-8) — options only, NOT decided

| # | Question | Options |
|---|---|---|
| O-1 | Native trap exit codes | per-kind codes (`T_TRUNC`, `T_OOB`, `T_CAP`) vs one code |
| O-2 | Wasm bytes-result channel | evaluator API returning `memory[off..off+len]` vs in-guest FNV + len |
| O-3 | Multi-mode input window | 0x10000 B (32,768 words) vs 0x20000 B |
| O-4 | Label ids outside the M2 slot domain (≥ 4080, outside `0x7EF0..0x7EFF`) | M2-only reject vs Rust spec'd reject (multi mode / both modes) |
| O-5 | Native IMOD stale-`rdx` fix | leave + lint (needs no frozen-hash change) vs deliberate Stage-1 hash bump |
| O-6 | Pack5 dynamic-index bloat | leave vs fix with golden byte bump |
| O-7 | Routing of multi-function modules in AOT | explicit new entry point vs auto-route `functions.len() > 1` |
| O-8 | `validate_func_pack5_ops` monolith/workspace divergence | pick one semantics in M2-07 |

## Standing invariants (every M2 commit)

- 192 SSA registers / 15 usable labels per function (M2 code targets ≤ 120 SSA / ≤ 12 labels).
- Call depth ≤ 64 frames; no recursion in `.omni`; eager argument evaluation.
- No heap structs, maps, or GC in `.omni`.
- The 128-entry opcode table (`0x7F00..0x7F7F`) is never renumbered.
- Frozen bootstrap hashes unchanged (enforced by `selftest_m2_frozen_hashes`, M2-01):
  - Phase D `omni_self_compiler.omni_bc`: 159 words, FNV-1a64 `0x23B8D17257CCA9C4`
  - Phase D ELF: 824 B, `0xE221085777B8AC47`
  - 18-module pure core suite stream: 14,559 words, `0x78462AE4C81DF408`
  - Stage-1 self-compiler ELF: 106,242 B, `0xEAFC81F46B482FF8`
- Zero external crates.
