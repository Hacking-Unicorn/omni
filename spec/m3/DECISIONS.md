# M3 decision record

Recorded in M3-00 (`6234c0c`). **Updated 2026-11-14** to match `HANDOFF.md` §6/§8 on `docs/handoff` (`aa624b7`), which is the authority where the two disagree. Nothing here is new policy.

## Settled (from Boss Lady / Caleb; treated as fixed)

| # | Decision |
|---|---|
| S-1 | M3 Omni code lives in `pure/m3/`. M2 lives in `pure/m2/`. `pure/compiler.omni` is **frozen**, and so are `PURE_OMNI_CORE_SUITE_MODULES` and the 14,559-word suite (`0x78462AE4C81DF408`). New `.omni` modules are never added to the suite list or the embedded stdlib literals before the planned re-freeze. |
| S-2 | The label cap is **16** per function everywhere (ids 0..15). The one named legacy exemption is the **Rust-built Stage-1 stream**. M3 takes no exemption. |
| S-3 | **C4** (main line) fixes the source compiler, which numbers labels from 1 (`omni_rust_bootstrap.rs:3587`, `next_lbl: 1`; workspace `compiler/mod.rs:1585`) and rejects the 16th with `BUDGET_EXCEEDED` (`:3646-3656`; workspace `:1644-1654`). The emitter is fine (Vera, `lbl16_asm_ids_0_15`). **Until C4 lands, frontend-compiled functions get 15 labels.** Every M3 function is therefore written for ≤ 15 labels (target ≤ 12) and ≤ 192 SSA (target ≤ 120). The M3-01 lock pins the boundary: `q8d_label_15` compiles, `q8b_label_limit` (16) is rejected and must flip in the C4 commit, `q8e_label_17` stays rejected. |
| S-4 | No recursion in core `.omni`. Call depth ≤ 64. Eager args. No heap structs, maps or GC (flat linear arena; compile-time offsets and an arena-backed sorted array with binary search are allowed). The opcode table `0x7F00..0x7F7F` is never renumbered. Frozen hashes don't move except at the single planned re-freeze after the M2 fixed point. Zero external crates. |
| S-5 | Scarlett's scope on `scarlett/m3-groundwork`: **M3-00, M3-01, M3-02, M3-16**, plus the monolith seed-vs-duplicate map (no monolith cutting until ISA-B and C land). Each is one commit, with the full guard green first. **M3-03 / M3-04 (shared ISA table and packet encoder) belong to Raven Encoder**, built on Nyx's M2-04 decoder in `pure/m2/`; M3 only consumes them read-only. |
| S-6 | Arena bands: M2 below `0x80000` (multi mode: tokens `0x00000-0x20000`, labels `0x20000`, register state `0x24000`, output `0x28000`, 128 KiB window); **M3 `0x80000-0xFFFFF`**; **host runtime `0x100000-0x17FFFF`** (Ember). |
| S-7 | KV/DB (Ember's lane; recorded so M3 never contradicts it): an arena sorted array with binary search is allowed; `DB_QRY` returns **-1** on a miss, never `key*10`. |
| S-8 | The M3 rodata block at relative `0x01000` (4 KiB) holds Raven's 128 × 8 B opcode table and 81 real mnemonics × 16 B (`pure/isa/table.omni` at `raven/isa-core-port` `bf7f212`, approved by Boss Lady; not the original "128 × 16 B" plan). Mnemonic ids 0..80 are frozen (13BA golden list at `bf7f212`; new ids only appended at 81+). Free intrinsic room is (4096 − 81×16 − 128×8) / 16 = 111 slots, with a floor of 110 (one slot of headroom). M3-11 must count its exact intrinsic names before coding. If there are more than 111, the overflow moves to a new rodata block, decided in DECISIONS. Vera's gate caveat: only the workspace copy of drift selftest 13BA reads the real `omni-isa/src/lib.rs:112-286` and `omni-assembler` crates; the monolith copy checks against mirror tables inside the monolith. |
| S-9 | ISA-B is strict bounds (Boss Lady, 2026-11-14). An out-of-bounds IGET or ISET is a deterministic runtime error, identical on all four backends. It never reads as 0 and is never a silent no-op. Recorded so M3 docs and goldens never claim otherwise. |

Frozen values the guard checks on every commit:
- Phase D `.omni_bc`: 159 words, `0x23B8D17257CCA9C4`
- Phase D ELF: 824 B, `0xE221085777B8AC47`
- Suite: 14,559 words, `0x78462AE4C81DF408`
- Stage-1 ELF: 106,242 B, `0xEAFC81F46B482FF8`
- Stage-1 bytecode `omni_self_compiler.omni_bc`: 19,731 words / 39,462 B, `0x677F57AE71D28FED`

## Closed since M3-00

| # | Was | Now |
|---|---|---|
| O-1 | 15 or 16 labels? | **16** (S-2/S-3); 15 for frontend-compiled code until C4. |
| O-2 | Hoist shared ISA/decoder code, or include `pure/m2/` read-only? | The shared decoder is Nyx's M2-04 in `pure/m2/`; Raven's ISA table/encoder build on it. M3 includes them read-only. Raven picks the module path. |
| O-10 | M2's input window | **128 KiB** (M2 D-7), which fits the measured 34.5k-57.5k-word M3 image. |

## Open (options only; not decided)

| # | Question |
|---|---|
| O-3 | Do linear-scan symbol tables with fnv32 compare keys satisfy "no hash maps"? Not ruled on directly. The nearest ruling is KV/DB (HANDOFF §6.7): an arena sorted array with binary search is allowed. A linear scan with no buckets is the more conservative option, and that is what the plan uses. |
| O-4 | Source input encoding: `Bytes` on the VM vs NUL-padded u16 words natively |
| O-5 | When to port include resolution to Omni (via a host-provided source bundle) |
| O-6 | New M3 capacity limits (nesting 512, 1024 fns, 128 macros, pools) that the Rust frontend doesn't have |
| O-7 | Error parity on `(class, line, col)` only, or exact message text too? (M3-01 locks `(class, line, col)` only.) |
| O-8 | Port the `emit_asm` text mode, or keep it as Rust tooling? |
| O-9 | Retire the monolith mirror before the M3 lockstep (M3-14)? |
| O-11 | M3-02 gate: `m2-lint` and the M2-08 call-depth check are not on any shipped branch yet (see `spec/m3/GATE.md`). The gate fails closed until they land. Reuse Nyx's tools when they ship, rather than writing a second lint. |
