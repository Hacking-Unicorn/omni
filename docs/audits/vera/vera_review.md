# Vera Parity: review of `backend_parity_matrix.md`

## 1. Scope and commit

- **Repo / commit:** `github.com/Hacking-Unicorn/omni` @ `8160842` (detached; *Fix value-returning if_pos SSA join-slot holes…*). My own clone: `/workspace/audit-vera`. I did not touch `/workspace/omni`, `/workspace/audit-parity`, or the matrix file. No commits, pushes, branches, PRs, or issues.
- **Build:** `rustc -O --edition 2021 omni_rust_bootstrap.rs` (rustc 1.85.1, zero crates), output in `/workspace/vera-build/`. Baseline at this commit, CONFIRMED-RUN: `omnic selftest` reports **416 checks PASS**, and `omnic acceptance-diff` reports **39 goldens PASS**.
- **Differential harness:** `/workspace/audits/vera_review_repro/harness.rs` loads the monolith with `#[path]` and runs each surface program through:
  - **VM:** `OmniBinaryVm::execute_function` directly.
  - **AOT-first:** `execute_omni_surface_aot_first`, i.e. what `run` and acceptance actually use.
  - **Wasm:** `compile_pure_u16_to_wasm` + `execute_wasm_module_i64`.
  - **x86-64 ELF:** `compile_pure_u16_to_x86_64_elf_with_args`, then **executed natively**, decoding the 8-byte stdout as an i64.
  - **Stage-1 ELF:** `compile_omni_surface_via_stage1_to_elf`, then **executed natively**.
  - It also prints whether the Stage-1 ELF equals the host ELF.
  
  All test programs are in `/workspace/audits/vera_review_repro/*.omni`. Unless noted, the input is x=7.
- **What "Stage-1" means here:** the matrix is right that it means `build_step7_stage1_compiler_stream` (`omni_rust_bootstrap.rs:15422-16002`). That is a u16 program built by Rust and run on the VM, not `pure/compiler.omni`. `pure/compiler.omni` (117 lines) is a classifier/validator kernel. It has no ELF emitter.
- **Tags:** CONFIRMED-RUN means I executed it. CONFIRMED-READ means I verified it by reading source at `8160842`. UNCERTAIN comes with a reason. Every line number below is in `omni_rust_bootstrap.rs` unless another file is named.

## 2. Top-10 verification table

**Result: 9 AGREE (several refined), 1 CORRECTED, 0 UNCERTAIN.**

| # | Matrix claim (short) | Verdict | Evidence (file:line @8160842) | Tag |
|---|---|---|---|---|
| 1 | Wasm `ISET` only updates the length. Wasm `IGET` of a runtime buffer returns 0. | **AGREE**, refined | ISET arm `14752-14783` only adds a delta to `buf_len_local`; mode `-9` gives delta 0. The IGET fallback `14747-14750` is `i64.const 0`. Refinement: the const path `14708-14746` comes from `collect_func_pack5_map` (`13836-13883`). That map is **flow-insensitive** and **silently skips non-immediate operands**, so it can return the *wrong* value, not just 0 (see N4). Run: `iget_buf_dyn` (stores x, reads it back) gives VM **7**, Wasm **0**. | CONFIRMED-RUN |
| 2 | `byte_buf_golden` cannot catch #1. | **AGREE**, refined | The acceptance entry is at `8904`. The selftest at `17402-17405` checks only that the u16 stream matches. There is a third use the matrix did not cite: the 4-way block at `16909-16945` compares VM result, Wasm result, and Stage-1 ELF vs host ELF **bytes**. It still observes only `slen`+`cmov` (315). No backend ever runs `iget` on that buffer. | CONFIRMED-READ + RUN (315 on all 4) |
| 3 | x86 and Stage-1 `IGET` do not read `ISET` buffers. | **AGREE**, refined | x86 fallback `14177-14179`: `movzx eax, word [rbp+rax*2-0x2E000]`. Stage-1 does the same at `15832-15836`. Refinement: `rbp-0x2E000` is the buffer that `read(argv[1], …, 0xE000)` fills, and only in the **TY_BYTES-return prologue** (`14021-14039`, same as Stage-1 `15518-15536`). This is how the self-compiler reads its input file when it runs natively. In an i64-returning program the region is untouched stack (it reads 0). The ISET heap is at `[rbp+r11-0x20000]`, so `IGET` index ≥ 0x7000 would alias it as u16 words. Run: `iget_buf_dyn` gives x86 0 and Stage-1 0, while VM gives 7. | CONFIRMED-READ + RUN |
| 4 | Stage-1 is not a parity twin of x86 for 7 gated opcodes (`ARENA DROP TPACK5 TUNPACK5 TDOT EGATE TLOGIC`). | **CORRECTED** | **ARENA is byte-identical to host x86.** It runs the generic path (decode operand → `lbl_store_dst` `15943-15947`), which is exactly x86's identity arm `14129-14132`. Run: the `arena_lifecycle_golden` Stage-1 ELF is host + exactly 14 bytes, and all 14 come from the DROP misdecode. So **6** opcodes diverge: DROP, TPACK5, TUNPACK5, TDOT, EGATE, TLOGIC. Stage-1 `IGET` of constant buffers also diverges (no pack5 fold): `iget_buf` gives x86/Wasm/VM **20** and Stage-1 **0**. Run: `pack5_direct_aot` gives VM/Wasm/x86 **2422** and Stage-1 **120**. `pack5_trit_vec_golden` gives 2455 vs Stage-1 **2004**. `p5_dyn` gives 41 vs **11**. The matrix's sub-claim that "pack5_direct_aot / pack5_trit_vec compare VM/Wasm/x86" is also wrong; see §4 C3/C4. | CONFIRMED-RUN |
| 5 | Sanitizers are identity on all AOT backends. | **AGREE**, extended | x86 `14129-14132`, Wasm `14649-14652`, Stage-1 copy list `15692-15695`. VM `1477-1479` (`SAN_PRM` is identity even on the VM). New: the verifier accepts `san_sql(x)` with **x: i64**. The VM then returns `Str("")` and every AOT backend returns `7` (`san_int.omni`). | CONFIRMED-RUN |
| 6 | `ARENA`/`BORROW_*`/`DROP` are stubs on AOT. `arena_lifecycle_golden` does not observe them. | **AGREE**, extended | x86 `14128-14132`, Wasm `14648-14652`. The golden (`8909`, `17643-17657`) runs via `execute_omni_surface_program`, which is AOT-first, i.e. **Wasm**. Extension: on Stage-1, DROP is not a no-op. It **misdecodes and crashes** (see §3). | CONFIRMED-RUN |
| 7 | "aot" means Wasm, not x86 ELF. | **AGREE** | `execute_omni_surface_aot_first` `5600-5691`: `compile_pure_u16_to_wasm` + `execute_wasm_module_i64_with_budgets` at `5672-5675`. The default CLI `run`/`eval` go through `execute_omni_surface_via_pure_omni_driver` (`9626-9720`), which does the same AOT-first dispatch. Run: `omnic run iget_buf_dyn.omni 7` prints **`Int(0)`**, while VM semantics give 7. So **the shipped `run` gives a wrong answer today** for buffer programs. | CONFIRMED-RUN |
| 8 | The `DB_QRY` miss fallback makes `route_db_query_golden` weak. | **AGREE** | `1500-1518`: on a miss, `unwrap_or(key.wrapping_mul(10))`. The golden at `8912` expects `200000420`, which is 200·10⁶ + 42·10. | CONFIRMED-READ |
| 9 | 51 opcodes are Void on the VM and unsupported on AOT. | **AGREE** | I scripted it: of the 128 table opcodes, 77 have a VM `match` arm in `1171-1666`, and exactly **51** have none and fall to `_ => OmniValue::Void` at `1666`. The list matches the matrix. Gates `302-317` and `319-335` are identical (52 opcodes; I recounted). | CONFIRMED-RUN (script) |
| 10 | `if_pos` 4-backend coverage is compile-only on x86 and Stage-1. | **AGREE** (the test is weak), but there is no hidden bug | Case loop `12051-12113`: Wasm/x86 compile only at `12083-12086`, Stage-1 only for `arith_else_temps` at `12088-12092`, results checked for VM+Wasm only at `12095-12105`. I ran **all 16 case/input pairs natively on x86 and Stage-1**. All match the expected values, and Stage-1 ELF == host ELF for every case. The suite is weak, but at `8160842` it is not hiding a join-slot emit bug. | CONFIRMED-RUN |

## 3. Uncertain-cell resolutions

The matrix has one `uncertain` cell, Stage-1 `DROP`. I also resolved the `W?` test cells the matrix flagged for x86/Stage-1 buffers.

### 3.1 Stage-1 `DROP` (0x7F05): **misdecode. RET is swallowed when a function has an odd number of DROPs before it, and the ELF crashes (SIGSEGV).** CONFIRMED-RUN + CONFIRMED-READ

- **Encoding:** DROP is a no-dest packet `[0x7F05, reg, INS_END]`, 3 words. From `omnic compile-surface` of `drop1.omni`: `… 7F05 7E00 7EE4 7F4A 7E01 7EE4 7EEF`.
- **Stage-1 no-dest dispatch** (`15644-15662`) recognises only `MOD_END`, `0`, `NOP`, `LABEL`, `JMP`, `BR_TRIT`, `BR_POS`, `BR_NEG`, and `RET`. Every other tok0 is assumed to be a dest-form packet `[dst, op, ty, tc, …]`:
  - `r_op = tok[pos+1]` (`15665`)
  - `r_disp_dst = (0x7DFF - tok0)*8` (`15666-15667`). For tok0 = 0x7F05 that is `-0x830`, outside the 192 SSA slots (`-8..-0x600`).
  - `r_cur = pos+4` (`15668`). That is already **inside the next packet**.
- **What happens in `drop1`** (`… DROP %0; RET %1`):
  1. `r_cur` lands on RET's operand `7E01`, so it emits `mov rax,[rbp-0x10]`.
  2. Then it sees `INS_END`, so it dispatches with `r_op = 0x7E00` (no arm), takes `store_dst`, and emits `mov [rbp-0x830],rax`.
  3. It sets `r_pos = r_cur+1`, which is `MOD_END`. **RET is never emitted.** Execution runs off the end of the code.
- **Run results** (`drop1.omni`, `drop3.omni`): VM / Wasm / x86 = **49**. Stage-1 = **SIGSEGV** (exit 139 via `omnic compile-stage1` + run, signal 11 in the harness).
- **With an even number of DROPs**, as in `arena_lifecycle_golden`, the second DROP gets eaten instead of the RET. The result is correct by luck. Byte diff vs host: exactly `48 8b 85 f8 ff ff ff` + `48 89 85 d0 f7 ff ff` inserted, and the tails are identical. **The golden passes only because it has two drops.**
- **Workspace mirror:** same decode at `omni-workspace/crates/omni-aot/src/lib.rs:1836` and copy list `:1860`. CONFIRMED-READ.

### 3.2 `W?` cells (x86/Stage-1 `ANEW`/`ISET`/`SLEN`)

Resolved by running.
- x86 and Stage-1 do store ISET bytes at `[rbp+r11-0x20000]`. SLEN returns the global `r11`, and x86 length counts match the VM in every single-buffer test.
- Readback through `IGET` does not work (gap #3).
- With two buffers, the length is wrong too: `two_bufs.omni` gives VM **31**, all AOT **11** (one global `r11`/`buf_len`).
- Wasm: lengths match, no bytes are stored.

Overlap note: all of this is in the **IGET/ISET byte-storage area that main work is fixing now**. I am only reporting it, not designing a fix.

## 4. Corrections to the matrix (cell / text level)

| ID | Where in matrix | Matrix says | Actually (@8160842) | Tag |
|---|---|---|---|---|
| C1 | Row `0x7F05` Stage-1 | uncertain | **Broken:** the misdecode swallows the next instruction, and with an odd DROP count before RET the ELF SIGSEGVs (§3.1). | RUN |
| C2 | Gap 4, row `0x7F01` Stage-1 | ARENA among the diverging opcodes | ARENA emission is **byte-identical** to host x86 (identity). 6 opcodes diverge, not 7. | RUN |
| C3 | Weak-goldens row `pack5_trit_vec_golden` | compares VM/Wasm/x86 | The selftest block `17505-17551` checks only `execute_omni_surface_program`, i.e. **Wasm**, at `17544-17545`. No VM, no x86. | READ |
| C4 | Same row, `pack5_direct_aot` | compares VM/Wasm/x86 | x86 ELF is **header-checked only, never executed** (`17770-17773`). VM (`_with_budgets`, `5547-5592`, a real VM) and Wasm are executed. I ran x86 natively: 2422, correct. | READ + RUN |
| C5 | Weak-goldens row `while_sum_even_golden` | "VM result, Wasm result, ELF bytes" | The "VM" value comes from `execute_omni_surface_program` (`17620`), which is AOT-first, i.e. **Wasm**. So the block runs Wasm twice and the VM never. | READ + RUN (AOT-first tag = `aot`) |
| C6 | Weak-goldens row `decision_hook_golden` | "Executes on the VM" | `17493-17501` uses `execute_omni_surface_program`, i.e. **Wasm** (scalar, single function). | RUN (`via aot`) |
| C7 | Rows `0x7F14` IMOD, x86 + Stage-1 | real / T | **partial:** `x % 0` and `INT_MIN % -1` return a **stale `rdx`** (N1). | RUN |
| C8 | Rows `0x7F28/29/2A/2B/2C` TNOT/TAND/TOR/TXOR/TMAJ, x86 + Wasm + Stage-1 | real / T | **partial:** no clamping of i64 operands. `spec.md:53` requires `v.clamp(-1,1)`, and the VM does it (`1115-1121`, `340-346`) (N3). | RUN |
| C9 | Rows `0x7F43/44` BR_POS/BR_NEG, x86 + Wasm + Stage-1 | real / T | **partial:** a non-trit i64 condition takes the wrong branch. AOT compares `== ±1`, while the VM clamps (N2). | RUN |
| C10 | Shared note "VM `-9` silently does nothing on OOB" | silent no-op | True for most OOB offsets. **Offsets −8..−1 make the Rust host panic** at `1326` (N6). | RUN |
| C11 | Minor line anchors | e.g. `5595-5680`, `14747-14750` | Function starts at `5600` (5595 is its doc comment). All other anchors I checked are within ±7 lines. No substantive misattribution. | READ |

Count impact: x86 loses about 8 `real` cells to `partial` (IMOD, 5 trit ops, BR_POS, BR_NEG). So does Stage-1. Wasm loses 7 (its IMOD is correct). Stage-1 `uncertain` becomes 0.

## 5. New findings (not in the matrix)

| ID | Finding | Evidence | Tag | Overlaps IGET/ISET fix? |
|---|---|---|---|---|
| **N1** | **x86 + Stage-1 `IMOD` returns stale `rdx`** for divisor 0 and for `INT_MIN % -1`. The guard jumps over `cqo; idiv`, then `mov rax, rdx` copies whatever `rdx` holds. After a `<=>` (TCMP leaves `rdx` = trit), `x % 0` gives **1** (VM/Wasm **0**). `MIN % -1` after TCMP also gives **1**. The existing native tests `16528-16556` (IMOD0) and `16452-16480` (MIN/−1) pass only because `rdx` happens to be 0 in one-instruction programs. Wasm is correct (`14800-14826` selects divisor 1). | x86 `14274-14281`. Stage-1 `15764-15766`. Workspace `omni-aot/src/lib.rs:445-447, 1929-1933`. `mod0_after_tcmp`, `minmod_tcmp`, `G3_imod_clobber` give x86 / S1 **11**, VM / Wasm **0**. | RUN | No |
| **N2** | **`BR_POS`/`BR_NEG` with a non-trit i64 condition diverge:** `if_pos x {100} else {200}` with x=7 gives VM **100**, all AOT **200**. Likewise `if_neg` with x=−7. The VM clamps (`1213-1230` via `eval_as_trit`). x86 `cmp rax,±1; je` (`14400-14406`), Wasm `14947-14960`, Stage-1 `15925-15941`. `SEL_TRIT` and `BR_TRIT` agree, because they are sign-based. | `brpos_int`, `brneg_int` | RUN | No |
| **N3** | **Trit ops do not clamp on AOT:** `tand(7,3)` gives VM `1`, AOT `3`. `tnot(7)` gives −1 vs −7. `txor(7,3)` gives −1 vs −21. `tor(7,−3)` gives 1 vs 7. `tmaj(7,−1,−1)` gives −1 vs +1. x86 TAND/TOR **share the IMIN/IMAX encoding** (`14321-14331`), TNOT shares INEG (`14283`). Wasm `14828`, `14848-14880`. Stage-1 `15714`, `15731`, `15781`, `15804`. `spec.md:53` says operands are clamped. | `tand_int`, `tnot_int`, `txor_int`, `tor_int`, `tmaj_int`, `G4_trit_clamp` (VM 13111, AOT 37053) | RUN | No |
| **N4** | **The const-fold "pack5 map" behind Wasm/x86 `IGET` is flow-insensitive and drops non-immediate bytes:** `13845-13870` pushes only `ImmInt` operands, ignores typed writes, and scans straight-line, ignoring branches. Results: `iset(buf, 0-1, 300)` gives VM 255044, Wasm/x86 **44000** (the indices shift). `if_pos c {iset(buf,9)} else {buf}` with the untaken branch gives VM 0, Wasm/x86 **9**. `G5_pack5_flow` gives VM 2255044, Wasm/x86 **2009000**, Stage-1 2000000. | `iset_neg_byte`, `branch_iset`, `G5_pack5_flow` | RUN | **Yes**: same IGET path. Flag to main work so the fix does not keep this fold as a "correct" fast path. |
| **N5** | **x86/Stage-1 `ISET` is unbounded and overwrites SSA register slots.** The heap starts at `rbp-0x20000` and grows up toward the SSA slots at `rbp-0x608..-8`. 140,000 appends make `return keep` (1234) return **−1** (0xFF bytes). VM/Wasm return 1234. | `14181-14196`, `15856-15880`. `iset_overrun.omni 140000`. | RUN | **Yes**: bounds belong in the byte-storage fix. Flagging only. |
| **N6** | **VM host panic:** `iset_patch_i64(buf, -3, v)` gives `off as usize` + 8, which wraps to 5 ≤ len, then `buf[huge..5]` panics: *"slice index starts at 18446744073709551613 but ends at 5"* at `1326`. That is a guest-reachable Rust panic instead of an `error[…]`. Workspace `omni-vm/src/lib.rs:310` has the same pattern. | `patch_neg.omni` | RUN (monolith), READ (workspace) | Adjacent (ISET -9). VM-only. |
| **N7** | **The verifier admits `san_*` on `i64`.** VM gives `Str("")` and AOT returns the integer unchanged. | `san_int.omni`. VM `1477`. | RUN | No |
| **N8** | **`iset_i32` readback:** VM 4255, AOT 4000 (no IGET). | `i32_trunc.omni` | RUN | Yes |
| **N9** | **Edges that agree on all 4 backends:** `x/0`=0, `MIN/−1`=MIN, `abs(MIN)`=MIN, wrapping `IMUL`, `SHL` masked to 63 (`shl_65`), `SHR` arithmetic and masked (`(−7)>>65 = −4`), `TCMP`, `SEL_TRIT`/`BR_TRIT` with non-trit cond, `slen` after `iset_patch_i64` OOB (8). | `mindiv`, `iabs_min`, `mul_ovf`, `shr_neg`, `patch_oob`, `sel_int`, `brtrit_int` | RUN | — |
| **N10** | **The "frozen bootstrap hashes" are not pinned in code.** The literals appear only in docs (`spec.md:94,191`, `omni_progress_checklist.md:5-7`, `spec/PURE_SELF_HOST_ROADMAP.md:53`, a doc comment at `10383`). The code asserts **self-consistency** (gen1 == gen2 at `10228-10245`, Stage-1 == Stage-2 at `16185`, `18021`), not specific values. **A change that moves a frozen hash would not fail any test.** | grep for 16-hex literals across `*.rs` | READ | — |
| **N11** | **Doc hash not reproduced:** `omni_progress_checklist.md:131` claims the driver is `1,412` words with `0x187D5EDEDC2F4E37`. `build_phase_b_pure_omni_driver_stream()` (`9618`) currently gives **120 words, `0xA90BB2AD592811A8`**. The doc may mean a different artifact. I also did not reproduce `0x65005EAAEA44E1CE` (the old 9-module set; the current set has 18 modules). | `hist.rs` | RUN / UNCERTAIN (which artifact the doc means) | — |
| N12 | x86 TY_BYTES-return prologue sets `r12 = 0` (`14013`), so `p0` is not `arg0` for bytes-returning functions. Possibly intended (p0 is the input array), but it differs from the VM for `fn f(x: i64) -> bytes`. | `14012-14019` | READ (not run) | Maybe |
| N13 | Side note for Scarlett: the on-disk `pure/*.omni` and `std/*.omni` are **not** what the suite hash compiles. `lookup_omni_stdlib_module` (`4425+`) serves embedded string literals, and `"pure/compiler.omni" \| "spec/pure_self_host/omni_selfhost_compiler.omni"` share one literal (`4469`). | READ | — | — |

## 6. Where the frozen hashes come from (needed for the hash-safety column)

I reproduced each one at `8160842` with `omnic fixed-point`, `omnic pure-bootstrap`, and `hist.rs` (CONFIRMED-RUN):

| Hash | Artifact | Inputs that feed it |
|---|---|---|
| `0x677F57AE71D28FED` | `omni_self_compiler.omni_bc`, 19,731 words | `build_step7_stage1_compiler_stream` `15422-16002` and helpers `15357-15420`, `emit_cli_arg_parser`, `pack_fn_module` `13017` |
| `0xEAFC81F46B482FF8` | Stage-1 ELF, 106,242 B; Stage-2 must be identical | The stream above **plus the x86 emitter arms for the opcodes that stream uses**. Opcode histogram (RUN): `CINT CTRIT ANEW IGET×68 ISET×163 IADD ISUB IMUL BOR SHL SHR CEQ×653 CGT TAND×3 TCMP LABEL JMP BR_POS×86 SEL_TRIT×642 RET SLEN`. **No IDIV/IMOD/DROP/ARENA/pack5/BR_NEG.** |
| `0x23B8D17257CCA9C4` / `0xE221085777B8AC47` | Phase D gen-1 u16 (159 words) / 824-B ELF | `PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE` (`10179`), both frontends, x86 emitter and Stage-1 for `CINT TCMP LABEL JMP BR_TRIT BR_POS RET` |
| `0x78462AE4C81DF408` | 18-module suite (14,559 words) | The embedded module literals (`4425+`), `PURE_OMNI_CORE_SUITE_MODULES` (`10503`), and both frontends. **u16 only; no backend feeds it.** |

What follows from this:
- **Hash-safe:** adding tests, goldens, or manifest entries, and changing the Wasm backend. Wasm feeds no frozen hash.
- **Hash-moving:**
  - Any change to the Stage-1 builder, including fixing DROP decode (N1) and Stage-1 IMOD, moves `0x677F…` and `0xEAFC…`.
  - Changing the x86 encodings of `IGET`/`ISET`/`ANEW`/`SLEN`/`TAND`/`BR_POS`/`CEQ`/`SEL_TRIT` moves `0xEAFC…`. **Main work's x86 IGET/ISET fix will move it unless the u16 `IGET` of the param-0 input array and the current ISET byte stores stay byte-identical.**
  - x86 `BR_POS`/`TCMP`/`BR_TRIT` changes also move `0xE221…`.
  - Fixing **x86 IMOD only** does not move any frozen hash, because no hashed stream contains IMOD. But Stage-1 must emit the same bytes for its ELF-equality tests, which then forces a Stage-1 change, which moves `0x677F…`/`0xEAFC…`.

## 7. Stronger-goldens proposal (prioritised)

Design rules:
- Every expected value is **derived by hand or in Python from the spec/byte layout**, not taken from a backend. I then checked it against the VM, which agrees for all of them.
- Arguments come from **runtime inputs**, so the compile-time pack5 fold cannot fake the result.
- The observable is the **native stdout i64 / exit status** on x86 and Stage-1, and the return value on VM and Wasm.
- All four backends must give identical results.

**Where they would live:**
- (a) A new `spec/acceptance/goldens/<id>.omni` plus a manifest entry. Since "aot" = Wasm, this only covers one backend.
- (b) **Primarily**, a new block `"1z. Cross-backend effect goldens"` in `run_unit_and_differential_selftests` (`16245+`). It would use the existing pattern at `16272-16300`: VM via `OmniBinaryVm`, `execute_wasm_module_i64`, the host ELF run by `Command::new`, and the Stage-1 ELF **also run**, not just byte-compared.

**Status today** is my harness result at `8160842`. "FAILS-TODAY" goldens should land as known-divergence (xfail) checks, or together with their fix.

| P | Golden id | Opcode(s) | Input sketch | Expected (independently derived) | Backends | Today | Location | Hash-safe to ADD? |
|---|---|---|---|---|---|---|---|---|
| **P0** | `frozen_hash_pin` | — (bootstrap) | Compute FNV-1a64 of the Stage-1 bc, Stage-1 ELF, Phase-D bc/ELF, and suite stream, and compare them to the four doc constants. | `0x677F57AE71D28FED`, `0xEAFC81F46B482FF8`, `0x23B8D17257CCA9C4`/`0xE221085777B8AC47`, `0x78462AE4C81DF408` | host | **PASSES** (reproduced) | new selftest next to `run_pure_core_phase4_selfhost_fixed_point_selftests` (`10932`) | **Yes.** It reads the hashes, it does not feed them. It makes the "unchanged" invariant enforceable (N10). |
| **P1** | `byte_readback_golden` | ANEW, ISET (bytes, -4), IGET, SLEN | `iset(buf,10,x,240); iset_i32(buf,-2)`, then read b0,b1,b2,b3,b6 and pack them as `((((n·256+b6)·256+b3)·256+b2)·256+b1)·256+b0` | x=7: bytes `[10,7,240,FE,FF,FF,FF]`, n=7, giving **8796075198218** | VM, Wasm, x86, Stage-1 | VM ✔. Wasm/x86 7696581455882 ✘. S1 7696581394432 ✘ | `G1_byte_readback.omni`, block 1z + acceptance | **Yes.** This is the **acceptance test for main work's IGET/ISET fix**. |
| **P2** | `stage1_drop_parity_golden` | ARENA, BORROW_R, DROP, RET | `drop1` (1 drop) and `drop3` (3 drops), returning x+42 | x=7: **49**, exit 0 | all 4, Stage-1 **executed** | S1 SIGSEGV ✘ | block 1z + extend `17643-17657` to run the Stage-1 ELF | **Yes to add.** The fix moves `0x677F…`/`0xEAFC…` (Stage-1 builder). |
| **P3** | `imod_rdx_clobber_golden` | TCMP, IMOD, SHL, ISUB | `c=x<=>5; r0=x%0; c2=x<=>5; r1=MIN%-1; r0*10+r1` | **0** (`spec`: mod by 0 = 0; MIN%−1 = 0, matching VM `1344` and Wasm) | all 4 | x86 / S1 **11** ✘ | block 1z; also strengthen `16528-16556` and `16452-16480` by putting a TCMP first | **Yes to add.** An x86-only fix moves no hash; the matching Stage-1 fix moves `0x677F…`/`0xEAFC…`. |
| **P4** | `trit_clamp_golden` | TAND TOR TXOR TNOT TMAJ | `tand(x,3), tor(x,-3), txor(x,3), tnot(x), tmaj(x,-1,-1)` packed in decimal | x=7 by `spec.md:53` clamp: 1,1,−1,−1,−1, giving **13111** | all 4 | VM ✔, AOT 37053 ✘ | block 1z + acceptance | **Yes to add.** A TAND fix in x86 moves `0xEAFC…` (TAND×3 in the stream). Alternative: the verifier/frontend rejects non-trit operands, which leaves hashed streams untouched (all their trit-op operands are already trits). **Needs a decision** (Q2). |
| **P5** | `br_nontrit_cond_golden` | BR_POS, BR_NEG | `if_pos x {100} else {200}` at x=7; `if_neg x …` at x=−7 | **100**, **100** (clamp) | all 4 | AOT 200 ✘ | block 1z | **Yes to add.** x86/Stage-1 encoding fixes move `0xEAFC…` and `0xE221…`. A frontend/verifier route is hash-safe (Q2). |
| **P6** | `pack5_flow_golden` | ANEW, ISET (non-imm byte), BR_POS, IGET | Branch-dependent `iset`; `iset(buf, 0-1, 44)` | x=7: n=2, b0=255, b1=44, giving **2255044** | all 4 | Wasm/x86 2009000 ✘, S1 2000000 ✘ | block 1z | **Yes** (test only; overlaps the IGET fix) |
| **P7** | `two_buffer_golden` | ANEW×2, ISET, SLEN | `a=[1,2,3]; b=[4]; slen(a)*10+slen(b)` | **31** | all 4 | AOT 11 ✘ | block 1z | **Yes** (overlaps the fix) |
| **P8** | `iset_bounds_golden` | ISET (loop), RET | Append n=140000 bytes, return the untouched local 1234 | **1234**, or a defined `error[BUDGET_EXCEEDED]`/non-zero exit. **Never a corrupted value.** | all 4 | x86 / S1 −1 ✘ | block 1z | **Yes** (overlaps the fix; the semantics need deciding) |
| **P9** | `vm_patch_negative_offset` | ISET -9 | `iset_patch_i64(buf, 0-3, 77)` | A structured `Err("error[…]")`, **not a panic** | VM (Wasm/x86 for parity) | Rust panic ✘ | block 1z | **Yes.** A VM fix is hash-safe too: the Stage-1 stream only patches offsets 96/104, which are in range (`15981-15982`). |
| **P10** | `pack5_stage1_exec` | TPACK5/TUNPACK5/TDOT/EGATE | Run the existing `pack5_direct_aot` and `pack5_trit_vec` **natively on x86 and Stage-1** | **2422** / **2455** (comment at `17541`, re-derived) | all 4 | x86 ✔, S1 120 / 2004 ✘ | extend `17505-17551`, `17733-17784` | **Yes to add.** A Stage-1 fix, or rejecting at the Stage-1 gate, moves `0x677F…`/`0xEAFC…`. |
| **P11** | Harden passing suites | if_pos join-slot, while_sum_even, arena (2-drop), pack5_direct x86 | Execute the x86 and Stage-1 ELFs natively. Replace `execute_omni_surface_program` with a true `OmniBinaryVm` where the code says "vm" (C5, C6). | Existing expected values | all 4 | **All PASS today** (I ran all 16 if_pos IOs and wse) | `12051-12113`, `17606-17640`, `17493-17501`, `17770-17773` | **Yes.** It can land immediately as hard assertions. |
| **P12** | `route_db_query_hit_golden` | KV_PUT/DB_INS, DB_QRY | Seed a row with value 4242 for id 42, then `db_query` | `200004242` (cannot equal the miss fallback 420) | VM (AOT rejects host ops) | not written | acceptance + `8912` | **Yes** |

**Recommended order:** P0, then P11 (both green today, so they are pure strengthening). Then P1/P6/P7/P8 to gate the in-flight IGET/ISET fix. Then P2, P3, P4/P5 (each needs a hash or spec decision). Then P9, P10, P12.

**General hash-safety for test additions** (CONFIRMED-READ): none of `16245+` selftests, `8889-8913` positive_suite, `spec/acceptance/goldens/*`, or the manifest feed any hashed stream. The manifest check (`9016-9019`) only requires four substrings. **Do not** put new goldens in `PURE_OMNI_CORE_SUITE_MODULES`, `lookup_omni_stdlib_module`, `pure/programs`, or `PHASE_D_*`. That would move `0x78462AE4C81DF408` or `0x23B8…`. Adding positive_suite entries changes the acceptance count (39) and doc counts (416/378), but not any hash.

## 8. What was run vs read

- **RUN:**
  - Selftest (416 PASS) and acceptance-diff (39 PASS).
  - `fixed-point` and `pure-bootstrap` hashes.
  - The 4-backend harness on 6 repo goldens, 5 if_pos cases × 16 IOs, and 40+ targeted edge programs (all saved in `vera_review_repro/`).
  - The opcode-arm count script.
  - Opcode histograms of the hashed streams.
  - `omnic run` and `omnic compile-stage1` CLI repros.
- **READ:** gates, and the VM / x86 / Wasm / Stage-1 arms cited above; acceptance and selftest drivers; the pack5 map; the hash-feeding functions; workspace mirrors (`omni-aot/src/lib.rs:296,305,349,445,816,876,915,920,1836,1860,1929`; `omni-vm/src/lib.rs:310`).
- **Not run:** the workspace crates (no `cargo build`), the DB/route goldens, and N12.

## 9. Open questions

1. **Frozen hashes vs correctness:** the Stage-1 DROP crash (P2), Stage-1 IMOD (P3), and Stage-1 pack5 divergence (P10) cannot be fixed in the Stage-1 builder without moving `0x677F57AE71D28FED`/`0xEAFC81F46B482FF8`. **Caleb/Boss Lady:** may those be re-frozen once, or should Stage-1 instead *reject* these opcodes at its gate (`5722-5726`)? Note that rejecting is a Rust host change, so it is hash-safe.
2. **Non-trit operands into trit ops and branches (N2/N3):** the spec (`spec.md:53`) says clamp. Should the AOT backends clamp (moves `0xEAFC…`, and `0xE221…` for BR_POS), or should the verifier/frontend reject i64 into trit positions (hash-safe)?
3. **Main work's x86 IGET/ISET fix:** the Stage-1 compiler stream depends on x86 `IGET` doing a **u16 load from the argv[1] buffer** when the array is param 0 (68 uses). Any encoding change there moves `0xEAFC…`. Please make sure the fix keeps that path byte-identical (N4/N5 relate to this).
4. `omni_progress_checklist.md:131` driver hash `0x187D5EDEDC2F4E37` (1,412 words): which artifact does it mean? `build_phase_b_pure_omni_driver_stream` is 120 words, `0xA90BB2AD592811A8` (N11).
5. Should `san_*` on non-string operands be rejected by the verifier (N7)?
