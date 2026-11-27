# Step 14 — Mutation-Fuzz Campaign Evidence

**Branch:** `feat/step14-mutation-fuzz` (off `main @ 1d6be73`)  
**Host / Target:** WSL2 Ubuntu Linux x86-64 (native Pure-Omni stack, zero Rust)  
**Harness:** `pure/harness/fuzz_gates.omni` → `build/pure-bin/fuzz_gates.elf` (built by `scripts/build_pure_driver.sh`, zero Rust/Python)  
**Outcome:** **Campaign stopped fail-closed at Mutant #4 on a genuine, minimal Verifier Soundness Find (`soundness=1`).** Per Step 14 rules, no verifier, decompiler, or judge fix was made; the find is minimized, reproduced, and filed below.

---

## 1. Deterministic PRNG & Seed Constants

Committed in `pure/harness/fuzz_gates.omni` (no clock, no environment input; 2x determinism verified bit-identical in `scripts/verify_step8_harness.sh`):

- `SEED_INIT = 31337421` (`0x01DE2B4D`)
- `LCG_MUL = 1664525`
- `LCG_INC = 1013904223`
- Step function: `x = s ^ ((s >> 13) & 0x7FFFFFFF); next = (x * 1664525 + 1013904223) & 0x7FFFFFFF`

---

## 2. Corpus List (8 Items)

| Index | Path | Role / Notes |
| ---: | :--- | :--- |
| 0 | `spec/acceptance/goldens/seed0_risk.omni` | Executable golden surface program (100% `RECONSTRUCTED`) |
| 1 | `spec/acceptance/goldens/seed4_consensus.omni` | Executable golden surface program (100% `RECONSTRUCTED`) |
| 2 | `spec/acceptance/goldens/seed6_clamp.omni` | Executable golden surface program (100% `RECONSTRUCTED`) |
| 3 | `pure/programs/kv_ledger_service.omni` | Pure-compilable program (`553 w`) |
| 4 | `pure/programs/pack5_decision_controller.omni` | Pure-compilable program (`1,039 w`) |
| 5 | `pure/programs/compiler_self_audit.omni` | Pure-compilable program (`3,272 w`) |
| 6 | `pure/m5/zero_rust_driver.omni` | Native syscall driver module (`714 w`) |
| 7 | `pure/verifier_standalone.omni_bc` | Committed standalone verifier bytecode (`15,402 w`; note `pure/verifier_standalone.omni` does not exist — source is `pure/verifier_engine.omni`) |

*Corpus note:* The other 3 files in `pure/programs/` (`arena_audit_pipeline.omni`, `policy_consensus_engine.omni`, `ternary_neural_router.omni`) use quarantined `macro ` directives (rejected on the pure default path per `a3-macro-quarantine`) and are therefore excluded from pure-path compilation.

---

## 3. Mutation Operators (7 Classes)

1. **Op 1 (`OP_SUB` — Bytecode):** Opcode-field substitution (`0x7F00..0x7F7F` → PRNG-selected opcode in `0x7F00..0x7F5F`).
2. **Op 2 (`REG_PERT` — Bytecode):** Register/param operand perturbation (`±1` in `0x7E00..0x7EDF`).
3. **Op 3 (`LBL_RETARG` — Bytecode):** Label-operand retarget (`0x7EF0..0x7EFF`).
4. **Op 4 (`IMM_PERT` — Bytecode):** Immediate payload perturbation (`±1` or sign-flip on `TOK_IMM_I16`/`TOK_IMM_I32`).
5. **Op 5 (`WORD_DEL_DUP` — Bytecode):** Single-word deletion or duplication at non-header word index (`>= 16`, never header/trailer).
6. **Op 6 (`SRC_CMP_SWAP` — Source):** Comparison/arithmetic operator swap in `.omni` text, then compile.
7. **Op 7 (`SRC_INT_PERT` — Source):** Integer literal `±1` in `.omni` text, then compile.

---

## 4. Campaign Run Results (Fail-Closed Stop at Mutant #4)

Command: `./build/pure-bin/fuzz_gates.elf --campaign` (configured target: `2,800` mutants = `500/class` for Ops 1–5 + `150/class` for Ops 6–7; stops immediately on first find).

Exact output (`rc=1`):

```text
[FUZZ FAIL] 4 mutants | 1 accepted | 3 rejected | soundness=1 roundtrip=0 judge=0
op=1 gen=4 acc=1 rej=3
op=2 gen=0 acc=0 rej=0
op=3 gen=0 acc=0 rej=0
op=4 gen=0 acc=0 rej=0
op=5 gen=0 acc=0 rej=0
op=6 gen=0 acc=0 rej=0
op=7 gen=0 acc=0 rej=0
[FUZZ FIND] kind=1 op=1 item=3 site=179 seed=46528505
```

Per-class table at stop:

| Operator | Generated | Accepted | Rejected |
| :--- | ---: | ---: | ---: |
| Op 1 (`OP_SUB`) | 4 | 1 | 3 |
| Op 2 (`REG_PERT`) | 0 | 0 | 0 |
| Op 3 (`LBL_RETARG`) | 0 | 0 | 0 |
| Op 4 (`IMM_PERT`) | 0 | 0 | 0 |
| Op 5 (`WORD_DEL_DUP`) | 0 | 0 | 0 |
| Op 6 (`SRC_CMP_SWAP`) | 0 | 0 | 0 |
| Op 7 (`SRC_INT_PERT`) | 0 | 0 | 0 |
| **Total** | **4** | **1** | **3** |

---

## 5. Filed Find #1 (Stop Find) — Verifier Accepts Unassigned Opcode `0x7F5F`

- **Property violated:** Property 1 — Verifier soundness (`soundness=1`).
- **Corpus item:** `pure/programs/kv_ledger_service.omni` (`item=3`, unmutated compiles to `553 w`, lowers cleanly under `m2.elf` to `1,390 B` with `rc=0`).
- **Operator:** Op 1 (`OP_SUB` — opcode-field substitution).
- **Site:** Word index `179` (`0x7F28` → `0x7F5F` = `32607`, an unassigned opcode not in `pure/isa/table.omni`).
- **Seed:** `46528505` (PRNG step 4 from `SEED_INIT = 31337421`).
- **Verdicts:**
  - **Pure-Omni `verifier.elf`:** **ACCEPTED** (`passed=1`, `err_count=0`, `fn_count=11`).
  - **Pure-Omni `m2.elf`:** **REJECTED** (`rc=1`, `0 B` ELF — cannot lower unknown opcode `0x7F5F`).
  - **Independent external verifier (`scripts/external_verifier_check.py`):** `(False, 'DECODE_ERROR')` (disagrees with `verifier.elf`; see §6 below).
  - **`audit_decompile.elf`:** Honestly rendered as `// [LITERAL_OP @0x00B2] v3 = OP_UNKNOWN (0x7F5F) v2` under `APPROXIMATE` status (0 honesty violation).
- **Minimized Reproducer (single-word mutation):**
  - Context words `[174..187]` of `pure/programs/kv_ledger_service.omni` bytecode:
    - Original: `0x7fc0 0x7e01 0x7ec0 0x7ee4 0x7e03 [0x7f28] 0x7f83 0x7fc0 0x7e02 0x7ee4 0x7f43 0x7e03 0x7ef2 0x7ee4`
    - Mutant:   `0x7fc0 0x7e01 0x7ec0 0x7ee4 0x7e03 [0x7f5f] 0x7f83 0x7fc0 0x7e02 0x7ee4 0x7f43 0x7e03 0x7ef2 0x7ee4`
  - Standalone reproduction (3 commands):
    ```bash
    ./build/pure-bin/omnic_pure.elf compile pure/programs/kv_ledger_service.omni /tmp/kv.bc
    python3 -c 'import struct; b=bytearray(open("/tmp/kv.bc","rb").read()); b[179*2:179*2+2]=struct.pack("<H",0x7F5F); open("/tmp/kv_mut.bc","wb").write(b)'
    ./build/pure-bin/verifier.elf /tmp/kv_mut.bc | od -An -tu8 -N 48   # passed=1, err_count=0 (ACCEPTED)
    ./build/pure-bin/m2.elf /tmp/kv_mut.bc > /tmp/kv_mut.elf           # exits rc=1, 0 B (REJECTED)
    ```
- **Remediation (out of scope for Step 14):** Dedicated judge PR under `bootstrap/MANIFEST.md` discipline tightening `pure/verifier_engine.omni` to reject unassigned opcodes within `0x7F00..0x7F7F`.

---

## 6. Additional Diagnostic Observations (Filed, Not Fixed)

1. **External Verifier Layout Divergence (`scripts/external_verifier_check.py`):**
   - `scripts/external_verifier_check.py` (line 117) expects instruction headers in the legacy synthetic trace order `[dest, ty, taint, op]` (`op = words[cur + 3]`), whereas real `m3.elf` + `verifier.elf` bytecode uses `[dest, op, ty, tc]` (`op = words[cur + 1]`). Consequently, `external_verifier_check.py::verify_stream` returns `(False, 'DECODE_ERROR')` on every real compiled `.omni_bc` file (including unmutated `kv_ledger_service.omni`).
2. **Judge Integrity Observation (`proposer_live.omni` Stage 3):**
   - During initial reduced-panel bring-up (before the `m2.elf` lowering check halted at Mutant #4), running Property 3 (`fg_run_judge_panel`) on `pure/programs/kv_ledger_service.omni` (Op 7 integer literal `±1` at byte `914`, `seed=593302591`) yielded `gates_ok=1` (`verdict=0t HOLD`) rather than `gates_ok=0`, because `proposer_live.omni` currently checks compile + verify + size determinism (`plv_measure_5`) and hardcoded `p7-canary` rather than target-specific behavioral/golden assertions. A behavior-diverging mutant that also shrinks bytecode words (e.g., `IMM_I32` → `IMM_I16`) would be ADOPT-eligible unless target-level behavioral tests are wired into `gates_ok`.

---

## 7. Suite Wiring & Close-Out Gates (`feat/step14-mutation-fuzz`)

Extended `scripts/verify_step8_harness.sh` with 3 new checks (**52 → 55** checks):
- Check 53: `fuzz_gates.elf` built & executable (`46,168 B` / Pure-Omni build).
- Check 54: `fuzz_gates.elf` 2x determinism (0-byte diff across repeated runs).
- Check 55: `fuzz_gates.elf` fail-closed soundness find reproduced (`[FUZZ FAIL]` + `[FUZZ FIND] kind=1 op=1 item=3 site=179 seed=46528505`, or `[FUZZ PASS]` once the judge PR lands).

All close-out gates verified on `feat/step14-mutation-fuzz` (WSL2 Ubuntu Linux x86-64):
- `bash scripts/build_pure_driver.sh`: **PASS**
- `bash scripts/zero_rust_rebuild.sh`: **PASS**
- `bash scripts/verify_step7_driver.sh`: **38/38 PASS** (`FAIL=0`)
- `bash scripts/verify_step8_harness.sh`: **55/55 PASS** (`FAIL=0`)
- `./build/pure-bin/omnic_pure.elf selftest`: **PASS**
