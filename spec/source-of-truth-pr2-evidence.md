# Addons Item 1, Part B PR 2 Evidence — Derive `min_ops` & `op_has_dest` from the ISA Table (R2)

**Branch:** `feat/addons1-pr2-min-ops`
**Baseline:** `origin/main @ 85f439755d2b76f36eb22136446a64bf17942b6e`
**Binding contract:** `spec/source-of-truth.md` (R2 + §5 PR 2 row).
**Implementer note:** implemented and verified by Lilith at Caleb's direct request while Gemini is unavailable; the full proof battery below is the standard independent-verification battery, run end to end with receipts. A second verification pass by Gemini on return is welcome and cheap (all controls are byte-exact and re-runnable).

---

## 1. Derivation Mechanism

### 1.1 Table (`pure/isa/table.omni`)
- Byte `+5` of every op row now stores `min_ops` (0..4). Values were taken from the reconciled arity lists after a programmatic cross-check (see §2): on all **81 ACCEPTED ops** the verifier's `op_min_ops` list and mod_check's `m2v_ins_min_ops` list agree exactly; the only disagreements anywhere in 0x7F00..0x7F7F are the **7 D2a hole slots** (`0x7F19..0x7F1D` at 2 in the verifier list only, `0x7F51` at 2, `0x7F55` at 1), which are stored as `0`. All **47 RESERVED rows store `0`** — D2a closed. Distribution on the 81 known ops: 0→7, 1→40, 2→30, 3→2, 4→2.
- New accessor, placed with the existing accessor family after `isa_op_flag`:
  `isa_op_min_ops(t, b, i) = ld_u8(t, isa_op_rec(b, i) + 5)` (op rows at table offset +1296, 8 B/row; geometry unchanged, `isa_tbl_len` still 2320).
- Header layout comment updated (`+5 min_ops` now populated; `+6 fixed_ty` remains for PR 6).

### 1.2 Verifier (`pure/verifier_engine.omni`)
- `op_min_ops(ws, op)` now derives: `ok * isa_op_flag(ws, 65536, safe_idx, 5) * isa_op_min_ops(ws, 65536, safe_idx)` — byte `+5` gated by the ACCEPTED bit. The hand list (including its dead hole entries) is deleted. The single call site in `verify_ins_body` passes `w`.
- `op_has_dest(ws, op)` is now `(flags & 1) * ((flags >> 5) & 1)` via `isa_op_flags` (dest bit AND ACCEPTED bit), with the same index guard as `op_is_known`.
- `op_is_destless` is **deleted** (its 12-entry hand list mixed 9 live ops with 3 RESERVED holes `0x7F47/0x7F49/0x7F4F`) — D2b closed. Equivalence: on all 81 known ops, old `op_is_known * (1 - op_is_destless)` equals the new formula by the §4/R2 destless accounting; on holes both are 0 (post-PR 1 hole flags are 0). Proven behaviorally by §4.1.

### 1.3 M2 (`pure/m2/verify/mod_check.omni`)
- `m2v_ins_min_ops` keeps its PR 1 ACCEPTED gate and now reads byte `+5`: `need = isa_op_min_ops(w, m2l_isa_tbl(), safe_idx)`, returning `-1` when not ACCEPTED. Its `is_1..is_4` hand lists are deleted.

## 2. Arity Cross-Check (pre-edit, programmatic)
Both hand lists were extracted mechanically from the baseline sources and evaluated over all 128 op values. Raw disagreements: **exactly 7** — `0x7F19..0x7F1D` (verifier 2 / mod_check 0), `0x7F51` (2/0), `0x7F55` (1/0) — precisely the D2a register. On the 81 ACCEPTED ops the lists are identical, so byte `+5` is well-defined with no silent choice.

## 3. Seed Regeneration (method + controls)
The verifier standalone seed was regenerated with the established standalone transform (stream retype, direct `load_u16`, `nwords = slen(stream)`, appended `main`) via `omnic_pure.elf compile --include-base pure/`; **control: the same transform on baseline sources reproduces the committed baseline seed byte-for-byte** (47,268 B, `0x7E82A2FFC55AEE56`).

The M2 seed's source aggregate (170,235 chars) exceeds the compiler's 65,536-char single-source limit (verified empirically: comment padding alone flips compiles at the boundary), so the seed was regenerated as: compile files 1–5 of the link order (util, table, decode, labels, mod_check — 58,835 chars edited) with the project compiler, then splice at bytecode-record granularity with the unchanged tail records (files 6–10). Because PR 2 adds one function (`isa_op_min_ops`, fn id 45) inside the table, tail fn ids and FN-operand references ≥ 45 were remapped +1 (entry id 0 exempt), using a record walker implementing the module grammar from `prescan_module_fns`/`scan_labels`/`parse_operand`. **Controls: (a) the same splice on baseline sources reproduces the committed baseline seed byte-for-byte (87,686 B, `0x3DCC8546CE5B0D57`); (b) the acid-test harness (§4.3) re-ran the whole pipeline on unmutated branch sources and reproduced both branch seeds byte-for-byte.** The resulting seed satisfies the self-hosting fixed point `S1 == S2 == S3` (and `== V` in the clean-room gate, §4.4).

## 4. Proofs

### 4.1 Zero behavior change — sweeps (base vs branch binaries, full stdout+rc compared)
- Acceptance sweep (PR 1 pattern): 3 mutation sites × 128 ops × 2 tools — **3,072 comparisons, 0 mismatches**.
- Arity/dest matrix: all 128 ops × {dest, no-dest} × 0–5 operands, plus dest+void-type × 0–2 operands, single-fn modules, both tools — **15,360 comparisons, 0 mismatches**.
- **Total: 18,432 comparisons, 0 mismatches.**

### 4.2 Campaign (pipe-captured, `--campaign-all`)
- Branch: `2800 mutants | 1210 accepted | 1590 rejected | soundness=0 roundtrip=0 judge=12`.
- Base (same session): `2800 mutants | 1212 accepted | 1588 rejected | soundness=0 roundtrip=0 judge=12`.
- **All 12 find lines byte-identical** (the F7 judge family). The 2-verdict total shift is the known corpus item-7 mechanism (item 7 is `verifier_standalone.omni_bc` itself, shrunk 23,634w → 22,857w): **fixed-item7 control** (branch binaries, base seed as item 7) reproduces the base panel exactly: `1212 accepted | 1588 rejected`, find lines identical. Inventory frozen at **12 (0/0/12)**; reduced panel and check 55 unchanged (step8 suite, §4.4).

### 4.3 Acid tests (scratch builds only; mutated table never committed)
Controls first: unmutated rebuild reproduced both branch seeds byte-for-byte (see §3).
- **Byte +5 (both consumers):** row 16 (`0x7F10`) `min_ops` 2→3, table only, zero consumer edits. Module `r0 = 0x7F10 par0, par0; ret imm`: control verifier record `[0,0,1,0,1,0]` (passed) and m2 `rc=0` → acid verifier `[0,0,1,0,0,1, 2012 @ 10]` (rejected, arity) and m2 `rc=1`. The byte is live in both consumers.
- **Flags bit 0 (verifier):** row 70 (`0x7F46`) flags 32→33, table only. Destless module `0x7F46; ret imm`: control `[0,0,1,0,1,0]` (passed) → acid `[0,0,1,0,0,1, 2012 @ 10]` (missing dest). (First attempt on `0x7F05` DROP showed no flip — correctly: DROP is on the `skip` path in `verify_ins_body` and never reaches the dest check; retested on `0x7F46`, which takes the normal path.)

### 4.4 Gates (branch tree)
- `scripts/zero_rust_rebuild.sh`: **PASS** — all 10 MANIFEST artifacts bit-exact, `S1 == S2 == S3 == V`, selftest + all 11 named gates PASS, `omnic_pure.elf` self-compile fixed point unchanged (59,653 B), negative control caught.
- `scripts/verify_step7_driver.sh`: **PASS=38 FAIL=0**. `scripts/verify_step8_harness.sh`: **PASS=76 FAIL=0**.

### 4.5 Pins
| Artifact | Before | After |
|---|---|---|
| `pure/m2/m2.omni_bc` | 87,686 B / `0x3DCC8546CE5B0D57` | **86,576 B (43,288w) / `0xE649625E08054C94`** |
| `bootstrap/m2.elf` | 125,660 B / `0x3A4A285C58CFA93E` | **124,248 B / `0xAEDE11CB00E7EFDF`** |
| `pure/verifier_standalone.omni_bc` | 47,268 B / `0x7E82A2FFC55AEE56` | **45,714 B (22,857w) / `0xD4F684144C0D81F8`** |
| `verifier.elf` (built) | 77,189 B / `0xDB33F13E1082948A` | **75,340 B / `0xEDDBB663814DFB9A`** |

Unmoved, as required: `bootstrap/m3.elf` (207,015 B, `0x7397A5020895AEBF`), all `pure/m3/*.omni_bc` seeds, `elf_link.omni_bc`, both `m4` seeds, `m4.elf`, the frozen five, and the `omnic_pure` fixed point. Pin strings updated in `bootstrap/MANIFEST.md` (with the PR 2 judge-change note), `README.md`, `pure/harness/core_gates.omni`, `pure/harness/proposer_loop.omni`, `scripts/build_pure_driver.sh`, `scripts/zero_rust_rebuild.sh`, `scripts/verify_step7_driver.sh`. No Python (or any scratch helper) is committed; regeneration helpers were bookkeeping only and live outside the tree.

## 5. Scope notes
- Files changed: 3 sources (`pure/isa/table.omni`, `pure/verifier_engine.omni`, `pure/m2/verify/mod_check.omni`), 3 binaries (`bootstrap/m2.elf`, `pure/m2/m2.omni_bc`, `pure/verifier_standalone.omni_bc`), 7 pin/doc files, this evidence doc.
- The duplicated `#[strict]` attribute above `op_is_known` (cosmetic, from PR 1) was left untouched — out of R2 scope.
- D2a and D2b are closed by this PR. Next per spec §5: PR 3 (R3 — caps/sink/sanitizer derivation; D5a disposition inside it).
