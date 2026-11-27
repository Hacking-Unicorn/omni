# Omni Binary-Seeded Bootstrap Manifest (`bootstrap/MANIFEST.md`)

As of Step 9 (Complete Rust Removal), the Omni repository contains **zero Rust source files (`.rs`) and zero Cargo manifests (`Cargo.toml` / `Cargo.lock`)**. The toolchain bootstraps directly from two committed Pure-Omni-built Linux x86-64 ELF64 binaries (`bootstrap/m2.elf` and `bootstrap/m3.elf`) plus the committed `.omni_bc` bytecode seeds.

> **Judge / Pin Discipline:** After Step 9, the frozen regression hashes and this bootstrap manifest are the sole cryptographic anchors. Every hash/pin change from Step 9 onward is a judge change: its own PR, with from-scratch derivation and fixed-point proof, never auto-merged and never proposer-initiated.

## 1. Committed Bootstrap Binaries (`bootstrap/`)

| Artifact | Size (B) | FNV-1a64 | Role |
|---|---:|---|---|
| `bootstrap/m2.elf` | `126,032` | `0x1F040D79AE834D14` | Pure-Omni M2 x86-64 ELF64 compiler (built from `pure/m2/m2.omni_bc`, includes Step 11b `sys_clock_ns` and Step 16 Part B7 `0x7F51`/`0x7F55` reservation, self-hosts `S1 == S2 == S3`) |
| `bootstrap/m3.elf` | `207,015` | `0x7397A5020895AEBF` | Pure-Omni M3 surface `.omni` → `.omni_bc` frontend (linked from `m3_standalone.omni_bc` + `mod_parse.omni_bc` via `elf_link.omni_bc`, includes Step 11b `sys_clock_ns`) |

> **Step 11b Judge Change:** `bootstrap/m2.elf`, `bootstrap/m3.elf`, `pure/m2/m2.omni_bc`, and `pure/m3/m3_standalone.omni_bc` were intentionally re-pinned in Step 11b to add the Linux x86-64 `sys_clock_ns()` intrinsic (`clock_gettime(CLOCK_MONOTONIC)`, syscall 228, `#[cap(SYS)]`). All 5 frozen hashes, `m4.elf` (`91,576 B`, `0xFBA8272337B1BAC4`), `mod_parse.omni_bc`, and `elf_link.omni_bc` are unchanged.
>
> **Step 15 Judge Change (Fuzz Find #1):** `pure/verifier_standalone.omni_bc` (`30,804 B` / `0xFF327F8976741D20` → `31,048 B` / `0x430CC3C933918F64`) and `verifier.elf` (`37,907 B` / `0x2ECD2CE3DCB69346` → `38,193 B` / `0x23E3D55383B3333B`) were intentionally re-pinned in Step 15 after tightening `pure/verifier_engine.omni::op_is_known` to reject all 18 unassigned opcode holes in `0x7F00..=0x7F7F` (including Find #1's `0x7F5F`) with existing error code `3001`. No other pin or frozen hash moved.
>
> **Step 16 Part B1 Judge Change (Family F4 — `FN_HDR fn_id`):** `pure/verifier_standalone.omni_bc` (`31,048 B` / `0x430CC3C933918F64` → `31,168 B` / `0x2AF95DB4A6628007`) and `verifier.elf` (`38,193 B` / `0x23E3D55383B3333B` → `38,365 B` / `0x035987D581B3F315`) were intentionally re-pinned in Step 16 Part B1 after adding `in_range(fn_idx, 0, 1023)` validation (matching `pure/m2/verify/mod_check.omni::m2v_record_fn`) at both `FN_HDR fn_id` read sites in `pure/verifier_engine.omni` (`prescan_module_fns` and `omni_verify_module_with_mode`), rejecting out-of-range / token-band `fn_id` words (such as F4's `0x7EFB`) with existing error code `3006`. No other pin or frozen hash moved.
>
> **Step 16 Part B2 Judge Change (Family F3 — Dest-Shape & `LABEL` Operand):** `pure/verifier_standalone.omni_bc` (`31,168 B` / `0x2AF95DB4A6628007` → `31,526 B` / `0x485D1C5C2D0A69C6`) and `verifier.elf` (`38,365 B` / `0x035987D581B3F315` → `38,895 B` / `0xB60283901F089BE9`) were intentionally re-pinned in Step 16 Part B2 after tightening `pure/verifier_engine.omni` to (1) reject dest-requiring opcodes emitted in destless slots (`op_has_dest(op) * (1 - is_dest)`) in `verify_ins_body` with existing error code `2012`, and (2) require `LABEL (0x7F40)` operand 0 to be a label/imm operand (`(fl & 2) + (fl & 8) > 0`) in `check_branch_labels` with existing error code `2009`. No other pin or frozen hash moved.
>
> **Step 16 Part B3 Judge Change (Family F1 — Unassigned Sanitizer Ops `0x7F5A`–`0x7F5C`):** `pure/verifier_standalone.omni_bc` (`31,526 B` / `0x485D1C5C2D0A69C6` → `31,564 B` / `0x061DACCE78F5ADDB`) and `verifier.elf` (`38,895 B` / `0xB60283901F089BE9` → `38,943 B` / `0x1D4E0C270335720E`) were intentionally re-pinned in Step 16 Part B3 after excluding unassigned opcodes `0x7F5A`–`0x7F5C` (`32602`–`32604`) from `pure/verifier_engine.omni::op_is_known` (rejected with existing error code `3001`), fully closing Find #2 (both F1 and F3 components). No other pin or frozen hash moved.
>
> **Step 16 Part B3b Judge Change (`LABEL` / Branch Label-Operand Corner):** `pure/verifier_standalone.omni_bc` (`31,564 B` / `0x061DACCE78F5ADDB` → `31,744 B` / `0x00B2D6710B6D00D0`) and `verifier.elf` (`38,943 B` / `0x1D4E0C270335720E` → `39,223 B` / `0x5CCDC9F0C0897E0B`) were intentionally re-pinned in Step 16 Part B3b after tightening `parse_operand_raw`, `record_labels_from_ops`, and `check_branch_labels` (both `LABEL` definition and branch-target validation) in `pure/verifier_engine.omni` to require either `LBL` tokens (`0x7EF0..=0x7EFF`, `fl & 2`) or in-bounds `IMM` label IDs (`0..=4079`, `fl & 16`), matching `pure/m2/labels.omni::m2lbl_slot_of` with existing error code `2009`. No other pin or frozen hash moved.
>
> **Step 16 Part B3c Judge Change (Label Identity Resolution — 4,096 Non-Aliasing Slots + Bit-63 Fix):** `pure/verifier_standalone.omni_bc` (`31,744 B` / `0x00B2D6710B6D00D0` → `31,958 B` / `0xE99BF4CBF1455275`) and `verifier.elf` (`39,223 B` / `0x5CCDC9F0C0897E0B` → `39,407 B` / `0x01F8692886A0F9E0`) were intentionally re-pinned in Step 16 Part B3c after replacing the 256-bit shared `ws[4..7]` label bitset with a 4,096-slot non-aliasing bitset at `ws[960..1023]` (`IMM` `0..=4079` → `0..4079`, `LBL` `0..=15` → `4080..4095` via `op_label_slot`, matching `pure/m2/labels.omni::m2lbl_slot_of`) and fixing the signed bit-63 mask bug in `has_label` (`1 - is_eq(bits & m, 0)`). No other pin or frozen hash moved.
>
> **Step 16 Part B4 Judge Change (Family F5 — Undefined Register in `RET` & `void` Dest Type):** `pure/verifier_standalone.omni_bc` (`31,958 B` / `0xE99BF4CBF1455275` → `32,286 B` / `0xEA324F363AC8D90E`) and `verifier.elf` (`39,407 B` / `0x01F8692886A0F9E0` → `39,891 B` / `0x48E5270D8B93F4F1`) were intentionally re-pinned in Step 16 Part B4 after tightening `pure/verifier_engine.omni` to (1) reject undefined/moved/borrowed/dropped register slots (`rst` in `{0, 3, 4, 5}`) in `handle_ret` with existing error code `1005`, and (2) reject `void` (`32640` / `0x7F80`) return types on value-producing destination instructions (`do_dest`) in `verify_ins_body` with existing error code `2012`. No other pin or frozen hash moved.
>
> **Step 16 Part B5 Judge Change (Family F2 — Align Verifier with M2 `mod_check.omni` Opcode Set & Final-`RET` Rule):** `pure/verifier_standalone.omni_bc` (`32,286 B` / `0xEA324F363AC8D90E` → `32,760 B` / `0x676886F908AD2887`) and `verifier.elf` (`39,891 B` / `0x48E5270D8B93F4F1` → `40,562 B` / `0xB3A9FFC5328CAFE4`) were intentionally re-pinned in Step 16 Part B5 after tightening `pure/verifier_engine.omni` to (1) align `op_is_known` to the exact 83-opcode set accepted by `pure/m2/verify/mod_check.omni::m2v_ins_min_ops` (rejecting all 24 unlowered opcodes including `0x7F19..=0x7F1D`, `0x7F32..=0x7F33`, `0x7F38`, `0x7F47`, `0x7F49`, `0x7F4F`, `0x7F60..=0x7F62`, `0x7F71..=0x7F72`, `0x7F74..=0x7F75`, `0x7F77..=0x7F7C` with existing error code `3001`), and (2) require every decoded function's final instruction to be `RET (0x7F4A = 32586)` in `verify_ins_body` (matching `pure/m2/verify/mod_check.omni:273` with existing error code `2012`). No other pin or frozen hash moved.
>
> **Step 16 Part B7 Judge Change (Option 2 — Reserve Evaporating Ops `0x7F51` `scat` & `0x7F55` `s2i`):** `pure/m2/m2.omni_bc` (`87,836 B` / `0x74FB1652CC0E974B` → `87,802 B` / `0xFD2F8B389C2AAEA1`), `bootstrap/m2.elf` (`126,152 B` / `0xAA5F84B6AA45BB51` → `126,032 B` / `0x1F040D79AE834D14`, self-hosting fixed point `S1 == S2 == S3 == bootstrap/m2.elf`), `pure/verifier_standalone.omni_bc` (`32,760 B` / `0x676886F908AD2887` → `32,760 B` / `0xD3482994AFC37D25`), and `verifier.elf` (`40,562 B` / `0xB3A9FFC5328CAFE4` → `40,562 B` / `0x965E0FA56E7B6046`) were intentionally re-pinned in Step 16 Part B7 after removing `0x7F51` (`scat`) and `0x7F55` (`s2i`) from `pure/m2/verify/mod_check.omni::m2v_ins_min_ops`, adding them to `pure/verifier_engine.omni::op_is_known` `hole` (`3001`), and marking their `pure/isa/table.omni` rows RESERVED (while routing `0x7F58` `san_dom` through `APPROXIMATE` in `pure/audit/decompile_rhs.omni`). `bootstrap/m3.elf`, `m4.elf`, all frozen-5 hashes, and `omnic_pure.elf` fixed point are unchanged.

## 2. Committed `.omni_bc` Bytecode Seeds (`pure/`)

| Seed Artifact | Size (B) | Words | FNV-1a64 | Built Artifact (Rebuilt in Clean Room) |
|---|---:|---:|---|---|
| `pure/m2/m2.omni_bc` | `87,802` | `43,901` | `0xFD2F8B389C2AAEA1` | `m2.elf` (`126,032` B, `0x1F040D79AE834D14`; `S1 == S2 == S3 == bootstrap/m2.elf`) |
| `pure/m3/m3.omni_bc` | `132,498` | `66,249` | `0x0E3052A4BE00D599` | Monolithic M3 frontend fixed-point reference |
| `pure/m3/m3_standalone.omni_bc` | `52,498` | `26,249` | `0xD83FBCA0D2E8FA78` | Modular M3 part 1 (linked into `m3.elf`) |
| `pure/m3/mod_parse.omni_bc` | `84,530` | `42,265` | `0x0F581C62270F2863` | Modular M3 part 2 (linked into `m3.elf`) |
| `pure/link/elf_link.omni_bc` | `7,164` | `3,582` | `0x5F261D791F65ED2A` | Pure-Omni multi-module ELF64 linker (`elf_link.elf`, `10,971` B; also rebuilt bit-exact from `pure/link/elf_link.omni`) |
| `pure/m4/m4.omni_bc` | `56,124` | `28,062` | `0x0561704B3E50EFAC` | Pure-Omni M4 VM (`90,402` B ELF64, `0xB43AFBB0DED59488`) |
| `pure/m4/m4_standalone.omni_bc` | `57,330` | `28,665` | `0x6F86B759214A2765` | Standalone native `m4.elf` (`91,576` B, `0xFBA8272337B1BAC4`) |
| `pure/verifier_standalone.omni_bc` | `32,760` | `16,380` | `0xD3482994AFC37D25` | Standalone native `verifier.elf` (`40,562` B, `0x965E0FA56E7B6046`) |

## 3. Rebuilt Toolchain & Harness ELFs (`build/pure-bin/`)

Produced from scratch by `bash scripts/build_pure_driver.sh build/pure-bin` (with `PATH=/usr/bin:/bin`, zero `rustc`, zero `cargo`, zero `python3`):

1. `pure_build_util.elf` — built from `pure/m5/pure_build_util.omni` via `bootstrap/m3.elf` + `bootstrap/m2.elf` (provides FNV-1a64 hashing and UTF-16LE widening without Python).
2. `m2.elf` — rebuilt from `pure/m2/m2.omni_bc` through 3 generations (`S1 == S2 == S3 == bootstrap/m2.elf`, `126,032` B, `0x1F040D79AE834D14`); `m2-fixed-point` additionally verifies `S1 == S2 == S3 == V` against the same pin (`126,032` B, `0x1F040D79AE834D14`).
3. `verifier.elf` (`40,562` B, `0x965E0FA56E7B6046`) and `m4.elf` (`91,576` B, `0xFBA8272337B1BAC4`) — rebuilt from their `.omni_bc` seeds using freshly built `m2.elf`.
4. `audit_diff.elf`, `audit_decompile.elf`, `omnic_pure.elf`, `m2_lint.elf`, `audit_view_gate.elf`, `proposer_loop.elf`, `core_gates.elf`, `extra_gates.elf` — compiled from `.omni` sources via `m3.elf` + `verifier.elf` + `m2.elf`.
5. `m3.elf` (`207,015` B, `0x7397A5020895AEBF`) — rebuilt via `omnic_pure.elf link-elf pure/m3/m3_standalone.omni_bc pure/m3/mod_parse.omni_bc` and verified bit-identical (`cmp`) to `bootstrap/m3.elf`.
6. `omnic_pure.elf` self-compile fixed point — `omnic_pure.elf` compiles `pure/driver/omnic_driver.omni` to `omnic_pure_gen2.elf` and verifies `0`-byte diff (`58,184` B).

## 4. Exact From-Scratch Clean-Room Procedure (Linux x86-64)

```bash
rm -rf build/pure-bin
PATH=/usr/bin:/bin bash scripts/zero_rust_rebuild.sh
```

## 5. Historical Rust Bootstrap Recovery Path

The deleted Rust bootstrap (`omni_rust_bootstrap.rs`, `seed0/omni_seed0.rs`, `m2_host/`, `host_runtime/`, `p4_shim/`, `omni_dev_host.rs`, and `oracle/archived-workspace/`) remains preserved in git history. The final commit containing the full Rust tree is `88ec669` (PR #48). Restoring it for archaeological or differential investigation is a git history operation, not a tree operation:

```bash
git checkout 88ec669 -- omni_rust_bootstrap.rs seed0 m2_host host_runtime p4_shim omni_dev_host.rs
```
