# Step 16 Part B3b — LABEL-Operand Corner: Probe & Judge PR Evidence

- **Branch:** `feat/step16-label-corner-fix` (branched off `main @ 5209ae4`, after PR #63 merged)
- **PR Type:** **Judge PR** under `bootstrap/MANIFEST.md` discipline (Part A probe confirmed genuine `verifier.elf` PASS vs `m2.elf` FAIL divergences on both corners and at both `LABEL` definition and branch-target sites)
- **Host:** WSL2 Ubuntu Linux x86-64

---

## Part A — Probe Analysis & Findings

### 1. Exact M2 Rule (`pure/m2/labels.omni::m2lbl_slot_of`, L21–32)
```omni
let kind: i64 = m2d_kind(packed);
let is_lbl: i64 = m2u_eq(kind, 3);
let is_imm: i64 = m2u_eq(kind, 5);
let uval: i64 = m2d_uval(packed);
let ival: i64 = m2d_ival(packed);
let lbl_ok: i64 = is_lbl * m2u_in(uval, 32496, 32511);
let imm_ok: i64 = is_imm * m2u_in(ival, 0, 4079);
```
- **Accepted LBL tokens:** `kind == 3`, `uval` in `32496..=32511` (`0x7EF0..=0x7EFF`) → slots `4080..4095`.
- **Accepted IMM label IDs:** `kind == 5`, signed 32-bit `ival` in `0..=4079` (`< 4080`, negative `< 0` and `>= 4080` rejected per D-11) → slots `0..4079`.
- **All other operand classes:** `REG (1)`, `PARAM (2)`, `TRIT (4)`, `ENG_SPAN (6)`, `SCHEMA_REF (7)`, `FN_REF (8)`, `SKILL_REF (9)`, `FIELD_KV (10)` return `-1`, rejected as `BAD_LABEL = 6` in `pure/m2/verify/mod_check.omni::m2v_check_branch_ins` (L150–167).

### 2. Verifier Site Map (`pure/verifier_engine.omni`)
In `parse_operand_raw` (L431–479), flag bit `8` was assigned to:
- `TRIT_*` (`0x7EE7..0x7EE9`): `simple_flags = 8` (`fl = 8`)
- `IMM_I16` (`0x7EE5`) & `IMM_I32` (`0x7EE6`): `pack_op(imm_pos, 8, imm_tok)` (`fl = 8`, truncating `i32` to low 16 bits at L457)
- `SCHEMA_REF` (`0x7EEB`), `SKILL_REF` (`0x7EED`): `sym_fl = 8`; `FN_REF` (`0x7EEC`): `sym_fl = 72` (`8 | 64`)

Three label-consuming sites checked `is_gt(fl & 2, 0) + is_gt(fl & 8, 0)` (prior to fix):
1. `record_labels_from_ops` (L652): records defined label IDs from `LABEL (0x7F40)` during `scan_labels`.
2. `check_branch_labels` — `LABEL (0x7F40)` definition check (L676–680): validates operand 0 of `LABEL`.
3. `check_branch_labels` — branch-target loop (L687–696): validates target operands of `JMP (0x7F41)`, `BR_TRIT (0x7F42)`, `BR_POS (0x7F43)`, `BR_NEG (0x7F44)`.

### 3. Probe Mutants & Verbatim Tool Verdicts

Base program used for clean probes: `pure/programs/kv_ledger_service.omni` (`item=3`), `fn_5` (`FN_HDR` at word `145`, `icnt` at word `153`: `13 -> 14`, inserting a `LABEL` instruction before word `212` and patching `JMP` word `189` / `LABEL` word `213` for branch-target probes).
*(Note: `seed6_clamp` word `30` replaces its sole `RET`, so even valid `LABEL 0x7EF0` fails `m2.elf` for missing `RET` rather than label validity; hence `kv_ledger_service` `fn_5` is used for clean label-validity isolation.)*

| Probe Mutant | Pre-Fix `verifier.elf` | Post-Fix `verifier.elf` | `m2.elf` | Divergence Resolved? |
| :--- | :--- | :--- | :--- | :---: |
| `kv original` | `PASS (st=0, bad=[])` | `PASS (st=0, bad=[])` | `PASS (rc=0, 1390 B)` | Baseline |
| `kv + LABEL 0x7EF4` (`LBL 4`, valid) | `PASS (st=0, bad=[])` | `PASS (st=0, bad=[])` | `PASS (rc=0, 1390 B)` | Baseline |
| `kv + LABEL IMM 0` (valid) | `PASS (st=0, bad=[])` | `PASS (st=0, bad=[])` | `PASS (rc=0, 1390 B)` | Baseline |
| `kv + LABEL IMM 4079` (`bound-1`, valid) | `PASS (st=0, bad=[])` | `PASS (st=0, bad=[])` | `PASS (rc=0, 1390 B)` | Baseline |
| `kv + LABEL 0x7EE9` (`TRIT_POS`, `fl=8`) | `PASS (st=0, bad=[])` | `FAIL (5, [(2009, 212)])` | `FAIL (rc=1, 0 B)` | **Yes** |
| `kv + LABEL 0x7EEB 0` (`SCHEMA_REF`, `fl=8`) | `PASS (st=0, bad=[])` | `FAIL (5, [(2009, 212)])` | `FAIL (rc=1, 0 B)` | **Yes** |
| `kv + LABEL 0x7EEC 0` (`FN_REF`, `fl=72`) | `PASS (st=0, bad=[])` | `FAIL (5, [(2009, 212)])` | `FAIL (rc=1, 0 B)` | **Yes** |
| `kv + LABEL IMM 4080` (`bound`) | `PASS (st=0, bad=[])` | `FAIL (5, [(2009, 212)])` | `FAIL (rc=1, 0 B)` | **Yes** |
| `kv + LABEL IMM 4081` (`bound+1`) | `PASS (st=0, bad=[])` | `FAIL (5, [(2009, 212)])` | `FAIL (rc=1, 0 B)` | **Yes** |
| `kv + LABEL IMM 0xFFFF` (`-1`) | `PASS (st=0, bad=[])` | `FAIL (5, [(2009, 212)])` | `FAIL (rc=1, 0 B)` | **Yes** |
| `kv + LABEL IMM32 4080` (`hi=0, lo=4080`) | `PASS (st=0, bad=[])` | `FAIL (5, [(2009, 212)])` | `FAIL (rc=1, 0 B)` | **Yes** |
| `kv + LABEL IMM32 65536` (`hi=1, lo=0`) | `PASS (st=0, bad=[])` | `FAIL (5, [(2009, 212)])` | `FAIL (rc=1, 0 B)` | **Yes** |
| Branch: `JMP TRIT_POS (0x7EE9) -> LABEL L0` | `PASS (st=0, bad=[])` | `FAIL (5, [(2009, 188)])` | `FAIL (rc=1, 0 B)` | **Yes** |
| Branch: `JMP SCH_REF 1 (0x7EEB 1) -> LABEL L1` | `PASS (st=0, bad=[])` | `FAIL (5, [(2009, 188)])` | `FAIL (rc=1, 0 B)` | **Yes** |

*(Additional non-probed observations recorded honestly outside this PR's scope: `m2.elf` separates `LBL` slots `4080..4095` from `IMM` slots `0..4079` so `JMP IMM 1` does not alias `LABEL L1`, whereas verifier shares IDs `0..255` via a 256-bit `has_label` bitset (`IMM` label IDs `256..4079` fail verifier `2009` when branched to, even though accepted by `m2`). Left untouched per scope discipline.)*

---

## Part B — Fix, Re-Pin, & Verification

### 1. Minimal Fix (`pure/verifier_engine.omni`) & Scope Statement
- **Explicit scope:** Tightening applies to **both** `LABEL` definition (`record_labels_from_ops` + `check_branch_labels` L679) AND branch-target validation (`check_branch_labels` L690), because the Part A probe proved that `JMP TRIT_POS` and `JMP SCHEMA_REF` also caused `verifier.elf PASS` vs `m2.elf FAIL`.
- **Change:**
  - `parse_operand_raw` (L459–469): added unused flag bit `16` to `imm_fl = 8 + (i16_lbl_ok + i32_lbl_ok) * 16`, where `i16_lbl_ok = is_i16 * need_i16 * in_range(i16_val, 0, 4079)` and `i32_lbl_ok = is_i32 * need_i32 * is_eq(i32_hi, 0) * in_range(i32_lo, 0, 4079)`.
  - `record_labels_from_ops` (L655), `check_branch_labels` `LABEL` check (L681), and `check_branch_labels` branch loop (L692): replaced `fl & 8` with `fl & 16` (`lbl_fl & 16`), keeping existing error code **`2009`** at both sites. All non-label consumers of `fl & 8` are unaffected.

### 2. Moved Pins (`bootstrap/MANIFEST.md` Discipline)

From-scratch derivation verified first on unmodified `5209ae4:pure/verifier_engine.omni` (`31,564 B`, `0x061DACCE78F5ADDB` — **0-byte diff**), then re-run identically on the fixed `pure/verifier_engine.omni`:

| Artifact | Old Size / FNV-1a64 | New Size / FNV-1a64 | Reason |
| :--- | :--- | :--- | :--- |
| `pure/verifier_standalone.omni_bc` | `31,564 B` (`15,782 w`) / `0x061DACCE78F5ADDB` | **`31,744 B` (`15,872 w`) / `0x00B2D6710B6D00D0`** | Step 16 Part B3b Judge Fix (`LABEL` / branch label-operand corner) |
| `verifier.elf` (rebuilt in `build/pure-bin/`) | `38,943 B` / `0x1D4E0C270335720E` | **`39,223 B` / `0x5CCDC9F0C0897E0B`** | Rebuilt from new `verifier_standalone.omni_bc` via `m2.elf` |

- **Unmoved pins:** `bootstrap/m2.elf` (`126,152 B`, `0xAA5F84B6AA45BB51`, `S1==S2==S3==V`), `bootstrap/m3.elf` (`207,015 B`, `0x7397A5020895AEBF`), `m4.elf` (`91,576 B`, `0xFBA8272337B1BAC4`), all frozen-5 hashes, all other `.omni_bc` seeds, and `omnic_pure.elf` self-compile fixed point (`59,653 B`, 0-byte diff).

### 3. Permanent Step 8 Checks (`60 -> 63`) & Check 55 Verification
- **Check 61:** `kv + LABEL TRIT_POS (0x7EE9)` and `kv + LABEL SCHEMA_REF (0x7EEB)` rejected with err `2009`; `kv + LABEL 0x7EF4` accepted; original runs cleanly (`rc=0`).
- **Check 62:** `kv + LABEL IMM 4079` accepted; `kv + LABEL IMM 4080`, `4081`, `0xFFFF (-1)`, and `IMM32 4080` all rejected with err `2009`.
- **Check 63:** `JMP TRIT_POS (0x7EE9) -> LABEL L0 (0x7EF0)` rejected with err `2009`.
- **Check 55 verified empirically unchanged:**
  ```text
  [FUZZ FAIL] 10 mutants | 5 accepted | 5 rejected | soundness=0 roundtrip=1 judge=0
  [FUZZ FIND] kind=3 op=1 item=1 site=17 seed=977037651
  ```

### 4. Full `--campaign-all` Piped Re-Run & Line-by-Line Inventory Diff

Command: `./build/pure-bin/fuzz_gates.elf --campaign-all | cat > /tmp/s16_b3b_after.txt` (`RC = 1`):

```text
[FUZZ FAIL] 2800 mutants | 1317 accepted | 1483 rejected | soundness=31 roundtrip=143 judge=12
op=1 gen=500 acc=193 rej=307
op=2 gen=500 acc=223 rej=277
op=3 gen=500 acc=176 rej=324
op=4 gen=500 acc=382 rej=118
op=5 gen=500 acc=46 rej=454
op=6 gen=150 acc=150 rej=0
op=7 gen=150 acc=147 rej=3
```

**Line-by-line inventory diff vs PR #63 (`186 -> 186`):**
- **Unchanged corpus items `item=0..6`:** **0 closed, 0 added** — every find line on `item=0..6` is 100% identical to post-B3.
- **Re-pinned corpus item `item=7` (`pure/verifier_standalone.omni_bc`, `15,782 w -> 15,872 w`):** 1 line shifted in offset only due to `nw` modulus change (same seed and mutation, existing Family F2 unlowered `0x7F49`):
  - Closed: `[FUZZ FIND] kind=1 op=1 item=7 site=14617 seed=2037675677 orig=0x7F48 mut=0x7F49`
  - Resampled: `[FUZZ FIND] kind=1 op=1 item=7 site=14527 seed=2037675677 orig=0x7F48 mut=0x7F49`
- **Zero new root-cause families.**

---

## Close-Out Gates (WSL2 Ubuntu Linux x86-64)

- `bash scripts/zero_rust_rebuild.sh`: **PASS** (`S1==S2==S3==V`, all 10 MANIFEST pins bit-exact)
- `bash scripts/verify_step7_driver.sh`: **38/38 PASS**
- `bash scripts/verify_step8_harness.sh`: **63/63 PASS**
- `./build/pure-bin/omnic_pure.elf selftest`: **PASS**
- `python3 scripts/external_verifier_check.py`: **PASS** (`8/8` traces)
- All 8 corpus items verified & runnable goldens executed `rc=0` (0 completeness regressions).
