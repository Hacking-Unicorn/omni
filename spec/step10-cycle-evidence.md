# Step 10 — First Live Proposer Cycle Evidence (`spec/step10-cycle-evidence.md`)

- **Branch:** `feat/step10-live-flywheel` (off `main @ c0e05fd`, post-Step 9, zero Rust)
- **Environment:** WSL2 Ubuntu Linux x86-64, `PATH=/usr/bin:/bin` (`rustc`/`cargo` absent, `0` `.rs`/`Cargo.*` files in tree)
- **Reference Spec:** `spec/m7/PROPOSER_LOOP_DESIGN.md`
- **Final Cycle Verdict:** **`-1t` (REJECT — halted fail-closed at Stage 1 / Stage 2 due to native tooling gaps: `omnic_pure.elf p7-measure` and `omnic_pure.elf p7-canary` both exit `rc=2`, and `proposer_loop.elf` ignores CLI arguments / has no live candidate input path)**
- **`flywheel/auto` Commit SHA:** None (no ADOPT; no `flywheel/auto` branch created)
- **TCB / Pin Integrity:** Zero TCB files modified; zero frozen hashes, seeds, or `bootstrap/MANIFEST.md` pins moved.

---

## 1. Preconditions (All PASS)

| Precondition | Command | Exit Code | Result |
|---|---|---:|---|
| Clean-room zero-Rust rebuild | `PATH=/usr/bin:/bin bash scripts/zero_rust_rebuild.sh` | `0` | **PASS** (`0` Rust/Cargo files, all 10 `bootstrap/MANIFEST.md` pins bit-exact, `m2.elf` `S1==S2==S3`, `m3.elf` `link-elf` bit-exact, `omnic_pure.elf` self-compile fixed point `54,923 B`, all 11 named harness gates + `selftest` PASS) |
| Native proposer gate | `./build/pure-bin/omnic_pure.elf proposer-loop` | `0` | **PASS** (`242 B`: `[PROPOSER-LOOP PASS] 10 checks \| ...`) |
| Native selftest | `./build/pure-bin/omnic_pure.elf selftest` | `0` | **PASS** (`[PURE-HARNESS PASS] ...`) |

---

## 2. Stage-by-Stage Live Execution Trail

### Stage 0 — Scope & TCB Deny-List Check
- **Verdict:** **PASS (scope verified) / GAP OBSERVED in `proposer_loop.elf`**
- **Scope:** Evaluated non-TCB candidate pool under `pure/programs/` (`compiler_self_audit.omni`, `kv_ledger_service.omni`, `pack5_decision_controller.omni`) and `pure/m5/zero_rust_driver.omni`. None are in the TCB deny-list (`pure/verifier_engine.omni`, `verifier.elf`, `pure/m7/judge.omni`, frozen seeds/goldens, `bootstrap/`, `spec/harness-accounting.md`).
- **Observed Gap:** `pure/harness/proposer_loop.omni::pl_stage0_guard()` (lines 124–144) only hashes 3 files (`pure/verifier_standalone.omni_bc`, `pure/m7/judge.omni` non-empty check only, and `pure/m2/m2.omni_bc`). It does not accept a candidate file/diff to validate against the allowlist, nor does it digest-lock `bootstrap/m2.elf`, `bootstrap/m3.elf`, `bootstrap/MANIFEST.md`, `pure/verifier_engine.omni`, `pure/isa/table.omni`, `pure/m7/heal.omni`, `scripts/zero_rust_rebuild.sh`, `.github/workflows/ci.yml`, or `spec/harness-accounting.md`.

### Stage 1 — Measurement-Driven Proposal (`p7-measure` & `propose-run`)
- **Verdict:** **BLOCKED / HALTED (Tooling Gap)**
- **Commands Executed:**
  1. `./build/pure-bin/omnic_pure.elf p7-measure` → exited **`rc=2`** (stdout empty; subcommand not recognized in `pure/driver/omnic_driver.omni::od_classify_cmd`, lines 456–486).
  2. `./build/pure-bin/omnic_pure.elf propose-run --dry-run` → exited `rc=0`, **but** silently dropped `--dry-run` (`omnic_driver.omni:525` spawns `proposer_loop.elf` with `empty = anew(0)`) and executed only the hardcoded synthetic self-test (`x*3 + 0 + (y+7)*1`) inside `proposer_loop.omni::main()`, which never inspects `argc`/`argv`.
- **Candidate Diff:** None applied (per Step 10 rules, candidate selection must be driven by `p7-measure` data and not taste, and the loop/driver must not be modified to force a pass).
- **Supplemental Sequential Baseline Telemetry (5 runs, non-concurrent, external wall-clock via `date +%s%N` because M2 lacks a `clock_gettime` syscall intrinsic):**

| Non-TCB Program | `bc_words` (5 runs, 0-var) | `elf_bytes` (5 runs, 0-var) | `compile_us` (5 sequential runs) | Median `compile_us` |
|---|---:|---:|---|---:|
| `pure/programs/compiler_self_audit.omni` | `3272, 3272, 3272, 3272, 3272` | `6369, 6369, 6369, 6369, 6369` | `19690, 14801, 15541, 14529, 15468` | `15,468` |
| `pure/programs/kv_ledger_service.omni` | `553, 553, 553, 553, 553` | `1390, 1390, 1390, 1390, 1390` | `14034, 11637, 9384, 10541, 9603` | `10,541` |
| `pure/programs/pack5_decision_controller.omni` | `1039, 1039, 1039, 1039, 1039` | `2824, 2824, 2824, 2824, 2824` | `12941, 11206, 9912, 10176, 10290` | `10,290` |
| `pure/m5/zero_rust_driver.omni` | `733, 733, 733, 733, 733` | `3609, 3609, 3609, 3609, 3609` | `8760, 10075, 9395, 9015, 9375` | `9,375` |

### Stage 2 — Canary Ruler Proof (`p7-canary`)
- **Verdict:** **`-1t` (REJECT — Halted Fail-Closed)**
- **Command Executed:** `./build/pure-bin/omnic_pure.elf p7-canary` → exited **`rc=2`** (stdout empty; subcommand not implemented on the native Pure-Omni stack).
- **Spec Rule Applied (`spec/m7/PROPOSER_LOOP_DESIGN.md` §4 Stage 2):** *"Before the candidate is allowed to be judged, the measurement apparatus itself must pass `omnic p7-canary` on the full 6-benchmark corpus... If `p7-canary` fails or drifts from `+1t`/`0t`, the proposal is immediately rejected and the loop halts (never judge a candidate with a broken ruler)."*
- **Action:** Cycle halted cleanly at Stage 2 with verdict **`-1t` (REJECT)**. No candidate was tuned, retried, or hand-judged.

### Stage 3 — 5-Run Consensus Measurement
- **Verdict:** **NOT ENTERED (short-circuited by Stage 2 REJECT)**
- **Native Stack Readiness Findings:**
  - M2 Phase 2a syscalls (`sys_read`, `sys_write`, `sys_open`, `sys_close`, `sys_exit`, `sys_spawn`) do not include `clock_gettime` (Linux x86-64 syscall 228), so `proposer_loop.elf` cannot measure `compile_us` or `vm_us` natively.
  - `proposer_loop.omni` executes only 1 run of its hardcoded synthetic snippet and has no 5-run sequential timing harness or 0-variance assertion across runs.

### Stage 4 — Judge (`pure/m7/judge.omni`) & Auto-Commit (`flywheel/auto`)
- **Verdict:** **NOT ENTERED (short-circuited by Stage 2 REJECT; overall cycle verdict `-1t` REJECT)**
- **Native Stack Readiness Findings:**
  - `pure/m7/judge.omni` compiles cleanly under `omnic_pure.elf` (`4,340 B` / `2,170` words), **but** `spec/m7/judge_harness.omni` fails standalone compilation (`rc=1`) because it omits `include "../../pure/m7/judge.omni";`.
  - `pure/harness/proposer_loop.omni` never invokes `pure/m7/judge.omni`, `pure/m7/heal.omni`, or `pure/m7/learn.omni` — it uses an inlined 3-line `pl_judge_verdict()` (lines 243–247) without noise-band thresholding or 5-run voting, and has zero Git / `flywheel/auto` branch commit logic despite printing `+ flywheel/auto Git SHA-256 store & GC` in `pl_print_pass()`.

---

## 3. Post-Cycle Integrity Verification

- `bash scripts/zero_rust_rebuild.sh`: **PASS**
- `bash scripts/verify_step7_driver.sh`: **38/38 PASS** (`FAIL=0`)
- `bash scripts/verify_step8_harness.sh`: **37/37 PASS** (`FAIL=0`)
- `./build/pure-bin/omnic_pure.elf selftest`: **PASS**
- `git branch -a`: confirms no `flywheel/auto` branch was created.
