#!/usr/bin/env bash
# ==============================================================================
# OMNI PURE-OMNI `omnic` DRIVER VERIFICATION SUITE (`scripts/verify_step7_driver.sh`)
# ==============================================================================
# Verifies (with ZERO Rust files, ZERO rustc/cargo, ZERO Python):
#   1. Zero-Rust build of the Pure-Omni toolchain, driver, and audit tools via
#      `scripts/build_pure_driver.sh` starting from `bootstrap/m2.elf` + `bootstrap/m3.elf`.
#   2. Driver subcommands: --version/-V/version, compile, compile-elf, link-elf,
#      run (native x86-64 exec), audit-diff (plain & --tsv), decompile, inspect --m3.
#   3. Self-compilation fixed point (`omnic_pure.elf` compiles itself -> 0-byte diff)
#      and `link-elf` modular M3 rebuild (`206,946 B`, 0-byte diff vs `bootstrap/m3.elf`).
#   4. Fail-closed negative controls (quarantined `macro`, truncated `.omni_bc`,
#      unknown subcommand, missing arguments).
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

echo "=== [1/8] Zero-Rust Pure-Omni Driver Build ==="
bash scripts/build_pure_driver.sh "$PURE_DIR"
for f in omnic_pure.elf audit_decompile.elf audit_diff.elf m2.elf m3.elf m4.elf verifier.elf; do
  if [ -x "$PURE_DIR/$f" ]; then
    pass "binary present & executable: $PURE_DIR/$f ($(wc -c < "$PURE_DIR/$f") B)"
  else
    fail "missing binary: $PURE_DIR/$f"
  fi
done

TMP=$(mktemp -d /tmp/omni_step7_verify.XXXXXX)
trap 'rm -rf "$TMP"' EXIT

echo "=== [2/8] Version Flags (--version, -V, version) ==="
for flag in --version -V version; do
  OUT=$("$PURE_BIN" "$flag")
  if [ "$OUT" = "omni 2.0.0 (pure-omni)" ]; then
    pass "$flag -> '$OUT'"
  else
    fail "$flag -> unexpected '$OUT'"
  fi
done

echo "=== [3/8] Pure-Omni compile + self-compile fixed point ==="
for src in \
  pure/programs/compiler_self_audit.omni \
  pure/programs/kv_ledger_service.omni \
  pure/programs/pack5_decision_controller.omni \
  pure/m5/zero_rust_driver.omni \
  pure/audit/diff_summary.omni \
  pure/audit/decompile.omni
do
  tag=$(echo "$src" | tr '/.' '__')
  "$PURE_BIN" compile "$src" "$TMP/p_${tag}.omni_bc"
  sz=$(wc -c < "$TMP/p_${tag}.omni_bc")
  if [ "$sz" -ge 8 ] && [ $((sz % 2)) -eq 0 ]; then
    pass "compile $src (${sz} B)"
  else
    fail "compile $src invalid output (${sz} B)"
  fi
done

"$PURE_BIN" compile pure/driver/omnic_driver.omni "$TMP/p_self_driver.omni_bc"
"$PURE_BIN" compile-elf "$TMP/p_self_driver.omni_bc" "$TMP/p_self_driver.elf"
if cmp -s "$TMP/p_self_driver.elf" "$PURE_BIN"; then
  pass "self-compile fixed-point: omnic_pure.elf -> omnic_pure.elf (0-byte diff, $(wc -c < "$PURE_BIN") B)"
else
  fail "self-compile fixed-point mismatch (self=$(wc -c < "$TMP/p_self_driver.elf") B vs orig=$(wc -c < "$PURE_BIN") B)"
fi

echo "=== [4/8] Pure-Omni compile-elf ==="
for pair in \
  "pure/m2/m2.omni_bc:m2:124248" \
  "pure/m4/m4.omni_bc:m4:90402" \
  "pure/verifier_standalone.omni_bc:verifier:75340"
do
  IFS=':' read -r bc name exp_sz <<< "$pair"
  "$PURE_BIN" compile-elf "$bc" "$TMP/p_${name}.elf"
  sz=$(wc -c < "$TMP/p_${name}.elf")
  if [ "$sz" = "$exp_sz" ]; then
    pass "compile-elf $bc (${sz} B)"
  else
    fail "compile-elf $bc mismatch (sz=${sz}, expected=${exp_sz})"
  fi
done
if cmp -s "$TMP/p_m2.elf" bootstrap/m2.elf; then
  pass "compile-elf m2.omni_bc == bootstrap/m2.elf (0-byte diff)"
else
  fail "compile-elf m2.omni_bc differed from bootstrap/m2.elf"
fi

"$PURE_BIN" compile-elf pure/programs/pack5_decision_controller.omni "$TMP/p_pack5.elf" 45 10 40
chmod +x "$TMP/p_pack5.elf"
NATIVE_PACK5=$("$TMP/p_pack5.elf" | od -An -td8 | tr -d ' \n')
pass "compile-elf pack5_decision_controller.omni 45 10 40 ($(wc -c < "$TMP/p_pack5.elf") B, native=${NATIVE_PACK5})"

echo "=== [5/8] Pure-Omni link-elf (exact 207,015 B m3.elf == bootstrap/m3.elf) ==="
"$PURE_BIN" link-elf pure/m3/m3_standalone.omni_bc pure/m3/mod_parse.omni_bc -o "$TMP/p_m3.elf"
m3_sz=$(wc -c < "$TMP/p_m3.elf")
if cmp -s "$TMP/p_m3.elf" bootstrap/m3.elf && [ "$m3_sz" = "207015" ]; then
  pass "link-elf m3_standalone + mod_parse -> m3.elf (0-byte diff vs bootstrap/m3.elf, 207,015 B)"
else
  fail "link-elf m3.elf mismatch (sz=${m3_sz})"
fi

echo "=== [6/8] Pure-Omni run (native x86-64 exec path) ==="
OUT_P=$("$PURE_BIN" run pure/programs/kv_ledger_service.omni 10)
if [ "$OUT_P" = "Int(0)" ]; then
  pass "run kv_ledger_service.omni 10 -> $OUT_P"
else
  fail "run kv_ledger_service.omni 10 -> '$OUT_P' (want Int(0))"
fi

OUT_P=$("$PURE_BIN" run pure/programs/compiler_self_audit.omni 10 20 30)
if [ "$OUT_P" = "Int(-92229)" ]; then
  pass "run compiler_self_audit.omni 10 20 30 -> $OUT_P"
else
  fail "run compiler_self_audit.omni 10 20 30 -> '$OUT_P' (want Int(-92229))"
fi

OUT_P_BC=$("$PURE_BIN" run "$TMP/p_pure_programs_compiler_self_audit_omni.omni_bc" 10 20 30)
if [ "$OUT_P_BC" = "Int(-92229)" ]; then
  pass "run <precompiled.omni_bc> 10 20 30 -> $OUT_P_BC"
else
  fail "run <precompiled.omni_bc> 10 20 30 -> '$OUT_P_BC'"
fi

OUT_P_PACK5=$("$PURE_BIN" run pure/programs/pack5_decision_controller.omni 45 10 40)
if [ "$OUT_P_PACK5" = "Int(${NATIVE_PACK5})" ]; then
  pass "run pack5_decision_controller.omni 45 10 40 -> $OUT_P_PACK5 (matches native ELF)"
else
  fail "run pack5_decision_controller.omni 45 10 40: pure='$OUT_P_PACK5' vs native='Int(${NATIVE_PACK5})'"
fi

echo "=== [7/8] Pure-Omni audit-diff (plain & --tsv) and decompile / inspect --m3 ==="
BC_A="$TMP/p_pure_programs_pack5_decision_controller_omni.omni_bc"
BC_B="$TMP/p_pure_programs_compiler_self_audit_omni.omni_bc"
for pair in \
  "${BC_A}:${BC_A}:identical_pack5" \
  "${BC_A}:${BC_B}:pack5_vs_self_audit" \
  "pure/verifier_standalone.omni_bc:pure/m2/m2.omni_bc:verifier_vs_m2"
do
  IFS=':' read -r bef aft label <<< "$pair"
  "$PURE_BIN" audit-diff "$bef" "$aft" > "$TMP/p_ad_${label}.txt"
  "$PURE_BIN" audit-diff "$bef" "$aft" --tsv > "$TMP/p_tsv_${label}.txt"
  if [ -s "$TMP/p_ad_${label}.txt" ] && [ -s "$TMP/p_tsv_${label}.txt" ]; then
    pass "audit-diff plain & --tsv ($label)"
  else
    fail "audit-diff empty output ($label)"
  fi
done

"$PURE_BIN" decompile pure/verifier_standalone.omni_bc > "$TMP/p_dc_ver.omni"
"$PURE_BIN" inspect --m3 pure/verifier_standalone.omni_bc > "$TMP/p_ins_ver.omni"
if cmp -s "$TMP/p_dc_ver.omni" "$TMP/p_ins_ver.omni" && grep -q 'fn fn_0(' "$TMP/p_dc_ver.omni"; then
  pass "decompile == inspect --m3 (verifier_standalone, $(wc -c < "$TMP/p_dc_ver.omni") B)"
else
  fail "decompile / inspect --m3 mismatch on verifier_standalone"
fi

"$PURE_BIN" decompile pure/programs/pack5_decision_controller.omni > "$TMP/p_dc_src.omni"
"$PURE_BIN" decompile "$BC_A" > "$TMP/p_dc_bc.omni"
if cmp -s "$TMP/p_dc_src.omni" "$TMP/p_dc_bc.omni"; then
  pass "decompile .omni source == decompile .omni_bc (0-byte diff)"
else
  fail "decompile .omni source differed from decompile .omni_bc"
fi

echo "=== [8/8] Negative Controls (fail-closed) ==="
for mac_src in pure/programs/policy_consensus_engine.omni pure/programs/ternary_neural_router.omni; do
  if "$PURE_BIN" compile "$mac_src" "$TMP/should_fail.omni_bc" >/dev/null 2>&1; then
    fail "negative control: macro source $mac_src unexpectedly succeeded"
  else
    pass "negative control: quarantined macro $mac_src rejected fail-closed"
  fi
done

head -c 20 "$BC_A" > "$TMP/trunc.omni_bc"
if "$PURE_BIN" decompile "$TMP/trunc.omni_bc" >/dev/null 2>&1; then
  fail "negative control: truncated .omni_bc unexpectedly decompiled"
else
  pass "negative control: truncated .omni_bc rejected by decompile"
fi
if "$PURE_BIN" audit-diff "$TMP/trunc.omni_bc" "$BC_A" >/dev/null 2>&1; then
  fail "negative control: truncated .omni_bc unexpectedly passed audit-diff"
else
  pass "negative control: truncated .omni_bc rejected by audit-diff"
fi

if "$PURE_BIN" no-such-cmd >/dev/null 2>&1; then
  fail "negative control: unknown subcommand exited 0"
else
  pass "negative control: unknown subcommand rejected"
fi
if "$PURE_BIN" >/dev/null 2>&1; then
  fail "negative control: zero args exited 0"
else
  pass "negative control: zero args rejected"
fi

echo "=============================================================================="
echo "[STEP7 SUMMARY] PASS=$PASS FAIL=$FAIL"
echo "=============================================================================="
[ "$FAIL" -eq 0 ]
