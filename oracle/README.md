# Omni Stage-0 Seed vs. Runtime Host & Rust Oracle Classification (Seed-Shrink C3 / Audit §4)

Measured against the pre-Seed-Shrink Rust baseline (**25 files, 48,590 lines** on `main@bc2711e6`) and live `wc -l` counts on the current tree:

| Tier | Files | Lines | Role | Runtime Requirement Today |
|---|---:|---:|---|---|
| **Classified Stage-0 Seed** | 2 (`seed0/omni_seed0.rs` 134, `p4_shim/mod.rs` 102) | **236** | Embeds & verifies FNV-1a64 pins of the 3 committed `.omni_bc` seeds (`M3`/`M2`/`M4`), extracts seeds, and provides the 3-op thin shim (`read_bytes`/`write_bytes`/`spawn`) | **Classified seed** (`99.51%` classified seed reduction vs `48,590`; `p4_shim/mod.rs` is `include!`d into `omni_rust_bootstrap.rs`) |
| **Optional Dev-Host** | 1 (`omni_dev_host.rs`) | **40** | Entry wrapper for optional HTTP/KV `serve` tooling (`omni-dev-host`, pulls in `omni_rust_bootstrap.rs` via `#[path]`) | **No** (dev/serve only) |
| **Production Default Host & Single Rust Oracle** | 4 active files (`omni_rust_bootstrap.rs` 25789, `m2_host/` 7993, `host_runtime/pure_default.rs` 674; `omni-workspace/` archived in `oracle/archived-workspace/`) | **34,456** (34456; active Rust total 236 + 40 + 34456 = **34,732** / 34732) | Production default host runner (`host_runtime/pure_default.rs` 674 + `omni_rust_bootstrap.rs` 25789 + `m2_host/` 7993 + `p4_shim/mod.rs` 102 = **34,558** / 34558 runtime host Rust lines), plus the single `--rust`/`--oracle` reference compiler (duplicate 8-crate `omni-workspace/` archived in `oracle/archived-workspace/` in §6.1) | **Yes for host CLI orchestration & Pure-Omni verifier (`34,558` / 34558 lines)**; on Linux x86-64, default `compile`/`compile-elf`/`run` execute hash-pinned native `m3.elf`/`m2.elf`/`m4.elf` binaries directly (`--interpreted` / `OMNI_FORCE_INTERPRETED=1` selects the VM) |

## Zero-Rust-Toolchain Rebuild Proof (Phase C1)
`scripts/zero_rust_rebuild.sh` strips `rustc` and `cargo` from `PATH` and proves **"no Rust toolchain (`rustc`/`cargo`) on `PATH`"**, not "no Rust code executing": the script **does** invoke the pre-built `./omni` binary (`34,558`-line runtime host), which on Linux x86-64 executes the hash-pinned native `m3.elf`/`m2.elf`/`m4.elf` toolchain binaries directly on the default path and `OmniBinaryVm` for differential gates. The genuinely Rust-free native execution binaries are standalone `m2.elf` (which includes `pure/m2/verify/mod_check.omni` for structural/CFG/call-graph verification; full capability/taint/linearity verification via `pure/verifier_engine.omni` runs via `pure_default_verify_u16` on the host VM), `m3.elf`, `m4.elf`, and the 237-byte M5 `.omni_bc` header validator/entry stub (`pure/m5/launcher.omni`, which validates the `0x7EE0` magic header and exits 0/1 while `b4-pure-launcher` orchestrates the chain):
```bash
bash scripts/zero_rust_rebuild.sh
```
This verifies all 3 embedded seeds via `./omni-seed0`, runs the 181-byte zero-shim M5 `.omni_bc` validator/entry stub ELF64 (`0x87E30A9BB58BB3C4`), executes the native zero-Rust-binary `m2.elf -> m4.elf` compilation (`81,663 B`, `0x05304F996E83AD5B`), and checks `m2-fixed-point`, `m3-fixed-point`, `p5-sys`, `p7-measure`, and `p7-canary`.

## Independent Verifier Checker (Phase C2)
Run the zero-dependency Python 3 checker over the published verifier traces:
```bash
python3 scripts/external_verifier_check.py
```
