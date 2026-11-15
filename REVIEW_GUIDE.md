# Review guide for an outside AI — 2026-11-14

Caleb wants an independent review of what is on GitHub now, and of the updates on the new branches, to judge whether the work is heading in the right direction.

This is a review, not a merge. **Do not push to, force-push, rebase, or otherwise alter `main` or any existing origin branch.** Read and comment. If you need a scratch tree, clone a new one.

Canonical context: [`HANDOFF.md`](./HANDOFF.md). Invariants, decisions, known bugs, and exact suite commands live there. This file is the tour: which branch to open, in what order, what each commit claims, how to check it, and the questions to answer.

## Invariants to hold every branch against

- 192 SSA and 16 labels per function (surface compiler today still rejects the 16th; that is a known C4 bug, not a reason to shrink the cap).
- 64-deep calls, eager args, no recursion in core `.omni`.
- No heap structs, hash maps, or GC: flat linear arena only. Compile-time offset constants, and an arena-backed sorted array with binary search, are allowed.
- Opcode table `0x7F00..0x7F7F` is never renumbered.
- 4-backend parity (VM, x86-64, Stage-1, Wasm) on every commit that changes behavior.
- Zero external crates.
- Frozen hashes must not move except for one planned re-freeze after the M2 fixed point:

  `0x23B8D17257CCA9C4` (159 words), `0xE221085777B8AC47` (824 B), `0x78462AE4C81DF408` (14,559 words), `0xEAFC81F46B482FF8` (106,242 B), `0x677F57AE71D28FED` (19,731 words).

- Omni is AI-first. The `u16` token stream is the real artifact.
- README claims must not overstate what actually runs in Omni versus Rust.

## How compare URLs work here

Every branch is also compared to `main` (`b3d42dd`, 2026-11-07), because that is the last tree Caleb asked an outsider to treat as "what is already on GitHub" before these updates. `main...branch` for anything based on `isa/multibyte-loads` is large: it includes the Nov 9–10 integration work as well as that branch's own commits.

When a tighter compare against the branch's real base exists, it is listed second. Use that one to judge the new work itself.

## Review order

### 1. `main` — baseline, read only

- Head: `b3d42dd` — *Strengthening Phase S4: Derived pure-bootstrap health metrics...* (2026-11-07).
- Compare: this *is* the baseline.
- Repo: https://github.com/Hacking-Unicorn/omni/tree/main

What it claims: Pure-Omni self-hosting is done enough that regular workflows route through a Pure-Omni frontend, verifier kernel, driver, and host by default, with Gen-1 == Gen-2 hashes and a 4-way differential lock. See `README.md` on this branch (those claims are still present later on `isa/multibyte-loads` as well).

How to verify: read `README.md` against `docs/audits/readme_honesty.md` and `docs/audits/scarlett_review.md` on this `docs/handoff` branch (those audits were of a later commit, `8160842`, but the Nov 6–7 "Pure-Omni" claims originate on `main`). Optionally build the monolith with `rustc -O --edition 2021 omni_rust_bootstrap.rs` and run `selftest` / `pure-bootstrap` / `pure-omni-diff`. Expected check counts on this old tip were 378 monolith / 339 workspace (Linux may have been 340). Do not treat those counts as current.

Judge it against the honesty questions in the last section.

### 2. `isa/multibyte-loads` — current integration tip

- Head: `d2d6c13` — *Give Wasm ISET/IGET real linear memory and stop pack5-folding runtime buffers.*
- Base: `main`.
- Compare vs `main`: https://github.com/Hacking-Unicorn/omni/compare/main...isa/multibyte-loads

Commits after `main`, oldest first, and what each claims:

| SHA | Claim |
|---|---|
| `ce42fc0` | Flatten the Cargo workspace; drop duplicate spec/std/pure/docs trees. |
| `a1273c8` | Keep a single canonical `omni_rust_bootstrap.rs`; drop identical copies. |
| `5c87b0f` | Add the root MIT LICENSE named in Cargo.toml. |
| `12492aa` | Split `omni-lang` into port-ready modules; keep the crate-root API. |
| `a876eb4` | Add cargo test wrappers for the full selftest matrix and run them in CI. |
| `f030d4d` | Bind host FS and DB opens to a canonical sandbox root; add escape selftests. |
| `14a4e51` | Port the token-stream verifier into real Omni and differentially test it against Rust. |
| `ff86c4b` | Convert FieldKV operand parsing from mutual recursion to an iterative skip. |
| `8160842` | Fix value-returning `if_pos` SSA join-slot holes; add a regression selftest. |
| `74a215c` | Enable lockstep shadow mode for compile, run, and serve. |
| `d2d6c13` | Give Wasm ISET/IGET real linear memory; stop pack5-folding runtime buffers. |

How to verify (from a clean checkout of this branch; last known counts from Vera's run at this SHA):

```
rustc -O --edition 2021 omni_rust_bootstrap.rs -o /tmp/omnic
/tmp/omnic selftest            # 444
/tmp/omnic acceptance-diff     # 39
/tmp/omnic pure-bootstrap
/tmp/omnic fixed-point         # writes ELF/.omni_bc into cwd
```

Invariants: hashes unchanged (Vera recomputed all five at this SHA). No new crates. Verifier engine is iterative. Lockstep must not replace the Rust verifier yet; it is a shadow check. `d2d6c13` must not change 2-op `IGET` of the Stage-1 input array (68 uses) if `0xEAFC…` is to stay still.

Known holes to look for while reviewing `d2d6c13`: `two_bufs` still 11 not 31; `pack5_flow` still 2009000 not 2255044; x86/Stage-1 byte readback still wrong. The commit message claims to stop pack5-folding of *runtime* buffers; Vera believes a branch-dependent all-immediate buffer is still folded.

### 3. `vera/parity-goldens` — the harness that should gate later work

- Head: `d257cb0`.
- Base: `d2d6c13`.
- Compare vs `main`: https://github.com/Hacking-Unicorn/omni/compare/main...vera/parity-goldens
- Compare vs actual base: https://github.com/Hacking-Unicorn/omni/compare/isa/multibyte-loads...vera/parity-goldens

Commits (all 2026-11-10, test/host code in the monolith only, per Vera):

| SHA | Claim |
|---|---|
| `b032e23` | Add a four-backend differential harness and cross-backend effect goldens (P1–P7, P9, P11 cases). |
| `78cd1f8` | P11: execute existing goldens natively and on the real VM. |
| `d257cb0` | Add label-cap boundary cases (15 / 16 labels) to the diff4 suite. |

P0 (in-tree hash pin) was deliberately omitted so it would not collide with main-line C1.

How to verify:

```
rustc -O --edition 2021 omni_rust_bootstrap.rs -o /tmp/omnic
/tmp/omnic selftest                 # 572
/tmp/omnic acceptance-diff          # 39
/tmp/omnic backend-diff --suite     # 109 pairs: 74 PASS, 35 XFAIL, 0 XPASS, 0 FAIL
/tmp/omnic pure-bootstrap
/tmp/omnic fixed-point
```

Invariants: no backend/VM/IGET/Stage-1-builder/stdlib-literal changes; five hashes unchanged at every commit (proof in `docs/audits/vera_harness.md` §5); strict XPASS (a listed divergence that starts passing must fail the suite until the xfail entry is removed); goldens derived by hand, not copied from a backend.

Read `docs/audits/vera_harness.md` and `docs/audits/vera_review.md` alongside the diff. The 16-label assembly result (ids 0..15 pass everywhere; the surface compiler is the real cap) is the factual basis for C4.

### 4. `nyx/m2-foundation` — Milestone 2 groundwork

- Head: `abea5ff`.
- **Actual base: `74a215c`, not `d2d6c13`.** This branch does **not** contain the Wasm ISET/IGET linear-memory commit.
- Compare vs `main`: https://github.com/Hacking-Unicorn/omni/compare/main...nyx/m2-foundation
- Compare vs actual base: https://github.com/Hacking-Unicorn/omni/compare/74a215c...nyx/m2-foundation
- Do not use `isa/multibyte-loads...nyx/m2-foundation` as a clean "what Nyx added" view; git will also show `d2d6c13` as missing on Nyx's side.

Commits:

| SHA | Claim |
|---|---|
| `cd2b94d` | M2-00: Milestone 2 decision record and plan. Docs only (`spec/m2/DECISIONS.md`, `spec/m2/PLAN.md`). |
| `cb600a3` | M2-01: frozen-hash guard, single-mode byte lock, and shape census. Read-only guards; new CLI `m2-frozen`, `m2-lock`, `m2-census`; `spec/m2/single_mode_lock.tsv`, `shape_census.tsv`. |
| `abea5ff` | M2-02: label-resolution reference tests. Locks Rust behavior; records known Stage-1 and Omni-verifier divergences in `spec/m2/label_reference.tsv`. CLI `m2-labels`. |

How to verify, on this branch (not on `d2d6c13`):

```
rustc -O --edition 2021 omni_rust_bootstrap.rs -o /tmp/omnic
/tmp/omnic m2-frozen
/tmp/omnic m2-lock
/tmp/omnic m2-census
/tmp/omnic m2-labels
/tmp/omnic selftest
/tmp/omnic fixed-point
```

Uncommitted lint fixtures and a `m2-lint` handler exist only in `docs/audits/wip/`; they are not part of these three commits.

Invariants: `pure/compiler.omni` untouched; no emitted-byte change from M2-01/M2-02 if the "read-only guard" claim is true; frozen hashes still match; label policy is "first definition wins, undefined rejected"; do not treat the 15-usable-label sentence in M2-00 as current (C4 is 16 — see HANDOFF.md §6.5).

### 5. `scarlett/m3-groundwork` — Milestone 3 plan

- Head: `6234c0c`.
- Base: `d2d6c13`.
- Compare vs `main`: https://github.com/Hacking-Unicorn/omni/compare/main...scarlett/m3-groundwork
- Compare vs actual base: https://github.com/Hacking-Unicorn/omni/compare/isa/multibyte-loads...scarlett/m3-groundwork

Commit:

| SHA | Claim |
|---|---|
| `6234c0c` | M3-00: M3 frontend plan and decision record. Docs only: `spec/m3/PLAN.md`, `spec/m3/DECISIONS.md`. Guard claim: selftests and frozen hashes identical to `d2d6c13`. |

How to verify: `git diff isa/multibyte-loads...scarlett/m3-groundwork` should be those two files only. Optionally re-run `selftest` / `acceptance-diff` / `fixed-point` if you want to confirm the "docs only" claim, but there is no code delta to test. Uncommitted lexer/fixture work is in `docs/audits/wip/` only.

Invariants: `pure/m3/` is the planned home; `pure/compiler.omni` and the 14,559-word suite stay frozen; M3 takes no Stage-1 label-cap exemption. S-5 still lists M3-03 as Scarlett's; that was later handed to Raven (HANDOFF.md §8). M3 O-1 (15 vs 16 labels) was later decided as 16.

### 6. `wip/isa-a-continuation` — UNVERIFIED, do not merge

- Head: `3b828b8`.
- Base: `d2d6c13`.
- Compare vs `main`: https://github.com/Hacking-Unicorn/omni/compare/main...wip/isa-a-continuation
- Compare vs actual base: https://github.com/Hacking-Unicorn/omni/compare/isa/multibyte-loads...wip/isa-a-continuation

Commit:

| SHA | Claim |
|---|---|
| `3b828b8` | *WIP (unverified, do not merge): ISA-A continuation, x86/Stage-1 byte readback in progress.* |

What the diff appears to contain (read it; do not trust this summary as a test result): +837/−6 in `omni_rust_bootstrap.rs`, `omni-aot`, `omni-vm`, and `omni-lang` `{compiler,pure_omni,verifier}`. Adds `ld_*`/`st_*` surface work, `run_multibyte_load_parity_selftests` / `MULTIBYTE_LOAD_EXPECTED = 200700007020`, a frozen-stream scan that forbids 3-op IGET and 4-op ISET `-2`/`-4` in locked streams, a verifier-engine case `ok_multibyte_ld`, and x86/Wasm/VM arms for the new shapes.

**This commit was published from an uncommitted working tree after the original executor stopped without reporting. The test suite was not run on it. It may be incomplete or wrong.** Review it as a snapshot of intent, not as accepted ISA-A.

How to verify, if you choose to: check it out in a **new** clone, then run at least Vera's `backend-diff --suite` (after cherry-picking or rebuilding the harness against this tree — the harness is not on this branch) plus monolith `selftest`, `acceptance-diff`, and `fixed-point`. Watch `0xEAFC…` especially: 2-op `IGET` of Stage-1 param 0 must stay byte-identical. Do not merge on the basis of reading the diff alone.

### 7. `docs/handoff` — this documentation pack

- Head: the tip of this branch (docs-only commit on top of `d2d6c13`).
- Base: `d2d6c13`.
- Compare vs `main`: https://github.com/Hacking-Unicorn/omni/compare/main...docs/handoff
- Compare vs actual base: https://github.com/Hacking-Unicorn/omni/compare/isa/multibyte-loads...docs/handoff

Claim: adds `HANDOFF.md`, this file, and `docs/audits/**` (plans, reviews, Vera patches and repro programs, and WIP diffs/tars). **No compiler, VM, ISA, or test code.**

How to verify: `git diff --stat isa/multibyte-loads...docs/handoff` should be documentation paths only. The quickest behavioral suite, `acceptance-diff` (39), was run on a `rustc -O` monolith built from the unchanged `d2d6c13` code; see the bottom of `HANDOFF.md` for the result. You can repeat it. Do not expect this branch to contain `backend-diff` or Nyx's `m2-*` commands.

`docs/audits/wip/scarlett-m3-groundwork.untracked.tar` is large because it contains a compiled `omni_mono`; that is a captured build artifact, not a design doc.

### Other existing origin branches (context only)

- `port/verifier-to-omni` (`ff86c4b`): https://github.com/Hacking-Unicorn/omni/compare/main...port/verifier-to-omni — already an ancestor of `d2d6c13`.
- `chore/repo-hygiene-and-hardening` (`f030d4d`): https://github.com/Hacking-Unicorn/omni/compare/main...chore/repo-hygiene-and-hardening — already an ancestor of `d2d6c13`.

There are no origin branches yet for Ember or Raven.

## Questions we want you to judge

Please answer these directly, with evidence (file:line or a compare URL). "Looks fine" is not enough; "I cannot tell because X was not run" is acceptable.

1. **Is Option A — new operand shapes on IGET/ISET — the right call?** 3-op `IGET buf, mode, off` with mode `-2/-4/-8` as LE loads; 4-op `ISET buf, mode, off, val` with mode `-2/-4` as patches; strict bounds; leave `-9` and 2-op `IGET` exactly as they are because Stage-1 uses 2-op IGET 68 times. Is this the right ISA extension, or is there a safer way to get byte-accurate buffers without new shapes?

2. **Is the lockstep strategy sound?** At `74a215c`, compile/run/serve shadow the Rust verifier with the Omni engine (VM-only). Overhead policy is deferred to Caleb. Does this actually protect the port, or does VM-only shadowing give false confidence? Should it stay off the default path until the engine also runs natively?

3. **Is the Milestone 2 plan sound?** Two-pass O(n) u16→ELF64 compiler in iterative Omni under `pure/m2/`, frozen `pure/compiler.omni`, 128 KiB multi-mode window, the approved arena layout, first-definition-wins labels, Wasm CALL required for sign-off, re-freeze hashes once after stage2==stage3. Read `spec/m2/PLAN.md` on `nyx/m2-foundation` and `docs/audits/milestone2_plan_final.md`. Call out anything that would violate an invariant or cannot actually reach a fixed point.

4. **Are the honesty findings about the Nov 6–7 "Pure-Omni" commits correct?** Those commits are on `main` (Phase A `baee4e2` through Strengthening S4 `b3d42dd`, 2026-11-06 and 2026-11-07). The README still says regular workflows route through Pure-Omni and that Gen-1 == Gen-2 is a self-hosting fixed point. Scarlett and the honesty audit say: only the verifier is a real Omni port; "Pure-Omni frontend/host" are Rust wrappers or selftest-only kernels; Phase D is the same Rust compiler twice; `pure-omni-diff` is not a 4-backend lock; the suite hash excludes `verifier_engine.omni` on purpose. See `docs/audits/readme_honesty.md`, `docs/audits/scarlett_review.md`, and `docs/audits/README_draft.scarlett.md`. Note that lockstep (`74a215c`) landed *after* those audits.

Also welcome, if you have time:

5. Is C4 correctly aimed at the source compiler's `next_lbl` starting at 1 (`omni_rust_bootstrap.rs:3587`, `:3646-3656`) rather than at the emitter?
6. Is returning `-1` on `DB_QRY` miss, and allowing an arena sorted array for KV/DB, consistent with the no-maps rule and with existing goldens?
7. Given `d2d6c13` did not actually close `two_bufs` / `pack5_flow`, is the current ISA-A sequence (finish x86/Stage-1 readback, then hunt those mismatches) the right order?
8. Should `nyx/m2-foundation` be replayed onto `d2d6c13` before more M2 code lands, or is leaving it on `74a215c` acceptable for docs-and-guards work?

Please report what you ran, what you only read, and anything you found contradictory.
