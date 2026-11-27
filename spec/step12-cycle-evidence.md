# Step 12 — Flywheel Operations: First Bounded Batch of Live Cycles

**Branch:** `feat/step12-flywheel-batch` (off `main @ 1538e6d`)  
**Host / Target:** WSL2 Ubuntu Linux x86-64 (native Pure-Omni stack, zero Rust, native `sys_clock_ns()` timing from Step 11b)  
**Outcome:** **2 live cycles run, 0 ADOPTs, 2 REJECTs (`-1t`, `-1t`) — halted early after Cycle 2 by the standing stop condition (2 consecutive non-ADOPT verdicts).**

---

## 1. Batch Setup & Standing Rules Record

1. **Sanctioned pre-batch `flywheel/auto` rebase (Standing Rule 1 & Constraints):**
   - Rebased the Step 11 ADOPT commit (`pure/m5/zero_rust_driver.omni`, single-line redundant-condition removal) cleanly onto `main @ 1538e6d` with zero conflicts, and pushed to `origin` before Cycle 1:
   - Remote confirmation (`git ls-remote origin flywheel/auto`):
     ```text
     d0ca579ca7f28bb7a1a2f8cdcd9914062011f464	refs/heads/flywheel/auto
     ```
2. **Pre-batch harness fixes on `feat/step12-flywheel-batch` (named per Standing Rule 2):**
   - `pure/m7/judge_cli.omni`: added per-cycle `learn_delta` (`+2` on `+1t` ADOPT, `0` on `0t` HOLD, `-3` on `-1t` REJECT) alongside cumulative `learn_updates` in the judge output line (`verdict=<...> | learn_delta=<D> | learn_updates=<N> | heal_action=<A>`), resolving the Steps 11/11b learn-counter evidence reproducibility issue.
   - `pure/harness/proposer_live.omni`: updated `plv_git_adopt_commit` from `git checkout -B flywheel/auto` to `git checkout flywheel/auto 2>/dev/null || git checkout -b flywheel/auto` so multi-cycle ADOPTs stack cleanly on the existing `flywheel/auto` branch instead of resetting it to `HEAD`.
3. **Preconditions (all PASS on `feat/step12-flywheel-batch` before Cycle 1):**
   - `bash scripts/zero_rust_rebuild.sh`: **PASS**
   - `bash scripts/verify_step7_driver.sh`: **38/38 PASS** (`FAIL=0`)
   - `bash scripts/verify_step8_harness.sh`: **49/49 PASS** (`FAIL=0`)
   - `./build/pure-bin/omnic_pure.elf selftest`: **PASS** (clean 3-line summary)
4. **TCB & Pins:** Untouched (Standing Rule 3). No pin, frozen hash, or `bootstrap/MANIFEST.md` entry moved.

---

## 2. Measurement-Driven Candidate Selection (`p7-measure`)

Baseline `omnic_pure.elf p7-measure` across the non-TCB, non-frozen standalone candidate pool on `main @ 1538e6d` (ranked by code size / compile latency):

| Rank | Target | Words (`w`) | Bytes (`B`) | Digest (FNV-1a64) | Median Compile (`us`) | Variance |
| ---: | :--- | ---: | ---: | :--- | ---: | ---: |
| 1 | `pure/audit/decompile.omni` | 29,283 | 78,306 | `0xF2DD9A742C8DE009` | 66,052 | 0 |
| 2 | `pure/audit/diff_summary.omni` | 14,376 | 42,242 | `0x9C6D483C83C64E18` | 36,515 | 0 |
| 3 | `pure/harness/audit_view_gate.omni` | 12,168 | 40,802 | `0xE07FFE780D0251C1` | 32,482 | 0 |
| 4 | `pure/harness/extra_gates.omni` | 10,353 | 32,552 | `0xCD608A6E6D657E93` | 27,018 | 0 |
| 5 | `pure/harness/m2_lint.omni` | 9,361 | 27,445 | `0x7BC322E87F1F1292` | 25,643 | 0 |
| 6 | `pure/m5/pure_build_util.omni` | 2,879 | 8,949 | `0x7D701DF6CF99C691` | 9,964 | 0 |

---

## 3. Cycle-by-Cycle Evidence

### Cycle 1 — `pure/audit/decompile.omni`

- **Target:** `pure/audit/decompile.omni` (Rank 1 by `p7-measure`: `29,283 w`, `78,306 B`, `66,052 us`)
- **Candidate Diff Summary:** Local CSE and branch fold in `dc_emit_br_fallback` and `dc_arm_tail_nested` (hoisting repeated buffer/offset lookups into local variables and collapsing redundant guard branches). Candidate written to out-of-tree temp path `/tmp/.../c1.omni` and submitted via `omnic_pure.elf propose-run --target pure/audit/decompile.omni --candidate /tmp/.../c1.omni`.
- **Stage Verdicts:**
  - **Stage 0 (TCB Guard — 10-file digests + path allowlist):** `OK`
  - **Stage 1/2 (Canary):** `canary:OK`
  - **Stage 3 (5-Run Measure, native `sys_clock_ns`):** `5r/0v`
    - Baseline: `29,283 w` / `78,306 B` / `67,418 us`
    - Candidate: `-1 w` / `-1 B` / `528 us` (candidate compile exited non-zero; fail-closed sentinel `-1/-1`)
  - **Stage 4 (Real Judge — `judge_cli.elf`):**
    - Inputs: `base_w=29283`, `cand_w=-1`, `base_b=78306`, `cand_b=-1`, `base_us=67418`, `cand_us=528`, `var=0`
    - Verdict: **`-1t REJECT`** (`rc=1`)
    - Per-cycle learn delta: **`learn_delta=-3`** (`learn_updates=1`, `heal_action=1`)
- **Raw Loop Output:**
  ```text
  [PROPOSE-RUN LIVE] stage0=OK stage2=canary:OK stage3=5r/0v base=29283/78306 cand=-1/-1 us=67418/528 dry=0
  verdict=-1t REJECT | learn_delta=-3 | learn_updates=1 | heal_action=1
  ```
- **Git / `flywheel/auto`:** No commit written (`—`). `NON_ADOPT_STREAK=1`.

---

### Cycle 2 — `pure/audit/diff_summary.omni`

- **Target:** `pure/audit/diff_summary.omni` (Rank 2 by `p7-measure`: `14,376 w`, `42,242 B`, `36,515 us`)
- **Candidate Diff Summary:** Local CSE hoisting `let w1 = ad_u16(..., pos + 1)` in `ad_imm_val` to eliminate duplicate halfword reads. Candidate written to out-of-tree temp path `/tmp/.../c2.omni` and submitted via `omnic_pure.elf propose-run --target pure/audit/diff_summary.omni --candidate /tmp/.../c2.omni`.
- **Stage Verdicts:**
  - **Stage 0 (TCB Guard — 10-file digests + path allowlist):** `OK`
  - **Stage 1/2 (Canary):** `canary:OK`
  - **Stage 3 (5-Run Measure, native `sys_clock_ns`):** `5r/0v`
    - Baseline: `14,376 w` / `42,242 B` / `36,101 us`
    - Candidate: `-1 w` / `-1 B` / `533 us` (candidate compile exited non-zero; fail-closed sentinel `-1/-1`)
  - **Stage 4 (Real Judge — `judge_cli.elf`):**
    - Inputs: `base_w=14376`, `cand_w=-1`, `base_b=42242`, `cand_b=-1`, `base_us=36101`, `cand_us=533`, `var=0`
    - Verdict: **`-1t REJECT`** (`rc=1`)
    - Per-cycle learn delta: **`learn_delta=-3`** (`learn_updates=2`, `heal_action=1`)
- **Raw Loop Output:**
  ```text
  [PROPOSE-RUN LIVE] stage0=OK stage2=canary:OK stage3=5r/0v base=14376/42242 cand=-1/-1 us=36101/533 dry=0
  verdict=-1t REJECT | learn_delta=-3 | learn_updates=2 | heal_action=1
  ```
- **Git / `flywheel/auto`:** No commit written (`—`). `NON_ADOPT_STREAK=2`.

---

## 4. Stop-Condition Halt & Root-Cause Finding

- **Stop Condition Triggered:** 2 consecutive non-ADOPT verdicts (`-1t REJECT`, `-1t REJECT`). Per Standing Rule 4 (*"Fail closed, report honestly. Never tune a candidate and retry it within the batch; never assist a verdict"*), the batch halted immediately after Cycle 2 with zero retries and zero mid-batch workarounds.
- **Root Cause Identified (logged as Backlog Item 8 in `spec/step10-hardening-backlog.md`):**
  - Both `pure/audit/decompile.omni` and `pure/audit/diff_summary.omni` contain relative `include "../..."` directives (e.g., `include "../m2/util.omni";`, `include "../driver/sys_util.omni";`).
  - When `propose-run --target <repo-path> --candidate /tmp/.../cand.omni` invokes `omnic_pure.elf compile` on the out-of-tree `/tmp/.../cand.omni`, `omnic_driver.omni::od_compile_omni` resolves relative `include` directives against the candidate's parent directory (`/tmp/.../`) instead of the target's directory (`pure/audit/`), causing `od_read_include` to fail (`rc=1`, `cand=-1/-1`).
  - The Step 11 ADOPT target (`pure/m5/zero_rust_driver.omni`) had no `include` directives, so this path-resolution gap had not surfaced until Step 12 ran live cycles on multi-file included modules.
  - Because the loop failed closed (`cand=-1/-1` → `-1t REJECT`, `learn_delta=-3`, `heal_action=1`, zero Git writes), the safety gates functioned exactly as designed.

---

## 5. Cumulative `p7-measure` Effect (`flywheel/auto` vs `main`)

Measured by running `omnic_pure.elf p7-measure` on `flywheel/auto @ d0ca579` vs `main @ 1538e6d` at batch end:

| Module | `main @ 1538e6d` (`w` / `B` / `us`) | `flywheel/auto @ d0ca579` (`w` / `B` / `us`) | Delta (`w` / `B` / `us`) | Source |
| :--- | :--- | :--- | :--- | :--- |
| `pure/m5/zero_rust_driver.omni` | `733 w` / `3,609 B` / `5,066 us` | `714 w` / `3,574 B` / `4,979 us` | **`-19 w` / `-35 B` / `-87 us`** | Rebased Step 11 ADOPT (`d0ca579`) |
| `pure/audit/decompile.omni` | `29,283 w` / `78,306 B` / `66,052 us` | `29,283 w` / `78,306 B` / `66,052 us` | `0 w` / `0 B` / `0 us` | Cycle 1 REJECT |
| `pure/audit/diff_summary.omni` | `14,376 w` / `42,242 B` / `36,515 us` | `14,376 w` / `42,242 B` / `36,515 us` | `0 w` / `0 B` / `0 us` | Cycle 2 REJECT |
| `pure/harness/audit_view_gate.omni` | `12,168 w` / `40,802 B` / `32,482 us` | `12,168 w` / `40,802 B` / `32,482 us` | `0 w` / `0 B` / `0 us` | Not run (halted) |
| `pure/harness/extra_gates.omni` | `10,353 w` / `32,552 B` / `27,018 us` | `10,353 w` / `32,552 B` / `27,018 us` | `0 w` / `0 B` / `0 us` | Not run (halted) |
| `pure/harness/m2_lint.omni` | `9,361 w` / `27,445 B` / `25,643 us` | `9,361 w` / `27,445 B` / `25,643 us` | `0 w` / `0 B` / `0 us` | Not run (halted) |

- **Step 12 batch-only delta:** `0 w` / `0 B` / `0 us` (0 new ADOPTs in 2 cycles).
- **Cumulative `flywheel/auto @ d0ca579` vs `main @ 1538e6d`:** **`-19 w` / `-35 B` / `-87 us`** (from the rebased Step 11 ADOPT on `pure/m5/zero_rust_driver.omni`).
- **Final remote `flywheel/auto` (`git ls-remote origin flywheel/auto`):**
  ```text
  d0ca579ca7f28bb7a1a2f8cdcd9914062011f464	refs/heads/flywheel/auto
  ```

---

## 6. Post-Batch Close-Out Gates

All four close-out gates verified on `feat/step12-flywheel-batch` (WSL2 Ubuntu Linux x86-64):

- `bash scripts/zero_rust_rebuild.sh`: **PASS**
- `bash scripts/verify_step7_driver.sh`: **38/38 PASS** (`FAIL=0`)
- `bash scripts/verify_step8_harness.sh`: **49/49 PASS** (`FAIL=0`)
- `./build/pure-bin/omnic_pure.elf selftest`: **PASS**
