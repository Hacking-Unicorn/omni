# Omni Phase 7 Plan — Maturation (Measurement, Emitter, Self-Healing, Self-Adaptive, Self-Learning)

**Base:** `main@bc2711e6` (M1–M3, Phase 4, Phase 5, Phase 6 merged and verified clean on Linux: monolith selftest `1,501` checks, workspace selftest `706` checks, zero-Rust proof passed with `rustc`/`cargo` off `PATH`, all 5 frozen FNV-1a64 hashes intact).

**Order of execution:** One step at a time, hard gate at the end of each step, verified on Linux:
`P7-0` → `P7-0a` → M2 Emitter → VM Hot Paths → `P7-1` (Self-Healing) → `P7-2` (Self-Adaptive) → `P7-3` (Self-Learning).

---

## P7-0 — Measurement (Current Step)

Build the eyes before the hands. Nothing is improved on purpose until the system can measure itself and distinguish **better (`+1t`)**, **neutral (`0t`)**, and **worse (`-1t`)**.

### 1. Fixed Benchmark Corpus (5 Deterministic Programs)

Every benchmark is compiled through the self-hosted pipeline, lowered to native x86-64 ELF64 via M2, and executed on the Pure-Omni M4 VM (`pure/m4/m4.omni_bc`) with the P4-5 binary trace ring (`0x70000..0x7FFFF`) enabled:

| ID | Benchmark | Args | Expected Result | What It Stresses |
|---:|---|---|---:|---|
| 0 | `bench_clamp` | `150, 10, 90` | `90` | Pure ALU / `imin` / `imax` |
| 1 | `bench_loop_sum` | `100` | `5050` | `while` loop runtime, fuel & trace ring |
| 2 | `bench_branch_tree` | `20` | `70` | Nested `if_pos` + `match_trit` CFG dispatch |
| 3 | `bench_multi_call` | `17, 23` | `1209` | Multi-function `CALL` DAG & M2-multi stack ABI |
| 4 | `bench_arena_buf` | `16` | `120` | Byte-buffer `anew` + `iset_i64` + ISA-B `ld_u64`/`st_u32` |

### 2. Measured Metrics (Plain `i64` Integers)

1. `compile_us` — compile wall-clock time (microseconds)
2. `bc_words` — emitted `.omni_bc` token-stream length (`u16` words)
3. `elf_bytes` — emitted native x86-64 ELF64 binary size (bytes)
4. `vm_steps` — deterministic fuel/steps executed by Pure-Omni M4 VM (`m4s_steps`)
5. `trace_ops` — live trace record count read from P4-5 binary trace ring (`0x70000..0x7FFFF`)
6. `arena_bytes` — peak VM arena heap usage (`m4s_heap_top`, bytes)

### 3. Balanced Ternary Decision Layer & Written Adopt/Reject Rule (`pure/m7/judge.omni`)

Plain integer measurements cross into balanced ternary (`{-1t, 0t, +1t}`) strictly at the decision boundary:

- **Thresholding without division (`p7_cmp_band(base, cand, band) -> trit`):**
  Let `delta = base - cand` (lower-is-better).
  - `delta > band` → `+1t` (**better**)
  - `delta < -band` → `-1t` (**worse**)
  - `-band <= delta <= band` → `0t` (**neutral / within noise band**)
  Uses only integer subtraction, `<=>` (`TCMP`), and branchless/3-way `if_pos` / `if_neg` / `match_trit`. Zero division, zero floating-point.
- **Consensus voting across repeated runs (`p7_vote3`, `p7_vote5`):**
  Uses ternary majority (`tmaj` from `std/trit.omni` for 3 runs; sign-sum `TCMP` for 5 runs) so noisy runs collapse to one robust trit verdict.
- **Written Adopt/Reject Rule (`p7_adopt_rule` + `p7_aggregate_suite`):**
  1. **Gates must pass (`gates_ok == 1`):** If any verification gate or program output check fails → **`-1t` (REJECT)**.
  2. **No regression on the fixed evaluation set:** If **any** benchmark in the corpus is judged `-1t` → **`-1t` (REJECT)**.
  3. **Measurable improvement required:** If no benchmark regresses and **at least one** benchmark is judged `+1t` → **`+1t` (ADOPT)**.
  4. **Neutral is held, not adopted:** If all benchmarks are `0t` → **`0t` (HOLD)**. A change that merely passes is not an improvement.
- **`@PACK5` Verdict Packing (`p7_pack5_byte`, `p7_unpack5_trit`):**
  All 5 benchmark trit verdicts pack into **1 single `@PACK5` radix-3 byte** (`0..=242`, `5` trits/byte, byte-identical to `pack5_encode`).

### 4. P7-0 Acceptance Gate (`omnic p7-measure`)

- `pure/m7/judge.omni` passes `m2-lint` (`0` gated findings, `<= 120` SSA, `<= 12` labels, no recursion, `<= 1` ANEW).
- Executes identically on Rust `OmniBinaryVm` and Pure-Omni M4 VM (`pure/m4/m4.omni_bc`).
- Runs the 5-benchmark corpus end-to-end (M3 compile, M2 ELF64 lower, M4 VM execution with P4-5 trace ring active) and verifies exact outputs and trace integrity.
- Proves the decision loop correctly classifies:
  1. A known **better** change (tighter verified stream: fewer words, smaller ELF, fewer VM steps/traces) → **`+1t` (ADOPT)**
  2. A known **worse** change (extra redundant ops: more words, larger ELF, more VM steps/traces) → **`-1t` (REJECT)**
  3. A known **neutral** change (within noise band / identical metrics) → **`0t` (HOLD)**
  4. A **mixed regression** suite (one benchmark better, one benchmark worse) → **`-1t` (REJECT)**
  5. Repeated-run ternary consensus (`tmaj` / 5-run vote) and `@PACK5` round-trip parity across all 243 valid bytes (`0..=242`).
- Verifier and all 5 frozen FNV-1a64 hashes remain completely unchanged.

---

## P7-0a — Measurement, Second Round (Sharpen, Then Stop)

Sharpen the measurement engine with repeated-run consensus, subsystem attribution, and trace-ring opcode histograms, prove it against two canaries, and then **stop refining measurement** and move on to actual improvements (M2 Emitter).

### 1. Pure-Omni Additions (`pure/m7/judge.omni`)

- **`p7_attribute_subsystem(v_m3: trit, v_m2: trit, v_m4: trit) -> i64`:** Branchless subsystem attribution bitmask (`0` = `NONE`, `1` = `M3_FRONTEND`, `2` = `M2_EMITTER`, `4` = `M4_VM`, combinations `3`/`5`/`6`/`7`).
- **`p7_judge_5runs(b0..c4, band) -> trit`:** 5-run paired comparison feeding `p7_cmp_band` into `p7_vote5` so single-run noise cannot flip verdicts.
- **`p7_op_class(op_tok) -> i64` + `p7_trace_histogram(w: bytes) -> i64`:** Reads the live P4-5 binary trace ring (`0x70000..0x7FFFF`) and packs 4 × 16-bit opcode class counts `(call_ret << 48) | (ctrl << 32) | (mem << 16) | alu` with invariant `alu + mem + ctrl + call_ret == min(trace_ops, 4096)`.

### 2. Phase-Separated Metrics & Two-Part Canary Proof (`omnic p7-canary`)

- Every benchmark records phase-separated timings (`fe_us`, `emit_us`, `vm_us`) alongside deterministic metrics (`bc_words`, `elf_bytes`, `vm_steps`, `trace_ops`, `arena_bytes`, 4-class trace histogram) across **5 repeated runs** with zero-variance verification on deterministic fields.
- **Canary A (Predictable M2 Emitter Tweak — `compact_zero_imm`):** Lowering `ImmInt(0)`/`ImmTrit(0)` via 2-byte `xor eax,eax`/`xor ecx,ecx` instead of 10-byte `mov reg, 0` saves exactly `8 * zero_loads` bytes. Judged **`+1t` (ADOPT)** across 5 runs, strictly attributed to **`M2_EMITTER` (`2`)**, zero trace-histogram drift, and natively verified on Linux x86-64.
- **Canary B (Deliberately Neutral Change):** Comment/whitespace-only source change across 5 runs. Judged **`0t` (HOLD)** and attributed to **`NONE` (`0`)**.
- **Discrimination Check:** Closed-form `bench_loop_sum` attributes to **`7` (`M3+M2+M4`)** and drops `hist_ctrl` to `0`, cleanly separated from pure M2 emitter moves (`2`).
- **Frozen Hashes:** All 5 frozen FNV-1a64 hashes and all 3 committed seeds remain completely untouched.

---

## First Improvement Target — M2 Multi-Function Emitter (`pure/m2/emit_multi.omni`)

4 flag-safe x86-64 compact encodings applied in lockstep across `pure/m2/emit_multi.omni`, `omni_rust_bootstrap.rs`, and `omni-workspace/crates/omni-aot/src/lib.rs` (leaving single-mode `emit_x86.omni`, `spec/m2/single_mode_lock.tsv`, and all 5 frozen FNV-1a64 hashes untouched):

1. **`disp8` frame loads & stores** (`Reg r <= 15`, `Param p <= 13`): 4 B vs 7 B (−3 B/op)
2. **3-tier compact immediates**: `0` → `xor r32, r32` (2 B), `1..2³¹−1` → `mov r32, imm32` (5 B), `−2³¹..−1` → `mov r64, imm32` (7 B), else 10 B
3. **Compact function prologue** (`frame_bytes <= 127` → `sub rsp, imm8`, −3 B/fn)
4. **Compact `CALL` stack cleanup** (`0` args → omit −7 B; `1..15` args → `add rsp, imm8` −3 B/call)

### Measured Results (`p7-measure`, `p7-canary`, `m2-fixed-point`, `m3-fixed-point`, `p5-selfhost`)

- **5 P7 Benchmarks:** `3,724 B → 3,401 B` (**−323 B**), 5/5 `+1t` (ADOPT), strictly attributed to `2 (M2_EMITTER)`, 0 trace-histogram drift.
- **`pure/m2/m2.omni_bc` ELF64 (`m2-fixed-point`):** `137,246 B → 93,561 B` (**−43,685 B / −31.8%**, `0x763478CC11DB8026`, 0-byte diff vs Rust; regenerated seed `33,551` words, `0xC30924711C8C95E9`).
- **`pure/m4/m4.omni_bc` ELF64 (`p5-selfhost`):** `109,740 B → 75,632 B` (**−34,108 B / −31.1%**, `0xBF4ABC26A91C2DE0`, 0-byte diff vs Rust).
- **`pure/verifier_engine.omni` E2E ELF64 (`m3-fixed-point`):** `41,285 B → 30,063 B` (**−11,222 B / −27.2%**, `0xA9290A98CF42DD6B`, 0-byte diff vs Rust).
- **All 5 frozen FNV-1a64 hashes + zero-shim `p5-sys` (177 B, `0x4C5943AD64761FA1`) + `single_mode_lock.tsv`:** 100% intact.

---

## Second Improvement Target — M4 VM Hot Paths (`pure/m4/vm.omni` + `pure/m4/arena.omni`)

Profile-guided tuning driven by the P4-5 trace ring (`bench_loop_sum`: 404/809 control-flow ops; `bench_arena_buf`: 136/271 control-flow ops):

1. **O(1) Pre-Indexed Label Cache (`0x20000..0x23FFF`):** Populates 16 local label slots per function once at load time (`m4v_index_fn_labels` / `m4v_record_lbl_slot`) so taken branches (`JMP`/`BR_TRIT`/`BR_POS`/`BR_NEG`) resolve via one `ld_u32` instead of an O(N) full-function decode scan (`m4v_lookup_label`, with fallback for >16 labels).
2. **Guarded Opcode-Family Dispatch (`m4v_exec_pure_val`):** Evaluates only the matching family (`MOV`, `m4o_arith`, `m4o_bit_cmp`, `m4o_trit`, `SEL`/`SLEN`) instead of calling all three unconditionally per step.
3. **`LABEL`/`NOP` Fast Path (`m4v_step_noncall`):** Skips `m4v_exec_alu_mem` completely on `LABEL` (`0x7F40`) and `NOP` (`0x7F00`).
4. **Single `m2d_ins_scan` per Step (`m4v_step_one`):** Reuses `nops` across `m4v_exec_alu_mem`/`m4v_exec_call`/`m4v_exec_ret` and skips `res_peek` when `trace_mode == 0`.

### Measured Results (`p7-measure`, `p4-vm`, `p4-wasm`, `p5-selfhost`)

- **`bench_loop_sum` VM runtime:** `581,512 us → 314,397 us` (**−45.9% / 1.85× faster**).
- **`bench_arena_buf` VM runtime:** `292,346 us → 115,114 us` (**−60.6% / 2.54× faster**).
- **Total 5-benchmark VM runtime:** `897,069 us → 455,642 us` (**−49.2% / ~2.0× faster**).
- **Regenerated seed `pure/m4/m4.omni_bc`:** `26,076` words (`0x00299F8C0BB3F88A`, `0`-word diff vs Rust), M2 ELF64 = `78,020` B (`0xF66A7081E2B43319`, `0`-byte diff vs Rust).
- **0 change** in `vm_steps`, `trace_ops`, 4-class trace histograms, or all 5 frozen FNV-1a64 hashes; `m2-lint` 16/16 (`0` findings).

---

## P7-1 — Self-Healing (`pure/m7/heal.omni`, `omnic p7-heal`)

Pure-Omni self-healing decision & trace engine (`pure/m7/heal.omni`, 118 lines, registered in `m2-lint` 17/17 with 0 findings):

1. **Balanced-Ternary Failure Taxonomy (`p7_classify_failure`):**
   - `0t` = `OK` (`0`)
   - `+1t` = **Healable** (`1`=DECODE_CORRUPT, `2`=VERIFY_REJECT, `3`=RUNTIME_TRAP, `4`=OUTPUT_MISMATCH, `5`=GATE_REGRESS)
   - `-1t` = **Fatal (fail-closed)** (`10`=VERIFIER_TAMPER, `11`=CAP_VIOLATION, `12`=FROZEN_HASH_DRIFT, `13`=ROLLBACK_CORRUPT, or any unknown/negative code)
2. **Content-Addressed Rollback Target (`P7RollbackTarget`):** Verifies `fnv1a_64_hash(rollback_stream) == expected_hash` before every rollback; a tampered rollback buffer triggers `ACTION_FATAL_ABORT`.
3. **Bounded N-Attempt Repair State Machine (`p7_heal_decide`):** `0`=PASS, `1`=ROLLBACK_AND_RETRY (`attempt < max_attempts`), `2`=ROLLBACK_FINAL (`attempt >= max_attempts` -> stop retrying and keep verified rollback), `3`=FATAL_ABORT (immediate halt on any fatal fault or corrupt rollback).
4. **Binary Healing Log (`p7_heal_log_push`, `p7_heal_log_verify`):** 16-B records `[attempt:u32, err_code:u32, action:u32, hash_lo32:u32]` in a 4 KiB buffer with monotonic-attempt & action-range integrity verification.
5. **Hard Gate (`omnic p7-heal`, 6 checks):** Fault-injected corrupt token stream (`0xDEAD` + truncated) → detected, rolled back to FNV-1a64-verified target (`0x51E71970112F81B7`), re-emitted passing stream (`ret=5050` on M4 VM), logged & verified (`2` entries); repeated bad streams cleanly halt at `max_attempts = 3`; fatal faults & tampered rollback abort immediately; verifier & all 5 frozen FNV-1a64 hashes remain untouched.

---

## P7-2 — Self-Adaptive (`pure/m7/adapt.omni`, `omnic p7-adapt`)

Pure-Omni workload-driven self-adaptive policy, hysteresis, and ledger engine (`pure/m7/adapt.omni`, registered in `m2-lint` 22/22 with 0 findings):

1. **Live Trace-Driven Workload Classifier (`p7_classify_workload`):** Reuses `pure/m7/judge.omni::p7_trace_histogram` on live P4-5 binary trace rings (`0x70000..0x7FFFF`) to classify executing programs into `WORKLOAD_IDLE (0)`, `WORKLOAD_ALU (1)`, `WORKLOAD_MEM (2)`, `WORKLOAD_CTRL (3)`, or `WORKLOAD_CALL (4)` (division-free, float-free).
2. **Capability-Bounded Policy Selector (`p7_select_policy`):** Maps workload profiles into deterministic strategy masks (`POLICY_STRIP_NOP=1`, `POLICY_PREFETCH_LBL=2`, `POLICY_ARENA_FAST=4`, `POLICY_CALL_DAG=8`) strictly intersected with `cap_mask` (`cap_mask == 0` fails closed to `POLICY_HOLD`).
3. **Hysteresis Anti-Thrashing State Machine (`p7_adapt_decide`):** Requires sustained `min_streak` observations + `+1t` verdict before emitting `ADAPT_COMMIT (1)`; holds (`0`) on noisy flips or `0t`; immediately emits `ADAPT_REVERT (2)` and resets streak to `0` on any `-1t` regression.
4. **Binary Adaptation Ledger (`p7_adapt_log_push`, `p7_adapt_log_verify`):** 16-B records `[epoch:u32, profile:u32, policy:u32, verdict_action:u32]` in a 4 KiB buffer with monotonic-epoch, profile-range, and action-range integrity checks.
5. **Hard Gate (`omnic p7-adapt`, 6 checks):** M3/M2 0-diff on `pure/m7/adapt.omni`, 5-benchmark live trace classification (`ALU`/`CTRL`/`CTRL`/`CALL`/`MEM`), capability-bounded policy selection, hysteresis anti-thrashing transitions, closed-loop self-adaptive run combining `pure/m7/adapt.omni` + `pure/api/adaptive.omni` + M2 ELF64 + M4 VM (`ret=90`) + binary ledger verification, and all 5 frozen FNV-1a64 hashes intact.

---

## P7-3 — Self-Learning (`pure/m7/learn.omni`, `omnic p7-learn`)

Pure-Omni persistent, deterministic, balanced-ternary experience table & policy-ranking engine (`pure/m7/learn.omni`, registered in `m2-lint` 23/23 with 0 findings):

1. **Tamper-Checked Binary Experience Table (`p7_learn_init`, `p7_learn_stamp_checksum`, `p7_learn_verify`):** 512 B buffer (`magic="OML7"`, `version=1`, `total_updates`, 31-bit FNV-1a rolling checksum over header + 20 cells of `[wins, losses, neutral, score]` for 5 workload profiles × 4 candidate policies). Any bit-flip fails closed (`verify == -1`, `rank_best == 0`).
2. **Asymmetric Ternary Reinforcement (`p7_learn_update`):** `+1t` → `wins += 1, score += 2`; `0t` → `neutral += 1`; `-1t` → `losses += 1, score -= 3` (clamped `[0, 65535]`, bias `32768`) — safety bias ensures a single regression outweighs a single win.
3. **Safety-Floored, Capability-Bounded Policy Ranker (`p7_learn_rank_best`):** Ranks the 4 candidate policy slots (`bits 1/2/4/8`) intersected with `cap_mask` and strictly rejects any slot below the neutral safety floor (`32768`), failing closed to `0` (`POLICY_HOLD`).
4. **`@PACK5` Experience Summary Export (`p7_learn_export_pack5`):** Packs the learned ternary preference (`+1t`/`0t`/`-1t`) across all 5 workload profiles for a policy slot into 1 single `@PACK5` radix-3 byte (`0..=242`) via `pure/m7/judge.omni::p7_pack5_byte`.
5. **Hard Gate (`omnic p7-learn`, 6 checks):** M3/M2 0-diff on `pure/m7/learn.omni`, init + checksum + tamper fail-closed, asymmetric reinforcement + safety floor + `cap_mask` gating, `@PACK5=52` summary export, closed-loop `p7-learn → Stage 2.5 adaptive_step → M2 ELF64 → M4 VM (ret=90) + native Linux exec`, and all 5 frozen FNV-1a64 hashes intact.

---

## Post-Round-2 Baselines (`main@19a448d`) & 6th Benchmark (`bench_verifier_engine`)

After Flywheel Round 2 (PR #16, `main@19a448d`), full re-measurement (`p7-measure`, `p7-canary`, `selftest` 1,474 checks on Windows) and expansion of the benchmark corpus from 5 micro-benchmarks to **6 benchmarks** (adding `bench_verifier_engine`, which compiles `pure/verifier_engine.omni` + a minimal 1-function harness through the **default Pure-Omni M3 frontend** `pure/m3/m3.omni_bc`, lowers via M2-multi, and executes `omni_verify_module` on the Pure-Omni M4 VM with P4-5 trace ring active):

| ID | Benchmark | Compiler Path | `ret` | `bc_words` | `elf_bytes` (Compact) | `vm_steps` / `trace_ops` | `arena_bytes` | `hist (alu/mem/ctrl/cr)` | `fe_us` | `emit_us` | `vm_us` |
|---:|---|---|---:|---:|---:|---:|---:|---|---:|---:|---:|
| 0 | `bench_clamp` | Rust strict | `90` | `34` | `591 B` | `3` | `0 B` | `2/0/0/1` | `25` | `3` | `4,460` |
| 1 | `bench_loop_sum` | Rust strict | `5050` | `75` | `656 B` | `809` | `0 B` | `404/0/404/1` | `34` | `6` | `304,097` |
| 2 | `bench_branch_tree` | Rust strict | `70` | `69` | `650 B` | `6` | `0 B` | `2/0/3/1` | `80` | `11` | `10,760` |
| 3 | `bench_multi_call` | Rust strict | `1209` | `73` | `665 B` | `10` | `0 B` | `5/0/0/5` | `80` | `11` | `10,271` |
| 4 | `bench_arena_buf` | Rust strict | `120` | `143` | `810 B` | `271` | `128 B` | `101/33/136/1` | `194` | `13` | `119,826` |
| 5 | **`bench_verifier_engine`** | **Pure-Omni M3** | **`11`** | **`13,426`** | **`32,672 B`** | **`25,692`** | **`3,682 B`** | **`2377/430/254/1035`** | **`12,350,860`** | **`386`** | **`13,097,108`** |

- **Target confirmed by measurements:** On `bench_verifier_engine`, M2 emission is `0.39 ms`, while the **Pure-Omni M3 frontend** takes **`12.35 s`** (`12,350,860 us`) and **M4 VM** execution takes **`13.10 s`** (`13,097,108 us`). Pure-default CLI `omnic compile pure/verifier_engine.omni` takes `12,952 ms` vs `655 ms` on `--rust` (~20× slower). M3 frontend (interpreted on the VM) is confirmed as the next optimization target.
- **Canary proof (`omnic p7-canary`, 6 benchmarks × 5 runs):** `PASS` (7 checks) — Canary A = `+1t (ADOPT, attr=2 [M2_EMITTER])` with exact `1,032 B` (`8 * 129`) reduction on `bench_verifier_engine` and `11,637 B` total production M2 savings (`44,021 B → 32,672 B` on `bench_verifier_engine`); Canary B = `0t (HOLD, attr=0 [NONE])`; 0 histogram drift; all 5 frozen FNV-1a64 hashes intact.
- **Measurement-engine bugs caught by `bench_verifier_engine`:**
  1. `pure/m7/judge.omni::p7_verify_trace_ring` now handles circular P4-5 ring wraparound (`trace_cnt > 4096`) via `start = sel_trit((trace_cnt - 4096) <=> 0, 0, 0, trace_cnt & 4095)` and `slot = (start + i) & 4095`.
  2. `p7_count_m2_zero_imm_loads` updated to exclude `operands[1] == 0` on opcodes lowered via Round 2's `imm8` ALU / `test rax, rax` fast paths (`168 → 129`, exact `1,032 B` match).
- **Closed-Loop Proposer Design:** See `spec/m7/PROPOSER_LOOP_DESIGN.md` (design only — awaiting agreement before implementation).

---

## 7th Benchmark — `bench_m3_frontend` (`main@bfbfbdc` + Step 1)

Expanded the fixed Phase 7 corpus from 6 to **7 benchmarks** by adding `bench_m3_frontend` (`args = [5, 2, -8]`, `ret = 1652`), which exercises the Pure-Omni M3 surface-language frontend path not covered by the 5 micro-benchmarks: `std/` includes (`std/math.omni`, `std/trit.omni`, `std/pack5.omni` via `pure/m3/include.omni`), `egate`, trit consensus/mux/quorum, `@PACK5` radix-3 packing & validation, `match_trit`, `if_pos`, and multi-function `CALL`, compiled through the default Pure-Omni M3 frontend (`pure/m3/m3.omni_bc`).

Linux x86-64 (WSL2) baselines (`./omnic p7-measure` & `./omnic p7-canary`, 7 benchmarks × 5 runs, zero variance on deterministic fields):

| ID | Benchmark | Compiler Path | `ret` | `bc_words` | `elf_bytes` (Compact) | `vm_steps` / `trace_ops` | `arena_bytes` | `hist (alu/mem/ctrl/cr)` | `fe_us` | `emit_us` | `vm_us` |
|---:|---|---|---:|---:|---:|---:|---:|---|---:|---:|---:|
| 0 | `bench_clamp` | Rust strict | `90` | `34` | `493 B` | `3` | `0 B` | `2/0/0/1` | `24` | `1` | `612` |
| 1 | `bench_loop_sum` | Rust strict | `5050` | `75` | `558 B` | `809` | `0 B` | `404/0/404/1` | `30` | `8` | `46,369` |
| 2 | `bench_branch_tree` | Rust strict | `70` | `69` | `552 B` | `6` | `0 B` | `2/0/3/1` | `100` | `12` | `2,042` |
| 3 | `bench_multi_call` | Rust strict | `1209` | `73` | `567 B` | `10` | `0 B` | `5/0/0/5` | `46` | `6` | `2,092` |
| 4 | `bench_arena_buf` | Rust strict | `120` | `143` | `756 B` | `271` | `128 B` | `101/33/136/1` | `54` | `7` | `16,421` |
| 5 | `bench_verifier_engine` | Pure-Omni M3 | `11` | `15,718` | `38,630 B` | `82,071` | `65,562 B` | `2113/103/347/1533` | `13,018` | `337` | `4,660,936` |
| 6 | **`bench_m3_frontend`** | **Pure-Omni M3 + `std/`** | **`1652`** | **`1,204`** | **`2,958 B`** | **`555`** | **`0 B`** | **`304/0/96/155`** | **`9,901`** | **`38`** | **`48,582`** |

- **Canary proof (`./omnic p7-canary`, 7 benchmarks × 5 runs):** `PASS` (7 checks) — Canary A = `+1t (ADOPT, saved=14,761 B, attr=2 [M2_EMITTER])`; Canary B = `0t (HOLD, attr=0 [NONE])`; native Linux x86-64 execution verified across all 7 benchmarks; zero variance on all deterministic fields; all 5 frozen FNV-1a64 hashes and all committed seeds untouched.

---

## Flywheel Round 3 — M3 Frontend Hot-Path Pass (`main@017d0c8` + Step 3)

**Target subsystem:** Pure-Omni M3 frontend (`pure/m3/lex.omni`, `pure/m3/skim.omni`, `pure/m3/inc/lower_body.omni`, `pure/m3/m3_standalone.omni`), built modularly via Step 2b (`mod_parse.omni_bc` + `m3_standalone.omni_bc` linked by Pure-Omni `pure/link/elf_link.omni_bc` into `m3.elf`).

**Proposed changes (0-word diff on all compiled output):**
1. **Guarded span equality (`pure/m3/inc/lower_body.omni`, `pure/m3/skim.omni`):** Guard cross-module `m3s_eq_span` calls behind `if_pos hm <=> 0` in `m3w_var_find` (eliminating eager evaluation on 99.9% hash misses), behind length equality in `m3w_fn_find`, and short-circuit `m3s_eq_span` on length mismatch in `m3s_finalize_sigs`.
2. **Lexer hot-path tightening (`pure/m3/lex.omni`):** Skip `m3l_has_strict_attr` when `strict_arg == 1` and exit early on first `#[strict]` match; fast-path spaces/comments in `m3l_skip_ws`; fast-path identifiers in `m3l_lex_one` (split non-ident dispatch to `m3l_lex_non_id` to stay `<= 12` labels); gate `m3l_check_legacy_trit` on `len ∈ {8, 9} && src[start] == 'T'` in `m3l_lex_ident`.
3. **Call-dispatch short-circuit (`pure/m3/inc/lower_body.omni`):** In `m3w_emit_call_expr`, evaluate `is_mem` before `icls` and `m3w_host_cls` so common memory/core intrinsics skip 12–40 unused hash comparisons.
4. **Dynamic 8× unrolled arena init (`pure/m3/m3_standalone.omni`):** Size the M3 arena dynamically (`min(98304, 20480 + slen(src) * 3)` qwords) and unroll `iset_i64(w, 0)` 8× per iteration.

**Measured Linux x86-64 (WSL2) Before vs. After (5 warm runs, `pure/m7/judge.omni` 3% noise band):**

| Benchmark / Probe | Before (median) | After (median) | Delta | `p7_judge_5runs` Verdict |
|---|---:|---:|---:|---|
| `bench_verifier_engine` (`p7-measure` `fe_us`) | `11,361 us` | `8,124 us` | **−28.5% (1.40× faster)** | **`+1t` (5/5 `+1t`, ADOPT)** |
| `bench_m3_frontend` (`p7-measure` `fe_us`) | `9,961 us` | `8,756 us` | **−12.1% (1.14× faster)** | **`+1t` (5/5 `+1t`, ADOPT)** |
| Direct `m3.elf` `pure/verifier_engine.omni` (52 KB, 9 runs) | `9,829 us` | `6,687 us` | **−32.0% (1.47× faster)** | **`+1t`** |
| Direct `m3.elf` `pure/m3/include.omni` (8 KB, 9 runs) | `2,207 us` | `1,589 us` | **−28.0% (1.39× faster)** | **`+1t`** |
| Direct `m3.elf` 40-B minimal fn (9 runs) | `736 us` | `433 us` | **−41.2% (1.70× faster)** | **`+1t`** |
| 5 micro-benchmarks (`bench_clamp`..`bench_arena_buf`) | unchanged | unchanged | `0%` | **`0t` (HOLD, 0 regressions)** |

- **Verdict:** **`+1t` (ADOPT, attr = `1 [M3_FRONTEND]`)** — measurable improvement on both Pure-Omni M3 benchmarks, zero `-1t` regressions, 0-word diff on all compiled streams (`m3-fixed-point`, `m3-lower`, `m3-modular`, `s3-perf`), `m2-lint` 0 findings, `p7-measure` (10 checks) + `p7-canary` (7 checks) pass.
- **Updated M3 pins (M2, M4, verifier, and all 5 frozen hashes unchanged):**
  - `pure/m3/m3.omni_bc`: `65,541` words (`131,082 B`, `0xE12D02135BCEB128`, `0`-word self-compile fixed point)
  - `pure/m3/mod_parse.omni_bc`: `42,265` words (`84,530 B`, `23,271 w` / `35.5%` headroom below 65,536)
  - `pure/m3/m3_standalone.omni_bc`: `25,515` words (`51,030 B`, `40,021 w` / `61.1%` headroom below 65,536)
  - `m3.elf`: `205,025 B` (`0x177809347B74102D`, linked by Pure-Omni `pure/link/elf_link.omni_bc` `3,582 w`)

---

## Hard Rules (All of Phase 7)

- The verifier, determinism, and capability model are never self-modified.
- Frozen hashes move only through the improvement loop, with the gates as witness.
- No human-facing capabilities reintroduced (`serve`, `repair_cmd`, shell execution stay out of Pure-Omni).
- Do not delete Rust source files as part of Phase 7.
- One step at a time; hard gate at the end of each step, verified on Linux.
