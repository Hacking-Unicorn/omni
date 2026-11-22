# Omni Stage-0 Seed vs. Archived Rust Oracle Classification (Seed-Shrink C3)

Measured against the pre-Seed-Shrink Rust baseline (**25 files, 48,590 lines** on `main@bc2711e6`):

| Tier | Files | Lines | Role | Required for Default Pure Path? |
|---|---:|---:|---|---|
| **Mandatory Stage-0 Seed** | 2 (`seed0/omni_seed0.rs`, `p4_shim/mod.rs`) | **236** | Embeds & verifies FNV-1a64 of the 3 committed `.omni_bc` seeds (`M3`/`M2`/`M4`) + thin 3-op shim | **Yes** (`99.51%` reduction vs `48,590`) |
| **Optional Dev-Host** | 1 (`omni_dev_host.rs`) | **40** | Optional HTTP/KV `serve` tooling (`omni-dev-host`) | **No** |
| **Differential Oracle (Frozen Reference)** | 24 (`omni_rust_bootstrap.rs`, `omni-workspace/`, `m2_host/`) | **52,816** | Frozen Rust frontend/verifier/AOT oracle & differential CI harness (`--rust` / `--oracle` only; monolith 25,143 + workspace 20,326 + `m2_host` 7,347; total Rust 236 + 40 + 52,816 = 53,092) | **No** (not invoked on default `compile`/`run`/`self-host` or `zero_rust_rebuild.sh`) |

## Zero-Rust Rebuild Proof (Phase C1)
With `rustc` and `cargo` absent from `PATH`, run:
```bash
bash scripts/zero_rust_rebuild.sh
```
This verifies all 3 embedded seeds via `./omni-seed0`, runs the 181-byte zero-shim M5 launcher ELF64 (`0x87E30A9BB58BB3C4`), executes the native zero-Rust-binary `m2.elf -> m4.elf` chain (`76,782 B`, `0x82F217F319C2BDF5`), and checks `m2-fixed-point`, `m3-fixed-point`, `p5-sys`, `p7-measure`, and `p7-canary`.

## Independent Verifier Checker (Phase C2)
Run the zero-dependency Python 3 checker over the published verifier traces:
```bash
python3 scripts/external_verifier_check.py
```
