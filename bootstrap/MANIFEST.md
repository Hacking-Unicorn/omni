# Omni Binary-Seeded Bootstrap Manifest (`bootstrap/MANIFEST.md`)

As of Step 9 (Complete Rust Removal), the Omni repository contains **zero Rust source files (`.rs`) and zero Cargo manifests (`Cargo.toml` / `Cargo.lock`)**. The toolchain bootstraps directly from two committed Pure-Omni-built Linux x86-64 ELF64 binaries (`bootstrap/m2.elf` and `bootstrap/m3.elf`) plus the committed `.omni_bc` bytecode seeds.

> **Judge / Pin Discipline:** After Step 9, the frozen regression hashes and this bootstrap manifest are the sole cryptographic anchors. Every hash/pin change from Step 9 onward is a judge change: its own PR, with from-scratch derivation and fixed-point proof, never auto-merged and never proposer-initiated.

## 1. Committed Bootstrap Binaries (`bootstrap/`)

| Artifact | Size (B) | FNV-1a64 | Role |
|---|---:|---|---|
| `bootstrap/m2.elf` | `126,152` | `0xAA5F84B6AA45BB51` | Pure-Omni M2 x86-64 ELF64 compiler (built from `pure/m2/m2.omni_bc`, includes Step 11b `sys_clock_ns`, self-hosts `S1 == S2 == S3`) |
| `bootstrap/m3.elf` | `207,015` | `0x7397A5020895AEBF` | Pure-Omni M3 surface `.omni` → `.omni_bc` frontend (linked from `m3_standalone.omni_bc` + `mod_parse.omni_bc` via `elf_link.omni_bc`, includes Step 11b `sys_clock_ns`) |

> **Step 11b Judge Change:** `bootstrap/m2.elf`, `bootstrap/m3.elf`, `pure/m2/m2.omni_bc`, and `pure/m3/m3_standalone.omni_bc` were intentionally re-pinned in Step 11b to add the Linux x86-64 `sys_clock_ns()` intrinsic (`clock_gettime(CLOCK_MONOTONIC)`, syscall 228, `#[cap(SYS)]`). All 5 frozen hashes, `m4.elf` (`91,576 B`, `0xFBA8272337B1BAC4`), `verifier.elf` (`37,907 B`, `0x2ECD2CE3DCB69346`), `mod_parse.omni_bc`, and `elf_link.omni_bc` are unchanged.

## 2. Committed `.omni_bc` Bytecode Seeds (`pure/`)

| Seed Artifact | Size (B) | Words | FNV-1a64 | Built Artifact (Rebuilt in Clean Room) |
|---|---:|---:|---|---|
| `pure/m2/m2.omni_bc` | `87,836` | `43,918` | `0x74FB1652CC0E974B` | `m2.elf` (`126,152` B, `0xAA5F84B6AA45BB51`; `S1 == S2 == S3 == bootstrap/m2.elf`) |
| `pure/m3/m3.omni_bc` | `132,498` | `66,249` | `0x0E3052A4BE00D599` | Monolithic M3 frontend fixed-point reference |
| `pure/m3/m3_standalone.omni_bc` | `52,498` | `26,249` | `0xD83FBCA0D2E8FA78` | Modular M3 part 1 (linked into `m3.elf`) |
| `pure/m3/mod_parse.omni_bc` | `84,530` | `42,265` | `0x0F581C62270F2863` | Modular M3 part 2 (linked into `m3.elf`) |
| `pure/link/elf_link.omni_bc` | `7,164` | `3,582` | `0x5F261D791F65ED2A` | Pure-Omni multi-module ELF64 linker (`elf_link.elf`, `10,971` B; also rebuilt bit-exact from `pure/link/elf_link.omni`) |
| `pure/m4/m4.omni_bc` | `56,124` | `28,062` | `0x0561704B3E50EFAC` | Pure-Omni M4 VM (`90,402` B ELF64, `0xB43AFBB0DED59488`) |
| `pure/m4/m4_standalone.omni_bc` | `57,330` | `28,665` | `0x6F86B759214A2765` | Standalone native `m4.elf` (`91,576` B, `0xFBA8272337B1BAC4`) |
| `pure/verifier_standalone.omni_bc` | `30,804` | `15,402` | `0xFF327F8976741D20` | Standalone native `verifier.elf` (`37,907` B, `0x2ECD2CE3DCB69346`) |

## 3. Rebuilt Toolchain & Harness ELFs (`build/pure-bin/`)

Produced from scratch by `bash scripts/build_pure_driver.sh build/pure-bin` (with `PATH=/usr/bin:/bin`, zero `rustc`, zero `cargo`, zero `python3`):

1. `pure_build_util.elf` — built from `pure/m5/pure_build_util.omni` via `bootstrap/m3.elf` + `bootstrap/m2.elf` (provides FNV-1a64 hashing and UTF-16LE widening without Python).
2. `m2.elf` — rebuilt from `pure/m2/m2.omni_bc` through 3 generations (`S1 == S2 == S3 == bootstrap/m2.elf`, `125,539` B, `0x0A5CD5B55707AA0D`); `m2-fixed-point` additionally verifies hosted `S1 == S2 == S3 == V` (`125,539` B, `0xBDA0A8A86960CB26`).
3. `verifier.elf` (`37,907` B, `0x2ECD2CE3DCB69346`) and `m4.elf` (`91,576` B, `0xFBA8272337B1BAC4`) — rebuilt from their `.omni_bc` seeds using freshly built `m2.elf`.
4. `audit_diff.elf`, `audit_decompile.elf`, `omnic_pure.elf`, `m2_lint.elf`, `audit_view_gate.elf`, `proposer_loop.elf`, `core_gates.elf`, `extra_gates.elf` — compiled from `.omni` sources via `m3.elf` + `verifier.elf` + `m2.elf`.
5. `m3.elf` (`206,946` B, `0x8CFD973873CAFF46`) — rebuilt via `omnic_pure.elf link-elf pure/m3/m3_standalone.omni_bc pure/m3/mod_parse.omni_bc` and verified bit-identical (`cmp`) to `bootstrap/m3.elf`.
6. `omnic_pure.elf` self-compile fixed point — `omnic_pure.elf` compiles `pure/driver/omnic_driver.omni` to `omnic_pure_gen2.elf` and verifies `0`-byte diff (`54,923` B).

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
