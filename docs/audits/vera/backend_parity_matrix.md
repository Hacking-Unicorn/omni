# Omni 4-backend opcode parity matrix

**Read-only audit.** Repo: `github.com/Hacking-Unicorn/omni`. Clone: `/workspace/audit-parity` (detached `origin/isa/multibyte-loads` @ `8160842` — *Fix value-returning if_pos SSA join-slot holes…*). `/workspace/omni` was not touched. No commits, pushes, or PRs.

## Scope and method

This audit grades all **128** ISA opcodes `0x7F00..0x7F7F` from `OMNI_ISA_TABLE` (`omni_rust_bootstrap.rs:161-241`, identical copy in `omni-workspace/crates/omni-isa/src/lib.rs:112-241`) against how each backend **actually** implements them, not how the docs describe the supported subset.

Backends:

- **VM** — `OmniBinaryVm::execute_function` in `omni_rust_bootstrap.rs:1123-1677` (workspace mirror: `omni-workspace/crates/omni-vm/src/lib.rs`).
- **x86-64 ELF** — Rust AOT emitter `compile_pure_u16_to_x86_64_elf_with_args` (`omni_rust_bootstrap.rs:13979-14491`; workspace: `omni-workspace/crates/omni-aot/src/lib.rs`). Rejects any opcode outside `is_x86_64_aot_supported_opcode` (`omni_rust_bootstrap.rs:302-317`).
- **Stage-1** — `compile_omni_surface_via_stage1_to_elf` (`omni_rust_bootstrap.rs:5704-5744`) runs the hand-built u16 compiler `build_step7_stage1_compiler_stream` (`omni_rust_bootstrap.rs:15422-16002`) on the VM and emits an ELF. The **same** x86 supported-opcode gate is applied first (`5722-5725`). Stage-1 is therefore a second x86 lowering, not a fourth ISA, and it does **not** implement every opcode the gate accepts.
- **Wasm** — `compile_pure_u16_to_wasm` (`omni_rust_bootstrap.rs:14526-15025`) plus the in-process evaluator. Gate: `is_wasm_aot_supported_opcode` (`319-335`), which is **identical** to the x86 predicate (52 opcodes).

The 52-opcode AOT subset is: `0x7F00`, `0x7F01..0x7F06`, `0x7F09`, `0x7F0D..0x7F18`, `0x7F1E..0x7F2D`, `0x7F30..0x7F31`, `0x7F34..0x7F35`, `0x7F3F..0x7F45`, `0x7F4A`, `0x7F50`, `0x7F57..0x7F59`. The other 76 opcodes are compile-time **unsupported** on x86, Stage-1, and Wasm.

Primary evidence is the monolith `omni_rust_bootstrap.rs`, which is what `omnic selftest` / `run_unit_and_differential_selftests` exercise. The workspace crates were spot-checked and match on the ISA table, the two AOT predicates, and the Wasm IGET/ISET stubs.

## Category definitions

| Tag | Meaning |
|---|---|
| `real` | Implements the intended operation (or a faithful lowering). Designed no-ops (`NOP`, `LABEL`) count as real. |
| `real*` | Simplified in-process host simulation (in-memory KV/tables, synthetic HTTP, dummy skill/DOM objects). Observably does *something*, but it is not a real OS/DB/net/GPU backend. |
| `partial` | Real for some operand shapes, stub or wrong for others. See the cell text. |
| `stub` | Constant result, identity move, length-only update, explicit no-op where work was required, or VM `Void` fallthrough (`omni_rust_bootstrap.rs:1666`). |
| `unsupported` | Backend refuses to compile / lower the opcode (error). |
| `uncertain` | Evidence is incomplete or the arm may misdecode; not overclaimed. |

Test tags in the matrix (separate from implementation class):

- `T` — a selftest or golden **observes the opcode’s effect** on that backend (result compare and/or native ELF stdout).
- `W` — a test touches the opcode but is **weak**: it would still pass if the real effect were missing (length-only, unused identity, dummy fallback).
- `W?` — x86/Stage-1 may compile or bit-compare ELF for a program that uses the opcode, but I found no run that reads back the effect (especially buffer contents).
- `-` — no test found that exercises this opcode on that backend.

A cell can be `real` and still `-` (implemented, untested) or `stub` and `W` (a golden passes *because* it does not look).

## Counts per category per backend

| Backend | real | real* (simplified host) | partial | stub | unsupported | uncertain |
|---|---:|---:|---:|---:|---:|---:|
| VM | 55 | 16 | 1 | 56 | 0 | 0 |
| x86-64 ELF | 39 | 0 | 6 | 7 | 76 | 0 |
| Stage-1 | 41 | 0 | 1 | 9 | 76 | 1 |
| Wasm | 38 | 0 | 6 | 8 | 76 | 0 |

Collapsing `real`+`real*`+`partial` as “has a lowering”, and `stub`+`uncertain` as “not a real implementation”:

| Backend | has a lowering | stub/uncertain | unsupported |
|---|---:|---:|---:|
| VM | 72 | 56 | 0 |
| x86-64 ELF | 45 | 7 | 76 |
| Stage-1 | 42 | 10 | 76 |
| Wasm | 44 | 8 | 76 |

Notes on the counts:

- VM never returns **unsupported**. The 51 ISA slots with no `match` arm take `_ => OmniValue::Void` (`omni_rust_bootstrap.rs:1666`) and are counted as **stub**.
- x86 / Wasm / Stage-1 share the 76-opcode reject set. Differences among those three live entirely inside the 52-opcode gate.
- VM `real*` (16) are host/ABI opcodes with in-process stand-ins (see matrix). They are not available on the three AOT backends.

## Top 10 most consequential gaps

Ranked by how much they undermine the “4-backend parity” claim or a security/correctness invariant, not by how easy they are to fix.

1. **Wasm `ISET` is length-only; Wasm `IGET` of a runtime byte buffer returns 0.** `omni_rust_bootstrap.rs:14752-14776` increments `buf_len_local` (and treats `-9` as `delta=0`, i.e. a no-op). It never writes linear memory. `omni_rust_bootstrap.rs:14747-14750` is the IGET fallback: `i64.const 0`. Compile-time `@PACK5` constants are handled earlier (`14708-14746`) and *do* work. Guest `anew`/`iset`/`iget` buffers do not. Workspace mirror: `omni-aot/src/lib.rs:920` / `:876`.
2. **`byte_buf_golden` cannot catch (1).** The frozen golden (`omni_rust_bootstrap.rs:8904`, `17403-17405`, and `spec/acceptance/goldens/byte_buf_golden.omni`) is `anew` + `iset` + `iset_i32` + `iset_i64` + `slen` + `cmov`. Expected result `315` = length `15` + `cmov` `300`. It never `iget`s a stored byte. Acceptance wants backend `"aot"`, and `execute_omni_surface_aot_first` (`5595-5680`) runs **Wasm**, not native x86. The golden is green on a backend that does not store bytes.
3. **x86 and Stage-1 `IGET` do not read back `ISET` byte buffers either.** After the pack5-const path, Rust x86 emits `movzx eax, word [rbp+rax*2-0x2E000]` (`14177-14178`). Stage-1 emits the same encoding (`15832-15836`) and has **no** pack5-const path. That addressing is a **u16 token-stream** load (the region the Stage-1 compiler itself uses to read its input), not a byte load from the ISET heap at `[rbp+r11-0x20000]` (`14181-14197`, `15856-15880`). So even the two “real store” backends lack a faithful byte `IGET`. No golden I found executes `iget` on an `iset` buffer and compares native/Wasm/VM bytes.
4. **Stage-1 is not a parity twin of Rust x86 for 7 gated opcodes.** The gate accepts `ARENA`, `DROP`, `TPACK5`, `TUNPACK5`, `TDOT`, `EGATE`, and `TLOGIC`, but Stage-1 has no dispatch arms for them (`15649-15743`). They fall through to `lbl_store_dst` (`15743`) — typically “store whatever is in rax.” Rust x86 *does* lower the pack5/tdot/egate/tlogic family via `collect_func_pack5_map` (`13836`, `14207-14260`). 4-way ELF-equality tests (`9334+`, `17802-17804`) avoid these opcodes (e.g. `hot_loop`, Phase-7 kernels). `pack5_direct_aot` / `pack5_trit_vec_golden` compare VM/Wasm/x86, **not** Stage-1 ELF.
5. **Sanitizers are identity on every AOT backend, including the ones in the official supported subset.** VM `SAN_SQL` / `SAN_DOM` actually rewrite strings (`1477-1478`). `SAN_PRM` is identity even on the VM (`1479`). x86 and Wasm group `0x7F57|0x7F58|0x7F59` with `CINT` as a plain operand move (`14129-14132`, `14649-14652`). Stage-1 jumps copy-ops to `store_dst` (`15692-15694`). A program that sanitizes and then returns a string cannot use standalone AOT anyway (`CSTR`/`SCAT` are unsupported), but any future AOT string path would silently skip sanitization. `san_dom("<b>ok</b>")` is checked on the **VM only** (`17725-17726`).
6. **`ARENA` / `BORROW_*` / `DROP` are stubs or identities on AOT, and `arena_lifecycle_golden` does not observe them.** Wasm/x86 treat `ARENA`/`BORROW_*` as identity and `DROP` as a no-op (`14128-14132`, `14648-14652`). The golden (`8909`, `17645-17657`) returns `x+42`. Acceptance wants `"aot"` (Wasm). The test stays green if arena/borrow/drop do nothing.
7. **`execute_omni_surface_aot_first` “aot” means Wasm, not x86-64 ELF.** Comment and body at `5595-5680`. Acceptance goldens tagged `"aot"` (`8889-8910`) therefore ratify Wasm semantics. Combined with (1) and (6), several “AOT-equivalent” goldens never execute a native ELF and never look at memory.
8. **`DB_QRY` / `DB_GET` dummy fallback makes `route_db_query_golden` weak.** VM (`1500-1518`) returns `key.wrapping_mul(10)` on a miss. The golden (`8912`) does `db_query(..., id=42)` and expects `200000420` = `200*1_000_000 + 42*10`. That passes with **no row in the store**. `route_kv_golden` (`8911`) is stronger: `kv_put` then `db_get` observes 777. Both are **VM-only** (`want_backend: "vm"`); AOT rejects `0x7F64/65/6B` as unsupported.
9. **51 ISA opcodes are Void on the VM and unsupported on AOT — including advertised FS/SYS/float/struct ops.** `FS_READ`, `ENV_GET`, `LOG`, `TIME_MS`, `TX_*`, `DB_UPD`/`DB_DEL`, `SSLICE`/`SFIND`/`SSPLIT`, all float ops (`CFLT`/`FADD`/…), `SNEW`/`FGET`/`FSET`, `TRY`/`ERR`/`ASSERT`, `WGPU`/`WAUDIO`, etc. take the VM default (`1666`) and are rejected by the AOT gate. The table still names them as first-class opcodes. I found no execution test that expects real FS or float behavior.
10. **`if_pos` 4-backend coverage is compile-only on x86 and (mostly) Stage-1.** `run_if_pos_join_slot_selftests` (`12016-12109`) executes VM and Wasm results for every case, compiles Wasm and x86 ELF for every case, and compiles Stage-1 ELF only for `arith_else_temps`. It does **not** run the native ELF or compare Stage-1 ELF to host ELF. A silent x86 join-slot emit bug could pass this suite.

## Weak goldens (pass without observing the real effect)

| Golden / test | Where | Why it is weak |
|---|---|---|
| `byte_buf_golden` | `omni_rust_bootstrap.rs:8904`, `11790`, `12131`, `17403-17405`; `spec/acceptance/goldens/byte_buf_golden.omni` | Observes only `slen` (15) + `cmov` (300). Wasm ISET never stores; IGET of the buffer is untested. |
| `arena_lifecycle_golden` | `8909`, `17645-17657`; `spec/acceptance/goldens/arena_lifecycle_golden.omni` | Result is `x+42`. Arena/borrow/drop unobserved. Acceptance backend is Wasm AOT, where those ops are identity/no-op. |
| `route_db_query_golden` | `8912`; `spec/acceptance/goldens/route_db_query_golden.omni` | Expected body `420` equals `id*10`, the VM miss fallback (`1502-1503`), not a stored row. VM-only. |
| `pack5_trit_vec_golden` / `pack5_direct_aot` | `8906`, `8910`, `17505-17551`, `17733-17784` | Byte `iget` and `tdot`/`egate` go through **compile-time** `reg_pack5` (`13836`, `14708-14746`, `14137-14175`). They do not prove runtime memory loads. Stage-1 is not compared. `slen`/`pack5_len` succeed because lowering is `ANEW`+`ISET` of constant bytes (`3684-3705`), which bumps the length register even on Wasm. |
| `decision_hook_golden` | `8905`, `17490-17502` | Executes on the VM (`egate` of an i64 uses the “else load operand / sign” path, `1436-1447` / `14247-14259`). Not a 4-backend memory test. |
| `str_pipeline_golden` / bare `san_dom` | `17700-17730` | VM-only. `san_prm` is identity even on the VM (`1479`), so the “sanitizer” in the pipeline is untested as a transform. AOT sanitizer identity is untested. |
| Acceptance `"aot"` tag | `8889-8910`, `5595-5680` | Means Wasm AOT-first, not native x86 and not Stage-1. |
| `if_pos` join-slot suite | `12016-12109` | VM+Wasm results are checked; x86/Stage-1 only have to compile (Stage-1 only for one case). |
| `while_sum_even_golden` 4-way block | `17606-17640` | VM result, Wasm result, and **ELF bytes** Stage-1==host are checked. The native ELF is not executed in that block (other loop goldens in `run_unit_and_differential_selftests` *do* run native ELF). |

Goldens that look **strong** (for the numeric/ternary subset): `iabs_golden`, `idiv_zero`, `shl_65`, `seed0/4/6`, `custom_non_seed_alu`, `sel_mix`, `br_pos_golden`, `br_neg_arith_golden`, `nested_branch_golden`, `match_trit_golden`, `branch_loop_golden` — `run_unit_and_differential_selftests` (`16245+`) compares VM, Wasm, and **native ELF stdout** for these, and many also assert Stage-1 ELF == host ELF.

## Shared implementation notes (do not over-read the matrix)

- **`ISET` modes (current tree, no new load opcodes):** 2-operand+ bytes append; 3-operand mode `-4` / `-8` append i32/i64; 4-operand mode `-9` in-place i64 patch. VM `-9` silently does nothing on OOB (`1324-1327`) — left as-is. x86/Stage-1 `-9` store with no bounds check (`14188-14191`, `15876-15880`). Wasm `-9` does not write and does not change length (`14757-14758`). There is **no** 3-operand `IGET` width mode (`-2/-4/-8`) in this commit; IGET is 2-operand only (`1288-1303`).
- **Single global buffer on AOT.** x86 `SLEN` is `mov rax, r11` (`14203-14206`); Wasm `SLEN` returns `buf_len_local` (`14777-14782`); both ignore the operand. Only one live byte buffer is modeled.
- **x86 `ANEW` dest is stale rax.** `14133-14136` clears `r11d` then `emit_store_rax`. Subsequent `ISET` overwrites dest with `r11`. Fragile, but `byte_buf_golden` never inspects the handle.
- **`CALL` (`0x7F48`) is VM-only.** `multi_fn_weighted_term` is tagged `"vm"` (`8907`). AOT requires a single-function module (`is_module_aot_hot_path`, `13934-13948`).
- **Workspace mirrors** the same Wasm IGET-zero / ISET-length behavior (`omni-aot/src/lib.rs` around the `0x7F0E` / `0x7F0F` arms) and the same AOT predicates (`omni-isa/src/lib.rs:253-335`).

## Full 128-opcode matrix

Evidence anchors: VM `omni_rust_bootstrap.rs:1171-1666`; x86 `14127-14449` (gate `302-317`, `13993`); Wasm `14647-14998` (gate `319-335`, `14535`); Stage-1 gate `5722-5725`, dispatch `15649-15743`, IGET/ISET `15822-15884`.

| Tok | Name | Cap | VM | t | x86-64 ELF | t | Stage-1 | t | Wasm | t |
|---|---|---|---|:-:|---|:-:|---|:-:|---|:-:|
| `0x7F00` | NOP | — | real | T | real (no-op) | T | real (no-op / skip) | T | real (no-op) | T |
| `0x7F01` | ARENA | — | real | W | stub (identity) | - | stub (no arm; falls through to store_dst after load) | - | stub (identity) | W |
| `0x7F02` | BORROW_R | — | real* | W | stub (identity) | - | real (load operand, store dest) | - | stub (identity) | W |
| `0x7F03` | BORROW_W | — | real* | - | stub (identity) | - | real (load operand, store dest) | - | stub (identity) | - |
| `0x7F04` | MOVE | — | real | - | real | - | real (load operand, store dest) | - | real | - |
| `0x7F05` | DROP | — | real | W | stub (no-op) | - | uncertain (no dest form; may misdecode tok0=DROP) | - | stub (no-op) | W |
| `0x7F06` | CINT | — | real | T | real | T | real (load operand, store dest) | T | real | T |
| `0x7F07` | CFLT | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F08` | CSTR | — | real | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F09` | CTRIT | — | real | T | real | T | real (load operand, store dest) | T | real | T |
| `0x7F0A` | SNEW | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F0B` | FGET | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F0C` | FSET | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F0D` | ANEW | — | real | W | real (xor r11d,r11d; dest stores rax not r11) | W? | real (xor r11d,r11d then store_dst) | W? | real (sets buf_len=0, dest=0) | W |
| `0x7F0E` | IGET | — | partial (byte/u16/trit; no width modes) | T | partial (pack5-const or u16-word load; not byte-buffer readback) | T | partial (always u16-word load from token region; no pack5 fold) | - | partial (pack5-const real; else i64.const 0) | T |
| `0x7F0F` | ISET | — | real (append bytes/-4/-8; -9 in-place, silent OOB) | W | real (writes [rbp+r11-0x20000]; -9 no bounds check) | W? | real (emits same stores as Rust x86; -9 no bounds check) | W? | stub (updates buf_len only; -9 is no-op; no stores) | W |
| `0x7F10` | IADD | — | real | T | real | T | real | T | real | T |
| `0x7F11` | ISUB | — | real | T | real | T | real | T | real | T |
| `0x7F12` | IMUL | — | real | T | real | T | real | T | real | T |
| `0x7F13` | IDIV | — | real | T | real | T | real | T | real | T |
| `0x7F14` | IMOD | — | real | T | real | T | real | T | real | T |
| `0x7F15` | INEG | — | real | T | real | T | real | T | real | T |
| `0x7F16` | IABS | — | real | T | real | T | real | T | real | T |
| `0x7F17` | IMIN | — | real | T | real | T | real | T | real | T |
| `0x7F18` | IMAX | — | real | T | real | T | real | T | real | T |
| `0x7F19` | FADD | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F1A` | FSUB | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F1B` | FMUL | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F1C` | FDIV | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F1D` | FSQRT | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F1E` | BAND | — | real | T | real | T | real | T | real | T |
| `0x7F1F` | BOR | — | real | T | real | T | real | T | real | T |
| `0x7F20` | BXOR | — | real | T | real | T | real | T | real | T |
| `0x7F21` | BNOT | — | real | T | real | T | real | T | real | T |
| `0x7F22` | SHL | — | real | T | real | T | real | T | real | T |
| `0x7F23` | SHR | — | real | T | real | T | real | T | real | T |
| `0x7F24` | CEQ | — | real | T | real | T | real | T | real | T |
| `0x7F25` | CNE | — | real | T | real | T | real | T | real | T |
| `0x7F26` | CLT | — | real | T | real | T | real | T | real | T |
| `0x7F27` | CGT | — | real | T | real | T | real | T | real | T |
| `0x7F28` | TNOT | — | real | T | real | T | real | T | real | T |
| `0x7F29` | TAND | — | real | T | real | T | real | T | real | T |
| `0x7F2A` | TOR | — | real | T | real | T | real | T | real | T |
| `0x7F2B` | TXOR | — | real | T | real | T | real | T | real | T |
| `0x7F2C` | TMAJ | — | real | T | real | T | real | T | real | T |
| `0x7F2D` | TCMP | — | real | T | real | T | real | T | real | T |
| `0x7F2E` | TADD | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F2F` | TMUL | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F30` | TPACK5 | — | real | T | partial (compile-time fold of dest pack5; else 0) | T | stub (no arm; falls through to store_dst) | - | partial (compile-time fold of dest pack5; else 0) | T |
| `0x7F31` | TUNPACK5 | — | real | T | partial (compile-time fold of dest pack5; else 0) | T | stub (no arm; falls through to store_dst) | - | partial (compile-time fold of dest pack5; else 0) | T |
| `0x7F32` | TQUANT | AI | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F33` | TMATMUL | AI | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F34` | TDOT | — | real | T | partial (unrolls compile-time pack5; else load/sign) | T | stub (no arm; falls through to store_dst) | - | partial (unrolls compile-time pack5; else load/sign) | T |
| `0x7F35` | EGATE | AI | real | T | partial (unrolls compile-time pack5; else load/sign) | T | stub (no arm; falls through to store_dst) | - | partial (unrolls compile-time pack5; else load/sign) | T |
| `0x7F36` | LOOKAHEAD | AI | stub (const -1) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F37` | PREFETCH | AI | stub (empty struct) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F38` | HMATMUL | AI | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F39` | HOLO_EXEC | AI | stub (partial dummy) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F3A` | TALL | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F3B` | TANY | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F3C` | TVNEW | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F3D` | TVGET | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F3E` | TVSET | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F3F` | TLOGIC | — | real | T | partial (unrolls compile-time pack5; else load/sign) | T | stub (no arm; falls through to store_dst) | - | partial (unrolls compile-time pack5; else load/sign) | T |
| `0x7F40` | LABEL | — | real | T | real | T | real | T | real | T |
| `0x7F41` | JMP | — | real | T | real | T | real | T | real | T |
| `0x7F42` | BR_TRIT | — | real | T | real | T | real | T | real | T |
| `0x7F43` | BR_POS | — | real | T | real | T | real | T | real | T |
| `0x7F44` | BR_NEG | — | real | T | real | T | real | T | real | T |
| `0x7F45` | SEL_TRIT | — | real | T | real | T | real | T | real | T |
| `0x7F46` | LBOUND | — | real | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F47` | LNEXT | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F48` | CALL | — | real | T | unsupported | - | unsupported | - | unsupported | - |
| `0x7F49` | TCALL | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F4A` | RET | — | real | T | real | T | real | T | real | T |
| `0x7F4B` | TRY | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F4C` | ERR | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F4D` | IS_OK | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F4E` | UNWRAP | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F4F` | ASSERT | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F50` | SLEN | — | real | W | real (returns r11 global length, ignores operand) | W? | real (mov rax,r11 global length) | W? | real (returns buf_len local, ignores operand) | W |
| `0x7F51` | SCAT | — | real | T | unsupported | - | unsupported | - | unsupported | - |
| `0x7F52` | SSLICE | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F53` | SFIND | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F54` | SSPLIT | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F55` | S2I | — | real | T | unsupported | - | unsupported | - | unsupported | - |
| `0x7F56` | I2S | — | real | T | unsupported | - | unsupported | - | unsupported | - |
| `0x7F57` | SAN_SQL | — | real | T | stub (identity) | - | stub (identity copy) | - | stub (identity) | - |
| `0x7F58` | SAN_DOM | — | real | T | stub (identity) | - | stub (identity copy) | - | stub (identity) | - |
| `0x7F59` | SAN_PRM | AI | stub (identity) | W | stub (identity) | - | stub (identity copy) | - | stub (identity) | - |
| `0x7F5A` | SAN_PTH | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F5B` | VAL_UUID | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F5C` | VAL_JSON | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F5D` | FNV1A | — | real | T | unsupported | - | unsupported | - | unsupported | - |
| `0x7F5E` | HEX_ENC | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F5F` | HEX_DEC | — | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F60` | TX_BEG | DB_W | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F61` | TX_COM | DB_W | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F62` | TX_RBK | DB_W | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F63` | DB_INS | DB_W | real* | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F64` | DB_GET | DB_R | real* | T | unsupported | - | unsupported | - | unsupported | - |
| `0x7F65` | DB_QRY | DB_R | real* | W | unsupported | - | unsupported | - | unsupported | - |
| `0x7F66` | DB_UPD | DB_W | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F67` | DB_DEL | DB_W | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F68` | DB_CNT | DB_R | real* | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F69` | TV_INS | DB_W | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F6A` | TV_SRCH | DB_R | real* | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F6B` | KV_PUT | DB_W | real* | T | unsupported | - | unsupported | - | unsupported | - |
| `0x7F6C` | ROUTE | NET | real* | T | unsupported | - | unsupported | - | unsupported | - |
| `0x7F6D` | REQ_PAR | NET | real* | T | unsupported | - | unsupported | - | unsupported | - |
| `0x7F6E` | RSP_JSON | NET | real* | T | unsupported | - | unsupported | - | unsupported | - |
| `0x7F6F` | RSP_HTML | NET | real* | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F70` | AUTH_VER | AUTH | stub (identity) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F71` | RATELIM | NET | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F72` | TIME_MS | SYS | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F73` | LOG | SYS | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F74` | ENV_GET | SYS | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F75` | FS_READ | FS | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F76` | DOM_NODE | DOM | real* | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F77` | DOM_ATTR | DOM | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F78` | DOM_TEXT | DOM | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F79` | DOM_EVT | DOM | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F7A` | DOM_MNT | DOM | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F7B` | WGPU | DOM | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F7C` | WAUDIO | AUDIO | stub (void fallthrough) | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F7D` | EMBED | AI | real* | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F7E` | SK_LOAD | VISION | real* | - | unsupported | - | unsupported | - | unsupported | - |
| `0x7F7F` | SK_EXEC | VISION | real* | - | unsupported | - | unsupported | - | unsupported | - |

## Opcode groups (for navigation)

- **0x7F00–0x7F0C** — NOP, arenas/borrows/move/drop, consts, struct ops. Struct ops (`SNEW`/`FGET`/`FSET`) and `CFLT` are VM-void / AOT-unsupported.
- **0x7F0D–0x7F0F** — ANEW/IGET/ISET: the largest inter-backend split (see gaps 1–3).
- **0x7F10–0x7F2D** — i64 ALU, bitwise, comparisons, ternary. This is the actually-tested 4-backend core, except `TADD`/`TMUL` (`0x7F2E`/`0x7F2F`) which are VM-void / AOT-unsupported, and float ops `0x7F19–0x7F1D` which are the same.
- **0x7F30–0x7F3F** — Pack5 / AI. Const-fold partial on Rust x86 and Wasm; Stage-1 missing arms for 30/31/34/35/3F; several AI ops are VM stubs or void.
- **0x7F40–0x7F4F** — Control, call, errors. Branches/RET are the tested core. `CALL` is VM-only. `LNEXT`/`TCALL`/`TRY`/`ERR`/`UNWRAP`/`ASSERT` are void/unsupported. `LBOUND` is a VM no-op (grouped with NOP/LABEL at `1172`) and is **not** in the AOT predicate (unsupported on AOT).
- **0x7F50–0x7F5F** — Strings, sanitizers, hash, hex. Only `SLEN` is in the AOT subset (global length). Sanitizers are AOT identity. Other string ops are VM-only or void.
- **0x7F60–0x7F75** — DB, HTTP, auth, sys, FS. VM has a simplified subset; AOT rejects all of them. Many slots are VM-void as well (`TX_*`, `DB_UPD`/`DEL`, `FS_READ`, `ENV_GET`, `LOG`, `TIME_MS`, `RATELIM`).
- **0x7F76–0x7F7F** — DOM / wgpu / audio / embed / skills. VM has simplified `DOM_NODE`/`EMBED`/`SK_*`; the rest are void; all unsupported on AOT.

## What this audit is not

- It does not claim the 52-opcode numeric/ternary core is broken. That core (ALU, shifts, ternary, branches, `SEL_TRIT`, `RET`, `IABS`, div/mod edge cases) has genuine VM / Wasm / native-ELF agreement in `run_unit_and_differential_selftests`.
- It does not re-run the selftests. Classifications are from reading the emit/eval arms and the tests that call them.
- Where a test might exist in a path I did not find, the test tag is `-` or `W?`, not a claim that no test can exist.

