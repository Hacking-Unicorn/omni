# Milestone 2 — Decision Record

Status: **recorded 2026-11-10** (Caleb Peacock, relayed by Boss Lady) by Nyx Fixpoint in M2-00 (`cd2b94d` on `nyx/m2-foundation`). **Updated 2026-11-14** by Scarlett Truth to match `HANDOFF.md` §6 on `docs/handoff` (`aa624b7`), which is the authority where the two disagree. Decisions made in chat after M2-00 are folded in below; nothing here is new policy.
Scope: the `pure/m2/` two-pass u16 → ELF64 compiler written in Omni.
Companion plan: [`PLAN.md`](./PLAN.md) (on `nyx/m2-foundation`).

> This file is created on `scarlett/m3-groundwork` because that branch is based on `d2d6c13`, which has no `spec/m2/`. Its D-1..D-6 table and invariants are copied from Nyx's M2-00 text so that a later merge with `nyx/m2-foundation` only differs in the updated rows.

## Decided (D-1 .. D-6)

| # | Decision | Where it lands |
|---|---|---|
| D-1 | 3-operand `RET buf, off, len` (byte-slice return) | M2-06 (VM, x86-64, Wasm, both verifiers, frontend `ret_slice`) |
| D-2 | `IGET` byte mode `-1` | C1 shape set (`isa/multibyte-loads`); M2 consumes it via `ld_u8` (M2-05) |
| D-3 | u32 label-table entries (`code_off + 1`; `0` = undefined) | M2-09 / M2-11 / M2-14 |
| D-4 | Native multi-function ABI, 1 MiB stack arena (`rbx` = arena base, stack args at `[rbp+16+8k]`) | M2-09 |
| D-5 | M2 lives in **`pure/m2/`**. `pure/compiler.omni` stays **frozen** (stub kernel; never edited) | all commits; `PURE_OMNI_CORE_SUITE_MODULES` untouched |
| D-6 | Wasm `CALL` required for M2 sign-off | M2-10 (handed to Ember Runtime; starts after ISA-A), fixed-point witness W |

## Decided after M2-00 (HANDOFF §6.5, §6.6, §6.3)

| # | Decision |
|---|---|
| D-7 (was O-3) | Multi-mode input window is **128 KiB** (`0x20000` B). Multi-mode arena layout: tokens `0x00000-0x20000`, labels at **`0x20000`**, register state at **`0x24000`**, output at **`0x28000`**. Single mode keeps the original layout (tokens `0x00000`, labels `0x10000`, register state `0x14000`, output `0x18000`; `spec/m3/PLAN.md` §3) and stays byte-identical to today. |
| D-8 (was O-5) | IMOD template left untouched; IMOD is rejected via lint (avoids a Stage-1 hash bump). |
| D-9 (was O-7) | Multi-function modules go only through the new explicit entry point; no silent auto-route on `functions.len() > 1`. |
| D-10 (was O-4) | Label ids `>= 4080` are rejected in M2 now; the Rust-side rule is to be spec'd later. |
| D-11 (was O-2) | Wasm results are checked via an in-guest hash. |
| D-12 | Label resolution copies Rust exactly: first definition wins; undefined labels are rejected. |
| D-13 | **Label cap is 16** per function (ids 0..15). The emitter/verifier already accept 16 (Vera, `lbl16_asm_ids_0_15`). The source compiler numbers labels from 1 (`omni_rust_bootstrap.rs:3587`, `next_lbl: 1`) and rejects the 16th (`:3646-3656`); fixing that is **C4**. Until C4 lands, **frontend-compiled** functions get **15**; assembly-built / M2-hand-built functions (label ids from 0) get 16. This supersedes the "15 usable labels" sentence in M2-00. |
| D-14 | The one named legacy exemption from the label cap is the **Rust-built Stage-1 stream** (about 83 IMM labels). M2 and M3 code take no exemption. |
| D-15 | Arena bands: M2 stays below `0x80000` (output starts at `0x28000`, 352 KiB of room); **M3 `0x80000-0xFFFFF`**; **host runtime (Ember) `0x100000-0x17FFFF`**. |
| D-16 | Shared decoder: the token decoder is M2-04 in `pure/m2/`. Raven Encoder (M3-03/M3-04, ISA table and packet encoder) builds on it; nobody writes a second decoder. |
| D-17 | Hash policy: re-freeze **once**, deliberately, after the M2 fixed point (stage2 == stage3). Until then, stop and report before any change that would move a frozen hash. |

## Still open — options only, NOT decided

| # | Question | Options |
|---|---|---|
| O-1 | Native trap exit codes | per-kind codes (`T_TRUNC`, `T_OOB`, `T_CAP`) vs one code |
| O-6 | Pack5 dynamic-index bloat | leave vs fix with golden byte bump |
| O-8 | `validate_func_pack5_ops` monolith/workspace divergence | pick one semantics in M2-07 |

O-2, O-3, O-4, O-5 and O-7 are closed above (D-7..D-11).

## Related decisions outside M2 (recorded here so M2 code does not contradict them)

- KV/DB: an arena-backed **sorted array with binary search** is allowed under the no-maps rule. `DB_QRY` returns **-1** on a miss (a documented not-found value, never `key*10`); `route_db_query_golden` gets a real hit case. Stop and report if that change would move a frozen hash. (HANDOFF §6.7; Ember Runtime's lane.)
- Lockstep overhead policy is deferred to Caleb (HANDOFF §6.2).

## Standing invariants (every M2 commit)

- 192 SSA registers / **16 labels** per function (15 for frontend-compiled code until C4; M2 code targets ≤ 120 SSA / ≤ 12 labels).
- Call depth ≤ 64 frames; no recursion in `.omni`; eager argument evaluation.
- No heap structs, maps, or GC in `.omni` (flat linear arena; compile-time offsets and an arena sorted array with binary search are allowed).
- The 128-entry opcode table (`0x7F00..0x7F7F`) is never renumbered.
- Frozen bootstrap hashes unchanged (enforced by `selftest_m2_frozen_hashes`, M2-01, on `nyx/m2-foundation`; C1 is the planned main-line pin):
  - Phase D `omni_self_compiler.omni_bc`: 159 words, FNV-1a64 `0x23B8D17257CCA9C4`
  - Phase D ELF: 824 B, `0xE221085777B8AC47`
  - 18-module pure core suite stream: 14,559 words, `0x78462AE4C81DF408`
  - Stage-1 self-compiler ELF: 106,242 B, `0xEAFC81F46B482FF8`
  - Stage-1 bytecode `omni_self_compiler.omni_bc`: 19,731 words / 39,462 B, `0x677F57AE71D28FED`
- Zero external crates.
