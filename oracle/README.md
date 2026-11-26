# Omni Stage-0 Seed vs. Runtime Host & Rust Oracle Classification (Seed-Shrink C3 / Audit §4)

Measured against the pre-Seed-Shrink Rust baseline (**25 files, 48,590 lines** on `main@bc2711e6`) and live `wc -l` counts on the current tree:

| Tier | Files | Lines | Role | Runtime Requirement Today |
|---|---:|---:|---|---|
| **Classified Stage-0 Seed** | 2 (`seed0/omni_seed0.rs` 134, `p4_shim/mod.rs` 102) | **236** | Embeds & verifies FNV-1a64 pins of the 3 committed `.omni_bc` seeds (`M3`/`M2`/`M4`), extracts seeds, and provides the 3-op thin shim (`read_bytes`/`write_bytes`/`spawn`) | **Classified seed** (`99.51%` classified seed reduction vs `48,590`; `p4_shim/mod.rs` is `include!`d into `omni_rust_bootstrap.rs`) |
| **Optional Dev-Host** | 1 (`omni_dev_host.rs`) | **40** | Entry wrapper for optional HTTP/KV `serve` tooling (`omni-dev-host`, pulls in `omni_rust_bootstrap.rs` via `#[path]`) | **No** (dev/serve only) |
| **Production Default Host & Single Rust Oracle** | 6 active files (`omni_rust_bootstrap.rs` 26898, `m2_host/` 12201, `host_runtime/pure_default.rs` 1085; `omni-workspace/` archived in `oracle/archived-workspace/`) | **40,184** (40184; active Rust total 236 + 40 + 40184 = **40,460** / 40460) | Production default host runner (`host_runtime/pure_default.rs` 1085 + `omni_rust_bootstrap.rs` 26898 + `m2_host/` 12201 + `p4_shim/mod.rs` 102 = **40,286** / 40286 runtime host Rust lines), plus the single `--rust`/`--oracle` reference compiler (duplicate 8-crate `omni-workspace/` archived in `oracle/archived-workspace/` in §6.1) | **Yes for host CLI orchestration (`40,286` / 40286 lines)**; on Linux x86-64, default `compile`/`compile-elf`/`run`/verify execute hash-pinned native `m3.elf`/`m2.elf`/`m4.elf`/`verifier.elf` binaries directly (`--interpreted` / `OMNI_FORCE_INTERPRETED=1` selects the VM) |

## Zero-Rust-Toolchain Rebuild Proof (Phase C1 + Phase 2a)
`scripts/zero_rust_rebuild.sh` strips `rustc` and `cargo` from `PATH` and proves both C1 ("no Rust toolchain on `PATH`") AND Phase 2a (**literal zero Rust processes** in the native ELF toolchain chain): after verifying the embedded seeds and gates via the pre-built `./omni` binary (`40,286`-line runtime host), the Phase 2a section executes ONLY native Pure-Omni ELF64 binaries (`m2.elf` rebuilding `m2_gen2.elf` bit-exact, `m4_gen2.elf`, `verifier_gen2.elf`, `elf_link_gen2.elf`, and modular M3 objects, then `m3.elf` + `verifier_gen2.elf` + `m4_gen2.elf` + `m2_gen2.elf` compiling/verifying/running `weighted_diff_pure_golden.omni` and `pure/m5/zero_rust_driver.omni` directly on the Linux x86-64 kernel via M2 raw `0F 05` syscall intrinsics). The genuinely Rust-free native execution binaries are standalone `m2.elf` (which includes `pure/m2/verify/mod_check.omni` for structural/CFG/call-graph verification), `verifier.elf` (full capability/taint/linearity/arity verification via `pure/verifier_engine.omni`), `m3.elf`, `m4.elf`, `zero_rust_driver.elf`, and the 237-byte M5 `.omni_bc` header validator/entry stub (`pure/m5/launcher.omni`, which validates the `0x7EE0` magic header and exits 0/1 while `b4-pure-launcher` orchestrates the chain):
```bash
bash scripts/zero_rust_rebuild.sh
```
This verifies all 3 embedded seeds via `./omni-seed0`, runs the 237-byte zero-shim M5 `.omni_bc` validator/entry stub ELF64 (`0x45EE82AAE5008841`), executes the native zero-Rust-binary `m2.elf -> m4.elf` compilation (`90,402 B`, `0xB43AFBB0DED59488`), and checks `m2-fixed-point`, `m3-fixed-point`, `p5-sys`, `p7-measure`, and `p7-canary`.

## Independent Verifier Checker (Phase C2)
Run the zero-dependency Python 3 checker over the published verifier traces:
```bash
python3 scripts/external_verifier_check.py
```
