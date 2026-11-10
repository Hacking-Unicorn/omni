# M3 decision record

## Settled (from Boss Lady / Caleb; treated as fixed)

| # | Decision |
|---|---|
| S-1 | M3 Omni code lives in `pure/m3/`. `pure/m2/` belongs to M2. `pure/compiler.omni`, `PURE_OMNI_CORE_SUITE_MODULES`, and the 14,559-word suite (`0x78462AE4C81DF408`) are frozen. |
| S-2 | The 16-label cap applies everywhere. The one named legacy exemption is the Rust-built Stage-1 stream. M3 takes no exemption. |
| S-3 | Until O-1 is decided, every M3 function uses at most **15 labels** (target 12) and at most 192 SSA (target 120). |
| S-4 | No recursion in core `.omni`. Call depth ≤ 64. Eager args. No heap structs, maps or GC. The opcode table `0x7F00..0x7F7F` is never renumbered. Frozen hashes don't move. Zero external crates. |
| S-5 | Groundwork scope on `scarlett/m3-groundwork`: M3-00..M3-03 and M3-16 only. Each is one commit, with the full guard green before each commit. |

Frozen values the guard checks on every commit:
- Phase D `.omni_bc`: 159 words, `0x23B8D17257CCA9C4`
- Phase D ELF: 824 B, `0xE221085777B8AC47`
- Suite: 14,559 words, `0x78462AE4C81DF408`
- Stage-1 ELF: 106,242 B, `0xEAFC81F46B482FF8`

## Open (options only; not decided)

| # | Question |
|---|---|
| O-1 | Usable label cap: 15 (ids 1..15, Nyx's emitter measurement) or 16? |
| O-2 | Hoist shared ISA/decoder code to a common directory, or have M3 include `pure/m2/` read-only? |
| O-3 | Do linear-scan symbol tables with fnv32 compare keys satisfy "no hash maps"? |
| O-4 | Source input encoding: `Bytes` on the VM vs NUL-padded u16 words natively |
| O-5 | When to port include resolution to Omni (via a host-provided source bundle) |
| O-6 | New M3 capacity limits (nesting 512, 1024 fns, 128 macros, pools) that the Rust frontend doesn't have |
| O-7 | Error parity on `(class, line, col)` only, or exact message text too? |
| O-8 | Port the `emit_asm` text mode, or keep it as Rust tooling? |
| O-9 | Retire the monolith mirror before the M3 lockstep (M3-14)? |
| O-10 | M2's input window (M2 O-3). Measured: a 3-5k-line M3 compiles to about 34.5k-57.5k words at verifier-engine density (11.50 words/line). That doesn't fit 64 KiB but does fit 128 KiB (PLAN.md §9, R-2). |
