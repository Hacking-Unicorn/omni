# Omni Port Map

**Repo:** `github.com/Hacking-Unicorn/omni`  
**Branch:** `origin/isa/multibyte-loads` (`8160842`)  
**Clone used:** `/workspace/audit-readme` (read-only; `/workspace/omni` was not touched)  
**Goal:** replace Rust with Omni until Rust is only a thin Stage-0 seed.  
**Invariants that block or shape every port:** 192 SSA registers and 16 labels per function, 64-deep calls, eager evaluation, no heap structs/maps/GC, no recursion in core `.omni`, zero external crates, never overclaim coverage.

Line counts are `wc -l` on this commit. "Coverage" means how much of the *Rust job* an existing `pure/*.omni` or `std/*.omni` file already performs — not whether a similarly named kernel exists.

---

## Snapshot

| Tree | Lines | Role |
|---|---:|---|
| `omni_rust_bootstrap.rs` (monolith) | 18,379 | Full Stage-0 reference. Duplicates the workspace. Still required to build and to run `rustc -O` selftests. |
| `omni-workspace/` Rust sources | ~16,733 (crates only; 35,122 including the `#[path]` include of the monolith from `omni-cli`) | Production 8-crate split. |
| `pure/*.omni` + `pure/programs/*.omni` | 1,719 | Guest modules. One real port (`verifier_engine.omni`, 1,034). The rest are small kernels or demos. |
| `std/*.omni` | 286 | Real but small standard library. |
| `spec/pure_self_host/*.omni` | 96 | Even smaller copies of the Phase A–D kernels. |

The Stage-0 seed is **not thin** today. Most of the 18k-line monolith and the workspace crates must keep working until their Omni twins are default and lockstep-clean.

---

## Coverage scale

- **none** — no Omni code does this job.
- **kernel-only** — a `.omni` file with the right name maps pre-digested `i64` flags to error codes or packed statuses. Rust still performs the analysis.
- **partial** — some of the real algorithm exists in Omni (often as a byte-level helper or a stdlib routine), but the production path is Rust.
- **substantial** — a real Omni implementation exists and matches the Rust twin on a differential suite, but is not the default path.
- **complete** — Omni is the production implementation. (Nothing in the core compiler/verifier/host stack is here yet.)

---

## `omni-lang` modules

### `compiler/` — `omni-workspace/crates/omni-lang/src/compiler/mod.rs` (3,901 lines, 68 public fns)

**Function groups**

| Group | Examples (line) | Approx. share |
|---|---|---|
| Lexer | `tokenize_omni_surface_inner` (132), `tokenize_omni_surface` (327) | large |
| Parser | `new`/`peek`/`parse_module`/`parse_fn_def`/`parse_stmt`/`parse_expr` and the precedence climb (526–1513) | large |
| Lowering to `u16` | `emit`, `alloc_reg`, `alloc_lbl`, `lower_expr_into`, `lower_stmt`, `lower_call_rhs`, `lower_trit_vec_pack5` (1565–2241) | large |
| Includes, macros, stdlib embedding | `check_macro_body_hygiene` (352), `resolve_omni_surface_includes` (2983), `lookup_omni_stdlib_module` (2423) | medium |
| Public compile/execute API | `compile_omni_surface_to_u16*` (3199–3360), `compile_omni_surface_file_to_u16` (3394), `execute_omni_surface_program*` (3545–3764), AOT/Wasm/Stage-1 wrappers | medium |
| `if_pos` join-slot regression | `run_if_pos_join_slot_selftests` (3797) — 33 checks across VM/Wasm/x86-64/Stage-1 | small |

**Existing Omni:** `pure/compiler.omni` (117 lines), `spec/pure_self_host/frontend.omni` (16 lines), `spec/pure_self_host/omni_selfhost_compiler.omni` (34 lines). All **kernel-only**. They classify `(src_len, flags)` or `(src_len, missing_cap, taint)`; they do not lex, parse, or emit a token stream. `compile_omni_surface_via_pure_omni_frontend_with_sourcemap` (`pure_omni/mod.rs:61-86`) runs the kernel on `(src.len(), 0, 0)` and then calls this Rust compiler.

**Must it stay in the Stage-0 seed?** **Yes, until a true Omni frontend exists.** This is the production compiler. After a real port, a thin Rust driver can still invoke it (read a file, hand bytes to the Omni compiler, write the result).

**Blockers**

- A real AST in Omni cannot use heap structs or maps. It has to live in arenas / byte buffers (this is why multi-byte `IGET`/`ISET` shapes matter).
- Lexer and parser must be loop-only, split into many functions so each stays under 192 SSA / 16 labels.
- Eager evaluation is fine for pure functions; short-circuit is not available and must not be added as a "gap fix."
- Tokenizing must eventually happen *in Omni*, or any later `stage2 == stage3` fixed point still starts after a Rust lexer.
- `SurfaceSourceMap` and `error[<CODE>]` span provenance have to be preserved; the Phase A kernel hard-codes `line 1, col 1` on empty input.

**Port order:** last of the recommended next three, but it is the self-hosting bottleneck. Do not start it until the decoder and `@PACK5` helpers are in place, and preferably until multi-byte loads exist so the frontend does not burn SSA on byte-at-a-time reconstruction.

---

### `ternary/` — `.../ternary/mod.rs` (14 lines, 0 public fns of its own)

**Function groups:** re-exports of `omni-isa` `@PACK5` helpers (`pack5_encode`, `pack5_decode`, `pack5_tdot_direct`, …). Comment at lines 1–8 states that trit-literal lowering stays in `compiler` on purpose.

**Existing Omni:** `std/pack5.omni` (43), `std/trit.omni` (34) — **partial** (surface helpers, not the zero-alloc codec itself).

**Stay in seed?** The re-export module can vanish once Omni has the codec. The Rust codec in `omni-isa` stays as the reference until then.

**Blockers:** none unusual. Good early port, folded into the `omni-isa` recommendation below.

---

### `verifier/` — `.../verifier/mod.rs` (721 lines, 9 public fns)

**Function groups**

| Group | Examples | Coverage |
|---|---|---|
| Phase 7 summary kernel wrapper | `build_phase7_pure_omni_verifier_kernel_stream` (55), `verify_module_via_phase7_pure_omni_kernel` (71), `run_phase7_pure_omni_selftests` (177) | kernel-only (`pure/verifier.omni`, 51 lines) |
| Omni engine driver + differential | `verify_u16_stream_via_omni_engine` (472), `compare_rust_and_omni_verifier` (500), `run_verifier_engine_diff_selftests` (566), error taxonomy (294–415) | **substantial** (`pure/verifier_engine.omni`, 1,034 lines, 58 fns, no recursive call pairs) |

The **production** verifier algorithm is not in this file; it lives in `crates/omni-verifier` (316 lines: `verify_binary_function`, `verify_u16_stream`). This module is the Omni bridge.

**Stay in seed?** The Rust reference twin **should stay in the seed** for differential testing even after Omni becomes default. The Phase 7 wrapper can shrink once the engine is default. The driver that loads `verifier_engine.omni` stays in the seed until the compiler is self-hosted (it is a host loader, not verifier logic).

**Blockers already handled:** recursion in operand parsing was flattened to a loop on this port branch. Remaining: SSA pressure from byte-at-a-time `load_u16` / `load_i64le` (`verifier_engine.omni:72-96`); lockstep is not wired into compile/run/serve yet; engine is excluded from the 14,559-word suite hash.

**Port order:** in progress. Next work on this module is integration (lockstep), not a new port.

---

### `host/` — `.../host/mod.rs` (1,119 lines, 24 public fns)

**Function groups**

| Group | Examples (line) |
|---|---|
| Narrow host trait + cap check | `OmniNarrowHost` (37), `check_narrow_host_cap` (25) — 10 syscalls: tcp listen/accept/read/write/close, file open/read/write/close, time_ns |
| Sandbox | `resolve_sandboxed_host_path` (54), `with_sandbox_root` (160), `run_host_sandbox_selftests` (1002, 4 checks, `PATH_ESCAPE`) |
| Std and simulated adapters | `StdNarrowHostAdapter`, `SimulatedNarrowHostAdapter` |
| HTTP/1.1 + route + KV | `serve_omni_http_via_narrow_host` (502), `extract_omni_routes` (606), `handle_omni_http_request` (680), `serve_omni_http_batch*` / `serve_omni_http_once` (785–905), `run_omni_serve_cli` (929), KV load/save |

**Existing Omni:** `pure/host_runtime.omni` (70 lines) — **kernel-only** (cap mask, taint flag, `status * 1e6 + payload`, a fake `kv_apply` that does not touch a store). `std/http.omni` (33) — **partial** packing helpers.

**Stay in seed?** **The ten syscalls must stay in the Stage-0 seed.** That is the trust boundary (`spec/host_interface.md`). Route matching, HTTP framing, KV policy, and cap/taint checks *can* move to Omni later. Do not invent a wider syscall surface.

**Blockers:** TCP and filesystem cannot be expressed in pure Omni. HTTP parsing in Omni will need byte-buffer loads and will bump into SSA/label caps unless it is split. Sandbox canonicalization uses `std::path` and should stay next to the syscalls.

**Port order:** after the frontend can express the route/HTTP logic; syscalls never leave Rust.

---

### `repair_agent/` — `.../repair_agent/mod.rs` (1,456 lines, 13 public fns)

**Function groups:** strategy catalog (`list_omni_repair_strategies` 35, `omni_repair_strategies_json` 153, `select_omni_repair_strategy` 177); capability inference (270); include-graph repair (330); source-to-source repair from harness errors (424); agent loop + CLI (`run_omni_agent_repair_loop*` 1022–1276), prompt JSON (1187).

**Existing Omni:** `pure/tooling.omni` `pure_select_repair_strategy` / `pure_multi_step_repair_budget` (lines 40–99) — **kernel-only** (maps error ids and budgets to integers). The actual edits are Rust string rewriting.

**Stay in seed?** **Yes for now**, and possibly for a long time. It is a host-side developer tool that rewrites UTF-8 source. It can stay in the seed after the compiler is self-hosted, the same way a rustc-adjacent rustfmt might. Porting it is not on the self-host critical path.

**Blockers:** heavy string/source-map mutation; no Omni string library beyond bytes; 16-label/192-SSA caps make a multi-strategy rewriter awkward without an arena text model.

**Port order:** late, optional.

---

### `harness/` — `.../harness/mod.rs` (276 lines, 7 public fns)

**Function groups:** error-code mapping and retry hints (34–56), `parse_omni_structured_error` (78), JSON helpers, `omni_harness_spec_json` (184), `run_omni_harness_json` (193). (`run_omni_harness_cli` actually lives in `pure_omni/mod.rs:2763`.)

**Existing Omni:** none for JSON I/O. `pure/tooling.omni` `pure_explain_error_stage` is **kernel-only**.

**Stay in seed?** **Yes.** It is a host-facing JSON contract (`omni-harness-v1.0.0`) over stdin/stdout. Keep it in Rust even after the compiler moves.

**Blockers:** JSON serialization, CLI argv, process exit codes — all host concerns.

---

### `inspect/` — `.../inspect/mod.rs` (376 lines, 9 public fns)

**Function groups:** cap/type pretty-printing, `format_omni_inspect` (100), `inspect_omni_surface_program` (181), `inspect_omni_surface_via_pure_omni` (187) (Rust inspect after the "pure frontend" wrapper), human diagnostics and `--explain` (194–206), CLI (320).

**Existing Omni:** `pure/tooling.omni` `pure_inspect_opcode_category` / `pure_inspect_module_metrics` — **kernel-only** / **partial** (category and a numeric digest, not a full disassembly).

**Stay in seed?** Diagnostics can stay in the seed. A Pure-Omni disassembler is nice-to-have after the decoder port, not blocking.

**Blockers:** formatting strings; source-map join is Rust.

---

### `acceptance/` — `.../acceptance/mod.rs` (369 lines, 5 public fns)

**Function groups:** `run_error_contract_selftest` (28), `run_omni_acceptance_diff` (82), `run_acceptance_suite_selftest` (203), CLI (227), `run_compositional_invariant_selftests` (254).

**Existing Omni:** `pure/tooling.omni` `pure_acceptance_diff_check` — **kernel-only** (compares two word counts and a hash diff already computed in Rust).

**Stay in seed?** **Yes.** This is the Stage-0 test harness that keeps the port honest. It should remain Rust (or at least host-side) so it can load both twins.

**Blockers:** filesystem, JSON manifest (`24` positive + `15` negative goldens in-manifest; `neg_circular_include.omni` is on disk but not listed), process-level diffs.

---

### `pure_omni/` — `.../pure_omni/mod.rs` (2,869 lines, 34 public fns)

**Function groups:** Phase A–E wrappers and selftests; `build_pure_omni_core_suite_stream` / suite word-count lock (14,559); Strengthening S1–S4 program and metrics selftests; `run_ai_first_surface_selftests` (2053) which also runs sandbox, verifier-engine, and `if_pos` suites; `run_omni_harness_cli` (2763).

**Existing Omni:** this module is the Rust *host* for the `pure/` and `std/` files. It is not a port target so much as scaffolding that should shrink as real Omni modules take over.

**Stay in seed?** The loader/selftest scaffolding stays until the suites move. A lot of this file is the overclaim surface (names that say "frontend/compiler/host" while calling Rust). Honesty work here is documentation and, later, deletion of redundant kernels.

**Blockers:** none for porting — the work is to stop treating this file as if it were the Omni compiler.

---

### `omni-lang` crate root — `src/lib.rs` (26 lines)

Re-exports the modules above. No port work.

---

## Other workspace crates

### `omni-isa` — `crates/omni-isa/src/lib.rs` (457 lines, 16 public fns)

**Function groups:** token partition constants (192 regs `0x7E00..=0x7EBF`, 32 params, 16 labels `0x7EF0..=0x7EFF`, opcodes `0x7F00..=0x7F7F`, types `0x7F80+`); `OMNI_ISA_TABLE: [OpcodeSpec; 128]`; `decode_opcode_token`; AOT support predicates (253–285); ternary algebra (`tnot`/`tand`/`tor`/`txor`/`tmaj`); `@PACK5` codec (`pack5_encode`/`decode`/`get_trit`/`set_trit`/`tdot`/`sum`/`gate`).

**Existing Omni:** `std/pack5.omni`, `std/trit.omni`, `pure/tooling.omni` `pure_inspect_opcode_category` — **partial**. The 128-row table and the zero-alloc codec are still Rust.

**Stay in seed?** A frozen table can be data in Omni (a byte/u16 blob in an arena). The Rust table should remain the reference twin. AOT predicates can move with the AOT port.

**Blockers:** low. The table is static data. Codec is loop-friendly. Must keep the `0..=242` valid-lane rule and the locked golden token counts unchanged.

**Port order:** **recommended #2 after the verifier.**

---

### `omni-ir` — `crates/omni-ir/src/lib.rs` (303 lines, 6 public fns)

**Function groups:** `BinOperand` / `BinInstruction` / `BinFunction` / `BinModule` types; `decode_omni_token_stream` (121); encode/decode bytes; `pack_fn_module`; debug formatting.

**Existing Omni:** `pure/verifier_engine.omni` already decodes a little-endian u16 stream from `bytes` (`load_u16`, `parse_operand`, `parse_operand_raw`, `parse_eng`) — **partial / substantial** for decode, none for a shared public IR module. The engine's decode is inlined for verification, not exposed as a standalone decoder.

**Stay in seed?** The Rust decoder stays as the reference twin. Once Omni can decode, later ports (verifier default, assembler inverse, compiler backend) should share one Omni decoder.

**Blockers:** `BinModule` is a heap of structs and `Vec`s. An Omni twin must encode the same information in arenas/buffers (function table, operand packs). The engine already uses a workspace buffer at documented offsets (`verifier_engine.omni:26-33`). That layout should be the starting point, not a parallel one. Multi-byte loads would shrink it.

**Port order:** **recommended #1 after the verifier** — extract and share the engine's decoder rather than writing a second one.

---

### `omni-verifier` — `crates/omni-verifier/src/lib.rs` (316 lines, 5 public fns)

**Function groups:** `RegState` / `Taint` / `RegSlot`; `verify_binary_function` (75), `verify_u16_stream` (273), `token_to_slot`, `format_debug_lens`. This is the production O(n) reference.

**Existing Omni:** `pure/verifier_engine.omni` — **substantial** (77/77 differential). `pure/verifier.omni` — kernel-only, not a substitute.

**Stay in seed?** **Yes, as the reference twin**, even after Omni is default.

**Port order:** algorithm already ported. Remaining work is lockstep + default flip, tracked under `omni-lang/verifier`.

---

### `omni-vm` — `crates/omni-vm/src/lib.rs` (698 lines, 8 public fns)

**Function groups:** `OmniValue` (including a Rust `Struct` variant that core `.omni` must not grow into a language feature); `OmniBinaryVm` (`execute_function` 125) with tables, KV, budgets, `MAX_VM_CALL_DEPTH = 64`.

**Existing Omni:** none. An Omni VM running on the Omni VM is not a bootstrap strategy.

**Stay in seed?** **Yes, permanently until AOT/Stage-1 can host the compiler and verifier.** This is seed runtime, not a port target for self-hosting.

**Blockers:** circularity. Also `OmniValue::Struct` / `HashMap` tables are host representations.

**Port order:** do not port. If a second implementation is ever needed, write it in Rust or as native AOT, not as a guest interpreter.

---

### `omni-assembler` — `crates/omni-assembler/src/lib.rs` (328 lines, 5 public fns)

**Function groups:** `encode_asm_ins_packet_u16` (27), `assemble_omni_text_to_u16` (217), plus two custom program builders used in tests.

**Existing Omni:** none.

**Stay in seed?** The Rust assembler stays as a twin. A Pure-Omni encoder is needed for Milestone 2 (the Omni compiler must emit packets).

**Blockers:** text assembler needs a lexer (overlaps the surface frontend). The *binary* packet encoder is the smaller, higher-value slice and is loop-friendly.

**Port order:** strong candidate immediately after ISA/decoder, or as a prelude to the frontend. Not in the top-3 list only because the decoder and `@PACK5` already have Omni footholds and unblock more modules; the encoder should follow close behind.

---

### `omni-aot` — `crates/omni-aot/src/lib.rs` (2,199 lines, 9 public fns)

**Function groups:** `is_module_aot_hot_path` (102); x86-64 ELF64 emitter (`compile_pure_u16_to_x86_64_elf_with_args` 147); Wasm emitter (`compile_pure_u16_to_wasm` 694) and budgeted evaluator (`execute_wasm_module_i64_with_budgets` 1287); **Stage-1 stream builder** `build_step7_stage1_compiler_stream` (1590, locked 19,731 words); `fnv1a_64_hash`.

**Existing Omni:** no surface-language emitter. The Stage-1 stream *is* an Omni-ISA program, but it is generated by Rust macros/helpers, not written as `pure/compiler.omni`. `pure/compiler.omni` remains a stub relative to Milestone 2.

**Stay in seed?** The Rust emitters stay as reference twins. Wasm/x86 host loaders stay in the seed. The Stage-1 stream is the current stand-in for an Omni emitter and should eventually be *replaced* (not merely wrapped) by a true two-pass Omni compiler.

**Blockers:** known Milestone 2 issues from earlier review (tokenizer gap; pass-1/pass-2 size mismatches; `SYSCALL_EXIT` size vs emit; missing bounds on label/register ids; `MOV_IMM` 32-bit immediate; opcode table fidelity). Wasm `IGET`/`ISET` content gap (`lib.rs:876-944`). SSA/label caps will shape a handwritten Omni emitter the same way they shaped `verifier_engine.omni`.

**Port order:** Milestone 2, after the frontend can at least emit the subset the Stage-1 stream already handles. Do not advertise the 159-word Phase D kernel as this work.

---

### `omni-cli` — `src/main.rs` (1,287) + `src/lib.rs` (8) + `bin/omni-harness.rs` (4) + `bin/omni-agent-driver.rs` (6)

**Function groups:** `run_workspace_selftest` (159) — monolith-vs-workspace differentials; CLI dispatch (`selftest`, `run`, `eval`, `repl`, `serve`, `compile-*`, `inspect`, `acceptance-diff`, `pure-bootstrap`, `pure-omni-diff`, …); `#[path = "../../../../omni_rust_bootstrap.rs"]` include of the monolith (main.rs:37) for clone comparison; `#[test]` wrappers (selftest lower bounds, pure-bootstrap, pure-omni-diff, verifier-engine).

**Existing Omni:** none. Guest programs are loaded *by* the CLI.

**Stay in seed?** **Yes.** Argv, filesystem, process, and CI entry are Stage-0 by definition.

**Blockers:** none; not a port target.

---

## `pure/` and `std/` inventory (Omni side)

| File | Lines | What it really does | Covers which Rust |
|---|---:|---|---|
| `pure/verifier_engine.omni` | 1,034 (58 fns) | Walks a u16 stream; cap, taint, linear, borrow, SSA, labels. Loop-only. | Substantial twin of `omni-verifier`. Not default. |
| `pure/compiler.omni` | 117 | Flag → frontend/verifier error codes. | Kernel-only vs `compiler/`. |
| `pure/verifier.omni` | 51 | Three integers → `1001..=1005`. | Kernel-only vs Phase 7 wrapper. |
| `pure/host_runtime.omni` | 70 | Cap/taint/status packing; stub KV. | Kernel-only vs `host/`. |
| `pure/tooling.omni` | 99 | Diff/repair-id/category/stage/fixed-point/metrics/budget kernels. | Kernel-only vs inspect, repair, acceptance. |
| `pure/selfhost_suite.omni` | 65 | Orchestrates the above kernels; fixed-point *compare*. | Does not compile a compiler. |
| `pure/programs/*.omni` (6) | 37–59 each | Real demos (PACK5 router, arenas, routes, consensus, self-audit, pack5 controller). | Tests, not ports. |
| `std/prelude.omni` | 34 | Prelude includes / thin wrappers. | Embedded by Rust `lookup_omni_stdlib_module`. |
| `std/math.omni` | 47 | Clamp and arithmetic helpers. | Small slice of lowering/stdlib. |
| `std/trit.omni` | 34 | Ternary helpers. | Partial vs `omni-isa` algebra. |
| `std/http.omni` | 33 | Response packing. | Partial vs host framing. |
| `std/pack5.omni` | 43 | Surface `@PACK5` helpers. | Partial vs `omni-isa` codec. |
| `std/decision.omni` | 57 | Decision-hook helpers. | Partial vs `EGATE`/`TLOGIC` surface. |
| `std/resource.omni` | 38 | Budget/resource helpers. | Partial vs budget attributes. |
| `spec/pure_self_host/*.omni` | 16–34 | Copies of Phase A–D kernels. | Duplicate of the kernels above. |

---

## What must remain in the Stage-0 Rust seed (even after a successful port)

1. **Process entry:** `omnic` / monolith `main`, argv, filesystem, CI.
2. **`OmniNarrowHost` syscalls:** TCP, file, time (and the path sandbox).
3. **The VM and/or a native loader** sufficient to run Stage-1 until AOT can host the compiler.
4. **Reference twins:** Rust verifier (and, as they are ported, decoder / `@PACK5` / assembler / compiler) for lockstep differentials.
5. **Acceptance, harness, and agent-driver JSON contracts** — host tools.
6. **`rustc` itself** as Gen-0, until Omni compiles Omni from source end-to-end.

Everything else in `compiler/`, `omni-ir`, `omni-isa`, `omni-assembler`, and eventually `omni-aot` is on the path to leave the seed or shrink to a twin.

---

## Dependency order for porting

```
omni-isa constants + @PACK5 codec     (data + loops)
        \ 
         \-> omni-ir token-stream decoder  <— already started in verifier_engine.omni
                  \
                   +--> verifier_engine default + lockstep   (in progress)
                   +--> assembler packet encoder
                          \
                           +--> surface frontend (lex / parse / lower)
                                  \
                                   +--> host policy (HTTP/KV) on top of Rust syscalls
                                    \-> two-pass Omni ELF64 emitter (Milestone 2)
                                         \
                                          +--> thin the Stage-0 seed
```

Do not port the VM into Omni. Do not port the CLI. Do not raise the SSA/label/call limits to make the frontend fit in one function.

---

## Recommended next three modules after the verifier

These assume the verifier engine stays on its current path (lockstep, then default) and that multi-byte `IGET`/`ISET` *shapes* land without new opcodes, which unblocks all three.

### 1. Token-stream decoder (`omni-ir` / extract from `pure/verifier_engine.omni`)

- Smallest *real* next step: the engine already parses operands and module structure from `bytes`.
- Unblocks a shared guest IR for the default verifier, the assembler inverse, and the frontend's emitter checks.
- Stay inside the engine's workspace-buffer discipline; do not introduce heap structs.
- Rust `decode_omni_token_stream` remains the twin; add differentials like the 77 verifier cases.
- Blockers: arena layout only; SSA pressure until multi-byte loads exist (the engine's `load_u16` already does two `iget`s per word).

### 2. `@PACK5` and ISA helpers (`omni-isa`, building on `std/pack5.omni` and `std/trit.omni`)

- 457 lines, loop-shaped, no recursion, no heap.
- Put the 128-opcode table in a frozen u16 blob the guest can `iget`.
- Needed by the decoder (opcode properties), the verifier (cap/sink/sanitizer bits — already duplicated as `op_req_cap_*` in the engine), and the frontend (trit-vector literals).
- Keep golden `@PACK5` token counts and the `0..=242` lane rule unchanged.
- `omni-lang/ternary` is only a re-export; it disappears when this lands.

### 3. Surface frontend (`omni-lang` `compiler/` lexer, parser, lowering)

- This is the actual self-hosting bottleneck. `pure/compiler.omni` does not do this job.
- Split into many small loop-only functions (lex, parse-expr, lower-stmt, …) that respect 192 SSA, 16 labels, and 64-deep calls.
- Represent the AST in arenas/byte buffers, not structs or maps.
- Preserve `error[<CODE>]` and 1-based spans; do not fall back to Phase A's `line 1, col 1`.
- Keep the Rust compiler as the default and as a bit-exact twin (`Vec<u16>` + span parity on the acceptance goldens) until the Omni frontend is boringly identical.
- The packet encoder in `omni-assembler` should be ported alongside or immediately after the first lowering pass; it is 328 lines and is what the frontend emits.

**Why not host, repair, or AOT as #3?** Host syscalls cannot leave Rust; repair is a host tool; AOT is Milestone 2 and depends on a real frontend and encoder. Assembler-only as #3 would also be defensible if the frontend looks too large in one step — in that case do `encode_asm_ins_packet_u16` first, then start the lexer.

---

## Suggested rule for every later port

Start from the existing `pure/` or `std/` file with the same name, and extend it until it does the Rust job. Do not add a parallel `*_engine.omni` next to a kernel unless the kernel is left as a tiny oracle (as with `verifier.omni` vs `verifier_engine.omni`, which was the right split because the 51-line kernel is in the frozen 18-module hash suite). Prefer extending `pure/compiler.omni` in place, or keep the kernel and add a clearly named `pure/frontend_engine.omni` if the 14,559-word lock must stay untouched — and document that split in the README so it is not advertised as a finished compiler.
