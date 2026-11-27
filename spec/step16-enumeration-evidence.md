# Step 16 Part A — Collect-All Enumeration & Root-Cause Triage Evidence

- **Branch:** `feat/step16-collect-all` (branched off `main @ 4fd9ac5`)
- **PR Type:** Harness PR (**NOT** a Judge PR — zero TCB edits, zero verifier/compiler/decompiler/judge fixes, zero pin moves)
- **Host:** WSL2 Ubuntu Linux x86-64
- **Command:** `./build/pure-bin/fuzz_gates.elf --campaign-all | cat > /tmp/s16_run1.txt` (executed twice back-to-back)

---

## 1. 2× Determinism Verification

Both back-to-back `--campaign-all` runs completed all **2,800 mutants** (`500×5` bytecode Ops 1–5 + `150×2` source Ops 6–7) plus the **12-mutant judge-integrity panel** and produced **0-byte identical output** (`cmp -s /tmp/s16_run1.txt /tmp/s16_run2.txt` = exit `0`, `246` lines each, `RC1 = 1`, `RC2 = 1`).

### Summary Lines (Identical in Run 1 and Run 2)

```text
[FUZZ FAIL] 2800 mutants | 1359 accepted | 1441 rejected | soundness=71 roundtrip=143 judge=12
op=1 gen=500 acc=243 rej=257
op=2 gen=500 acc=217 rej=283
op=3 gen=500 acc=171 rej=329
op=4 gen=500 acc=382 rej=118
op=5 gen=500 acc=49 rej=451
op=6 gen=150 acc=150 rej=0
op=7 gen=150 acc=147 rej=3
```

### Counts by Leg, Kind, and Operato

| Leg | Kind | Op 1 | Op 2 | Op 3 | Op 4 | Op 5 | Op 6 | Op 7 | Total |
| :--- | :---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Soundness (`m2.elf` rejects verifier-accepted `.omni_bc`) | `1` | 67 | 0 | 1 | 3 | 0 | 0 | 0 | **71** |
| Round-trip (`RECONSTRUCTED` fails byte-identical recompile) | `3` | 35 | 21 | 52 | 11 | 24 | 0 | 0 | **143** |
| Decompiler Honesty (`RECONSTRUCTED` with `LITERAL_OP`/`APPROXIMATE`) | `4` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | **0** |
| Judge Integrity (`propose-run --dry-run` reports `gates_ok=1` or `+1t ADOPT`) | `5` | — | — | — | — | — | — | 12 | **12** |
| **Total Finds** | — | **102** | **21** | **53** | **14** | **24** | **0** | **12** | **226** |

> **Default reduced no-arg panel preserved:** `./build/pure-bin/fuzz_gates.elf` (without `--campaign-all`) still runs the bounded fail-closed panel and stops at Find #2 (`seed4_consensus` word `30: 0x7F4A -> 0x7F5B`), keeping the Step 8 harness gate at `56/56 PASS`.

---

## 2. Threshold Question Settled First: `0x7F5A`–`0x7F5C` (`32602`–`32604`)

**Verdict: `0x7F5A`–`0x7F5C` are UNASSIGNED (residual opcode-hole gap left behind by Step 15).**

### Backing Repo-Wide Grep Evidence

```text
$ grep -rn '32602\|32603\|32604\|7f5a\|7f5b\|7f5c\|7F5A\|7F5B\|7F5C' pure/ spec/ scripts/
pure/verifier_engine.omni:337:    let s4: i64 = is_eq(op, 32602);
pure/verifier_engine.omni:338:    let s5: i64 = is_eq(op, 32603);
pure/verifier_engine.omni:339:    let s6: i64 = is_eq(op, 32604);
pure/isa/table.omni:256:    b = iset(b, 90, 5, 1, 0, 2, 0, 0, 0);
pure/isa/table.omni:257:    b = iset(b, 91, 5, 1, 0, 2, 0, 0, 0);
pure/isa/table.omni:258:    b = iset(b, 92, 5, 1, 0, 2, 0, 0, 0);
```

- **Zero** mnemonic rows in `pure/isa/table.omni` (`isa_mn_rows_0..7`).
- **Zero** lowering, codegen, VM dispatch, or decompiler references anywhere in `pure/m2/`, `pure/m3/`, `pure/m4/`, or `pure/audit/decompile.omni`.
- **Implication for Find #2 (`seed4_consensus`, word `30: 0x7F4A -> 0x7F5B`):** Find #2 simultaneously triggers **two** independent bugs:
  1. **Family F1 (Residual Unassigned Opcodes):** `0x7F5B` (`32603`) is unassigned and rejected by `m2.elf`, yet still accepted by `pure/verifier_engine.omni::op_is_known`.
  2. **Family F3 (Dest-Header Shape Mismatch):** Word `30` is a destless instruction (`0x7F4A = RET`, `is_dest == 0`), while `op_has_dest(0x7F5B) == 1`; `verify_ins_body` skips `handle_dest` instead of rejecting `op_has_dest(op) * (1 - is_dest)`.

---

## 3. Root-Cause Family Triage (7 Distinct Families Covering All 226 Finds)

Every one of the **226** enumerated finds maps cleanly to one of **7** root-cause families (`F1`–`F7`):

| Family | Leg | Finds | Shared Code Path | Summary of Root Cause |
| :--- | :--- | ---: | :--- | :--- |
| **F1** | Soundness | **18** | `pure/verifier_engine.omni::op_is_known`, `op_is_sanitizer` | Unassigned sanitizer opcode holes `0x7F5A`–`0x7F5C` (`32602`–`32604`) accepted by verifier, rejected by `m2.elf`. |
| **F2** | Soundness | **40** | `pure/verifier_engine.omni::op_is_known` vs `pure/m2/` (46 lowered ops) | Verifier/M2 lowering contract gap: `14` unlowered float ops (`0x7F19`–`0x7F1D`) + `26` other unlowered ISA ops (`0x7F00`, `0x7F02`, `0x7F03`, `0x7F32`, `0x7F33`, `0x7F36`, `0x7F37`, `0x7F38`, `0x7F46`, `0x7F47`, `0x7F49`, `0x7F4F`, `0x7F55`). |
| **F3** | Soundness | **12** | `pure/verifier_engine.omni::verify_ins_body` (L969) & `scan_labels` | Dest-header / operand-shape mismatch: `11` finds where `op_has_dest(op)==1` with `is_dest==0` (`RET 0x7F4A` mutated to M2-lowered dest ops `0x7F04/06/09/0D/15/21/3F/59`) + `1` find where `RET 0x7F4A -> LABEL 0x7F40` has a register operand instead of a label token. |
| **F4** | Soundness | **1** | `pure/verifier_engine.omni::verify_module` / `verify_one_fn` | Function header (`0x7EE3 FN_HDR`) `fn_id` field accepts a `VAL_IMM` token (`0x0007 -> 0x7EFB`), rejected by `m2.elf`. |
| **F5** | Round-trip | **33** | `pure/verifier_engine.omni::verify_ins_body` / `handle_dest` + `pure/audit/decompile.omni` | Verifier accepts undefined register operands (`21` Op-2 finds, e.g., `return v2`) and `void` (`0x7F80`) / mismatched types (`8` Op-1 + `4` Op-4 finds); decompiler stamps `RECONSTRUCTED (exact)` and `omnic_pure.elf compile` fails (`rc=1`). |
| **F6** | Round-trip | **110** | `pure/audit/decompile.omni` + `pure/verifier_engine.omni::verify_ins_body` (`min_ops` only, no `max_ops`) | Decompiler stamps `Status: RECONSTRUCTED (exact)` on non-canonical mutated bytecode that recompiles to different length/bytes: `52` Op-3 inline-imm (`+2 B`), `24` Op-5 excess-operand duplication (`-2/-4 B`), `27` Op-1 & `7` Op-4 opcode/operand-count drift. (100% on `item=0..2`; `item=3..7` are marked `APPROXIMATE`.) |
| **F7** | Judge | **12** | `pure/harness/proposer_live.omni::plv_run_cycle` (L416-436) & `plv_measure_5` | `propose-run --dry-run` checks only static `p7-canary` + 5× compile/ELF 0-variance + TCB digest; it **never executes the candidate binary** to verify behavioral equivalence, so all `12/12` mutated candidates get `gates_ok=1` (`10` `0t HOLD`, `2` `-1t REJECT`, `0` `+1t ADOPT`). |
| **Total** | All 3 legs | **226** | — | **100% of finds triaged (0 unclassified).** |

### Minimized Reproducers (One Per Family)

- **F1:** `seed4_consensus.omni` (`item=1`), word `30`: `0x7F4A -> 0x7F5B` (`seed=1283088049`) — Find #2 (`verifier.elf`: PASS, `m2.elf`: `rc=1`).
- **F2:** `kv_ledger_service.omni` (`item=3`), word `79`: `0x7F45 -> 0x7F19` (`seed=2019098872`, dest-header present `prev=0x7E01`, `verifier.elf`: PASS, `m2.elf`: `rc=1`).
- **F3:** `seed6_clamp.omni` (`item=2`), word `30`: `0x7F4A -> 0x7F3F` (`seed=1456384405`, destless slot `prev=0x7EE4`, `verifier.elf`: PASS, `m2.elf`: `rc=1`).
- **F4:** `kv_ledger_service.omni` (`item=3`), word `249` (immediately after `248: 0x7EE3 FN_HDR`): `0x0007 -> 0x7EFB` (`seed=1853283376`, `verifier.elf`: PASS, `m2.elf`: `rc=1`).
- **F5:** `seed4_consensus.omni` (`item=1`), word `31`: `0x7E01 -> 0x7E02` (`seed=1792116395`, decompiles to `return v2;` with `Status: RECONSTRUCTED (exact)`, `omnic_pure.elf compile`: `rc=1`).
- **F6:** `seed4_consensus.omni` (`item=1`), word `28`: `0x7E00 -> 0x7EFD` (`seed=1428629972`, decompiles to `let v1: trit = !13;` with `Status: RECONSTRUCTED (exact)`, recompiles `68 B -> 70 B`).
- **F7:** `kv_ledger_service.omni` (`item=3`), byte `914`: `std_clamp(clean_id, 1, 10000)` -> `std_clamp(clean_id, 2, 10000)` (`seed=1291295325`, `propose-run --dry-run`: `gates_ok=1`, `verdict=0t HOLD`).

### Recommended Part B Per-Family Fix Order (Smallest Family First)

1. **Part B1 — Family F4 (1 find, Judge PR):** Reject non-numeric / token-range `fn_id` in `pure/verifier_engine.omni` (`0x7EE3 FN_HDR`) + re-pin `verifier_standalone.omni_bc` & `verifier.elf` + permanent negative control.
2. **Part B2 — Family F3 (12 finds, Judge PR):** Reject `op_has_dest(op) * (1 - is_dest)` and non-label operand on `LABEL (0x7F40)` in `pure/verifier_engine.omni::verify_ins_body` + re-pin + permanent negative control.
3. **Part B3 — Family F1 (18 finds, Judge PR):** Exclude unassigned sanitizer opcodes `0x7F5A`–`0x7F5C` from `pure/verifier_engine.omni::op_is_known` + re-pin + permanent negative control.
4. **Part B4 — Family F2 (40 finds, Judge PR):** Align `pure/verifier_engine.omni::op_is_known` with the M2-lowerable ISA subset (reject unlowered float & unlowered opcodes) + re-pin + permanent negative control.
5. **Part B5 — Family F5 (33 finds, Judge PR):** Enforce defined-register checks on operands and reject `void` (`0x7F80`) on value-producing dest ops in `pure/verifier_engine.omni` + re-pin + permanent negative control.
6. **Part B6 — Family F6 (110 finds, Judge/Audit PR):** Enforce exact operand arity (`max_ops`) in `pure/verifier_engine.omni` and downgrade non-canonical / non-round-trippable instructions in `pure/audit/decompile.omni` from `RECONSTRUCTED` to `APPROXIMATE` + permanent negative controls.
7. **Part B7 — Family F7 (12 finds, Harness/Proposer PR):** Add candidate execution / behavioral-equivalence check to `pure/harness/proposer_live.omni` so behavior-diverging candidates fail `gates_ok=0` (`-1t REJECT`) + permanent negative control.

---

## 4. 12-Mutant Judge-Integrity Panel Results

| idx | Corpus Item | Byte Site | Mutation | Seed | `gates_ok` | Verdict | Find? |
| ---: | :--- | ---: | :---: | ---: | :---: | :--- | :---: |
| 0 | `3` (`kv_ledger_service.omni`) | 914 | `1->2` | `1291295325` | `1` | `0t HOLD` | YES (`kind=5`) |
| 1 | `4` (`pack5_decision_controller.omni`) | 1576 | `1->2` | `1930264012` | `1` | `0t HOLD` | YES (`kind=5`) |
| 2 | `5` (`compiler_self_audit.omni`) | 1320 | `2->3` | `1895835354` | `1` | `0t HOLD` | YES (`kind=5`) |
| 3 | `6` (`zero_rust_driver.omni`) | 2661 | `0->1` | `78452350` | `1` | `-1t REJECT` | YES (`kind=5`) |
| 4 | `3` (`kv_ledger_service.omni`) | 108 | `4->5` | `2025541501` | `1` | `0t HOLD` | YES (`kind=5`) |
| 5 | `4` (`pack5_decision_controller.omni`) | 2013 | `4->5` | `1868269530` | `1` | `0t HOLD` | YES (`kind=5`) |
| 6 | `5` (`compiler_self_audit.omni`) | 1650 | `3->4` | `240791725` | `1` | `-1t REJECT` | YES (`kind=5`) |
| 7 | `6` (`zero_rust_driver.omni`) | 2706 | `0->1` | `1805499307` | `1` | `0t HOLD` | YES (`kind=5`) |
| 8 | `3` (`kv_ledger_service.omni`) | 966 | `1->2` | `427595245` | `1` | `0t HOLD` | YES (`kind=5`) |
| 9 | `4` (`pack5_decision_controller.omni`) | 831 | `2->3` | `1142443988` | `1` | `0t HOLD` | YES (`kind=5`) |
| 10 | `5` (`compiler_self_audit.omni`) | 1711 | `7->8` | `321870717` | `1` | `0t HOLD` | YES (`kind=5`) |
| 11 | `6` (`zero_rust_driver.omni`) | 2952 | `1->2` | `802789306` | `1` | `0t HOLD` | YES (`kind=5`) |

---

## 5. Complete Finds Inventory (All 226 Finds, Deterministic Run Order)

```text
[FUZZ FIND] kind=1 op=1 item=1 site=30 seed=1367066037 orig=0x7F4A mut=0x7F5B
[FUZZ FIND] kind=3 op=1 item=1 site=17 seed=977037651 orig=0x7F2C mut=0x7F15
[FUZZ FIND] kind=1 op=1 item=3 site=341 seed=849667523 orig=0x7F2C mut=0x7F5C
[FUZZ FIND] kind=1 op=1 item=2 site=30 seed=1882296823 orig=0x7F4A mut=0x7F3F
[FUZZ FIND] kind=1 op=1 item=3 site=79 seed=2106444702 orig=0x7F45 mut=0x7F19
[FUZZ FIND] kind=3 op=1 item=0 site=22 seed=1395293013 orig=0x7F10 mut=0x7F55
[FUZZ FIND] kind=3 op=1 item=1 site=25 seed=790834394 orig=0x7F28 mut=0x7F0D
[FUZZ FIND] kind=1 op=1 item=0 site=29 seed=660081049 orig=0x7F2D mut=0x7F19
[FUZZ FIND] kind=1 op=1 item=6 site=577 seed=2139679140 orig=0x7F73 mut=0x7F1A
[FUZZ FIND] kind=1 op=1 item=0 site=49 seed=535143849 orig=0x7F4A mut=0x7F5A
[FUZZ FIND] kind=3 op=1 item=1 site=25 seed=336143379 orig=0x7F28 mut=0x7F21
[FUZZ FIND] kind=3 op=1 item=0 site=37 seed=563097424 orig=0x7F09 mut=0x7F55
[FUZZ FIND] kind=1 op=1 item=1 site=25 seed=1148979964 orig=0x7F28 mut=0x7F4F
[FUZZ FIND] kind=1 op=1 item=2 site=30 seed=2129932747 orig=0x7F4A mut=0x7F5C
[FUZZ FIND] kind=1 op=1 item=0 site=49 seed=1909115813 orig=0x7F4A mut=0x7F5A
[FUZZ FIND] kind=1 op=1 item=5 site=58 seed=1532752537 orig=0x7F4A mut=0x7F49
[FUZZ FIND] kind=3 op=1 item=0 site=29 seed=132297604 orig=0x7F2D mut=0x7F18
[FUZZ FIND] kind=1 op=1 item=4 site=891 seed=1634624305 orig=0x7F48 mut=0x7F33
[FUZZ FIND] kind=3 op=1 item=0 site=29 seed=1741592204 orig=0x7F2D mut=0x7F08
[FUZZ FIND] kind=1 op=1 item=1 site=30 seed=1936414711 orig=0x7F4A mut=0x7F3F
[FUZZ FIND] kind=1 op=1 item=5 site=1276 seed=661206981 orig=0x7F10 mut=0x7F1C
[FUZZ FIND] kind=3 op=1 item=0 site=29 seed=5106049 orig=0x7F2D mut=0x7F18
[FUZZ FIND] kind=1 op=1 item=1 site=30 seed=1453507189 orig=0x7F4A mut=0x7F47
[FUZZ FIND] kind=1 op=1 item=4 site=935 seed=1384158088 orig=0x7F2C mut=0x7F38
[FUZZ FIND] kind=3 op=1 item=2 site=24 seed=810020964 orig=0x7F17 mut=0x7F26
[FUZZ FIND] kind=1 op=1 item=2 site=30 seed=138943525 orig=0x7F4A mut=0x7F02
[FUZZ FIND] kind=3 op=1 item=0 site=22 seed=1254987038 orig=0x7F10 mut=0x7F51
[FUZZ FIND] kind=1 op=1 item=7 site=476 seed=701268424 orig=0x7F2D mut=0x7F5C
[FUZZ FIND] kind=3 op=1 item=1 site=17 seed=1001234230 orig=0x7F2C mut=0x7F13
[FUZZ FIND] kind=1 op=1 item=2 site=30 seed=1634217457 orig=0x7F4A mut=0x7F3F
[FUZZ FIND] kind=3 op=1 item=0 site=43 seed=2009856514 orig=0x7F29 mut=0x7F20
[FUZZ FIND] kind=1 op=1 item=1 site=30 seed=1240234520 orig=0x7F4A mut=0x7F21
[FUZZ FIND] kind=1 op=1 item=0 site=49 seed=1728482248 orig=0x7F4A mut=0x7F5C
[FUZZ FIND] kind=1 op=1 item=3 site=122 seed=381000605 orig=0x7F10 mut=0x7F19
[FUZZ FIND] kind=3 op=1 item=2 site=17 seed=1472426637 orig=0x7F18 mut=0x7F08
[FUZZ FIND] kind=1 op=1 item=5 site=859 seed=1115846069 orig=0x7F11 mut=0x7F1B
[FUZZ FIND] kind=1 op=1 item=0 site=37 seed=686289333 orig=0x7F09 mut=0x7F5B
[FUZZ FIND] kind=1 op=1 item=1 site=30 seed=274412641 orig=0x7F4A mut=0x7F06
[FUZZ FIND] kind=3 op=1 item=2 site=24 seed=1943833271 orig=0x7F17 mut=0x7F2B
[FUZZ FIND] kind=1 op=1 item=6 site=448 seed=1175267274 orig=0x7F73 mut=0x7F5C
[FUZZ FIND] kind=3 op=1 item=0 site=29 seed=524629877 orig=0x7F2D mut=0x7F57
[FUZZ FIND] kind=3 op=1 item=1 site=25 seed=818481419 orig=0x7F28 mut=0x7F30
[FUZZ FIND] kind=3 op=1 item=0 site=43 seed=594101790 orig=0x7F29 mut=0x7F01
[FUZZ FIND] kind=1 op=1 item=3 site=142 seed=382178132 orig=0x7F4A mut=0x7F55
[FUZZ FIND] kind=3 op=1 item=0 site=29 seed=953883799 orig=0x7F2D mut=0x7F09
[FUZZ FIND] kind=3 op=1 item=0 site=22 seed=1757202185 orig=0x7F10 mut=0x7F50
[FUZZ FIND] kind=1 op=1 item=3 site=264 seed=1040858532 orig=0x7F2C mut=0x7F1A
[FUZZ FIND] kind=1 op=1 item=4 site=742 seed=872744656 orig=0x7F4A mut=0x7F0D
[FUZZ FIND] kind=1 op=1 item=1 site=30 seed=888936234 orig=0x7F4A mut=0x7F32
[FUZZ FIND] kind=1 op=1 item=2 site=30 seed=519196673 orig=0x7F4A mut=0x7F40
[FUZZ FIND] kind=1 op=1 item=3 site=417 seed=1524697046 orig=0x7F12 mut=0x7F1D
[FUZZ FIND] kind=3 op=1 item=0 site=29 seed=1366605615 orig=0x7F2D mut=0x7F12
[FUZZ FIND] kind=1 op=1 item=2 site=30 seed=37879232 orig=0x7F4A mut=0x7F5C
[FUZZ FIND] kind=1 op=1 item=6 site=224 seed=77687538 orig=0x7F4A mut=0x7F4F
[FUZZ FIND] kind=1 op=1 item=2 site=30 seed=121749842 orig=0x7F4A mut=0x7F15
[FUZZ FIND] kind=1 op=1 item=0 site=22 seed=1662285741 orig=0x7F10 mut=0x7F1A
[FUZZ FIND] kind=3 op=1 item=2 site=24 seed=210399787 orig=0x7F17 mut=0x7F02
[FUZZ FIND] kind=1 op=1 item=3 site=245 seed=371058551 orig=0x7F4A mut=0x7F37
[FUZZ FIND] kind=3 op=1 item=0 site=43 seed=1184440540 orig=0x7F29 mut=0x7F0D
[FUZZ FIND] kind=1 op=1 item=1 site=30 seed=308041328 orig=0x7F4A mut=0x7F47
[FUZZ FIND] kind=3 op=1 item=1 site=25 seed=706166811 orig=0x7F28 mut=0x7F01
[FUZZ FIND] kind=1 op=1 item=7 site=14876 seed=2037675677 orig=0x7F43 mut=0x7F49
[FUZZ FIND] kind=3 op=1 item=0 site=22 seed=402146181 orig=0x7F10 mut=0x7F58
[FUZZ FIND] kind=1 op=1 item=4 site=467 seed=48622983 orig=0x7F10 mut=0x7F38
[FUZZ FIND] kind=1 op=1 item=6 site=411 seed=2036886458 orig=0x7F73 mut=0x7F5B
[FUZZ FIND] kind=3 op=1 item=1 site=25 seed=322073233 orig=0x7F28 mut=0x7F09
[FUZZ FIND] kind=1 op=1 item=2 site=30 seed=1926868601 orig=0x7F4A mut=0x7F47
[FUZZ FIND] kind=1 op=1 item=6 site=703 seed=1886819068 orig=0x7F48 mut=0x7F4F
[FUZZ FIND] kind=3 op=1 item=0 site=43 seed=1407770622 orig=0x7F29 mut=0x7F1F
[FUZZ FIND] kind=1 op=1 item=1 site=30 seed=904018996 orig=0x7F4A mut=0x7F03
[FUZZ FIND] kind=1 op=1 item=0 site=43 seed=2100045222 orig=0x7F29 mut=0x7F5A
[FUZZ FIND] kind=3 op=1 item=2 site=24 seed=1522161410 orig=0x7F17 mut=0x7F30
[FUZZ FIND] kind=3 op=1 item=1 site=25 seed=797234575 orig=0x7F28 mut=0x7F58
[FUZZ FIND] kind=1 op=1 item=2 site=24 seed=2035476212 orig=0x7F17 mut=0x7F4F
[FUZZ FIND] kind=3 op=1 item=2 site=24 seed=806362993 orig=0x7F17 mut=0x7F57
[FUZZ FIND] kind=1 op=1 item=0 site=29 seed=225430957 orig=0x7F2D mut=0x7F5A
[FUZZ FIND] kind=1 op=1 item=1 site=30 seed=841842198 orig=0x7F4A mut=0x7F21
[FUZZ FIND] kind=1 op=1 item=5 site=1493 seed=1787607198 orig=0x7F06 mut=0x7F49
[FUZZ FIND] kind=1 op=1 item=6 site=279 seed=52757434 orig=0x7F43 mut=0x7F1B
[FUZZ FIND] kind=1 op=1 item=6 site=684 seed=300548056 orig=0x7F10 mut=0x7F1D
[FUZZ FIND] kind=1 op=1 item=2 site=30 seed=1487432506 orig=0x7F4A mut=0x7F33
[FUZZ FIND] kind=3 op=1 item=1 site=17 seed=104420728 orig=0x7F2C mut=0x7F17
[FUZZ FIND] kind=3 op=1 item=2 site=24 seed=1088027753 orig=0x7F17 mut=0x7F26
[FUZZ FIND] kind=1 op=1 item=3 site=53 seed=665768397 orig=0x7F16 mut=0x7F5C
[FUZZ FIND] kind=1 op=1 item=1 site=30 seed=1235414888 orig=0x7F4A mut=0x7F36
[FUZZ FIND] kind=1 op=1 item=5 site=959 seed=1650239938 orig=0x7F2D mut=0x7F5C
[FUZZ FIND] kind=1 op=1 item=3 site=312 seed=1215401934 orig=0x7F2C mut=0x7F1C
[FUZZ FIND] kind=1 op=1 item=0 site=29 seed=1993885131 orig=0x7F2D mut=0x7F1C
[FUZZ FIND] kind=1 op=1 item=2 site=30 seed=572384150 orig=0x7F4A mut=0x7F59
[FUZZ FIND] kind=1 op=1 item=6 site=570 seed=594783669 orig=0x7F48 mut=0x7F5B
[FUZZ FIND] kind=1 op=1 item=4 site=27 seed=404347865 orig=0x7F13 mut=0x7F1D
[FUZZ FIND] kind=1 op=1 item=0 site=49 seed=256352086 orig=0x7F4A mut=0x7F55
[FUZZ FIND] kind=1 op=1 item=1 site=25 seed=265803201 orig=0x7F28 mut=0x7F5C
[FUZZ FIND] kind=3 op=1 item=2 site=24 seed=941507282 orig=0x7F17 mut=0x7F2D
[FUZZ FIND] kind=1 op=1 item=0 site=37 seed=1056065988 orig=0x7F09 mut=0x7F5C
[FUZZ FIND] kind=1 op=1 item=6 site=110 seed=801329670 orig=0x7F4A mut=0x7F00
[FUZZ FIND] kind=3 op=1 item=2 site=24 seed=1612684826 orig=0x7F17 mut=0x7F01
[FUZZ FIND] kind=1 op=1 item=5 site=563 seed=1922476101 orig=0x7F4A mut=0x7F04
[FUZZ FIND] kind=1 op=1 item=0 site=49 seed=1173168749 orig=0x7F4A mut=0x7F46
[FUZZ FIND] kind=1 op=1 item=1 site=30 seed=1311126675 orig=0x7F4A mut=0x7F09
[FUZZ FIND] kind=3 op=1 item=2 site=24 seed=1960249497 orig=0x7F17 mut=0x7F09
[FUZZ FIND] kind=3 op=1 item=0 site=29 seed=1699591942 orig=0x7F2D mut=0x7F10
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
[FUZZ FIND] kind=3 op=3 item=1 site=28 seed=1239242472 orig=0x7E00 mut=0x7EFD
[FUZZ FIND] kind=3 op=3 item=1 site=31 seed=595763302 orig=0x7E01 mut=0x7EFC
[FUZZ FIND] kind=3 op=3 item=2 site=21 seed=1852856118 orig=0x7EC1 mut=0x7EF6
[FUZZ FIND] kind=3 op=3 item=1 site=20 seed=1720794362 orig=0x7EC0 mut=0x7EFF
[FUZZ FIND] kind=3 op=3 item=2 site=20 seed=977390129 orig=0x7EC0 mut=0x7EF6
[FUZZ FIND] kind=3 op=3 item=1 site=22 seed=926001186 orig=0x7EC2 mut=0x7EF4
[FUZZ FIND] kind=3 op=3 item=1 site=22 seed=1846744220 orig=0x7EC2 mut=0x7EF3
[FUZZ FIND] kind=3 op=3 item=1 site=31 seed=1069924541 orig=0x7E01 mut=0x7EF7
[FUZZ FIND] kind=3 op=3 item=2 site=31 seed=1070589965 orig=0x7E01 mut=0x7EF1
[FUZZ FIND] kind=3 op=3 item=1 site=20 seed=1367218207 orig=0x7EC0 mut=0x7EF3
[FUZZ FIND] kind=3 op=3 item=0 site=47 seed=723783035 orig=0x7E03 mut=0x7EFF
[FUZZ FIND] kind=3 op=3 item=1 site=21 seed=164855294 orig=0x7EC1 mut=0x7EFF
[FUZZ FIND] kind=3 op=3 item=1 site=20 seed=97892062 orig=0x7EC0 mut=0x7EFB
[FUZZ FIND] kind=3 op=3 item=1 site=22 seed=466099263 orig=0x7EC2 mut=0x7EF7
[FUZZ FIND] kind=3 op=3 item=0 site=32 seed=124908060 orig=0x7E01 mut=0x7EF3
[FUZZ FIND] kind=3 op=3 item=2 site=20 seed=55026894 orig=0x7EC0 mut=0x7EF9
[FUZZ FIND] kind=3 op=3 item=2 site=20 seed=1225521444 orig=0x7EC0 mut=0x7EF4
[FUZZ FIND] kind=3 op=3 item=0 site=40 seed=251487972 orig=0x7EE9 mut=0x7EFC
[FUZZ FIND] kind=3 op=3 item=2 site=20 seed=633443499 orig=0x7EC0 mut=0x7EF5
[FUZZ FIND] kind=3 op=3 item=2 site=20 seed=1204560949 orig=0x7EC0 mut=0x7EF6
[FUZZ FIND] kind=3 op=3 item=2 site=31 seed=1849984470 orig=0x7E01 mut=0x7EFA
[FUZZ FIND] kind=3 op=3 item=1 site=28 seed=915188143 orig=0x7E00 mut=0x7EF5
[FUZZ FIND] kind=3 op=3 item=1 site=20 seed=776154480 orig=0x7EC0 mut=0x7EFE
[FUZZ FIND] kind=3 op=3 item=0 site=50 seed=1278891858 orig=0x7E04 mut=0x7EFA
[FUZZ FIND] kind=3 op=3 item=1 site=22 seed=1400944607 orig=0x7EC2 mut=0x7EFB
[FUZZ FIND] kind=3 op=3 item=1 site=28 seed=909203448 orig=0x7E00 mut=0x7EFF
[FUZZ FIND] kind=3 op=3 item=0 site=46 seed=2031037438 orig=0x7E02 mut=0x7EFF
[FUZZ FIND] kind=3 op=3 item=2 site=27 seed=1961595759 orig=0x7E00 mut=0x7EFD
[FUZZ FIND] kind=3 op=3 item=0 site=46 seed=1296356514 orig=0x7E02 mut=0x7EF4
[FUZZ FIND] kind=3 op=3 item=0 site=17 seed=1311714561 orig=0x7EC0 mut=0x7EF0
[FUZZ FIND] kind=3 op=3 item=1 site=31 seed=1330410871 orig=0x7E01 mut=0x7EFE
[FUZZ FIND] kind=3 op=3 item=0 site=40 seed=2004886732 orig=0x7EE9 mut=0x7EF9
[FUZZ FIND] kind=3 op=3 item=2 site=21 seed=723832910 orig=0x7EC1 mut=0x7EF9
[FUZZ FIND] kind=3 op=3 item=2 site=21 seed=369338129 orig=0x7EC1 mut=0x7EF2
[FUZZ FIND] kind=3 op=3 item=0 site=46 seed=563032946 orig=0x7E02 mut=0x7EFE
[FUZZ FIND] kind=3 op=3 item=2 site=21 seed=842187679 orig=0x7EC1 mut=0x7EF3
[FUZZ FIND] kind=3 op=3 item=1 site=31 seed=29873559 orig=0x7E01 mut=0x7EF2
[FUZZ FIND] kind=3 op=3 item=2 site=31 seed=359028340 orig=0x7E01 mut=0x7EFE
[FUZZ FIND] kind=1 op=3 item=3 site=249 seed=1232722137 orig=0x0007 mut=0x7EFB
[FUZZ FIND] kind=3 op=3 item=0 site=26 seed=331679470 orig=0x7EC1 mut=0x7EFD
[FUZZ FIND] kind=3 op=3 item=1 site=31 seed=1672255853 orig=0x7E01 mut=0x7EFD
[FUZZ FIND] kind=3 op=3 item=0 site=32 seed=111611352 orig=0x7E01 mut=0x7EFB
[FUZZ FIND] kind=3 op=3 item=1 site=22 seed=1174511692 orig=0x7EC2 mut=0x7EF9
[FUZZ FIND] kind=3 op=3 item=2 site=28 seed=1282419884 orig=0x7EC2 mut=0x7EF5
[FUZZ FIND] kind=3 op=3 item=2 site=31 seed=90595383 orig=0x7E01 mut=0x7EF6
[FUZZ FIND] kind=3 op=3 item=1 site=21 seed=789680491 orig=0x7EC1 mut=0x7EFD
[FUZZ FIND] kind=3 op=3 item=0 site=40 seed=619700044 orig=0x7EE9 mut=0x7EF9
[FUZZ FIND] kind=3 op=3 item=1 site=31 seed=2019423709 orig=0x7E01 mut=0x7EFB
[FUZZ FIND] kind=3 op=3 item=2 site=21 seed=1192733929 orig=0x7EC1 mut=0x7EFD
[FUZZ FIND] kind=3 op=3 item=1 site=31 seed=356383754 orig=0x7E01 mut=0x7EF1
[FUZZ FIND] kind=3 op=3 item=1 site=28 seed=1303540173 orig=0x7E00 mut=0x7EF9
[FUZZ FIND] kind=3 op=3 item=0 site=50 seed=163511806 orig=0x7E04 mut=0x7EFF
[FUZZ FIND] kind=3 op=3 item=2 site=28 seed=1250815866 orig=0x7EC2 mut=0x7EFF
[FUZZ FIND] kind=3 op=4 item=2 site=25 seed=877594268 orig=0x7F81 mut=0x7F80
[FUZZ FIND] kind=3 op=4 item=2 site=24 seed=1642623076 orig=0x7F17 mut=0x7F16
[FUZZ FIND] kind=3 op=4 item=2 site=25 seed=1982411308 orig=0x7F81 mut=0x7F80
[FUZZ FIND] kind=1 op=4 item=1 site=30 seed=1697655404 orig=0x7F4A mut=0x7F49
[FUZZ FIND] kind=3 op=4 item=1 site=17 seed=875675517 orig=0x7F2C mut=0x7F2D
[FUZZ FIND] kind=3 op=4 item=2 site=24 seed=1360931668 orig=0x7F17 mut=0x7F16
[FUZZ FIND] kind=1 op=4 item=2 site=30 seed=1776508812 orig=0x7F4A mut=0x7F49
[FUZZ FIND] kind=3 op=4 item=1 site=17 seed=42661701 orig=0x7F2C mut=0x7F2D
[FUZZ FIND] kind=3 op=4 item=2 site=24 seed=101941868 orig=0x7F17 mut=0x7F16
[FUZZ FIND] kind=3 op=4 item=2 site=24 seed=1913260916 orig=0x7F17 mut=0x7F16
[FUZZ FIND] kind=3 op=4 item=2 site=25 seed=2134189284 orig=0x7F81 mut=0x7F80
[FUZZ FIND] kind=3 op=4 item=1 site=18 seed=1490600061 orig=0x7F83 mut=0x7F84
[FUZZ FIND] kind=3 op=4 item=2 site=23 seed=1805374132 orig=0x7E01 mut=0x7E00
[FUZZ FIND] kind=1 op=4 item=2 site=30 seed=1464408700 orig=0x7F4A mut=0x7F49
[FUZZ FIND] kind=3 op=5 item=2 site=21 seed=1054517774 orig=0x7EC1 mut=0x7EC1
[FUZZ FIND] kind=3 op=5 item=0 site=46 seed=1532507606 orig=0x7E02 mut=0x7E02
[FUZZ FIND] kind=3 op=5 item=2 site=27 seed=1177757786 orig=0x7E00 mut=0x7E00
[FUZZ FIND] kind=3 op=5 item=0 site=50 seed=550584890 orig=0x7E04 mut=0x7E04
[FUZZ FIND] kind=3 op=5 item=1 site=22 seed=503410140 orig=0x7EC2 mut=0x7EC2
[FUZZ FIND] kind=3 op=5 item=2 site=21 seed=1365680458 orig=0x7EC1 mut=0x7EC1
[FUZZ FIND] kind=3 op=5 item=1 site=21 seed=1305427416 orig=0x7EC1 mut=0x7EC1
[FUZZ FIND] kind=3 op=5 item=0 site=46 seed=725563918 orig=0x7E02 mut=0x7E02
[FUZZ FIND] kind=3 op=5 item=1 site=22 seed=227373290 orig=0x7EC2 mut=0x7EC2
[FUZZ FIND] kind=3 op=5 item=2 site=21 seed=1854786540 orig=0x7EC1 mut=0x7EC1
[FUZZ FIND] kind=3 op=5 item=0 site=40 seed=198364204 orig=0x7EE9 mut=0x7EE9
[FUZZ FIND] kind=3 op=5 item=0 site=46 seed=1043091094 orig=0x7E02 mut=0x7E02
[FUZZ FIND] kind=3 op=5 item=0 site=40 seed=1178488104 orig=0x7EE9 mut=0x7EE9
[FUZZ FIND] kind=3 op=5 item=2 site=28 seed=896468962 orig=0x7EC2 mut=0x7EC2
[FUZZ FIND] kind=3 op=5 item=2 site=20 seed=1525552452 orig=0x7EC0 mut=0x7EC0
[FUZZ FIND] kind=3 op=5 item=1 site=20 seed=1399664970 orig=0x7EC0 mut=0x7EC0
[FUZZ FIND] kind=3 op=5 item=0 site=46 seed=702626138 orig=0x7E02 mut=0x7E02
[FUZZ FIND] kind=3 op=5 item=1 site=20 seed=1511327638 orig=0x7EC0 mut=0x7EC0
[FUZZ FIND] kind=3 op=5 item=2 site=20 seed=1509300680 orig=0x7EC0 mut=0x7EC0
[FUZZ FIND] kind=3 op=5 item=2 site=31 seed=637870352 orig=0x7E01 mut=0x7E01
[FUZZ FIND] kind=3 op=5 item=0 site=32 seed=1524609784 orig=0x7E01 mut=0x7E01
[FUZZ FIND] kind=3 op=5 item=1 site=28 seed=1409549728 orig=0x7E00 mut=0x7E00
[FUZZ FIND] kind=3 op=5 item=1 site=22 seed=1641234510 orig=0x7EC2 mut=0x7EC2
[FUZZ FIND] kind=3 op=5 item=0 site=50 seed=1351237626 orig=0x7E04 mut=0x7E04
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
