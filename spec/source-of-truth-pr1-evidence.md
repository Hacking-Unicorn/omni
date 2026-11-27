# Addons Item 1, Part B PR 1 Evidence — Derive `op_is_known` & Complete 47 `RESERVED` Slots

**Branch:** `feat/addons1-pr1-op-is-known`
**Baseline:** `origin/main @ 080f825521e8751948a93826637a17960a3e44e8` (`git ls-remote origin refs/heads/main`)
**Binding contract:** `spec/source-of-truth.md` (R1 + §4), with the 3 ratified PR 1 resolutions (`ACCEPTED` bit 5 = `32` in `flags` byte `+4`; both `verifier_engine.omni` and `mod_check.omni` derive in PR 1; D1b `spec/trusted_core.md:25` updated `25 -> 24`).

---

## 1. Derivation Mechanism & Named Compilation Unit

### 1.1 Static Verifier (`pure/verifier_engine.omni`)
- **Named compilation unit & include mechanism:** `pure/verifier_engine.omni` adds `include "isa/table.omni";` at the top of the file. When compiled via `omnic_pure.elf compile <src> <out.omni_bc> --include-base pure/`, `od_expand_includes_pass` resolves `"isa/table.omni"` relative to `pure/` (`pure/isa/table.omni`, `18,439 B` LF) directly into the single translation unit alongside `pure/verifier_engine.omni` (`46,739 B` LF). Total expanded standalone source is `65,148 B` (`< 65,536 B` native `m3.elf` read cap), using the exact same `include` mechanism already used by `pure/audit/decompile.omni`, `pure/driver/omnic_driver.omni`, and `pure/harness/*.omni`.
- **Standalone bytecode rebuild:** `pure/verifier_standalone.omni_bc` is compiled from `pure/verifier_engine.omni` with standalone CLI lowering (`stream: bytes` → `stream: array`, direct `load_u16` `return iget(stream, idx);`, `nwords = slen(stream)`, and appended `main(stream: array, a0: i64, a1: i64, a2: i64) -> bytes { return omni_verify_module_with_mode(stream, a0); }`) via `omnic_pure.elf compile --include-base pure/` (reproducing `080f825:pure/verifier_standalone.omni_bc` at `0`-byte diff before edits).
- **Once-per-module table caching:** In `omni_verify_module_with_mode`, the workspace buffer is initialized once per module invocation via `let ws: bytes = isa_tbl_append(init_zero_buf(8192));`, placing the `2,320 B` ISA table at byte offset `65536..67855` — strictly disjoint from internal state words `0..1023` (`0..8191`) and result output words `1024..8191` (`8192..65535`). Per-instruction checks never rebuild the table.
- **`op_is_known` derivation:** Deleted the entire hand-transcribed `hole` range expression. `op_is_known(ws: bytes, op: i64)` now calls `isa_op_index(op)` (`0..127`, or `-1` outside `0x7F00..=0x7F7F`) and `isa_op_flag(ws, 65536, safe_idx, 5)` (`(flags >> 5) & 1`). `op_has_dest(ws: bytes, op: i64)` and the three call sites in `verify_ins_body` pass `w`.

### 1.2 M2 Compiler Verifier (`pure/m2/m2_standalone.omni` & `pure/m2/verify/mod_check.omni`)
- **Existing linkage:** `pure/m2/m2.omni_bc` already links `pure/isa/table.omni` before `pure/m2/decode.omni`, `pure/m2/labels.omni`, and `pure/m2/verify/mod_check.omni`. Rebuilt from source via 3 chunked `bootstrap/m3.elf` passes (verified `0`-byte diff at `87,802 B` against `080f825:pure/m2/m2.omni_bc` prior to edits; zero Python in toolchain loop).
- **Once-per-invocation caching:** `pure/m2/m2_standalone.omni::m2_standalone_entry` pads the workspace arena `w` to `66048` `i64` words (`528,384 B = 0x81000 = m2l_isa_tbl()`) and executes `w = isa_tbl_append(w);` once before `m2_verify_module_into(w, arr)`.
- **`m2v_ins_min_ops` derivation:** Replaced `is_0` in `pure/m2/verify/mod_check.omni::m2v_ins_min_ops(w: bytes, op: i64)` with `known = in_blk * isa_op_flag(w, m2l_isa_tbl(), safe_idx, 5)` (`m2v_step_ins` passes `w`), while leaving `is_1..is_4` arities unchanged until R2 (PR 2). `m2_lint.elf` shows `fn#139` (`m2v_ins_min_ops`) improved from `86/120` SSA to `77/120` SSA (`0/12` labels), and `fn#0` (`m2_standalone_entry`) remains `14/120` SSA (`5/12` labels).

---

## 2. Layout, `RESERVED` Completion, D1a Table, and D1b Fix

- **Layout (`pure/isa/table.omni`):** Preserved the exact `2,320 B` table geometry (`isa_tbl_len = 2320`, `isa_op_off = 1296`, bytes `+5..+7` remain `0`). Set `flags` bit `5` (`32 = ACCEPTED`) on all `81` accepted opcodes (including `0x7F46 / 32582` at row `70`, changing `flags: 0 -> 32`), and normalized all `47` unassigned/reserved slots (`18` Step-15 + `3` B3-F1 + `24` B5-F2 + `2` B7-F5c) to `cap=0, flags=0 // RESERVED (0x7FXX)`.
- **D1a Masking Proof:** As proven in `spec/source-of-truth.md` §1 & §4 on `080f825`, all `47` unassigned slots already return `op_is_known == 0` in `pure/verifier_engine.omni` (rejected with error `3001` before capability/sink/sanitizer handling via `skip = (1 - known) + is_drop + is_ret`) and `m2v_ins_min_ops == -1` in `pure/m2/verify/mod_check.omni` (rejected with error `1`). Therefore clearing the stale `cap`/`flags` fields on the `42` non-zero hole rows cannot alter runtime acceptance or diagnostics (`0` diffs across the 128-op binary `cmp` sweep below).

### All 47 `RESERVED` Rows (42 D1a Normalized Rows + 5 Pre-Existing Zero Rows)

| idx | Hex | Origin | `table.omni` Before `(cap, flags)` | `table.omni` After `(cap, flags)` | Group |
|---:|---|---|---|---|---|
| 7 | `0x7F07` | Step-15 | `(0, 1)` | `(0, 0) // RESERVED (0x7F07)` | **D1a** |
| 10 | `0x7F0A` | Step-15 | `(0, 1)` | `(0, 0) // RESERVED (0x7F0A)` | **D1a** |
| 11 | `0x7F0B` | Step-15 | `(0, 1)` | `(0, 0) // RESERVED (0x7F0B)` | **D1a** |
| 12 | `0x7F0C` | Step-15 | `(0, 1)` | `(0, 0) // RESERVED (0x7F0C)` | **D1a** |
| 25 | `0x7F19` | B5-F2 | `(0, 1)` | `(0, 0) // RESERVED (0x7F19)` | **D1a** |
| 26 | `0x7F1A` | B5-F2 | `(0, 1)` | `(0, 0) // RESERVED (0x7F1A)` | **D1a** |
| 27 | `0x7F1B` | B5-F2 | `(0, 1)` | `(0, 0) // RESERVED (0x7F1B)` | **D1a** |
| 28 | `0x7F1C` | B5-F2 | `(0, 1)` | `(0, 0) // RESERVED (0x7F1C)` | **D1a** |
| 29 | `0x7F1D` | B5-F2 | `(0, 1)` | `(0, 0) // RESERVED (0x7F1D)` | **D1a** |
| 50 | `0x7F32` | B5-F2 | `(16, 1)` | `(0, 0) // RESERVED (0x7F32)` | **D1a** |
| 51 | `0x7F33` | B5-F2 | `(16, 3)` | `(0, 0) // RESERVED (0x7F33)` | **D1a** |
| 56 | `0x7F38` | B5-F2 | `(16, 3)` | `(0, 0) // RESERVED (0x7F38)` | **D1a** |
| 58 | `0x7F3A` | Step-15 | `(0, 1)` | `(0, 0) // RESERVED (0x7F3A)` | **D1a** |
| 59 | `0x7F3B` | Step-15 | `(0, 1)` | `(0, 0) // RESERVED (0x7F3B)` | **D1a** |
| 60 | `0x7F3C` | Step-15 | `(0, 1)` | `(0, 0) // RESERVED (0x7F3C)` | **D1a** |
| 61 | `0x7F3D` | Step-15 | `(0, 1)` | `(0, 0) // RESERVED (0x7F3D)` | **D1a** |
| 62 | `0x7F3E` | Step-15 | `(0, 1)` | `(0, 0) // RESERVED (0x7F3E)` | **D1a** |
| 71 | `0x7F47` | B5-F2 | `(0, 0)` | `(0, 0) // RESERVED (0x7F47)` | Pre-zero |
| 73 | `0x7F49` | B5-F2 | `(0, 0)` | `(0, 0) // RESERVED (0x7F49)` | Pre-zero |
| 75 | `0x7F4B` | Step-15 | `(0, 1)` | `(0, 0) // RESERVED (0x7F4B)` | **D1a** |
| 76 | `0x7F4C` | Step-15 | `(0, 1)` | `(0, 0) // RESERVED (0x7F4C)` | **D1a** |
| 77 | `0x7F4D` | Step-15 | `(0, 1)` | `(0, 0) // RESERVED (0x7F4D)` | **D1a** |
| 78 | `0x7F4E` | Step-15 | `(0, 1)` | `(0, 0) // RESERVED (0x7F4E)` | **D1a** |
| 79 | `0x7F4F` | B5-F2 | `(0, 0)` | `(0, 0) // RESERVED (0x7F4F)` | Pre-zero |
| 81 | `0x7F51` | B7-F5c | `(0, 0)` | `(0, 0) // RESERVED (0x7F51)` | Pre-zero (B7) |
| 82 | `0x7F52` | Step-15 | `(0, 1)` | `(0, 0) // RESERVED (0x7F52)` | **D1a** |
| 83 | `0x7F53` | Step-15 | `(0, 1)` | `(0, 0) // RESERVED (0x7F53)` | **D1a** |
| 84 | `0x7F54` | Step-15 | `(0, 1)` | `(0, 0) // RESERVED (0x7F54)` | **D1a** |
| 85 | `0x7F55` | B7-F5c | `(0, 0)` | `(0, 0) // RESERVED (0x7F55)` | Pre-zero (B7) |
| 90 | `0x7F5A` | B3-F1 | `(0, 5)` | `(0, 0) // RESERVED (0x7F5A)` | **D1a** |
| 91 | `0x7F5B` | B3-F1 | `(0, 5)` | `(0, 0) // RESERVED (0x7F5B)` | **D1a** |
| 92 | `0x7F5C` | B3-F1 | `(0, 5)` | `(0, 0) // RESERVED (0x7F5C)` | **D1a** |
| 94 | `0x7F5E` | Step-15 | `(0, 1)` | `(0, 0) // RESERVED (0x7F5E)` | **D1a** |
| 95 | `0x7F5F` | Step-15 | `(0, 1)` | `(0, 0) // RESERVED (0x7F5F)` | **D1a** |
| 96 | `0x7F60` | B5-F2 | `(2, 1)` | `(0, 0) // RESERVED (0x7F60)` | **D1a** |
| 97 | `0x7F61` | B5-F2 | `(2, 3)` | `(0, 0) // RESERVED (0x7F61)` | **D1a** |
| 98 | `0x7F62` | B5-F2 | `(2, 1)` | `(0, 0) // RESERVED (0x7F62)` | **D1a** |
| 113 | `0x7F71` | B5-F2 | `(4, 1)` | `(0, 0) // RESERVED (0x7F71)` | **D1a** |
| 114 | `0x7F72` | B5-F2 | `(512, 1)` | `(0, 0) // RESERVED (0x7F72)` | **D1a** |
| 116 | `0x7F74` | B5-F2 | `(512, 3)` | `(0, 0) // RESERVED (0x7F74)` | **D1a** |
| 117 | `0x7F75` | B5-F2 | `(256, 3)` | `(0, 0) // RESERVED (0x7F75)` | **D1a** |
| 119 | `0x7F77` | B5-F2 | `(8, 3)` | `(0, 0) // RESERVED (0x7F77)` | **D1a** |
| 120 | `0x7F78` | B5-F2 | `(8, 3)` | `(0, 0) // RESERVED (0x7F78)` | **D1a** |
| 121 | `0x7F79` | B5-F2 | `(8, 3)` | `(0, 0) // RESERVED (0x7F79)` | **D1a** |
| 122 | `0x7F7A` | B5-F2 | `(8, 3)` | `(0, 0) // RESERVED (0x7F7A)` | **D1a** |
| 123 | `0x7F7B` | B5-F2 | `(8, 3)` | `(0, 0) // RESERVED (0x7F7B)` | **D1a** |
| 124 | `0x7F7C` | B5-F2 | `(64, 3)` | `(0, 0) // RESERVED (0x7F7C)` | **D1a** |

- **D1b (`spec/trusted_core.md:25`):** Updated `(25 capability-gated opcodes)` → `(24 capability-gated opcodes)` to match the exact `24` active `cap != 0` opcodes in `pure/isa/table.omni`.

---

## 3. Zero-Behavior-Change Proofs

### 3.1 Full 128-Opcode Binary Sweep Diff (Empty)
Across all `128` opcode slots `0x7F00..=0x7F7F` on both `seed0_risk` (word `22`) and `seed4_consensus` (word `25`), comparing binary output of `verifier.elf` (`cmp -s`) and `m2.elf` exit codes (`512` comparisons) between `080f825` and PR 1:
```text
128-op binary cmp sweep diffs (seed0 + seed4 x verifier + m2 = 512 checks): 0
```

### 3.2 Corpus Verdicts & Decompiler Stamps Diff (Empty)
All 8 corpus items (`seed0_risk`, `seed4_consensus`, `seed6_clamp`, `kv_ledger_service`, `pack5_decision_controller`, `compiler_self_audit`, `zero_rust_driver`, `verifier_standalone.omni_bc`) had `0`-byte diff in verifier status (`0`), M2 exit (`0`), and decompiler status headers (`3 RECONSTRUCTED`, `5 APPROXIMATE`). Self-verification of `pure/verifier_standalone.omni_bc` passed `113/113` functions (`status=0, dec_off=0, nfns=113, bad=[]`).

### 3.3 Reduced Panel & Piped `--campaign-all` (With `item=7` Self-Mutation Note)

Running `fuzz_gates.elf` with the re-pinned `pure/verifier_standalone.omni_bc` (`item=7` in the corpus, which grew from `16,380` words to `23,634` words by linking `table.omni`):
- **Reduced panel (`fuzz_gates.elf`):**
  ```text
  [FUZZ FAIL] 300 mutants | 140 accepted | 160 rejected | soundness=0 roundtrip=0 judge=1
  op=1 gen=48 acc=19 rej=29
  op=2 gen=48 acc=14 rej=34
  op=3 gen=48 acc=12 rej=36
  op=4 gen=48 acc=32 rej=16
  op=5 gen=48 acc=3 rej=45
  op=6 gen=30 acc=30 rej=0
  op=7 gen=30 acc=30 rej=0
  [FUZZ FIND] kind=5 op=7 item=3 site=1091 seed=1910166380
  ```
  *(Baseline `080f825` had `142 accepted | 158 rejected` with `op=3 acc=13 rej=35`, `op=5 acc=4 rej=44`. Check 55's assertions — `soundness=0 roundtrip=0 judge=1` and `[FUZZ FIND] kind=5 op=7 item=3 site=1091 seed=1910166380` — are unchanged.)*
- **Piped `--campaign-all` (`fuzz_gates.elf --campaign-all`):**
  ```text
  [FUZZ FAIL] 2800 mutants | 1212 accepted | 1588 rejected | soundness=0 roundtrip=0 judge=12
  op=1 gen=500 acc=155 rej=345
  op=2 gen=500 acc=176 rej=324
  op=3 gen=500 acc=165 rej=335
  op=4 gen=500 acc=374 rej=126
  op=5 gen=500 acc=45 rej=455
  op=6 gen=150 acc=150 rej=0
  op=7 gen=150 acc=147 rej=3
  ```
  *(Unchanged find inventory at **12 finds (`soundness=0 roundtrip=0 judge=12`)**; baseline `080f825` had `1217 accepted | 1583 rejected`.)*
- **Fixed-`item=7` Zero-Diff Behavioral Control:** Because `item=7` in `fuzz_gates.omni` IS `pure/verifier_standalone.omni_bc` itself, changing its word count (`16,380 -> 23,634`) shifts `site_lim = nw - 17` and therefore mutates different offsets in `item=7`. Running PR 1's `verifier.elf` and `m2.elf` against the `080f825` `item=7` corpus file in an isolated temp directory produced **zero mutant-count diff** against `080f825` on both the reduced panel (`142 accepted | 158 rejected | op=1..7 exact match`) and `--campaign-all` (`1217 accepted | 1583 rejected | op=1..7 exact match`).

---

## 4. Scratch Source-Only Acid Test Transcript (Both `verifier.elf` and `m2.elf`)

Run inside isolated `/home/sniper/.pr1_work/acid` (subsequently deleted; mutated source never committed), modifying **only `pure/isa/table.omni`** with **zero edits** to `pure/verifier_engine.omni`, `pure/m2/verify/mod_check.omni`, or `pure/m2/m2_standalone.omni`:

```text
--- /home/sniper/omni/pure/isa/table.omni
+++ /home/sniper/.pr1_work/acid/probe1/pure/isa/table.omni
@@ -155,7 +155,7 @@
 #[strict]
 fn isa_op_rows_1(t: bytes) -> bytes {
     let o: bytes = t;
-    o = iset(o, 16, 127, 0, 0, 57, 0, 0, 0);
+    o = iset(o, 16, 127, 0, 0, 25, 0, 0, 0);
     o = iset(o, 17, 127, 0, 0, 57, 0, 0, 0);
     o = iset(o, 18, 127, 0, 0, 57, 0, 0, 0);
     o = iset(o, 19, 127, 0, 0, 57, 0, 0, 0);
PROBE1 (0x7F10 flags 57->25) seed0.bc:
  ver_acid words:  0 0 1 0 0 3 3001 21 3003 28 1005 28 
  m2_acid rc=1 (expected 1)

--- /home/sniper/omni/pure/isa/table.omni
+++ /home/sniper/.pr1_work/acid/probe2/pure/isa/table.omni
@@ -258,7 +258,7 @@
     o = iset(o, 92, 127, 0, 0, 0, 0, 0, 0); // RESERVED (0x7F5C)
     o = iset(o, 93, 127, 0, 0, 33, 0, 0, 0);
     o = iset(o, 94, 127, 0, 0, 0, 0, 0, 0); // RESERVED (0x7F5E)
-    o = iset(o, 95, 127, 0, 0, 0, 0, 0, 0); // RESERVED (0x7F5F)
+    o = iset(o, 95, 127, 0, 0, 32, 0, 0, 0); // ACID_TEST (0x7F5F)
     return o;
 }
PROBE2 (0x7F5F flags 0->32) seed0_7f5f.bc (orig verifier had [0 * 1 0 * 3 3001 21 3003 28 1005 28], orig m2 had rc=1):
  ver_acid words:  0 0 1 0 1 0 
  m2_acid rc=0 (expected 0)
```

---

## 5. M3 Byte-Neutrality Proof & `bootstrap/MANIFEST.md` Pin Movements

`pure/m3/` contains `0` references to `pure/isa/table.omni` / `isa_*`. Clean-room post-rebuild hashes match `080f825` byte-for-byte:

| Artifact | Before (`080f825`) | After (`feat/addons1-pr1-op-is-known`) | Delta / Status |
|---|---|---|---|
| `bootstrap/m3.elf` | `207,015 B` / `0x7397A5020895AEBF` | `207,015 B` / `0x7397A5020895AEBF` | **Unchanged** (0 B; bit-exact `link-elf`) |
| `pure/m3/m3.omni_bc` | `132,498 B` / `0x0E3052A4BE00D599` | `132,498 B` / `0x0E3052A4BE00D599` | **Unchanged** (0 B) |
| `pure/m3/m3_standalone.omni_bc` | `52,498 B` / `0xD83FBCA0D2E8FA78` | `52,498 B` / `0xD83FBCA0D2E8FA78` | **Unchanged** (0 B) |
| `pure/m3/mod_parse.omni_bc` | `84,530 B` / `0x0F581C62270F2863` | `84,530 B` / `0x0F581C62270F2863` | **Unchanged** (0 B) |
| `pure/m2/m2.omni_bc` | `87,802 B` (`43,901w`) / `0xFD2F8B389C2AAEA1` | **`87,686 B` (`43,843w`) / `0x3DCC8546CE5B0D57`** | **Moved** (`-116 B`; `table.omni` + `m2_standalone` + `mod_check`) |
| `bootstrap/m2.elf` | `126,032 B` / `0x1F040D79AE834D14` | **`125,660 B` / `0x3A4A285C58CFA93E`** | **Moved** (`-372 B`; `S1 == S2 == S3 == V`) |
| `pure/verifier_standalone.omni_bc` | `32,760 B` (`16,380w`) / `0xD3482994AFC37D25` | **`47,268 B` (`23,634w`) / `0x7E82A2FFC55AEE56`** | **Moved** (`+14,508 B`; links `table.omni`) |
| `verifier.elf` (rebuilt) | `40,562 B` / `0x965E0FA56E7B6046` | **`77,189 B` / `0xDB33F13E1082948A`** | **Moved** (`+36,627 B`; rebuilt via new `m2.elf`) |
| `pure/link/elf_link.omni_bc` | `7,164 B` / `0x5F261D791F65ED2A` | `7,164 B` / `0x5F261D791F65ED2A` | **Unchanged** |
| `pure/m4/m4.omni_bc` | `56,124 B` / `0x0561704B3E50EFAC` | `56,124 B` / `0x0561704B3E50EFAC` | **Unchanged** |
| `pure/m4/m4_standalone.omni_bc` | `57,330 B` / `0x6F86B759214A2765` | `57,330 B` / `0x6F86B759214A2765` | **Unchanged** |
| `m4.elf` (rebuilt) | `91,576 B` / `0xFBA8272337B1BAC4` | `91,576 B` / `0xFBA8272337B1BAC4` | **Unchanged** |
| `omnic_pure.elf` fixed point | `59,653 B` (0-B diff) | `59,653 B` (0-B diff) | **Unchanged** |
| All 5 frozen hashes (`core_gates`) | PASS | PASS | **Unchanged** |

---

## 6. Cost Note (Size & Runtime)

| Metric | Baseline (`080f825`) | PR 1 (`feat/addons1-pr1-op-is-known`) | Delta |
|---|---:|---:|---:|
| `pure/verifier_standalone.omni_bc` | `32,760 B` (`16,380w`) | `47,268 B` (`23,634w`) | `+14,508 B` (`+7,254w`) |
| `build/pure-bin/verifier.elf` | `40,562 B` | `77,189 B` | `+36,627 B` |
| `audit_decompile.elf` output (`verifier_standalone`) | `64,495 B` | `85,245 B` | `+20,750 B` (`< 120,000 B` Check 17 cap) |
| `pure/m2/m2.omni_bc` | `87,802 B` (`43,901w`) | `87,686 B` (`43,843w`) | `-116 B` (`-58w`) |
| `bootstrap/m2.elf` | `126,032 B` | `125,660 B` | `-372 B` |
| Reduced panel (`fuzz_gates.elf`) wall time | `2,393 ms` | `2,389 ms` | `-4 ms` (`-0.2%`) |
| `--campaign-all` wall time (2,800 mutants) | `20,916 ms` | `21,908 ms` | `+992 ms` (`+4.7%`) |

*(Note: Once-per-module table caching inside `omni_verify_module_with_mode` and `m2_standalone_entry` keeps per-instruction lookup cost constant; the modest `+4.7%` `--campaign-all` delta reflects spawning the slightly larger `77 KiB` `verifier.elf` binary and mutating the larger `47 KiB` `item=7`.)*

---

## 7. Clean-Room Gates Summary

- `rm -rf build/pure-bin && PATH=/usr/bin:/bin bash scripts/zero_rust_rebuild.sh` — **PASS** (`S1 == S2 == S3 == bootstrap/m2.elf`, `m3.elf == bootstrap/m3.elf`, `omnic_pure.elf` fixed point `59,653 B`, all `10` manifest pins, `11/11` gates + selftest + negative control)
- `PATH=/usr/bin:/bin bash scripts/verify_step7_driver.sh` — **PASS=38 FAIL=0**
- `PATH=/usr/bin:/bin bash scripts/verify_step8_harness.sh` — **PASS=76 FAIL=0** (Check 55 untouched)
- `./build/pure-bin/omnic_pure.elf selftest` — **PASS**
- `python3 scripts/external_verifier_check.py` — **PASS (8/8)**
