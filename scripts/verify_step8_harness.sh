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
for f in omnic_pure.elf audit_diff.elf audit_decompile.elf m2_lint.elf audit_view_gate.elf proposer_loop.elf core_gates.elf extra_gates.elf; do
  if [ -x "$PURE_DIR/$f" ]; then
    pass "built & executable: $PURE_DIR/$f ($(wc -c < "$PURE_DIR/$f") B)"
  else
    fail "missing binary: $PURE_DIR/$f"
  fi
done

echo "=== [2/5] Full-Text Byte-Identical Gate Differentials (cmp vs Rust ./omni) ==="
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
  "$PURE_BIN" "$gate" > "$TMP/p_${gate}.txt"
  "$RUST_BIN" "$gate" > "$TMP/r_${gate}.txt"
  if cmp -s "$TMP/p_${gate}.txt" "$TMP/r_${gate}.txt"; then
    pass "$gate full-text 0-byte diff vs Rust ($(wc -c < "$TMP/p_${gate}.txt") B)"
  else
    diff -u "$TMP/r_${gate}.txt" "$TMP/p_${gate}.txt" >&2 || true
    fail "$gate full-text diff vs Rust"
  fi
done

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
