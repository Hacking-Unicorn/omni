# Omni Rust Removal Record (`oracle/README.md`)

As of **Step 9 (Complete Rust Removal)**, all Rust source files and Cargo manifests have been deleted from the repository tree:
- Deleted: `omni_rust_bootstrap.rs`, `seed0/omni_seed0.rs`, `m2_host/`, `host_runtime/`, `p4_shim/`, `omni_dev_host.rs`, `oracle/archived-workspace/`, and `scripts/bootstrap_stage0.sh`.
- Active Rust lines in tree: **0**.

## Binary-Seeded Pure-Omni Bootstrap
The toolchain now bootstraps directly from the two committed Pure-Omni-built Linux x86-64 ELF64 binaries (`bootstrap/m2.elf` and `bootstrap/m3.elf`) and the committed `.omni_bc` seeds listed in [`bootstrap/MANIFEST.md`](../bootstrap/MANIFEST.md).

```bash
rm -rf build/pure-bin
PATH=/usr/bin:/bin bash scripts/zero_rust_rebuild.sh
```

## Independent Verifier Checker (Phase C2)
Run the zero-dependency Python 3 checker over the published verifier traces:
```bash
python3 scripts/external_verifier_check.py
```

## Historical Rust Recovery Path
The pre-deletion Rust bootstrap and archived workspace remain in git history at commit `88ec669` (PR #48). Restoring them is a git history operation, not a tree operation:
```bash
git checkout 88ec669 -- omni_rust_bootstrap.rs seed0 m2_host host_runtime p4_shim omni_dev_host.rs
```
