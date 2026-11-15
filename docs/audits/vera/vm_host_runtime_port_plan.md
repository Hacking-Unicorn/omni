# Port plan: VM + host runtime (`omnic run`, `serve`, HTTP, DB/KV, Repair Agent) from Rust to iterative Omni

*Vera Parity. This is a design only; it contains no code. **v2, 2026-11-14 (America/New_York):** arena re-laid into Ember's approved band `0x100000-0x2FFFFF` (§4.1), and updated for the decisions recorded in `HANDOFF.md` §6 on `docs/handoff` (summary of changes in §8). Line cites are unchanged from v1. Base: `origin/isa/multibyte-loads` `d2d6c13`. The pushed harness branch `vera/parity-goldens` `d257cb0` changes only lines ≥ ~12480 plus the CLI table. Every cite below is `omni_rust_bootstrap.rs` at `d2d6c13` unless another file is named. The cited regions are byte-identical at `d257cb0`, except the Wasm executor and `main`, which shift by ~+846/+870 there.*

## 0. Summary

1. **Target.** Today the VM and host runtime are about 4,700 lines of Rust in the monolith, plus ~3,300 in workspace mirrors. Most of it moves into **new** `pure/vm_*.omni` and `pure/host_*.omni` modules. Each module is iterative and arena-resident, and runs natively via the x86/Stage-1 backends.
2. **What stays in Stage-0.** Rust keeps a thin seed: the u16 loader/decoder, a minimal bootstrap VM core big enough to run the Stage-1 compiler stream and the new Omni modules, and a 10-call syscall shim (`OmniNarrowHost`).
3. **How it lands.** About 14 small commits. Each one adds the Omni twin of exactly one Rust subsystem and gates it with diff4. The harness gains a 5th backend, `omni_vm` (the Omni interpreter compiled to a native ELF), and every case must agree byte-for-byte with `vm`. Strict xfail keeps any remaining gap visible.
4. **Invariants and hashes.** 192 SSA / 16 labels per fn (decided; the surface frontend still allows only 15 until main-line C4), 64-deep calls, eager args, no heap structs/maps/GC, flat arena, no recursion, the opcode table untouched. No new module enters `PURE_OMNI_CORE_SUITE_MODULES` or the embedded stdlib literals before the single planned re-freeze after Nyx's M2 fixed point, so **no frozen hash moves**.
5. **Fix first.** Five bugs. **Main line owns** the VM negative-patch panic and `omnic run` buffer programs going through Wasm (`two_bufs` 11, `pack5_flow` 2009000); Ember does not touch those. **Ember owns** the `DB_QRY`/`DB_GET` miss value (-1). Still open for whoever takes them: the AOT-first "vm" mislabels, and the Stage-1 DROP/IMOD crash and stale-value bugs (C2, main line). The Omni modules will be compiled through the Stage-1 emitter, so the last one hits them directly.
6. **Arena.** Everything the host runtime allocates lives in `0x100000-0x2FFFFF` (2 MiB, §4.1), clear of M2 (`< 0x80000`) and M3 (`0x80000-0xFFFFF`).

## 1. Rust inventory (lines at `d2d6c13`)

### 1.1 Monolith `omni_rust_bootstrap.rs` (18,958 lines)

| # | Subsystem | file:line | Lines | Port? | Notes |
|---|---|---|---:|---|---|
| A | Token partition + 128-opcode table | 54-505 | 452 | **No** (seed + doc) | The table is never renumbered. Omni gets a generated read-only copy as data, not code. |
| B | Token-stream decoder `decode_omni_token_stream` | 506-740 | 235 | Seed keeps it; Omni twin | `pure/verifier_engine.omni` (1,034 lines) already decodes in Omni. Reuse its decode loop. |
| C | Binary verifier `verify_binary_function` / `verify_u16_stream` | 741-1009 | 269 | Already twinned | The Omni engine plus lockstep exist (11456-11755, 11756+). Out of scope except as a dependency. |
| D | `OmniValue` enum | 1010-1022 | 13 | Re-design | 11 variants, including `Str(String)`, `Bytes(Vec)`, `Struct(BTreeMap)`, `DomNode{children: Vec}`. These become a tagged-slot arena layout (§4). |
| E | `OmniBinaryVm` struct + budgets | 1024-1039 | 16 | Port | `tables: HashMap<u16, Vec<BTreeMap>>`, `kv_store: BTreeMap`, `req_params: BTreeMap`. All become flat arena tables (§4). `MAX_VM_CALL_DEPTH = 64` (1026). |
| F | VM helpers (`charge_memory`, `eval_op/as_i64/as_trit`) | 1041-1122 | 82 | Port | `eval_as_trit` (1115-1121) is the spec clamp. N2/N3 parity depends on it. |
| G | `execute_function` (interpreter loop) | 1123-1679 | 557 | **Port (core of the work)** | Opcode `match` at 1171-1666. 77 opcodes have arms; 51 fall to `Void` at 1666. **CALL recurses** at 1471-1472 (`self.execute_function`). That must become an explicit frame stack. |
| G1 | of which ANEW/IGET/ISET/buffers | 1284-1354 | 71 | Port | N6 panic at **1324-1326**. |
| G2 | of which strings/sanitizers/FNV | 1355-1386, 1477-1486 | ~45 | Port | `SAN_PRM` is identity (1479). |
| G3 | of which trit/AI (`EGATE`/`TLOGIC`/`TDOT`/`pack5`) | 1391-1476 | ~85 | Port | |
| G4 | of which DB/KV/route/HTTP ops (0x7F63-0x7F6F) | 1487-1640 | ~155 | Port, **core on purpose** | `DB_GET/QRY` miss fallback `key*10` is at 1500-1518. **Decided: both return -1 on a miss** (Ember: `DB_QRY` local `7d55a81`, `DB_GET` next commit). |
| H | u16 ↔ bytes helpers | 1680-1694 | 15 | Seed | |
| I | Surface execution entry points (`execute_omni_surface_*`) | 5549-5796 | 248 | Port (driver logic) | AOT-first dispatch is 5602-5699. "aot" means Wasm. |
| J | Harness / structured diagnostics (§7C) | 5799-6055 | 257 | Port | JSON emission becomes byte writes into the arena. |
| K | Agent driver + **Repair Agent** (§7D) | 6056-7492 | 1,437 | Port, **core on purpose** | `run_omni_agent_repair_loop_with_model` 7055-7219, `repair_omni_program_from_harness_error` 6457-7054 (598 lines; largest single fn). |
| L | `OmniNarrowHost` trait (10 syscalls) | 7507-7518 | 12 | **Seed (syscall shim)** | tcp listen/accept/read/write/close, file open/read/write/close, time_ns. |
| L1 | Sandbox path policy | 7524-7609 | 86 | Split | Canonicalize/realpath stays in the shim. The `..`/NUL/absolute policy checks move to Omni and run twice (Omni and shim). |
| L2 | `StdNarrowHostAdapter` | 7610-7783 | 174 | **Seed** | Real OS calls. |
| L3 | `SimulatedNarrowHostAdapter` | 7784-7930 | 147 | Seed (test double) | Used by the parity gates. |
| M | KV over narrow host, `serve_omni_http_via_narrow_host` | 7931-8033 | 103 | Port | |
| N | Routes + HTTP: `extract_omni_routes`, `match_omni_route_pattern`, `handle_omni_http_request`, `OmniHttpResponse` | 8034-8228 | 195 | Port, **core on purpose** | |
| O | KV file load/save, `serve_omni_http_batch*`, `serve_once`, `run_omni_serve_cli` | 8229-8469 | 241 | Port (shim keeps the file bytes) | |
| P | Phase B pure driver: `execute_omni_surface_via_pure_omni_driver` (the `omnic run` path), `run_pure_omni_driver_json/cli` | 9605-9891 | 287 | Port | `run` CLI → 9628-9746. |
| Q | Phase C pure host: `serve_omni_http_via_pure_omni_host` | 9982-10098 | 117 | Port (it already wraps `pure/host_runtime.omni`) | |
| R | Wasm executor `execute_wasm_module_i64_with_budgets` | 15620-15931 | 312 | **Not in scope** | It is a test oracle. It stays in Rust as a differential reference. |
| S | CLI `main` (`run` 18764+, `eval` 18727, `repl` 18738) | 18526-18958 | 433 | Port the arg dispatch last | The seed keeps a ~40-line `main` that loads the Omni driver ELF/bc. |

**Port scope:** D–Q plus S, about 4,700 lines.
**Stays in Rust** (seed, test oracle, or already twinned): A, B, C, H, L, L2, L3, and R.

### 1.2 Workspace mirrors (`omni-workspace/crates`)

| Crate | Lines | Fate |
|---|---:|---|
| `omni-vm/src/lib.rs` | 698 | Same N6 pattern at `:310`. Retire after step 10. Keep compiling until then. |
| `omni-lang/src/host/mod.rs` | 1,119 | Retire after step 12. |
| `omni-lang/src/repair_agent/mod.rs` | 1,456 | Retire after step 13. |
| `omni-cli/src/main.rs` | 1,287 | Retire after step 14. |
| `omni-lang/src/pure_omni/mod.rs` | 2,875 | Driver glue. It shrinks as the steps land. |

Out of scope: `omni-aot` 2,397, `compiler` 4,057, `verifier` 963 (Nyx/M2 and verifier work).

### 1.3 Existing Omni to build on

- `pure/host_runtime.omni` (70 lines; cap/taint/KV/dispatch scalar kernels)
- `std/http.omni` (33)
- `pure/verifier_engine.omni` (1,034; a real token decoder and verifier in Omni, the best template)
- `pure/programs/kv_ledger_service.omni` (37)

Today these are **scalar kernels**: the Rust host does the real work and calls them for decisions. This plan replaces the Rust work itself.

## 2. Stage-0 seed: what stays in Rust

The seed has three parts:

1. **Loader.** Read a `.omni_bc` or ELF, decode u16 (B, H), and pass argv and stdin bytes into the arena. About 300 lines.
2. **Bootstrap VM core.** The subset of G needed to run the Stage-1 compiler stream and the new `pure/vm_*.omni` interpreter before it is compiled natively. Measured histogram of the Stage-1 stream (review §6): `CINT CTRIT ANEW IGET ISET IADD ISUB IMUL BOR SHL SHR CEQ CGT TAND TCMP LABEL JMP BR_POS SEL_TRIT RET SLEN`, plus `CALL` for multi-fn modules. That is about 25 opcodes and ~250 lines once the DB/HTTP/string/AI arms are removed. The full 128-opcode semantics move to Omni.
3. **Syscall shim.** `OmniNarrowHost` (L) plus `StdNarrowHostAdapter` (L2), with realpath/canonicalize from L1. These are the only places that touch the OS. Every call stays capability-gated (`check_narrow_host_cap` 7495).

That is about 900 lines of Rust in total. Everything else becomes Omni or test oracle (Wasm executor R, Rust VM kept **only as the diff4 reference** until retired).

## 3. Port order (small commits, each with a diff4 parity gate)

**Mechanism for every gate.** Add backend `omni_vm` to the diff4 harness. It is `pure/vm_*.omni` compiled to a native ELF through `compile_pure_u16_to_x86_64_elf`; it takes the program's u16 stream in its arena and runs it. A step is green only when:
- every existing diff4 case agrees with `vm` **byte-for-byte** (exit, stdout, buffer dump);
- the new per-step cases pass;
- `selftest` and `acceptance-diff` stay green;
- the five frozen hashes are recomputed unchanged (`fixed-point`, `pure-bootstrap`, and main's C1 pin).

Strict xfail (`OMNI_DIFF4_KNOWN_DIVERGENCES`) covers anything not yet ported, so each step shrinks the list.

| Step | New module (never edit `pure/compiler.omni`) | Ports | Parity gate (diff4 cases that must pass on `omni_vm`) |
|---|---|---|---|
| 0 | none (Rust test only) | Ember's fix-first bugs (§6.3) + `omni_vm` backend slot in the harness, xfail for every case. §6.1/§6.2 land on the main line, not here | Suite green with `omni_vm` all-xfail; hashes unchanged |
| 1 | `pure/vm_arena.omni` | Arena layout (§4.1, band `0x100000-0x2FFFFF`): register file 224×(tag, i64), bump heap, frame stack (64), label table. Offsets are compile-time constants from §4.1 | New unit cases: alloc/read/write round-trips, OOB → structured error code (not a crash) |
| 2 | `pure/vm_decode.omni` | u16 → flat instruction table (reuse the `verifier_engine.omni` decode loop as a template; new file) | Decoded table == Rust `decode_omni_token_stream` for all 39 acceptance goldens + the 64-corpus (dump compared byte-wise) |
| 3 | `pure/vm_core.omni` | Fetch/dispatch loop, CINT/CTRIT, IADD..IMAX, BAND..SHR, CEQ..CGT, TCMP, LABEL/JMP/BR_*, SEL_TRIT, RET, step budget | `p11_*` if_pos/while/match_trit, `p3_*` IMOD (VM semantics: 0), `p5_*` (clamp), `lbl15_*`, `lbl16_asm_*`, seeds 0/3/4/6 |
| 4 | `pure/vm_trit.omni` | TNOT..TMAJ with clamp (`eval_as_trit` 1115), EGATE/TLOGIC/TDOT/pack5 | `p4_trit_clamp`, `p11_pack5_*`, `p11_decision_hook` |
| 5 | `pure/vm_buf.omni` | ANEW/ISET (all modes incl. −4/−8/−9)/IGET/SLEN, per-buffer length | `p1_*` (8796075198218), `p6`, `p7` (31), `p9` (**decided: silent no-op, `slen` = 8**; diff4 expects that since `49091f2`), buffer dumps |
| 6 | `pure/vm_call.omni` | CALL via an explicit frame stack in the arena (**replaces Rust recursion 1471-1472**), depth 64 → `BUDGET_EXCEEDED` | Multi-fn goldens (`weighted_term`/main = 80), 64-deep chain OK, 65 → structured error |
| 7 | `pure/vm_arena_ops.omni` | ARENA/BORROW_R/W/MOVE/DROP (linear tracking; verifier already proves safety) | `p2_drop1/2/3` (49), `arena_lifecycle` (50) |
| 8 | `pure/vm_str.omni` | Strings as (off, len) in the heap: SLEN/SCAT/S2I/I2S/SAN_*/FNV1A | `str_pipeline_golden` (130), `san_dom` golden, FNV equality against the Rust `fnv1a_64_hash` |
| 9 | `pure/host_kv.omni` | KV/DB tables (DB_INS/GET/QRY/UPD/DEL/CNT, KV_PUT, TX_*) as flat arena tables (§4.1): KV as an arena sorted array with iterative binary search (approved); a miss on `DB_GET`/`DB_QRY` returns -1, `DB_UPD`/`DB_DEL` stay **no-ops** exactly as the Rust VM (they fall to `Void` today; real update/delete semantics are an open item for Caleb's hardening phase), an 8,192-key cap in both twins, KV file (de)serialization (bytes via the shim) | `route_db_query_golden` + **new P12 hit golden** (`200004242`); KV file round-trip == Rust `save_omni_kv_file` bytes |
| 10 | `pure/vm_driver.omni` | `omnic run` driver: verify → execute (no "AOT-first = Wasm"; Wasm stays an oracle only) | Every acceptance golden via `omni_vm`; then **switch `omnic run` to `omni_vm`**, with the Rust VM behind `--legacy-rust` for one release. Retire `omni-vm` crate. |
| 11 | `pure/host_http.omni` | HTTP/1.1 request parse, `extract_omni_routes`/`match_omni_route_pattern`/`handle_omni_http_request`, response framing (N, M) | Byte-identical HTTP responses vs Rust over `SimulatedNarrowHostAdapter` for every route golden + malformed-request corpus |
| 12 | `pure/host_serve.omni` | `serve` loop (accept/read/dispatch/write/close) over the shim; batch + `--db` file; sandbox policy checks (L1 policy half) | Serve transcript (bytes in/out per connection) identical to Rust; `PATH_ESCAPE` cases from `run_host_sandbox_selftests` still rejected. Retire `omni-lang/host`. |
| 13 | `pure/repair_agent_*.omni` (split ≥ 6 files: strategies, harness JSON, include graph, per-error repairers, loop, prompt JSON) | §7C harness + §7D Repair Agent (K, J) | `run_omni_agent_repair_loop` outputs byte-identical JSON vs Rust for the full strategy list (`list_omni_repair_strategies` 6068) + pure-omni-diff repair case. Retire `repair_agent` crate. |
| 14 | `pure/cli_main.omni` | Arg dispatch for run/eval/repl/serve/harness (S); seed `main` shrinks to the loader | All CLI-level tests (`omnic run …` output strings) identical; retire `omni-cli` |

**Commit size rule.** Each step is one commit of roughly ≤ 400 lines of Omni and ≤ 100 lines of Rust test glue. Split any step larger than that (step 13 will be several commits). No step deletes Rust until its gate has been green on `omni_vm` for one full suite run. Retirement goes in a separate follow-up commit.

## 4. Invariant compliance design

| Invariant | How the port satisfies it |
|---|---|
| **192 SSA / function** | The interpreter is split per opcode group (steps 3–8). Each handler fn takes `(arena, pc, ins_off)` and returns a packed status. Measure SSA with the frontend after each commit. Budget target ≤ 140 (Nyx measured `handle_dest` at 134, the riskiest). |
| **16 labels / function** (decided. The surface frontend allows only 15 until main-line C4; diff4 `lbl16_surface` is xfail until then, `lbl17_*` must stay rejected) | Dispatch over 128 opcodes **cannot** be one `match`. Use a 2-level tree: op group = `(op - 0x7F00) >> 4` decoded with **`sel_trit` chains (0 labels)** and at most 3 `match_trit` per fn (4 labels each = 12). Leaf handlers each ≤ 12 labels. One `while` = 3 labels for the fetch loop, in `vm_core` only. |
| **64-deep calls** | Two separate limits. (a) The *guest* call depth is the arena frame stack, capped at 64 frames → `BUDGET_EXCEEDED`, same as `MAX_VM_CALL_DEPTH` (1026). (b) The *interpreter's own* Omni call depth stays ≤ 4 (loop → group → handler → helper), so it never nests per guest call. |
| **Eager args** | All handlers take scalars or arena offsets. No lazily evaluated operands. Guest args are copied into the frame before entry (matches 1162-1166). |
| **No heap structs/maps/GC** | `OmniValue` becomes a tagged slot: `tag` (u8: Void/Int/Trit/Str/Bytes/U16Arr/TritVec/Struct/Arena/Skill/Dom) + `i64` payload. For heap kinds the payload is `(off << 20) \| len` into a **bump-allocated region** that is reset per request, so no GC is needed. `kv_store`/`req_params` become fixed-capacity **sorted flat arrays** (binary search, iterative). `tables` becomes fixed `schema × row × field` slabs. Capacity overflow → `BUDGET_EXCEEDED`. **Decided (Boss Lady, HANDOFF §6.7):** an arena-resident sorted array with binary search is allowed. Open addressing / hashing is **not** covered by that decision, so do not use it. |
| **Flat linear arena** | One region, fixed offsets, entirely inside Ember's band `0x100000-0x2FFFFF`. See §4.1. (v1's `0x00000-0xC0000+` layout is withdrawn because it overlapped M2 and M3.) |
| **No recursion in core .omni** | Iterative fetch loop. CALL/RET push and pop arena frames. Include-graph repair (`repair_omni_include_graph` 6363) and route pattern matching become explicit worklists. Add a verifier/lint check: no call graph cycle in `pure/vm_*`/`pure/host_*`. |
| **Opcode table never renumbered** | The Omni handlers key on the existing `0x7F00..0x7F7F` tokens. The 51 opcodes that are Void on the VM today stay Void (same observable) until separately specced. |
| **Frozen hashes unchanged until the single re-freeze** | New modules are **loaded from disk by new test code only**. They are **not** added to `PURE_OMNI_CORE_SUITE_MODULES` (10503) or `lookup_omni_stdlib_module` (4425+), and they don't touch `build_step7_stage1_compiler_stream` or the x86 encodings it uses. Every step's gate recomputes all five. The **single planned re-freeze** happens after Nyx's M2 fixed point (stage2 == stage3). Folding the new modules into the core suite then moves `0x78462AE4C81DF408` once, with sign-off. |
| **HTTP, DB, Repair Agent in core** | Steps 9, 11–13 put them in `pure/host_*` and `pure/repair_agent_*`. They are first-class core modules, not plugins. |
| **Never edit `pure/compiler.omni`** | Every step adds new files only. |

### 4.1 Arena layout in the host-runtime band `0x100000-0x2FFFFF` (v2)

Boss Lady's assignment: this band covers **everything** the host runtime allocates, including VM interpreter memory, call frames, tagged slots, the bump heap, KV/DB arrays and HTTP buffers. Regions are contiguous and checked by script: they start at `0x100000` and end exactly at `0x300000`, with no gaps or overlaps.

| Region | Start | End (incl.) | Size | Sizing / contents |
|---|---|---|---:|---|
| Control block | `0x100000` | `0x100FFF` | 4 KiB | **`0x100100-0x10013F` is taken by Ember's KV control words** (`pure/host_kv.omni`, `ember/runtime-port-r2@3de7843`; do not reuse). The rest holds status/error code, step and memory budget counters, heap bump pointer, decoder scratch (opcode, operand count, next offset, written to fixed offsets since there are no multi-value returns), frame depth |
| Arg / return staging | `0x101000` | `0x101FFF` | 4 KiB | guest argv as tagged slots (256 × 16 B), return slot |
| Function table | `0x102000` | `0x103FFF` | 8 KiB | 256 fns × 32 B (ins-table offset, ins count, param count, ret ty, label-table offset) |
| Label table | `0x104000` | `0x107FFF` | 16 KiB | 256 fns × 16 labels × u32 ins index (16-label cap) |
| Frame headers | `0x108000` | `0x108FFF` | 4 KiB | 64 frames × 64 B (return fn, return pc, dest reg, slot base); depth 64 → `BUDGET_EXCEEDED` |
| reserved | `0x109000` | `0x10FFFF` | 28 KiB | growth for the tables above |
| u16 program | `0x110000` | `0x12FFFF` | 128 KiB | 65,536 words. The Stage-1 stream is 19,731 words, so it fits 3.3× over |
| Decoded instruction table | `0x130000` | `0x16FFFF` | 256 KiB | 4,096 ins × 64 B (op, ty, dest, nops, ≤ 6 operands × 8 B) |
| Frames (register file, tagged slots) | `0x170000` | `0x1A7FFF` | 224 KiB | 64 frames × 224 slots × 16 B (tag + i64) = **229,376 B = `0x38000`**, exactly the size of the region |
| reserved | `0x1A8000` | `0x1AFFFF` | 32 KiB | |
| KV store (sorted array) | `0x1B0000` | `0x1EFFFF` | 256 KiB | 8,192 entries × 32 B (key i64, tag, value i64, pad), binary search, iterative. **Implemented** by Ember at exactly this base (`3de7843`). **Cap decided (Boss Lady):** the 8,193rd distinct key → `BUDGET_EXCEEDED` in Omni **and** in the Rust VM, which gets a matching cap in its own gated commit with a cap-hitting test |
| Request params (sorted array) | `0x1F0000` | `0x1F0FFF` | 4 KiB | 128 × 32 B |
| DB table headers | `0x1F1000` | `0x1F1FFF` | 4 KiB | 8 tables × schema (name hash, field count, row count, field hashes) |
| reserved | `0x1F2000` | `0x1FFFFF` | 56 KiB | |
| DB row slabs | `0x200000` | `0x27FFFF` | 512 KiB | 8 tables × 512 rows × 8 fields × 16 B = **524,288 B** |
| HTTP request buffer | `0x280000` | `0x28FFFF` | 64 KiB | raw bytes from the shim |
| HTTP response buffer | `0x290000` | `0x29FFFF` | 64 KiB | framed response bytes to the shim |
| Route table | `0x2A0000` | `0x2A0FFF` | 4 KiB | 64 routes × 64 B |
| reserved | `0x2A1000` | `0x2AFFFF` | 60 KiB | |
| Bump heap | `0x2B0000` | `0x2FFFFF` | 320 KiB | Str/Bytes/U16Arr/TritVec/Struct payloads, reset per request (no GC). Heap tagged-slot payload = `(off << 20) \| len`, where `off` is absolute (22 bits suffice below `0x300000`) and `len` is < 2^20 |

**Overlap check.** M2 is below `0x80000` (HANDOFF §6.6) and M3 is `0x80000-0xFFFFF`. This band starts at `0x100000`, so there is no overlap with either. Every capacity overflow (program > 64 Ki words, > 4,096 ins, > 256 fns, depth > 64, KV > 8,192, heap exhausted) → `BUDGET_EXCEEDED`, never a wrap.

**Uncertain (verify before step 1):** the native backends must provide a linear arena of at least `0x300000` bytes at these absolute offsets. Today the x86 ISET heap is addressed at `[rbp+r11-0x20000]` (cited in the diff4 `p1_byte_readback` xfail reason), which suggests a much smaller region. Whether ISA-A/B or M2's arena work grows it to 3 MiB is not confirmed. Step 1 is blocked on ISA-A anyway (Ember: x86/Stage-1 still read buffers from the wrong memory).

**Frontend constraints to plan around** (from Nyx's review):
- Literals must fit in i32. 44-bit masks get computed or split.
- The entry fn is the last-declared fn.
- `#[fn_id]` is limited to 0..15. **Uncertain:** whether a module may hold more than 16 fns this way, which affects how far step 13 is split.
- Native compile goes through Stage-1, which has known bugs (§6). Gate every module on x86 *and* Stage-1 and keep them byte-identical.

## 5. Risks

- **Stage-1 emitter bugs** (DROP misdecode, IMOD stale rdx, no pack5/EGATE arms, BR_POS non-trit) would mis-execute the new Omni modules themselves when compiled natively. Mitigation: a lint that the `pure/vm_*` sources avoid DROP-before-RET with odd counts, IMOD, pack5 and non-trit `if_pos`, enforced until fixed. diff4 catches the rest.
- **Performance.** An interpreter-in-Omni run natively should be fine for `run`. For `serve`, budget tests are needed (the step budget is already enforced).
- **Repair Agent size** (1,437 lines, one 598-line fn) is the biggest SSA/label risk. Split by error code before porting.

## 6. Bugs to fix before / at the start of the port (step 0)

1. **VM negative patch offset panic (main line, not Ember).** `iset_patch_i64(buf, -3, v)` → `off as usize` wraps → Rust panic at **1324-1326**; workspace `omni-vm/src/lib.rs:310` has the same. **Decided (HANDOFF §6.4):** a silent no-op. Gate: `p9_vm_patch_negative_offset` expects `slen` = 8 since `vera/parity-goldens` `49091f2` and will XPASS when the fix lands (a local simulation of the guard confirmed this). This is VM-only and hash-safe.
2. **`omnic run` wrong buffer results (main line, not Ember).** The review found `omnic run iget_buf_dyn.omni 7` → `Int(0)`. At `d2d6c13` the Wasm fix makes it `Int(7)`, and `G1` gives 8796075198218 (RUN). **Still wrong via `run`** (RUN at `d257cb0`): `two_bufs` → `Int(11)` (want 31) and `pack5_flow` → `Int(2009000)` (want 2255044), because `run` is AOT-first = Wasm (5602-5699, 9628-9746). Fix the Wasm per-buffer length and the flow-insensitive fold, or have `run` fall back to the VM for buffer programs. After step 10, `run` uses `omni_vm` and the issue goes away by construction.
3. **`DB_GET/DB_QRY` miss fallback `key*10`** (1500-1518) makes DB goldens unable to tell a hit from a miss. **Decided:** both return **-1** on a miss, a documented not-found value. `route_db_query_golden` gets a real hit case (P12). Ember owns this. **Known guest/host difference:** `pure/host_runtime.omni:47-56` still returns `key*10`. Changing it would move the suite hash `0x78462AE4C81DF408`, so it **waits for the planned re-freeze** after the M2 fixed point. Until then the Rust host and the Omni guest kernel disagree on a miss by design. This is logged in HANDOFF.
4. **Mislabelled "VM" legs.** Tests that call `execute_omni_surface_program` and name it VM are really Wasm (C5/C6; partly fixed by P11). Audit the rest before using them as port oracles.
5. **Stage-1 DROP misdecode / IMOD / pack5** need a decision: re-freeze, or reject at the Stage-1 gate (5723-5727). Rejecting is hash-safe and enough for the port, since the new modules avoid those ops.

## 7. Open decisions (owner: Caleb / Boss Lady)

Resolved since v1 (HANDOFF §6):
- Label cap: **16** (C4 fixes the frontend's numbering at `alloc_lbl` 3587 / 3646-3656).
- Arena sorted array + binary search: **allowed**. Hash or open-addressing tables are not covered.
- `DB_QRY` and `DB_GET` miss: **-1**. The `pure/host_runtime.omni` twin waits for the re-freeze.
- VM negative patch offset: **silent no-op** (main line).
- Ember's band: **`0x100000-0x2FFFFF`**, covering everything it allocates.
- KV cap: **8,192 keys in both Omni and Rust** (`BUDGET_EXCEEDED` on the 8,193rd).
- `DB_UPD`/`DB_DEL`: **no-ops in both**, matching today's Rust. Real semantics are an open hardening item.

Still open:
- Trit clamp vs verifier reject for non-trit operands (C3 is the hash-safe reject route). This affects what `vm_trit`/`vm_core` must reproduce.
- Lockstep overhead policy (deferred to Caleb).

## 8. v2 changes (2026-11-14)

- §4.1: new arena table in `0x100000-0x2FFFFF`, contiguous, ending exactly at `0x300000`. Frames are 229,376 B = `0x38000`, and DB slabs are 524,288 B. Clear of M2 and M3. Flagged uncertainty: native backend arena size.
- §0/§3/§6: the negative-offset and run-via-Wasm fixes moved to the main line. P9 expects a no-op. DB miss is -1. The `pure/host_runtime.omni` `key*10` is a known guest/host difference until the re-freeze.
- §4: 16 labels decided. Sorted-array KV approved and the uncertainty removed.
- §7: decisions moved to resolved.
- Later the same evening: recorded Ember's implemented KV placement (entries `0x1B0000`, control words `0x100100-0x10013F`, both inside this layout), Boss Lady's 8,192-key cap for Rust too, and `DB_UPD`/`DB_DEL` staying no-ops.
