# Step 16 Part B6 Evidence — Family F6 (Honest Decompiler Stamp on Non-Canonical Bytecode)

## 1. Summary & Ownership Verdict

- **Branch:** `feat/step16-f6-decompile-honesty` (branched from `main @ 008ceb8`)
- **Ownership conclusion (Part A empirical probe):**
  - Across all `118` `kind=3` lines in the `main @ 008ceb8` `--campaign-all` inventory (`130` finds total), `8` fail recompile (`rec_rc != 0`, already filed as F5c/F5d) and **`110` belong to Family F6** (`rec_rc == 0`, recompiled bytecode differs from the mutant).
  - Running `build/pure-bin/m2.elf` on **all `110/110` F6 mutants returned `rc=0` (accepted)**. Therefore, tightening `pure/verifier_engine.omni` against F6 would violate completeness (making the verifier stricter than `m2.elf`), so **all 4 F6 delta groups are 100% decompiler-owned** (`pure/audit/decompile_rhs.omni` and `pure/audit/decompile.omni`).
  - **Zero manifest pins moved:** `pure/verifier_engine.omni`, `pure/verifier_standalone.omni_bc`, `verifier.elf`, `bootstrap/m2.elf`, `bootstrap/m3.elf`, `m4.elf`, and all 5 frozen hashes remain unchanged.

## 2. Part A — Taxonomy, `m2.elf` Verdicts, Stamp Analysis (`napprox`), and Expressibility

Every one of the `110` F6 finds maps to one primary delta group (exact sum `52 + 43 + 14 + 1 = 110`, `0` missed):

| Group | Finds | Mutator Operators | `m2.elf` Verdict | M3 Source-Expressible? | Decompile Delta (`napprox` Gap) |
|---|---:|---|---|---|---|
| **F6-A** (`inline_lbl_val`) | **52** | `op=3` (`52`) | `rc=0` (`elf != rec_elf`) | **No** — M3 compiles literals `0..15` to 2-word `0x7EE5 <val>`, never 1-word inline `LBL` `0x7EF0..=0x7EFF`. | `dc_emit_op_sym` printed `t - 32496` without setting approximation flag `slot 72 = 1`. Recompile emits `dw = -1` word. |
| **F6-B** (`excess_arity`) | **43** | `op=5` dup (`24`), `op=4` (`6`), `op=1` (`13`) | `rc=0` (`elf == rec_elf`) | **No** — M3 always emits fixed canonical operand arity per opcode (ignoring/never emitting trailing duplicate/extra operands). | `dc_can_rhs` (`is_1..is_4` using `m2u_ge`) and `dc_emit_single` (`DROP`/`RET` using `m2u_ge`) emitted canonical syntax while silently dropping excess operands without setting `slot 72 = 1`. Recompile emits `dw = +1/+2` shorter word count. |
| **F6-C** (`dest_ty_mismatch`) | **14** | `op=4` (`1`), `op=1` (`13`) | `rc=0` (`elf == rec_elf`) | **No** — M3 emits each opcode's intrinsic type word (`i64`/`trit`/`bytes`/`trit_vec`/`arena`), regardless of source `let v: T` annotations on canonical opcodes. | `dc_can_rhs` ignored `dc_ins_ty(w, idx)`, stamping `RECONSTRUCTED` while M3 recompiled with the intrinsic `ty` word (`dw = 0`, 1 word differs). |
| **F6-D** (`noncanon_op_alias`) | **1** (+ co-occurs in `0x7F15`/`0x7F08`) | `op=1` (`item=1 site=25 0x7F28->0x7F09`; also `0x7F15 INEG`, `0x7F08 MOVE`) | `rc=0` (`elf == rec_elf` on `0x7F09`/`0x7F15`; `elf != rec_elf` on `0x7F08`) | **No** — M3 emits `0x7F11 ISUB` for `0 - x` (never `0x7F15 INEG`), `0x7F06 COPY` for variable moves (never `0x7F08 MOVE`), and only literals `0x7EE5..0x7EEA` on `0x7F09 CONST`. | `dc_can_rhs` accepted `32533`, `32520`, and non-literal `32521` without fallback (`slot 72 = 1`), so M3 recompiled using canonical opcodes. |
| **Total F6** | **110** | — | **110/110 `rc=0`** | **0/110** expressible | **All 110 missed by `slot 72` (`napprox`) accounting** |

In `pure/audit/decompile.omni`, `dc_emit_top_hdr` (L544) stamps `// Status: RECONSTRUCTED (exact)` whenever `napprox == 0`, where `napprox` accumulates per-function `approx = ld_u64(w, 72) | bad_fid` (L480–482). Setting `slot 72 = 1` (either directly in `dc_emit_op_sym` for inline `LBL` value operands, or via `dc_emit_gen_fallback` when `dc_can_rhs` / `dc_emit_single` rejects non-canonical arity, type, or opcode aliases) makes the `APPROXIMATE` stamp match exact recompile capability without altering any canonical program's `RECONSTRUCTED (exact)` output.

## 3. Part B Fix & Pin Verification

- **[pure/audit/decompile_rhs.omni](file:///C:/Users/Snipe/.gemini/antigravity/scratch/omni/pure/audit/decompile_rhs.omni):**
  - `dc_emit_op_sym` (L170): sets `iset_patch_i64(w, 72, 1)` when formatting inline `0x7EF0..=0x7EFF` (`32496..=32511`) as value operands (**F6-A**).
  - Added `dc_op_fixed_ty(op)` and updated `dc_can_rhs`: enforces exact arity (`m2u_eq` for `is_1..is_4`), fixed result type match (`dc_ins_ty == want_ty` when `want_ty != 0`), and canonical opcode/const form (excludes `32520`, `32533`, and non-literal `32521`) (**F6-B, F6-C, F6-D**).
- **[pure/audit/decompile.omni](file:///C:/Users/Snipe/.gemini/antigravity/scratch/omni/pure/audit/decompile.omni):**
  - `dc_emit_single` (L87, L90): restricts direct `drop(...)` and `return ...;` formatting to `m2u_eq(nops, 1)` on `DROP (32517)` and `(1 - m2u_lt(1, nops))` on `RET (32586)`, routing excess-operand forms to `dc_emit_gen_fallback` (**F6-B**).
- **Zero pins moved:** `scripts/zero_rust_rebuild.sh` verified all 10 `bootstrap/MANIFEST.md` pins bit-exact, `S1 == S2 == S3 == bootstrap/m2.elf` (`126,152 B`, `0xAA5F84B6AA45BB51`), `m4.elf` (`91,576 B`), `verifier.elf` (`40,562 B`, `0xB3A9FFC5328CAFE4`), `m3.elf` (`207,015 B`), and `omnic_pure.elf` fixed point (`59,653 B`, 0-byte diff).

## 4. Check 55 Transition

- **Previous Check 55 (`main @ 008ceb8`):**
  `[FUZZ FIND] kind=3 op=1 item=1 site=17 seed=977037651` (`soundness=0 roundtrip=1 judge=0`)
- **New Check 55 (reduced `./build/pure-bin/fuzz_gates.elf` verbatim):**
  ```text
  [FUZZ FAIL] 300 mutants | 143 accepted | 157 rejected | soundness=0 roundtrip=0 judge=1
  op=1 gen=48 acc=20 rej=28
  op=2 gen=48 acc=14 rej=34
  op=3 gen=48 acc=13 rej=35
  op=4 gen=48 acc=32 rej=16
  op=5 gen=48 acc=4 rej=44
  op=6 gen=30 acc=30 rej=0
  op=7 gen=30 acc=30 rej=0
  [FUZZ FIND] kind=5 op=7 item=3 site=1091 seed=1910166380
  ```
  All 240 `op=1..5` and 30 `op=6` mutants in the reduced panel passed with `soundness=0 roundtrip=0`, advancing Check 55 to the first **Family F7** (judge-integrity) find (`kind=5 op=7 item=3 site=1091 seed=1910166380`).

## 5. Step 8 Controls (Checks 70–73) & Corpus Round-Trip

- `build/pure-bin/audit_view_gate.elf`: **27 checks PASS** (all 16 goldens byte-identical round-trip).
- **8-corpus verification (`audit_decompile.elf` + `omnic_pure.elf compile`):**
  - `seed0_risk.omni`: `RECONSTRUCTED (exact)` (`53w`, `0x9866061821D06A54`), `rec_rc=0`, `byte_eq=True`
  - `seed4_consensus.omni`: `RECONSTRUCTED (exact)` (`34w`, `0x2A4989813A2DB173`), `rec_rc=0`, `byte_eq=True`
  - `seed6_clamp.omni`: `RECONSTRUCTED (exact)` (`34w`, `0xD8D650C0BA199DDA`), `rec_rc=0`, `byte_eq=True`
  - `kv_ledger_service.omni`, `pack5_decision_controller.omni`, `compiler_self_audit.omni`, `zero_rust_driver.omni`, `verifier_standalone.omni_bc`: all stamped `APPROXIMATE` as before (no regressions).
- **Step 8 Controls (`scripts/verify_step8_harness.sh`):**
  - **Check 70 (F6-A):** `seed4_consensus` w28 (`0x7E00 -> 0x7EFD`) stamped `APPROXIMATE`; orig `RECONSTRUCTED (exact)` + byte-exact recompile.
  - **Check 71 (F6-B):** `seed4_consensus` RET w31 duplicated (`nops=2`) & `seed6_clamp` w24 (`0x7F17 -> 0x7F16` `iabs`, `nops=2`) stamped `APPROXIMATE`; orig `RECONSTRUCTED (exact)`.
  - **Check 72 (F6-C):** `seed4_consensus` w18 (`0x7F83 -> 0x7F84` `tmaj` `ty=str`) stamped `APPROXIMATE`; orig `RECONSTRUCTED (exact)`.
  - **Check 73 (F6-D):** `seed4_consensus` w25 (`0x7F28 -> 0x7F09` reg-`CONST`) & w17 (`0x7F2C -> 0x7F15` `INEG`) stamped `APPROXIMATE`; orig `RECONSTRUCTED (exact)`.

## 6. Piped `--campaign-all` Inventory Diff vs `main @ 008ceb8`

- **Before (`main @ 008ceb8`):** `2800 mutants | 1224 accepted | 1576 rejected | soundness=0 roundtrip=118 judge=12` (`130` finds)
- **After (`feat/step16-f6-decompile-honesty`):** `2800 mutants | 1224 accepted | 1576 rejected | soundness=0 roundtrip=3 judge=12` (`15` finds)
- **Line-by-line delta:** **115 closed, 0 added**:
  - **110** Family F6 lines (`52` F6-A + `43` F6-B + `14` F6-C + `1` F6-D)
  - **5** mechanically-entailed F5c/F5d lines that also violated F6-B or F6-C:
    - `kind=3 op=1 item=0 site=22 seed=1395293013 orig=0x7F10 mut=0x7F55` (`s2i` with `nops=2 > 1`)
    - `kind=3 op=1 item=1 site=25 seed=336143379 orig=0x7F28 mut=0x7F21` (`bnot` with `ty=trit != i64`)
    - `kind=3 op=1 item=0 site=29 seed=524629877 orig=0x7F2D mut=0x7F57` (`san_sql` with `nops=2 > 1`)
    - `kind=3 op=1 item=0 site=22 seed=402146181 orig=0x7F10 mut=0x7F58` (`san_dom` with `nops=2 > 1`)
    - `kind=3 op=1 item=2 site=24 seed=806362993 orig=0x7F17 mut=0x7F57` (`san_sql` with `nops=2 > 1`)
- **All 15 remaining `[FUZZ FIND]` lines (3 pure F5c/F5d + 12 F7):**
  ```text
  [FUZZ FIND] kind=3 op=1 item=0 site=37 seed=563097424 orig=0x7F09 mut=0x7F55
  [FUZZ FIND] kind=3 op=1 item=0 site=22 seed=1254987038 orig=0x7F10 mut=0x7F51
  [FUZZ FIND] kind=3 op=1 item=1 site=25 seed=797234575 orig=0x7F28 mut=0x7F58
  [FUZZ FIND] kind=5 op=7 item=3 site=914 seed=1291295325 orig=0x7EE5 mut=0x7EE5
  [FUZZ FIND] kind=5 op=7 item=4 site=1576 seed=1930264012 orig=0x7EE5 mut=0x7EE5
  [FUZZ FIND] kind=5 op=7 item=5 site=1320 seed=1895835354 orig=0x7EE5 mut=0x7EE5
  [FUZZ FIND] kind=5 op=7 item=6 site=2661 seed=78452350 orig=0x7EE5 mut=0x7EE5
  [FUZZ FIND] kind=5 op=7 item=3 site=108 seed=2025541501 orig=0x7EE5 mut=0x7EE5
  [FUZZ FIND] kind=5 op=7 item=4 site=2013 seed=1868269530 orig=0x7EE5 mut=0x7EE5
  [FUZZ FIND] kind=5 op=7 item=5 site=1650 seed=240791725 orig=0x7EE5 mut=0x7EE5
  [FUZZ FIND] kind=5 op=7 item=6 site=2706 seed=1805499307 orig=0x7EE5 mut=0x7EE5
  [FUZZ FIND] kind=5 op=7 item=3 site=966 seed=427595245 orig=0x7EE5 mut=0x7EE5
  [FUZZ FIND] kind=5 op=7 item=4 site=831 seed=1142443988 orig=0x7EE5 mut=0x7EE5
  [FUZZ FIND] kind=5 op=7 item=5 site=1711 seed=321870717 orig=0x7EE5 mut=0x7EE5
  [FUZZ FIND] kind=5 op=7 item=6 site=2952 seed=802789306 orig=0x7EE5 mut=0x7EE5
  ```

## 7. Close-Out Gates Verified

- `PATH=/usr/bin:/bin bash scripts/zero_rust_rebuild.sh` — **PASS**
- `PATH=/usr/bin:/bin bash scripts/verify_step7_driver.sh` — **PASS=38 FAIL=0**
- `PATH=/usr/bin:/bin bash scripts/verify_step8_harness.sh` — **PASS=73 FAIL=0**
- `./build/pure-bin/omnic_pure.elf selftest` — **PASS**
- `python3 scripts/external_verifier_check.py` — **PASS (8/8)**
