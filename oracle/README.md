# Omni Stage-0 Seed vs. Runtime Host & Rust Oracle Classification (Seed-Shrink C3 / Audit §4)

Measured against the pre-Seed-Shrink Rust baseline (**25 files, 48,590 lines** on `main@bc2711e6`) and live `wc -l` counts on the current tree:

| Tier | Files | Lines | Role | Runtime Requirement Today |
|---|---:|---:|---|---|
| **Classified Stage-0 Seed** | 2 (`seed0/omni_seed0.rs` 134, `p4_shim/mod.rs` 102) | **236** | Embeds & verifies FNV-1a64 pins of the 3 committed `.omni_bc` seeds (`M3`/`M2`/`M4`), extracts seeds, and provides the 3-op thin shim (`read_bytes`/`write_bytes`/`spawn`) | **Classified seed** (`99.51%` classified seed reduction vs `48,590`; `p4_shim/mod.rs` is `include!`d into `omni_rust_bootstrap.rs`) |
| **Optional Dev-Host** | 1 (`omni_dev_host.rs`) | **40** | Entry wrapper for optional HTTP/KV `serve` tooling (`omni-dev-host`, pulls in `omni_rust_bootstrap.rs` via `#[path]`) | **No** (dev/serve only) |
| **Host VM Runner & Differential Oracle** | 24 (`omni_rust_bootstrap.rs` 25579, `m2_host/` 7907, `omni-workspace/` 20435) | **53,921** (53921; total Rust 236 + 40 + 53921 = **54,197** / 54197) | Host `OmniBinaryVm` interpreter that executes the Pure-Omni `.omni_bc` seeds on the default CLI path (`monolith` 25579 + `m2_host` 7907 + `p4_shim` 102 = **33,588** / 33588 runtime-required Rust lines today), plus the `--rust`/`--oracle` reference compiler and 8-crate workspace (`20,435` / 20435 lines) | **Yes for interpreted `.omni_bc` execution today (`33,588` / 33588 lines)** until standalone native `m3.elf`/`m4.elf` execution (§3) lands; the genuinely Rust-free piece today is native `m2.elf` |

## Zero-Rust-Toolchain Rebuild Proof (Phase C1)
`scripts/zero_rust_rebuild.sh` strips `rustc` and `cargo` from `PATH` and proves **"no Rust toolchain (`rustc`/`cargo`) on `PATH`"**, not "no Rust code executing": the script **does** invoke the pre-built `./omni` binary (which interprets the committed Pure-Omni `m3.omni_bc` / `m2.omni_bc` / `m4.omni_bc` seeds on `OmniBinaryVm` across the `33,588`-line runtime host). The genuinely Rust-free native execution step today is the standalone `m2.elf` (which includes `pure/m2/verify/mod_check.omni` for structural/CFG/call-graph verification; full capability/taint/linearity verification via `pure/verifier_engine.omni` runs on the host until the standalone native chain lands) and the 181-byte M5 `.omni_bc` header validator/entry stub (`pure/m5/launcher.omni`, which validates the `0x7EE0` magic header and exits 0/1 while `b4-pure-launcher` orchestrates the chain):
```bash
bash scripts/zero_rust_rebuild.sh
```
This verifies all 3 embedded seeds via `./omni-seed0`, runs the 181-byte zero-shim M5 `.omni_bc` validator/entry stub ELF64 (`0x87E30A9BB58BB3C4`), executes the native zero-Rust-binary `m2.elf -> m4.elf` compilation (`81,663 B`, `0x05304F996E83AD5B`), and checks `m2-fixed-point`, `m3-fixed-point`, `p5-sys`, `p7-measure`, and `p7-canary`.

## Independent Verifier Checker (Phase C2)
Run the zero-dependency Python 3 checker over the published verifier traces:
```bash
python3 scripts/external_verifier_check.py
```
