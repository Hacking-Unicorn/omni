# Archived Duplicate Rust Oracle (`omni-workspace`)

Archived in Audit §6.1 (`omni-full-audit-2026-11-22`).

`omni-workspace/` (8 crates, ~20,575 lines) mirrored `omni_rust_bootstrap.rs` (272 of 299 function names duplicated), requiring every VM and emitter change to be patched twice.

**Single active Rust oracle:** `omni_rust_bootstrap.rs` (standalone `rustc -O omni_rust_bootstrap.rs -o omni`, zero external crates, zero `unsafe`), alongside the honestly counted production default host module `host_runtime/pure_default.rs`, `m2_host/`, `p4_shim/`, and `seed0/`.

This directory is retained read-only as a historical archive and is no longer built or tested in CI.
