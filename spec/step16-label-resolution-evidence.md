# Step 16 Part B3c — Label Identity Resolution: Probe & Judge PR Evidence

- **Branch:** `feat/step16-label-resolution-fix` (branched off `main @ df5105a`, after PR #65 merged)
- **PR Type:** **Judge PR** under `bootstrap/MANIFEST.md` discipline (Part A probes confirmed Class 1 aliasing soundness divergence and Class 2 truncated IMM / signed bit-63 completeness divergence; Class 3 duplicate handling agreed cleanly and is excluded from fix scope)
- **Host:** WSL2 Ubuntu Linux x86-64

---

## Part A — Resolution Models & Probe Results

### 1. Resolution Models Compared from Source

#### Verifier Pre-Fix (`pure/verifier_engine.omni`)
- **ID extraction (`record_labels_from_ops`, L654–662, & `check_branch_labels`, L690–696):** extracted `lid = op_tok(p)`. For `LBL` (`0x7EF0..=0x7EFF`), `parse_operand_raw` (L451) encoded `simple_tok = tok - 32496` (`0..=15`); for `IMM_I16`/`IMM_I32` in `0..=4079` (B3b), `imm_tok = val` (`0..=4079`). Thus `LBL k` and `IMM k` mapped to the **same integer `k`** (`0..15` collided directly).
- **Bitset structure (`has_label` L365–373, `set_label` L376–384):** 256-bit shared bitset in workspace words `ws[4..7]` (`in_range(lid, 0, 255)`, `4 + (lid >> 6)`). Any `IMM` label ID `>= 256` was silently ignored on `set_label` and rejected on `has_label`.
- **Signed bit-63 mask bug (`has_label` L370):** `is_gt(bits & m, 0)` evaluated `(bits & (1 << 63))` using signed `i64` comparison (`1 << 63 = -9223372036854775808 < 0`), causing every slot with `(slot & 63) == 63` (IMM `63, 127, 191, 255`) to evaluate `has_label == 0` and falsely fail with error `2009`.
- **Per-function label-count cap:** None in bytecode instruction count beyond the ID range (`0..255` pre-fix, `0..4095` post-fix).

#### M2 Backend & Module Verifier (`pure/m2/labels.omni`, `pure/m2/verify/mod_check.omni`)
- **Slot allocation (`pure/m2/labels.omni::m2lbl_slot_of`, L21–32):** 4,096 distinct non-aliasing `u32` slots in arena `0x20000..0x23FFF` (`pure/m2/labels.omni:6–8`, `45`):
  - `IMM` (`kind == 5`, `0 <= ival <= 4079`) → slots **`0..4079`** (`m2lbl_slot_of` L28, L30).
  - `LBL` (`kind == 3`, `32496..=32511` = `0x7EF0..=0x7EFF`) → slots **`4080..4095`** (`4080 + (uval - 32496)`, L27, L29–30).
- **Definition & target checks:** `m2lbl_define` (L57–73) uses first-definition-wins (`unset = m2u_eq(cur, 0)` at L62); `m2lbl_lookup` (L77–86) & `m2v_check_lbl_op` (`pure/m2/verify/mod_check.omni:116–131`) return error `6` (`BAD_LABEL`) if slot `< 0` or `lookup < 0`.
- **Per-function label-count cap:** None beyond the 4,096 non-aliasing slots (`0..4095`); M3 surface syntax caps source labels at 15, while M2 bytecode allows all 4,096 slots (`m2lbl_clear_all`, L43–52).

### 2. Reconstructible Word-Level Probe Constructions

All probes start from compiled `pure/programs/kv_ledger_service.omni` (`omnic_pure.elf compile`), in **`fn_5`**:
- `w145 = 0x7EE3` (`FN_HDR`), `w146 = 0x0005` (`fn_id=5`), `w147 = 0x0001` (`pcnt=1`), `w153 = 0x000D` (`icnt=13`)
- `w168..w170`: `[0x7F40, 0x7EF1, 0x7EE4]` (`LABEL L1`)
- `w188..w190`: `[0x7F41, 0x7EF3, 0x7EE4]` (`JMP L3`, target word at **`w189`**)
- `w212..w214`: `[0x7F40, 0x7EF3, 0x7EE4]` (`LABEL L3`, label-ID word at **`w213`**)

*(When patching both `w213` and `w189`, apply `w213` replacement before `w189` replacement — or adjust `w213` by the length delta at `w189` — so the two offsets do not shift each other.)*

| Probe | Exact Word-Level Patch (on `kv_ledger_service.omni`) | Pre-Fix `verifier.elf` | Post-Fix `verifier.elf` | `m2.elf` | Class (Pre → Post) |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `(e1)` orig `LBL 3 -> LBL 3` | Unmodified (`w189=0x7EF3`, `w213=0x7EF3`) | `PASS (st=0, [])` | `PASS (st=0, [])` | `PASS (rc=0, 1390 B)` | `AGREE` → `AGREE` |
| `(e2)` same-ns `IMM 3 -> IMM 3` | `w213: [0x7EF3]->[0x7EE5, 3]`, then `w189: [0x7EF3]->[0x7EE5, 3]` | `PASS (st=0, [])` | `PASS (st=0, [])` | `PASS (rc=0, 1390 B)` | `AGREE` → `AGREE` |
| `(a)` alias `def LBL 1, br IMM 1` | `w189: [0x7EF3]->[0x7EE5, 1]` (`w168..w170` defines `LBL 1`) | `PASS (st=0, [])` | `FAIL (5, [(2009, 188)])` | `FAIL (rc=1, 0 B)` | **`SOUNDNESS`** → **`AGREE`** |
| `(a')` alias `def IMM 3, br LBL 3` | `w213: [0x7EF3]->[0x7EE5, 3]` (`w189` stays `0x7EF3`) | `PASS (st=0, [])` | `FAIL (5, [(2009, 188)])` | `FAIL (rc=1, 0 B)` | **`SOUNDNESS`** → **`AGREE`** |
| `(b)` `IMM 63 -> IMM 63` *(bit 63)* | `w213: [0x7EF3]->[0x7EE5, 63]`, `w189: [0x7EF3]->[0x7EE5, 63]` | `FAIL (5, [(2009, 188)])` | `PASS (st=0, [])` | `PASS (rc=0, 1390 B)` | **`COMPLETENESS`** → **`AGREE`** |
| `(b)` `IMM 254 -> IMM 254` | `w213: [0x7EF3]->[0x7EE5, 254]`, `w189: [0x7EF3]->[0x7EE5, 254]` | `PASS (st=0, [])` | `PASS (st=0, [])` | `PASS (rc=0, 1390 B)` | `AGREE` → `AGREE` |
| `(b)` `IMM 255 -> IMM 255` *(bit 63)* | `w213: [0x7EF3]->[0x7EE5, 255]`, `w189: [0x7EF3]->[0x7EE5, 255]` | `FAIL (5, [(2009, 188)])` | `PASS (st=0, [])` | `PASS (rc=0, 1390 B)` | **`COMPLETENESS`** → **`AGREE`** |
| `(b)` `IMM 256 -> IMM 256` | `w213: [0x7EF3]->[0x7EE5, 256]`, `w189: [0x7EF3]->[0x7EE5, 256]` | `FAIL (5, [(2009, 188)])` | `PASS (st=0, [])` | `PASS (rc=0, 1390 B)` | **`COMPLETENESS`** → **`AGREE`** |
| `(b)` `IMM 300 -> IMM 300` | `w213: [0x7EF3]->[0x7EE5, 300]`, `w189: [0x7EF3]->[0x7EE5, 300]` | `FAIL (5, [(2009, 188)])` | `PASS (st=0, [])` | `PASS (rc=0, 1390 B)` | **`COMPLETENESS`** → **`AGREE`** |
| `(b)` `IMM 4079 -> IMM 4079` *(max)* | `w213: [0x7EF3]->[0x7EE5, 4079]`, `w189: [0x7EF3]->[0x7EE5, 4079]` | `FAIL (5, [(2009, 188)])` | `PASS (st=0, [])` | `PASS (rc=0, 1390 B)` | **`COMPLETENESS`** → **`AGREE`** |
| `(edge)` `LBL 0 -> LBL 0` *(slot 4080)* | `w213: [0x7EF3]->[0x7EF0]`, `w189: [0x7EF3]->[0x7EF0]` | `PASS (st=0, [])` | `PASS (st=0, [])` | `PASS (rc=0, 1390 B)` | `AGREE` → `AGREE` |
| `(edge)` `LBL 15 -> LBL 15` *(slot 4095, bit 63)* | `w213: [0x7EF3]->[0x7EFF]`, `w189: [0x7EF3]->[0x7EFF]` | `PASS (st=0, [])` | `PASS (st=0, [])` | `PASS (rc=0, 1390 B)` | `AGREE` → `AGREE` |
| `(c)` dual-ns `LBL 3` & `IMM 3` | `w153 += 2 (13->15)`; before `w212` insert `[0x7F41, 0x7EE5, 3, 0x7EE4, 0x7F40, 0x7EE5, 3, 0x7EE4]` | `PASS (st=0, [])` | `PASS (st=0, [])` | `PASS (rc=0, 1395 B)` | `AGREE` → `AGREE` |
| `(c2)` same-ns dup `LBL 3` | `w153 += 1 (13->14)`; before `w212` insert `[0x7F40, 0x7EF3, 0x7EE4]` | `PASS (st=0, [])` | `PASS (st=0, [])` | `PASS (rc=0, 1390 B)` | `AGREE` → `AGREE` |
| `(d)` undef `JMP IMM 300` | `w189: [0x7EF3]->[0x7EE5, 300]` (`w213` unchanged) | `FAIL (5, [(2009, 188)])` | `FAIL (5, [(2009, 188)])` | `FAIL (rc=1, 0 B)` | `AGREE` → `AGREE` |

*(Summary of Part A classes: **Class 1 Aliasing** = confirmed **soundness** divergence; **Class 2 Truncated IMM + bit-63 bug** = confirmed **completeness** divergence; **Class 3 Duplicates** = **AGREE**, excluded from fix scope.)*

---

## Part B — Fix, Equivalence Proof, Re-Pin, & Inventory Diff

### 1. Fix (`pure/verifier_engine.omni`) & Honest Complexity Note
Replacing the 256-bit `ws[4..7]` structure with a 4,096-slot model is larger than B3b's flag arithmetic and grows the verifier seed (`31,744 B -> 31,958 B`). Workspace words **`ws[960..1023]`** (64 i64 words = 4,096 bits, previously unused inside `init_zero_buf(8192)` between `mcnt < 512` at `ws[448..959]` and output at `ws[1024..8191]`) hold the exact 4,096-slot label bitset:
1. Added **`op_label_slot(p: i64) -> i64`**: maps `fl & 2` (`LBL 0..15`) → `4080 + tok` (`4080..4095`), `fl & 16` (`IMM 0..4079` from B3b) → `tok` (`0..4079`), and everything else → `-1`.
2. Updated **`has_label` & `set_label`**: indexed `ws[960 + (lid >> 6) * in_rng]` over `0..4095` and fixed the signed bit-63 bug in `has_label` (`1 - is_eq(bits & m, 0)` instead of `is_gt(bits & m, 0)`).
3. Updated **`clear_labels_and_regs`**: clears `ws[960..1023]` (`j = 0..63`) per function via a second `do_while` loop.
4. Updated **`record_labels_from_ops`** & **`check_branch_labels`** (`LABEL` check + branch-target loop) to map operands through `op_label_slot(p)` and raise existing error code **`2009`** on any failure.

### 2. Case-by-Case Observable Equivalence Argument (`verifier.elf` ≡ `m2.elf` on Labels)
- **Aliasing pair (`LBL k` vs `IMM k`):** `op_label_slot(LBL k) = 4080 + k` ∈ `[4080, 4095]`, while `op_label_slot(IMM k) = k` ∈ `[0, 4079]`. Their images are disjoint (`[0, 4079] ∩ [4080, 4095] = ∅`), matching `m2lbl_slot_of`, so cross-namespace define+branch fails `2009`.
- **Full IMM range & bit-63 (`0..=4079`, `63/254/255/256/300/4079`):** B3b `fl & 16` passes iff `IMM ∈ [0, 4079]`; `set_label`/`has_label` accept `slot ∈ [0, 4095]`; and `1 - is_eq(bits & m, 0)` evaluates bit `63` (`m = 1 << 63 = i64::MIN`) accurately as non-zero. Thus all `IMM 0..=4079` define and resolve identically to `m2.elf`.
- **`LBL` slot edges (`0x7EF0..=0x7EFF`):** map to `4080..=4095`; `LBL 15 (0x7EFF)` hits word `1023` bit `63`, now tested safely by `1 - is_eq(bits & m, 0)`.
- **Duplicates:** `bits | m` in `set_label` is idempotent for same-namespace duplicates (matching `m2lbl_define` first-definition-wins existence), and dual-namespace `LBL k` + `IMM k` touch disjoint slots (`4080+k` vs `k`).

### 3. Moved Pins (`bootstrap/MANIFEST.md` Discipline)

From-scratch derivation verified first on unmodified `df5105a:pure/verifier_engine.omni` (`31,744 B`, `0x00B2D6710B6D00D0` — **0-byte diff**), then re-run identically on the fixed `pure/verifier_engine.omni`:

| Artifact | Old Size / FNV-1a64 | New Size / FNV-1a64 | Reason |
| :--- | :--- | :--- | :--- |
| `pure/verifier_standalone.omni_bc` | `31,744 B` (`15,872 w`) / `0x00B2D6710B6D00D0` | **`31,958 B` (`15,979 w`) / `0xE99BF4CBF1455275`** | Step 16 Part B3c Judge Fix (4,096 non-aliasing label slots + bit-63 fix) |
| `verifier.elf` (rebuilt in `build/pure-bin/`) | `39,223 B` / `0x5CCDC9F0C0897E0B` | **`39,407 B` / `0x01F8692886A0F9E0`** | Rebuilt from new `verifier_standalone.omni_bc` via `m2.elf` |

- **Unmoved pins:** `bootstrap/m2.elf` (`126,152 B`, `0xAA5F84B6AA45BB51`, `S1==S2==S3==V`), `bootstrap/m3.elf` (`207,015 B`, `0x7397A5020895AEBF`), `m4.elf` (`91,576 B`, `0xFBA8272337B1BAC4`), all frozen-5 hashes, all other `.omni_bc` seeds, and `omnic_pure.elf` self-compile fixed point (`59,653 B`, 0-byte diff).

### 4. Permanent Step 8 Checks (`63 -> 66`) & Check 55 Verification
- **Check 64:** aliasing probes `(a)` and `(a')` rejected `err 2009`; same-ns `(e2)` `IMM 3 -> IMM 3` and `(e1)` original accepted & run `rc=0`.
- **Check 65:** `IMM 63/255/256/300/4079` and `LBL 15 (0x7EFF)` all accepted by `verifier.elf` and lowered by `m2.elf` (`rc=0`).
- **Check 66:** undef `(d)` `JMP IMM 300` rejected `err 2009`; dual-ns `(c)` `LBL 3` + `IMM 3` accepted by both `verifier.elf` and `m2.elf` (`rc=0`).
- **Check 55 verified empirically unchanged:**
  ```text
  [FUZZ FAIL] 10 mutants | 5 accepted | 5 rejected | soundness=0 roundtrip=1 judge=0
  [FUZZ FIND] kind=3 op=1 item=1 site=17 seed=977037651
  ```

### 5. Full `--campaign-all` Piped Re-Run & Line-by-Line Inventory Diff

Command: `./build/pure-bin/fuzz_gates.elf --campaign-all | cat > /tmp/s16_b3c_after.txt` (`RC = 1`):

```text
[FUZZ FAIL] 2800 mutants | 1310 accepted | 1490 rejected | soundness=31 roundtrip=143 judge=12
op=1 gen=500 acc=188 rej=312
op=2 gen=500 acc=222 rej=278
op=3 gen=500 acc=173 rej=327
op=4 gen=500 acc=382 rej=118
op=5 gen=500 acc=48 rej=452
op=6 gen=150 acc=150 rej=0
op=7 gen=150 acc=147 rej=3
```

**Line-by-line inventory diff vs PR #65 (`186 -> 186`):**
- **Unchanged corpus items `item=0..6`:** **0 closed, 0 added** (100% identical to post-B3b).
- **Re-pinned corpus item `item=7` (`pure/verifier_standalone.omni_bc`, `15,872 w -> 15,979 w`):** 1 modulus-shift delta confined to `item=7`, within already-triaged **Family F2** (unlowered M2 opcodes `0x7F49`/`0x7F47`):
  - Closed: `[FUZZ FIND] kind=1 op=1 item=7 site=14527 seed=2037675677 orig=0x7F48 mut=0x7F49`
  - Resampled: `[FUZZ FIND] kind=1 op=1 item=7 site=13969 seed=1875523199 orig=0x7F4A mut=0x7F47` *(destless unlowered `0x7F47`, Family F2)*
- **Zero new root-cause families.**

---

## Close-Out Gates (WSL2 Ubuntu Linux x86-64)

- `bash scripts/zero_rust_rebuild.sh`: **PASS** (`S1==S2==S3==V`, all 10 MANIFEST pins bit-exact)
- `bash scripts/verify_step7_driver.sh`: **38/38 PASS**
- `bash scripts/verify_step8_harness.sh`: **66/66 PASS**
- `./build/pure-bin/omnic_pure.elf selftest`: **PASS**
- `python3 scripts/external_verifier_check.py`: **PASS** (`8/8` traces)
- All 8 corpus items verified & runnable goldens executed `rc=0` (0 completeness regressions).
