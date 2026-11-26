#!/usr/bin/env bash
# ==============================================================================
# OMNI STAGE-0 HOST BOOTSTRAP (`scripts/bootstrap_stage0.sh`)
# ==============================================================================
# Two-phase zero-Rust rebuild proof:
#   Phase 1 (this script): build the 3 Stage-0 host binaries (`./omni`,
#     `./omni-seed0`, `./omni-dev-host`) once using `rustc` (matching CI).
#   Phase 2 (`scripts/zero_rust_rebuild.sh`): strip `rustc` and `cargo` from
#     PATH and run the full seed/gate/literal-zero-Rust native ELF rebuild proof.
# ==============================================================================
set -euo pipefail

if ! command -v rustc >/dev/null 2>&1; then
  echo "[BOOTSTRAP FAIL] rustc is required to build Stage-0 host binaries" >&2
  exit 1
fi

echo "[BOOTSTRAP] Building Stage-0 host binaries with $(rustc --version)..."
rustc -O --edition 2021 omni_rust_bootstrap.rs -o ./omni
rustc -O --edition 2021 seed0/omni_seed0.rs -o ./omni-seed0
rustc -O --edition 2021 omni_dev_host.rs -o ./omni-dev-host
echo "[BOOTSTRAP PASS] Built ./omni, ./omni-seed0, and ./omni-dev-host. Next: PATH=/usr/bin:/bin bash scripts/zero_rust_rebuild.sh"
