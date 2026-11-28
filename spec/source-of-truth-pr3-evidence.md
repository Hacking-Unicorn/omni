# Addons Item 1 Part B PR 3 — Evidence (spec R3: `op_req_cap`, `op_is_sink_raw`, `op_is_sanitizer`)

Implemented and verified by Lilith at Caleb's direct request while Gemini is
unavailable (Gemini can re-verify on return; every control below is byte-exact
and re-runnable). Baseline: `main @ 173db68` (after PR #73 / Part B PR 2).

## Mechanism

`pure/verifier_engine.omni` only; no table edits, no M2 changes.

- Deleted the hand-transcribed accumulators `op_req_cap_0`, `op_req_cap_1`,
  `op_is_sink_raw_0`, `op_is_sink_raw_1` (~115 lines of `is_eq` lists).
- `op_req_cap(ws, op)` now derives the capability mask from
  `isa_op_cap(ws, 65536, idx)` (table bytes `+2..+3`), gated by the `ACCEPTED`
  bit: `known * isa_op_cap(...)`.
- `op_is_sink_raw(ws, op)` derives `known * isa_op_flag(ws, 65536, idx, 1)`.
- `op_is_sanitizer(ws, op)` derives `known * isa_op_flag(ws, 65536, idx, 2)`.
- The three call sites in `handle_dest` pass the workspace (`w`) through,
  exactly as PR 2 did for `op_min_ops`.

## Pre-edit cross-check (programmatic, all 128 slots)

The five hand lists were extracted mechanically and compared against
`pure/isa/table.omni` (`cap = cap_lo + 256*cap_hi`, sink = flags bit 1,
sanitizer = flags bit 2):

- **81 accepted ops: 0 mismatches** on all three predicates.
- **19 RESERVED slots differ** — the hand lists still carry the stale values
  PR 1 zeroed in the table (16 cap-bearing, 11 sink, 3 sanitizer:
  `0x7F5A..=0x7F5C`). These values are unreachable: the sole call site of all
  three predicates is `handle_dest`, invoked only under
  `do_dest = known * ...` in `verify_ins_body`, so an unaccepted op never
  reaches them. The derivations return 0 there, which is the behavior the
  `known` gate already enforced.

## Seed regeneration

The verifier standalone seed was regenerated with the standalone transform
(stream retype, direct `load_u16`, appended `main`) and
`omnic_pure.elf compile --include-base pure/`. Control: the same transform on
the unmodified baseline sources reproduces the committed seed byte-for-byte
(`45,714 B`, `0xD4F684144C0D81F8`). The M2 seed is untouched by this PR
(`verifier_engine.omni` is not part of the M2 aggregate), and
`zero_rust_rebuild.sh` confirms the M2 pins did not move.

## Proofs

- `scripts/zero_rust_rebuild.sh`: **PASS** — all 10 MANIFEST artifacts
  bit-exact, `S1 == S2 == S3 == V`, all 11 gates + selftest PASS,
  `omnic_pure.elf` fixed point unchanged (59,653 B), negative control caught.
- `scripts/verify_step7_driver.sh`: **PASS=38 FAIL=0**.
- `scripts/verify_step8_harness.sh`: **PASS=76 FAIL=0**.
- **Differential sweeps, baseline vs branch binaries, full stdout + rc:
  20,480 comparisons, 0 mismatches.**
  - Sweep A: op substitution at 3 corpus sites × 128 ops × {verifier, m2}
    (3,072 runs).
  - Sweep B: arity/dest matrix, all 128 ops × dest/nodest × 0–5 operands +
    void-ty variants (15,360 runs; same battery as PR 2).
  - Sweep C (new, R3 surface; 2,048 verifier runs): for every op X —
    (i) X consumes an `@UNTRUSTED`-tainted operand with X's tc trusted
    (upgrade + sink + sanitizer paths), (ii) same with X's tc untrusted
    (isolates the sink term), (iii) X in a cap-less function and in an
    all-caps function (the `1003` surface). Coverage is non-vacuous on the
    baseline: exactly **24** ops produce `1003` at fn-cap 0 (the spec's 24
    active gated ops); exactly the 4 live sanitizers (`0x7F57..=0x7F59`,
    `0x7F70`) come out clean on the tainted-operand flow; 20 dest-form sink
    ops produce `1002` under the untrusted-tc variant.
- **Campaign** (`--campaign-all`, pipe-captured): baseline
  `2800 | 1210 accepted | 1590 rejected`, branch
  `2800 | 1218 accepted | 1582 rejected`; **all 12 find lines identical**
  (inventory frozen at 12 = 0 soundness / 0 roundtrip / 12 judge, F7 only).
  The +8 accepted shift is the known corpus item-7 mechanism (item 7 is
  `verifier_standalone` itself, which shrank 22,857w → 20,964w): the
  fixed-item7 control (branch binaries, baseline seed swapped in as item 7)
  reproduces **1210/1590 exactly**. Seed restored and hash-verified after.
- **Acid tests** (scratch trees; table-only mutations on op `0x7F10`, row 16;
  zero consumer edits; unmutated control rebuild reproduced both branch
  artifacts byte-for-byte first):
  - **Cap:** `cap_lo` 0 → 16 → a cap-less-function module using `0x7F10`
    gains exactly one error, `1003` (was a clean pass).
  - **Sink:** flags bit 1 set (57 → 59) → the sink-isolating module
    (tainted operand, instruction tc untrusted) gains exactly one error,
    `1002` (was a clean pass).
  - **Sanitizer:** flags bit 2 set (57 → 61) → the tainted-operand module
    loses its only error (`1002` from the upgrade rule) and passes.

## Pins

| Artifact | Before | After |
|---|---|---|
| `pure/verifier_standalone.omni_bc` | 45,714 B / 22,857w `0xD4F684144C0D81F8` | 41,928 B / 20,964w `0x6BF60A0097BDC69D` |
| `verifier.elf` (built) | 75,340 B `0xEDDBB663814DFB9A` | 71,059 B `0x1607790934FA7455` |

Unmoved, as predicted by `spec/source-of-truth.md` §5 for PR 3:
`bootstrap/m2.elf` (124,248 B, `0xAEDE11CB00E7EFDF`), `pure/m2/m2.omni_bc`
(86,576 B, `0xE649625E08054C94`), `bootstrap/m3.elf`, all M3 seeds,
`elf_link`, both M4 seeds, `m4.elf`, the frozen five, and the
`omnic_pure.elf` fixed point.

## Scope notes

- One rule family (R3), one file of source. D5a is **not** touched here —
  the spec assigns its unification to PR 5.
- No Python (or any scratch tooling) is committed, and no pin was produced
  by anything but the project's own rebuild: the seed came from
  `omnic_pure.elf` and `verifier.elf` from `bootstrap/m2.elf`, both
  re-verified end-to-end by `zero_rust_rebuild.sh`.
