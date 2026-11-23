#!/usr/bin/env bash
# ==============================================================================
# OMNI SEED-SHRINK (PHASE C1) â€” ZERO-RUST FIXED-POINT RELEASE GATE
# `scripts/zero_rust_rebuild.sh`
# ==============================================================================
# Proves that with `rustc` and `cargo` completely absent from PATH, the
# committed Pure-Omni seeds (`pure/m3/m3.omni_bc`, `pure/m2/m2.omni_bc`,
# `pure/m4/m4.omni_bc`) verify bit-exact, pass the 237-B zero-shim M5 launcher
# ELF64 and native zero-Rust-binary `m2.elf -> m4.elf` chain, and match every
# published FNV-1a64 hash.
# ==============================================================================
set -euo pipefail

export PATH="/usr/bin:/bin"

if command -v rustc >/dev/null 2>&1 || command -v cargo >/dev/null 2>&1; then
  echo "[C1 FAIL] rustc or cargo is still reachable on PATH" >&2
  exit 1
fi

echo "[C1] Verified rustc/cargo absent from PATH ($PATH)"

./omni-seed0 verify-seeds
./omni-seed0 line-audit
./omni-dev-host --version

./omni a1-pure-default
./omni a2-pure-include
./omni a3-macro-quarantine
./omni b2-pure-verifier
./omni b4-pure-launcher
./omni self-host
./omni p5-sys
./omni m2-fixed-point
./omni m3-fixed-point

./omni compile spec/acceptance/goldens/weighted_diff_pure_golden.omni /tmp/c1_wd.omni_bc
OUT=$(./omni run /tmp/c1_wd.omni_bc 45 10 40)
if [ "$OUT" != "Int(100)" ]; then
  echo "[C1 FAIL] pure-default compile+run expected Int(100), got $OUT" >&2
  exit 1
fi
rm -f /tmp/c1_wd.omni_bc

./omni p7-measure
./omni p7-canary

echo "[C1 ZERO-RUST RELEASE GATE PASS] All seeds, M5 237-B launcher, native M2->M4 chain, M2/M3 fixed points, and P7 gates passed with rustc/cargo off PATH."
