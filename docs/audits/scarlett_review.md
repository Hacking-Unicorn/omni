# Scarlett review of the first-pass README / port-map audit

**Repo:** `github.com/Hacking-Unicorn/omni` · **Commit:** `8160842` (`git -C /workspace/audit-readme rev-parse HEAD` → `81608426c3fb7cf29e99b104b6779227acc205a2`, so the HEAD matches and there is no mismatch).
**Reviewed:** `/workspace/audits/readme_honesty.md`, `port_map.md`, `README_draft.md`.
**Method:** I read every cited `file:line` in `/workspace/audit-readme`, without modifying it. I made my own working clone at `/workspace/audit-scarlett-review`, cloned locally from the read-only clone and checked out at `8160842`, and used it to **build and run** these on rustc 1.85.1 / Linux x86-64: monolith `selftest`, `pure-bootstrap`, `pure-omni-diff`, `acceptance-diff`; workspace `selftest`, `pure-bootstrap`, `pure-omni-diff`, `acceptance-diff`, `cargo test --workspace`; plus README quick-start smoke runs. The cargo target dir was `/tmp/ws-review/target`. Nothing was committed or pushed, and `/workspace/omni` was not touched. One side effect: the monolith selftest wrote `grpo_policy_checkpoint.json` into the scratch clone, and the selftests wrote `/tmp/*stage1_self*`.
**Confidence key:** **H** = I read the code and ran it, or it is pure arithmetic from the code. **M** = I read the code but did not execute that path. **L** = inference.

---

## 1. Summary

- **readme_honesty.md is mostly sound.** Of 57 claims, I rate **46 Confirmed, 3 Wrong, 3 Overstated, 4 Understated, 1 Unverifiable**. Its main thesis holds up in the code (H): the "Pure-Omni" frontend, driver, host, and Phase D wrappers are tiny integer kernels sitting in front of the Rust compiler, verifier, and host.
- **Measured counts:** monolith **416**, workspace **378** on Linux x86-64, 0 build warnings. `pure-omni-diff` 47, `acceptance-diff` 39, `cargo test --workspace` passes (H). The audit's predicted 416/378 are therefore confirmed by a real run.
- **Where the audit is wrong:**
  - (a) It says the FNV-1a64 hashes are "still locked" by the selftest. **No hash literal is asserted anywhere in code.** Only sizes and equalities are asserted. I recomputed all four hashes and they match (H).
  - (b) The Windows/Linux one-check split is **not** the symlink sandbox case. It is the native Stage-1==Stage-2 exec gated on `cfg(all(unix, x86_64))` (H).
- **Where the audit understates the problem:**
  - Phase C "host runtime" is **not on the `omni serve` path at all**.
  - Phase B's driver kernel is fed constant zeros.
  - `pure-omni-diff` contains no 4-way or NarrowHost step.
  - `compile-surface` writes `.omni_bc` that only the Phase 7 summary kernel has checked, not the Rust verifier.
- **Biggest missed fact:** every `std/*.omni` and `pure/*.omni` file except `verifier_engine.omni` is compiled from a **Rust string literal** in `compiler/mod.rs:2423-2958`, not from disk. The disk copies match today (normalized), but editing them changes nothing. The same is true of the acceptance goldens.
- **port_map.md** has accurate LOC and pub-fn counts. It needs a handful of fixes: one citation, one arithmetic slip, the assembler mischaracterized, the M2 dependency, and the 33-check description. It also has two **invariant concerns**: extending `pure/compiler.omni` in place would change the frozen 14,559-word suite, and the draft's proposed `-4`/`-8` ISET modes looked like a collision. That concern is now resolved by design, because forms are dispatched by operand count; see §5. Only the two shipped forms exist in code. Separately, the Rust-generated Stage-1 stream uses ~83 labels in one function.

---

## 2. Verdict table — readme_honesty.md claims

`hon:N` refers to the line in `readme_honesty.md`. All other cites are at `8160842`.

### Cross-cutting section (hon:23-40)

| # | Audit claim | Verdict | Evidence / correction | Conf |
|---|---|---|---|---|
| C1 | Pattern: Rust builds a tiny kernel, runs it on pre-digested ints, then does the real work in Rust | **Confirmed** | `pure_omni/mod.rs:61-86`, `verifier/mod.rs:71-171`, `pure_omni/mod.rs:229-280, 612-700, 806-852` | H |
| C2 | Frontend runs kernel on `(src.len(),0,0)` then Rust `compile_omni_surface_to_u16_with_sourcemap` | **Confirmed** | `pure_omni/mod.rs:72-86` | H |
| C3 | Phase 7: Rust walks stream, kernel maps 3 ints | **Confirmed** (nit) | Walk at `verifier/mod.rs:76-127`; kernel exec `:128-139`. Nit: what runs is the Rust const `PHASE7_PURE_OMNI_VERIFIER_KERNEL_SURFACE` (`verifier/mod.rs:33-52`), not the file. It is textually identical to `pure/verifier.omni:33-51`, a 19-line function inside a 51-line, 3-function file. | H |
| C4 | Phase D: kernel compiled by Rust twice, lowered by Rust AOT / Stage-1; Gen-1 doesn't emit Gen-2 | **Confirmed** | `pure_omni/mod.rs:806-852`. gen1_elf comes from the Rust x86 emitter (`:840`); gen2_elf from `compile_omni_surface_via_stage1_to_elf` (`:841`), which runs the Stage-1 stream in `OmniBinaryVm` (`compiler/mod.rs:3725-3727`). | H |
| C5 | `verifier_engine.omni` not on compile/run/serve path; no lockstep | **Confirmed** | Only callers are `verifier/mod.rs:502, 694, 717`, `pure_omni/mod.rs:2746`, and `omni-cli/src/main.rs:1282`. All are selftests or tests. | H |
| C6 | `PURE_SELF_HOST_ROADMAP.md:42-43` admits "not independence from Rust" | **Confirmed** | `spec/PURE_SELF_HOST_ROADMAP.md:42-43` | H |

### README.md claims (hon:46-110)

| # | Audit claim | Verdict | Evidence / correction | Conf |
|---|---|---|---|---|
| R1 | Components exist | **Confirmed** | crate tree | H |
| R2 | 8-crate workspace members | **Confirmed** | `omni-workspace/Cargo.toml:2-11` (members list) | H |
| R3 | Monolith 18,379 lines; CI `rustc -O` at `ci.yml:21` | **Confirmed** | `wc -l`; `.github/workflows/ci.yml:21` | H |
| R4 | Zero external crates | **Confirmed** | `Cargo.lock` has 8 `name` entries; all deps are `path =` | H |
| R5 | ISA `0x7F00..=0x7F7F`, `OMNI_ISA_TABLE: [OpcodeSpec;128]` | **Confirmed** | `omni-isa/src/lib.rs:45-46, 112` | H |
| R6 | Rust verifier gives `error[CODE]`+spans; Phase 7 hard-codes `line 1, col 1` | **Confirmed** | `verifier/mod.rs:145-165`; `compiler/mod.rs:3163`; harness output showed `"line":1,"col":58` on a real run | H |
| R7 | VM budgets; `MAX_VM_CALL_DEPTH = 64` at "lib.rs:31-41, 57" | **Confirmed** (cite fix) | Constant is at `omni-vm/src/lib.rs:28` and enforced at `:457`; `:57` is `with_budgets` | H |
| R8 | In-VM sim + live host | **Confirmed** | `omni-vm/src/lib.rs:588`; `host/mod.rs:37-47, 502, 929`. Smoke `eval` of the route example → `Int(200000420)`. | H |
| R9 | "Self-hosting Stage-1/2 ELF64 and Wasm AOT" overstated | **Confirmed** (incomplete) | Two additions. (1) Both AOT emitters compile **only `module.functions.first()`** (`omni-aot/src/lib.rs:153-154, 695-696`). (2) There is no Stage-1 for Wasm at all, so "self-hosting … Wasm" has nothing behind it. Wasm `IGET` constant-folds compile-time-known buffers (`:880-910`) and returns 0 otherwise (`:913-916`); `ISET` only bumps the length local (`:920-943`). | H |
| R10 | omni-lang is all Rust | **Confirmed** | — | H |
| R11 | 52-opcode AOT allowlist; Section 6 names stale | **Confirmed** | `omni-isa/src/lib.rs:253-285` (52 opcodes); `spec.md:120` lists `T_MATMUL…` at `0x7F60`, but the table says `TX_BEG` (`omni-isa/src/lib.rs:209`) | H |
| R12 | Two host modes, 10 syscalls, sandbox missing from README | **Confirmed** | `host/mod.rs:37-47, 54`; `spec.md:154-158` | H |
| R13 | `omni-pure-selfhost-v1.0.0` at `omni-isa:19` | **Confirmed** (cite fix) | It is at `omni-isa/src/lib.rs:18` | H |
| R14 | Stage-1 stream is Rust-built, 19,731 words | **Confirmed** | `omni-aot/src/lib.rs:1590`; lock at `pure_omni/mod.rs:1592`; measured `/tmp/ws_stage1_self.omni_bc` = 39,462 B = 19,731 words | H |
| R15 | Phase 7 "kernel" overstated | **Confirmed** | see C3 | H |
| R16 | Phase A overstated | **Confirmed** | `pure_omni/mod.rs:31-51, 61-86`. See Missed M8 for drift. | H |
| R17 | Phase B is a three-status mux | **Understated** | The driver kernel is executed on **constant `(0,0,0)`** (`pure_omni/mod.rs:266-269`), so it can never reject and is a no-op gate | H |
| R18 | Phase C kernel checks flags; syscalls Rust | **Understated** | `omni serve` → `run_omni_serve_cli` → `serve_omni_http_batch(_with_db_file)` (`host/mod.rs:971-973`) → `serve_omni_http_batch_inner` (`:823`), which **never calls** `serve_omni_http_via_pure_omni_host`. That function is called only from selftests (`pure_omni/mod.rs:745, 762, 769, 1470, 1506, 1793`), and even there its kernel result is discarded (`let _dispatch_val`, `:675`). | H |
| R19 | Phase D "sizes/hashes likely still locked … the selftest still locks them" | **Wrong** | Code asserts sizes plus `g1==g2` only (`pure_omni/mod.rs:1592`). No FNV literal exists in any `.rs` file except doc comments (`pure_omni/mod.rs:986`; monolith `:10383`). I recomputed via `pure-bootstrap`: `0x23B8D17257CCA9C4` / `0xE221085777B8AC47` / `0x78462AE4C81DF408` all match. | H |
| R20 | 14,559-word count locked; Gen-1==Gen-2 is the same compiler; engine excluded | **Confirmed** (cite nit) | `pure_omni/mod.rs:1982`, `:1127-1150`; exclusion comment is at `pure/verifier_engine.omni:16-17` (audit said 17-18) | H |
| R21 | Phase E 4-way runs on tiny kernels | **Understated** | `run_pure_omni_acceptance_diff` (`pure_omni/mod.rs:989-1050`) = 39 acceptance cases + Phase D + agent-repair + 6 programs = **47** (measured). It contains **no** VM/ELF/Stage-1/Wasm 4-way and **no** NarrowHost step, even though its doc comment promises one (`:985-987`). The 4-way runs live in the per-phase selftests (e.g. `:99-140`). | H |
| R22 | "All workflows route through Pure-Omni by default" overstated, borderline false | **Understated** | Audit said "Production verification is `verify_u16_stream`" for these paths. That is false for `compile-surface`: `main.rs:1021-1026` → `compiler/mod.rs:3394-3401` runs only the Phase A wrapper and Phase 7 kernel, with no Rust verifier. `compile-elf`/`-wasm` get `verify_binary_function` on fn #0 inside AOT (`omni-aot/src/lib.rs:155, 697`). `inspect` (`inspect/mod.rs:366` → `:181-184`) verifies nothing. `serve` never uses the Phase C host (R18). Verdict for the README sentence: **false** for "host runtime by default". | H |
| R23 | `pure/` 18 modules are small kernels | **Confirmed** | See M4: they are compiled from embedded copies | H |
| R24 | `std/` 7 real modules | **Confirmed** | Files exist; the compiled copies are embedded at `compiler/mod.rs:2425-2466` | H |
| R25 | 378 monolith stale → 416 | **Confirmed** | Measured `[SELFTEST PASS] 416`; 0 rustc warnings | H |
| R26a | 339/340 stale → 378 | **Confirmed** | Measured `[WORKSPACE SELFTEST PASS] 378` (Linux x86-64) | H |
| R26b | Windows/Linux split comes from the symlink sandbox | **Wrong** | Sandbox check 4 increments on non-Unix too (`host/mod.rs:1109-1113`). The +1 on Linux is the native Stage-1==Stage-2 exec under `#[cfg(all(unix, target_arch = "x86_64"))]` (`omni-cli/src/main.rs:923-940`). The monolith has similar gates (`omni_rust_bootstrap.rs:16148, 16291, 16344, 16432`), so the Windows monolith count probably also differs from 416 (L, not run). | H/L |
| R27 | Crate blurbs "true (mostly)" | **Overstated** (audit too lenient) | README:18 says `omni-isa` has "FNV-1a64 hashing". It does not; `fnv1a_64_hash` is at `omni-aot/src/lib.rs:2172`, and omni-isa only has the `0x7F5D FNV1A` table row (`:206`). README:24 "Two-pass x86-64 compiler" is first-function-only (R9). | H |
| R28 | omni-lang lists Phase 7 kernel as if it were the verifier | **Confirmed** | — | H |
| R29 | `spec/` executable acceptance suite = manifest + 40 goldens, "true with a nit" | **Overstated** | The runner executes **embedded** copies (`acceptance/mod.rs:86-109, 169-183`). The manifest is only substring-checked if present (`:203-224`), and disk goldens are never read. I diffed them: the 39 manifest goldens equal the embedded copies (normalized); `neg_circular_include.omni` is in neither. | H |
| R30 | `spec.md` normative; counts stale | **Confirmed** | `spec.md:1, 3` | H |
| R31 | Status files exist; contents overclaim | **Confirmed** | `omni_full_status_audit.md:36, 124, 129` | H |

### Quick Start (hon:104-110)

| # | Claim | Verdict | Evidence | Conf |
|---|---|---|---|---|
| Q1 | Headings stale | **Confirmed** | README.md:33, 41 | H |
| Q2 | Commands exist | **Confirmed** | I ran `omni-harness --strict`, `--retry-hint`, `--legacy-rust`, `omni-agent-driver`, `eval` ×2, and all the bootstrap/diff commands; all succeed | H |
| Q3 | PowerShell-only | **Confirmed** | README.md:34-73 | H |

### Other docs (hon:118-146) and "What is actually true" (hon:150-158)

| # | Claim | Verdict | Evidence | Conf |
|---|---|---|---|---|
| D1 | ROADMAP `:3` stale / "complete" overstated | **Confirmed** | `spec/PURE_SELF_HOST_ROADMAP.md:3` | H |
| D2 | ROADMAP table overstated | **Confirmed** | — | H |
| D3 | ROADMAP note honest | **Confirmed** | `:42-43` | H |
| D4 | "Locked hashes … true as locks" | **Wrong** | Hashes are not asserted in code (R19). They are printed by `pure-bootstrap` and they reproduce. | H |
| D5 | Stage-1==Stage-2 19,731 / 106,242 | **Confirmed** | Measured `/tmp/ws_stage1_self.elf` 106,242 B, FNV `0xEAFC81F46B482FF8` | H |
| D6 | `spec.md:3` counts stale | **Confirmed** | — | H |
| D7 | Section 6 names stale | **Confirmed** | R11 | H |
| D8 | 7.9 two-mode ABI true | **Confirmed** | `spec.md:154-158` | H |
| D9 | `spec.md:191, :194` overstated | **Confirmed** | — | H |
| D10 | status audit `:124, :129` overstated | **Confirmed** | — | H |
| D11 | 6,115-word suite stale | **Confirmed** | `spec.md:191`; `omni_full_status_audit.md:124, 128` | H |
| D12 | Instructions counts stale | **Confirmed** | `OMNI_PURE_SELF_HOST_INSTRUCTIONS.md:17` | H |
| D13 | "Phases 1–6 complete" is a process claim | **Unverifiable** | Historical process claim, nothing to check in code | — |
| D14 | LICENSE MIT, (c) 2026 Caleb Peacock | **Confirmed** | `LICENSE:1-3` | H |
| T1 | `verifier_engine.omni` 1,034 lines, 58 fns, loop-only, 77-case diff | **Confirmed** | 58 fns, 13 loops, no call-graph cycle (script). 77 = 16 surfaces + 6 goldens + 3 raw + 4 fuzz + 32 rand + 16 mutants (`verifier/mod.rs:566-704`). It compares verdict, code, and **word offset**, not source spans (`:500-563`). | H |
| T2 | Invariants "encoded in the ISA/VM" | **Overstated** | 192 SSA is enforced by the surface compiler (`compiler/mod.rs:1633`). 16 labels is enforced by the surface compiler (`:1645`) and by the 16 `LBL` tokens (`omni-isa/src/lib.rs:42-43`), but the decoder also accepts `ImmInt` label ids (`omni-ir/src/lib.rs:26`), and the Rust verifier does not cap label count (`omni-verifier/src/lib.rs:80-87, 156-175`). 64-deep calls is VM-enforced (`omni-vm/src/lib.rs:457`). Eager args and no-recursion are conventions; I found no checker for them. | H |

**Totals:** 46 Confirmed · 3 Wrong (R19, R26b, D4) · 3 Overstated (R27, R29, T2) · 4 Understated (R17, R18, R21, R22) · 1 Unverifiable (D13). Total 57.

---

## 3. Missed claims (guest vs host), not in readme_honesty.md

| # | Where | Claim / fact | Reality | Conf |
|---|---|---|---|---|
| M1 | README.md:10 | `serve` routes through the Pure-Omni host runtime by default | False. The Phase C kernel is selftest-only (R18). | H |
| M2 | README.md:5 | "self-hosting … 64-bit WebAssembly AOT backends" | No Wasm Stage-1 exists; the Wasm emitter is Rust only (`omni-aot/src/lib.rs:694`) | H |
| M3 | README.md:18 | `omni-isa` has FNV-1a64 hashing | False. It is `omni-aot/src/lib.rs:2172`. | H |
| M4 | README.md:14-15 | `pure/` and `std/` files are "the" Pure-Omni modules | Compiled sources are **Rust string literals** (`compiler/mod.rs:2423-2958`; include resolution prefers them at `:3014`). Only `verifier_engine.omni` is `include_str!`'d (`verifier/mod.rs:271-272`). The disk files match today modulo comments/whitespace (scripted diff, all 18). | H |
| M5 | README.md:26 | Executable acceptance suite runs the manifest + 40 goldens | It runs embedded copies (R29) | H |
| M6 | README.md:23 | Verification on the compile path | `compile-surface` writes `.omni_bc` checked only by the Phase 7 summary kernel (`compiler/mod.rs:3394-3401`, `omni-cli/src/main.rs:1021-1026`). That kernel does not check SSA order, label resolution, reassignment, or unknown opcodes. Practical risk is low because the compiler itself is careful, but the claim is wrong. | H (path) / M (risk) |
| M7 | README.md:10 | Phase E "verified across the 4-way … and `OmniNarrowHost`" | `pure-omni-diff` has neither (R21). Its "Rust vs Pure-Omni" comparisons compare the same compiler. | H |
| M8 | `spec.md:189`; `pure_omni/mod.rs:25` | Phase A kernel "is" `spec/pure_self_host/frontend.omni` | **Drifted.** The disk file returns `2016` and passes codes through; the executed const returns `2001/2002/2003` (`pure_omni/mod.rs:31-51`). Also `lookup_omni_stdlib_module` aliases `spec/pure_self_host/omni_selfhost_compiler.omni` to the **pure/compiler.omni** text (`compiler/mod.rs:2467`) and `spec/pure_self_host/host_runtime.omni` to the pure/ text (`:2596`), so an `include` of those paths gets different code than the disk file. Driver/host/Phase D disk copies match their consts. | H |
| M9 | README.md:24 | "Two-pass x86-64 Linux ELF64 compiler" | It compiles only the first function of a module (`omni-aot/src/lib.rs:153-154`). The Stage-1 surface path requires a single function (`compiler/mod.rs:3717-3719`). | H |
| M10 | `spec.md:169` | "16 negative structured-error goldens" | 15 in the manifest and runner | H |
| M11 | README.md:10 (Phase D) | Fixed-point hashes are "verified" | Not asserted in code (R19). Reproducible, but unguarded. | H |
| M12 | README.md:24, `spec.md` | Stage-1 is a self-hosted ELF backend under Omni's limits | The Rust-generated Stage-1 stream is a single function that defines ~83 labels (ids 0..82; `MAX_LABELS = 83` at `omni-aot/src/lib.rs:1612`; ≥38 static `alloc_lbl` calls plus `emit_decode_operand`). It encodes branch targets as `IMM_I16` ids (`:1556-1566`), so it sits outside the 16-label/function rule that the surface language enforces. | H (count from byte-scan + source) |
| M13 | README.md:9 | `omni serve --db` is sandboxed | True, with nuance: the jail root is the CWD, or the **parent dir of an absolute `--db` path** (`host/mod.rs:759-768, 862`) | M |

---

## 4. Port map corrections

LOC and public-fn counts for **all 20 Rust files match** `wc -l` and a grep that counts both `pub fn` and `pub(crate) fn` (H). No Rust file is omitted.

| Item | port_map says | Correction | Conf |
|---|---|---|---|
| Snapshot total | "35,122 including the monolith" | 16,733 + 18,379 = **35,112** | H |
| `#[path]` include | `main.rs:37` | **`omni-cli/src/main.rs:7`** | H |
| `compiler/` if_pos selftest | "33 checks across VM/Wasm/x86-64/Stage-1" | 33 is correct (5 cases × 3 + 1 locked + 1 Stage-1 + 16 I/O pairs; `compiler/mod.rs:3797-3890`). But x86-64 and Stage-1 are **compile-only**, Stage-1 runs for one case only, and the executed checks are VM + Wasm. | H |
| `omni-assembler` | "binary packet encoder `encode_asm_ins_packet_u16` is the smaller, loop-friendly slice" | `encode_asm_ins_packet_u16(line: &str, …)` (`omni-assembler/src/lib.rs:27`) **parses an asm text line**. The Rust surface compiler lowers by emitting text and calling it (`compiler/mod.rs:1597-1629, 2019`). An Omni port wants a **direct (op, operands) → packet** encoder, not a port of this text parser. | H |
| `acceptance/` blockers | "filesystem, JSON manifest" | The runner does not load files or the manifest beyond a substring check (`acceptance/mod.rs:203-224`) | H |
| `omni-aot` port order | "Milestone 2, after the frontend can at least emit the subset" | Wrong dependency. A u16-token → ELF64 compiler consumes tokens that the Rust frontend already produces. M2 depends on the **decoder + opcode table + x86 encodings** (`omni-aot/src/lib.rs:147-690`), **not** on the Omni frontend. | H |
| `omni-aot` | (missing) | Note the single-function limitation (`:153-154`) and Stage-1's ~83-label single function (M12). A hand-written Omni emitter must be split into many ≤16-label functions; it cannot mirror the Stage-1 layout. | H |
| `std/prelude.omni` | "Prelude includes / thin wrappers" | Four functions duplicated from `math`/`trit`; no includes (`compiler/mod.rs:2425-2430`) | H |
| `spec/pure_self_host/*.omni` | "Copies of Phase A–D kernels" | The Phase A copy has **drifted** from the executed const, and two paths are aliased to `pure/` text (M8) | H |
| Inventory (all `pure/` and `std/` rows) | describes the disk files | Add: they are compiled from embedded Rust literals (M4). A port that edits only the disk file is a no-op. | H |
| `verifier/` | `verifier_engine.omni:72-96` load helpers | `load_u16` is at `:71-78`, `load_i64le` at `:80-95` | H |
| `harness/` | `run_omni_harness_cli` at `pure_omni/mod.rs:2763` | **Confirmed** | H |
| `inspect/` | CLI uses `inspect_omni_surface_via_pure_omni` (implied) | CLI uses `inspect_omni_surface_program` (`inspect/mod.rs:366` → `:181`), which applies the Phase A wrapper and **no verifier** | H |

Responsibility descriptions are otherwise accurate: host 10-syscall trait at `host/mod.rs:37-47`; sandbox at `:54`; VM `Struct`/`HashMap` at `omni-vm/src/lib.rs:20, 31`; `decode_omni_token_stream` at `omni-ir/src/lib.rs:121`; ISA ranges at `omni-isa/src/lib.rs:20-46`. HTTP, DB, and the Repair Agent stay in core as intended; I did not flag them.

---

## 5. Port order review

**Dependencies (from code):**
- The decoder needs opcode properties. The engine duplicates them today as `op_req_cap_*`.
- The surface frontend's lowering needs a packet encoder (`compiler/mod.rs:1625`).
- The M2 emitter needs the decoder, the opcode table, and the x86 encodings, but not the frontend.
- The verifier default needs only the engine and lockstep wiring.

**Issues found:**
1. **Graph and list disagree.** The ASCII graph puts `omni-isa` constants first, but the "next three" list puts the decoder #1 and ISA #2. Both are defensible because the engine already embeds what the decoder needs. Pick one. My recommendation: **decoder (extracted from the engine) → opcode table as frozen u16 data → direct packet encoder → frontend lexer → parser → lowering**. Run M2 **in parallel** after the decoder and table land. (M)
2. **The packet encoder belongs inside the top three**, ahead of lowering, because lowering cannot emit without it (H).
3. **M2 placement:** decouple it from the frontend (see §4).

**Invariant check of the audit's suggestions:**

| Suggestion | Where | Invariant at risk | Verdict |
|---|---|---|---|
| "Prefer extending `pure/compiler.omni` in place" | port_map.md:376; README_draft.md:126 ("emitter in `pure/compiler.omni`") | **Frozen hashes.** `pure/compiler.omni` is in the closed 18-module suite (`pure_omni/mod.rs:1114`; embedded text `compiler/mod.rs:2467`), and its size is asserted (`:1982`). Any edit changes 14,559 / `0x78462AE4C81DF408`. It does not change the Phase D 159/824 hashes, which come from a separate const (`pure_omni/mod.rs:782`). | **Flag.** Use a new module (e.g. `pure/compiler_engine.omni`), as the port map's own alternative says. Needs Caleb's call because Nyx's brief says "replace the stub `pure/compiler.omni`". |
| Multi-byte forms (agreed design, from the coordinator): 3-op `ISET -4/-8` append i32/i64 (unchanged); 4-op `ISET -9` u64 patch (unchanged); **new** 4-op `ISET -2/-4` u16/u32 patch; **new** 3-op `IGET -2/-4/-8` u16/u32/u64 load | README_draft.md:120 (superseded); `README_draft.scarlett.md` roadmap item 3 | Frozen goldens and hashes; no new opcodes or renumbering | **Resolved by design** (re-checked at `8160842`, which is still the GitHub tip of `isa/multibyte-loads`). Every existing typed-`ISET` branch tests exact arity: 3-op `-4`/`-8` and 4-op `-9` in the VM (`omni-vm/src/lib.rs:294, 299, 304`), x86-64 (`omni-aot/src/lib.rs:350, 353, 356`), the Wasm length tracker (`:921, 923, 925`), the AOT pack5 pre-scan (`:28-30`), and the monolith mirrors (`omni_rust_bootstrap.rs:1311-1319, 13847-13848, 14182-14188, 14753-14757`). A 4-op `-4` therefore cannot alias the 3-op append. **Not implemented yet:** no branch exists for 4-op `ISET -2/-4`; today they fall through to generic byte-append (`omni-vm/src/lib.rs:314-319`, `omni-aot/src/lib.rs:360-364`). `IGET` reads only operands 0-1 on the VM (`omni-vm/src/lib.rs:272-287`), x86 (`omni-aot/src/lib.rs:305-344`), and Wasm (`:876-917`). There are no surface intrinsics for them; `iget` is arity-2 (`compiler/mod.rs:1897-1907`). **Keep:** `byte_buf_golden` must stay at 86 tokens (`acceptance/mod.rs:100`; it uses 3-op `-4`/`-8`). Residual edge case: generic `iset(buf, -2\|-4, a, b)` with 4 operands means "append 3 bytes" today and would change meaning. I found no such stream in the repo (H), so a lint or golden guarding it is advisable. |
| "Put the 128-opcode table in a frozen u16 blob" | port_map.md:356 | Renumbering | OK as long as the blob is generated from `OMNI_ISA_TABLE` and diffed against it. |
| Split frontend into many loop-only fns, arena AST | port_map.md:364-365 | 192/16/64, no heap, no recursion | OK. |
| "Do not raise SSA/label/call limits" | port_map.md:337 | — | OK. Note that Stage-1 already sits outside the label rule (M12); do not cite it as precedent. |
| "Do not port the VM" | port_map.md:240 | — | OK. |

---

## 6. README draft changes (`README_draft.md` → `README_draft.scarlett.md`)

Diff summary:
- **Removed:**
  - FNV-1a64 under `omni-isa` (false, M3).
  - "Windows may differ by the symlink sandbox case" (wrong, R26b).
  - "Expected 416/378" hedging; replaced with the measured counts and their environment.
  - The link to `audits/port_map.md`, which is not a repo path.
  - "lib.rs split and MIT license are already done on the hygiene branch". Those commits (`12492aa`, `5c87b0f`) are already ancestors of `8160842`, so it is not roadmap.
  - The earlier `-2/-4/-8/-9` ISET-mode wording. It is replaced by an operand-shape table that labels the 3-op `-4/-8` and 4-op `-9` forms **shipped** and the 4-op `ISET -2/-4` and 3-op `IGET -2/-4/-8` forms **planned**.
  - "Milestone 2 in `pure/compiler.omni`" (hash risk).
  - The `vm/` boundary roadmap item (vague, unverified).
- **Added:**
  - A separate **guest table** and **host table**.
  - The embedded-copy fact (M4).
  - `serve` does not use the Phase C kernel (M1).
  - The Phase B constant-input gate (R17).
  - `compile-surface` uses only the Phase 7 kernel (M6).
  - AOT is first-function-only (M9).
  - The 52-opcode allowlist; "everything else rejected" replaces the narrower `0x7F64..` range.
  - The Wasm `IGET` constant-fold nuance.
  - Hashes are printed, reproduced, but not asserted (R19).
  - Acceptance runs embedded goldens (R29).
  - The Stage-1 native fixed point runs only on Unix x86-64.
  - The sandbox root rule.
- **Tightened:**
  - Constraints say *who enforces* each limit.
  - The roadmap is explicitly "none of this is done" and reordered by dependency (M2 decoupled from the frontend).
- **Kept:** the zero-crates claim, the hard limits, the HTTP/DB/repair-in-core note, and the Linux + PowerShell quick starts.

**Citations backing every README_draft.scarlett.md claim** (all at `8160842`):

| README claim | Citation |
|---|---|
| 128 opcodes `0x7F00..=0x7F7F` | `omni-isa/src/lib.rs:45-46, 112` |
| Zero crates | `omni-workspace/Cargo.lock` (8 packages); crate `Cargo.toml`s are path-only |
| 192 SSA / 16 labels by compiler; 64 calls by VM | `compiler/mod.rs:1633, 1645`; `omni-vm/src/lib.rs:28, 457` |
| Engine: 1,034 lines, 58 fns, loops, no recursion, 77 cases, not default, excluded | `pure/verifier_engine.omni:1-33, 16-17`; `verifier/mod.rs:271-272, 500-704`; callers in C5 |
| Kernels map ints to codes | `pure/compiler.omni:26-117`, `pure/verifier.omni:12-51`, `pure/host_runtime.omni:19-70`, `pure/tooling.omni:29-99`, `pure/selfhost_suite.omni:31-65` |
| Embedded copies; disk matches | `compiler/mod.rs:2423-2958, 3014`; my normalized diff |
| Phase A on `(len,0,0)` then Rust | `pure_omni/mod.rs:72-86` |
| Verifier used by run/eval/repl/harness/agent/serve; compile-surface Phase 7 only; AOT verifies fn #0 | `pure_omni/mod.rs:244-259`; `host/mod.rs:840-851`; `repair_agent/mod.rs:1046`; `compiler/mod.rs:3394-3401`; `omni-cli/src/main.rs:1021-1026`; `omni-aot/src/lib.rs:153-155, 695-697` |
| Phase 7 reduction + `line 1, col 1` | `verifier/mod.rs:76-165` |
| VM sim packing | `omni-vm/src/lib.rs:588` |
| AOT first fn only; 52 allowlist; Wasm IGET/ISET | `omni-aot/src/lib.rs:153-154, 695-696, 876-943`; `omni-isa/src/lib.rs:253-285` |
| Stage-1 Rust-generated, 19,731 words, VM-run, native fixed point on Unix x86-64, 106,242 B | `omni-aot/src/lib.rs:1590`; `pure_omni/mod.rs:1592`; `compiler/mod.rs:3725-3727`; `omni-cli/src/main.rs:923-940`; measured |
| Serve = Rust over 10 syscalls; sandbox; Phase C not on path | `host/mod.rs:37-47, 54, 759-768, 823, 971-973`; `pure_omni/mod.rs:675` + callers |
| Wrappers by default; Phase B constant inputs; `--legacy-rust` | `omni-cli/src/main.rs:1130, 1165, 1192`; `pure_omni/mod.rs:266-269, 2766, 2790` |
| Hash table values; sizes asserted, hashes not | `pure_omni/mod.rs:1592, 1982`; `pure-bootstrap` output; FNV of `/tmp/ws_stage1_self.elf` |
| Phase D two lowerings agree | `pure_omni/mod.rs:840-848` |
| Crate roles; assembler line encoder; FNV in aot | `omni-assembler/src/lib.rs:27, 217`; `compiler/mod.rs:1625`; `omni-aot/src/lib.rs:2172`; `omni-cli/Cargo.toml` bins |
| Goldens 24 + 15 (+1 off-manifest); runner embedded | `spec/acceptance/acceptance_manifest.json`; `acceptance/mod.rs:86-183, 203-224` |
| `spec.md` stale items | `spec.md:3, 120, 169` vs `omni-isa/src/lib.rs:209` |
| Counts 416/378/47/39, cargo test, 0 warnings | Measured runs (logs in `/workspace/audit-scarlett-review/*.log`, `/tmp/ws-review/*.log`) |
| Quick-start commands work | Smoke runs (Q2); `host/mod.rs:931-968` for serve flags |

---

## 7. Open questions / uncertainty

1. **Windows counts** were not run. The monolith and the workspace both have `cfg(unix)` / `cfg(all(unix, x86_64))` gates, so I would not state any Windows number until one is measured (L).
2. **Is the 16-label rule an ISA rule or a surface rule?** `LBL` tokens are 16 (`omni-isa:42-43`), but `ImmInt` label ids are accepted by the decoder (`omni-ir:26`) and the Rust verifier, and Stage-1 uses ~83. Caleb should decide whether the Rust verifier should cap labels on raw streams. That would reject the Stage-1 stream, so it is a breaking change that needs a plan. (H fact / decision open)
3. **"Frozen bootstrap hashes": which set?** If the 14,559-word suite hash is frozen, M2 cannot edit `pure/compiler.omni`. Nyx's brief says "replace the stub". The two need reconciling (Caleb / Boss Lady).
4. **Hashes are unguarded.** Should a selftest assert them? Adding the assertion changes the check counts again (+N). That is a decision, not a fix I made.
5. I did not run the Windows PowerShell path, `omni serve` over real TCP, or `compile-surface` on a deliberately malformed stream (M6 risk rated M).
6. **Label count (~83)** comes from a byte scan of `0x7F40, IMM_I16, id` patterns in the Stage-1 `.omni_bc`, cross-checked against `MAX_LABELS = 83`. A data word could in principle mimic the pattern, but the source agrees (H-).
7. readme_honesty's "known Milestone 2 issues from earlier review" (`port_map.md:266`) cite a review I don't have. I mark them **Unverifiable**.
