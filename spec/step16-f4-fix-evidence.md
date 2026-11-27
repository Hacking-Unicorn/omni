# Step 16 Part B1 — Judge PR Evidence: Family F4 (`FN_HDR fn_id` Validation)

- **Branch:** `feat/step16-f4-fn-id-fix` (branched off `main @ 020c922`)
- **PR Type:** **Judge PR** under `bootstrap/MANIFEST.md` discipline
- **Host:** WSL2 Ubuntu Linux x86-64
- **Canonical §5 Find Record (from `spec/step16-enumeration-evidence.md` line 285):**
  ```text
  [FUZZ FIND] kind=1 op=3 item=3 site=249 seed=1232722137 orig=0x0007 mut=0x7EFB
  ```

---

## 1. Root Cause & Minimal Fix (`pure/verifier_engine.omni`)

- **Root Cause:** In `pure/verifier_engine.omni`, both `FN_HDR` (`0x7EE3` = `32483`) parsing loops — `prescan_module_fns` (line 1191) and `omni_verify_module_with_mode` (line 1267) — read `fn_idx = load_u16(stream, pos)` without validating that `fn_idx` is a plain numeric function ID rather than a token-band word (`>= 0x7E00` = `32256`). Consequently, mutating `kv_ledger_service.omni` word `249` (immediately following `FN_HDR 0x7EE3` at word `248`) from `0x0007` to `0x7EFB` (`32507`, a `VAL_IMM` token) was accepted by `verifier.elf` (PASS) while rejected by `m2.elf` (`rc=1`).
- **Validity Predicate & Codebase Source:**
  - Predicate: `in_range(fn_idx, 0, 1023)` (i.e. `0 <= fn_idx <= 1023`).
  - Source: `pure/m2/verify/mod_check.omni:70` (`m2v_record_fn`) enforces `m2u_lt(fid, 0) + m2u_ge(fid, 1024)` (`DECODE_ERROR = 1`), and all token bands in `pure/verifier_engine.omni` and `pure/m2/decode.omni` begin at `0x7E00` (`32256`). Maximum `#[fn_id(...)]` across the entire repository is `6`.
- **Both Read Sites Enforced:**
  1. `prescan_module_fns` (`pure/verifier_engine.omni:1192`): `w = set_decode_err_if(w, 1 - in_range(fn_idx, 0, 1023), pos);`
  2. `omni_verify_module_with_mode` (`pure/verifier_engine.omni:1269`): `ws = set_decode_err_if(ws, 1 - in_range(fn_idx, 0, 1023), pos);`
- **Error Code:** Existing structural decode error code **`3006`** (`set_decode_err_if`, identical to all other malformed `FN_HDR` checks in both functions). Zero new error codes.

---

## 2. Moved Pins (`bootstrap/MANIFEST.md` Discipline)

From-scratch derivation verified first on unmodified `020c922:pure/verifier_engine.omni` (`31,048 B`, `0x430CC3C933918F64` — **0-byte diff** vs committed seed), then re-run identically on the fixed `pure/verifier_engine.omni`:

| Artifact | Old Size / FNV-1a64 | New Size / FNV-1a64 | Reason |
| :--- | :--- | :--- | :--- |
| `pure/verifier_standalone.omni_bc` | `31,048 B` (`15,524 w`) / `0x430CC3C933918F64` | **`31,168 B` (`15,584 w`) / `0x2AF95DB4A6628007`** | Step 16 Part B1 Judge Fix (Family F4) |
| `verifier.elf` (rebuilt in `build/pure-bin/`) | `38,193 B` / `0x23E3D55383B3333B` | **`38,365 B` / `0x035987D581B3F315`** | Rebuilt from new `verifier_standalone.omni_bc` via `m2.elf` |

- **Unmoved pins:** `bootstrap/m2.elf` (`126,152 B`, `0xAA5F84B6AA45BB51`, `S1==S2==S3==V`), `bootstrap/m3.elf` (`207,015 B`, `0x7397A5020895AEBF`), `m4.elf` (`91,576 B`, `0xFBA8272337B1BAC4`), all frozen-5 hashes, `m2.omni_bc`, `m3.omni_bc`, `m3_standalone.omni_bc`, `mod_parse.omni_bc`, `elf_link.omni_bc`, `m4.omni_bc`, `m4_standalone.omni_bc`, and `omnic_pure.elf` self-compile fixed point (`59,653 B`, 0-byte diff).

---

## 3. Permanent Negative Control & Completeness Check

- **New Step 8 Check 57 (`scripts/verify_step8_harness.sh`, suite `56 -> 57`):**
  - Mutated `kv_ledger_service.omni` (word `249` / byte offset `498`: `0x0007 -> 0x7EFB`) rejected by `verifier.elf` with **`status=3006`, `off=249`, `nfns=0`**.
  - Original `kv_ledger_service.omni` verifies (`status=0`, `nfns=11`) and executes cleanly (`rc=0`).
- **Check 55 unchanged:** Default reduced no-arg `fuzz_gates.elf` still stops fail-closed at Find #2 (`item=1 site=30 0x7F4A->0x7F5B`).
- **Corpus Completeness (0 regressions):**
  - `item=0` `spec/acceptance/goldens/seed0_risk.omni`: verifier=PASS (`nfns=1`), `539 B` ELF
  - `item=1` `spec/acceptance/goldens/seed4_consensus.omni`: verifier=PASS (`nfns=1`), `507 B` ELF
  - `item=2` `spec/acceptance/goldens/seed6_clamp.omni`: verifier=PASS (`nfns=1`), `493 B` ELF
  - `item=3` `pure/programs/kv_ledger_service.omni`: verifier=PASS (`nfns=11`), `1,390 B` ELF, run `rc=0`
  - `item=4` `pure/programs/pack5_decision_controller.omni`: verifier=PASS (`nfns=13`), `2,824 B` ELF, run `rc=0`
  - `item=5` `pure/programs/compiler_self_audit.omni`: verifier=PASS (`nfns=34`), `6,369 B` ELF, run `rc=0`
  - `item=6` `pure/m5/zero_rust_driver.omni`: verifier=PASS (`nfns=6`), `3,574 B` ELF, run `rc=0`
  - `item=7` `pure/verifier_standalone.omni_bc`: self-verifier=PASS (`nfns=68`)

---

## 4. Full `--campaign-all` Piped Re-Run & Inventory Diff

Command: `./build/pure-bin/fuzz_gates.elf --campaign-all | cat > /tmp/s16_f4_after.txt` (`RC = 1`):

```text
[FUZZ FAIL] 2800 mutants | 1355 accepted | 1445 rejected | soundness=73 roundtrip=143 judge=12
op=1 gen=500 acc=239 rej=261
op=2 gen=500 acc=215 rej=285
op=3 gen=500 acc=170 rej=330
op=4 gen=500 acc=382 rej=118
op=5 gen=500 acc=52 rej=448
op=6 gen=150 acc=150 rej=0
op=7 gen=150 acc=147 rej=3
```

### Exact Inventory Diff vs `spec/step16-enumeration-evidence.md` §5

- **Unchanged corpus items (`item=0..6`): `225 -> 224` (exact `-1`, 0 new finds):**
  - **CLOSED (Family F4):** `[FUZZ FIND] kind=1 op=3 item=3 site=249 seed=1232722137 orig=0x0007 mut=0x7EFB`
  - All other `224` finds on `item=0..6` are **100% identical** in `kind`, `op`, `item`, `site`, `seed`, `orig`, and `mut`.
- **Re-pinned corpus item (`item=7` = `pure/verifier_standalone.omni_bc`, `15,524 w -> 15,584 w`):**
  - Because `item=7` itself grew by 60 words when re-pinned, `fg_mod(rng, nw - 8)` mapped the fixed PRNG seeds to shifted word offsets within `item=7`:
    - Shifted (same seed `2037675677`, still **Family F2** unlowered `0x7F49`): `site=14876 (0x7F43->0x7F49)` → `site=14820 (0x7F06->0x7F49)`
    - 3 additional existing **Family F3** (`RET 0x7F4A` mutated to dest-requiring op in destless slot) hits landed on `0x7F4A` sites: `site=6225 (0x7F4A->0x7F50)`, `site=15503 (0x7F4A->0x7F14)`, `site=15550 (0x7F4A->0x7F57)`
  - **Zero new root-cause families introduced.** All 4 `item=7` records belong to existing triaged families (`1× F2`, `3× F3`).

---

## 5. For the Record (Part A Harness Change)

In PR #60 (`main @ 020c922`), `pure/harness/fuzz_gates.omni::fg_verify_bc`'s verifier-output read ceiling was raised from `4096 B` to `32768 B` (band B `+200000`) to avoid clobbering `/proc/self/cmdline`; harness-internal only, no component under test affected.

---

## 6. Close-Out Gates (WSL2 Ubuntu Linux x86-64)

- `bash scripts/zero_rust_rebuild.sh`: **PASS** (all 10 MANIFEST pins bit-exact, `S1==S2==S3==V`, `omnic_pure.elf` fixed point `59,653 B` 0-byte diff)
- `bash scripts/verify_step7_driver.sh`: **38/38 PASS**
- `bash scripts/verify_step8_harness.sh`: **57/57 PASS**
- `./build/pure-bin/omnic_pure.elf selftest`: **PASS**
- `python3 scripts/external_verifier_check.py`: **PASS** (`8/8` traces)
