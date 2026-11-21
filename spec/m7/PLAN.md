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

## Hard Rules (All of Phase 7)

- The verifier, determinism, and capability model are never self-modified.
- Frozen hashes move only through the improvement loop, with the gates as witness.
- No human-facing capabilities reintroduced (`serve`, `repair_cmd`, shell execution stay out of Pure-Omni).
- Do not delete Rust source files as part of Phase 7.
- One step at a time; hard gate at the end of each step, verified on Linux.
