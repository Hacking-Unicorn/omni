# README Honesty Audit

**Repo:** `github.com/Hacking-Unicorn/omni`  
**Branch audited:** `origin/isa/multibyte-loads` (`8160842` — "Fix value-returning if_pos SSA join-slot holes and add a regression selftest.")  
**Method:** Read-only clone at `/workspace/audit-readme`. Every claim below was checked against source on that commit. Selftests were **not** re-executed in this audit; check counts are derived from the code that increments them, plus the counts reported when the `if_pos` fix landed.  
**Standard:** The README must be 100% transparent about what runs in the `.omni` guest versus the Rust host, and must never overclaim self-hosting.

A proposed rewrite is in [`README_draft.md`](./README_draft.md).

---

## How to read the verdicts

- **true** — the claim matches the code, with at most wording nits that do not change meaning.
- **overstated** — something real exists, but the wording implies more than the code does (typical pattern: a tiny `.omni` decision kernel plus a Rust implementation that does the actual work, advertised as "Pure-Omni").
- **false** — the claim is stale or contradicted by the code on this branch.
- **stale** — a numeric or status claim that was true on an earlier commit (often `main` at `b3d42dd`) and is wrong on `8160842`.

Line references use the audited tree (`README.md`, `omni-workspace/crates/...`, `pure/...`, `spec.md`, etc.).

---

## Cross-cutting finding (applies to many claims)

The functions named `compile_omni_surface_via_pure_omni_frontend*`, `execute_omni_surface_via_pure_omni_driver`, `verify_module_via_phase7_pure_omni_kernel`, `serve_omni_http_via_pure_omni_host`, and `run_phase_d_pure_omni_selfhost_bootstrap` do **not** implement a surface compiler, a stream-walking verifier, a TCP/HTTP host, or a token-to-ELF emitter in Omni.

The repeated pattern is:

1. Rust compiles a tiny `#[strict]` kernel (tens of lines) that maps pre-digested `i64` flags to error codes.
2. Rust runs that kernel in `OmniBinaryVm` on those flags (often `(src.len(), 0, 0)`).
3. Rust then does the real work: `compile_omni_surface_to_u16_with_sourcemap`, `verify_u16_stream` / `verify_binary_function`, `OmniNarrowHost` syscalls, or `compile_pure_u16_to_x86_64_elf_with_args`.

Evidence:

- Frontend: `pure_omni/mod.rs:61-86` compiles `PHASE_A_PURE_OMNI_FRONTEND_KERNEL_SURFACE` (source-length / strict / syntax flags only), executes it on `(src.len(), 0, 0)`, then calls **Rust** `compile_omni_surface_to_u16_with_sourcemap`.
- Phase 7 verifier: `verifier/mod.rs:71-136` walks the token stream **in Rust**, computes `(missing_cap_bits, taint_violation_kind, arena_balance)`, then runs `pure/verifier.omni`'s 51-line `omni_pure_verify_header` on those three integers.
- Phase D "self-host": `pure_omni/mod.rs:806-841` compiles the 34-line `omni_selfhost_compiler_main` kernel with Rust (`compile_omni_surface_strict_to_u16`), executes it as a status check, compiles "gen2" with the same Rust-backed "Pure-Omni frontend", and lowers both with the **Rust** x86-64 / Stage-1 emitters. Gen-1 does not compile Gen-2.
- The one genuine Omni-side stream walker, `pure/verifier_engine.omni` (1,034 lines, 58 functions), is **not** on the default compile/run/serve path. It is exercised by `run_verifier_engine_diff_selftests` and `verify_u16_stream_via_omni_engine`. There is no lockstep/shadow wiring in `compile`, `run`, or `serve` on this commit.

`spec/PURE_SELF_HOST_ROADMAP.md:42-43` already admits this more honestly than the README: the Pure-Omni path is still a subset of the Rust bootstrap, the kernel is 159 words, and "Gen-1 == Gen-2" proves agreement with the Rust reference, **not independence from Rust**. That note never made it into `README.md`.

---

## README.md claims

### Title and workspace path (lines 1–3)

| Claim | Verdict | Evidence |
|---|---|---|
| The project is an Omni language, ISA, verifier, VM, AOT compiler, and model harness | **true** | All of those components exist. The honesty issue is *which language they are written in*, treated below. |
| The 8-crate Cargo workspace lives at `omni-workspace/` | **true** | `omni-workspace/Cargo.toml` members: `omni-isa`, `omni-ir`, `omni-verifier`, `omni-vm`, `omni-assembler`, `omni-lang`, `omni-aot`, `omni-cli`. |
| The standalone single-file reference compiler is `omni_rust_bootstrap.rs` and compiles with `rustc -O omni_rust_bootstrap.rs -o omni` and no flags | **true** | File is 18,379 lines at repo root. CI (`.github/workflows/ci.yml:21`) uses exactly that command. |

### Lead paragraph (line 5)

| Claim | Verdict | Evidence |
|---|---|---|
| Zero-dependency Rust implementation | **true** | Every crate `Cargo.toml` depends only on path-local workspace crates. `Cargo.lock` lists exactly the eight workspace packages; no crates.io dependencies. This is a real sovereignty constraint and should stay in the README. |
| 16-bit binary token ISA `0x7F00..=0x7F7F` | **true** | `omni-isa/src/lib.rs:45-46, 99-112`: `OP_BASE`/`OP_MAX` and `OMNI_ISA_TABLE: [OpcodeSpec; 128]`, one entry per opcode in that range. |
| Static linear / capability / taint verifier with machine-readable `error[<CODE>]` and 1-based `(line, col)` spans | **true** (with a scope note) | The **Rust** verifier (`omni-verifier/src/lib.rs:75`, surface annotation in `compiler/mod.rs:3163` and `harness/mod.rs:78`) does this. Phase 7 kernel errors often hard-code `line 1, col 1` (`verifier/mod.rs:143-165`). The Omni engine reports instruction offsets, not source spans, unless Rust annotates them. |
| Deterministic ternary & numeric VM (`OmniBinaryVm`) with step/fuel and memory budgets | **true** | `omni-vm/src/lib.rs:31-41, 57`, `MAX_VM_CALL_DEPTH = 64`. |
| In-VM host contract simulator **and** live native TCP HTTP/1.1 + file-backed DB/KV host (`omni serve`) | **true** | Simulator: `omni-vm/src/lib.rs:588` returns `Int(status.wrapping_mul(1_000_000).wrapping_add(body))`. Live host: `host/mod.rs:37-47, 502, 929` (`OmniNarrowHost`, `serve_omni_http_via_narrow_host`, `run_omni_serve_cli`). Route/param/db/kv/auth/respond surface names exist in `spec.md:155`. |
| Self-hosting Stage-1/Stage-2 `x86-64` ELF64 and 64-bit Wasm AOT backends, "verified for the pure numeric/ternary/`@PACK5`/arena subset" | **overstated** | Two different things are being bundled. (1) A **Rust-generated** Stage-1 compiler stream (`build_step7_stage1_compiler_stream`, `omni-aot/src/lib.rs:1590`, locked at 19,731 `u16` words) is a real (subset) u16-to-ELF64 emitter, and Stage-1 == Stage-2 is a meaningful fixed point for that stream. (2) Standalone x86-64 and Wasm AOT emitters are **Rust** (`compile_pure_u16_to_x86_64_elf_with_args`, `compile_pure_u16_to_wasm`). (3) Wasm `IGET`/`ISET` are incomplete: unhandled `0x7F0E` returns `i64.const 0`, and `0x7F0F` updates a length local without writing bytes (`omni-aot/src/lib.rs:876-944`). `byte_buf_golden.omni` only checks `slen(buf) + cmov(...)`, so it cannot catch that gap. "Verified" holds for the kernels and goldens that never read buffer contents back on Wasm. |
| `.omni` surface language compiler (`omni-lang`) with `#[strict]`, hygienic macros, stdlib prelude, harness, and agent driver | **true** | All implemented in **Rust** (`compiler/mod.rs`, `harness/mod.rs`, `repair_agent/mod.rs`, `omni-cli` bins). |

### Scope & Honesty Note — AOT and host ABI (lines 7–9)

| Claim | Verdict | Evidence |
|---|---|---|
| AOT covers the verified numeric, ternary, control-flow, byte-buffer, `@PACK5`, and linear arena subset; host opcodes `0x7F64..=0x7F7F` are rejected by standalone AOT (`SPEC.md` Section 6) | **true** (minor nits) | `is_x86_64_aot_supported_opcode` / `is_wasm_aot_supported_opcode` (`omni-isa/src/lib.rs:253-285`) allow a 52-opcode subset ending at `0x7F59` (sanitizers). Host opcodes from `0x7F60`/`0x7F64` upward are rejected. File is `spec.md`, not `SPEC.md`. Section 6 exists (`spec.md:111`) but some opcode *names* in that table are stale versus `OMNI_ISA_TABLE` (e.g. `0x7F60` is `TX_BEG` in the table, not `T_MATMUL`). Byte-buffer **content** parity on Wasm is the gap noted above. |
| Host ABI has two explicit modes: in-VM simulation (`Int(status * 1_000_000 + body)`) and live TCP + file-backed DB/KV over the 10-syscall `OmniNarrowHost` (`omni-narrow-host-v1.0.0`) | **true** | Trait has exactly ten methods (`host/mod.rs:37-47`). Simulation return packing at `omni-vm/src/lib.rs:588`. `spec.md:157` and `spec/host_interface.md` document the split. FS/DB paths are also root-jailed (`resolve_sandboxed_host_path`, `error[PATH_ESCAPE]`, `host/mod.rs:54`, 4 sandbox checks) — this is real and currently **missing** from the README. |

### Scope & Honesty Note — Pure Omni Self-Hosting (line 10)

This is the most overclaimed paragraph in the repository.

| Claim | Verdict | Evidence |
|---|---|---|
| Contract version `omni-pure-selfhost-v1.0.0` exists | **true** | `omni-isa/src/lib.rs:19`. |
| "Stage-1/Stage-2 self-hosts the binary token-to-ELF64 backend (`omni_self_compiler.omni_bc`)" | **overstated** | The Stage-1 stream is built in **Rust** (`build_step7_stage1_compiler_stream`) and can compile a supported u16 module to ELF64, including itself, at a locked size of 19,731 words (`pure_omni/mod.rs:1592`). That is a real bootstrap artifact. It is not a Pure-Omni *surface* compiler, and it is not `pure/compiler.omni`. |
| "Phase 7 Pure-Omni static verifier kernel (`omni-pure-kernel-v1.0.0`)" as if it verified token streams | **overstated** | `PHASE7_PURE_OMNI_VERIFIER_KERNEL_SURFACE` (`verifier/mod.rs:33-52`) and `pure/verifier.omni` (51 lines) map three precomputed integers onto `1001..=1005`. Stream analysis is Rust (`verifier/mod.rs:76-127`). The real stream-walking Omni verifier is `pure/verifier_engine.omni`, not this kernel, and is not default. |
| "Phase A Pure-Omni surface frontend (`omni-pure-frontend-v1.0.0`)" | **overstated** | `pure/compiler.omni` and `spec/pure_self_host/frontend.omni` are flag-to-error-code kernels. Tokenizing, parsing, and lowering are Rust (`compiler/mod.rs:132-3352`). See the cross-cutting finding. |
| "Phase B Pure-Omni driver (`omni-pure-driver-v1.0.0`)" | **overstated** | `PHASE_B_PURE_OMNI_DRIVER_SURFACE` (`pure_omni/mod.rs:209-218`) is a three-status mux. `execute_omni_surface_via_pure_omni_driver` (`pure_omni/mod.rs:229-280`) still compiles and verifies in Rust (plus the Phase 7 summary kernel) and executes in `OmniBinaryVm`. |
| "Phase C Pure-Omni host runtime (`omni-pure-host-v1.0.0`)" | **overstated** | `pure/host_runtime.omni` (70 lines) checks cap/taint flags and packs `status * 1000000 + payload`. TCP, file, and time syscalls are Rust `OmniNarrowHost`. `omni_pure_host_kv_apply` does not talk to a real KV store (`pure/host_runtime.omni:47-55`). |
| Phase D fixed point `gen1 == gen2`: `159` words `0x23B8D17257CCA9C4`, `824` ELF64 bytes `0xE221085777B8AC47` | **overstated** (sizes/hashes likely still locked; meaning is not self-hosting) | Code still asserts `g1_u16.len() == 159`, `g1_elf.len() == 824`, and `g1 == g2` (`pure_omni/mod.rs:1592`). Those artifacts are the tiny `omni_selfhost_compiler_main` kernel compiled **twice by Rust**, then lowered by Rust AOT / Stage-1. Gen-1 never emits Gen-2. The hashes are not recomputed in this audit; the selftest still locks them. |
| Closed 18-module suite: `14,559` `u16` words, `0x78462AE4C81DF408`, `gen1 == gen2` "agreement between the Pure-Omni frontend and the Rust reference" | **overstated** (count is locked; interpretation is not "self-host") | `pure_omni/mod.rs:1982` still requires `suite_u16.len() == 14559`. "Gen-1 == Gen-2" here is `compile_omni_surface_via_pure_omni_frontend` vs `compile_omni_surface_strict_to_u16` — two names for the same Rust compiler after a no-op kernel check (`pure_omni/mod.rs:61-86, 1133`). `pure/verifier_engine.omni` is intentionally **excluded** from this suite so the hash stays still (comment at `pure/verifier_engine.omni:17-18`). |
| Phase E full differential lock (`omni pure-omni-diff`) "verified across the 4-way VM / Host-ELF64 / Stage-1-ELF64 / Wasm differential and `OmniNarrowHost`" | **overstated** | `pure-omni-diff` exists and is run in CI. The 4-way checks run on the **tiny kernels** (Phase A/B/C/D/7 surfaces), not on a self-hosted compiler or a full host. Wasm byte-buffer contents are not in that lock. |
| "All regular workflows (`run`, `eval`, `repl`, `serve`, `compile-surface`, `inspect`/`disasm`, `harness`, and `agent-driver`) routing through the Pure-Omni frontend, verifier kernel, driver, and host runtime **by default**" | **overstated** (borderline **false** if read as "those stages run in Omni") | CLI `run`/`eval`/`repl` do call `execute_omni_surface_via_pure_omni_driver` (`omni-cli/src/main.rs:1130, 1165, 1192`). `compile_omni_surface_file_to_u16` calls the "pure frontend" and Phase 7 kernel (`compiler/mod.rs:3394-3400`). Harness defaults `pure_omni = true` with a `--legacy-rust` escape hatch (`pure_omni/mod.rs:2766, 2789-2790`). **All of those entry points still perform compile, verify, and host I/O in Rust**, as shown above. Production verification is `verify_u16_stream` / `verify_binary_function`, not `pure/verifier_engine.omni`. |

### Repository layout (lines 12–29)

| Claim | Verdict | Evidence |
|---|---|---|
| `pure/` holds the core compiler, static verifier, host runtime, tooling kernels, and a 6-program self-host suite, totaling 18 modules and `14,559` words, `Gen-1 == Gen-2` | **overstated** | The files exist (5 core + 6 programs + 7 `std/` = 18). They are small decision/demo kernels, not replacements for the Rust compiler/verifier/host. Word count is a locked token-stream size, not a measure of self-hosting. |
| `std/` lists seven Pure-Omni standard library modules | **true** | `std/{prelude,math,trit,http,pack5,decision,resource}.omni` exist and are real (if small) Omni modules. |
| Monolith has `378` self-test checks, `0` warnings | **stale / false** on this branch | `378` was the S4/`main` figure (`omni_full_status_audit.md:36`). This branch adds 4 sandbox + 1 engine gate + 33 `if_pos` checks (`pure_omni/mod.rs:2743-2755`, `host/mod.rs:1002`, `compiler/mod.rs:3797-3890`). Expected monolith total: **416**. `0` warnings was not re-checked here; CI does not pass `-D warnings`. |
| Workspace has `339` Windows / `340` Linux `x86-64` monolith-vs-workspace differential checks, `0` warnings | **stale / false** on this branch | Cargo test still only asserts `n >= 340` (`omni-cli/src/main.rs:1243`). The same +4+1+33 additions flow through `run_ai_first_surface_selftests` into the workspace selftest. Expected Linux total: **378**. The Windows/Linux split (symlink sandbox) may still differ by one on older counts; check 4 of the sandbox suite is a no-op increment on non-Unix (`host/mod.rs:1109-1113`). |
| Crate blurbs for `omni-isa`, `omni-ir`, `omni-verifier`, `omni-vm`, `omni-assembler`, `omni-aot`, `omni-cli` | **true** (mostly) | These crates contain real implementations, not empty facades. Version constants listed under `omni-isa` exist; newer ones (`OMNI_PURE_FRONTEND_VERSION`, etc.) are omitted, which is fine. `omni-vm` README range `0x7F64..=0x7F70` is the documented "full-stack host ABI" slice (`spec.md:155`); the VM implements a wider opcode set. |
| `omni-lang` "Phase 7 Pure-Omni verifier kernel (`verify_module_via_phase7_pure_omni_kernel`)" listed as if it were the crate's verifier | **overstated** | The crate's production verifier is `omni-verifier` plus wrappers. Phase 7 is the summary kernel. The module also contains `verify_u16_stream_via_omni_engine`, which the README does not mention. |
| `spec/` has an executable acceptance suite (`acceptance_manifest.json` + 40 `.omni` goldens) plus the listed design docs | **true** with a nit | 40 golden *files* exist. The manifest has 24 positive + 15 negative = **39** entries; `neg_circular_include.omni` is on disk but not in the manifest. Design docs all exist. |
| `spec.md` is the normative specification (`SPEC.md`) | **true** | `spec.md:1`. Counts inside it (`378` / `339` / `340`) are stale on this branch. |
| `omni_full_status_audit.md` and `omni_progress_checklist.md` are a full status audit and Phase 1–7 checklist | **true** that the files exist; their **contents** overclaim completeness (see below). |

### Quick Start (lines 31–73)

| Claim | Verdict | Evidence |
|---|---|---|
| Headings advertise `339`/`340` and `378` checks | **stale / false** | Same arithmetic as above; should read 378 workspace / 416 monolith on this branch, after a confirming selftest run. |
| Commands `selftest`, `pure-omni-diff`, `pure-bootstrap`, `acceptance-diff`, `inspect`, `--explain`, harness, agent-driver, `eval`, `serve`, `repl` exist | **true** | Wired in `omni-cli/src/main.rs` and the two extra bins. |
| All Quick Start samples are Windows PowerShell (`$env:CARGO_INCREMENTAL`, `omni.exe`) | **overstated** as the only path | CI and the monolith are documented elsewhere with POSIX commands (`.github/workflows/ci.yml:21-23`). A Linux/macOS snippet should sit beside the PowerShell one. This is a docs gap, not a functional lie. |

---

## Other top-level docs

These are not the README, but they are what a new reader (or a later agent) will treat as status, and they currently amplify the same overclaims.

### `spec/PURE_SELF_HOST_ROADMAP.md`

- Header (`:3`) says "Phases A–E … Complete & Verified (`378` / `339` / `340`)" — **stale counts**, and "complete" is **overstated** in the self-hosting sense.
- The table marks Phase A–E and Pure-Core 1–5 as **Complete & Verified**. Each "implementation" is a kernel + Rust wrapper. **Overstated.**
- The note at lines 42–43 is the most honest paragraph in the tree and should be promoted into the README. **true**, and it contradicts the README's "by default" sentence.
- Locked hashes and the 14,559-word suite size are **true as locks**, **overstated as self-host proofs**.
- The Stage-1 == Stage-2 row (`19,731` words / `106,242` bytes) refers to the Rust-generated compiler stream, which is a stronger artifact than the 159-word kernel. Keep it, but name it accurately.

### `spec.md`

- Opening check counts (`:3`) are **stale**.
- Section 6 AOT-rejection policy is **true in intent**; some opcode *names* in the Section 6 table are **stale** versus `OMNI_ISA_TABLE` (see `0x7F60..=0x7F6B`).
- Section 7.9 two-mode host ABI is **true**.
- Phase D / S4 word-count and hash statements (`:191, :194`) have the same overstatement as the README.

### `omni_full_status_audit.md` and `omni_progress_checklist.md`

- Useful as a historical log of what was *added*.
- Rows 36–41 and the "routes `run`/`eval`/`repl`/`serve` through Pure-Omni by default" language (`omni_full_status_audit.md:124, 129`) are **overstated** in the same way as README line 10.
- Check counts and the older 6,115-word 9-module hash are **stale** relative to the 14,559-word 18-module suite (the audit still mentions both in different rows).

### `OMNI_PURE_SELF_HOST_INSTRUCTIONS.md` and `OMNI_IMPROVEMENT_INSTRUCTIONS.md`

- These are implementation briefs, not status reports. Counts (`333` monolith / `294–295` workspace, line 17 of the self-host instructions) are **stale**.
- "Phases 1–6 complete" is a process claim; treat as historical, not as proof that Phase 7 self-hosting is done.

### `LICENSE`

- **true:** MIT, Copyright (c) 2026 Caleb Peacock.

---

## What is actually true and should be kept

- Zero external crates.
- 128-opcode `0x7F00..=0x7F7F` ISA, `@PACK5`, capability / linear / taint verifier in Rust with `error[<CODE>]` and source spans.
- Working VM, two-mode host (in-VM simulation + live `omni serve` over a 10-syscall `OmniNarrowHost`), root-jail `PATH_ESCAPE` tests.
- Rust x86-64 ELF64 and Wasm AOT for a documented opcode subset; host opcodes rejected; Stage-1 compiler *stream* is a real (limited) u16-to-ELF64 emitter with a locked size.
- 18 small but real `.omni` modules and programs, compiled and executed through the Rust pipeline, with a locked combined token-stream size of 14,559 words.
- `pure/verifier_engine.omni` is a real 1,034-line, 58-function, loop-only stream-walking verifier with a 77-case differential against the Rust twin. It is the actual start of replacing Rust, and the README should say so clearly — including that it is **not** the default path yet.
- Hard language invariants (192 SSA registers, 16 labels per function, 64-deep calls, eager evaluation, no heap structs/maps/GC, no recursion in core `.omni`) are encoded in the ISA/VM (`REG_BASE..=REG_MAX`, `LBL_BASE..=LBL_MAX`, `MAX_VM_CALL_DEPTH`) and must stay documented as design, not as temporary gaps.

---

## Recommended README changes (implemented in the draft)

1. Lead with "Stage-0 is Rust; Stage-1 Omni is partial."
2. Replace "workflows route through Pure-Omni by default" with a table of *who actually runs each stage*.
3. Name `pure/verifier_engine.omni` as the first real port, and the Phase 7 / Phase A–D kernels as decision oracles, not compilers.
4. Update check counts to 416 / 378 (after a confirming `selftest` run) and mention the 77-case verifier differential separately, not as a single `checks += 1` footnote only.
5. Document the Wasm `IGET`/`ISET` content gap and the FS/DB sandbox.
6. Add a POSIX Quick Start next to PowerShell.
7. Add a **Roadmap** that matches the real remaining work (lockstep verifier, multi-byte loads on existing `IGET`/`ISET` shapes, honest docs, then IR / `@PACK5` / surface frontend ports, then a true two-pass Omni emitter).
