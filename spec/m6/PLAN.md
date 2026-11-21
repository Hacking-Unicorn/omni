# Omni Phase 6 — Consolidation, Tests, CI, Docs & Merge-Ready Checklist

**Status:** Phase 6 complete on `isa/multibyte-loads` (2026-11-20). Zero new features, zero new opcodes, zero frozen-hash moves. All M1–M3, Phase 4 (`P4-1..P4-5`), and Phase 5 (`P5-1`, `P5-2`) gates are implemented, wired into `.github/workflows/ci.yml`, and verified clean across both the standalone monolith (`omni_rust_bootstrap.rs`) and the 8-crate Cargo workspace (`omni-workspace/`).

---

## 1. Phase 6 Nits & Test Hygiene Closeout

| Item | Resolution | Verification |
|---|---|---|
| `--help` / `-h` / `help` / `--version` CLI hang | Added explicit `--help` / `-h` / `help` and `--version` / `-V` / `version` arms at the top of `fn main()` in `omni_rust_bootstrap.rs` and `omni-workspace/crates/omni-cli/src/main.rs` (plus updated `run_m2_cli` usage string). Previously, `--help` fell through `_ => {}` into the full 7-step bootstrap & GRPO training suite. | `./omnic --help` and `cargo run --release -p omni-cli -- --help` print usage and exit `0` in `< 50 ms`. |
| Cargo workspace `unused manifest key: workspace.package.license` | Added `license.workspace = true` to all 8 workspace crates (`omni-isa`, `omni-ir`, `omni-verifier`, `omni-vm`, `omni-assembler`, `omni-lang`, `omni-aot`, `omni-cli`). | `cargo check/build/test --release --workspace` produces `0` warnings. |
| `spec/acceptance/goldens/neg_circular_include.omni` (42 files on disk vs 41 in single-file manifest) | Recorded explicitly under `"deferred_multi_file_goldens"` in `spec/acceptance/acceptance_manifest.json` (and asserted in `run_acceptance_suite_selftest()`). `neg_circular_include.omni` requires filesystem/VFS `include` path resolution rather than single-string compilation, and is actively tested by selftest `6g` (`resolve_omni_surface_includes`), `spec/m3/fixtures/neg_circular_include.omni` in `spec/m3/corpus.tsv`, and `M2_GOLDEN_SOURCES`. | `omnic acceptance-diff` (41/41 PASS) + `selftest` `6g` + `fe-dump --lock`. |
| Exact selftest counts & `.omni` line counts in `README.md` | Verified every `.omni` line count on disk (`9,847` lines of real iterative compiler/verifier/VM modules across `pure/m3/`, `pure/m2/`, `pure/m4/`, `pure/m5/`, `pure/isa/`, `pure/verifier_engine.omni`, `pure/host_kv.omni`; `569` lines in `std/*` + `pure/programs/*`; `402` lines in frozen legacy kernels) and recorded exact selftest counts (`1,385` Windows / `1,501` Linux on monolith; `666` Windows / `706` Linux on 8-crate workspace). | Matches `wc -l` / `Measure-Object -Line` and live `selftest` outputs. |

---

## 2. Frozen Regression Hashes & Deterministic Self-Host Seeds

### 2.1 Five Frozen Regression Hashes (Sacred Invariant — Never Moved)

| Artifact | Size | FNV-1a64 | Gate |
|---|---:|---|---|
| Phase D kernel `.omni_bc` | `159` words (`318` B) | `0x23B8D17257CCA9C4` | `pure-bootstrap`, `m2-frozen`, `selftest` (`13BF`) |
| Phase D kernel `.elf` | `824` B | `0xE221085777B8AC47` | `pure-bootstrap`, `m2-frozen`, `selftest` (`13BF`) |
| 18-module core suite | `14,559` words (`29,118` B) | `0x78462AE4C81DF408` | `pure-bootstrap`, `m2-frozen`, `selftest` (`13BF`) |
| Stage-1 bytecode (`omni_self_compiler.omni_bc`) | `19,731` words (`39,462` B) | `0x677F57AE71D28FED` | `fixed-point`, `m2-compile`, `selftest` (`13BF`) |
| Stage-1 ELF (`stage1.elf == stage2.elf`) | `106,242` B | `0xEAFC81F46B482FF8` | `fixed-point`, `m2-compile`, `m2-frozen`, `selftest` (`13BF`) |

### 2.2 Three Committed Pure-Omni Self-Hosting Seeds (`0`-Byte / `0`-Word Diff vs Rust)

| Seed | Words (Bytes) | Seed FNV-1a64 | Emitted Native x86-64 ELF64 | Gate |
|---|---:|---|---|---|
| `pure/m2/m2.omni_bc` (M2 multi-fn x86-64 backend) | `32,796` words (`65,592` B) | `0x6B1D4846DF0CA61A` | `137,246` B (`0xF05F0BFAD0E2DCCC`, `0`-B diff; `S1 == S2 == S3 == V` on Linux x86-64) | `m2-fixed-point` |
| `pure/m3/m3.omni_bc` (M3 surface `.omni` → `.omni_bc` frontend) | `59,474` words (`118,948` B) | `0x0668919FB4378D88` | Compiles `pure/m3/*` with `0`-word diff & chains `pure/verifier_engine.omni` → M3 → M2 → `41,285`-B ELF64 (`0xCFF611B4DDBD80DD`, `0`-B diff) | `m3-fixed-point` |
| `pure/m4/m4.omni_bc` (M4 Pure-Omni token-stream VM) | `25,026` words (`50,052` B) | `0x44FB7C039E69843C` | `109,740` B (`0xA7A6CA9D72BBD84C`, `0`-B diff via Pure-Omni M2; compiled with `0`-word diff via Pure-Omni M3) | `p5-selfhost` |

---

## 3. Complete Gate Matrix (Wired in `.github/workflows/ci.yml`)

| Gate Command | Expected Result |
|---|---|
| `rustc -O omni_rust_bootstrap.rs -o omni` | `0` warnings, `0` errors |
| `./omni --help` | Prints usage and exits `0` immediately |
| `./omni fixed-point` | `19,731` words (`0x677F57AE71D28FED`), `106,242` B (`0xEAFC81F46B482FF8`), `diff=0 bytes` |
| `./omni pure-bootstrap` | Phase D `159` words / `824` B + 18-module suite `14,559` words (`0x78462AE4C81DF408`) |
| `./omni m2-lint` | `15/15` registered `pure/m2`, `pure/m3`, `pure/m4`, `pure/m5`, `pure/isa` modules, `0` gated findings |
| `./omni m2-elf` | `4` M2-11 arena/ELF64 checks PASS |
| `./omni m2-lock` | `0` diff vs `spec/m2/single_mode_lock.tsv` (`216` single-mode lock rows) |
| `./omni m2-compile` | `4` checks PASS (incl. `19,731`-word Stage-1 anchor → `106,242` B, `0`-byte diff) |
| `./omni m2-multi` | `8` Rust M2-09 multi-function x86-64 ELF64 checks PASS |
| `./omni m2-multi-pure` | `6` Pure-Omni multi-function ELF64 checks PASS (`0`-byte diff vs Rust) |
| `./omni m2-wasm-multi` | `7` M2-10 multi-function Wasm v1 + `CALL` + shared arena + depth-64 checks PASS |
| `./omni m2-verify` | `10` M2-14 Pure-Omni multi-AOT module verifier + Kahn cycle + L1–L12 label checks PASS |
| `./omni m2-fixed-point` | `32,796` words (`0x6B1D4846DF0CA61A`) → `137,246` B (`0xF05F0BFAD0E2DCCC`), `S1 == S2 == S3 == V` on Linux |
| `./omni m3-gate` | `6/6` `pure/m3/*.omni` modules PASS (`m2-lint` + M2-08 call-graph depth check) |
| `./omni m3-lex` | `47` files lexed in Pure-Omni (`100%` parity vs Rust) |
| `./omni m3-skim` | `50` files skimmed in Pure-Omni (`100%` parity vs Rust) |
| `./omni m3-expr` | `24` expression ASTs parsed in Pure-Omni (`100%` parity vs Rust) |
| `./omni m3-stmt` | `14` full-module ASTs parsed in Pure-Omni (`100%` parity vs Rust) |
| `./omni m3-lower` | `20` modules lowered to `.omni_bc` in Pure-Omni (`0`-word diff vs Rust) |
| `./omni m3-lockstep` | `6` `lockstep_shadow_frontend` checks PASS (`OMNI_M3_LOCKSTEP=1` parity + mismatch hard-fail) |
| `./omni m3-4backend` | `3` multi-function programs verified across `VM == Wasm-multi == Rust-x86-multi == Pure-Omni-M2-ELF64` |
| `./omni m3-fixed-point` | `59,474` words (`0x0668919FB4378D88`), `0`-word diff + E2E `verifier_engine.omni` → `41,285`-B ELF64 (`0xCFF611B4DDBD80DD`) |
| `./omni p4-vm` | `11` Pure-Omni token-stream VM (`pure/m4/vm.omni`) == Rust `OmniBinaryVm` checks PASS (incl. P4-5 trace ring) |
| `./omni p4-shim` | `3` minimal 3-op thin host shim (`read_bytes`, `write_bytes`, `spawn`) checks PASS |
| `./omni p4-wasm` | `3` Wasm-multi host VM checks PASS (`pure/m4/vm.omni` → `62,213`-B Wasm v1 `0x557AA8121383D047`) |
| `./omni p4-exec` | Executes raw `.omni_bc` directly on Pure-Omni M4 VM via `OmniThinShim` with `--trace` |
| `./omni p5-sys` | `3` checks PASS (`177`-B zero-shim Linux x86-64 ELF64 `0x4C5943AD64761FA1`, 3 raw `0F 05` syscalls, `native=VERIFIED` on Linux) |
| `./omni p5-selfhost` | `25,026` words (`0x44FB7C039E69843C`), `M3_vm(pure/m4/*) == seed` (`0`-word diff), `M2_vm(m4.omni_bc) == 109,740-B ELF64` (`0xA7A6CA9D72BBD84C`, `0`-byte diff), zero-Rust M3→M4 & M3→M2 loop PASS |
| `./omni backend-diff --suite` | `181` graded: `132` pass, `49` xfail, `0` fail, `0` xpass |
| `./omni selftest` | `1,385` checks PASS on Windows / `1,501` checks PASS on Linux x86-64 |
| `./omni pure-omni-diff` | `49/49` Pure-Omni differential checks PASS |
| `./omni acceptance-diff` | `41/41` executable acceptance goldens PASS |
| `cargo check --release -p omni-aot` | `0` warnings, `0` errors |
| `cargo run --release -p omni-cli -- selftest` | `666` checks PASS on Windows / `706` checks PASS on Linux x86-64 |
| `cargo test --release --workspace` | `13/13` workspace tests PASS (`0` warnings) |

---

## 4. Merge-Ready Checklist (`isa/multibyte-loads` → `main`)

> **Note:** Phase 6 prepares the branch for merge. Do **not** merge automatically; execute the steps below when ready to land on `main` (`b3d42dd`).

1. **Pre-Merge Branch Audit**
   - [x] `origin/main` is untouched at `b3d42dd`.
   - [x] `origin/isa/multibyte-loads` is a clean fast-forward descendant of `main@b3d42dd` (`git merge-base main origin/isa/multibyte-loads == b3d42dd`).
   - [x] `git status` is clean with zero untracked build artifacts (`omnic`, `*.elf`, `*.wasm`, `omni_sft_corpus.jsonl`, `tokenizer.json`, `*.omni_holo`).
   - [x] `Cargo.lock` contains only the 8 internal workspace packages (`zero` external crates).
   - [x] All 5 frozen FNV-1a64 hashes (`0x23B8D17257CCA9C4`, `0xE221085777B8AC47`, `0x78462AE4C81DF408`, `0x677F57AE71D28FED`, `0xEAFC81F46B482FF8`) and all 3 committed Pure-Omni seeds (`pure/m2/m2.omni_bc`, `pure/m3/m3.omni_bc`, `pure/m4/m4.omni_bc`) are verified bit-exact.

2. **Merge Procedure (PR with Merge Commit — No Squash, No Rebase)**
   - [ ] Open a Pull Request from `isa/multibyte-loads` → `main`.
   - [ ] Wait for GitHub Actions CI (`.github/workflows/ci.yml` on `ubuntu-latest`) to pass all monolith and 8-crate workspace gates.
   - [ ] Merge using **Create a merge commit** (`git merge --no-ff isa/multibyte-loads`) so the full M1/M2/M3/P4/P5/P6 commit history and audit trail recorded in `HANDOFF.md` §14 remain intact. **Do not squash or rebase.**

3. **Post-Merge Verification on `main`**
   - [ ] Checkout `main` and pull the merge commit.
   - [ ] Run `rustc -O omni_rust_bootstrap.rs -o omni && ./omni selftest && ./omni m2-fixed-point && ./omni m3-fixed-point && ./omni p5-selfhost && ./omni p5-sys`.
   - [ ] Run `cargo test --release --manifest-path omni-workspace/Cargo.toml --workspace`.
   - [ ] Record the final merge commit SHA in `HANDOFF.md` §14 on `origin/docs/handoff`.
