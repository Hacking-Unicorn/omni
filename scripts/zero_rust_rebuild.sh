#!/usr/bin/env bash
# ==============================================================================
# OMNI SEED-SHRINK (PHASE C1) + PHASE 2a — LITERAL ZERO-RUST RELEASE GATE
# `scripts/zero_rust_rebuild.sh`
# ==============================================================================
# Two-phase rebuild proof:
#   Phase 1 (Stage-0 host bootstrap): build `./omni`, `./omni-seed0`, and
#     `./omni-dev-host` once with `rustc` via `bash scripts/bootstrap_stage0.sh`.
#   Phase 2 (this script): strip `rustc` and `cargo` from PATH and prove that:
#     1. All committed Pure-Omni seeds (`m3.omni_bc`, `m2.omni_bc`, `m4.omni_bc`)
#        verify bit-exact and pass every Pure-Omni self-host / fixed-point / P7 gate.
#     2. Phase 2a Literal Zero-Rust Native Toolchain Chain runs with ZERO Rust
#        processes in the loop: native Pure-Omni `m2.elf` rebuilds itself bit-exact
#        (`m2.elf == m2_gen2.elf`), rebuilds `m4.elf`, `verifier.elf`, `mod_parse.elf`,
#        `m3_standalone.elf`, and `elf_link.elf`; then native `m3.elf` + `verifier.elf`
#        + `m4.elf` + `m2.elf` compile, verify, and execute both `weighted_diff_pure_golden.omni`
#        and `pure/m5/zero_rust_driver.omni` (raw `0F 05` syscalls: `sys_open`/`sys_write`/
#        `sys_read`/`sys_close`/`sys_spawn`/`sys_exit` + OOB trap) directly on Linux x86-64.
# ==============================================================================
set -euo pipefail

export PATH="/usr/bin:/bin"

if command -v rustc >/dev/null 2>&1 || command -v cargo >/dev/null 2>&1; then
  echo "[C1 FAIL] rustc or cargo is still reachable on PATH" >&2
  exit 1
fi

for bin in ./omni ./omni-seed0 ./omni-dev-host; do
  if [ ! -x "$bin" ]; then
    echo "[C1 PREREQ FAIL] Missing Stage-0 host binary '$bin'. Run 'bash scripts/bootstrap_stage0.sh' once (with rustc) before running 'scripts/zero_rust_rebuild.sh'." >&2
    exit 1
  fi
done

echo "[C1] Verified rustc/cargo absent from PATH ($PATH)"

./omni-seed0 verify-seeds
./omni-seed0 line-audit
./omni-dev-host --version

./omni a1-pure-default
./omni a2-pure-include
./omni a3-macro-quarantine
./omni b2-pure-verifier
./omni b4-pure-launcher
./omni self-host
./omni p5-sys
./omni m2-fixed-point
./omni m3-fixed-point
./omni m3-modular
./omni p2a-syscalls

./omni compile spec/acceptance/goldens/weighted_diff_pure_golden.omni /tmp/c1_wd.omni_bc
OUT=$(./omni run /tmp/c1_wd.omni_bc 45 10 40)
if [ "$OUT" != "Int(100)" ]; then
  echo "[C1 FAIL] pure-default compile+run expected Int(100), got $OUT" >&2
  exit 1
fi
rm -f /tmp/c1_wd.omni_bc

./omni p7-measure
./omni p7-canary

# ==============================================================================
# PHASE 2a: LITERAL ZERO-RUST NATIVE TOOLCHAIN PROOF (ZERO RUST PROCESSES)
# ==============================================================================
echo "[P2A] Starting literal zero-Rust native ELF toolchain chain (zero Rust processes)..."
CACHE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/omni/native-v1"
M2_ELF="$CACHE_DIR/m2_0a5cd5b55707aa0d.elf"
M3_ELF="$CACHE_DIR/m3_8cfd973873caff46.elf"
if [ ! -x "$M2_ELF" ] || [ ! -x "$M3_ELF" ]; then
  echo "[P2A FAIL] expected cached native m2.elf / m3.elf under $CACHE_DIR" >&2
  exit 1
fi

P2A_TMP=$(mktemp -d /tmp/omni_p2a_gate.XXXXXX)
trap 'rm -rf "$P2A_TMP" /tmp/omni_p2a_io.bin /tmp/omni_p2a_child.elf' EXIT

# 1. Native m2.elf rebuilds itself bit-exact + rebuilds m4.elf, verifier.elf, elf_link.elf, modular M3 objects.
"$M2_ELF" pure/m2/m2.omni_bc > "$P2A_TMP/m2_gen2.elf"
cmp "$M2_ELF" "$P2A_TMP/m2_gen2.elf"
chmod +x "$P2A_TMP/m2_gen2.elf"

"$P2A_TMP/m2_gen2.elf" pure/m4/m4_standalone.omni_bc > "$P2A_TMP/m4_gen2.elf"
chmod +x "$P2A_TMP/m4_gen2.elf"

"$P2A_TMP/m2_gen2.elf" pure/verifier_standalone.omni_bc > "$P2A_TMP/verifier_gen2.elf"
chmod +x "$P2A_TMP/verifier_gen2.elf"

"$P2A_TMP/m2_gen2.elf" pure/link/elf_link.omni_bc > "$P2A_TMP/elf_link_gen2.elf"
"$P2A_TMP/m2_gen2.elf" pure/m3/mod_parse.omni_bc > "$P2A_TMP/mod_parse_gen2.elf"
"$P2A_TMP/m2_gen2.elf" pure/m3/m3_standalone.omni_bc > "$P2A_TMP/m3_standalone_gen2.elf"

# 2. Build zero-include Pure-Omni build utility (`pure/m5/pure_build_util.omni`) via gen2 toolchain
#    and verify exact byte sizes & FNV-1a64 pins of zero-Rust rebuilt native toolchain ELFs (ZERO Python!).
LC_ALL=C od -An -tu1 -v pure/m5/pure_build_util.omni | LC_ALL=C awk '{for(i=1;i<=NF;i++) printf "%c%c", $i, 0}' > "$P2A_TMP/pbu.u16"
"$M3_ELF" "$P2A_TMP/pbu.u16" 1 > "$P2A_TMP/pbu.omni_bc"
PBU_VER_ERRS=$("$P2A_TMP/verifier_gen2.elf" "$P2A_TMP/pbu.omni_bc" | od -An -tu2 -N 2 | tr -d ' \n')
if [ "${PBU_VER_ERRS:-1}" -ne 0 ]; then
  echo "[P2A FAIL] verifier_gen2.elf rejected pure/m5/pure_build_util.omni with ${PBU_VER_ERRS} error(s)" >&2
  exit 1
fi
"$P2A_TMP/m2_gen2.elf" "$P2A_TMP/pbu.omni_bc" > "$P2A_TMP/pure_build_util.elf"
chmod +x "$P2A_TMP/pure_build_util.elf"

for spec in \
  "m2_gen2.elf:125539:0x0A5CD5B55707AA0D" \
  "m4_gen2.elf:91576:0xFBA8272337B1BAC4" \
  "verifier_gen2.elf:37907:0x2ECD2CE3DCB69346"
do
  IFS=':' read -r name want_len want_fnv <<< "$spec"
  read -r got_len got_fnv < <("$P2A_TMP/pure_build_util.elf" fnv "$P2A_TMP/$name")
  if [ "$got_len" != "$want_len" ] || [ "$got_fnv" != "$want_fnv" ]; then
    echo "[P2A FAIL] ${name}: got ${got_len} B / ${got_fnv}, want ${want_len} B / ${want_fnv}" >&2
    exit 1
  fi
  echo "[P2A] zero-Rust rebuilt ${name}: ${got_len} B (${got_fnv}) OK"
done

# 3. Compile + verify + VM-run + native-compile + native-run weighted_diff_pure_golden.omni (ZERO Rust, ZERO Python!).
"$P2A_TMP/pure_build_util.elf" widen spec/acceptance/goldens/weighted_diff_pure_golden.omni "$P2A_TMP/wd.u16"
"$P2A_TMP/pure_build_util.elf" widen pure/m5/zero_rust_driver.omni "$P2A_TMP/drv.u16"

"$M3_ELF" "$P2A_TMP/wd.u16" 1 > "$P2A_TMP/wd.omni_bc"
"$P2A_TMP/verifier_gen2.elf" "$P2A_TMP/wd.omni_bc" > "$P2A_TMP/wd_ver.bin"
"$P2A_TMP/m4_gen2.elf" "$P2A_TMP/wd.omni_bc" 45 10 40 > "$P2A_TMP/wd_vm.bin"
"$P2A_TMP/m2_gen2.elf" "$P2A_TMP/wd.omni_bc" > /tmp/omni_p2a_child.elf
chmod +x /tmp/omni_p2a_child.elf
WD_NAT=$(/tmp/omni_p2a_child.elf 45 10 40 | od -An -td8 | tr -d ' \n')
if [ "$WD_NAT" != "100" ]; then
  echo "[P2A FAIL] zero-Rust native wd.elf(45,10,40) expected 100, got $WD_NAT" >&2
  exit 1
fi

# 4. Compile + verify + run Pure-Omni syscall driver (`pure/m5/zero_rust_driver.omni`) directly on Linux kernel!
"$M3_ELF" "$P2A_TMP/drv.u16" 1 > "$P2A_TMP/drv.omni_bc"
"$P2A_TMP/verifier_gen2.elf" "$P2A_TMP/drv.omni_bc" > "$P2A_TMP/drv_ver.bin"
"$P2A_TMP/m2_gen2.elf" "$P2A_TMP/drv.omni_bc" > "$P2A_TMP/drv.elf"
chmod +x "$P2A_TMP/drv.elf"

DRV0=$("$P2A_TMP/drv.elf" 0 | tail -c 8 | od -An -td8 | tr -d ' \n')
DRV2=$("$P2A_TMP/drv.elf" 2 | tail -c 8 | od -An -td8 | tr -d ' \n')
set +e
"$P2A_TMP/drv.elf" 1 >/dev/null 2>&1; RC1=$?
"$P2A_TMP/drv.elf" 3 >/dev/null 2>&1; RC3=$?
set -e
if [ "$DRV0" != "1016" ] || [ "$DRV2" != "2016" ] || [ "$RC1" != "1" ] || [ "$RC3" != "42" ]; then
  echo "[P2A FAIL] zero-Rust syscall driver mismatch: mode0=$DRV0 mode2=$DRV2 rc1=$RC1 rc3=$RC3" >&2
  exit 1
fi

echo "[C1 + P2A ZERO-RUST RELEASE GATE PASS] All seeds, M5 launcher, native M2/M3/M4/verifier zero-Rust rebuild, P2a kernel syscall driver, and P7 gates passed with rustc/cargo off PATH."
