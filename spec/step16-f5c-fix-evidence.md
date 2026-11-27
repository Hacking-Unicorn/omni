# Step 16 Part B7 Evidence — Option 2: Reserve Evaporating Ops `0x7F51` (`scat`) & `0x7F55` (`s2i`) + Decompiler Honesty for `0x7F58` (`san_dom`)

## 1. Part A — Acceptance-Site Map, Corpus Safety, and Option 2 Disposition

### Acceptance-Site Map
Every repo site referencing `0x7F51` (`32593`, `scat`/`sconc`), `0x7F55` (`32597`, `s2i`), or `0x7F58` (`32600`, `san_dom`):
- `pure/m2/verify/mod_check.omni::m2v_ins_min_ops`: `32593` (`is_2`, L216), `32597..=32601` (`is_1`, L221).
- `pure/verifier_engine.omni`: `op_is_known` (L176–184), `op_min_ops` (`32593` at L196, `32597..=32601` at L201), `op_is_sanitizer` (`32600` at L341).
- `pure/isa/table.omni`: mnemonic rows `81` (`SCAT`/`SCONC`, L16–17), `85` (`S2I`, L24), `88` (`SAN_DOM`, L95) and op rows `81`, `85`, `88` (`isa_op_rows_5`, L247, L251, L254).
- `pure/m2/emit_multi.omni` (L440, L1278), `pure/m2/emit_x86.omni` (L182), `pure/m2/driver.omni` (L42): **zero** emitter cases for `32593` (`0x7F51`) and `32597` (`0x7F55`); live lowering `m2u_in(op, 32599, 32601)` (`is_mov`) for `32600` (`0x7F58`).
- `pure/audit/decompile_rhs.omni`: `dc_rhs_g4b` (`32600`, L363), `dc_rhs_g4` (`32593`, `32597`, L372–373), `dc_can_rhs` (`32597..=32601`, L435; `32593`, L439).
- `pure/programs/arena_audit_pipeline.omni`: calls `san_dom(...)` (`0x7F58`, L38) and `s2i(...)` (`0x7F55`, L40). *(Excluded from pure-path fuzz corpus per `spec/step14-fuzz-evidence.md:34`.)*

### Option 2 Decision (Confirmed with User)
- **`0x7F51` (`scat`) and `0x7F55` (`s2i`)** have zero M2 emitter cases (they evaporate in `m2.elf`) and zero active pure-path callers: both become **RESERVED** across `mod_check.omni`, `verifier_engine.omni`, and `isa/table.omni`.
- **`0x7F58` (`san_dom`)** is lowered by M2 (`m2u_in(op, 32599, 32601)`) and used in `arena_audit_pipeline.omni`, failing round-trip only because `bootstrap/m3.elf` does not parse `san_dom(...)`: kept live in M2/verifier and fixed via decompiler honesty (`APPROXIMATE`) in `pure/audit/decompile_rhs.omni`.

## 2. Predicted vs Actual Pin Footprint (`bootstrap/MANIFEST.md`)

Rebuilt `pure/m2/m2.omni_bc` from source in 3 chunked `bootstrap/m3.elf` passes (verified bit-exact against `338d3d6` at `87,836 B`, `0x74FB1652CC0E974B` before edits), established 3-generation `bootstrap/m2.elf` fixed point (`S1 == S2 == S3`), and rebuilt `pure/verifier_standalone.omni_bc` (`omnic_pure.elf compile`) and `verifier.elf` (`bootstrap/m2.elf`).

| Artifact | Predicted? | Before (`main @ 338d3d6`) | After (`feat/step16-f5c-reserve-evap-ops`) | Fixed Point / Status |
|---|---|---|---|---|
| `pure/m2/m2.omni_bc` | **Yes (Moved)** | `87,836 B` (`43,918w`), `0x74FB1652CC0E974B` | `87,802 B` (`43,901w`), `0xFD2F8B389C2AAEA1` | Rebuilt from M2 source |
| `bootstrap/m2.elf` | **Yes (Moved)** | `126,152 B`, `0xAA5F84B6AA45BB51` | `126,032 B`, `0x1F040D79AE834D14` | **`S1 == S2 == S3 == V`** |
| `pure/verifier_standalone.omni_bc` | **Yes (Moved)** | `32,760 B` (`16,380w`), `0x676886F908AD2887` | `32,760 B` (`16,380w`), `0xD3482994AFC37D25` | `status=0, nfns=69, all_pass=True` |
| `verifier.elf` | **Yes (Moved)** | `40,562 B`, `0xB3A9FFC5328CAFE4` | `40,562 B`, `0x965E0FA56E7B6046` | Rebuilt via new `m2.elf` |
| `bootstrap/m3.elf` | Unchanged | `207,015 B`, `0x7397A5020895AEBF` | `207,015 B`, `0x7397A5020895AEBF` | **Unchanged** (bit-exact `link-elf`) |
| `m4.elf` (`pure/m4/m4_standalone.omni_bc`) | Unchanged | `91,576 B`, `0xFBA8272337B1BAC4` | `91,576 B`, `0xFBA8272337B1BAC4` | **Unchanged** (rebuilt via new `m2.elf`) |
| `omnic_pure.elf` fixed point | Unchanged | `59,653 B` (0-byte diff) | `59,653 B` (0-byte diff) | **Unchanged** |
| All 5 frozen hashes (`core_gates`) | Unchanged | Pass | Pass | **Unchanged** |

**Actual vs predicted:** **4/4 exact match** (zero unpredicted moves).

## 3. Per-Op Before / After Verdicts & Step 8 Controls (Checks 74–76)

| Mutant | Before (`338d3d6`) | After (`feat/step16-f5c-reserve-evap-ops`) | Step 8 Control |
|---|---|---|---|
| `0x7F51` (`scat`, `seed0_risk` w22 `0x7F10->0x7F51`) | `verifier`: PASS, `m2`: `rc=0`, `audit_decompile`: `RECONSTRUCTED` | `verifier`: `[3001 @ 21, 3003 @ 28, 1005 @ 28]` (`passed=0`), `m2`: `rc=1` | **Check 74** |
| `0x7F55` (`s2i`, `seed0_risk` w37 `0x7F09->0x7F55`) | `verifier`: PASS, `m2`: `rc=0`, `audit_decompile`: `RECONSTRUCTED` | `verifier`: `[3001 @ 36, 3003 @ 42, 1005 @ 42]` (`passed=0`), `m2`: `rc=1` | **Check 75** |
| `0x7F58` (`san_dom`, `seed4_consensus` w25 `0x7F28->0x7F58`) | `verifier`: PASS, `m2`: `rc=0`, `audit_decompile`: `RECONSTRUCTED` | `verifier`: `[0, 0, 1, 4, 1, 0]`, `m2`: `rc=0`, `audit_decompile`: `Status: APPROXIMATE` | **Check 76** |

All 8 corpus items (`0..7`) pass `verifier.elf` (`status=0`, `all_pass=True`) and `m2.elf` (`rc=0`), and goldens compile/decompile/run cleanly.

## 4. Check 55 Panel Output (Reduced `./build/pure-bin/fuzz_gates.elf`)

```text
[FUZZ FAIL] 300 mutants | 142 accepted | 158 rejected | soundness=0 roundtrip=0 judge=1
op=1 gen=48 acc=19 rej=29
op=2 gen=48 acc=14 rej=34
op=3 gen=48 acc=13 rej=35
op=4 gen=48 acc=32 rej=16
op=5 gen=48 acc=4 rej=44
op=6 gen=30 acc=30 rej=0
op=7 gen=30 acc=30 rej=0
[FUZZ FIND] kind=5 op=7 item=3 site=1091 seed=1910166380
```
The first-find line and summary fail-counts (`soundness=0 roundtrip=0 judge=1`) match Check 55's existing assertion unchanged; total accepted/rejected on the 300-mutant slice shifted by 1 (`143/157 -> 142/158`, `op=1 acc=20/28 -> 19/29`) due to reserving `0x7F51`/`0x7F55`.

## 5. Close-Out Gates & Campaign Round-Trip Zero

- `PATH=/usr/bin:/bin bash scripts/zero_rust_rebuild.sh` — **PASS**
- `PATH=/usr/bin:/bin bash scripts/verify_step7_driver.sh` — **PASS=38 FAIL=0**
- `PATH=/usr/bin:/bin bash scripts/verify_step8_harness.sh` — **PASS=76 FAIL=0**
- `./build/pure-bin/omnic_pure.elf selftest` — **PASS**
- `python3 scripts/external_verifier_check.py` — **PASS (8/8)**

### Piped `--campaign-all` Inventory Diff vs `main @ 338d3d6`
- **Before (`main @ 338d3d6`):** `2800 mutants | 1224 accepted | 1576 rejected | soundness=0 roundtrip=3 judge=12` (`15` finds)
- **After (`feat/step16-f5c-reserve-evap-ops`):** `2800 mutants | 1217 accepted | 1583 rejected | soundness=0 roundtrip=0 judge=12` (`12` finds)
- **Closed (`3`, `0` added):**
  ```text
  [FUZZ FIND] kind=3 op=1 item=0 site=37 seed=563097424 orig=0x7F09 mut=0x7F55
  [FUZZ FIND] kind=3 op=1 item=0 site=22 seed=1254987038 orig=0x7F10 mut=0x7F51
  [FUZZ FIND] kind=3 op=1 item=1 site=25 seed=797234575 orig=0x7F28 mut=0x7F58
  ```

**Campaign soundness & round-trip zero reached:** With this PR, the full 2,800-mutant campaign reaches **`soundness=0, roundtrip=0`**, leaving only the **12 Family F7 (judge-integrity)** finds as Step 17's input:
```text
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
