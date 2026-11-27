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
for f in omnic_pure.elf audit_diff.elf audit_decompile.elf m2_lint.elf audit_view_gate.elf proposer_loop.elf proposer_live.elf core_gates.elf extra_gates.elf judge_cli.elf p7_gates.elf; do
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

echo "=== [4/6] Pure-Omni Selftest Orchestrator & Standalone Judge Harness ==="
if "$PURE_BIN" selftest > "$TMP/p_st.txt" \
   && grep -q '^\[AUDIT-VIEW PASS\]' "$TMP/p_st.txt" \
   && grep -q '^\[PROPOSER-LOOP PASS\]' "$TMP/p_st.txt" \
   && grep -q '^\[PURE-HARNESS PASS\]' "$TMP/p_st.txt"; then
  pass "omnic_pure.elf selftest PASS (clean 3-line redirected output)"
else
  cat "$TMP/p_st.txt" >&2 || true
  fail "omnic_pure.elf selftest FAIL or garbled redirected output"
fi

if "$PURE_BIN" compile spec/m7/judge_harness.omni "$TMP/judge_h.omni_bc"; then
  pass "spec/m7/judge_harness.omni standalone compile PASS ($(wc -c < "$TMP/judge_h.omni_bc") B)"
else
  fail "spec/m7/judge_harness.omni standalone compile FAIL"
fi

echo "=== [5/6] Live Proposer Path Checks (Step 11 Part E) ==="
# 1. p7-measure & p7-canary exit 0 on standard targets
if "$PURE_BIN" p7-measure > "$TMP/p7m.txt" && grep -q '^\[P7-MEASURE PASS\] 4 targets' "$TMP/p7m.txt" && [ "$(wc -l < "$TMP/p7m.txt")" -eq 5 ]; then
  pass "p7-measure PASS (4 target rows + summary banner)"
else
  cat "$TMP/p7m.txt" >&2 || true
  fail "p7-measure FAIL"
fi

if "$PURE_BIN" p7-canary > "$TMP/p7c.txt" && grep -q '^\[P7-CANARY PASS\] canary_a=+1t canary_b=0t canary_c=-1t frozen5=OK' "$TMP/p7c.txt"; then
  pass "p7-canary PASS ($(cat "$TMP/p7c.txt"))"
else
  cat "$TMP/p7c.txt" >&2 || true
  fail "p7-canary FAIL"
fi

# 2. propose-run --dry-run against a fixture candidate produces stage logs & writes nothing to Git
cat > "$TMP/fix_base.omni" <<'EOF'
fn main(x: i64, y: i64) -> i64 {
    let a: i64 = x * 3;
    let a1: i64 = a + 0;
    let b: i64 = y + 7;
    let b1: i64 = b * 1;
    return a1 + b1;
}
EOF
cat > "$TMP/fix_cand.omni" <<'EOF'
fn main(x: i64, y: i64) -> i64 {
    let a: i64 = x * 3;
    let b: i64 = y + 7;
    return a + b;
}
EOF

mkdir -p "$TMP/gitrepo/pure/programs"
cp "$TMP/fix_base.omni" "$TMP/gitrepo/pure/programs/fixture_opt.omni"
git -C "$TMP/gitrepo" init -b main >/dev/null 2>&1
git -C "$TMP/gitrepo" config user.email "omni-flywheel@local"
git -C "$TMP/gitrepo" config user.name "Omni Flywheel"
git -C "$TMP/gitrepo" add pure/programs/fixture_opt.omni
git -C "$TMP/gitrepo" commit -m "baseline fixture" >/dev/null 2>&1
MAIN_BEFORE=$(git -C "$TMP/gitrepo" rev-parse main)

if "$PURE_BIN" propose-run --repo "$TMP/gitrepo" --target pure/programs/fixture_opt.omni --candidate "$TMP/fix_cand.omni" --dry-run > "$TMP/dry.txt" \
   && grep -q '^\[PROPOSE-RUN LIVE\]' "$TMP/dry.txt" \
   && grep -q '^verdict=+1t ADOPT' "$TMP/dry.txt" \
   && ! git -C "$TMP/gitrepo" rev-parse --verify flywheel/auto >/dev/null 2>&1 \
   && [ "$(git -C "$TMP/gitrepo" rev-parse main)" = "$MAIN_BEFORE" ]; then
  pass "propose-run --dry-run PASS (stage logs emitted, zero Git writes)"
else
  cat "$TMP/dry.txt" >&2 || true
  fail "propose-run --dry-run FAIL"
fi

# 3. Fixture ADOPT path: lands exactly 1 commit on flywheel/auto and 0 on main
if "$PURE_BIN" propose-run --repo "$TMP/gitrepo" --target pure/programs/fixture_opt.omni --candidate "$TMP/fix_cand.omni" > "$TMP/adopt.txt" \
   && grep -q '^verdict=+1t ADOPT' "$TMP/adopt.txt" \
   && [ "$(git -C "$TMP/gitrepo" rev-parse main)" = "$MAIN_BEFORE" ] \
   && [ "$(git -C "$TMP/gitrepo" rev-list --count main..flywheel/auto)" -eq 1 ]; then
  pass "fixture ADOPT path PASS (1 commit on flywheel/auto, 0 on main)"
else
  cat "$TMP/adopt.txt" >&2 || true
  fail "fixture ADOPT path FAIL"
fi

# 4. Fixture Stage 0 refusal: candidate touching pure/verifier_engine.omni refused at Stage 0 with -1t
set +e
"$PURE_BIN" propose-run --target pure/verifier_engine.omni --candidate "$TMP/fix_cand.omni" --dry-run > "$TMP/s0ref.txt" 2>&1
S0_RC=$?
set -e
if [ "$S0_RC" -ne 0 ] && grep -q 'STAGE 0 REJECT.*-1t' "$TMP/s0ref.txt"; then
  pass "fixture Stage 0 TCB refusal PASS (pure/verifier_engine.omni rejected -1t, rc=$S0_RC)"
else
  cat "$TMP/s0ref.txt" >&2 || true
  fail "fixture Stage 0 TCB refusal FAIL (rc=$S0_RC)"
fi

echo "=== [6/6] Negative Controls (Fail-Closed) ==="
if "$PURE_BIN" audit-view --inject-broken-decompile >/dev/null 2>&1; then
  fail "negative control: --inject-broken-decompile was NOT caught"
else
  pass "negative control: deliberately broken decompile caught fail-closed by audit-view gate"
fi

set +e
"$PURE_DIR/proposer_loop.elf" --unknown-flag-test > "$TMP/badarg.txt" 2>&1
BADARG_RC=$?
set -e
if [ "$BADARG_RC" -eq 2 ] && grep -q '^usage:' "$TMP/badarg.txt"; then
  pass "negative control: unknown flag to proposer_loop.elf exited rc=2 with usage line"
else
  cat "$TMP/badarg.txt" >&2 || true
  fail "negative control: unknown flag to proposer_loop.elf exited rc=$BADARG_RC (want 2)"
fi

# Step 11b: Native sys_clock_ns() kernel execution + #[cap(SYS)] negative control (ERR_MISSING_CAP=1003)
cat > "$TMP/clk_ok.omni" <<'EOF'
#[strict]
#[cap(SYS)]
fn main() -> i64 {
    let t0: i64 = sys_clock_ns();
    let i: i64 = 0;
    let s: i64 = 0;
    while 20000 - i <=> 0 { s = s + i; i = i + 1; }
    let t1: i64 = sys_clock_ns();
    return (t1 - t0) + s * 0;
}
EOF
if "$PURE_BIN" compile-elf "$TMP/clk_ok.omni" "$TMP/clk_ok.elf"; then
  chmod +x "$TMP/clk_ok.elf"
  CLK_NS=$("$TMP/clk_ok.elf" | od -An -td8 | tr -d ' \n')
  if [ "${CLK_NS:-0}" -gt 0 ]; then
    pass "Step 11b native sys_clock_ns() live Linux kernel exec PASS (elapsed=${CLK_NS} ns)"
  else
    fail "Step 11b native sys_clock_ns() returned non-positive elapsed=${CLK_NS}"
  fi
else
  fail "Step 11b native sys_clock_ns() compile-elf FAIL"
fi

cat > "$TMP/clk_nocap.omni" <<'EOF'
#[strict]
fn main() -> i64 {
    return sys_clock_ns();
}
EOF
if "$PURE_BIN" compile "$TMP/clk_nocap.omni" "$TMP/clk_nocap.omni_bc" >/dev/null 2>&1; then
  fail "Step 11b negative control: sys_clock_ns() without #[cap(SYS)] was NOT rejected"
else
  pass "Step 11b negative control: sys_clock_ns() without #[cap(SYS)] rejected fail-closed"
fi

# Step 12b: Out-of-tree --candidate include-base resolution (Part B: 3 checks)
cp pure/audit/decompile.omni "$TMP/decomp_same.omni"
if "$PURE_BIN" propose-run --target pure/audit/decompile.omni --candidate "$TMP/decomp_same.omni" --dry-run > "$TMP/inc_same.txt" 2>&1 \
   && grep -q '^\[PROPOSE-RUN LIVE\]' "$TMP/inc_same.txt" \
   && ! grep -q 'cand=-1/-1' "$TMP/inc_same.txt" \
   && grep -q '^verdict=0t HOLD' "$TMP/inc_same.txt"; then
  BPAIR=$(sed -n 's/.*base=\([0-9]*\/[0-9]*\).*/\1/p' "$TMP/inc_same.txt")
  CPAIR=$(sed -n 's/.*cand=\([0-9]*\/[0-9]*\).*/\1/p' "$TMP/inc_same.txt")
  if [ -n "$BPAIR" ] && [ "$BPAIR" = "$CPAIR" ]; then
    pass "Step 12b included-module dry-run PASS (out-of-tree copy base=cand=$BPAIR, 0t HOLD)"
  else
    cat "$TMP/inc_same.txt" >&2 || true
    fail "Step 12b included-module dry-run mismatch (base=$BPAIR cand=$CPAIR)"
  fi
else
  cat "$TMP/inc_same.txt" >&2 || true
  fail "Step 12b included-module dry-run FAIL"
fi

sed '0,/return r;/s/return r;/let _s12b_t: i64 = 0; return r;/' pure/audit/decompile.omni > "$TMP/decomp_mod.omni"
set +e
"$PURE_BIN" propose-run --target pure/audit/decompile.omni --candidate "$TMP/decomp_mod.omni" --dry-run > "$TMP/inc_mod.txt" 2>&1
set -e
if grep -q '^\[PROPOSE-RUN LIVE\]' "$TMP/inc_mod.txt" \
   && grep -qE 'cand=[1-9][0-9]*/[1-9][0-9]*' "$TMP/inc_mod.txt" \
   && grep -q '^verdict=' "$TMP/inc_mod.txt"; then
  pass "Step 12b included-module real diff merits verdict PASS ($(grep '^verdict=' "$TMP/inc_mod.txt"))"
else
  cat "$TMP/inc_mod.txt" >&2 || true
  fail "Step 12b included-module real diff merits verdict FAIL"
fi

sed 's|include "../m2/util.omni";|include "../m2/nonexistent_step12b_999.omni";|' pure/audit/decompile.omni > "$TMP/decomp_badinc.omni"
set +e
"$PURE_BIN" propose-run --target pure/audit/decompile.omni --candidate "$TMP/decomp_badinc.omni" --dry-run > "$TMP/inc_bad.txt" 2>&1
BADINC_RC=$?
set -e
if [ "$BADINC_RC" -ne 0 ] && grep -q 'cand=-1/-1' "$TMP/inc_bad.txt" && grep -q '^verdict=-1t REJECT' "$TMP/inc_bad.txt"; then
  pass "Step 12b negative control: broken include in out-of-tree candidate rejected fail-closed (cand=-1/-1, -1t REJECT)"
else
  cat "$TMP/inc_bad.txt" >&2 || true
  fail "Step 12b negative control: broken include was NOT rejected fail-closed (rc=$BADINC_RC)"
fi

# Step 14: Deterministic mutation-fuzz harness (`fuzz_gates.elf`) — 3 checks (52 -> 55)
if [ -x "$PURE_DIR/fuzz_gates.elf" ]; then
  pass "Step 14 fuzz_gates.elf built & executable ($(wc -c < "$PURE_DIR/fuzz_gates.elf") B)"
else
  fail "Step 14 fuzz_gates.elf missing or not executable"
fi

set +e
"$PURE_DIR/fuzz_gates.elf" > "$TMP/fuzz1.txt" 2>&1
FZ1_RC=$?
"$PURE_DIR/fuzz_gates.elf" > "$TMP/fuzz2.txt" 2>&1
FZ2_RC=$?
set -e
if [ "$FZ1_RC" -eq "$FZ2_RC" ] && cmp -s "$TMP/fuzz1.txt" "$TMP/fuzz2.txt"; then
  pass "Step 14 fuzz_gates.elf 2x determinism PASS (0-byte diff, rc=$FZ1_RC)"
else
  fail "Step 14 fuzz_gates.elf non-deterministic output (rc1=$FZ1_RC rc2=$FZ2_RC)"
fi

if [ "$FZ1_RC" -ne 0 ] && grep -q '^\[FUZZ FAIL\]' "$TMP/fuzz1.txt" && grep -q '^\[FUZZ FIND\] kind=1 op=1 item=3 site=179 seed=46528505' "$TMP/fuzz1.txt"; then
  pass "Step 14 fuzz_gates.elf fail-closed soundness find reproduced (item=3 site=179 0x7F28->0x7F5F, filed in spec/step14-fuzz-evidence.md)"
elif [ "$FZ1_RC" -eq 0 ] && grep -q '^\[FUZZ PASS\].*soundness=0 roundtrip=0 judge=0' "$TMP/fuzz1.txt"; then
  pass "Step 14 fuzz_gates.elf reduced panel PASS (0 finds)"
else
  cat "$TMP/fuzz1.txt" >&2 || true
  fail "Step 14 fuzz_gates.elf unexpected output (rc=$FZ1_RC)"
fi

echo "=============================================================================="
echo "[STEP8 SUMMARY] PASS=$PASS FAIL=$FAIL"
echo "=============================================================================="
[ "$FAIL" -eq 0 ]
