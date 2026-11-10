# Milestone 2: final plan (pure/m2 two-pass u16 → ELF64 compiler written in Omni)

Author: Nyx Fixpoint. Imported from the audit plan; decisions in DECISIONS.md.

Inputs: the Milestone 2 design doc, the Nyx critical review, and Caleb's decisions of 2026-11-10 (see `DECISIONS.md`).

Line references are against `isa/multibyte-loads` @ `8160842`.

Refs:
- `A:` = `omni-workspace/crates/omni-aot/src/lib.rs`
- `M:` = `omni_rust_bootstrap.rs` (the monolith; `M = A + 13832` from `A:102`)
- `ISA:` = `omni-workspace/crates/omni-isa/src/lib.rs`
- `IR:` = `omni-workspace/crates/omni-ir/src/lib.rs`
- `FE:` = `omni-workspace/crates/omni-lang/src/compiler/mod.rs`
- `VE:` = `pure/verifier_engine.omni`

---

## 0. Decisions (fixed) vs still open

**Decided by Caleb (treated as fixed here, nothing more):**

| # | Decision | Consequence in this plan |
|---|---|---|
| D-1 | 3-operand `RET buf, off, len` | M2-06. VM, x86 single and multi, Wasm, both verifiers, frontend `ret_slice` |
| D-2 | `IGET` byte mode `-1` | Part of the C1 shape set. M2 consumes it via `ld_u8` (M2-05). **I don't define C1's encoding.** |
| D-3 | u32 label entries | 4096 × u32 at arena logical `0x10000-0x13FFF`, value = `code_off + 1`, `0` = undefined (§3.4) |
| D-4 | Native multi-function ABI with a 1 MiB stack arena | Stack args at `[rbp+16+8k]`, `rbx` = arena base, `sub rsp,0x110010` stub (§2) |
| D-5 | `pure/compiler.omni` stays frozen; M2 lives in `pure/m2/` | No change to `PURE_OMNI_CORE_SUITE_MODULES` (M:10503-10522) or `lookup_omni_stdlib_module` (M:4425) |
| D-6 | Wasm CALL required for sign-off | M2-10 is on the critical path. Sign-off needs a Wasm witness in §4 |

**Still open (options only; Caleb hasn't decided):**

| # | Open item | Options |
|---|---|---|
| O-1 | Trap exit codes | e.g. one code per kind (`TRUNC`, `OOB`, `CAP`), or a single code. The placeholders here are `T_TRUNC`, `T_OOB`, `T_CAP`. |
| O-2 | Wasm bytes-result channel | (a) a new evaluator API returning `memory[off..off+len]`, which is strongest; (b) an in-guest wrapper returning FNV-1a64 + len as i64s, which needs no new API but gives only a hash witness |
| O-3 | Multi-mode input window | 0x10000 B (32,768 words; baseline here) with a truncation trap, vs 0x20000 B (65,536 words; costs only stack) |
| O-4 | Label ids outside M2's slot domain (§3.4) | (i) M2 rejects with an explicit error, a documented D5 divergence; (ii) Rust gains the same reject as a spec'd error in multi mode only; (iii) both modes, once the census in M2-01 shows zero uses |
| O-5 | Native IMOD stale-`rdx` fix (A:445-447) | The same template is embedded in Stage-1 (A:1929, A:1933). Fixing it changes the Stage-1 stream and therefore **`0xEAFC81F46B482FF8`**. Either (a) leave it and lint M2 (this plan), or (b) a deliberate Stage-1 hash bump. This is Vera's beat. |
| O-6 | Pack5 dynamic-index bloat (n = 5×bytes iterations, A:323-341) | Leave the bytes as they are (this plan; M2 rejects pack5), or fix with a golden byte bump |
| O-7 | Routing multi-function modules | Today `compile_pure_u16_to_x86_64_elf_with_args` silently compiles only `functions.first()` (A:154). Either keep that and add an explicit multi entry point (this plan), or auto-route `len()>1`, which changes the output for any existing multi-fn input |
| O-8 | Reconciling `validate_func_pack5_ops` | A:79-101 ≠ M:13911-13933. Pick one in M2-07 |

---

## 1. Ordered commit list

**Legend:**
- **[EMIT]** = touches a Rust emitter: x86-64/ELF (A:147-659), Wasm (A:694-1290), or the VM (M:~1092-1500). These must land **after C1** (the in-flight IGET/ISET work on `isa/multibyte-loads`).
- **[RUST]** = Rust, but no emitted-byte path.
- **[OMNI]** = `.omni` only.
- **Indep** = can start now, independent of `isa/multibyte-loads`.

Every Rust change is mirrored in both the monolith (`M:`) and the workspace crates (`A:`, `IR:`, `FE:`). The A≡M identity diff is part of every Rust commit's acceptance check.

**Global guard G** (run after every commit; it's the acceptance prefix for all rows):
- `cargo test`
- `omnic selftest` (counts only go up)
- `omnic pure-bootstrap`
- `omnic fixed-point` (106,242 B, `0xEAFC81F46B482FF8`)
- `omnic pure-omni-diff`
- the 77/77 verifier differential
- 4-backend goldens
- **M2-01's frozen-hash and single-mode byte-lock selftests**

### Track A: independent of `isa/multibyte-loads` (start now)

| ID | Kind | Purpose | Files | Tests / goldens added | Acceptance |
|---|---|---|---|---|---|
| **M2-00** | docs, Indep | Decision record (D-1..D-6) and the open list (O-1..O-8) | `spec/m2/DECISIONS.md`, `spec/m2/PLAN.md` | none | G passes. `git diff` touches docs only |
| **M2-01** | [RUST], Indep | **Frozen guards and a census, captured *before* any emitter change.** (1) `selftest_frozen_hashes` asserts 4 values: Phase D `.omni_bc` 159 w `0x23B8D17257CCA9C4`, `.elf` 824 B `0xE221085777B8AC47` (M:10383, `run_phase_d_pure_omni_selfhost_bootstrap`), suite 14,559 w `0x78462AE4C81DF408` (`build_pure_omni_core_suite_stream`, M:10489), Stage-1 ELF 106,242 B `0xEAFC81F46B482FF8` (`compile_pure_u16_to_x86_64_elf_with_args(stage1,45,40,90)`). All 4 reproduced today by audit probe `frozen.rs`. (2) `selftest_single_mode_byte_lock` compiles every positive single-function AOT golden in `spec/acceptance/goldens/` with the current single-mode emitter and compares it with a committed manifest `spec/m2/single_mode_lock.tsv` (name, len, FNV). (3) `selftest_shape_census` scans all frozen streams and goldens and counts: 3-op RET; `IGET` with an imm idx < 0 on an ANEW/pack5 reg; Param ≥ 4 uses; `SLEN` on an ARRAY operand; IDIV/IMOD with a non-imm divisor; mixed LBL/IMM label forms; duplicate `LABEL`; IMM label ids ≥ 83 / ≥ 4080 / masked-negative; BR_TRIT with < 3 labels; multi-function modules fed to single-mode AOT. **C1's own IGET/ISET shape scan stays C1's job**; here it's reported read-only, never gated. | M + `omni-cli` selftest list; new `spec/m2/single_mode_lock.tsv` | 3 selftests | All pass at the base commit. The census writes the expected counts (expect all 0; any non-zero becomes a listed exclusion) |
| **M2-02** | [RUST], Indep | **Lock the Rust label-resolution reference (§3) as executable spec.** Record the known divergences of Stage-1 and the Omni verifier from it. | M selftest list; `spec/m2/labels/*.omni_asm` fixtures | `selftest_label_ref` with cases L1–L12 (§3.5): Rust emitter bytes are locked; Stage-1-on-VM and Omni-verifier results are asserted *as currently divergent* (expected-diff list, so a silent change fails) | Rust bytes match the locked FNVs. The divergence list is exactly {A, B, C≥83 for Stage-1; A, C≥255 for the Omni verifier} |
| **M2-03** | [RUST], Indep | **The mandatory M2 lint and budget report** (`omnic m2-lint`). It's token-level over the Rust frontend's output and runs on every `pure/m2/*.omni` (§5.2 lists rules L-1..L-11). | M + `omni-cli`; `spec/m2/lint/pos_*.omni`, `neg_*.omni` | One negative fixture per rule, each naming the rule it must trip. This includes my measured case `w = sel_trit(c, w, w, iset(w, 15, x))`, which the Rust verifier **accepts** today. Plus a positive fixture. | Every neg fixture fails with its rule id. The pos fixture passes. The budget report prints per-fn SSA, labels and params. |
| **M2-04** | [OMNI], Indep | `pure/m2/util.omni` and `pure/m2/decode.omni`: `tok`, `sext16`, `sext32`, `in_range`, `read_operand` (packed kind:4 / value:44 / newpos:16), `fn_header`, `fn_scan` (packed maxd / maxlid / end). They use **only** the 2-op `IGET` on the `array` param (A:345-347; VM M:1288-1293). The input end comes from `slen(arr)` (VM M:1355-1361), checked against `MOD_END`. | `pure/m2/util.omni`, `pure/m2/decode.omni`, M selftest | `selftest_m2_decode_diff`: VM-run `read_operand`/`fn_header` vs `decode_omni_token_stream` (M:621-737) over every golden, the Stage-1 stream (19,731 w) and `verifier_engine` (11,893 w). Truncated and garbage streams must report a decode error at the same offset as Rust. | 0 diffs. Lint and budget: every fn ≤ 120 SSA / ≤ 12 labels (`read_operand` prototype measured 84 / 3) |

### Gate: C1 merged (3-op `IGET -1/-2/-4/-8`, 4-op `ISET -2/-4`, its frozen-shape scan)

| ID | Kind | Purpose | Files | Tests / goldens | Acceptance |
|---|---|---|---|---|---|
| **M2-05** | [RUST] frontend only, **after C1** | Surface intrinsics `ld_u8/ld_u16/ld_u32/ld_u64` and `st_u16/st_u32`, lowering 1:1 to C1's shapes. Next to the existing `iset_i32`/`iset_i64`/`iset_patch_i64` (FE:1932-1958). | FE (+M mirror) | Frontend goldens: each intrinsic → exactly one instruction with C1's shape. VM-run round-trip at offsets 0, len−w, len−w+1 (OOB semantics = C1's) | Tokens match C1's spec. G passes; the single-mode byte lock is unchanged |
| **M2-06** | **[EMIT] VM + x86 single + Wasm**, verifiers, FE; **after C1** | **D-1: `RET buf, off, len`.** VM: `Bytes(buf[off..off+len])`, with an error if out of range (VM RET today: M:1260). x86 single, bytes ret (today A:587-598): see §2.3 row `RET3-S`. Wasm (today A:1077): equivalent, depending on O-2. Rust verifier: taint and move checks on all 3 operands (RET handling M:860-884). Omni verifier: `handle_ret` (VE:707). FE: `ret_slice(a,off,len)`. | A, M, VE, FE | New 4-backend goldens `ret_slice_{basic,zero_len,full,oob}`. The OOB golden must give the VM error and a native trap with exit `T_OOB`. **Stage-1 column = N/A**: Stage-1 is frozen and always emits the 31-B RET (A:1829 dispatch). These goldens go on an explicit exclusion list in the 4-way harness. The census proves 3-op RET count = 0 in frozen streams. | G passes. The single-mode byte lock is unchanged (1-op RET still 31 B). Verifier differential 77/77 + 4 new |
| **M2-07** | **[EMIT] x86 single (reject-only)**, after C1 | **Single-mode hardening that changes no successful output.** Every fix that would change bytes Stage-1 also emits is excluded, because Stage-1 embeds the same templates (A:1929, A:1933 IDIV/IMOD; A:1840 SLEN; A:1829 RET). The new explicit compile errors: (a) Param ≥ 4 (A:225-254, `_ => {}` at A:252); (b) `SLEN` whose operand is ARRAY-typed (A:371-374 returns r11); (c) `IGET` on a pack5/ANEW reg with an imm idx < 0, which silently falls into the input read (A:310-311); (d) `IGET` on any ANEW reg with an imm idx, which is constant-folded to `movabs 0` (A:315-316). For (d), M2 code must use the 3-op form; I measured 17 B emitted. (e) BR_TRIT with < 3 label operands, which emits 0 bytes today (A:550-554); JMP/BR operand forms that `extract_label_id` (IR:23-29) can't read. (f) Reconcile the pack5 validator (O-8). | A, M | Negative goldens `aot_reject_{param4,slen_array,iget_negimm_anew,iget_imm_anew,brtrit_2lbl}`. Each asserts an error string and that no ELF is produced | The single-mode byte lock is unchanged for every positive golden. Census counts are still 0. G passes |
| **M2-08** | [RUST] verifier (no bytes), **after C1** (merge-conflict risk only) | **Module pre-check for multi-function AOT**, `verify_module_for_multi_aot(&BinModule)`: (1) every `CALL` operand 0 is `FnRef` to an existing fn (VM CALL: M:1468-1474); (2) arg count == callee `p_cnt`; (3) the call graph is acyclic (**rejects recursion**); (4) the longest call chain ≤ 64 frames, entry = frame 1, matching the VM's `call_depth >= 64` refusal (M:1026, M:1129-1134); (5) entry = fn_idx 0 else last, same rule as M:5579; entry `p_cnt ≤ 4`; callee `p_cnt ≤ 32`; (6) fn count ≤ 1024; (7) label slot budget per §3.4. **The VM's runtime behaviour is unchanged**: it keeps its runtime depth error, so no existing golden changes. | IR or verifier crate + M | `multi_reject_{recursion_direct,recursion_mutual,depth65,arity_short,arity_long,missing_fn,entry_p5}` plus `multi_ok_depth64` (chain of exactly 64 frames: accepted, and the VM run succeeds) | Each reject gives a named error. `depth64` passes in both the VM and the pre-check |
| **M2-09** | **[EMIT] x86 multi-function reference**, after C1 + M2-06 + M2-08 | **D-4.** A new public `compile_pure_u16_to_x86_64_elf_multi(stream, d0, d1, d2)`. **The existing single-mode fn is untouched** (O-7). The full byte spec is in §2. Includes the `nwords` channel, typed `SLEN`, bounded 2-op `IGET`, append guards, patch semantics = VM (M:1319-1327), the r11 rule, and Rust label resolution per function (§3). | A, M | Multi goldens, run native vs VM: `mf_call_chain_{2,9}`, `mf_params_{5,8,32}`, `mf_forward_call`, `mf_bytes_thread`, `mf_ret_slice`, `mf_iget_oob` (VM 0 == native 0), `mf_slen_array`, `mf_input_trunc` (64 KiB + 1 → exit `T_TRUNC`), `mf_input_odd` (odd bytes → `T_TRUNC`), `mf_append_cap` (→ `T_CAP`), `mf_patch_oob` (no write, as in the VM), `mf_labels_L1..L12` (§3.5). `pure/compiler.omni` (7 fns, fn#0 last-declared per FE:3069-3075) and `verifier_engine` (58 fns) compile and match the VM. | G passes. The single-mode byte lock is unchanged. Every multi golden: native stdout == VM result |
| **M2-10** | **[EMIT] Wasm multi-function + CALL + evaluator**, after C1 + **the in-flight Wasm IGET/ISET fix** + M2-08 | **D-6.** Multiple functions plus `call` in the Wasm emitter (today single-fn, `functions.first()` at A:696, CALL unsupported at ISA:271-286). Arena in linear memory, per the in-flight Wasm fix. The evaluator (A:1277-1290, i64-only today) gains `call` and a depth limit of 64 matching the VM; whether it has one today is **UNVERIFIED**. The bytes result channel follows O-2. | A, M, ISA (`is_wasm_aot_supported_opcode` += 0x7F48) | The same `mf_*` set on Wasm == VM. `verifier_engine` on Wasm == VM over the 77 cases | 4-backend parity (VM, x86-single where applicable, x86-multi, Wasm) on every `mf_*`. G passes |

### Track C: M2 in Omni (no Rust emitter changes; needs M2-05/06; tests need M2-09/10)

| ID | Kind | Purpose | Files | Tests / goldens | Acceptance |
|---|---|---|---|---|---|
| **M2-11** | [OMNI] | `pure/m2/arena.omni` (`m2_init`, `chk_cap`, `g_get/g_set`, label-table helpers with `off+1` encoding) and `pure/m2/elf.omni` (120-B header, single prologues, multi stub and trap stubs per §2) | `pure/m2/arena.omni`, `pure/m2/elf.omni` | VM: `m2_init` then layout asserts (§3.6). The header and stub bytes == the Rust reference bytes for both ret types and both modes | Byte-equal. Lint passes. `m2_init` ≤ 6 labels |
| **M2-12** | [OMNI] | Single-function emitter subset: `emit_load/store/imm64`, ALU, mem, ctl, a **class-table dispatch** (128-B op→class table, then ≤ 2 `match_trit` levels per fn) | `pure/m2/emit_*.omni`, `pure/m2/dispatch.omni` | **D1**: for every positive single-fn AOT golden, `RustEmit(T_g,d) == M2_vm(T_g,d)`, and the resulting ELF stdout == VM. Exclusions are the census list + O-4 ids + inputs > 28,672 words or expected output > `0x20000 − 8R` (§6.2 harness guard) | 0 diffs. Exclusions printed by name |
| **M2-13** | [OMNI] | Legacy anchor | test only | **D2**: `M2_vm(build_step7_stage1_compiler_stream(),45,40,90)` = 106,242 B, `0xEAFC81F46B482FF8` | Exact |
| **M2-14** | [OMNI] (VE edits allowed; VE isn't in the frozen suite, VE:16-17) | Pass 0: re-hosted verifier **with the budget splits** (§5.1). Fixes the Omni verifier's label keys to match Rust (§3.3): today it normalises LBL→k (VE:426) and caps ids at 0..255 (VE:340-358), giving the measured divergences A and C≥255. Adds module checks equal to M2-08. | `pure/m2/verify/*.omni`; VE (same fix, so the existing differential gains coverage) | **D4**: 77/77 + L1–L12 + the M2-08 reject set; identical `(code, offset)` lists and **no ELF** on reject | 0 diffs. `omni_verify_module` split ≤ 12 labels, `handle_dest` split ≤ 120 SSA |
| **M2-15** | [OMNI] | Multi-function emission: CALL, stack params, stub, traps, forward-call two-pass | `pure/m2/emit_call.omni`, `pure/m2/driver.omni` | **D3**: `RustEmitMulti == M2_vm` on `pure/compiler.omni`, the re-hosted verifier, and all `mf_*`. Pass assertions `len(pass2)==len(pass1)` and `fn_off_p2[i]==fn_off_p1[i]` for all i | 0 diffs |
| **M2-16** | [RUST] CLI + [OMNI] seed | Commit seed `pure/m2/m2.omni_bc` (T) + FNV; drift check `RustFrontend(pure/m2/*.omni) == T`; `omnic m2-fixed-point` (§4) | `pure/m2/m2.omni_bc`, `omni-cli` | §4 procedure as a selftest, with an explicit SKIPPED line on non-Linux-x86-64 | S1 == S2 == S3 == V == W (O-2 form). Run twice |
| **M2-17** | docs | README/spec honesty: "token-level self-hosted (Rust still tokenizes)"; pack5 excluded; Stage-1 N/A for new shapes; Phase D is Rust-vs-Rust; O-5 status | `README.md`, `spec/*.md` | none | Scarlett's honesty review |

**Count: 18 commits** (M2-00..M2-17).

**Rust emitter commits (sequence after C1):**
- M2-06: VM, x86, Wasm
- M2-07: x86, reject-only
- M2-09: x86 multi
- M2-10: Wasm + evaluator, which also waits for the in-flight Wasm IGET/ISET fix

Other Rust, but not emitters: M2-01, M2-02, M2-03 (selftests and tooling), M2-05 (frontend, needs C1 shapes), M2-08 (verifier, after C1 only for conflicts), M2-16 (CLI).

**Independent of the branch:** M2-00..M2-04.

**Dependency chain:**

```
M2-00 → M2-01 → {M2-02, M2-03, M2-04}  ─┐
C1 ───────────────────────────────────────┼→ M2-05 → M2-06 → M2-07
                                          │           └→ M2-08 → M2-09 → M2-10 (also needs the Wasm IGET/ISET fix)
M2-05 + M2-06 → M2-11 → M2-12 → M2-13 → M2-14 → M2-15 (needs M2-09) → M2-16 (needs M2-10 for W) → M2-17
```

---

## 2. Native multi-function byte spec (M2-09 reference = what M2 must emit)

**Rule 0:** every encoding is fixed-width (always disp32, imm32, imm64, rel32), so pass-1 and pass-2 sizes are equal by construction.

Single-mode encodings are reused verbatim where noted: loads A:225-254, store A:256-260, ALU and the others at A:429-586.

### 2.1 Layout

```
ELF64 hdr + 1 PT_LOAD      120 B   exactly as A:631-655 (p_flags 5, base 0x400000, entry 0x400078)
entry stub                 fixed   §2.2
trap stubs                 3×12 B  BF imm32(T_*) B8 3C 00 00 00 0F 05   (exit(T_*))
functions                  stream order; fn#0 (entry) is physically last for surface code (FE:3069-3075)
```

Stack after the stub: `rbx = rsp`.

| Range | Contents |
|---|---|
| `[rbx, +0x100000)` | 1 MiB arena |
| `[rbx+0x100000, +0x10000)` | input window |
| `[rbx+0x110000]` | `nwords` (= `rbp-0x10`) |
| `[rbp-8]` | 1-byte probe |

The total reservation is `0x110010` (16-aligned).

### 2.2 Entry stub

All bytes below are objdump-verified (audit probes).

| Piece | Bytes | Size | Note |
|---|---|---|---|
| frame | `55 48 89 E5 48 81 EC 10 00 11 00` | 11 | The same `push rbp; mov rbp,rsp` as A:174-176, so the CLI parsers' `[rbp+8]` argc and `[rbp+0x18+]` argv offsets (A:121-123) stay valid |
| arena base | `48 89 E3` | 3 | `mov rbx,rsp` |
| nwords = 0 | `48 C7 83 00 00 11 00 00 00 00 00` | 11 | |
| defaults + CLI | byte-identical to single mode for the entry's ret type: bytes A:181-187 + 3×`emit_cli_arg_parser` A:208-210; non-bytes A:212-222 | 37+195 / 37+260 | **Both layouts must be reproduced**: bytes uses r12 = 0 and defaults in r13–r15; non-bytes uses defaults in r12–r14 and r15 = 0 |
| read (bytes entry only) | `48 83 7D 08 02 7C 5B 48 8B 7D 18 31 F6 31 D2 B8 02 00 00 00 0F 05 48 85 C0 78 47 48 89 C7 48 8D B3 00 00 10 00 BA 00 00 01 00 31 C0 0F 05 48 85 C0 7C 28 A8 01 0F 85 r32(T_TRUNC) 48 D1 E8 48 89 83 00 00 11 00 31 C0 48 8D 75 F8 BA 01 00 00 00 0F 05 48 85 C0 0F 8F r32(T_TRUNC) B8 03 00 00 00 0F 05` | 98 | open → read 0x10000 B → **save `nread>>1` as nwords** (fixes the discarded count at A:189-207). An **odd byte count → `T_TRUNC`**. A **1-byte probe read > 0 → `T_TRUNC`**, which replaces silent truncation (measured today in single mode: 28,830 w → wrong ELF, exit 0). An open or read error leaves nwords = 0, the same as the single-mode skip at A:190-198. |
| call entry | `41 57 41 56 41 55 41 54 45 31 DB E8 r32(fn#0) 48 83 C4 20` | 20 | Pushes r15..r12 so entry param k = `[rbp+16+8k]`. **r11 = 0 here** (arena empty) |
| result, non-bytes | the 41-B tail of A:602-614 (rax → 8 B stdout, exit 0) | 41 | |
| result, bytes | `48 8D 34 03 BF 01 00 00 00 B8 01 00 00 00 0F 05 31 FF B8 3C 00 00 00 0F 05` | 25 | Bytes fns return **rax = off, rdx = len** |

### 2.3 Per-function and per-opcode (multi mode)

| Item | Bytes | Size | Ref / note |
|---|---|---|---|
| fn prologue | `55 48 89 E5 48 81 EC imm32` | 11 | imm32 = `align16(8·(maxd+1))` |
| load Param k | `48 8B 85/8D disp32(16+8k)` | 7 | replaces the r12–r15 forms of A:227-238 in multi mode |
| load Reg, Imm, Trit; store | as A:239-260 | 7 / 10 / 7 | |
| CALL | per arg, reverse: `L(arg)→rax` + `50`; then `E8 rel32`, `48 81 C4 imm32(8·n)`, store if dest | Σ(L+1)+12+S | always imm32, even for n = 0 |
| RET, non-bytes | `L0` + `C9 C3` | L0+2 | |
| RET bytes, 1-op | `31 C0 4C 89 DA C9 C3` | 7 | off = 0, len = r11 |
| RET3-M `buf,off,len` | `L(off)→rax`, `L(len)→rcx`, `48 85 C0 0F 88 r32(T_OOB) 48 85 C9 0F 88 r32(T_OOB) 48 8D 34 08 4C 39 DE 0F 87 r32(T_OOB) 48 89 CA C9 C3` | L+L+36 | |
| RET3-S (single mode, M2-06) | same checks, then `48 8D B4 05 00 00 FE FF 48 89 CA BF 01 00 00 00 B8 01 00 00 00 0F 05 31 FF B8 3C 00 00 00 0F 05` | L+L+63 | base `rbp-0x20000` as A:591. The single-mode trap stub is appended after the code only when 3-op RET is present, so existing ELFs don't change |
| ANEW | `45 31 DB` + S | 10 | A:302. Lint L-2 allows exactly one |
| 2-op IGET (input) | `L(idx)→rax`, `48 3B 83 00 00 11 00 73 0A 0F B7 84 43 00 00 10 00 EB 02 31 C0` + S | L+22+S | **Bounded: idx ≥ nwords (unsigned) → 0**, the same as the VM's OOB → `Int(0)` (M:1302). Replaces the unbounded 8-B form proposed in the design |
| SLEN bytes / array | `4C 89 D8` / `48 8B 83 00 00 11 00` + S | 10 / 14 | **Typed SLEN fix.** VM SLEN semantics M:1355-1361. The type comes from the header param type or the dest type |
| append guard (before every appending ISET) | `49 8D 83 imm32(k) 48 3D 00 00 10 00 0F 87 r32(T_CAP)` | 19 | k = the number of bytes the instruction appends. M2's own `chk_cap` gives the VM-visible error first; the native trap is a backstop |
| ISET bytes | guard + per byte `L` + `42 88 84 1B 00 00 00 00 49 FF C3` | 19+Σ(L+11)+D | rbx-based form of A:363 |
| ISET -4 / -8 | guard + `L2` + `42 89 84 1B 00 00 00 00 49 83 C3 04` / `4A 89 84 1B 00 00 00 00 49 83 C3 08` | 19+L+12+D | A:352 / A:355 |
| ISET -9 patch | `L2→rcx`, `L3→rax`, `48 85 C9 78 0F 48 8D 71 08 4C 39 DE 77 06 48 89 04 0B` | L+L+18+D | **OOB → no write, matching the VM** (M:1319-1327 is a no-op when `off+8 > len`). The single-mode A:359 writes unconditionally |
| C1 shapes (3-op IGET -1/-2/-4/-8, 4-op ISET -2/-4) | candidates `0F B6 04 0B` / `0F B7 04 0B` / `8B 04 0B` / `48 8B 04 0B`; `66 89 04 0B` / `89 04 0B` (off in rcx) | — | **Placeholders.** Bounds and OOB semantics follow C1 as landed; this plan doesn't define them |
| D suffix | `4C 89 D8` + store | 10 | A:366-369 |
| labels / JMP / BR | as A:537-576 | 0 / 5 / L+21 / L+10 | resolved per §3 |

### 2.4 Register rules (normative for both the Rust reference and M2)

- **r11 rule:** r11 is the single global arena length. It's **never saved or restored by anyone**. Callees append to it, and syscalls (which clobber r11) occur only in the stub and traps.
- **ANEW** appears only once, before any CALL. Lint L-2 enforces it, because a callee ANEW would truncate the caller's arena (A:302).
- **rbx** is written only by the stub.
- rax, rcx, rdx, rsi, r8, r9 are scratch. No value lives in a register across instructions: every dest is stored to its slot.
- r12–r15 are used only by the stub. Callee-save is therefore empty and caller-save is trivial.

---

## 3. How M2 matches Rust label resolution

### 3.1 The reference behaviour (Rust single mode, verifier + emitter)

1. **Key.** `extract_label_id` (IR:23-29 = M:523-529):
   - `Label(tok)` → `tok`, i.e. the raw `0x7EF0+k` (the decoder keeps the token: M:567).
   - `ImmInt(v)` → `v & 0xFFFF`.
   - So:
     - `IMM_I16 -1` → `0xFFFF`
     - `IMM_I32` → its low 16 bits
     - **LBL k ≠ IMM k**
     - **LBL k ≡ IMM 0x7EF0+k**
     - Any other operand → `None`
2. **Definitions.** The verifier collects every `LABEL` with an extractable key (M:811-817).
3. **Uses.**
   - JMP checks *all* operands. BR_TRIT, BR_POS and BR_NEG check `operands[1..]` (M:886-891).
   - No label operands → `UNRESOLVED_LABEL` "Missing" (M:895-897).
   - Any key not defined → `UNRESOLVED_LABEL` (M:899-903).
   - Then AOT refuses an unverified stream (A:155-158).
   - **Duplicates are not rejected.**
4. **Emission.**
   - `LABEL` records `(key, code.len())` in instruction order (A:537-541).
   - JMP uses operand 0 (A:543).
   - BR_TRIT needs operands 1, 2 and 3 all extractable, **otherwise it emits 0 bytes** (A:550-554).
   - BR_POS and BR_NEG use operand 1 (A:569).
5. **Resolution.** `label_offsets.iter().find(...)` means **first definition wins** (A:621-626). `rel32 = target − (patch+4)`, code-relative (A:627).

### 3.2 Measured divergences (audit probes `lbldiv.rs`, `vdiff.rs`)

| Case | Rust (reference) | Stage-1 on VM (A:1764-1772 normalises LBL→k; A:2062-2066 sel_trit overwrite = last wins; `MAX_LABELS=83` A:1612) | Omni verifier (VE:426 normalises LBL→k; VE:340-358 keys 0..254 only) |
|---|---|---|---|
| **A** mixed: `LABEL LBL1` + `JMP IMM 1` | reject `UNRESOLVED_LABEL` | accepts, 484 B | **accepts** (verdict mismatch) |
| **B** duplicate `LABEL 1` | first wins: `E9 00 00 00 00` | last wins: `E9 11 00 00 00` (same length, different bytes) | agrees with Rust (no reject) |
| **C** IMM id 83 | ok | ok | ok |
| **C** IMM id 255 / 256 / 5000 / −1 | ok | 5000: same length, **different bytes** (silent miscompile for ids ≥ 83) | **rejects** `UNRESOLVED_LABEL` for 255, 256, 5000 and −1 |

D2 (the Stage-1 anchor) is unaffected: Stage-1's own stream is IMM-only, ids 0..82, no duplicates (measured in `lbl.rs`).

### 3.3 Plan: M2 reproduces Rust exactly on its accepted domain

These rules apply to pass 0 (the verifier), pass 1 and pass 2.

**Keys.** Compute the key exactly as in §3.1:
- LBL: `key = tok`
- IMM16: `key = sext16(w) & 0xFFFF`, i.e. the raw u16 word
- IMM32: `key = lo & 0xFFFF`
- anything else: no key (the same consequences as Rust: reject if it's a use; ignore if it's a def)

**Key → slot (a bijection on the accepted domain):**

| Key | Slot |
|---|---|
| `0x7EF0 ≤ key ≤ 0x7EFF` | `key − 0x7EF0` (0..15). LBL k and IMM `0x7EF0+k` alias, as they do in Rust |
| `key < 4080` | `16 + key` (16..4095) |
| any other key | `error[LABEL_ID_RANGE]` (O-4). Census today: 0 such ids in frozen streams |

**Per-function span** = `16 + (max IMM key + 1)` when there are IMM keys, else 16. `lbase` is the running sum, and each function needs `lbase + span ≤ 4096`, else `BUDGET_EXCEEDED`. The surface self-image needs ≤ 16 per fn; Stage-1 needs 99.

**Table** (D-3): u32 at `0x10000 + 4·(lbase+slot)`, holding `code_off + 1`; `0` = undefined. M2 zeroes the span on function entry in pass 1.

**Pass 1** (`LABEL`): write only if the entry is 0 → **first wins**.

**Pass 2:**
- At every *first* `LABEL` def, assert `code_off + 1 == entry`, else `EMIT_SIZE_MISMATCH`.
- At each use, an entry of 0 is unreachable, because pass 0 already rejected it, matching M:899-903. Assert it anyway.
- `rel32 = (entry − 1) − (patch_off + 4)`.

**Emission edge cases copied verbatim:**
- BR_TRIT with < 3 extractable labels emits 0 bytes. Where M2-07 lands, single mode rejects it instead, and M2 then rejects too: same verdict either way.
- JMP extra operands: checked, then ignored.
- BR_POS/BR_NEG use only operand 1 for emission but check all of `[1..]`.

**Pass 0:**
- Use the same key and definition set.
- No duplicate rejection (Rust doesn't reject them).
- "Missing" = no label operands.
- **The Omni verifier (VE) gets the same key function in M2-14**, which removes divergences A and C≥255.

**Stage-1 isn't changed** (frozen). Its divergences stay documented, and D1 never uses Stage-1 as a reference for the cases in §3.2.

### 3.4 Alternative (if O-4 goes the other way)

Rust gains a spec'd `LABEL_ID_RANGE` reject (keys ≥ 4080 outside the alias window), in multi mode or in both modes, *before* M2-12. M2 then matches Rust over its whole input domain, with no documented divergence. This is allowed only because successful outputs don't change, which the M2-01 census shows.

### 3.5 Label differential tests (M2-02 lock → M2-09 x86-multi → M2-12/14/15 M2)

| # | Stream | Expected (Rust is the oracle; M2_vm, x86-multi and Wasm must equal it) |
|---|---|---|
| L1 | LBL def + LBL use | bytes equal |
| L2 | IMM def + IMM use, ids 0..82 (Stage-1 style) | bytes equal |
| L3 | mixed `LABEL LBL1` / `JMP IMM 1` (case A) | reject `UNRESOLVED_LABEL` @ same offset, no ELF |
| L4 | `LABEL IMM 0x7EF1` / `JMP LBL1` (alias) | accepted, bytes equal |
| L5 | duplicate `LABEL 1`, forward and backward uses (case B) | first-def target: `E9 00 00 00 00` |
| L6 | IMM ids 83, 254, 255, 256, 4079 (case C) | accepted, bytes equal |
| L7 | IMM ids 4080, 5000, −1 (`0xFFFF`), IMM32 `0x00010001` (masks to 1) | O-4 (i): M2 `LABEL_ID_RANGE`, Rust accepts (listed); (ii)/(iii): both reject. IMM32 `0x00010001` = key 1 → accepted, equal bytes |
| L8 | BR_TRIT with 2 labels | Rust 0 bytes (or reject after M2-07); M2 identical |
| L9 | JMP with 2 label operands, second undefined | reject (Rust checks all JMP operands) |
| L10 | span budget: one fn with IMM id 4079 + a second fn | `BUDGET_EXCEEDED` in M2; Rust-multi applies the same check (M2-08 item 7) |
| L11 | 64 functions × 15 labels (surface max) | accepted, `lbase` = 64×16 = 1024 |
| L12 | Stage-1 stream (83 IMM labels) | 106,242 B, `0xEAFC81F46B482FF8` (D2) |

---

## 4. Fixed-point plan (no Rust host intrinsics, no CLI args)

```
T  = pure/m2/m2.omni_bc        committed seed. M2-16 selftest: RustFrontend(pure/m2/*.omni) == T (drift fails CI)
S1 = compile_pure_u16_to_x86_64_elf_multi(T, d0, d1, d2)     Rust reference; d0..d2 are fixed constants in the selftest
S2 = $ ./S1 T.omni_bc  > S2     # NO further argv: S1 uses its baked defaults d0..d2 (stub defaults, §2.2)
S3 = $ ./S2 T.omni_bc  > S3     # NO further argv: S2 baked the same d0..d2, because M2 received them as params
V  = VM(T, entry fn#0, [U16Array(T), d0, d1, d2])          4th witness
W  = Wasm(T)(T, d0, d1, d2)                                 5th witness (D-6). Form per O-2: bytes, or (len, FNV)
ASSERT  S1 == S2 == S3 == V   (byte equality; print len + FNV-1a64)   and  W ≡ S2 (per O-2)
```

**Why it needs no host intrinsics:**
- S2 and S3 are produced by native ELF processes whose only syscalls are `open/read/close/write/exit`: stub §2.2, traps §2.1, RET §2.3.
- All addressing is `rbp`/`rbx`-relative, so ASLR can't change bytes.
- The VM and Wasm witnesses run the same T.

**No-args rule:**
- The fixed-point command passes exactly one argv (the input path).
- If argv[2..4] were passed, the CLI parsers (A:208-210) would override the defaults, and S2 would bake different immediates than S1.
- The selftest asserts that `argc == 2` was used and that the 3 movabs defaults in S2's stub equal d0..d2.

**Determinism checks:**
- Run the whole chain twice; it must give the same FNV.
- Run S2 under `setarch -R` and without it (ASLR on/off); both must be equal.
- Write outputs to regular files, not pipes, because the single write isn't retried (A:588-598; §2.2 result tail).
- Lint rule L-9 forbids reading an uninitialised slot.

**Showing the frozen hashes are unchanged** (every commit, via M2-01):
1. `selftest_frozen_hashes`: the 4 values in M2-01, all reproduced at base by audit probe `frozen.rs`.
2. `omnic fixed-point` still gives 106,242 B `0xEAFC81F46B482FF8`. Natively, stage1 → stage2 → stage3 are byte-identical (re-verified in audit probe `fp.rs`).
3. `selftest_single_mode_byte_lock` is unchanged. That proves M2-06 (1-op RET), M2-07 (reject-only) and M2-09 (new entry point) altered no existing ELF.
4. A diff-scope check in review: no diff in `PURE_OMNI_CORE_SUITE_MODULES` (M:10503-10522), the embedded module strings behind `lookup_omni_stdlib_module` (M:4425ff), `build_step7_stage1_compiler_stream` (A:1590-2170 = M:15422-16002), or `pure/compiler.omni`.
5. The census still reports 0 for the new shapes in frozen streams (3-op RET here; C1's shapes per C1's own scan).

---

## 5. Budgets, splits, lint (folded from the review)

### 5.1 Caps and targets

- SSA cap 192 (FE:1632-1642). **Usable labels = 15** (ids 1..15, FE:1644-1654).
- Label costs: value `if_pos` 2, `match_trit` 4, `while` 3, `do_while` 1, `sel_trit` 0.
- Literals must fit i32 (assembler M:1863). `fn_id` attr only 0..15 (FE:780).
- The entry is the last-declared fn (FE:3069-3075), so `m2_main` goes last in `pure/m2/` concatenation order.
- **Targets (budget report gate):** ≤ 120 SSA and ≤ 12 labels per fn.

**Required splits:**

| Function | Today | Split |
|---|---|---|
| `omni_verify_module` | 14 labels | `verify_next_fn` (loop body) + `finish_status` |
| `handle_dest` | 134 SSA | `dest_ssa_check` / `dest_taint` / `dest_arena_state`, sequential |
| `m2_main` | prototype 12 labels | straight-line: `m2_init` (≤ 6 labels), `m2_pass(…,0/1/2)` ×3, `ret_slice`. 0 labels |
| dispatch | ≤ 3 `match_trit`/fn (12); the 4th = 16 > 15 | class table + 2 levels |
| `h_iset` | 11 labels | sub-emitters as separate fns |
| `emit_fn` | 96 SSA | `fn_scan` + `emit_fn_prologue`; ≤ 70 SSA |
| `read_operand` | 84 SSA | ok, but keep validation out of it |

No split introduces recursion. Every split is a sequential `a = f(a, …)` call; args are eager and pure.

### 5.2 `m2-lint` rules (mandatory, M2-03; gate for M2-04+)

| Rule | Requirement | Why |
|---|---|---|
| L-1 | No `SEL_TRIT` operand is bytes-typed | eager evaluation would append natively, while the VM discards. Measured: the Rust verifier accepts it |
| L-2 | Exactly one `ANEW` in the module, in `m2_init`, before any CALL | callee ANEW resets r11, A:302 |
| L-3 | Linear bytes: a bytes reg that has been consumed (ISET/CALL/RET into another dest) is never read again unless redefined. At most one bytes-typed arg per CALL | the VM clones values (M:1095, M:1468-1470); native has one r11 |
| L-4 | 2-op `IGET` only on ARRAY-typed operands | A:345-347 reads input whatever the buffer; the VM reads bytes |
| L-5 | Arena reads only via C1's 3-op forms (`ld_*`) | avoids pack5 folding, A:309-321 |
| L-6 | No pack5 ops (30/31/34/35/3F) | |
| L-7 | `IDIV`/`IMOD` divisor must be a non-zero, non-−1 immediate | native IMOD returns stale rdx: measured `1234 % 0` = 1234 vs VM 0 (A:445-447 vs M:1344). See O-5 |
| L-8 | No recursion; call depth ≤ 64 frames; arity matches | same as M2-08 |
| L-9 | Every read slot is written on all paths (join-slot rule of `8160842`) | |
| L-10 | Literals fit i32 | M:1863 |
| L-11 | Budget: ≤ 120 SSA, ≤ 12 labels | |

---

## 6. Capacity framing and guards (folded from the review)

### 6.1 Units

- **28,672 words** = 0xE000 B / 2, the single-function native read window (A:200-201, region `[rbp-0x2E000, rbp-0x20000)`).
- **32,768 words** = 0x10000 B, the multi-mode window (§2.2; O-3 may double it).
- Single-mode output = `0x20000 − 8R` B, including the 120-B header:
  - R = 98 (Stage-1): 130,288 B
  - R = 192: 129,536 B
- Measured with native Stage-1:
  - **28,830 words → silent truncation**, wrong ELF, exit 0.
  - **131,180 B output → SIGBUS.**
  - 130,288–131,072 B is a silent slot-corruption zone.
- Stage-1 self-image: 106,242 B = 81.5% of the cap.
- Multi mode: `0x8000 + 2N ≤ 0x100000` → N ≤ 507,904 B. My model puts M2's ELF at about 69–95 KB (3.4–3.5 B/word); the design's estimate was 140–170 KB. Either fits.
- The native stack peaks at about 1.18 MiB. Document `ulimit -s ≥ 2 MiB`, or fail fast in the selftest.

### 6.2 Guards

- **Multi mode:** truncation, odd-byte, append-cap, OOB-slice and patch-OOB behaviour per §2 (traps or VM-equal no-ops).
- **Single mode:** bytes are frozen, so no in-binary guards. The D1/D2 harness refuses (and names) any input > 28,672 words or expected output > `0x20000 − 8R`, so the cliffs can never pass silently.
- **VM wall time:** the VM clones on every register read, so cost is superlinear in arena size (measured: 256 KiB fill ≈ 0.72 s). §6.5's O(n) check records wall time as well as steps.

---

## 7. Invariant checklist per commit

✔ = preserved, with the reason. Zero external crates in every commit (✔ all).

| Commit | 192 SSA / 15 labels | 64-deep calls | eager args | no heap/maps/GC | no recursion | opcode table unchanged | frozen hashes |
|---|---|---|---|---|---|---|---|
| M2-00 | n/a | n/a | n/a | n/a | n/a | ✔ | ✔ docs only |
| M2-01 | n/a | n/a | n/a | ✔ selftest only | n/a | ✔ | ✔ **adds** the guards |
| M2-02 | n/a | n/a | n/a | ✔ | n/a | ✔ | ✔ tests only |
| M2-03 | ✔ enforces ≤120/≤12 | ✔ enforces ≤64 | ✔ enforces L-1/L-3 | ✔ | ✔ enforces L-8 | ✔ | ✔ |
| M2-04 | ✔ (report) | ✔ chain ≤ 3 | ✔ | ✔ packed i64 only | ✔ | ✔ | ✔ new files outside the suite |
| M2-05 | ✔ | n/a | ✔ intrinsics are single instructions | ✔ | n/a | ✔ no new opcode (C1 shapes) | ✔ byte lock |
| M2-06 | ✔ | n/a | ✔ | ✔ | n/a | ✔ new operand shape on 0x7F4A, no renumbering | ✔ census 0 + byte lock |
| M2-07 | n/a | n/a | n/a | ✔ | n/a | ✔ | ✔ reject-only; byte lock |
| M2-08 | n/a | ✔ **static ≤64 frames** | n/a | ✔ adjacency over fn_idx in Vec (Rust host side) | ✔ **rejects cycles** | ✔ | ✔ VM runtime unchanged |
| M2-09 | ✔ frame = 8·(maxd+1) ≤ 1,552 | ✔ needs M2-08 | ✔ args pushed after evaluation | ✔ flat 1 MiB stack arena | ✔ | ✔ 0x7F48 newly lowered, not renumbered | ✔ new entry point; byte lock |
| M2-10 | ✔ | ✔ evaluator depth 64 | ✔ | ✔ linear memory only | ✔ | ✔ | ✔ |
| M2-11..M2-15 | ✔ lint gate (≤120/≤12) | ✔ chain ≈ 9 | ✔ L-1/L-3 | ✔ single flat arena | ✔ L-8 | ✔ | ✔ `pure/m2/` only; VE isn't in the suite (VE:16-17) |
| M2-16 | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ plus the §4 procedure |
| M2-17 | n/a | n/a | n/a | n/a | n/a | ✔ | ✔ docs only |

---

## Appendix A: evidence (audit probes, not committed)

| Probe | Shows |
|---|---|
| `frozen.rs` | 4 frozen values reproduced |
| `fp.rs` | native stage1 == stage2 == stage3 |
| `enc.rs` | every single-mode encoding row |
| `imod.rs` | the IMOD stale-rdx bug |
| `lbl.rs` | Stage-1 labels 0..82, IMM-only; verifier budgets |
| `lbldiv.rs` / `vdiff.rs` | label divergences A, B, C (Stage-1 and the Omni verifier) |
| `budget.rs` + `proto/m2proto.omni` | prototype budgets |
| `cliff.rs` | input truncation and output SIGBUS |
| `sizemodel.rs` | output projection |
| `vmcost.rs` | VM clone cost |
| objdump transcripts | every new byte string in §2 |
