# Omni Phase 5 Plan — Full Self-Compilation (Zero Rust in the Loop) & Zero-Shim Syscall Intrinsics

**Status:** Phase 5 (`3e623e1`) completed and verified clean on both Windows and Linux x86-64 (2026-11-20):
- **`omnic p5-selfhost`**: PASS — `pure/m4/m4.omni_bc` (`25,026` words / `50,052` B, `FNV-1a64=0x44FB7C039E69843C`) matches `M3_vm(pure/m4/*)` with `0`-word diff, compiles via `M2_vm(pure/m4/m4.omni_bc)` to `109,740`-B x86-64 ELF64 (`FNV-1a64=0xA7A6CA9D72BBD84C`, `0`-byte diff vs Rust), and passes the zero-Rust M3→M4 & M3→M2 compile+exec loop.
- **`omnic p5-sys`**: 5/5 PASS — `pure/m5/elf_sys.omni` (73 lines, `m2-lint` 0 findings) emits a standalone `177`-byte zero-shim Linux x86-64 ELF64 (`FNV-1a64=0x4C5943AD64761FA1`) with 3 raw `0F 05` syscalls (`sys_read`/`sys_write`/`sys_exit`), verified natively on Linux x86-64 (`native=VERIFIED`).

## Goals

1. **P5-1 — Full Pure-Omni Self-Compilation Chain (Zero Rust in the Loop):**
   - Commit deterministic seed `pure/m4/m4.omni_bc` (the Pure-Omni VM compiled as a `.omni_bc` seed).
   - Prove `M3_vm(pure/m4/*) == RustFE(pure/m4/*) == pure/m4/m4.omni_bc` (0-word diff).
   - Prove `M2_vm(pure/m4/m4.omni_bc) == RustEmitMulti(pure/m4/m4.omni_bc)` (native x86-64 ELF64 of the Pure-Omni VM, **0-byte diff**).
   - Prove end-to-end **Zero-Rust Compile + Exec Loop**: `.omni` → Pure-Omni M3 (`pure/m3/m3.omni_bc`) → `.omni_bc` → executed directly by Pure-Omni M4 VM (`pure/m4/m4.omni_bc`) AND compiled directly to native x86-64 ELF64 by Pure-Omni M2 (`pure/m2/m2.omni_bc`, 0-byte diff vs Rust).

2. **P5-2 — Zero-Shim Linux x86-64 Syscall Intrinsics (`pure/m5/elf_sys.omni`):**
   - Fold the Phase 4 thin shim into the Pure-Omni ELF64 emitter by emitting raw Linux x86-64 `syscall` (`0F 05`) sequences (`sys_read`, `sys_write`, `sys_exit`) plus a minimal `_start` stack-arena entry stub.
   - Zero Rust, zero C, zero shim program — the only layer below the emitted ELF64 is the Linux kernel.
   - Preserves all 128 ISA opcodes and all 5 frozen FNV-1a64 hashes untouched.

## Acceptance Gates

- **`omnic p5-sys`**: Verifies `pure/m5/elf_sys.omni` (registered in `m2-lint`, 0 findings) emits a valid standalone Linux x86-64 ELF64 with exact `PT_LOAD` header, stack-arena prologue, and raw `0F 05` `sys_read`/`sys_write`/`sys_exit` opcodes — and executes it natively on Linux x86-64 (piping bytes `stdin` → `stdout`, exit 0; explicit `SKIPPED on non-Linux-x86-64` on non-Linux hosts).
- **`omnic p5-selfhost [--write-seed]`**: Verifies Stages A–C above (`pure/m4/m4.omni_bc` drift check + `M3_vm(pure/m4/*) == pure/m4/m4.omni_bc` + `M2_vm(pure/m4/m4.omni_bc) == RustEmitMulti` + zero-Rust M3→M4 & M3→M2 loop).
- Global guard: `m2-lint` (15/15), `selftest` non-regressing, 0 warnings on monolith + workspace, all 5 frozen FNV-1a64 hashes intact.
