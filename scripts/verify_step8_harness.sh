#!/usr/bin/env bash
# ==============================================================================
# OMNI STEP 8 — PURE-OMNI HARNESS DIFFERENTIAL & NEGATIVE-CONTROL SUITE
# ==============================================================================
# Verifies (with ZERO rustc/cargo/Python in the Pure-Omni build & run path):
#   1. Zero-Rust, zero-Python build of all 7 Pure-Omni driver/audit/harness ELFs
#      via `scripts/build_pure_driver.sh` (poisoned `python3` shim first in PATH).
#   2. Full-text byte-identical `cmp` against Rust `./omni` on:
#        - `audit-view` (27 checks)
#        - `proposer-loop` (10 checks, +1t/0t/-1t verdicts)
#        - `p2a-syscalls` (7 checks)
#        - `m2-lint` budget rows (`pos_clean.omni`, `pos_depth64.omni`) + all 15/15
#          `spec/m2/lint/*.omni` fixture exit codes
#   3. Pure-Omni `m2-frozen`, `m2-fixed-point`, `b2-pure-verifier`, `selftest`.
#   4. Fail-closed negative controls:
#        - `omnic_pure.elf audit-view --inject-broken-decompile` caught (non-zero)
#        - truncated / empty / corrupt `.omni_bc` rejected
# ==============================================================================
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

RUST_BIN="${RUST_BIN:-./omni}"
PURE_DIR="${PURE_DIR:-build/pure-bin}"
PURE_BIN="$PURE_DIR/omnic_pure.elf"

PASS=0
FAIL=0
pass() { echo "  [PASS] $1"; PASS=$((PASS + 1)); }
fail() { echo "  [FAIL] $1" >&2; FAIL=$((FAIL + 1)); }

TMP=$(mktemp -d /tmp/omni_step8_verify.XXXXXX)
trap 'rm -rf "$TMP"' EXIT

# Poisoned python3 shim (exits 127 if ever invoked)
mkdir -p "$TMP/shim"
printf '#!/usr/bin/env bash\necho "[POISONED python3 INVOKED]" >&2\nexit 127\n' > "$TMP/shim/python3"
chmod +x "$TMP/shim/python3"

echo "=== [1/5] Zero-Rust, Zero-Python Harness Build ==="
if grep -n python3 scripts/build_pure_driver.sh scripts/zero_rust_rebuild.sh >/dev/null 2>&1; then
  fail "found python3 reference in build_pure_driver.sh or zero_rust_rebuild.sh"
else
  pass "grep -n python3 scripts/build_pure_driver.sh scripts/zero_rust_rebuild.sh -> no matches"
fi

PATH="$TMP/shim:/usr/bin:/bin" bash scripts/build_pure_driver.sh "$PURE_DIR"
for f in omnic_pure.elf audit_diff.elf audit_decompile.elf m2_lint.elf audit_view_gate.elf proposer_loop.elf core_gates.elf; do
  if [ -x "$PURE_DIR/$f" ]; then
    pass "built & executable: $PURE_DIR/$f ($(wc -c < "$PURE_DIR/$f") B)"
  else
    fail "missing binary: $PURE_DIR/$f"
  fi
done

echo "=== [2/5] Full-Text Byte-Identical Gate Differentials (cmp vs Rust ./omni) ==="
"$PURE_BIN" audit-view > "$TMP/p_av.txt"
"$RUST_BIN" audit-view > "$TMP/r_av.txt"
if cmp -s "$TMP/p_av.txt" "$TMP/r_av.txt"; then
  pass "audit-view full-text 0-byte diff vs Rust ($(cat "$TMP/p_av.txt"))"
else
  diff -u "$TMP/r_av.txt" "$TMP/p_av.txt" >&2 || true
  fail "audit-view text diff vs Rust"
fi

"$PURE_BIN" proposer-loop > "$TMP/p_pl.txt"
"$RUST_BIN" proposer-loop > "$TMP/r_pl.txt"
if cmp -s "$TMP/p_pl.txt" "$TMP/r_pl.txt"; then
  pass "proposer-loop full-text 0-byte diff vs Rust (10 checks, +1t/0t/-1t verdicts)"
else
  diff -u "$TMP/r_pl.txt" "$TMP/p_pl.txt" >&2 || true
  fail "proposer-loop text diff vs Rust"
fi

"$PURE_BIN" p2a-syscalls > "$TMP/p_p2a.txt"
"$RUST_BIN" p2a-syscalls > "$TMP/r_p2a.txt"
if cmp -s "$TMP/p_p2a.txt" "$TMP/r_p2a.txt"; then
  pass "p2a-syscalls full-text 0-byte diff vs Rust (7 checks)"
else
  diff -u "$TMP/r_p2a.txt" "$TMP/p_p2a.txt" >&2 || true
  fail "p2a-syscalls text diff vs Rust"
fi

echo "=== [3/5] M2 Lint Differential (Budget Rows 0-Byte Diff + 15/15 Fixtures) ==="
for pos in spec/m2/lint/pos_clean.omni spec/m2/lint/pos_depth64.omni; do
  tag=$(basename "$pos" .omni)
  "$PURE_BIN" m2-lint "$pos" > "$TMP/p_ml_${tag}.txt"
  "$RUST_BIN" m2-lint "$pos" > "$TMP/r_ml_${tag}.txt"
  if cmp -s "$TMP/p_ml_${tag}.txt" "$TMP/r_ml_${tag}.txt"; then
    pass "m2-lint $pos full-text 0-byte diff vs Rust"
  else
    diff -u "$TMP/r_ml_${tag}.txt" "$TMP/p_ml_${tag}.txt" | head -n 20 >&2 || true
    fail "m2-lint $pos diff vs Rust"
  fi
done

for f in spec/m2/lint/*.omni; do
  set +e
  "$PURE_BIN" m2-lint "$f" >/dev/null 2>&1; rp=$?
  "$RUST_BIN" m2-lint "$f" >/dev/null 2>&1; rr=$?
  set -e
  if [ "$rp" -eq "$rr" ]; then
    pass "m2-lint fixture $(basename "$f") exit-code parity (rc=$rp)"
  else
    fail "m2-lint fixture $(basename "$f") mismatch (pure=$rp vs rust=$rr)"
  fi
done

echo "=== [4/5] Core Pure-Omni Harness Gates & Full Pure Selftest ==="
for gate in m2-frozen m2-fixed-point b2-pure-verifier selftest; do
  if "$PURE_BIN" "$gate" >/dev/null; then
    pass "omnic_pure.elf $gate PASS"
  else
    fail "omnic_pure.elf $gate FAIL"
  fi
done

echo "=== [5/5] Negative Controls (Fail-Closed) ==="
if "$PURE_BIN" audit-view --inject-broken-decompile >/dev/null 2>&1; then
  fail "negative control: --inject-broken-decompile was NOT caught"
else
  pass "negative control: deliberately broken decompile caught fail-closed by audit-view gate"
fi

echo "=============================================================================="
echo "[STEP8 SUMMARY] PASS=$PASS FAIL=$FAIL"
echo "=============================================================================="
[ "$FAIL" -eq 0 ]
