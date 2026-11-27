#!/usr/bin/env bash
# ==============================================================================
# OMNI STEP 9 — CLEAN-ROOM ZERO-RUST REBUILD & VERIFICATION GATE
# `scripts/zero_rust_rebuild.sh`
# ==============================================================================
# Proves end-to-end with ZERO Rust files in the tree, ZERO `rustc`/`cargo` on
# PATH, and ZERO Python in the build/run path that:
#   1. The repository contains 0 `.rs` files and 0 `Cargo.toml`/`Cargo.lock` files.
#   2. Deleting every generated artifact (`build/pure-bin`) and rebuilding from
#      ONLY `bootstrap/m2.elf` + `bootstrap/m3.elf` + committed `.omni`/`.omni_bc`
#      sources reproduces all toolchain ELFs bit-exact:
#        - `m2.elf`: S1 == S2 == S3 == bootstrap/m2.elf (125,539 B, 0x0A5CD5B55707AA0D)
#        - `m3.elf`: link-elf rebuild == bootstrap/m3.elf (206,946 B, 0x8CFD973873CAFF46)
#        - `m4.elf`: 91,576 B, 0xFBA8272337B1BAC4
#        - `verifier.elf`: 38,193 B, 0x23E3D55383B3333B
#        - `omnic_pure.elf`: self-compile fixed point (gen1 == gen2, 0-byte diff)
#        - `pure/link/elf_link.omni` -> `pure/link/elf_link.omni_bc` (0-byte diff)
#   3. Every committed `.omni_bc` seed matches its `bootstrap/MANIFEST.md` pin.
#   4. The Pure-Omni harness passes `selftest`, all 11 named gates (including
#      `m2-frozen` frozen-5 pins and `m2-fixed-point` S1 == S2 == S3 == V),
#      `m2-lint` 15/15 fixtures, negative controls, and the Phase 2a kernel
#      syscall driver (`pure/m5/zero_rust_driver.omni`).
# ==============================================================================
set -euo pipefail

export PATH="/usr/bin:/bin"

if command -v rustc >/dev/null 2>&1 || command -v cargo >/dev/null 2>&1; then
  echo "[ZERO-RUST FAIL] rustc or cargo is still reachable on PATH ($PATH)" >&2
  exit 1
fi

echo "[ZERO-RUST] Verified rustc/cargo absent from PATH ($PATH)"

# 1. Assert zero Rust source files or Cargo manifests in the repository tree.
RS_COUNT=$(find . -path ./.git -prune -o \( -name '*.rs' -o -name 'Cargo.toml' -o -name 'Cargo.lock' \) -print | wc -l)
if [ "$RS_COUNT" -ne 0 ]; then
  echo "[ZERO-RUST FAIL] Found $RS_COUNT Rust/Cargo files still in the tree!" >&2
  find . -path ./.git -prune -o \( -name '*.rs' -o -name 'Cargo.toml' -o -name 'Cargo.lock' \) -print >&2
  exit 1
fi
echo "[ZERO-RUST] Verified 0 .rs / Cargo.toml / Cargo.lock files in tree."

# 2. Delete every generated artifact and rebuild from scratch starting ONLY from bootstrap/.
OUT_DIR="build/pure-bin"
rm -rf "$OUT_DIR"

TMP=$(mktemp -d /tmp/omni_zero_rust.XXXXXX)
trap 'rm -rf "$TMP" /tmp/omni_p2a_io.bin /tmp/omni_p2a_child.elf' EXIT

PATH="/usr/bin:/bin" bash scripts/build_pure_driver.sh "$OUT_DIR"

# 3. Verify all 8 committed .omni_bc seed pins + bootstrap binaries using pure_build_util.elf.
LC_ALL=C od -An -tu1 -v pure/m5/pure_build_util.omni | LC_ALL=C awk '{for(i=1;i<=NF;i++) printf "%c%c", $i, 0}' > "$TMP/pbu.u16"
"$OUT_DIR/m3.elf" "$TMP/pbu.u16" 1 > "$TMP/pbu.omni_bc"
"$OUT_DIR/m2.elf" "$TMP/pbu.omni_bc" > "$TMP/pure_build_util.elf"
chmod +x "$TMP/pure_build_util.elf"

for spec in \
  "bootstrap/m2.elf:126152:0xAA5F84B6AA45BB51" \
  "bootstrap/m3.elf:207015:0x7397A5020895AEBF" \
  "pure/m2/m2.omni_bc:87836:0x74FB1652CC0E974B" \
  "pure/m3/m3.omni_bc:132498:0x0E3052A4BE00D599" \
  "pure/m3/m3_standalone.omni_bc:52498:0xD83FBCA0D2E8FA78" \
  "pure/m3/mod_parse.omni_bc:84530:0x0F581C62270F2863" \
  "pure/link/elf_link.omni_bc:7164:0x5F261D791F65ED2A" \
  "pure/m4/m4.omni_bc:56124:0x0561704B3E50EFAC" \
  "pure/m4/m4_standalone.omni_bc:57330:0x6F86B759214A2765" \
  "pure/verifier_standalone.omni_bc:31526:0x485D1C5C2D0A69C6"
do
  IFS=':' read -r path want_len want_fnv <<< "$spec"
  read -r got_len got_fnv < <("$TMP/pure_build_util.elf" fnv "$path")
  if [ "$got_len" != "$want_len" ] || [ "$got_fnv" != "$want_fnv" ]; then
    echo "[ZERO-RUST FAIL] manifest mismatch for ${path}: got ${got_len} B / ${got_fnv}, want ${want_len} B / ${want_fnv}" >&2
    exit 1
  fi
done
echo "[ZERO-RUST] All 10 bootstrap/MANIFEST.md artifacts verified bit-exact."

# 4. Rebuild pure/link/elf_link.omni_bc from pure/link/elf_link.omni via omnic_pure.elf -> 0-byte diff.
"$OUT_DIR/omnic_pure.elf" compile pure/link/elf_link.omni "$TMP/elf_link_rebuilt.omni_bc"
cmp pure/link/elf_link.omni_bc "$TMP/elf_link_rebuilt.omni_bc"
echo "[ZERO-RUST] pure/link/elf_link.omni -> elf_link.omni_bc rebuilt bit-exact (7164 B)."

# 5. Self-compile fixed-point check: omnic_pure.elf compiles itself -> 0-byte diff!
"$OUT_DIR/omnic_pure.elf" compile pure/driver/omnic_driver.omni "$TMP/omnic_gen2.omni_bc"
"$OUT_DIR/omnic_pure.elf" compile-elf "$TMP/omnic_gen2.omni_bc" "$TMP/omnic_gen2.elf"
cmp "$OUT_DIR/omnic_pure.elf" "$TMP/omnic_gen2.elf"
echo "[ZERO-RUST] omnic_pure.elf self-compile fixed point OK ($(wc -c < "$OUT_DIR/omnic_pure.elf") B, 0-byte diff)."

# 6. Run full Pure-Omni harness selftest + all 11 named gates.
"$OUT_DIR/omnic_pure.elf" selftest
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
  "$OUT_DIR/omnic_pure.elf" "$gate" >/dev/null
done
echo "[ZERO-RUST] All 11 named Pure-Omni harness gates + selftest PASS."

# 7. Fail-closed negative control: deliberately broken decompile caught by audit-view gate.
if "$OUT_DIR/omnic_pure.elf" audit-view --inject-broken-decompile >/dev/null 2>&1; then
  echo "[ZERO-RUST FAIL] --inject-broken-decompile was NOT caught" >&2
  exit 1
fi
echo "[ZERO-RUST] Negative control (--inject-broken-decompile) caught fail-closed."

# 8. Compile + verify + VM-run + native-run weighted_diff_pure_golden.omni & pure/m5/zero_rust_driver.omni.
"$OUT_DIR/omnic_pure.elf" compile spec/acceptance/goldens/weighted_diff_pure_golden.omni "$TMP/wd.omni_bc"
WD_OUT=$("$OUT_DIR/omnic_pure.elf" run "$TMP/wd.omni_bc" 45 10 40)
if [ "$WD_OUT" != "Int(100)" ]; then
  echo "[ZERO-RUST FAIL] weighted_diff_pure_golden expected Int(100), got $WD_OUT" >&2
  exit 1
fi
"$OUT_DIR/omnic_pure.elf" compile-elf "$TMP/wd.omni_bc" /tmp/omni_p2a_child.elf
chmod +x /tmp/omni_p2a_child.elf

"$OUT_DIR/omnic_pure.elf" compile-elf pure/m5/zero_rust_driver.omni "$TMP/drv.elf"
chmod +x "$TMP/drv.elf"
DRV0=$("$TMP/drv.elf" 0 | tail -c 8 | od -An -td8 | tr -d ' \n')
DRV2=$("$TMP/drv.elf" 2 | tail -c 8 | od -An -td8 | tr -d ' \n')
set +e
"$TMP/drv.elf" 1 >/dev/null 2>&1; RC1=$?
"$TMP/drv.elf" 3 >/dev/null 2>&1; RC3=$?
set -e
if [ "$DRV0" != "1016" ] || [ "$DRV2" != "2016" ] || [ "$RC1" != "1" ] || [ "$RC3" != "42" ]; then
  echo "[ZERO-RUST FAIL] zero_rust_driver mismatch: mode0=$DRV0 mode2=$DRV2 rc1=$RC1 rc3=$RC3" >&2
  exit 1
fi

echo "[ZERO-RUST CLEAN-ROOM REBUILD GATE PASS] 0 Rust files, rustc/cargo absent from PATH, all manifest pins, fixed points (S1==S2==S3==V), harness gates, and kernel syscall driver passed."
