# Step 13 (Part B) — Post-Promotion 5-Cycle Live Flywheel Batch

**Branch:** `feat/step13-batch` (docs-only PR off `main @ 26768d6`)  
**Host / Target:** WSL2 Ubuntu Linux x86-64 (native Pure-Omni stack, zero Rust, native `sys_clock_ns()` timing)  
**Outcome:** **5 live cycles run, 5 ADOPTs (`+1t`, `+1t`, `+1t`, `+1t`, `+1t`), 0 HOLDs, 0 REJECTs.** Every ADOPT stacked cleanly on `flywheel/auto` and pushed to `origin` in the same run.

---

## 1. Setup & Remote Confirmation

Following the merge of PR #56 (`Step 13: Promote flywheel/auto (6 ADOPTs) to main`) into `main @ 26768d6`, `flywheel/auto` was trivially rebased onto `origin/main` and pushed before Cycle 1:

```text
26768d61a78db3de2cfa17c6fb65edf23119aa30	refs/heads/main
26768d61a78db3de2cfa17c6fb65edf23119aa30	refs/heads/flywheel/auto
```

No judge, verifier, proposer, heal/learn, TCB, or pin files were touched. Working tree clean after every cycle.

---

## 2. Fresh `p7-measure` Ranking on `main @ 26768d6`

All non-TCB, non-frozen standalone `.omni` modules measured via `omnic_pure.elf p7-measure` on `main @ 26768d6`, sorted descending by bytecode words:

| Rank | Module | Words (`w`) | Bytes (`B`) | Digest (FNV-1a64) | Median (`us`) | Var | Selected |
| ---: | :--- | ---: | ---: | :--- | ---: | ---: | :--- |
| 1 | `pure/audit/decompile.omni` | 29,227 | 78,132 | `0x04BE14DC2AB2BF78` | 65,717 | 0 | **Cycle 1** |
| 2 | `pure/driver/omnic_driver.omni` | 19,002 | 59,653 | `0xC1D9D50042341E80` | 43,998 | 0 | **Cycle 2** |
| 3 | `pure/audit/diff_summary.omni` | 14,358 | 42,199 | `0x1D1F10E5983A6DDE` | 34,462 | 0 | **Cycle 3** |
| 4 | `pure/harness/core_gates.omni` | 12,784 | 40,156 | `0xC89B5353CFF8E486` | 30,846 | 0 | **Cycle 4** |
| 5 | `pure/harness/audit_view_gate.omni` | 12,029 | 40,478 | `0x236C0D39A229D969` | 29,176 | 0 | **Cycle 5** |
| 6 | `pure/harness/extra_gates.omni` | 10,318 | 32,477 | `0xCAE82BD423126BA2` | 26,665 | 0 | — |
| 7 | `pure/harness/p7_gates.omni` | 10,217 | 33,710 | `0x45540C7CCF58096F` | 25,751 | 0 | — |
| 8 | `pure/harness/m2_lint.omni` | 9,306 | 27,245 | `0xDC65DCA151778850` | 23,457 | 0 | — |
| 9 | `pure/programs/compiler_self_audit.omni` | 3,272 | 6,369 | `0x0D60079918FB367C` | 11,668 | 0 | — |
| 10 | `pure/m5/pure_build_util.omni` | 2,879 | 8,949 | `0x7D701DF6CF99C691` | 9,778 | 0 | — |
| 11 | `pure/programs/pack5_decision_controller.omni` | 1,039 | 2,824 | `0xD67BCC4F52B22152` | 6,236 | 0 | — |
| 12 | `pure/m5/zero_rust_driver.omni` | 714 | 3,574 | `0x843A55FB4842A685` | 5,009 | 0 | — |
| 13 | `pure/programs/kv_ledger_service.omni` | 553 | 1,390 | `0x05EC40933A646500` | 4,771 | 0 | — |

---

## 3. Cycle-by-Cycle Evidence

### Cycle 1 — `pure/audit/decompile.omni` (Rank 1)

- **Diff Summary:** Folded duplicate `if_pos m2u_eq(op, 32578)` trailing `'T'` emission directly into the `op == 32578` branch in `dc_emit_br_fallback`, and hoisted `let jidx: i64 = m2u_sel(m2u_ge(tli, 1), tli - 1, 0);` in `dc_arm_tail_nested`.
- **Baseline vs Candidate:** `29,227 w / 78,132 B / 66,945 us` → `29,161 w / 77,964 B / 66,107 us` (**`-66 w` / `-168 B` / `-838 us`**)
- **Stage Verdicts & Judge Line:**
  ```text
  [PROPOSE-RUN LIVE] stage0=OK stage2=canary:OK stage3=5r/0v base=29227/78132 cand=29161/77964 us=66945/66107 dry=0
  verdict=+1t ADOPT | learn_delta=+2 | learn_updates=1 | heal_action=0
  ```
- **Pushed `flywheel/auto` Confirmation (`git ls-remote origin flywheel/auto`):**
  ```text
  f728ff83283811300dae1e41da3f68a9e9c42683	refs/heads/flywheel/auto
  ```

---

### Cycle 2 — `pure/driver/omnic_driver.omni` (Rank 2)

- **Diff Summary:** Hoisted `let p: i64 = off + i;` in `od_has_macro` (eliminating 5 repeated `off + i` additions), `let sp: i64 = src_off + p;` in `od_expand_includes_pass` (eliminating 8 repeated `src_off + p` additions), and `let ba: i64 = su_band_a(); let bb: i64 = su_band_b();` out of the byte-widening loop in `od_run_m3_and_verify`.
- **Baseline vs Candidate:** `19,002 w / 59,653 B / 45,010 us` → `18,911 w / 59,380 B / 44,448 us` (**`-91 w` / `-273 B` / `-562 us`**)
- **Stage Verdicts & Judge Line:**
  ```text
  [PROPOSE-RUN LIVE] stage0=OK stage2=canary:OK stage3=5r/0v base=19002/59653 cand=18911/59380 us=45010/44448 dry=0
  verdict=+1t ADOPT | learn_delta=+2 | learn_updates=1 | heal_action=0
  ```
- **Pushed `flywheel/auto` Confirmation (`git ls-remote origin flywheel/auto`):**
  ```text
  997500eca1fb70c8fdf46d6c2e2a22cd2d7ca148	refs/heads/flywheel/auto
  ```

---

### Cycle 3 — `pure/audit/diff_summary.omni` (Rank 3)

- **Diff Summary:** Hoisted `let lim: i64 = nwords - 1;` in `ad_scan_module` and replaced 1-byte `su_out4(..., 79, 0, 0, 0)` with `su_out_u8(..., 79)` in `ad_emit_cap_hi`.
- **Baseline vs Candidate:** `14,358 w / 42,199 B / 35,845 us` → `14,344 w / 42,175 B / 35,265 us` (**`-14 w` / `-24 B` / `-580 us`**)
- **Stage Verdicts & Judge Line:**
  ```text
  [PROPOSE-RUN LIVE] stage0=OK stage2=canary:OK stage3=5r/0v base=14358/42199 cand=14344/42175 us=35845/35265 dry=0
  verdict=+1t ADOPT | learn_delta=+2 | learn_updates=1 | heal_action=0
  ```
- **Pushed `flywheel/auto` Confirmation (`git ls-remote origin flywheel/auto`):**
  ```text
  5a228b59a8b0dcec51bc23f6fcdfc9a23c636bc6	refs/heads/flywheel/auto
  ```

---

### Cycle 4 — `pure/harness/core_gates.omni` (Rank 4)

- **Diff Summary:** Hoisted `let bb: i64 = su_band_b();` and `let sz_ok: i64 = m2u_eq(nb, want_sz);` in `cg_check_pin`, and hoisted `let ba: i64 = su_band_a(); let bb: i64 = su_band_b();` out of the byte-widening loops in `cg_gate_p2a` and `cg_gate_ver_and_mac`.
- **Baseline vs Candidate:** `12,784 w / 40,156 B / 31,633 us` → `12,740 w / 40,064 B / 31,088 us` (**`-44 w` / `-92 B` / `-545 us`**)
- **Stage Verdicts & Judge Line:**
  ```text
  [PROPOSE-RUN LIVE] stage0=OK stage2=canary:OK stage3=5r/0v base=12784/40156 cand=12740/40064 us=31633/31088 dry=0
  verdict=+1t ADOPT | learn_delta=+2 | learn_updates=1 | heal_action=0
  ```
- **Pushed `flywheel/auto` Confirmation (`git ls-remote origin flywheel/auto`):**
  ```text
  7fbbc8030a55347f5bbef1048b0daef165a2bd69	refs/heads/flywheel/auto
  ```

---

### Cycle 5 — `pure/harness/audit_view_gate.omni` (Rank 5)

- **Diff Summary:** Hoisted `let ba: i64 = su_band_a(); let ba2: i64 = ba + 125000;` in `av_check_verifier_standalone` (eliminating 6 repeated `su_band_a()` calls and 1 repeated `+ 125000`), and reused existing `b = su_band_a()` across 4 calls in `av_check_b6`.
- **Baseline vs Candidate:** `12,029 w / 40,478 B / 30,201 us` → `11,950 w / 40,306 B / 29,865 us` (**`-79 w` / `-172 B` / `-336 us`**)
- **Stage Verdicts & Judge Line:**
  ```text
  [PROPOSE-RUN LIVE] stage0=OK stage2=canary:OK stage3=5r/0v base=12029/40478 cand=11950/40306 us=30201/29865 dry=0
  verdict=+1t ADOPT | learn_delta=+2 | learn_updates=1 | heal_action=0
  ```
- **Pushed `flywheel/auto` Confirmation (`git ls-remote origin flywheel/auto`):**
  ```text
  adea917c7e3a82a3ead95f48771d4e23e73ce53b	refs/heads/flywheel/auto
  ```

---

## 4. Batch Cumulative `p7-measure` Table (`flywheel/auto @ adea917` vs `main @ 26768d6`)

Measured head-to-head at batch end via `omnic_pure.elf p7-measure`:

| Module | `main @ 26768d6` (`w` / `B` / `us`) | `flywheel/auto @ adea917` (`w` / `B` / `us`) | Delta (`w` / `B` / `us`) | `flywheel/auto` SHA |
| :--- | :--- | :--- | :--- | :--- |
| `pure/audit/decompile.omni` | `29,227 w` / `78,132 B` / `69,366 us` | `29,161 w` / `77,964 B` / `67,721 us` | **`-66 w` / `-168 B` / `-1,645 us`** | `f728ff8` |
| `pure/driver/omnic_driver.omni` | `19,002 w` / `59,653 B` / `46,303 us` | `18,911 w` / `59,380 B` / `44,274 us` | **`-91 w` / `-273 B` / `-2,029 us`** | `997500e` |
| `pure/audit/diff_summary.omni` | `14,358 w` / `42,199 B` / `35,597 us` | `14,344 w` / `42,175 B` / `34,787 us` | **`-14 w` / `-24 B` / `-810 us`** | `5a228b5` |
| `pure/harness/core_gates.omni` | `12,784 w` / `40,156 B` / `31,225 us` | `12,740 w` / `40,064 B` / `31,538 us` | **`-44 w` / `-92 B` / `+313 us`** | `7fbbc80` |
| `pure/harness/audit_view_gate.omni` | `12,029 w` / `40,478 B` / `30,168 us` | `11,950 w` / `40,306 B` / `30,499 us` | **`-79 w` / `-172 B` / `+331 us`** | `adea917` |
| **Step 13 Part B Total (5 ADOPTs)** | `87,400 w` / `260,618 B` / `212,659 us` | `87,106 w` / `259,889 B` / `208,819 us` | **`-294 w` / `-729 B` / `-3,840 us`** | `adea917` |

- **New `flywheel/auto` remote head (`git ls-remote origin flywheel/auto`):**
  ```text
  adea917c7e3a82a3ead95f48771d4e23e73ce53b	refs/heads/flywheel/auto
  ```

---

## 5. Close-Out Gates (`feat/step13-batch`, WSL2 Ubuntu Linux x86-64)

- `bash scripts/build_pure_driver.sh`: **PASS**
- `bash scripts/zero_rust_rebuild.sh`: **PASS**
- `bash scripts/verify_step7_driver.sh`: **38/38 PASS** (`FAIL=0`)
- `bash scripts/verify_step8_harness.sh`: **52/52 PASS** (`FAIL=0`)
- `./build/pure-bin/omnic_pure.elf selftest`: **PASS**
