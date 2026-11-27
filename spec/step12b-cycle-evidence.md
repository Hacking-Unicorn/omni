# Step 12b — Include-Base Fix & Full 5-Cycle Live Flywheel Batch

**Branch:** `feat/step12b-include-base` (off `main @ bfc7b05`)  
**Host / Target:** WSL2 Ubuntu Linux x86-64 (native Pure-Omni stack, zero Rust, native `sys_clock_ns()` timing)  
**Outcome:** **5 live cycles run, 5 ADOPTs (`+1t`, `+1t`, `+1t`, `+1t`, `+1t`), 0 HOLDs, 0 REJECTs.** Every ADOPT stacked cleanly on `flywheel/auto` and pushed to `origin` in the same run.

---

## 1. Part A & Part B Summary (Backlog Item 8 Resolution)

- **Driver (`pure/driver/omnic_driver.omni`):**
  - Added `od_compile_omni_base(arena, in_off, in_len, out_bc_cstr, ib_off, ib_len)` and optional `--include-base <dir>` flag on `omnic_pure.elf compile <in.omni> <out.omni_bc> [--include-base <dir>]`.
  - When `--include-base` is provided, relative `include` directives resolve against that directory (normalizing a missing trailing `/` in `od_read_include`); when omitted, `od_compile_omni` resolves against the source file's own directory exactly as before. Missing includes remain fail-closed (`rc=1`).
- **Proposer (`pure/harness/proposer_live.omni`):**
  - Added `plv_parent_dir(tgt_src)` to derive the include-base from `--target` (e.g. `pure/audit/` for `pure/audit/decompile.omni`, or `<repo>/pure/audit/` under `--repo`), passing `--include-base "<dir>"` on candidate compilation in `plv_measure_5` while leaving baseline in-tree compilation untouched and keeping candidate files strictly out-of-tree.
- **Suite coverage (`scripts/verify_step8_harness.sh` — 52/52 PASS):**
  1. Included-module dry-run (unchanged out-of-tree copy of `pure/audit/decompile.omni`): `base=cand=29283/78306`, `0t HOLD`.
  2. Included-module real diff merits verdict: real Stage 3 numbers (`cand > 0`), real judge verdict.
  3. Negative control (broken `include` in out-of-tree candidate): fail-closed `cand=-1/-1`, `-1t REJECT`, non-zero exit.

---

## 2. Batch Setup & Standing Rules Record

1. **Sanctioned pre-batch `flywheel/auto` rebase (Standing Rule 1 & Constraints):**
   - Rebased the Step 11 ADOPT commit (`pure/m5/zero_rust_driver.omni`) cleanly onto `main @ bfc7b05` with zero conflicts and pushed once before Cycle 1:
   - Remote confirmation (`git ls-remote origin flywheel/auto`):
     ```text
     8f2c42bd587aa27bed471bd94227a7971863a53a	refs/heads/flywheel/auto
     ```
2. **Preconditions (all PASS on `feat/step12b-include-base` before Cycle 1):**
   - `bash scripts/zero_rust_rebuild.sh`: **PASS**
   - `bash scripts/verify_step7_driver.sh`: **38/38 PASS** (`FAIL=0`)
   - `bash scripts/verify_step8_harness.sh`: **52/52 PASS** (`FAIL=0`, including all 3 Part B checks)
   - `./build/pure-bin/omnic_pure.elf selftest`: **PASS**
3. **TCB & Pins:** Untouched (Standing Rule 3). No pin, frozen hash, or `bootstrap/MANIFEST.md` entry moved. Working tree clean after every cycle.

---

## 3. Cycle-by-Cycle Evidence

### Cycle 1 — `pure/audit/decompile.omni`

- **Target:** `pure/audit/decompile.omni` (Rank 1 by `p7-measure`)
- **Candidate Diff:**
  ```diff
  @@ -348,7 +348,6 @@
   let r: bytes = if_pos ld_u64(w,176) <=> 0 {
  -let ni: i64 = ld_u64(w,184);
   w
   } else {
  @@ -447,9 +446,10 @@
  -let pt: i64 = dc_u16(w,pos + 3 + pk * 3);
  -let pty: i64 = dc_u16(w,pos + 3 + pk * 3 + 1);
  -let ptc: i64 = dc_u16(w,pos + 3 + pk * 3 + 2);
  +let pbase: i64 = pos + 3 + pk * 3;
  +let pt: i64 = dc_u16(w,pbase);
  +let pty: i64 = dc_u16(w,pbase + 1);
  +let ptc: i64 = dc_u16(w,pbase + 2);
  ```
- **Stage Verdicts:** Stage 0 `OK`, Stage 2 `canary:OK`, Stage 3 `5r/0v`, Stage 4 **`+1t ADOPT`** (`rc=0`).
- **Stage 3 Measurements & Judge Inputs (`/tmp/omni_plv_j` + `p7-measure`):**
  - Baseline: `29,283 w` / `78,306 B` (`p7-measure` `65,882 us`)
  - Candidate: `29,227 w` / `78,132 B` (`p7-measure` `65,738 us`)
  - Delta: **`-56 w` / `-174 B`** (`-144 us` head-to-head)
  - Judge: `gates_ok=1`, `err_code=0`, `verdict=+1t ADOPT | learn_delta=+2 | learn_updates=1 | heal_action=0`
- **`flywheel/auto` Commit & Remote Confirmation:**
  ```text
  5c27b8438be84ab2b78a6f93531122bdf601deaa	refs/heads/flywheel/auto
  ```

---

### Cycle 2 — `pure/audit/diff_summary.omni`

- **Target:** `pure/audit/diff_summary.omni` (Rank 2 by `p7-measure`)
- **Candidate Diff:**
  ```diff
  @@ -71,8 +71,9 @@
   fn ad_imm_val(arena: bytes, base: i64, pos: i64) -> i64 {
       let t: i64 = ad_u16(arena, base, pos);
  -    let v16: i64 = m2u_sext16(ad_u16(arena, base, pos + 1));
  -    let v32: i64 = m2u_sext32((ad_u16(arena, base, pos + 1) << 16) | ad_u16(arena, base, pos + 2));
  +    let w1: i64 = ad_u16(arena, base, pos + 1);
  +    let v16: i64 = m2u_sext16(w1);
  +    let v32: i64 = m2u_sext32((w1 << 16) | ad_u16(arena, base, pos + 2));
       return m2u_sel(m2u_eq(t, 32485), v16, m2u_sel(m2u_eq(t, 32486), v32, 0));
   }
  ```
- **Stage Verdicts:** Stage 0 `OK`, Stage 2 `canary:OK`, Stage 3 `5r/0v`, Stage 4 **`+1t ADOPT`** (`rc=0`).
- **Stage 3 Measurements & Raw Loop Output:**
  ```text
  [PROPOSE-RUN LIVE] stage0=OK stage2=canary:OK stage3=5r/0v base=14376/42242 cand=14358/42199 us=34893/34030 dry=0
  verdict=+1t ADOPT | learn_delta=+2 | learn_updates=2 | heal_action=0
  ```
  - Delta: **`-18 w` / `-43 B` / `-863 us`** (in-cycle native timing)
- **`flywheel/auto` Commit & Remote Confirmation:**
  ```text
  ddbb6ac83403910431b35f82e7f09434013ef0af	refs/heads/flywheel/auto
  ```

---

### Cycle 3 — `pure/harness/audit_view_gate.omni`

- **Target:** `pure/harness/audit_view_gate.omni` (Rank 3 by `p7-measure`)
- **Candidate Diff:** Hoist `let p: i64 = off + i;` in `av_has4` (eliminating 3 repeated `off + i` additions per loop iteration) and hoist `let ba: i64 = su_band_a(); let ba2: i64 = ba + 65536;` in `av_roundtrip_file` (eliminating 9 repeated `su_band_a()` calls and 3 repeated `+ 65536` additions).
- **Stage Verdicts:** Stage 0 `OK`, Stage 2 `canary:OK`, Stage 3 `5r/0v`, Stage 4 **`+1t ADOPT`** (`rc=0`).
- **Stage 3 Measurements & Raw Loop Output:**
  ```text
  [PROPOSE-RUN LIVE] stage0=OK stage2=canary:OK stage3=5r/0v base=12168/40802 cand=12029/40478 us=30070/29859 dry=0
  verdict=+1t ADOPT | learn_delta=+2 | learn_updates=1 | heal_action=0
  ```
  - Delta: **`-139 w` / `-324 B` / `-211 us`** (in-cycle native timing)
- **`flywheel/auto` Commit & Remote Confirmation:**
  ```text
  f5512c906baa56142a3fd83eb665ed9841e2f4c9	refs/heads/flywheel/auto
  ```

---

### Cycle 4 — `pure/harness/extra_gates.omni`

- **Target:** `pure/harness/extra_gates.omni` (Rank 4 by `p7-measure`)
- **Candidate Diff:** Hoist `let bb: i64 = su_band_b();` and `let pbase: i64 = bb + phoff;` in `eg_check_gnu_stack` (eliminating 4 repeated `su_band_b()` calls and loop-invariant `su_band_b() + phoff`).
- **Stage Verdicts:** Stage 0 `OK`, Stage 2 `canary:OK`, Stage 3 `5r/0v`, Stage 4 **`+1t ADOPT`** (`rc=0`).
- **Stage 3 Measurements & Raw Loop Output:**
  ```text
  [PROPOSE-RUN LIVE] stage0=OK stage2=canary:OK stage3=5r/0v base=10353/32552 cand=10318/32477 us=26176/26143 dry=0
  verdict=+1t ADOPT | learn_delta=+2 | learn_updates=1 | heal_action=0
  ```
  - Delta: **`-35 w` / `-75 B` / `-33 us`** (in-cycle native timing)
- **`flywheel/auto` Commit & Remote Confirmation:**
  ```text
  87511df49591d4771d57911d5eb8e882f666b2a3	refs/heads/flywheel/auto
  ```

---

### Cycle 5 — `pure/harness/m2_lint.omni`

- **Target:** `pure/harness/m2_lint.omni` (Rank 5 by `p7-measure`)
- **Candidate Diff:**
  ```diff
  @@ -54,8 +54,9 @@
  -    let v16: i64 = m2u_sext16(ml_u16(w, pos + 1));
  -    let v32: i64 = m2u_sext32((ml_u16(w, pos + 1) << 16) | ml_u16(w, pos + 2));
  +    let w1: i64 = ml_u16(w, pos + 1);
  +    let v16: i64 = m2u_sext16(w1);
  +    let v32: i64 = m2u_sext32((w1 << 16) | ml_u16(w, pos + 2));
  @@ -124,7 +125,7 @@
  -    let is_p5: i64 = m2u_eq(op, 32560) + m2u_eq(op, 32561) + m2u_eq(op, 32564) + m2u_eq(op, 32565) + m2u_eq(op, 32575);
  +    let is_p5: i64 = m2u_in(op, 32560, 32561) + m2u_in(op, 32564, 32565) + m2u_eq(op, 32575);
  @@ -140,7 +141,7 @@
  -        rules = rules | ((m2u_eq(op, 32581) * is_b) << 0);
  +        rules = rules | (m2u_eq(op, 32581) * is_b);
  ```
- **Stage Verdicts:** Stage 0 `OK`, Stage 2 `canary:OK`, Stage 3 `5r/0v`, Stage 4 **`+1t ADOPT`** (`rc=0`).
- **Stage 3 Measurements & Raw Loop Output:**
  ```text
  [PROPOSE-RUN LIVE] stage0=OK stage2=canary:OK stage3=5r/0v base=9361/27445 cand=9306/27245 us=23506/23358 dry=0
  verdict=+1t ADOPT | learn_delta=+2 | learn_updates=1 | heal_action=0
  ```
  - Delta: **`-55 w` / `-200 B` / `-148 us`** (in-cycle native timing)
- **`flywheel/auto` Commit & Remote Confirmation:**
  ```text
  b13b2119309f7a058f8ae1e2ca71ce61f9036f89	refs/heads/flywheel/auto
  ```

---

## 4. Cumulative `p7-measure` Effect (`flywheel/auto` vs `main`)

Measured head-to-head at batch end by running `omnic_pure.elf p7-measure` on `flywheel/auto @ b13b211` vs `main @ bfc7b05`:

| Module | `main @ bfc7b05` (`w` / `B` / `us`) | `flywheel/auto @ b13b211` (`w` / `B` / `us`) | Delta (`w` / `B` / `us`) | `flywheel/auto` SHA |
| :--- | :--- | :--- | :--- | :--- |
| `pure/audit/decompile.omni` | `29,283 w` / `78,306 B` / `65,882 us` | `29,227 w` / `78,132 B` / `65,738 us` | **`-56 w` / `-174 B` / `-144 us`** | `5c27b84` (Cycle 1) |
| `pure/audit/diff_summary.omni` | `14,376 w` / `42,242 B` / `34,527 us` | `14,358 w` / `42,199 B` / `34,740 us` | **`-18 w` / `-43 B` / `+213 us`** | `ddbb6ac` (Cycle 2) |
| `pure/harness/audit_view_gate.omni` | `12,168 w` / `40,802 B` / `30,904 us` | `12,029 w` / `40,478 B` / `29,960 us` | **`-139 w` / `-324 B` / `-944 us`** | `f5512c9` (Cycle 3) |
| `pure/harness/extra_gates.omni` | `10,353 w` / `32,552 B` / `25,950 us` | `10,318 w` / `32,477 B` / `25,919 us` | **`-35 w` / `-75 B` / `-31 us`** | `87511df` (Cycle 4) |
| `pure/harness/m2_lint.omni` | `9,361 w` / `27,445 B` / `23,763 us` | `9,306 w` / `27,245 B` / `24,069 us` | **`-55 w` / `-200 B` / `+306 us`** | `b13b211` (Cycle 5) |
| `pure/m5/zero_rust_driver.omni` | `733 w` / `3,609 B` / `4,984 us` | `714 w` / `3,574 B` / `4,869 us` | **`-19 w` / `-35 B` / `-115 us`** | `8f2c42b` (Step 11 rebased) |
| **Step 12b Batch Total (5 ADOPTs)** | `75,541 w` / `221,347 B` / `181,026 us` | `75,238 w` / `220,531 B` / `180,426 us` | **`-303 w` / `-816 B` / `-600 us`** | 5 commits |
| **Cumulative Total (`flywheel/auto` vs `main`, 6 ADOPTs)** | `76,274 w` / `224,956 B` / `186,010 us` | `75,952 w` / `224,105 B` / `185,295 us` | **`-322 w` / `-851 B` / `-715 us`** | `b13b211` |

- **Final remote `flywheel/auto` (`git ls-remote origin flywheel/auto`):**
  ```text
  b13b2119309f7a058f8ae1e2ca71ce61f9036f89	refs/heads/flywheel/auto
  ```

---

## 5. Post-Batch Close-Out Gates

All four close-out gates verified on `feat/step12b-include-base` (WSL2 Ubuntu Linux x86-64):

- `bash scripts/zero_rust_rebuild.sh`: **PASS**
- `bash scripts/verify_step7_driver.sh`: **38/38 PASS** (`FAIL=0`)
- `bash scripts/verify_step8_harness.sh`: **52/52 PASS** (`FAIL=0`)
- `./build/pure-bin/omnic_pure.elf selftest`: **PASS**
