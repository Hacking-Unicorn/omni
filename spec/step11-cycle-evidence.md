# Step 11 Part F — Live Proposer Cycle Evidence (`spec/step11-cycle-evidence.md`)

- **Branch:** `feat/step11-live-path` (off `main @ 8d21afe`, zero Rust)
- **Environment:** WSL2 Ubuntu Linux x86-64, `PATH=/usr/bin:/bin` (`rustc`/`cargo` absent, `0` `.rs`/`Cargo.*` files in tree)
- **Reference Spec:** `spec/m7/PROPOSER_LOOP_DESIGN.md`
- **Final Judge Verdict:** **`+1t` (ADOPT)** — decided by `judge_cli.elf` (`pure/m7/judge.omni` + `pure/m7/learn.omni` + `pure/m7/heal.omni`) on deterministic metrics (`bc_words`: `733 -> 714` [`-19` w], `elf_bytes`: `3,609 -> 3,574` [`-35` B], `0`-variance across 5 sequential runs)
- **`flywheel/auto` Commit SHA:** `73792721583f53bdc2d64b154be13d4f3b9e36e2` (`7379272`), produced directly by `proposer_live.elf` on branch `flywheel/auto` only (`main` and `feat/step11-live-path` untouched)
- **TCB & Pin Integrity:** Zero TCB files modified; all 10 post-Step-9 TCB digests matched pre- and post-cycle; zero frozen hashes, seeds, or `bootstrap/MANIFEST.md` pins moved.

---

## 1. Stage 0 — Scope & TCB Deny-List Check
- **Verdict:** **PASS (`stage0=OK`)**
- **Target:** `pure/m5/zero_rust_driver.omni` — single non-TCB file, outside the TCB deny-list (`bootstrap/`, `pure/verifier*`, `pure/isa/table.omni`, `pure/m7/judge.omni`, `pure/m7/heal.omni`, `scripts/zero_rust_rebuild.sh`, `.github/workflows/ci.yml`, `spec/harness-accounting.md`, `spec/acceptance/goldens/`, `spec/verifier_traces/`) and outside the Frozen-5 suite.
- **TCB Digest Lock:** Combined FNV-1a64 digest of all 10 post-Step-9 TCB files snapshotted before evaluation and re-verified identical after evaluation.

---

## 2. Stage 1 — Measurement-Driven Target Selection (`p7-measure`)
- **Command:** `./build/pure-bin/omnic_pure.elf p7-measure`
- **Verbatim Output:**
```text
pure/programs/compiler_self_audit.omni	3272	6369	0x0D60079918FB367C	var=0
pure/programs/kv_ledger_service.omni	553	1390	0x05EC40933A646500	var=0
pure/programs/pack5_decision_controller.omni	1039	2824	0xD67BCC4F52B22152	var=0
pure/m5/zero_rust_driver.omni	733	3609	0x089318743A0EB233	var=0
[P7-MEASURE PASS] 4 targets | 5 runs | 0-var
```
- **Target Choice:** `pure/programs/*` belong to the frozen 18-module core suite (`m2-frozen`, `0x78462AE4C81DF408`), so `pure/m5/zero_rust_driver.omni` is the non-frozen, non-TCB target in the standard `p7-measure` set.
- **Candidate Diff (branch elimination on boolean `is_child ∈ {0, 1}`):**
```diff
--- a/pure/m5/zero_rust_driver.omni
+++ b/pure/m5/zero_rust_driver.omni
@@ -78,7 +78,7 @@ fn p2a_run_io_and_spawn(mode: i64) -> i64 {
     } else {
         sys_spawn(p2a_path_true(), empty_arg)
     };
-    let base: i64 = if_pos is_child <=> 0 { 2000 } else { 1000 };
+    let base: i64 = 1000 + is_child * 1000;
     return base * match_ok + nw + nr + child_rc + (cw + cr + oob_res + ex_res) * 0;
 }
```

---

## 3. Stage 2 — Canary Ruler Proof (`p7-canary`)
- **Verdict:** **PASS (`stage2=canary:OK`)**
- **Command (invoked by `proposer_live.elf`):** `p7_gates.elf p7-canary`
- **Output:** `[P7-CANARY PASS] canary_a=+1t canary_b=0t canary_c=-1t frozen5=OK | pure/m7/judge.omni`

---

## 4. Stage 3 — 5-Run Consensus Measurement (0-Variance)
- **Runs:** 5 sequential non-concurrent compiles (`m3.elf` + `verifier.elf` + `m2.elf`) of baseline and 5 of candidate.
- **Timing Note (Step 11 Part C.4):** Native timing (`sys_clock_ns`) is deferred to Step 11b; timing is recorded as `EXTERNAL` and is **not** used as an ADOPT input — the judge evaluated deterministic metrics only (`band = 0`).

| Metric | Baseline (5 runs, `var=0`) | Candidate (5 runs, `var=0`) | Delta |
|---|---:|---:|---:|
| `bc_words` | `733, 733, 733, 733, 733` | `714, 714, 714, 714, 714` | **`-19` words (`-2.59%`)** |
| `elf_bytes` | `3609, 3609, 3609, 3609, 3609` | `3574, 3574, 3574, 3574, 3574` | **`-35` B (`-0.97%`)** |
| Semantic check (Linux x86-64) | `mode0=1016, mode1=rc1, mode2=2016, mode3=rc42` | `mode0=1016, mode1=rc1, mode2=2016, mode3=rc42` | **Identical (4/4)** |

---

## 5. Stage 4 — Real Judge (`pure/m7/judge.omni`) & Auto-Commit (`flywheel/auto`)
- **Command Executed:**
  `./build/pure-bin/omnic_pure.elf propose-run --target pure/m5/zero_rust_driver.omni --candidate <cand_zrd.omni>`
- **Verbatim Output (`rc=0`):**
```text
[PROPOSE-RUN LIVE] stage0=OK stage2=canary:OK stage3=5r/0v base=733/3609 cand=714/3574 dry=0
verdict=+1t ADOPT | learn_updates=2 | heal_action=0
```
- **Judge Inputs (`judge_cli.elf` -> `pure/m7/judge.omni`):**
  - `gates_ok=1`, `base_w=733`, `cand_w=714`, `base_elf=3609`, `cand_elf=3574`, `band=0` (5-run consensus via `p7_judge_5runs` + `p7_combine2` + `p7_adopt_rule`) → **`+1t ADOPT`**
  - `pure/m7/learn.omni::p7_learn_update` persisted (`learn_updates=2`)
  - `pure/m7/heal.omni::p7_heal_decide(err_code=0)` → `0` (`ACTION_PASS`)
- **`flywheel/auto` Commit:**
```text
commit 73792721583f53bdc2d64b154be13d4f3b9e36e2
Author: Omni Flywheel <omni-flywheel@users.noreply.github.com>
Date:   Thu Nov 26 17:20:13 2026 -0500

    flywheel(auto): ADOPT

 pure/m5/zero_rust_driver.omni | 2 +-
 1 file changed, 1 insertion(+), 1 deletion(-)
```
