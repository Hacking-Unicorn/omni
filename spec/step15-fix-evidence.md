# Step 15 — Fuzz Find #1 Fix (Judge PR) & Campaign Re-Run Evidence

**Branch:** `feat/step15-verifier-opcode-fix`  
**Host / Target:** WSL2 Ubuntu Linux x86-64 (native Pure-Omni stack, zero Rust)  
**Outcome:**
1. **Part A (Minimal Judge Fix):** Tightened `pure/verifier_engine.omni::op_is_known` (6 lines) to reject all 18 unassigned opcode holes in `0x7F00..=0x7F7F` (including Find #1's `0x7F5F`) with existing error code `3001`.
2. **Part B (MANIFEST Pin Moves):** Rebuilt `pure/verifier_standalone.omni_bc` and `verifier.elf` cleanly from `pure/verifier_engine.omni`; updated `bootstrap/MANIFEST.md` and pin checks. Zero other pins or frozen-5 hashes moved.
3. **Part C (Regression + Full Campaign Re-Run):** Permanent Find #1 negative control PASS (fixed `verifier.elf` rejects `kv_ledger_service` word `179: 0x7F28 -> 0x7F5F` with `err=3001` while accepting original). Re-running the full campaign (`fuzz_gates.elf --campaign`) stopped fail-closed on a **new** soundness bug — **Find #2** (`seed4_consensus.omni` word `30: 0x7F4A -> 0x7F5B`, dest-requiring opcode emitted destless). Per Step 15 rules, Find #2 is minimized and filed below without fixing it in this PR. The 12-mutant judge-integrity panel was also run and is reported below.

---

## 1. Root-Cause Confirmation & Minimal Fix (`pure/verifier_engine.omni`)

- **Hypothesis check:** `pure/verifier_engine.omni` did **not** consult `pure/isa/table.omni` at all during opcode validation. Instead, `op_is_known(op)` (lines 171–175) solely checked `in_range(op, 32512, 32639)` (`0x7F00..=0x7F7F`), accepting all 18 unassigned opcode holes in that 128-slot block:
  - `0x7F07` (`32519`), `0x7F0A..0x7F0C` (`32522..32524`), `0x7F3A..0x7F3E` (`32570..32574`), `0x7F4B..0x7F4E` (`32587..32590`), `0x7F52..0x7F54` (`32594..32596`), `0x7F5E..0x7F5F` (`32606..32607`, including Find #1's `0x7F5F`).
- **Minimal diff (`pure/verifier_engine.omni:171-178`):**
  ```diff
   #[strict]
   fn op_is_known(op: i64) -> i64 {
  -    let r: i64 = in_range(op, 32512, 32639);
  +    let in_blk: i64 = in_range(op, 32512, 32639);
  +    let hole: i64 = is_eq(op, 32519) + in_range(op, 32522, 32524)
  +        + in_range(op, 32570, 32574) + in_range(op, 32587, 32590)
  +        + in_range(op, 32594, 32596) + in_range(op, 32606, 32607);
  +    let r: i64 = in_blk * is_eq(hole, 0);
       return r;
   }
  ```
- **Error code:** Uses existing `3001` at `verify_ins_body` line 944 (`append_err_if(w, 1 - known, 3001, ins_off)`). Zero new error codes.
- **Completeness:** All 68 `isa_mn_rows` opcodes plus all capability/syscall/float/sanitizer/sink opcodes (e.g. `0x7F73` `SYS_CALL`, `0x7F64` `DB_GET`) remain accepted; every corpus program, golden, and suite gate verifies with 0 regressions.

---

## 2. Moved Pins (`bootstrap/MANIFEST.md` Discipline)

| Artifact | Old Size / FNV-1a64 | New Size / FNV-1a64 | Reason |
| :--- | :--- | :--- | :--- |
| `pure/verifier_standalone.omni_bc` | `30,804 B` (`15,402 w`) / `0xFF327F8976741D20` | **`31,048 B` (`15,524 w`) / `0x430CC3C933918F64`** | Step 15 Judge Fix (Fuzz Find #1) |
| `verifier.elf` (rebuilt in `build/pure-bin/`) | `37,907 B` / `0x2ECD2CE3DCB69346` | **`38,193 B` / `0x23E3D55383B3333B`** | Rebuilt from new `verifier_standalone.omni_bc` via `m2.elf` |

- **Unmoved pins:** `bootstrap/m2.elf` (`126,152 B`, `0xAA5F84B6AA45BB51`, `S1==S2==S3`), `bootstrap/m3.elf` (`207,015 B`, `0x7397A5020895AEBF`), `m4.elf` (`91,576 B`, `0xFBA8272337B1BAC4`), all frozen-5 hashes, `m2.omni_bc`, `m3.omni_bc`, `m3_standalone.omni_bc`, `mod_parse.omni_bc`, `elf_link.omni_bc`, `m4.omni_bc`, `m4_standalone.omni_bc`, and `omnic_pure.elf` self-compile fixed point (`59,653 B`, 0-byte diff).

---

## 3. Permanent Find #1 Negative Control & Regression Wiring

- **Find #1 check (`scripts/verify_step8_harness.sh` Check 56, suite `55 -> 56`):**
  - Unmutated `pure/programs/kv_ledger_service.omni`: all 11 functions `passed=1, err_count=0` (**ACCEPTED**).
  - Mutated `kv_ledger_service.omni` (word `179: 0x7F28 -> 0x7F5F`, byte offset `358`): `fn_5: passed=0, err_count=1, err_code=3001, err_off=178` (**REJECTED**).
- **Harness check-bug fix (named per Standing Rule 2):** Updated `pure/harness/fuzz_gates.omni::fg_verify_bc` to walk all `nfns` function records in `verifier.elf`'s report rather than only `fn[0]`, and reconciled the `fuzz_gates.omni` header comment to match the true per-class campaign counts:
  - **Full campaign (`--campaign`):** `500/class` for Ops 1–5 (`2,500`) + `150/class` for Ops 6–7 (`300`) = **`2,800` total**.
  - **Reduced panel (default):** `48/class` for Ops 1–5 (`240`) + `30/class` for Ops 6–7 (`60`) = **`300` total**.

---

## 4. Full Campaign Re-Run — Stopped Fail-Closed on Find #2

Command: `./build/pure-bin/fuzz_gates.elf --campaign` (`SEED_INIT = 31337421`, `LCG_MUL = 1664525`, `LCG_INC = 1013904223`):

```text
[FUZZ FAIL] 2 mutants | 1 accepted | 1 rejected | soundness=1 roundtrip=0 judge=0
op=1 gen=2 acc=1 rej=1
op=2 gen=0 acc=0 rej=0
op=3 gen=0 acc=0 rej=0
op=4 gen=0 acc=0 rej=0
op=5 gen=0 acc=0 rej=0
op=6 gen=0 acc=0 rej=0
op=7 gen=0 acc=0 rej=0
[FUZZ FIND] kind=1 op=1 item=1 site=30 seed=1367066037
```

### Filed Find #2 (Stop Find — Verifier Accepts Dest-Requiring Opcode Emitted Without Dest Register)

- **Property violated:** Property 1 — Verifier soundness (`soundness=1`).
- **Corpus item:** `spec/acceptance/goldens/seed4_consensus.omni` (`item=1`, 33 words total — already minimal!).
- **Operator / Site / Seed:** Op 1 (`OP_SUB`), word `30`: `0x7F4A` (`RET`, destless) → `0x7F5B` (`32603`, `op_has_dest(32603) == 1`, dest-requiring), `seed=1367066037`.
- **Verdicts:**
  - **Pure-Omni `verifier.elf`:** **ACCEPTED** (`passed=1`, `err_count=0`, `fn_count=1`, `fn_idx=4`). `verify_ins_body` (line 965) skips `handle_dest` when `is_dest == 0` instead of rejecting `op_has_dest(op) * (1 - is_dest)` (and accepts a function with no `RET`).
  - **Pure-Omni `m2.elf`:** **REJECTED** (`rc=1`, `0 B` ELF).
- **Minimized Reproducer (33-word module, 1-word mutation):**
  - Context words `[26..33]` of `spec/acceptance/goldens/seed4_consensus.omni`:
    - Original: `0x7f83 0x7fc0 0x7e00 0x7ee4 [0x7f4a] 0x7e01 0x7ee4 0x7eef`
    - Mutant:   `0x7f83 0x7fc0 0x7e00 0x7ee4 [0x7f5b] 0x7e01 0x7ee4 0x7eef`
  - **Not fixed in this PR** (filed for its own dedicated judge PR per Step 15 rules).

---

## 5. Judge-Integrity Panel (12 Behavior-Diverging Dry-Run Candidates)

Executed on the first 12 Op 7 (integer literal `±1`) mutants across flywheel-eligible corpus modules (`kv_ledger_service`, `pack5_decision_controller`, `compiler_self_audit`, `zero_rust_driver`) via `omnic_pure.elf propose-run --dry-run`:

| # | Target | Byte Site | Seed | `gates_ok` | `rc` | Judge Verdict |
| ---: | :--- | ---: | ---: | ---: | ---: | :--- |
| J01 | `pure/programs/kv_ledger_service.omni` | 914 | 2053685099 | `1` | 0 | `0t HOLD` (`learn_delta=+0`) |
| J02 | `pure/programs/pack5_decision_controller.omni` | 831 | 1367066037 | `1` | 0 | `0t HOLD` (`learn_delta=+0`) |
| J03 | `pure/programs/compiler_self_audit.omni` | -1 | 1565102286 | `1` | 0 | `0t HOLD` (`learn_delta=+0`) |
| J04 | `pure/m5/zero_rust_driver.omni` | 2403 | 46528505 | `1` | 1 | `-1t REJECT` (`learn_delta=-3`) |
| J05 | `pure/programs/kv_ledger_service.omni` | 1228 | 1180479293 | `1` | 0 | `0t HOLD` (`learn_delta=+0`) |
| J06 | `pure/programs/pack5_decision_controller.omni` | 1952 | 1815676759 | `1` | 1 | `-1t REJECT` (`learn_delta=-3`) |
| J07 | `pure/programs/compiler_self_audit.omni` | 1048 | 97551218 | `1` | 0 | `0t HOLD` (`learn_delta=+0`) |
| J08 | `pure/m5/zero_rust_driver.omni` | 847 | 710790365 | `1` | 0 | `0t HOLD` (`learn_delta=+0`) |
| J09 | `pure/programs/kv_ledger_service.omni` | 1538 | 1025920502 | `1` | 0 | `0t HOLD` (`learn_delta=+0`) |
| J10 | `pure/programs/pack5_decision_controller.omni` | 831 | 977037651 | `1` | 0 | `0t HOLD` (`learn_delta=+0`) |
| J11 | `pure/programs/compiler_self_audit.omni` | 1542 | 1381463119 | `1` | 0 | `0t HOLD` (`learn_delta=+0`) |
| J12 | `pure/m5/zero_rust_driver.omni` | 901 | 849667523 | `1` | 0 | `0t HOLD` (`learn_delta=+0`) |

- **Summary:** `0/12` returned `+1t ADOPT` (`10` `0t HOLD`, `2` `-1t REJECT`). However, **all 12 returned `gates_ok=1`** rather than `gates_ok=0` because `proposer_live.omni` Stage 3 currently validates compile + verify + size determinism + hardcoded `p7-canary` without running target-specific golden functional tests. Filed as a judge-pipeline hardening item (out of scope for this PR).

---

## 6. Close-Out Gates (`feat/step15-verifier-opcode-fix`, WSL2 Ubuntu Linux x86-64)

- `bash scripts/zero_rust_rebuild.sh`: **PASS** (all 10 new MANIFEST pins bit-exact, `S1==S2==S3==V`, `omnic_pure.elf` self-compile fixed point `59,653 B` 0-byte diff, negative control caught)
- `bash scripts/verify_step7_driver.sh`: **38/38 PASS** (`FAIL=0`)
- `bash scripts/verify_step8_harness.sh`: **56/56 PASS** (`FAIL=0`, includes Check 56 Find #1 permanent negative control)
- `./build/pure-bin/omnic_pure.elf selftest`: **PASS**
- Frozen-5 verified unmoved via `m2-frozen` / `s5-refreeze`; original `kv_ledger_service` and all corpus programs verify and run their goldens cleanly.
