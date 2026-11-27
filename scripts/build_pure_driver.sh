#!/usr/bin/env bash
# ==============================================================================
# OMNI PURE-OMNI ZERO-RUST, ZERO-PYTHON TOOLCHAIN BUILD (`scripts/build_pure_driver.sh`)
# ==============================================================================
# Rebuilds the entire Pure-Omni native toolchain, driver, audit tools, and
# harness ELFs into `build/pure-bin/` starting ONLY from the committed bootstrap
# binaries (`bootstrap/m2.elf`, `bootstrap/m3.elf`) and `.omni`/`.omni_bc` files.
# ZERO `rustc`, ZERO `cargo`, and ZERO external scripting interpreters anywhere in this build.
# ==============================================================================
set -euo pipefail

export PATH="/usr/bin:/bin"
if command -v rustc >/dev/null 2>&1 || command -v cargo >/dev/null 2>&1; then
  echo "[PURE-BUILD FAIL] rustc or cargo is reachable on PATH ($PATH)" >&2
  exit 1
fi

OUT_DIR="${1:-build/pure-bin}"
BOOT_M2="bootstrap/m2.elf"
BOOT_M3="bootstrap/m3.elf"

for bin in "$BOOT_M2" "$BOOT_M3"; do
  if [ ! -x "$bin" ]; then
    echo "[PURE-BUILD FAIL] Missing committed bootstrap binary '$bin' (see bootstrap/MANIFEST.md)." >&2
    exit 1
  fi
done

mkdir -p "$OUT_DIR"
TMP=$(mktemp -d /tmp/omni_pure_build.XXXXXX)
trap 'rm -rf "$TMP"' EXIT

# 1. Build zero-include Pure-Omni build utility (`pure/m5/pure_build_util.omni`)
#    directly via bootstrap/m3.elf + bootstrap/m2.elf (ZERO Python!).
LC_ALL=C od -An -tu1 -v pure/m5/pure_build_util.omni | LC_ALL=C awk '{for(i=1;i<=NF;i++) printf "%c%c", $i, 0}' > "$TMP/pbu.u16"
"$BOOT_M3" "$TMP/pbu.u16" 1 > "$TMP/pbu.omni_bc"
"$BOOT_M2" "$TMP/pbu.omni_bc" > "$TMP/pure_build_util.elf"
chmod +x "$TMP/pure_build_util.elf"

verify_pin() {
  local path="$1"
  local want_len="$2"
  local want_fnv="$3"
  local got_len got_fnv
  read -r got_len got_fnv < <("$TMP/pure_build_util.elf" fnv "$path")
  if [ "$got_len" != "$want_len" ] || [ "$got_fnv" != "$want_fnv" ]; then
    echo "[PURE-BUILD FAIL] pin mismatch for $path: got ${got_len} B / ${got_fnv}, want ${want_len} B / ${want_fnv}" >&2
    exit 1
  fi
}

# 2. Verify committed bootstrap binaries against bootstrap/MANIFEST.md pins.
verify_pin "$BOOT_M2" 126032 0x1F040D79AE834D14
verify_pin "$BOOT_M3" 207015 0x7397A5020895AEBF

# 3. Rebuild m2.elf through 3 generations (S1 == S2 == S3 == bootstrap/m2.elf).
"$BOOT_M2" pure/m2/m2.omni_bc > "$TMP/m2_s1.elf"
chmod +x "$TMP/m2_s1.elf"
"$TMP/m2_s1.elf" pure/m2/m2.omni_bc > "$TMP/m2_s2.elf"
chmod +x "$TMP/m2_s2.elf"
"$TMP/m2_s2.elf" pure/m2/m2.omni_bc > "$OUT_DIR/m2.elf"
chmod +x "$OUT_DIR/m2.elf"
cmp "$BOOT_M2" "$TMP/m2_s1.elf"
cmp "$TMP/m2_s1.elf" "$TMP/m2_s2.elf"
cmp "$TMP/m2_s2.elf" "$OUT_DIR/m2.elf"
verify_pin "$OUT_DIR/m2.elf" 126032 0x1F040D79AE834D14
echo "[PURE-BUILD] m2.elf fixed point S1 == S2 == S3 == bootstrap/m2.elf (126032 B, 0x1F040D79AE834D14)"

# 4. Rebuild m4.elf and verifier.elf from committed .omni_bc seeds via freshly built m2.elf.
"$OUT_DIR/m2.elf" pure/m4/m4_standalone.omni_bc > "$OUT_DIR/m4.elf"
chmod +x "$OUT_DIR/m4.elf"
verify_pin "$OUT_DIR/m4.elf" 91576 0xFBA8272337B1BAC4
echo "[PURE-BUILD] m4.elf rebuilt (91576 B, 0xFBA8272337B1BAC4)"

"$OUT_DIR/m2.elf" pure/verifier_standalone.omni_bc > "$OUT_DIR/verifier.elf"
chmod +x "$OUT_DIR/verifier.elf"
verify_pin "$OUT_DIR/verifier.elf" 40562 0x965E0FA56E7B6046
echo "[PURE-BUILD] verifier.elf rebuilt (40562 B, 0x965E0FA56E7B6046)"

cp "$BOOT_M3" "$OUT_DIR/m3.elf"
chmod +x "$OUT_DIR/m3.elf"

check_m3_and_verify() {
  local bc="$1"
  local ver="$2"
  local src="$3"
  local bc_sz w0 w1 w2 w3 ver_errs
  bc_sz=$(wc -c < "$bc")
  read -r w0 w1 w2 w3 < <(od -An -tu2 -N 8 "$bc")
  if [ "$bc_sz" -eq 16 ] && [ "${w0:-0}" -eq 65535 ]; then
    echo "[PURE-BUILD FAIL] m3.elf error M3_CODE_${w1} at ${w2}:${w3} compiling ${src}" >&2
    exit 1
  fi
  if [ "$bc_sz" -lt 4 ] || [ $((bc_sz % 2)) -ne 0 ]; then
    echo "[PURE-BUILD FAIL] invalid .omni_bc len ${bc_sz} compiling ${src}" >&2
    exit 1
  fi
  echo "  m3.elf: $((bc_sz / 2)) words"
  "$OUT_DIR/verifier.elf" "$bc" > "$ver"
  ver_errs=$(od -An -tu2 -N 2 "$ver" | tr -d ' \n')
  if [ "${ver_errs:-1}" -ne 0 ]; then
    echo "[PURE-BUILD FAIL] verifier.elf rejected ${src} with ${ver_errs} error(s)" >&2
    exit 1
  fi
  echo "  verifier.elf: PASS (0 errors)"
}

# Verify pure_build_util.omni under freshly built verifier.elf as well.
check_m3_and_verify "$TMP/pbu.omni_bc" "$TMP/pbu.ver" "pure/m5/pure_build_util.omni" >/dev/null

declare -A SEEN_INC=()
expand_omni() {
  local ap dir line got_nl inc cand
  ap=$(realpath -m --relative-to=. "$1")
  if [ -n "${SEEN_INC[$ap]+x}" ]; then
    return 0
  fi
  SEEN_INC[$ap]=1
  dir=$(dirname "$ap")
  while true; do
    if IFS= read -r line; then
      got_nl=1
    elif [ -n "$line" ]; then
      got_nl=0
    else
      break
    fi
    if [[ "$line" =~ ^[[:space:]]*include[[:space:]]+\"([^\"]+)\"[[:space:]]*\;[[:space:]]*$ ]]; then
      inc="${BASH_REMATCH[1]}"
      if [ -f "$inc" ]; then
        cand="$inc"
      else
        cand="$dir/$inc"
      fi
      expand_omni "$cand"
      printf '\n'
    elif [ "$got_nl" -eq 1 ]; then
      printf '%s\n' "$line"
    else
      printf '%s' "$line"
    fi
  done < "$ap"
}

build_pure_elf() {
  local src="$1"
  local name="$2"
  echo "[PURE-BUILD] $src -> $OUT_DIR/$name (via Pure-Omni m3.elf + verifier.elf + m2.elf)"
  SEEN_INC=()
  expand_omni "$src" > "$TMP/${name}.expanded.omni"
  "$TMP/pure_build_util.elf" widen "$TMP/${name}.expanded.omni" "$TMP/${name}.u16"
  "$OUT_DIR/m3.elf" "$TMP/${name}.u16" 1 > "$TMP/${name}.omni_bc"
  check_m3_and_verify "$TMP/${name}.omni_bc" "$TMP/${name}.ver" "$src"
  "$OUT_DIR/m2.elf" "$TMP/${name}.omni_bc" > "$OUT_DIR/$name"
  chmod +x "$OUT_DIR/$name"
  echo "  m2.elf: $(wc -c < "$OUT_DIR/$name") B -> $OUT_DIR/$name"
}

build_pure_elf "pure/audit/diff_summary.omni"      "audit_diff.elf"
build_pure_elf "pure/audit/decompile.omni"         "audit_decompile.elf"
build_pure_elf "pure/driver/omnic_driver.omni"     "omnic_pure.elf"
build_pure_elf "pure/harness/m2_lint.omni"         "m2_lint.elf"
build_pure_elf "pure/harness/audit_view_gate.omni" "audit_view_gate.elf"
build_pure_elf "pure/harness/proposer_loop.omni"   "proposer_loop.elf"
build_pure_elf "pure/harness/proposer_live.omni"   "proposer_live.elf"
build_pure_elf "pure/harness/core_gates.omni"      "core_gates.elf"
build_pure_elf "pure/harness/extra_gates.omni"     "extra_gates.elf"
build_pure_elf "pure/m7/judge_cli.omni"            "judge_cli.elf"
build_pure_elf "pure/harness/p7_gates.omni"        "p7_gates.elf"
build_pure_elf "pure/harness/fuzz_gates.omni"      "fuzz_gates.elf"

# 5. Rebuild m3.elf from modular seeds via freshly built omnic_pure.elf link-elf
#    and verify bit-exact parity with bootstrap/m3.elf!
"$OUT_DIR/omnic_pure.elf" link-elf pure/m3/m3_standalone.omni_bc pure/m3/mod_parse.omni_bc -o "$TMP/m3_rebuilt.elf"
cmp "$BOOT_M3" "$TMP/m3_rebuilt.elf"
cp "$TMP/m3_rebuilt.elf" "$OUT_DIR/m3.elf"
chmod +x "$OUT_DIR/m3.elf"
verify_pin "$OUT_DIR/m3.elf" 207015 0x7397A5020895AEBF
echo "[PURE-BUILD] m3.elf rebuilt via Pure-Omni link-elf == bootstrap/m3.elf (207015 B, 0x7397A5020895AEBF)"

echo "[PURE-BUILD PASS] Built complete Pure-Omni toolchain, driver, audit, & harness in $OUT_DIR with ZERO rustc/cargo/Python."
