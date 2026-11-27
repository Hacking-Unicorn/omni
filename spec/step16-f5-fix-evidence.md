# Step 16 Part B4 Evidence — Family F5 (Undefined Register in `RET` & `void` Dest Type)

## 1. Summary

- **Branch:** `feat/step16-f5-reg-void-fix` (branched from `main @ af8d0b8`)
- **Scope:** Judge PR under `bootstrap/MANIFEST.md` discipline closing all verifier-owned **Family F5** finds in `pure/verifier_engine.omni`:
  1. **F5a (`handle_ret`):** Check register state (`rst = sl_state(pack)`) on slot operands in `RET` (`0x7F4A`) and reject undefined / moved / borrowed / dropped slots (`rst` in `{0, 3, 4, 5}`) with existing error code `1005` (`BORROW_CONFLICT`), matching `handle_dest` and `handle_drop`.
  2. **F5b (`verify_ins_body`):** Extract the instruction destination type word `ty = is_dest * load_u16(stream, pos + 2) + (1 - is_dest) * 32640` and reject `void` (`32640` / `0x7F80`) on value-producing destination instructions (`do_dest = known * (1 - skip) * op_has_dest(op) * is_dest`) with existing error code `2012`.

## 2. Clean-Room Derivation & Pin Table

Baseline reproducibility check on `af8d0b8:pure/verifier_engine.omni` reproduced `pure/verifier_standalone.omni_bc` bit-exact (`31,958 B`, `15,979` words, `FNV-1a64 = 0xE99BF4CBF1455275`) before applying the F5 edits.

| Artifact | Before (`main @ af8d0b8`) | After (`feat/step16-f5-reg-void-fix`) | Status |
|---|---|---|---|
| `pure/verifier_standalone.omni_bc` | `31,958 B` (`15,979w`), `0xE99BF4CBF1455275` | `32,286 B` (`16,143w`), `0xEA324F363AC8D90E` | **Moved (Judge PR)** |
| `verifier.elf` (rebuilt via `m2.elf`) | `39,407 B`, `0x01F8692886A0F9E0` | `39,891 B`, `0x48E5270D8B93F4F1` | **Moved (Judge PR)** |
| `bootstrap/m2.elf` (`S1 == S2 == S3 == V`) | `126,152 B`, `0xAA5F84B6AA45BB51` | `126,152 B`, `0xAA5F84B6AA45BB51` | Unchanged |
| `bootstrap/m3.elf` (linked ELF fixed point) | `207,015 B`, `0x7397A5020895AEBF` | `207,015 B`, `0x7397A5020895AEBF` | Unchanged |
| `m4.elf` (`pure/m4/m4_standalone.omni_bc`) | `91,576 B`, `0xFBA8272337B1BAC4` | `91,576 B`, `0xFBA8272337B1BAC4` | Unchanged |
| All 5 frozen hashes (`core_gates`) | All pass | All pass | Unchanged |

Self-verification on `pure/verifier_standalone.omni_bc`: `status=0`, `dec_off=0`, `nfns=69`, `all_pass=True`.

## 3. Family F5 Triage & 4-Way Ownership Split (All 33 Finds)

All 67 `kind=3` lines across `op in {1, 2, 4}` in the `main @ af8d0b8` `--campaign-all` baseline (`186` finds) split into **33 F5** (`rec_rc=1`) and **34 F6** (`rec_rc=0`, including Check 55 `kind=3 op=1 item=1 site=17 seed=977037651` `0x7F2C->0x7F15`).

### 3.1 F5a — Undefined Register in `RET` (22 Finds, Closed Here)

All 21 `kind=3 op=2` lines plus 1 `kind=3 op=4` line:
```text
[FUZZ FIND] kind=3 op=2 item=1 site=31 seed=912745351 orig=0x7E01 mut=0x7E02
[FUZZ FIND] kind=3 op=2 item=0 site=50 seed=1708718461 orig=0x7E04 mut=0x7E05
[FUZZ FIND] kind=3 op=2 item=2 site=23 seed=1296557268 orig=0x7E01 mut=0x7E00
[FUZZ FIND] kind=3 op=2 item=2 site=31 seed=1156557997 orig=0x7E01 mut=0x7E02
[FUZZ FIND] kind=3 op=2 item=2 site=31 seed=1274294033 orig=0x7E01 mut=0x7E02
[FUZZ FIND] kind=3 op=2 item=0 site=42 seed=1520414370 orig=0x7E04 mut=0x7E03
[FUZZ FIND] kind=3 op=2 item=2 site=31 seed=986948383 orig=0x7E01 mut=0x7E02
[FUZZ FIND] kind=3 op=2 item=1 site=24 seed=440940292 orig=0x7E01 mut=0x7E00
[FUZZ FIND] kind=3 op=2 item=2 site=23 seed=20518368 orig=0x7E01 mut=0x7E00
[FUZZ FIND] kind=3 op=2 item=2 site=23 seed=1968680204 orig=0x7E01 mut=0x7E00
[FUZZ FIND] kind=3 op=2 item=1 site=24 seed=1706160380 orig=0x7E01 mut=0x7E00
[FUZZ FIND] kind=3 op=2 item=2 site=23 seed=1257964598 orig=0x7E01 mut=0x7E00
[FUZZ FIND] kind=3 op=2 item=0 site=42 seed=86154694 orig=0x7E04 mut=0x7E03
[FUZZ FIND] kind=3 op=2 item=2 site=23 seed=1745838314 orig=0x7E01 mut=0x7E00
[FUZZ FIND] kind=3 op=2 item=1 site=31 seed=778309953 orig=0x7E01 mut=0x7E02
[FUZZ FIND] kind=3 op=2 item=0 site=42 seed=1280250032 orig=0x7E04 mut=0x7E03
[FUZZ FIND] kind=3 op=2 item=2 site=23 seed=72916338 orig=0x7E01 mut=0x7E00
[FUZZ FIND] kind=3 op=2 item=1 site=31 seed=1143678439 orig=0x7E01 mut=0x7E02
[FUZZ FIND] kind=3 op=2 item=1 site=31 seed=457505511 orig=0x7E01 mut=0x7E02
[FUZZ FIND] kind=3 op=2 item=2 site=31 seed=424702789 orig=0x7E01 mut=0x7E02
[FUZZ FIND] kind=3 op=2 item=1 site=31 seed=131485579 orig=0x7E01 mut=0x7E02
[FUZZ FIND] kind=3 op=4 item=2 site=23 seed=1805374132 orig=0x7E01 mut=0x7E00
```

### 3.2 F5b — `void` (`0x7F80`) Return Type on Value-Producing Dest Op (3 Finds, Closed Here)

```text
[FUZZ FIND] kind=3 op=4 item=2 site=25 seed=877594268 orig=0x7F81 mut=0x7F80
[FUZZ FIND] kind=3 op=4 item=2 site=25 seed=1982411308 orig=0x7F81 mut=0x7F80
[FUZZ FIND] kind=3 op=4 item=2 site=25 seed=2134189284 orig=0x7F81 mut=0x7F80
```

### 3.3 F5c — Unlowered-in-M2 Ops (5 `op=1` Finds, Out of Scope — Next PR: Family F2)

These valid opcodes are currently unlowered in `pure/m2/` (`driver.omni`, `emit_x86.omni`, `emit_multi.omni`) and deferred to the upcoming Family F2 PR:
```text
[FUZZ FIND] kind=3 op=1 item=0 site=22 seed=1254987038 orig=0x7F10 mut=0x7F51
[FUZZ FIND] kind=3 op=1 item=0 site=22 seed=1395293013 orig=0x7F10 mut=0x7F55
[FUZZ FIND] kind=3 op=1 item=0 site=37 seed=563097424 orig=0x7F09 mut=0x7F55
[FUZZ FIND] kind=3 op=1 item=0 site=22 seed=402146181 orig=0x7F10 mut=0x7F58
[FUZZ FIND] kind=3 op=1 item=1 site=25 seed=797234575 orig=0x7F28 mut=0x7F58
```

### 3.4 F5d — Decompiler-Owned Syntax Emission Bugs (3 `op=1` Finds, Out of Scope — Separate Decompiler PR)

These M2-lowered opcodes are valid in `.omni_bc` and pass M2 cleanly, but `pure/audit/decompile_rhs.omni` emits non-M3 function-call syntax (`bnot(v0)` at L333 instead of `~v0`, `san_sql(v1)` at L362) while `pure/audit/decompile.omni:544` stamps `RECONSTRUCTED` whenever `napprox == 0`:
```text
[FUZZ FIND] kind=3 op=1 item=1 site=25 seed=336143379 orig=0x7F28 mut=0x7F21
[FUZZ FIND] kind=3 op=1 item=0 site=29 seed=524629877 orig=0x7F2D mut=0x7F57
[FUZZ FIND] kind=3 op=1 item=2 site=24 seed=806362993 orig=0x7F17 mut=0x7F57
```

## 4. Check 55 Status

Reduced `./build/pure-bin/fuzz_gates.elf` still reproduces Check 55:
`[FUZZ FIND] kind=3 op=1 item=1 site=17 seed=977037651` (`0x7F2C -> 0x7F15`), which classified as **F6** (`rec_rc=0`), outside this PR.

## 5. Corpus Completeness & Step 8 Negative Controls

- **Corpus completeness (0..7):** All 8 corpus targets compiled/verified with `status=0, all_pass=True`:
  - `spec/acceptance/goldens/seed0_risk.omni` (`nfns=1`)
  - `spec/acceptance/goldens/seed4_consensus.omni` (`nfns=1`)
  - `spec/acceptance/goldens/seed6_clamp.omni` (`nfns=1`)
  - `pure/programs/kv_ledger_service.omni` (`nfns=11`)
  - `pure/programs/pack5_decision_controller.omni` (`nfns=13`)
  - `pure/programs/compiler_self_audit.omni` (`nfns=34`)
  - `pure/m5/zero_rust_driver.omni` (`nfns=6`)
  - `pure/verifier_standalone.omni_bc` (`nfns=69`)
- **Step 8 Check 67 (`F5a`):** `seed4_consensus` w31 `0x7E01 -> 0x7E02` rejected with `1005` at `RET` offset `30`; original accepted & lowered cleanly (`verifier.elf` returns `[0, 0, 1, 4, 1, 0]`).
- **Step 8 Check 68 (`F5b`):** `seed6_clamp` w25 `0x7F81 -> 0x7F80` rejected with `2012` at offset `23` (and w23 `0x7E01 -> 0x7E00` rejected with `1005` at offset `30`); original accepted & lowered cleanly (`verifier.elf` returns `[0, 0, 1, 6, 1, 0]`).

## 6. Piped `--campaign-all` Inventory Diff vs `main @ af8d0b8`

- **Before (`main @ af8d0b8`):** `2800 mutants | 1310 accepted | 1490 rejected | soundness=31 roundtrip=143 judge=12` (`186` total `[FUZZ FIND]` lines)
- **After (`feat/step16-f5-reg-void-fix`):** `2800 mutants | 1246 accepted | 1554 rejected | soundness=27 roundtrip=118 judge=12` (`157` total `[FUZZ FIND]` lines)
- **Line-by-line delta:** **29 closed, 0 added**:
  - 22 **F5a** finds (`21 op=2` + `1 op=4`)
  - 3 **F5b** finds (`3 op=4`)
  - 4 bonus `kind=1 op=1` soundness finds (`item=1 site=25 seed=1148979964 0x7F28->0x7F4F`, `item=7 site=13969 seed=1875523199 0x7F4A->0x7F47`, `item=6 site=703 seed=1886819068 0x7F48->0x7F4F`, `item=2 site=24 seed=2035476212 0x7F17->0x7F4F`)

## 7. Close-Out Gates Verified

- `PATH=/usr/bin:/bin bash scripts/zero_rust_rebuild.sh` — **PASS**
- `PATH=/usr/bin:/bin bash scripts/verify_step7_driver.sh` — **PASS=38 FAIL=0**
- `PATH=/usr/bin:/bin bash scripts/verify_step8_harness.sh` — **PASS=68 FAIL=0**
- `./build/pure-bin/omnic_pure.elf selftest` — **PASS**
- `python3 scripts/external_verifier_check.py` — **PASS (8/8)**
