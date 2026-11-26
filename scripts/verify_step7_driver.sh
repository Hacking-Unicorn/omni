#!/usr/bin/env bash
# ==============================================================================
# OMNI STEP 7 — PURE-OMNI `omnic` DRIVER ACCEPTANCE & DIFFERENTIAL SUITE
# ==============================================================================
# Verifies:
#   1. Zero-Rust build of `omnic_pure.elf`, `audit_decompile.elf`, `audit_diff.elf`
#      via `scripts/build_pure_driver.sh` (PATH=/usr/bin:/bin, no rustc/cargo).
#   2. Byte-identical differential parity against the Rust bootstrap `./omni` for:
#        - compile (`.omni` -> `.omni_bc`)
#        - compile-elf (`.omni` & `.omni_bc` -> native ELF64, with/without args)
#        - link-elf (`m3_standalone` + `mod_parse` -> exact 206,946 B `m3.elf`)
#        - run (native x86-64 exec path on `.omni` and `.omni_bc`)
#        - audit-diff (plain-language and `--tsv` 0-byte diff)
#        - decompile & inspect --m3 (exact framing, word count, FNV-1a64, fn count)
#        - --version / -V / version
#   3. Fail-closed negative controls (quarantined `macro`, truncated `.omni_bc`,
#      missing arguments, unknown subcommand).
# ==============================================================================
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

RUST_BIN="${RUST_BIN:-./omni}"
PURE_DIR="${PURE_DIR:-build/pure-bin}"
PURE_BIN="$PURE_DIR/omnic_pure.elf"

if [ ! -x "$RUST_BIN" ]; then
  echo "[STEP7 FAIL] Rust differential reference '$RUST_BIN' not found or not executable." >&2
  exit 1
fi

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

echo "=== [3/8] Differential compile (0-byte diff vs Rust ./omni) ==="
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
  "$RUST_BIN" compile "$src" "$TMP/r_${tag}.omni_bc" >/dev/null
  if cmp -s "$TMP/p_${tag}.omni_bc" "$TMP/r_${tag}.omni_bc"; then
    pass "compile $src (0-byte diff, $(wc -c < "$TMP/p_${tag}.omni_bc") B)"
  else
    fail "compile $src byte diff (pure=$(wc -c < "$TMP/p_${tag}.omni_bc") B vs rust=$(wc -c < "$TMP/r_${tag}.omni_bc") B)"
  fi
done

# Self-compilation fixed-point check: omnic_pure.elf compiles itself -> 0-byte diff!
"$PURE_BIN" compile pure/driver/omnic_driver.omni "$TMP/p_self_driver.omni_bc"
"$PURE_BIN" compile-elf "$TMP/p_self_driver.omni_bc" "$TMP/p_self_driver.elf"
if cmp -s "$TMP/p_self_driver.elf" "$PURE_BIN"; then
  pass "self-compile fixed-point: omnic_pure.elf -> omnic_pure.elf (0-byte diff, $(wc -c < "$PURE_BIN") B)"
else
  fail "self-compile fixed-point mismatch (self=$(wc -c < "$TMP/p_self_driver.elf") B vs orig=$(wc -c < "$PURE_BIN") B)"
fi

echo "=== [4/8] Differential compile-elf (0-byte diff vs Rust ./omni) ==="
for pair in \
  "pure/m2/m2.omni_bc:m2:125539" \
  "pure/m4/m4.omni_bc:m4:90402" \
  "pure/verifier_standalone.omni_bc:verifier:37907"
do
  IFS=':' read -r bc name exp_sz <<< "$pair"
  "$PURE_BIN" compile-elf "$bc" "$TMP/p_${name}.elf"
  "$RUST_BIN" compile-elf "$bc" "$TMP/r_${name}.elf" >/dev/null
  sz=$(wc -c < "$TMP/p_${name}.elf")
  if cmp -s "$TMP/p_${name}.elf" "$TMP/r_${name}.elf" && [ "$sz" = "$exp_sz" ]; then
    pass "compile-elf $bc (0-byte diff, ${sz} B)"
  else
    fail "compile-elf $bc mismatch (sz=${sz}, expected=${exp_sz})"
  fi
done

"$PURE_BIN" compile-elf pure/programs/pack5_decision_controller.omni "$TMP/p_pack5.elf" 45 10 40
"$RUST_BIN" compile-elf pure/programs/pack5_decision_controller.omni "$TMP/r_pack5.elf" 45 10 40 >/dev/null
if cmp -s "$TMP/p_pack5.elf" "$TMP/r_pack5.elf"; then
  pass "compile-elf pack5_decision_controller.omni 45 10 40 (0-byte diff, $(wc -c < "$TMP/p_pack5.elf") B)"
else
  fail "compile-elf pack5_decision_controller.omni 45 10 40 byte diff"
fi

echo "=== [5/8] Differential link-elf (exact 206,946 B m3.elf) ==="
"$PURE_BIN" link-elf pure/m3/m3_standalone.omni_bc pure/m3/mod_parse.omni_bc -o "$TMP/p_m3.elf"
"$RUST_BIN" link-elf pure/m3/m3_standalone.omni_bc pure/m3/mod_parse.omni_bc -o "$TMP/r_m3.elf" >/dev/null
m3_sz=$(wc -c < "$TMP/p_m3.elf")
if cmp -s "$TMP/p_m3.elf" "$TMP/r_m3.elf" && [ "$m3_sz" = "206946" ]; then
  pass "link-elf m3_standalone + mod_parse -> m3.elf (0-byte diff, 206,946 B)"
else
  fail "link-elf m3.elf mismatch (sz=${m3_sz})"
fi

echo "=== [6/8] Differential run (native x86-64 exec path) ==="
OUT_P=$("$PURE_BIN" run pure/programs/kv_ledger_service.omni 10)
OUT_R=$("$RUST_BIN" run pure/programs/kv_ledger_service.omni 10)
if [ "$OUT_P" = "$OUT_R" ] && [ "$OUT_P" = "Int(0)" ]; then
  pass "run kv_ledger_service.omni 10 -> $OUT_P (matches Rust)"
else
  fail "run kv_ledger_service.omni 10: pure='$OUT_P' vs rust='$OUT_R'"
fi

OUT_P=$("$PURE_BIN" run pure/programs/compiler_self_audit.omni 10 20 30)
OUT_R=$("$RUST_BIN" run pure/programs/compiler_self_audit.omni 10 20 30)
if [ "$OUT_P" = "$OUT_R" ] && [ "$OUT_P" = "Int(-92229)" ]; then
  pass "run compiler_self_audit.omni 10 20 30 -> $OUT_P (matches Rust)"
else
  fail "run compiler_self_audit.omni 10 20 30: pure='$OUT_P' vs rust='$OUT_R'"
fi

OUT_P_BC=$("$PURE_BIN" run "$TMP/p_pure_programs_compiler_self_audit_omni.omni_bc" 10 20 30)
if [ "$OUT_P_BC" = "Int(-92229)" ]; then
  pass "run <precompiled.omni_bc> 10 20 30 -> $OUT_P_BC"
else
  fail "run <precompiled.omni_bc> 10 20 30 -> '$OUT_P_BC'"
fi

OUT_P_PACK5=$("$PURE_BIN" run pure/programs/pack5_decision_controller.omni 45 10 40)
chmod +x "$TMP/r_pack5.elf"
NATIVE_PACK5=$("$TMP/r_pack5.elf" | od -An -td8 | tr -d ' \n')
if [ "$OUT_P_PACK5" = "Int(${NATIVE_PACK5})" ]; then
  pass "run pack5_decision_controller.omni 45 10 40 -> $OUT_P_PACK5 (matches native ELF)"
else
  fail "run pack5_decision_controller.omni 45 10 40: pure='$OUT_P_PACK5' vs native='Int(${NATIVE_PACK5})'"
fi

echo "=== [7/8] Differential audit-diff (plain & --tsv) and decompile / inspect --m3 ==="
BC_A="$TMP/p_pure_programs_pack5_decision_controller_omni.omni_bc"
BC_B="$TMP/p_pure_programs_compiler_self_audit_omni.omni_bc"
for pair in \
  "${BC_A}:${BC_A}:identical_pack5" \
  "${BC_A}:${BC_B}:pack5_vs_self_audit" \
  "pure/verifier_standalone.omni_bc:pure/m2/m2.omni_bc:verifier_vs_m2"
do
  IFS=':' read -r bef aft label <<< "$pair"
  "$PURE_BIN" audit-diff "$bef" "$aft" > "$TMP/p_ad_${label}.txt"
  "$RUST_BIN" audit-diff "$bef" "$aft" > "$TMP/r_ad_${label}.txt"
  if cmp -s "$TMP/p_ad_${label}.txt" "$TMP/r_ad_${label}.txt"; then
    pass "audit-diff plain ($label): 0-byte diff vs Rust"
  else
    diff -u "$TMP/r_ad_${label}.txt" "$TMP/p_ad_${label}.txt" | head -n 25 >&2 || true
    fail "audit-diff plain ($label): diff vs Rust"
  fi
  "$PURE_BIN" audit-diff "$bef" "$aft" --tsv > "$TMP/p_tsv_${label}.txt"
  "$RUST_BIN" audit-diff "$bef" "$aft" --tsv > "$TMP/r_tsv_${label}.txt"
  if cmp -s "$TMP/p_tsv_${label}.txt" "$TMP/r_tsv_${label}.txt"; then
    pass "audit-diff --tsv ($label): 0-byte diff vs Rust"
  else
    diff -u "$TMP/r_tsv_${label}.txt" "$TMP/p_tsv_${label}.txt" | head -n 25 >&2 || true
    fail "audit-diff --tsv ($label): diff vs Rust"
  fi
done

# Full-text byte-identical decompile & inspect --m3 vs Rust ./omni on:
#   - verifier_standalone (68 fns)
#   - verifier_engine (67 fns)
#   - all 16 audit-view goldens under spec/acceptance/goldens/
"$RUST_BIN" compile pure/verifier_engine.omni "$TMP/verifier_engine.omni_bc" >/dev/null
for pair in \
  "pure/verifier_standalone.omni_bc:verifier_standalone" \
  "$TMP/verifier_engine.omni_bc:verifier_engine"
do
  IFS=':' read -r bc tag <<< "$pair"
  "$PURE_BIN" decompile "$bc" > "$TMP/p_dc_${tag}.omni"
  "$RUST_BIN" decompile "$bc" > "$TMP/r_dc_${tag}.omni"
  if cmp -s "$TMP/p_dc_${tag}.omni" "$TMP/r_dc_${tag}.omni"; then
    pass "decompile full-text 0-byte diff vs Rust ($tag, $(wc -c < "$TMP/p_dc_${tag}.omni") B)"
  else
    diff -u "$TMP/r_dc_${tag}.omni" "$TMP/p_dc_${tag}.omni" | head -n 30 >&2 || true
    fail "decompile full-text diff vs Rust ($tag)"
  fi
  "$PURE_BIN" inspect --m3 "$bc" > "$TMP/p_ins_${tag}.omni"
  "$RUST_BIN" inspect --m3 "$bc" > "$TMP/r_ins_${tag}.omni"
  if cmp -s "$TMP/p_ins_${tag}.omni" "$TMP/r_ins_${tag}.omni"; then
    pass "inspect --m3 full-text 0-byte diff vs Rust ($tag, $(wc -c < "$TMP/p_ins_${tag}.omni") B)"
  else
    diff -u "$TMP/r_ins_${tag}.omni" "$TMP/p_ins_${tag}.omni" | head -n 30 >&2 || true
    fail "inspect --m3 full-text diff vs Rust ($tag)"
  fi
done

for g in \
  seed0_risk seed4_consensus seed6_clamp custom_non_seed_alu \
  iabs_golden idiv_zero shl_65 sel_mix \
  br_pos_golden br_neg_arith_golden nested_branch_golden match_trit_golden \
  branch_loop_golden byte_buf_golden decision_hook_golden arena_lifecycle_golden
do
  src="spec/acceptance/goldens/${g}.omni"
  if ! "$RUST_BIN" compile "$src" "$TMP/g_${g}.omni_bc" >/dev/null 2>&1; then
    "$RUST_BIN" compile --rust "$src" "$TMP/g_${g}.omni_bc" >/dev/null
  fi
  "$PURE_BIN" decompile "$TMP/g_${g}.omni_bc" > "$TMP/p_dc_${g}.omni"
  "$RUST_BIN" decompile "$TMP/g_${g}.omni_bc" > "$TMP/r_dc_${g}.omni"
  "$PURE_BIN" inspect --m3 "$TMP/g_${g}.omni_bc" > "$TMP/p_ins_${g}.omni"
  "$RUST_BIN" inspect --m3 "$TMP/g_${g}.omni_bc" > "$TMP/r_ins_${g}.omni"
  if cmp -s "$TMP/p_dc_${g}.omni" "$TMP/r_dc_${g}.omni" && cmp -s "$TMP/p_ins_${g}.omni" "$TMP/r_ins_${g}.omni"; then
    pass "decompile & inspect --m3 full-text 0-byte diff vs Rust (golden: $g)"
  else
    diff -u "$TMP/r_dc_${g}.omni" "$TMP/p_dc_${g}.omni" | head -n 20 >&2 || true
    fail "decompile / inspect --m3 diff vs Rust (golden: $g)"
  fi
done

"$PURE_BIN" decompile pure/programs/pack5_decision_controller.omni > "$TMP/p_dc_src.omni"
"$RUST_BIN" decompile "$BC_A" > "$TMP/r_dc_pack5.omni"
if cmp -s "$TMP/p_dc_src.omni" "$TMP/r_dc_pack5.omni"; then
  pass "decompile .omni source matches Rust decompile .omni_bc (0-byte diff)"
else
  fail "decompile .omni source differed from Rust decompile .omni_bc"
fi

echo "=== [8/8] Negative Controls (fail-closed) ==="
# 1. Quarantined macro source must fail closed
for mac_src in pure/programs/policy_consensus_engine.omni pure/programs/ternary_neural_router.omni; do
  if "$PURE_BIN" compile "$mac_src" "$TMP/should_fail.omni_bc" >/dev/null 2>&1; then
    fail "negative control: macro source $mac_src unexpectedly succeeded"
  else
    pass "negative control: quarantined macro $mac_src rejected fail-closed"
  fi
done

# 2. Truncated .omni_bc (missing TOK_MOD_END) must fail closed in decompile & audit-diff
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

# 3. Unknown subcommand / missing args must exit non-zero
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
