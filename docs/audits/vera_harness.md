# Vera Parity: four-backend differential harness + hash-safe goldens (P0, P1, P2, P11 + divergence cases)

## 1. Base, branch, patches

| Item | Value |
|---|---|
| Review audited at | `8160842`. The branch was first cut from `74a215c`, where every finding reproduced. |
| **Base** | `origin/isa/multibyte-loads` = **`d2d6c13a99ed49a2c0f612562b36c0e0c06876f8`** (main's Wasm ISET/IGET fix). I re-checked at push time (~09:50 CST); the tip had not moved. Main's C1/C2/C3 had **not** landed yet. |
| **Pushed branch** | `origin/vera/parity-goldens` → **head `d257cb0ed8092e787c44648375791d4a5c348321`**. It was a normal (non-force) push of a new branch. I pushed nothing else and opened no PR. |
| Commits | `b032e23`: harness + cross-backend goldens (P1, P2, P3, P4, P5, P6, P7, P9, P11 cases). `78cd1f8`: P11 hardening of existing selftests. `d257cb0`: label-cap 15/16 cases. |
| P0 | **Dropped** per Boss Lady, because main has its own hash-pin commit (C1). The harness never depended on P0 code; I removed P0's constants, compute function, selftest, and `hash-pin` CLI cleanly. The old P0 commit survives only on the local branch `vera/parity-goldens-with-p0` (not pushed). |
| Patches | `/workspace/audits/vera_patches/0001…0003`, made with `git format-patch d2d6c13`. |
| Scope | All changes are in `omni_rust_bootstrap.rs`. Test/host code only. No backend, VM, IGET/ISET, Stage-1 builder, stdlib-literal, `pure/`, or `PHASE_D_*` changes. Zero crates. `/workspace/omni` was not touched. |

## 2. How to run

```sh
cd /workspace/audit-vera            # needs pure/ next to the .rs (include_str!)
rustc -O --edition 2021 omni_rust_bootstrap.rs -o /tmp/omnic

/tmp/omnic backend-diff --suite        # grade all goldens: PASS / XFAIL / XPASS / FAIL per backend (add -v for every run)
/tmp/omnic backend-diff prog.omni 7    # any program (file or inline source) + up to 3 i64 args
/tmp/omnic diff4 "fn f(x: i64) -> i64 { return x * 3; }" 5
/tmp/omnic selftest                    # includes 13AY (cross-backend goldens)
```

Exit codes:
- `--suite`: 0 when there is no FAIL and no XPASS, otherwise 1.
- Single program: 0 when every comparable backend is byte-identical, 1 on divergence, 2 on usage or frontend/verifier rejection.

**What each backend runs.** All four get the *same* verified u16 stream:

| Backend | What runs |
|---|---|
| `vm` | `OmniBinaryVm::execute_function` directly. Not AOT-first, which is Wasm (review C5/C6). |
| `wasm` | `compile_pure_u16_to_wasm` + `execute_wasm_module_i64` |
| `x86` | `compile_pure_u16_to_x86_64_elf_with_args`, **executed natively** |
| `stage1` | `compile_u16_via_stage1_to_elf_for_diff`: the Stage-1 self-compiler stream `build_step7_stage1_compiler_stream`, run on the VM with the same gates as `compile_omni_surface_via_stage1_to_elf`. Its output ELF is **executed natively** |

**Sources.** A source is either surface `.omni`, or Omni assembly when its first line is `#asm`. Assembly is needed for label id 0 and the 16-label case. Frontend/verifier rejections can be graded with `DiffExpect::Rejected(code)`.

**Observable.** Results are normalized to `(exit code, stdout bytes)` and compared byte-for-byte:
- `i64`/`trit` results become 8 bytes LE.
- `bytes`-returning functions become the raw buffer. This is the buffer/arena memory dump: the ELF writes the buffer to stdout, and the VM returns `Bytes`.
- Other outcomes are recorded as their own kinds: `SIGNAL n`, `TIMEOUT` (20 s), structured `ERR`, host `PANIC` (each backend runs under `catch_unwind` with the panic hook silenced), and `UNSUPPORTED` (compile or gate rejection, excluded from comparison).
- Stage-1 ELF == host ELF is reported as well.

**Known-failures mechanism.** The repo has no xfail mechanism, so I added a clearly named list: `OMNI_DIFF4_KNOWN_DIVERGENCES: &[(golden, backend, reason with file:line)]`. It is **strict**:
- A listed backend must still fail at least one run. If all its runs match, the selftest fails with `XPASS … remove it from OMNI_DIFF4_KNOWN_DIVERGENCES`. This forces the fixing commit to delete the entry, and it already fired once, on main's Wasm fix.
- An unlisted mismatch is a `FAIL`.
- Negative control (RUN): I added a bogus entry and changed one expected value in a scratch copy. The output was `1 xpass, 4 fail`, exit 1.

**Limits:**
- Up to 3 args, baked into the ELF (emitter ABI); argv is not passed.
- Wasm has no memory-dump observable, since the executor returns only an i64.
- "Stage-1" is the Stage-1 compiler stream run on the VM, not the native `omni_stage1_self_compiler.elf`. The two are byte-identical by the fixed point, but the harness does not separately run the native compiler.
- Hosts without x86-64 Linux report SKIP, which is not counted.

## 3. Tests and status (at head `d257cb0`, base `d2d6c13`)

Every expectation is derived by hand (shown in the source comments), never copied from a backend.

| Golden | Proposal | Runs (args → expected) | VM | Wasm | x86 | Stage-1 |
|---|---|---|---|---|---|---|
| `p1_byte_readback` | P1 | 7→**8796075198218**, 200→8796075247626, 300→8796075207690 (byte = v&0xFF), −1→8796075261706 | PASS | **PASS** (fixed by d2d6c13) | XFAIL: got 7696581394432; 2-op IGET reads the argv token region `[rbp+rax*2-0x2E000]` (14530-14573), not the ISET heap (14574+) | XFAIL: same (lbl_iget 16411) |
| `p1_iget_runtime_value` | P1 | 7→7, 200→200 | PASS | **PASS** | XFAIL: 0 | XFAIL: 0 |
| `p1_buffer_dump_const` | P1 (dump) | 7→bytes `0a 14 f0 fe ff ff ff` | PASS | XFAIL: returns i64 7. No bytes-return ABI (14923 ignores ret_ty; RET 15413/15491) | PASS | PASS |
| `p1_buffer_dump_param` | P1 (dump) | 7→bytes `0a 07 f0 fe ff ff ff` | PASS | XFAIL: same | XFAIL: `0a 00 f0…`. N12: TY_BYTES prologue sets r12=0 (14405-14406) | XFAIL: N12 (lbl_ty_bytes_pr 16089) |
| `p6_pack5_flow` | P6 (bonus) | 7→2255044 | PASS | XFAIL: 2009000. **Still broken after d2d6c13**: branch-dependent all-immediate buffer is still folded (collect_func_pack5_map 14217; mechanism inferred) | XFAIL: 2009000 | XFAIL: 2000000 |
| `p7_two_buffers` | P7 (bonus) | 7→31 | PASS | XFAIL: 11. One `buf_len_local` for all buffers (15096/15183) | XFAIL: 11 (global r11) | XFAIL: 11 |
| `p2_drop1` | P2 | 7→49, −42→0 | PASS | PASS | PASS | XFAIL: **SIGSEGV**. No-dest dispatch has no DROP arm (16222-16241); the fallthrough (16243-16247) swallows RET |
| `p2_drop3` | P2 | 7→49 | PASS | PASS | PASS | XFAIL: SIGSEGV |
| `p2_drop2_arena_lifecycle` | P2 | 8→50, 7→49 | PASS | PASS | PASS | PASS (by luck: the 2nd DROP is eaten, not RET) |
| `p11_while_sum_even` | P11 | 10→30, 11→30, 2→2, 1→0, 0→0, 100→2550 | PASS | PASS | PASS | PASS |
| `p11_if_pos_arith_else_temps` | P11 | 5→200, 0→2, −3→−4 | PASS | PASS | PASS | PASS |
| `p11_if_pos_nested_complex_else` | P11 | 5→10, 0→30, −5→20, −6→20 | PASS | PASS | PASS | PASS |
| `p11_match_trit_complex_neg` | P11 | 5→55, 0→100, −3→0 | PASS | PASS | PASS | PASS |
| `p11_pack5_direct_aot` | P11/P10 | ()→2422 | PASS | PASS | PASS | XFAIL: 120. No TPACK5/TUNPACK5/TDOT/EGATE arms |
| `p11_pack5_trit_vec` | P11/P10 | ()→2455 | PASS | PASS | PASS | XFAIL: 2004 |
| `p11_decision_hook` | P11/P10 | (30,10)→1, (5,20)→−1, (12,12)→0 | PASS | PASS | PASS | XFAIL: 20 / −15 / 0. No EGATE arm; delta is copied through |
| `p3_imod_rdx_clobber` | P3 | 7→0 | PASS | PASS | XFAIL: 11. Stale rdx (14667-14675) | XFAIL: 11 (lbl_imod 16343) |
| `p3_imod_zero_after_tcmp` | P3 (x%0) | 7→0 | PASS | PASS | XFAIL: 1 | XFAIL: 1 |
| `p3_imod_min_neg1_after_tcmp` | P3 (MIN%−1) | 7→0 | PASS | PASS | XFAIL: 1 | XFAIL: 1 |
| `p3_imod_control_no_tcmp` | P3 control | 7→0 | PASS | PASS | PASS (rdx happens to be 0) | PASS |
| `p4_trit_clamp` | P4 | 7→13111 | PASS | XFAIL: 37053 (15332, 15352-15357) | XFAIL: 37053. TAND/TOR=IMIN/IMAX (14714), TNOT=INEG (14676) | XFAIL: 37053 (16293, 16310) |
| `p5_if_pos_nontrit` (`if_pos 7`) | P5 | 7→100, 1→100, 0→200, −7→200 | PASS | XFAIL: 7→200 (15451-15460) | XFAIL: 7→200. `cmp rax,1` (14793-14800) | XFAIL: 7→200 (16504) |
| `p5_if_neg_nontrit` | P5 | −7→100, −1→100, 7→200 | PASS | XFAIL: −7→200 | XFAIL: −7→200 | XFAIL: −7→200 (16513) |
| `lbl15_surface` | label cap | 6 value if_pos + 1 while = exactly **15** surface labels (ids 1..15). 5→521, −5→210, 0→210 | PASS | PASS | PASS | PASS |
| `lbl16_surface` | label cap (**open decision**) | 7 if_pos + while = 16 labels. Expected **frontend reject** `error[BUDGET_EXCEEDED]` (`alloc_lbl` `next_lbl > 15`, `omni_rust_bootstrap.rs:3646-3656`; ids start at 1, `:3587`). If the cap becomes 16, the expectation becomes 528 / 280 | PASS (rejected) | PASS (rejected) | PASS (rejected) | PASS (rejected) |
| `lbl16_asm_ids_0_15` | label cap (**open decision**) | Exactly **16** labels, ids 0..15 (assembly; the 15-label program + `JMP 0; LABEL 0`). 5→521, −5→210 | PASS | PASS | PASS | PASS (Stage-1 ELF == host ELF) |
| `lbl15_asm_ids_0_14` | label cap | 15 labels using ids 0..14 (isolates label id 0). 5→521, −5→210 | PASS | PASS | PASS | PASS |
| `p9_vm_patch_negative_offset` | P9 (VM only) | 7→structured `Err` | XFAIL: **host PANIC** "slice index starts at 18446744073709551613 but ends at 5" (1324-1326) | n/a (unspecified; observed 8) | n/a (8) | n/a (8) |

**Totals (13AY):** 109 graded: **74 PASS, 35 XFAIL, 0 XPASS, 0 FAIL, 0 SKIP**.

**Label-cap result (RUN at d2d6c13):** 16 labels work at emit time on every backend.
- The 16-label assembly program (ids 0..15) is accepted by:
  - the Rust binary verifier (`verify_u16_stream`)
  - lockstep (`verify_u16_stream_lockstep`)
  - the Phase-7 pure-Omni kernel
  - the Omni verifier engine
- It also runs correctly on VM, Wasm, x86-64 and Stage-1, and Stage-1 ELF == host ELF.
- The assembler allows ids 0..15 (`:1893-1922`).
- The only 15-label limit is the **surface frontend** (`alloc_lbl`, `:3646-3656`), whose ids start at 1 (`:3587`). So Nyx's "15 usable" is right for surface `.omni`, but it is a frontend limit, not an emitter limit. Raising it to 16 means starting `next_lbl` at 0 or changing the check. That is a Rust frontend change and moves no backend encoding, but the `pure/` frontend twin would need the same change.
- No label known-divergence was needed.

**P11 hard assertions added to existing selftests** (all pass):
- if_pos join slot: Stage-1 ELF is **run natively** for all 16 case/input pairs, and Stage-1 ELF == host ELF is required for every pair. main's d2d6c13 already added the native x86 leg; I kept it.
- `while_sum_even`: the "VM" leg is now the real VM (it was AOT-first = Wasm). Both ELFs run natively for limits 10/11/2/1/100.
- `pack5_direct_aot`: the x86 ELF is executed, giving 2422. Before, only its header was checked.
- `pack5_trit_vec`: added real-VM and native x86 legs (2455).
- `decision_hook`: added real-VM and native x86 legs.
- `arena_lifecycle`: added real VM, native x86, and native Stage-1 (50).
- The Stage-1 pack5/EGATE gaps stay in the xfail list, not as hard asserts.

## 4. Full-suite results (RUN, each commit built and run separately)

| Tree | `omnic selftest` | `omnic acceptance-diff` | 13AY suite |
|---|---|---|---|
| base `d2d6c13` | PASS **444** | PASS 39 | — |
| `b032e23` harness | PASS 537 (+93) | PASS 39 | 93: 58 pass / 35 xfail |
| `78cd1f8` P11 | PASS 556 (+19) | PASS 39 | same |
| `d257cb0` labels | PASS **572** (+16) | PASS 39 | 109: **74 pass / 35 xfail** / 0 xpass / 0 fail |

## 5. Frozen-hash recomputation proof (RUN at every commit: `b032e23`, `78cd1f8`, `d257cb0`)

P0 was dropped, so the proof uses a standalone recompute program and the shipping CLIs. The recompute program (`/workspace/vera-build/hash/hashes.rs`, compiled against each commit's source) builds every artifact exactly as `fixed-point` / `pure-bootstrap` do. Identical output at all three commits and at base `d2d6c13`:

```
stage1_bc   words=19731 bytes=39462 fnv=0x677F57AE71D28FED
stage1_elf  bytes=106242 fnv=0xEAFC81F46B482FF8
phaseD_bc   words=159 fnv=0x23B8D17257CCA9C4 g1==g2:true
phaseD_elf  bytes=824 fnv=0xE221085777B8AC47 g1==g2:true
suite       words=14559 fnv=0x78462AE4C81DF408
[FIXED-POINT PASS] omni_self_compiler.omni_bc=39462 B (0x677F57AE71D28FED) | stage1.elf=106242 B (0xEAFC81F46B482FF8) == stage2.elf (0xEAFC81F46B482FF8) | diff=0 bytes
pure-bootstrap: "bc_fnv1a64":"0x23B8D17257CCA9C4" "elf_fnv1a64":"0xE221085777B8AC47" "suite_fnv1a64":"0x78462AE4C81DF408"
```

- The regenerated `omni_self_compiler.omni_bc` and `omni_stage1_self_compiler.elf` at `d257cb0` are `cmp`-identical to the committed repo artifacts.
- **No hash moved.**
- In-code enforcement is now left to main's C1.

## 6. Notes for main / Boss Lady

- **IGET/ISET acceptance status after d2d6c13:**
  - Wasm readback is fixed (P1 passes).
  - **Still failing on Wasm:** `p6_pack5_flow` (2009000; the branch-dependent constant buffer is still folded), `p7_two_buffers` (11; one shared length), and the bytes-return ABI (dumps).
  - **x86/Stage-1:** every P1 run fails, as main expected.
  - When main fixes any of these, the matching xfail entries will XPASS and must be deleted in the same commit.
- **Suggestion for main: drop 13AW's record-only P1.** `13AW` (`run_byte_readback_parity_selftests`) asserts VM and Wasm and only *records* x86/Stage-1. `13AY`'s `p1_byte_readback` asserts the same value, 8796075198218, on all four backends with strict xfail, so a fix on x86/Stage-1 is forced to show up as XPASS. I did not edit 13AW.
- **If main's C1 hash pin lands:** this branch has no hash-pin code, so nothing duplicates it. Expect only a trivial chain-comment conflict near `13AW`/`13AY`.
- Line anchors in the xfail list are for `d2d6c13` before this patch. In the patched file, lines up to the new section (~12480) are unchanged; the emitter and Stage-1 regions shift by the section size, about +846 lines (e.g. x86 IMOD 14667 → 15513). Function and label names are the stable anchors.
- Not done:
  - P8 (140k-append overrun; slow, and its semantics are undecided).
  - P10 beyond running pack5/EGATE goldens on Stage-1.
  - P12 (DB hit).
  - No `cargo` run of the workspace crates (untouched).
- Decisions still open (from the review): re-freeze vs reject at the Stage-1 gate for DROP/IMOD/pack5, and clamp vs verifier-reject for trit operands. If the verifier-reject route is chosen, the P4/P5 expectations change from values to `UNSUPPORTED`/reject.
