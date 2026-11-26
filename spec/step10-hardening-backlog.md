# Step 10 — Live Proposer Cycle Hardening Backlog (`spec/step10-hardening-backlog.md`)

Every friction, stall, ambiguity, or tooling gap surfaced during the first live proposer cycle on the post-Step-9 zero-Rust native Pure-Omni stack (`main @ c0e05fd`), ordered by severity.

---

## P0 — Cycle Blockers (Prevented Stages 1–4 From Executing Live)

### 1. Missing native `p7-measure` and `p7-canary` subcommands (`omnic_pure.elf` exits `rc=2`)
- **Where:** `pure/driver/omnic_driver.omni::od_classify_cmd` (lines 456–486); no Pure-Omni ELF exists for `p7-measure` or `p7-canary` (both were classified `PURE-LOGIC` in `spec/harness-accounting.md` and their Rust host driver was deleted in Step 9).
- **What it blocked:**
  - **Stage 1 (Propose):** Could not obtain the 6-benchmark `p7-measure` telemetry corpus to select a measurement-driven candidate.
  - **Stage 2 (Canary):** Per `spec/m7/PROPOSER_LOOP_DESIGN.md` §4 Stage 2, failing to pass `p7-canary` immediately halts the cycle as `-1t` (REJECT).
- **Remediation:** Port `p7-measure` (6-benchmark corpus runner + deterministic metrics + P4-5 trace ring) and `p7-canary` (Canary A `+1t` M2 tweak + Canary B `0t` neutral + frozen-5 check) to Pure-Omni harness ELFs and wire them into `omnic_driver.omni`.

### 2. `omnic_driver.omni` drops all CLI arguments to `proposer_loop.elf`, and `proposer_loop.omni` only runs a hardcoded synthetic self-test
- **Where:** `pure/driver/omnic_driver.omni:524-525` (`sys_spawn(od_tool_path(w, 7), empty)` passes empty args for both `proposer-loop` and `propose-run`); `pure/harness/proposer_loop.omni:251-283` (`main()` never inspects `argc`/`argv`).
- **What it blocked:**
  - **Stage 0 / Stage 1:** Impossible to invoke `omnic_pure.elf propose-run` on a real candidate `.omni` / `.omni_bc` file, target subsystem, or `--dry-run` flag; `propose-run --dry-run` silently ran the synthetic self-test (`x*3 + 0 + (y+7)*1`) instead.
- **Remediation:** Forward full CLI arguments (or a request file, like `audit-diff`) from `omnic_driver.omni` to `proposer_loop.elf`, and implement a real `propose-run <target> <candidate>` mode separate from the `proposer-loop` selftest.

### 3. `proposer_loop.omni` does not actually call `pure/m7/judge.omni`, `heal.omni`, `learn.omni`, or Git (`flywheel/auto`)
- **Where:** `pure/harness/proposer_loop.omni:243-247` (`pl_judge_verdict` is a 3-line inline check with no noise band and no 5-run voting) and lines 219–239 (`pl_print_pass` prints `+ Stage 2 canary + Stage 3 5-run 0-var + Stage 4 judge/heal/learn ... + flywheel/auto Git SHA-256 store & GC`, none of which are executed).
- **What it blocked:**
  - **Stage 4 (Judge & Adopt):** Even if Stages 1–3 had completed, `proposer_loop.elf` cannot invoke `pure/m7/judge.omni::p7_judge_5runs` / `p7_adopt_rule`, cannot perform `heal.omni` rollback logging, cannot update `learn.omni`, and cannot auto-commit a `+1t` ADOPT to `flywheel/auto`.
  - **Honesty hazard:** The pass banner over-claims capabilities that only existed in the deleted Rust `m2_host/proposer_loop.rs`.
- **Remediation:** Link or spawn a compiled `pure/m7/judge.omni` evaluator for Stage 4, wire `heal.omni`/`learn.omni` state persistence, implement `sys_spawn` Git commit to `flywheel/auto` on `+1t` ADOPT only, and align the banner with actual executed checks.

---

## P1 — Measurement & TCB Guard Gaps

### 4. No native timing syscall (`clock_gettime`) in M2 Phase 2a syscall intrinsics
- **Where:** `pure/m2/emit_multi.omni` (Phase 2a raw `0F 05` syscall set: `sys_read`, `sys_write`, `sys_open`, `sys_close`, `sys_exit`, `sys_spawn` only).
- **What it blocked:**
  - **Stage 3 (5-Run Timing Measure):** Pure-Omni binaries (`proposer_loop.elf`, `p7-measure`) cannot measure `compile_us`, `emit_us`, or `vm_us` natively without shelling out to `/usr/bin/date` or an external wrapper.
- **Remediation:** Add a `sys_clock_ns()` intrinsic (Linux x86-64 `clock_gettime(CLOCK_MONOTONIC, &ts)`, syscall 228) under `#[cap(SYS)]` (note: touches `m2.elf` — must be its own dedicated judge-reviewed PR with fixed-point proof).

### 5. Stage 0 TCB deny-list guard (`pl_stage0_guard`) only hashes 3 files and has no path-allowlist enforcement
- **Where:** `pure/harness/proposer_loop.omni:124-144`.
- **What it blocked:**
  - **Stage 0 (Scope Guard):** Only checks `pure/verifier_standalone.omni_bc`, `pure/m7/judge.omni` (existence `!= 0`, not pinned hash), and `pure/m2/m2.omni_bc`. Does **not** snapshot/re-verify `bootstrap/m2.elf`, `bootstrap/m3.elf`, `bootstrap/MANIFEST.md`, `pure/verifier_engine.omni`, `pure/isa/table.omni`, `pure/m7/heal.omni`, `scripts/zero_rust_rebuild.sh`, `.github/workflows/ci.yml`, or `spec/harness-accounting.md`, and does not validate modified paths against a single-subsystem allowlist.
- **Remediation:** Expand `pl_stage0_guard` to snapshot FNV-1a64 digests of the entire post-Step-9 TCB deny-list before and after candidate evaluation, and enforce single-subsystem path allowlisting on live patches.

---

## P2 — Harness & Standalone Compilation Friction

### 6. `spec/m7/judge_harness.omni` fails standalone compilation under `omnic_pure.elf` (`rc=1`)
- **Where:** `spec/m7/judge_harness.omni:1-15` (references `p7_cmp_band`, `p7_vote5`, `p7_adopt_rule`, etc. without `include "../../pure/m7/judge.omni";`).
- **What it blocked:** Could not directly compile `spec/m7/judge_harness.omni` with `omnic_pure.elf compile` to execute `judge.omni` entry points from the CLI (it previously relied on Rust string concatenation).
- **Remediation:** Add `include "../../pure/m7/judge.omni";` (or provide a dedicated standalone `pure/m7/judge_cli.omni` entrypoint).

### 7. `omnic_pure.elf selftest` stdout file-offset overwrite when redirected to a file
- **Where:** `pure/harness/core_gates.omni` (`cg_run_selftest`) + `su_spawn_capture` / `sys_spawn` stdout handling.
- **What it blocked:** When running `./build/pure-bin/omnic_pure.elf selftest > /tmp/out.txt`, the child `proposer_loop.elf` writes 242 bytes at file offset 0 and `core_gates.elf` subsequently writes `[PURE-HARNESS PASS] ...` overwriting the first 61 bytes, producing garbled output (`...proposer\nage 1 CEGIS gradient...`). Also affects clean log capture for evidence trails.
- **Remediation:** Ensure `core_gates.elf` captures or suppresses child stdout cleanly (or uses `O_APPEND` / separate capture file) before emitting its final `[PURE-HARNESS PASS]` summary line.
