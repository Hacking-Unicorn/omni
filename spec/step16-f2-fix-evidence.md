# Step 16 Part B5 Evidence — Family F2 (Align Verifier with M2 `mod_check.omni` Opcode Set & Final-`RET` Rule)

## 1. Summary

- **Branch:** `feat/step16-f2-unlowered-ops-fix` (branched from `main @ 07c4e5e`)
- **Scope:** Judge PR under `bootstrap/MANIFEST.md` discipline closing **100% of Family F2** (`soundness=27 -> 0` across all 2,800 `--campaign-all` mutants) by aligning `pure/verifier_engine.omni` with M2's authoritative validation in `pure/m2/verify/mod_check.omni`:
  1. **`pure/verifier_engine.omni::op_is_known`:** Aligned the allowed opcode block (`0x7F00..=0x7F7F`, `32512..=32639`) from `107` ops to the exact `83`-opcode set accepted by `pure/m2/verify/mod_check.omni::m2v_ins_min_ops`, rejecting all `24` unaccepted opcodes (`0x7F19..=0x7F1D`, `0x7F32..=0x7F33`, `0x7F38`, `0x7F47`, `0x7F49`, `0x7F4F`, `0x7F60..=0x7F62`, `0x7F71..=0x7F72`, `0x7F74..=0x7F75`, `0x7F77..=0x7F7C`) with existing error code `3001`. Note: `0x7F64` (`OP_DB_GET`), `0x7F05` (`DROP`), `0x7F34`, `0x7F65`, `0x7F6B..=0x7F6E`, and `0x7F70` remain in the `83`-op `m2v_ins_min_ops` set because `pure/programs/kv_ledger_service.omni`, `pure/programs/pack5_decision_controller.omni`, and `pure/harness/core_gates.omni::cg_gate_ver_and_mac` (`b2-pure-verifier`) require them.
  2. **`pure/verifier_engine.omni::verify_ins_body`:** Tracked `last_op` and `last_off` per decoded instruction and rejected non-decode-error functions whose final instruction is not `RET (0x7F4A = 32586)` (`no_ret = (1 - is_eq(ws_get(w, 10), 3006)) * (1 - is_eq(last_op, 32586))`), matching `pure/m2/verify/mod_check.omni:273` with existing error code `2012`.

## 2. Clean-Room Derivation & Pin Table

Baseline reproducibility check on `07c4e5e:pure/verifier_engine.omni` reproduced `pure/verifier_standalone.omni_bc` bit-exact (`32,286 B`, `16,143` words, `FNV-1a64 = 0xEA324F363AC8D90E`) before applying the F2 edits.

| Artifact | Before (`main @ 07c4e5e`) | After (`feat/step16-f2-unlowered-ops-fix`) | Status |
|---|---|---|---|
| `pure/verifier_standalone.omni_bc` | `32,286 B` (`16,143w`), `0xEA324F363AC8D90E` | `32,760 B` (`16,380w`), `0x676886F908AD2887` | **Moved (Judge PR)** |
| `verifier.elf` (rebuilt via `m2.elf`) | `39,891 B`, `0x48E5270D8B93F4F1` | `40,562 B`, `0xB3A9FFC5328CAFE4` | **Moved (Judge PR)** |
| `bootstrap/m2.elf` (`S1 == S2 == S3 == V`) | `126,152 B`, `0xAA5F84B6AA45BB51` | `126,152 B`, `0xAA5F84B6AA45BB51` | Unchanged |
| `bootstrap/m3.elf` (linked ELF fixed point) | `207,015 B`, `0x7397A5020895AEBF` | `207,015 B`, `0x7397A5020895AEBF` | Unchanged |
| `m4.elf` (`pure/m4/m4_standalone.omni_bc`) | `91,576 B`, `0xFBA8272337B1BAC4` | `91,576 B`, `0xFBA8272337B1BAC4` | Unchanged |
| All 5 frozen hashes (`core_gates`) | All pass | All pass | Unchanged |

Self-verification on `pure/verifier_standalone.omni_bc`: `status=0`, `dec_off=0`, `nfns=69`, `all_pass=True`.
Negative check `b2-pure-verifier` (`OP_DB_GET = 32612` with `cap_mask=0`): `[0, 0, 1, 0, 0, 1, 1003, 7]` (`PASS`).

## 3. Closed Finds Inventory (All 27 `kind=1` Soundness Finds, `closed=27`, `added=0`)

- **25 unaccepted-opcode finds (`3001`, excluded from `op_is_known` to match `m2v_ins_min_ops`):**
  - 22 `op=1` finds (`mut` in `0x7F19..=0x7F1D`, `0x7F33`, `0x7F38`, `0x7F47`, `0x7F49`, `0x7F4F`)
  - 3 `op=4` finds (`mut=0x7F49` on `item=1,2 site=30`)
- **2 final-instruction non-`RET` finds (`2012`, `last_op != 32586`):**
  - `item=6 site=110 seed=801329670 orig=0x7F4A mut=0x7F00`
  - `item=0 site=49 seed=1173168749 orig=0x7F4A mut=0x7F46`

Complete closed list verbatim from `--campaign-all` diff (`157 -> 130`):
```text
[FUZZ FIND] kind=1 op=1 item=3 site=79 seed=2106444702 orig=0x7F45 mut=0x7F19
[FUZZ FIND] kind=1 op=1 item=0 site=29 seed=660081049 orig=0x7F2D mut=0x7F19
[FUZZ FIND] kind=1 op=1 item=6 site=577 seed=2139679140 orig=0x7F73 mut=0x7F1A
[FUZZ FIND] kind=1 op=1 item=5 site=58 seed=1532752537 orig=0x7F4A mut=0x7F49
[FUZZ FIND] kind=1 op=1 item=4 site=891 seed=1634624305 orig=0x7F48 mut=0x7F33
[FUZZ FIND] kind=1 op=1 item=5 site=1276 seed=661206981 orig=0x7F10 mut=0x7F1C
[FUZZ FIND] kind=1 op=1 item=1 site=30 seed=1453507189 orig=0x7F4A mut=0x7F47
[FUZZ FIND] kind=1 op=1 item=4 site=935 seed=1384158088 orig=0x7F2C mut=0x7F38
[FUZZ FIND] kind=1 op=1 item=3 site=122 seed=381000605 orig=0x7F10 mut=0x7F19
[FUZZ FIND] kind=1 op=1 item=5 site=859 seed=1115846069 orig=0x7F11 mut=0x7F1B
[FUZZ FIND] kind=1 op=1 item=3 site=264 seed=1040858532 orig=0x7F2C mut=0x7F1A
[FUZZ FIND] kind=1 op=1 item=3 site=417 seed=1524697046 orig=0x7F12 mut=0x7F1D
[FUZZ FIND] kind=1 op=1 item=6 site=224 seed=77687538 orig=0x7F4A mut=0x7F4F
[FUZZ FIND] kind=1 op=1 item=0 site=22 seed=1662285741 orig=0x7F10 mut=0x7F1A
[FUZZ FIND] kind=1 op=1 item=1 site=30 seed=308041328 orig=0x7F4A mut=0x7F47
[FUZZ FIND] kind=1 op=1 item=4 site=467 seed=48622983 orig=0x7F10 mut=0x7F38
[FUZZ FIND] kind=1 op=1 item=2 site=30 seed=1926868601 orig=0x7F4A mut=0x7F47
[FUZZ FIND] kind=1 op=1 item=5 site=1493 seed=1787607198 orig=0x7F06 mut=0x7F49
[FUZZ FIND] kind=1 op=1 item=6 site=684 seed=300548056 orig=0x7F10 mut=0x7F1D
[FUZZ FIND] kind=1 op=1 item=3 site=312 seed=1215401934 orig=0x7F2C mut=0x7F1C
[FUZZ FIND] kind=1 op=1 item=0 site=29 seed=1993885131 orig=0x7F2D mut=0x7F1C
[FUZZ FIND] kind=1 op=1 item=4 site=27 seed=404347865 orig=0x7F13 mut=0x7F1D
[FUZZ FIND] kind=1 op=1 item=6 site=110 seed=801329670 orig=0x7F4A mut=0x7F00
[FUZZ FIND] kind=1 op=1 item=0 site=49 seed=1173168749 orig=0x7F4A mut=0x7F46
[FUZZ FIND] kind=1 op=4 item=1 site=30 seed=1697655404 orig=0x7F4A mut=0x7F49
[FUZZ FIND] kind=1 op=4 item=2 site=30 seed=1776508812 orig=0x7F4A mut=0x7F49
[FUZZ FIND] kind=1 op=4 item=2 site=30 seed=1464408700 orig=0x7F4A mut=0x7F49
```

## 4. Check 55 Status

Reduced `./build/pure-bin/fuzz_gates.elf` continues to report Check 55 unchanged:
`[FUZZ FIND] kind=3 op=1 item=1 site=17 seed=977037651` (`soundness=0 roundtrip=1 judge=0`).

## 5. Corpus Completeness & Step 8 Check 69

- **Corpus completeness (0..7):** All 8 corpus items verified with `status=0, all_pass=True`:
  - `spec/acceptance/goldens/seed0_risk.omni` (`nfns=1`)
  - `spec/acceptance/goldens/seed4_consensus.omni` (`nfns=1`)
  - `spec/acceptance/goldens/seed6_clamp.omni` (`nfns=1`)
  - `pure/programs/kv_ledger_service.omni` (`nfns=11`)
  - `pure/programs/pack5_decision_controller.omni` (`nfns=13`)
  - `pure/programs/compiler_self_audit.omni` (`nfns=34`)
  - `pure/m5/zero_rust_driver.omni` (`nfns=6`)
  - `pure/verifier_standalone.omni_bc` (`nfns=69`)
- **Step 8 Check 69 (`F2`):**
  - `pure/programs/kv_ledger_service.omni` word 79 (`byte 158`, `0x7F45 -> 0x7F19`) rejected with `3001` at offset `78`; original accepted, lowered by `m2.elf`, and native ELF exits cleanly (`rc=0`).
  - `spec/acceptance/goldens/seed0_risk.omni` word 49 (`byte 98`, `0x7F4A -> 0x7F46`) rejected with `2012` at offset `49`; original accepted & lowered cleanly.

## 6. Piped `--campaign-all` Inventory Diff vs `main @ 07c4e5e`

- **Before (`main @ 07c4e5e`):** `2800 mutants | 1246 accepted | 1554 rejected | soundness=27 roundtrip=118 judge=12` (`157` total `[FUZZ FIND]` lines)
- **After (`feat/step16-f2-unlowered-ops-fix`):** `2800 mutants | 1224 accepted | 1576 rejected | soundness=0 roundtrip=118 judge=12` (`130` total `[FUZZ FIND]` lines)
- **Line-by-line delta:** **27 closed, 0 added** (`soundness` reduced from `27` to `0`).

## 7. Close-Out Gates Verified

- `PATH=/usr/bin:/bin bash scripts/zero_rust_rebuild.sh` — **PASS**
- `PATH=/usr/bin:/bin bash scripts/verify_step7_driver.sh` — **PASS=38 FAIL=0**
- `PATH=/usr/bin:/bin bash scripts/verify_step8_harness.sh` — **PASS=69 FAIL=0**
- `./build/pure-bin/omnic_pure.elf selftest` — **PASS**
- `python3 scripts/external_verifier_check.py` — **PASS (8/8)**
