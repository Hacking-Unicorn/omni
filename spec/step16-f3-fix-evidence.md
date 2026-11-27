# Step 16 Part B2 — Judge PR Evidence: Family F3 (Dest-Shape & `LABEL` Operand Validation)

- **Branch:** `feat/step16-f3-shape-fix` (branched off `main @ a6996a0`, after PR #61 merged)
- **PR Type:** **Judge PR** under `bootstrap/MANIFEST.md` discipline
- **Host:** WSL2 Ubuntu Linux x86-64

---

## 1. All 12 Verbatim Family F3 Member Lines (from `spec/step16-enumeration-evidence.md` §5)

### Sub-Shape 1 — Dest-Shape Mismatch (11 finds)
Canonical member: `seed6_clamp.omni` (`item=2`), word `30`: `0x7F4A -> 0x7F3F` (`seed=1882296823`).

```text
[FUZZ FIND] kind=1 op=1 item=2 site=30 seed=1882296823 orig=0x7F4A mut=0x7F3F
[FUZZ FIND] kind=1 op=1 item=1 site=30 seed=1936414711 orig=0x7F4A mut=0x7F3F
[FUZZ FIND] kind=1 op=1 item=2 site=30 seed=1634217457 orig=0x7F4A mut=0x7F3F
[FUZZ FIND] kind=1 op=1 item=1 site=30 seed=1240234520 orig=0x7F4A mut=0x7F21
[FUZZ FIND] kind=1 op=1 item=1 site=30 seed=274412641 orig=0x7F4A mut=0x7F06
[FUZZ FIND] kind=1 op=1 item=4 site=742 seed=872744656 orig=0x7F4A mut=0x7F0D
[FUZZ FIND] kind=1 op=1 item=2 site=30 seed=121749842 orig=0x7F4A mut=0x7F15
[FUZZ FIND] kind=1 op=1 item=1 site=30 seed=841842198 orig=0x7F4A mut=0x7F21
[FUZZ FIND] kind=1 op=1 item=2 site=30 seed=572384150 orig=0x7F4A mut=0x7F59
[FUZZ FIND] kind=1 op=1 item=5 site=563 seed=1922476101 orig=0x7F4A mut=0x7F04
[FUZZ FIND] kind=1 op=1 item=1 site=30 seed=1311126675 orig=0x7F4A mut=0x7F09
```

### Sub-Shape 2 — `LABEL` (`0x7F40`) Non-Label Operand (1 find)

```text
[FUZZ FIND] kind=1 op=1 item=2 site=30 seed=519196673 orig=0x7F4A mut=0x7F40
```

---

## 2. Root Cause & Minimal Fix (`pure/verifier_engine.omni`, +7 lines)

1. **Dest-shape (`pure/verifier_engine.omni::verify_ins_body`, L973-974):**
   - Previously, `do_dest = known * (1 - skip) * op_has_dest(op) * is_dest` silently skipped `handle_dest` whenever `op_has_dest(op) == 1` in a destless slot (`is_dest == 0`).
   - Fix:
     ```omni
     let missing_dest: i64 = known * (1 - skip) * op_has_dest(op) * (1 - is_dest);
     w = append_err_if(w, missing_dest, 2012, ins_off);
     ```
   - **Error code:** Existing `2012` (exact instruction operand-shape/arity code already raised on the preceding line L972: `append_err_if(w, bad_op_ar, 2012, ins_off)`).
2. **`LABEL` operand (`pure/verifier_engine.omni::check_branch_labels`, L675-679):**
   - Previously, `record_labels_from_ops` (L650-658) only recorded a label when `is_gt(fl & 2, 0) + is_gt(fl & 8, 0)` was non-zero, and `check_branch_labels` never validated `LABEL (0x7F40 = 32576)`, allowing `LABEL v1` (register operand `fl = 1`).
   - Label-operand class derived from `pure/verifier_engine.omni::parse_operand_raw` (L436/449/466), `record_labels_from_ops` (L652), `check_branch_labels` (L684), and `pure/m2/labels.omni::m2lbl_slot_of` (L21-32): `LBL` tokens `0x7EF0..=0x7EFF` (`fl & 2`) or `IMM` label IDs (`fl & 8`), rejecting register/param tokens (`0x7E00..=0x7EDF`, `fl = 1`).
   - Fix:
     ```omni
     let is_lbl_def: i64 = is_eq(op, 32576);
     let lbl_p0: i64 = ws_get(w, 384) * is_gt(nops, 0);
     let lbl_fl: i64 = op_flags(lbl_p0);
     let lbl_ok: i64 = is_gt(nops, 0) * is_gt((lbl_fl & 2) + (lbl_fl & 8), 0);
     w = append_err_if(w, is_lbl_def * (1 - lbl_ok), 2009, off);
     ```
   - **Error code:** Existing `2009` (`UNRESOLVED_LABEL` / invalid label operand, already raised by `check_branch_labels` at L678 and L687).

---

## 3. Moved Pins (`bootstrap/MANIFEST.md` Discipline)

From-scratch derivation verified first on unmodified `a6996a0:pure/verifier_engine.omni` (`31,168 B`, `0x2AF95DB4A6628007` — **0-byte diff** vs committed seed), then re-run identically on the fixed `pure/verifier_engine.omni`:

| Artifact | Old Size / FNV-1a64 | New Size / FNV-1a64 | Reason |
| :--- | :--- | :--- | :--- |
| `pure/verifier_standalone.omni_bc` | `31,168 B` (`15,584 w`) / `0x2AF95DB4A6628007` | **`31,526 B` (`15,763 w`) / `0x485D1C5C2D0A69C6`** | Step 16 Part B2 Judge Fix (Family F3) |
| `verifier.elf` (rebuilt in `build/pure-bin/`) | `38,365 B` / `0x035987D581B3F315` | **`38,895 B` / `0xB60283901F089BE9`** | Rebuilt from new `verifier_standalone.omni_bc` via `m2.elf` |

- **Unmoved pins:** `bootstrap/m2.elf` (`126,152 B`, `0xAA5F84B6AA45BB51`, `S1==S2==S3==V`), `bootstrap/m3.elf` (`207,015 B`, `0x7397A5020895AEBF`), `m4.elf` (`91,576 B`, `0xFBA8272337B1BAC4`), all frozen-5 hashes, all other `.omni_bc` seeds, and `omnic_pure.elf` self-compile fixed point (`59,653 B`, 0-byte diff).

---

## 4. Permanent Negative Controls & Check 55 Update

- **Step 8 Check 58 (F3 dest-shape):** `seed6_clamp.omni` word `30` (`0x7F4A -> 0x7F3F`) rejected by `verifier.elf` with error `2012` at offset `30`; original `seed6_clamp.omni` verifies cleanly (`passed=1, ecnt=0`).
- **Step 8 Check 59 (F3 LABEL-operand):** `seed6_clamp.omni` word `30` (`0x7F4A -> 0x7F40`) rejected by `verifier.elf` with error `2009` at offset `30`; original `seed6_clamp.omni` verifies cleanly.
- **Step 8 Check 55 (reduced no-arg panel):**
  - Find #2 (`seed4_consensus` word `30: 0x7F4A -> 0x7F5B`) triggers both F1 and F3; the word-30 mutant now dies via the F3 dest-shape check (`err 2012` at off `30`), while `0x7F5B` itself remains accepted in a dest-ful slot until the F1 PR (Find #2's F1 component is **NOT** closed).
  - The reduced panel therefore advances past Find #2 to its new first find:
    ```text
    [FUZZ FAIL] 10 mutants | 5 accepted | 5 rejected | soundness=0 roundtrip=1 judge=0
    [FUZZ FIND] kind=3 op=1 item=1 site=17 seed=977037651
    ```
  - Check 55 now asserts `[FUZZ FIND] kind=3 op=1 item=1 site=17 seed=977037651`.

---

## 5. Full `--campaign-all` Piped Re-Run & Line-by-Line Inventory Diff

Command: `./build/pure-bin/fuzz_gates.elf --campaign-all | cat > /tmp/s16_f3_after.txt` (`RC = 1`):

```text
[FUZZ FAIL] 2800 mutants | 1321 accepted | 1479 rejected | soundness=43 roundtrip=143 judge=12
op=1 gen=500 acc=201 rej=299
op=2 gen=500 acc=215 rej=285
op=3 gen=500 acc=175 rej=325
op=4 gen=500 acc=382 rej=118
op=5 gen=500 acc=51 rej=449
op=6 gen=150 acc=150 rej=0
op=7 gen=150 acc=147 rej=3
```

### Line-by-Line Inventory Diff vs PR #61 (`228 -> 198`)

- **Closed on unchanged corpus items `item=0..6` (27 lines, 0 new finds):**
  - **12 direct F3 members** (11 dest-shape + 1 LABEL-operand): all 12 lines from §1 above.
  - **6 F1+F3 dual-trigger lines** (unassigned `0x7F5A`/`0x7F5B`/`0x7F5C` in destless `0x7F4A` slot — rejected via dest-shape `2012`; F1 ops remain open in dest-ful slots):
    - `[FUZZ FIND] kind=1 op=1 item=0 site=49 seed=1728482248 orig=0x7F4A mut=0x7F5C`
    - `[FUZZ FIND] kind=1 op=1 item=0 site=49 seed=1909115813 orig=0x7F4A mut=0x7F5A`
    - `[FUZZ FIND] kind=1 op=1 item=0 site=49 seed=535143849 orig=0x7F4A mut=0x7F5A`
    - `[FUZZ FIND] kind=1 op=1 item=1 site=30 seed=1367066037 orig=0x7F4A mut=0x7F5B` *(Find #2)*
    - `[FUZZ FIND] kind=1 op=1 item=2 site=30 seed=2129932747 orig=0x7F4A mut=0x7F5C`
    - `[FUZZ FIND] kind=1 op=1 item=2 site=30 seed=37879232 orig=0x7F4A mut=0x7F5C`
  - **9 F2+F3 dual-trigger lines** (unlowered dest-requiring ops in destless `0x7F4A`/`0x7F43` slot — rejected via dest-shape `2012`; F2 ops remain open in dest-ful slots):
    - `[FUZZ FIND] kind=1 op=1 item=0 site=49 seed=256352086 orig=0x7F4A mut=0x7F55`
    - `[FUZZ FIND] kind=1 op=1 item=1 site=30 seed=1235414888 orig=0x7F4A mut=0x7F36`
    - `[FUZZ FIND] kind=1 op=1 item=1 site=30 seed=888936234 orig=0x7F4A mut=0x7F32`
    - `[FUZZ FIND] kind=1 op=1 item=1 site=30 seed=904018996 orig=0x7F4A mut=0x7F03`
    - `[FUZZ FIND] kind=1 op=1 item=2 site=30 seed=138943525 orig=0x7F4A mut=0x7F02`
    - `[FUZZ FIND] kind=1 op=1 item=2 site=30 seed=1487432506 orig=0x7F4A mut=0x7F33`
    - `[FUZZ FIND] kind=1 op=1 item=3 site=142 seed=382178132 orig=0x7F4A mut=0x7F55`
    - `[FUZZ FIND] kind=1 op=1 item=3 site=245 seed=371058551 orig=0x7F4A mut=0x7F37`
    - `[FUZZ FIND] kind=1 op=1 item=6 site=279 seed=52757434 orig=0x7F43 mut=0x7F1B`
- **Re-pinned corpus item `item=7` (`pure/verifier_standalone.omni_bc`, `15,584 w -> 15,763 w`):**
  - **Closed (4 lines):** 3 F3 dest-shape hits from PR #61 (`site=6225`, `site=15503`, `site=15550`) genuinely closed by this fix + 1 F2 hit (`site=14820 seed=2037675677`) shifted off by `nw` modulus change.
  - **Resampled (1 line, existing Family F2 — 0 new families):**
    - `[FUZZ FIND] kind=1 op=1 item=7 site=13970 seed=1875523199 orig=0x7F41 mut=0x7F47` (destless unlowered `0x7F47`, already triaged in Family F2).

---

## 6. Close-Out Gates (WSL2 Ubuntu Linux x86-64)

- `bash scripts/zero_rust_rebuild.sh`: **PASS** (`S1==S2==S3==V`, all 10 MANIFEST pins bit-exact)
- `bash scripts/verify_step7_driver.sh`: **38/38 PASS**
- `bash scripts/verify_step8_harness.sh`: **59/59 PASS**
- `./build/pure-bin/omnic_pure.elf selftest`: **PASS**
- `python3 scripts/external_verifier_check.py`: **PASS** (`8/8` traces)
- All 8 corpus items verified & runnable goldens executed `rc=0` (0 completeness regressions).
