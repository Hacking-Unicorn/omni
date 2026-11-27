# Step 16 Part B3 — Judge PR Evidence: Family F1 (Unassigned Sanitizer Ops `0x7F5A`–`0x7F5C`)

- **Branch:** `feat/step16-f1-unassigned-ops-fix` (branched off `main @ 8c0d3a1`, after PR #62 merged)
- **PR Type:** **Judge PR** under `bootstrap/MANIFEST.md` discipline
- **Host:** WSL2 Ubuntu Linux x86-64
- **Milestone:** **Find #2 (`seed4_consensus` word `30: 0x7F4A -> 0x7F5B`) is now FULLY CLOSED** — both its F3 dest-shape component (PR #62) and its F1 unassigned-opcode component (`0x7F5B`, this PR) are rejected fail-closed.

---

## 1. All 12 Post-B2 Verbatim Family F1 Member Lines (`spec/step16-f3-fix-evidence.md` Baseline)

```text
[FUZZ FIND] kind=1 op=1 item=3 site=341 seed=849667523 orig=0x7F2C mut=0x7F5C
[FUZZ FIND] kind=1 op=1 item=7 site=476 seed=701268424 orig=0x7F2D mut=0x7F5C
[FUZZ FIND] kind=1 op=1 item=0 site=37 seed=686289333 orig=0x7F09 mut=0x7F5B
[FUZZ FIND] kind=1 op=1 item=6 site=448 seed=1175267274 orig=0x7F73 mut=0x7F5C
[FUZZ FIND] kind=1 op=1 item=6 site=411 seed=2036886458 orig=0x7F73 mut=0x7F5B
[FUZZ FIND] kind=1 op=1 item=0 site=43 seed=2100045222 orig=0x7F29 mut=0x7F5A
[FUZZ FIND] kind=1 op=1 item=0 site=29 seed=225430957 orig=0x7F2D mut=0x7F5A
[FUZZ FIND] kind=1 op=1 item=3 site=53 seed=665768397 orig=0x7F16 mut=0x7F5C
[FUZZ FIND] kind=1 op=1 item=5 site=959 seed=1650239938 orig=0x7F2D mut=0x7F5C
[FUZZ FIND] kind=1 op=1 item=6 site=570 seed=594783669 orig=0x7F48 mut=0x7F5B
[FUZZ FIND] kind=1 op=1 item=1 site=25 seed=265803201 orig=0x7F28 mut=0x7F5C
[FUZZ FIND] kind=1 op=1 item=0 site=37 seed=1056065988 orig=0x7F09 mut=0x7F5C
```

---

## 2. Root Cause, Sanitizer Audit, & Minimal Fix (`pure/verifier_engine.omni`, +1 line)

- **Why `op_is_sanitizer` exists & what consumes it:** `pure/verifier_engine.omni::op_is_sanitizer` (L332–342) has a single caller at `handle_dest` (L844: `let san: i64 = op_is_sanitizer(op); let upgrade: i64 = (1 - san) * any * (1 - ins_untr); w = append_err_if(w, upgrade, 1002, off);`), where it exempts sanitizer opcodes (`0x7F57`–`0x7F59` and `0x7F70`) from the `@UNTRUSTED -> clean` taint-upgrade error (`1002`).
- **Can any legitimate stream contain `0x7F5A`–`0x7F5C` (`32602`–`32604`) in a sanitizer role?** **No.** Repo-wide audit confirms zero ISA mnemonics in `pure/isa/table.omni`, zero emission in `pure/m3/`, zero lowering in `pure/m2/`, zero dispatch in `pure/m4/`, zero support in `pure/audit/decompile.omni`, and scanning every committed `.omni_bc` file under `pure/` finds `0x7F5A`/`0x7F5B`/`0x7F5C` exclusively as integer literal constants inside `verifier_standalone.omni_bc` (`op_is_sanitizer` words `3863, 3888, 3913`).
- **Why they passed validation:** `op_is_known` (L172–179) excluded Step 15's 18 holes but omitted `32602..=32604` (`0x7F5A..=0x7F5C`).
- **Minimal Fix (`pure/verifier_engine.omni::op_is_known`, L176):**
  ```omni
      let hole: i64 = is_eq(op, 32519) + in_range(op, 32522, 32524)
          + in_range(op, 32570, 32574) + in_range(op, 32587, 32590)
          + in_range(op, 32594, 32596) + in_range(op, 32602, 32604)
          + in_range(op, 32606, 32607);
  ```
- **Error Code:** Existing `3001` (`verify_ins_body` L948: `append_err_if(w, 1 - known, 3001, ins_off)`). Zero other opcodes altered; `op_is_sanitizer` untouched.

---

## 3. Moved Pins (`bootstrap/MANIFEST.md` Discipline)

From-scratch derivation verified first on unmodified `8c0d3a1:pure/verifier_engine.omni` (`31,526 B`, `0x485D1C5C2D0A69C6` — **0-byte diff** vs committed seed), then re-run identically on the fixed `pure/verifier_engine.omni`:

| Artifact | Old Size / FNV-1a64 | New Size / FNV-1a64 | Reason |
| :--- | :--- | :--- | :--- |
| `pure/verifier_standalone.omni_bc` | `31,526 B` (`15,763 w`) / `0x485D1C5C2D0A69C6` | **`31,564 B` (`15,782 w`) / `0x061DACCE78F5ADDB`** | Step 16 Part B3 Judge Fix (Family F1) |
| `verifier.elf` (rebuilt in `build/pure-bin/`) | `38,895 B` / `0xB60283901F089BE9` | **`38,943 B` / `0x1D4E0C270335720E`** | Rebuilt from new `verifier_standalone.omni_bc` via `m2.elf` |

- **Unmoved pins:** `bootstrap/m2.elf` (`126,152 B`, `0xAA5F84B6AA45BB51`, `S1==S2==S3==V`), `bootstrap/m3.elf` (`207,015 B`, `0x7397A5020895AEBF`), `m4.elf` (`91,576 B`, `0xFBA8272337B1BAC4`), all frozen-5 hashes, all other `.omni_bc` seeds, and `omnic_pure.elf` self-compile fixed point (`59,653 B`, 0-byte diff).

---

## 4. Permanent Negative Control & Check 55 Verification

- **Step 8 Check 60 (suite `59 -> 60`):** `kv_ledger_service.omni` word `179` (byte offset `358`, dest-ful slot) mutated to `0x7F5A`, `0x7F5B`, and `0x7F5C` are **all three** rejected by `verifier.elf` with `err_code=3001` at `err_off=178` (`fn_5`); original `kv_ledger_service` verifies (`0` errors) and runs (`rc=0`).
- **Check 55 verified empirically unchanged:**
  ```text
  [FUZZ FAIL] 10 mutants | 5 accepted | 5 rejected | soundness=0 roundtrip=1 judge=0
  [FUZZ FIND] kind=3 op=1 item=1 site=17 seed=977037651
  ```

---

## 5. Full `--campaign-all` Piped Re-Run & Line-by-Line Inventory Diff

Command: `./build/pure-bin/fuzz_gates.elf --campaign-all | cat > /tmp/s16_f1_after.txt` (`RC = 1`):

```text
[FUZZ FAIL] 2800 mutants | 1315 accepted | 1485 rejected | soundness=31 roundtrip=143 judge=12
op=1 gen=500 acc=191 rej=309
op=2 gen=500 acc=218 rej=282
op=3 gen=500 acc=178 rej=322
op=4 gen=500 acc=382 rej=118
op=5 gen=500 acc=49 rej=451
op=6 gen=150 acc=150 rej=0
op=7 gen=150 acc=147 rej=3
```

### Line-by-Line Inventory Diff vs PR #62 (`198 -> 186`)

- **Closed on unchanged corpus items `item=0..6` (11 lines, 0 new finds):** exact 11 F1 lines on `item=0..6` from §1 above, zero other lines changed.
- **Re-pinned corpus item `item=7` (`pure/verifier_standalone.omni_bc`, `15,763 w -> 15,782 w`):**
  - **Closed (2 lines):**
    - `[FUZZ FIND] kind=1 op=1 item=7 site=476 seed=701268424 orig=0x7F2D mut=0x7F5C` *(12th F1 member, genuinely closed by this fix)*
    - `[FUZZ FIND] kind=1 op=1 item=7 site=13970 seed=1875523199 orig=0x7F41 mut=0x7F47` *(PR #62 `item=7` F2 hit shifted off by `nw` modulus change)*
  - **Resampled (1 line, existing Family F2 — 0 new families):**
    - `[FUZZ FIND] kind=1 op=1 item=7 site=14617 seed=2037675677 orig=0x7F48 mut=0x7F49` *(existing Family F2 unlowered `0x7F49`, same seed `2037675677`)*

---

## 6. Close-Out Gates (WSL2 Ubuntu Linux x86-64)

- `bash scripts/zero_rust_rebuild.sh`: **PASS** (`S1==S2==S3==V`, all 10 MANIFEST pins bit-exact)
- `bash scripts/verify_step7_driver.sh`: **38/38 PASS**
- `bash scripts/verify_step8_harness.sh`: **60/60 PASS**
- `./build/pure-bin/omnic_pure.elf selftest`: **PASS**
- `python3 scripts/external_verifier_check.py`: **PASS** (`8/8` traces)
- All 8 corpus items verified & runnable goldens executed `rc=0` (0 completeness regressions).
