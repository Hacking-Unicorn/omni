# Omni

Omni is a 16-bit token-stream ISA (128 opcodes, `0x7F00..=0x7F7F`) with a capability / linear-ownership / taint static verifier, a deterministic VM, `@PACK5` balanced-ternary packing, a `.omni` surface language, and a self-hosted native `x86-64` ELF64 backend.

**Status, in one line:** Omni is **100% Pure-Omni with zero Rust in the repository tree** (`0` `.rs` files, `0` `Cargo.toml` / `Cargo.lock` files). The x86-64 ELF64 backend (M2), surface `.omni` → `.omni_bc` frontend (M3), multi-module ELF64 linker (`pure/link/elf_link.omni`), deterministic token-stream VM (M4), static verifier (`pure/verifier_engine.omni`), driver (`pure/driver/omnic_driver.omni`), audit decompiler/diff (`pure/audit/`), and test/proposer harness (`pure/harness/`) all compile and run natively on Linux x86-64 via raw `0F 05` syscalls, binary-seeded from `bootstrap/m2.elf` and `bootstrap/m3.elf` with zero `rustc`, zero `cargo`, and zero `python3` in the build path.

## Binary-Seeded Bootstrap Manifest (`bootstrap/MANIFEST.md`)

All committed bootstrap artifacts and their pinned byte sizes and FNV-1a64 hashes (see [`bootstrap/MANIFEST.md`](bootstrap/MANIFEST.md)):

### Committed Bootstrap Binaries (`bootstrap/`)

| Artifact | Size (B) | FNV-1a64 | Role |
|---|---:|---|---|
| `bootstrap/m2.elf` | `125,660` | `0x3A4A285C58CFA93E` | Pure-Omni M2 x86-64 ELF64 compiler (`S1 == S2 == S3 == bootstrap/m2.elf`, incl. `sys_clock_ns`) |
| `bootstrap/m3.elf` | `207,015` | `0x7397A5020895AEBF` | Pure-Omni M3 surface `.omni` → `.omni_bc` frontend (rebuilt bit-exact via `link-elf`) |

### Committed `.omni_bc` Bytecode Seeds (`pure/`)

| Seed Artifact | Size (B) | Words | FNV-1a64 | Rebuilt Artifact |
|---|---:|---:|---|---|
| `pure/m2/m2.omni_bc` | `87,686` | `43,843` | `0x3DCC8546CE5B0D57` | `m2.elf` (`125,660` B, `0x3A4A285C58CFA93E`, `S1 == S2 == S3 == bootstrap/m2.elf`) |
| `pure/m3/m3.omni_bc` | `132,498` | `66,249` | `0x0E3052A4BE00D599` | Monolithic M3 frontend fixed-point reference |
| `pure/m3/m3_standalone.omni_bc` | `52,498` | `26,249` | `0xD83FBCA0D2E8FA78` | Modular M3 part 1 (linked into `m3.elf`) |
| `pure/m3/mod_parse.omni_bc` | `84,530` | `42,265` | `0x0F581C62270F2863` | Modular M3 part 2 (linked into `m3.elf`) |
| `pure/link/elf_link.omni_bc` | `7,164` | `3,582` | `0x5F261D791F65ED2A` | Pure-Omni ELF64 linker (`elf_link.elf`, `10,971` B; rebuilt bit-exact from `pure/link/elf_link.omni`) |
| `pure/m4/m4.omni_bc` | `56,124` | `28,062` | `0x0561704B3E50EFAC` | Pure-Omni M4 VM (`90,402` B ELF64, `0xB43AFBB0DED59488`) |
| `pure/m4/m4_standalone.omni_bc` | `57,330` | `28,665` | `0x6F86B759214A2765` | Standalone native `m4.elf` (`91,576` B, `0xFBA8272337B1BAC4`) |
| `pure/verifier_standalone.omni_bc` | `47,268` | `23,634` | `0x7E82A2FFC55AEE56` | Standalone native `verifier.elf` (`77,189` B, `0xDB33F13E1082948A`) |

> **Judge / Pin Discipline:** After Step 9, the frozen regression hashes and `bootstrap/MANIFEST.md` are the sole cryptographic anchors. Every hash/pin change is a judge change: its own PR with from-scratch derivation and fixed-point proof, never auto-merged and never proposer-initiated.

## Exact From-Scratch Clean-Room Build & Verification (Linux x86-64)

```bash
# 1. Clean-room zero-Rust, zero-Python rebuild + fixed-point & harness proof
rm -rf build/pure-bin
PATH=/usr/bin:/bin bash scripts/zero_rust_rebuild.sh

# 2. Full Pure-Omni driver and harness verification suites
PATH=/usr/bin:/bin bash scripts/verify_step7_driver.sh
PATH=/usr/bin:/bin bash scripts/verify_step8_harness.sh

# 3. Independent external verifier trace check (Python 3, zero Omni/Rust)
python3 scripts/external_verifier_check.py
```

### Using `build/pure-bin/omnic_pure.elf` directly

```bash
./build/pure-bin/omnic_pure.elf --version
./build/pure-bin/omnic_pure.elf compile <src.omni> <out.omni_bc>
./build/pure-bin/omnic_pure.elf compile-elf <src.omni|in.omni_bc> <out.elf> [args...]
./build/pure-bin/omnic_pure.elf link-elf <a.omni_bc> <b.omni_bc> -o <out.elf>
./build/pure-bin/omnic_pure.elf run <src.omni|in.omni_bc> [args...]
./build/pure-bin/omnic_pure.elf decompile <src.omni|in.omni_bc>
./build/pure-bin/omnic_pure.elf inspect --m3 <src.omni|in.omni_bc>
./build/pure-bin/omnic_pure.elf audit-diff <before.omni_bc> <after.omni_bc> [--tsv]
./build/pure-bin/omnic_pure.elf selftest
./build/pure-bin/omnic_pure.elf m2-lint <file.omni>
./build/pure-bin/omnic_pure.elf audit-view [--inject-broken-decompile]
./build/pure-bin/omnic_pure.elf proposer-loop
./build/pure-bin/omnic_pure.elf m2-frozen | m2-fixed-point | b2-pure-verifier | p2a-syscalls
./build/pure-bin/omnic_pure.elf m3-modular | a3-macro-quarantine | b4-pure-launcher | p5-sys | s5-refreeze
```

## Design Constraints

- **Zero external dependencies, zero Rust, zero Python in the build/run toolchain.**
- **192 SSA registers and 16 labels (`0..=15`) per function** (enforced by both surface compiler and verifier); **64-deep call stack** (enforced by the VM and `m2-lint`).
- **Eager argument evaluation.** No heap structs, hash maps, or GC in core `.omni`; data lives in flat linear byte arenas.
- **Pure verifier report limit: ~2,388 functions per module.** `pure/verifier_engine.omni` writes its per-function report into a fixed 7,168-word region at word 1024 of its 8,192-word workspace (`PURE_VERIFY_REPORT_OVERFLOW` fail-closed).
- **No recursion in core `.omni` modules** (iterative state machines + explicit worklists, enforced by `m2-lint`).
- **The 128-opcode table (`0x7F00..=0x7F7F`) is never renumbered.**

## Frozen Regression Hashes

Asserted by `omnic_pure.elf m2-frozen` and `omnic_pure.elf s5-refreeze`:

| Artifact | Size | FNV-1a64 | What it is |
|---|---:|---|---|
| Phase D kernel `.omni_bc` | 159 words | `0x23B8D17257CCA9C4` | 3-input decision kernel |
| Phase D kernel `.elf` | 880 B | `0x8D5AAAD22399A1EE` | Phase D kernel ELF64 (`PT_GNU_STACK`) |
| 18-module core suite | 14,559 words | `0x78462AE4C81DF408` | 7 `std/*` + 5 legacy `pure/*` + 6 `pure/programs/*` |
| Stage-1 bytecode | 20,443 words (40,886 B) | `0xED4C50AA79563EFF` | Stage-1 self-compiler token stream |
| Stage-1 ELF | 112,180 B | `0x62F14B335B52BECC` | Bit-exact Stage-1 == Stage-2 fixed-point ELF64 |

## Repository Layout

- `bootstrap/` — Committed Pure-Omni bootstrap binaries (`m2.elf`, `m3.elf`) and `MANIFEST.md`.
- `pure/` — Pure-Omni core modules (`pure/driver/`, `pure/audit/`, `pure/harness/`, `pure/verifier_engine.omni`, `pure/isa/`, `pure/link/`, `pure/m2/`, `pure/m3/`, `pure/m4/`, `pure/m5/`, `pure/m7/`, `pure/api/`, `pure/host_kv.omni`, `pure/programs/`).
- `std/` — Pure-Omni standard library (`std/prelude.omni`, `std/math.omni`, `std/trit.omni`, `std/http.omni`, `std/pack5.omni`, `std/decision.omni`, `std/resource.omni`).
- `scripts/` — Zero-Rust build & verification scripts (`build_pure_driver.sh`, `zero_rust_rebuild.sh`, `verify_step7_driver.sh`, `verify_step8_harness.sh`, `external_verifier_check.py`).
- `spec/` — Executable specs, acceptance goldens, lint fixtures, verifier traces, and [`spec/harness-accounting.md`](spec/harness-accounting.md).
- `spec.md` — ISA and language specification.
- `LICENSE` — MIT.

## Recovery Path (Historical Rust Bootstrap)

The historical Rust bootstrap (`omni_rust_bootstrap.rs`, `seed0/omni_seed0.rs`, `m2_host/`, `host_runtime/`, `p4_shim/`, `omni_dev_host.rs`, and `oracle/archived-workspace/`) was deleted in Step 9 and remains intact in git history. The last commit containing the Rust tree is `88ec669` (PR #48). Restoring it is a git history operation, not a tree operation:

```bash
git checkout 88ec669 -- omni_rust_bootstrap.rs seed0 m2_host host_runtime p4_shim omni_dev_host.rs
```
