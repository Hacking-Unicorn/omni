# Omni Phase 4 Plan — AI-Native Pure-Omni Runtime

Adopted from Caleb & Lilith's Phase 4 direction (2026-11-20), refined with three
mission-aligned safeguards:

1. **Verify-before-execute inside Pure-Omni:** Raw `.omni_bc` token streams are
   decoded via `pure/m2/decode.omni` and checked via `pure/m2/verify/mod_check.omni`
   before dispatch so AI-emitted token streams can never bypass safety invariants.
2. **Zero-allocation binary trace ring (P4-5 from Day 1):** Fixed 64 KiB arena ring
   (`0x70000..0x7FFFF`, 4,096 x 16-B records `[step, fn_idx, op_tok, pc_off, res_lo32]`)
   gated by `trace_mode` (`0` = zero overhead, `1` = record).
3. **Lint-clean modular split under `pure/m4/`:** Every module obeys M2 lint budgets
   (`<= 120` SSA, `<= 12` labels, no recursion, `<= 1` ANEW, strict linear bytes) so
   the Pure-Omni VM compiles cleanly on both `OmniBinaryVm` and M2-10 Wasm-multi.

## Deliverables & Gates

| Gate | Deliverable | Acceptance |
|---|---|---|
| **P4-1** | `pure/m4/arena.omni`, `pure/m4/ops_core.omni`, `pure/m4/vm.omni` | Pure-Omni VM == Rust `OmniBinaryVm` across arithmetic, ternary, control-flow, loops, byte buffers (ISA-B `ld_u*/st_u*`), multi-fn `CALL`, and budgets (`omnic p4-vm`) |
| **P4-2** | Raw `.omni_bc` token-stream entry point (`omnic p4-exec <file.omni_bc> [args...]`) | Executes raw `.omni_bc` directly with zero source text or parser involvement; matches frontend execution bit-for-bit |
| **P4-3** | Minimal 3-op thin host shim (`read_bytes`, `write_bytes`, `spawn`) | Proven self-contained with zero HTTP/shell/file-stack dependencies |
| **P4-4** | Pure-Omni VM running on M2-10 Wasm-multi host (`omnic p4-wasm`) | Same results on Wasm host as Rust VM & native |
| **P4-5** | Binary execution trace ring (`0x70000..0x7FFFF`) | Deterministic per-step binary trace emitted when `trace_mode=1`, zero overhead when `trace_mode=0` |

## Deliberately Dropped (Not Runtime)
- `omni serve` / live HTTP host, `repair_cmd` / shell execution, runtime source parsing,
  and `emit_asm` stay Rust-side dev tooling only.
- Raw syscall intrinsics + minimal assembly entry stub (replacing the thin shim) are
  completed in Phase 5 (`pure/m5/elf_sys.omni`, `omnic p5-sys`).

## Status (Verified 2026-11-20 on Windows & Linux x86-64, `6e416d4`)
- **P4-1 (`omnic p4-vm`)**: 11/11 PASS (`pure/m4/arena.omni` 147 lines + `pure/m4/ops_core.omni` 277 lines + `pure/m4/vm.omni` 436 lines = 860 lines, `m2-lint` 0 gated findings).
- **P4-2 (`omnic p4-exec`)**: PASS (raw `.omni_bc` execution with `--trace`).
- **P4-3 (`omnic p4-shim`)**: 3/3 PASS (`p4_shim/mod.rs`: `read_bytes`, `write_bytes`, `spawn`).
- **P4-4 (`omnic p4-wasm`)**: 3/3 PASS (full `pure/m4/vm.omni` compiles to `62,213`-B Wasm v1 `0x557AA8121383D047` + VM core executes on M2-10 Wasm-multi host).
- **P4-5**: 64 KiB binary trace ring (`0x70000..0x7FFFF`, 4,096 × 16-B records) verified inside `p4-vm`, `p4-shim`, and `p4-exec --trace`.
