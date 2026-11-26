#!/usr/bin/env bash
# ==============================================================================
# OMNI STEP 7 — ZERO-RUST, ZERO-PYTHON PURE-OMNI DRIVER BUILD (`scripts/build_pure_driver.sh`)
# ==============================================================================
# Builds the Pure-Omni native driver (`omnic_pure.elf`) and Pure-Omni audit tools
# (`audit_decompile.elf`, `audit_diff.elf`) into `build/pure-bin/` using ONLY the
# Pure-Omni native toolchain (`m3.elf` + `m2.elf` + `verifier.elf`).
# ZERO `rustc`, ZERO `cargo`, and ZERO external scripting interpreters anywhere in this build.
# ==============================================================================
set -euo pipefail

export PATH="/usr/bin:/bin"
if command -v rustc >/dev/null 2>&1 || command -v cargo >/dev/null 2>&1; then
  echo "[PURE-BUILD FAIL] rustc or cargo is reachable on PATH ($PATH)" >&2
  exit 1
fi

OUT_DIR="${1:-build/pure-bin}"
CACHE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/omni/native-v1"
M2_ELF="$CACHE_DIR/m2_0a5cd5b55707aa0d.elf"
M3_ELF="$CACHE_DIR/m3_8cfd973873caff46.elf"
M4_ELF="$CACHE_DIR/m4_fba8272337b1bac4.elf"
VER_ELF="$CACHE_DIR/verifier_2ecd2ce3dcb69346.elf"

for bin in "$M2_ELF" "$M3_ELF" "$M4_ELF" "$VER_ELF"; do
  if [ ! -x "$bin" ]; then
    echo "[PURE-BUILD FAIL] Missing cached Pure-Omni toolchain binary '$bin'. Run 'bash scripts/zero_rust_rebuild.sh' once to populate the verified native cache." >&2
    exit 1
  fi
done

mkdir -p "$OUT_DIR"
cp "$M2_ELF"  "$OUT_DIR/m2.elf"
cp "$M3_ELF"  "$OUT_DIR/m3.elf"
cp "$M4_ELF"  "$OUT_DIR/m4.elf"
cp "$VER_ELF" "$OUT_DIR/verifier.elf"
chmod +x "$OUT_DIR/m2.elf" "$OUT_DIR/m3.elf" "$OUT_DIR/m4.elf" "$OUT_DIR/verifier.elf"

TMP=$(mktemp -d /tmp/omni_step7_build.XXXXXX)
trap 'rm -rf "$TMP"' EXIT

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

# Build zero-include Pure-Omni build plumbing utility (`pure/m5/pure_build_util.omni`)
# directly via cached Pure-Omni toolchain (m3.elf + verifier.elf + m2.elf).
LC_ALL=C od -An -tu1 -v pure/m5/pure_build_util.omni | LC_ALL=C awk '{for(i=1;i<=NF;i++) printf "%c%c", $i, 0}' > "$TMP/pbu.u16"
"$OUT_DIR/m3.elf" "$TMP/pbu.u16" 1 > "$TMP/pbu.omni_bc"
check_m3_and_verify "$TMP/pbu.omni_bc" "$TMP/pbu.ver" "pure/m5/pure_build_util.omni" >/dev/null
"$OUT_DIR/m2.elf" "$TMP/pbu.omni_bc" > "$TMP/pure_build_util.elf"
chmod +x "$TMP/pure_build_util.elf"

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

build_pure_elf "pure/audit/diff_summary.omni" "audit_diff.elf"
build_pure_elf "pure/audit/decompile.omni"    "audit_decompile.elf"
build_pure_elf "pure/driver/omnic_driver.omni" "omnic_pure.elf"

echo "[PURE-BUILD PASS] Built Pure-Omni driver & audit toolchain in $OUT_DIR with ZERO rustc/cargo/Python."
