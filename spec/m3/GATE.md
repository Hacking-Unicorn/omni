# M3-02: the `pure/m3/` gate (fails closed)

**What the plan asks for.** Run `m2-lint` (no recursion; ≤ 15 labels hard until C4, 16 after; ≤ 192 SSA; targets 12 / 120) and the M2-08 module check (acyclic call graph, ≤ 64 frames) on every `pure/m3/*.omni` module (`spec/m3/PLAN.md` §4, §8).

**Status update (2026-11-20, verified on `isa/multibyte-loads` @ `3e623e1`).** The tools described above as missing have since landed and are wired in: M2-03 (`m2-lint`) ships in the monolith and lints all 15 registered `pure/m2`, `pure/m3`, `pure/m4`, `pure/m5`, and `pure/isa` modules (0 gated findings), the M2-08 module/call-depth check is wired into `m3-gate`, and `pure/m3/` holds 6 modules (`arena`, `lex`, `skim`, `expr`, `stmt`, `lower`, 3,621 lines total). `M3_GATE_TOOLS_WIRED = true`, and `omnic m3-gate` passes 6/6 modules. The fail-closed behavior above now applies only when a tool is missing, as designed. The "What exists on this base" snapshot below is preserved as the 2026-11-14 record.

**What exists on this base (`d2d6c13`, checked 2026-11-14).**
- `m2-lint`: **not shipped anywhere.** It is uncommitted WIP only (`docs/audits/wip/` on `docs/handoff`). On `nyx/m2-foundation` (`abea5ff`), `omni_rust_bootstrap.rs:18886` and `omni-cli/src/main.rs:1042` route the word `m2-lint` to `run_m2_cli`, but `run_m2_cli` only handles `m2-frozen`, `m2-lock`, `m2-census` and `m2-labels` (HANDOFF §9).
- M2-08 module/call-depth check: **not written** on any branch.
- `pure/m3/`: **does not exist yet.** The first M3 `.omni` module is M3-05.

**Decision.** M3 does not write a second lint next to Nyx's. Instead the gate **fails closed**:

```
omnic m3-gate [dir]        # default dir: pure/m3
```

- No `.omni` modules under `dir` → `[M3-GATE PASS] 0 modules` (vacuous) and exit 0.
- Any module → `error[M3_GATE_UNAVAILABLE]` and exit 1, whether the module is clean or recursive.
- Selftest 13M3b (monolith + workspace) and `cargo test` (`m3_gate_fails_closed`) assert both properties. When run from the repo root, the selftest also gates the real `pure/m3/`, so the first M3 module fails the suite until the real tools are wired in.

**To lift it.** When M2-03 (`m2-lint`) and M2-08 land on the base, call them from `m3_gate_check` on each module, set `M3_GATE_TOOLS_WIRED = true` in the same commit, and keep the fail-closed self-check for the case where a tool is missing. M3-05 is blocked on this, as the plan already requires (M3-02 needs M2-03 and M2-08).
