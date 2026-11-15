# Uncommitted work saved 2026-11-14 (America/New_York)

These files capture dirty trees that were **not** committed. They are not on the corresponding feature branches. Apply only in a private clone, then re-run the suites. Nothing here was re-tested on this date.

| File | Source tree | What it contains |
|---|---|---|
| `isa-multibyte-loads.patch` | `/workspace/omni` on `isa/multibyte-loads` at `d2d6c13` | Uncommitted ISA-A continuation (6 files, +837/−6). Also published, unverified, as `wip/isa-a-continuation` (`3b828b8`). Do not merge. |
| `nyx-m2-foundation.patch` | `/workspace/audit-nyx-impl` on `nyx/m2-foundation` at `abea5ff` | Uncommitted `omni_rust_bootstrap.rs` (+525/−1). Appears to add `m2-lint` CLI and related logic; not in the three pushed commits. |
| `nyx-m2-foundation.untracked.tar` | same | Untracked `spec/m2/lint/*.omni` fixtures (15 files). |
| `scarlett-m3-groundwork.patch` | `/workspace/audit-scarlett-review` on `scarlett/m3-groundwork` at `6234c0c` | Uncommitted edits in `omni-cli`, `compiler/mod.rs`, `pure_omni/mod.rs`, and the monolith (+665). |
| `scarlett-m3-groundwork.untracked.tar` | same | Untracked `spec/m3/FE_DUMP.md`, `spec/m3/corpus.tsv`, `spec/m3/fixtures/`, and a 13 MiB compiled `omni_mono` binary. The binary is a build artifact and can be rebuilt; it is kept only so the dirty tree can be restored exactly. |

`.stat.txt` files are `git diff --stat` companions.

Apply a patch from a clean checkout of that branch:

```
git apply --check docs/audits/wip/<name>.patch
git apply docs/audits/wip/<name>.patch
tar -xf docs/audits/wip/<name>.untracked.tar
```
