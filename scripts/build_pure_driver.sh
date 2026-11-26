#!/usr/bin/env bash
# ==============================================================================
# OMNI STEP 7 — ZERO-RUST PURE-OMNI DRIVER BUILD (`scripts/build_pure_driver.sh`)
# ==============================================================================
# Builds the Pure-Omni native driver (`omnic_pure.elf`) and Pure-Omni audit tools
# (`audit_decompile.elf`, `audit_diff.elf`) into `build/pure-bin/` using ONLY the
# Pure-Omni native toolchain (`m3.elf` + `m2.elf` + `verifier.elf`).
# ZERO `rustc` or `cargo` invocations anywhere in this build.
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

build_pure_elf() {
  local src="$1"
  local name="$2"
  echo "[PURE-BUILD] $src -> $OUT_DIR/$name (via Pure-Omni m3.elf + verifier.elf + m2.elf)"
  # Expand repo-relative `include "..."` recursively (pure text concat, zero Rust) & convert to u16-LE
  python3 - "$src" "$TMP/${name}.u16" <<'PY'
import sys, os, re
seen = set()
def expand(path):
    ap = os.path.normpath(path)
    if ap in seen:
        return ""
    seen.add(ap)
    out = []
    with open(ap, "r", encoding="utf-8") as f:
        for line in f:
            m = re.match(r'^\s*include\s+"([^"]+)"\s*;\s*$', line)
            if m:
                inc = m.group(1)
                cand = inc if os.path.isfile(inc) else os.path.join(os.path.dirname(ap), inc)
                out.append(expand(cand))
                out.append("\n")
            else:
                out.append(line)
    return "".join(out)
src_txt = expand(sys.argv[1]).encode("utf-8")
open(sys.argv[2], "wb").write(b"".join(bytes([b, 0]) for b in src_txt))
PY
  "$OUT_DIR/m3.elf" "$TMP/${name}.u16" 1 > "$TMP/${name}.omni_bc"
  # Fail closed if M3 returned a 16-byte 0xFFFF error sentinel
  python3 - "$TMP/${name}.omni_bc" "$src" <<'PY'
import sys, struct
d = open(sys.argv[1], "rb").read()
if len(d) == 16 and struct.unpack("<H", d[0:2])[0] == 0xFFFF:
    code, line, col = struct.unpack("<HHH", d[2:8])
    sys.stderr.write(f"[PURE-BUILD FAIL] m3.elf error M3_CODE_{code} at {line}:{col} compiling {sys.argv[2]}\n")
    sys.exit(1)
assert len(d) >= 4 and len(d) % 2 == 0, f"invalid .omni_bc len {len(d)}"
print(f"  m3.elf: {len(d)//2} words")
PY
  "$OUT_DIR/verifier.elf" "$TMP/${name}.omni_bc" > "$TMP/${name}.ver"
  python3 - "$TMP/${name}.ver" "$src" <<'PY'
import sys, struct
d = open(sys.argv[1], "rb").read()
errs = struct.unpack("<H", d[0:2])[0]
if errs != 0:
    sys.stderr.write(f"[PURE-BUILD FAIL] verifier.elf rejected {sys.argv[2]} with {errs} error(s)\n")
    sys.exit(1)
print(f"  verifier.elf: PASS (0 errors)")
PY
  "$OUT_DIR/m2.elf" "$TMP/${name}.omni_bc" > "$OUT_DIR/$name"
  chmod +x "$OUT_DIR/$name"
  echo "  m2.elf: $(wc -c < "$OUT_DIR/$name") B -> $OUT_DIR/$name"
}

build_pure_elf "pure/audit/diff_summary.omni" "audit_diff.elf"
build_pure_elf "pure/audit/decompile.omni"    "audit_decompile.elf"
build_pure_elf "pure/driver/omnic_driver.omni" "omnic_pure.elf"

echo "[PURE-BUILD PASS] Built Pure-Omni driver & audit toolchain in $OUT_DIR with ZERO rustc/cargo."
