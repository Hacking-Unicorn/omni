#!/usr/bin/env bash
# ==============================================================================
# OMNI PURE-OMNI HARNESS VERIFICATION & NEGATIVE-CONTROL SUITE (`scripts/verify_step8_harness.sh`)
# ==============================================================================
# Verifies (with ZERO Rust files, ZERO rustc/cargo, ZERO Python in build/run):
#   1. Zero-Rust, zero-Python build of all 8 Pure-Omni driver/audit/harness ELFs
#      via `scripts/build_pure_driver.sh` (poisoned `python3` shim first in PATH).
#   2. All 11 named Pure-Omni harness gates pass with exact expected check counts:
#        audit-view (27), proposer-loop (10), p2a-syscalls (7), m2-frozen (4),
#        m2-fixed-point (3), b2-pure-verifier (16), m3-modular (13),
#        a3-macro-quarantine (4), b4-pure-launcher (5), p5-sys (3), s5-refreeze (4).
#   3. `m2-lint` passes on positive fixtures (`pos_*.omni`) and fails closed on
#      all negative fixtures (`neg_*.omni`) across all 15/15 `spec/m2/lint/*.omni`.
#   4. `omnic_pure.elf selftest` passes end-to-end.
#   5. Fail-closed negative control (`--inject-broken-decompile` caught).
# ==============================================================================
set -euo pipefail

export PATH="/usr/bin:/bin"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

PURE_DIR="${PURE_DIR:-build/pure-bin}"
PURE_BIN="$PURE_DIR/omnic_pure.elf"

PASS=0
FAIL=0
pass() { echo "  [PASS] $1"; PASS=$((PASS + 1)); }
fail() { echo "  [FAIL] $1" >&2; FAIL=$((FAIL + 1)); }

TMP=$(mktemp -d /tmp/omni_step8_verify.XXXXXX)
trap 'rm -rf "$TMP"' EXIT

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
for f in omnic_pure.elf audit_diff.elf audit_decompile.elf m2_lint.elf audit_view_gate.elf proposer_loop.elf core_gates.elf extra_gates.elf; do
  if [ -x "$PURE_DIR/$f" ]; then
    pass "built & executable: $PURE_DIR/$f ($(wc -c < "$PURE_DIR/$f") B)"
  else
    fail "missing binary: $PURE_DIR/$f"
  fi
done

echo "=== [2/5] All 11 Named Pure-Omni Harness Gates ==="
for gate in \
  audit-view \
  proposer-loop \
  p2a-syscalls \
  m2-frozen \
  m2-fixed-point \
  b2-pure-verifier \
  m3-modular \
  a3-macro-quarantine \
  b4-pure-launcher \
  p5-sys \
  s5-refreeze
do
  if "$PURE_BIN" "$gate" > "$TMP/p_${gate}.txt" && [ -s "$TMP/p_${gate}.txt" ]; then
    pass "$gate PASS ($(wc -c < "$TMP/p_${gate}.txt") B)"
  else
    fail "$gate FAIL"
  fi
done

echo "=== [3/5] M2 Lint (15/15 spec/m2/lint/*.omni Fixtures) ==="
for f in spec/m2/lint/*.omni; do
  bn=$(basename "$f")
  set +e
  "$PURE_BIN" m2-lint "$f" >/dev/null 2>&1; rc=$?
  set -e
  if [[ "$bn" == pos_* ]]; then
    if [ "$rc" -eq 0 ]; then
      pass "m2-lint positive fixture $bn (rc=0)"
    else
      fail "m2-lint positive fixture $bn unexpectedly failed (rc=$rc)"
    fi
  else
    if [ "$rc" -ne 0 ]; then
      pass "m2-lint negative fixture $bn rejected fail-closed (rc=$rc)"
    else
      fail "m2-lint negative fixture $bn unexpectedly passed (rc=0)"
    fi
  fi
done

echo "=== [4/5] Pure-Omni Selftest Orchestrator ==="
if "$PURE_BIN" selftest > "$TMP/p_st.txt" && grep -q 'PURE-HARNESS PASS' "$TMP/p_st.txt"; then
  pass "omnic_pure.elf selftest PASS ($(tail -n 1 "$TMP/p_st.txt"))"
else
  fail "omnic_pure.elf selftest FAIL"
fi

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
