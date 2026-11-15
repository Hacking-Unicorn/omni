# How much Rust is left: live tracker

**Last updated SHA:** `74a215c03b1758962d86b98d5e3c10e5cf705bbc` (tip of `isa/multibyte-loads`). Baseline: `8160842`.
**Machine-readable copy:** `rust_lines_left.csv` (same rows; the extra `monolith-detail` rows split the monolith by section).
**Author:** Scarlett Truth. This is a count only. No code was changed.

## What this page is (plain words)

Omni is meant to end up written in Omni. Today most of the real work is still done by Rust. This page counts how many lines of Rust are left, groups them by what they do, and records how far each group has got on three steps:

1. **Ported:** an Omni version exists that does the same job. A tiny stand-in "kernel" doesn't count.
2. **Parity-checked (4 backends):** the Omni version gives the same answer as the Rust version, checked on all four ways of running code. Those are the VM, x86 native, Wasm, and the self-hosted native compiler (M2).
3. **Rust deleted:** the Rust lines are removed.

When a step is done, update the row and the "Last updated SHA" line.

## The headline number

**35,112 lines of Rust at `8160842`.** That is 18,379 in the single big file `omni_rust_bootstrap.rs` (the "monolith") plus 16,733 in the 20 files of the `omni-workspace` crates. The two are kept as mirrors of each other.

85,700 is the old pre-cleanup count on `main` at `b3d42dd`, and it includes duplicate copies of the same files. It is not a measure of Rust left to port.

At the current tip `74a215c`, the count is **35,591** (+479 lines, mostly the verifier lockstep shadow mode, mirrored in both the monolith and the workspace).

Exact commands (run inside a clone of the repo):

```
# total Rust lines at a commit (8160842 -> 35112; 74a215c -> 35591)
git ls-tree -r --name-only 8160842 | grep '\.rs$' | while read f; do git show "8160842:$f"; done | wc -l

# per file
git ls-tree -r --name-only 8160842 | grep '\.rs$' | while read f; do echo "$(git show 8160842:$f | wc -l) $f"; done
```

For a checked-out tree, `git ls-files '*.rs' | xargs wc -l` gives the same total.

## Per-subsystem tracker

Line counts are exact. Status words are mine and are checked against the code at `74a215c`.

| Subsystem | Files | Rust lines @8160842 | @74a215c | Omni counterpart | Status today | Stays in Stage-0 seed? | Ported | Parity-checked (4 backends) | Rust deleted |
|---|---|---:|---:|---|---|---|---|---|---|
| Frontend / compiler (lexer, parser, lowering) | `omni-lang/src/compiler/mod.rs` | 3,901 | 3,903 | `pure/m3/` (planned, see `milestone3_frontend_design.md`). `pure/compiler.omni` is a 7-function scalar kernel, not a frontend. | Not started | No. A committed M3 seed image replaces it. | No | No | No |
| Pure-Omni kernels / bootstrap phases | `omni-lang/src/pure_omni/mod.rs` | 2,869 | 2,872 | `pure/*.omni`, `std/*.omni` (small integer kernels) | Rust harness around toy kernels | No | n/a | n/a | No |
| Verifier | `omni-lang/src/verifier/mod.rs`, `omni-verifier/src/lib.rs` | 1,037 | 1,279 | `pure/verifier_engine.omni` (58 fns, 1,034 lines) | **Ported.** 77-case differential. At `74a215c`, a lockstep shadow check runs on every compile, run and serve (`verifier/mod.rs:680-703`). | No | **Yes** | Partial: the engine only runs on the VM (`verifier/mod.rs:474-500`) | No |
| Assembler / ISA table / IR decoder | `omni-assembler`, `omni-isa`, `omni-ir` | 1,088 | 1,088 | `pure/m2/decode.omni` (M2-04, planned); `pure/m3/isa.omni` + `pure/m3/packet.omni` (M3-03/04, planned) | Not started | ISA table and decoder: yes (the VM needs them). Assembler: no. | No | No | No |
| VM | `omni-vm`, `omni-lang/src/ternary` | 712 | 712 | None planned. The VM *is* the seed. | n/a | **Yes** | n/a | n/a | No |
| AOT x86-64 / Wasm (incl. Stage-1 builder) | `omni-aot` | 2,199 | 2,199 | `pure/m2/` (M2-11..M2-16, planned) | Not started | No, once M2 lands. Open: how the legacy Stage-1 stream is kept. | No | No | No |
| Host runtime + serve / HTTP / DB | `omni-lang/src/host/mod.rs` | 1,119 | 1,119 | `pure/host_runtime.omni` (kernels only). Full port planned; it belongs in core on purpose. | Toy kernels only | Only a thin syscall/socket shim | No | No | No |
| Repair Agent | `omni-lang/src/repair_agent/mod.rs` | 1,456 | 1,456 | `pure_select_repair_strategy` kernel only. Full port planned; it belongs in core on purpose. | Toy kernel only | No | No | No | No |
| CLI | `omni-cli/*`, `omni-lang/src/lib.rs` | 1,331 | 1,331 | None yet | Not started | A thin launcher only | No | No | No |
| Harness / acceptance / inspect | `omni-lang/src/{harness,acceptance,inspect}` | 1,021 | 1,021 | `pure_acceptance_diff_check`, `pure_inspect_*` kernels only | Toy kernels only | No (dev/test tooling) | No | No | No |
| **Workspace subtotal** | 20 files | **16,733** | **16,980** | | | | | | |
| Monolith `omni_rust_bootstrap.rs` | 1 file | **18,379** | **18,611** | Mirror of the rows above, plus ML steps 2-5 | Every Rust change is mirrored here | No | n/a | n/a | No |
| **Total Rust** | 21 files | **35,112** | **35,591** | | | | | | |

How the monolith splits by section at `8160842`. The boundaries are the file's own `// ====` section banners and function starts, so they're approximate to within a few lines.

| Monolith section | Lines | Count |
|---|---|---:|
| ISA table, decoder, assembler | 1-739, 1762-2023 | 1,001 |
| Verifier + Omni-engine glue | 740-1039, 11453-12992 | 1,840 |
| VM | 1040-1761 | 722 |
| Frontend / compiler | 2024-5795 | 3,772 |
| Harness, inspect, acceptance, selftests | 5796-6052, 8469-9174, 16242-17946 | 2,668 |
| Repair Agent / agent driver | 6053-7489 | 1,437 |
| Host runtime + HTTP/DB | 7490-8468 | 979 |
| Pure-Omni phases | 9175-11452 | 2,278 |
| ML steps 2-5 (transpiler, tokenizer, model, GRPO). Monolith only; no workspace copy. | 12993-13826 | 834 |
| AOT x86/Wasm + Stage-1 | 13827-16241 | 2,415 |
| CLI `main` | 17947-18379 | 433 |
| **Sum** | | **18,379** |

## Reading the table honestly

- **Only one subsystem is really ported:** the verifier. Everything marked "toy kernel" has an Omni file with the right name that does a small arithmetic stand-in job. It doesn't replace the Rust.
- **The monolith is roughly half of all Rust, but most of it is a copy.** Deleting it is a "stop keeping two copies" decision, not porting work. The 834 lines of ML steps 2-5 exist only in the monolith and need their own keep-or-drop decision.
- **What stays in the Stage-0 seed** (my proposal, not decided): the VM, the ISA table and decoder it needs, a thin host shim (files, sockets, clock), and a thin CLI launcher. That's about 1,200-2,000 lines once the mirrors are gone. Everything else, including HTTP/DB and the Repair Agent (which stay in core on purpose), is meant to move into Omni.
- Omni source today: 2,237 lines across all `.omni` files at `74a215c` (`git ls-files '*.omni' | xargs wc -l`).
