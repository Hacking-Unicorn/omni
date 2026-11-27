# Addons Item 1 — Single Source of Truth for Acceptance Rules (`spec/source-of-truth.md`)

**Baseline commit:** `main @ 8aec4e4e2a7590f5e8a5df243838331fa5cbfbc7` (after PR #70, Step 16 Part B7)
**Deliverable scope:** Part A inventory, single-source design, acid-test specifications, discovered-drift register, 47-slot `RESERVED` audit, and Part B PR sequencing. **No code changes, no behavior changes, no pin changes in this PR.**

> **Invariants across Part B:** Every Part B judge PR preserves exact runtime acceptance/rejection behavior across all 128 opcode slots (`0x7F00..=0x7F7F`), exact decompiler stamps, `--campaign-all` inventory at `2800 mutants | 1217 accepted | 1583 rejected | soundness=0 roundtrip=0 judge=12` (`0/0/12`), all `verify_step8_harness.sh` controls (`76/76` before new differential gates), all `verify_step7_driver.sh` checks (`38/38`), `scripts/zero_rust_rebuild.sh` clean-room parity (`S1 == S2 == S3 == bootstrap/m2.elf`), and the 5 frozen hashes. **Every drift uncovered below is reported for explicit triage and is never absorbed silently into restructuring.**

---

## 1. Sweep Methodology (Predicates, Ranges, Constants, and Semantics)

To avoid literal-value blind spots (the B7 `san_dom` lesson), every rule family was swept using **five complementary searches** against `pure/`, `scripts/`, and `spec/` at `main @ 8aec4e4`:

1. **Range & comparison predicates (`pure/`):**
   ```text
   git grep -n -E "(in_range|m2u_in|is_eq|m2u_eq)\(" -- pure/
   ```
   Covers every boundary check, inclusive range check (`[lo..=hi]`), and equality guard in `verifier_engine.omni`, `m2/verify/mod_check.omni`, `m2/labels.omni`, `m2/emit_multi.omni`, `audit/decompile_rhs.omni`, `audit/decompile.omni`, `m3/skim.omni`, `m3/stmt.omni`, `m3/inc/lower_body.omni`, and `m4/`.
2. **Named symbol & boundary-constant sweep (`pure/`, `scripts/`, `spec/`):**
   ```text
   git grep -n -E "(op_is_known|op_min_ops|op_is_sanitizer|op_is_destless|op_has_dest|op_req_cap|op_is_sink|m2v_ins_min_ops|dc_op_fixed_ty|dc_can_rhs|m2lbl_slot_of|op_label_slot|isa_tbl|isa_op_|isa_mn_|4080|4079|4095|4096)" -- pure/ scripts/ spec/
   ```
3. **Raw opcode token-band sweep (`pure/`, `scripts/`):**
   ```text
   git grep -n -E "(325[0-9][0-9]|326[0-3][0-9]|0x7[Ff][0-7][0-9A-Fa-f])" -- pure/ scripts/
   ```
4. **Full 128-entry (`0x7F00..=0x7F7F`) cross-evaluation:**
   Evaluated `pure/isa/table.omni` (`isa_op_rows_0..7`, `isa_mn_rows_0..7`), `pure/m2/verify/mod_check.omni::m2v_ins_min_ops`, `pure/verifier_engine.omni` (`op_is_known`, `op_min_ops`, `op_is_destless`, `op_has_dest`, `op_req_cap`, `op_is_sink_raw`, `op_is_sanitizer`), `pure/audit/decompile_rhs.omni` (`dc_op_fixed_ty`, `dc_can_rhs`), and `pure/m2/emit_multi.omni` across all 128 slots (`32512..=32639`).
   - Exact opcode partition on `main @ 8aec4e4`: **`81` accepted (`known == 1`, `m2v_ins_min_ops >= 0`)** and **`47` rejected (`known == 0`, `m2v_ins_min_ops == -1`)**.
   - Lowerability split in `pure/m2/emit_multi.omni`: `51` live lowered opcodes (`0` unaccepted ops lowered; `30` accepted opcodes have no `emit_multi` x86 case — `24` host/pack5/VM opcodes plus `0x7F08` `CSTR`, `0x7F2E` `TADD`, `0x7F2F` `TMUL`, `0x7F46`, `0x7F56` `I2S`, `0x7F5D` `FNV1A`).
5. **M3 emission & lowercase FNV-1a31 hash decode (`pure/m3/`):**
   Decoded all `57` FNV-1a31 lowercase builtin/intrinsic hashes (`hlow`) in `pure/m3/inc/lower_body.omni` and enumerated every emitted opcode across `pure/m3/{expr,stmt,inc/lower_body}.omni` (`59` distinct emitted opcodes; confirmed absence of non-existent M3 intrinsics `i2s`, `s2i`, `scat`, `sconc`, `san_sql`, `san_dom`, `san_prm`, `st_u8`).

> **Scope note on non-authoritative mirrors:** `spec/step*.md` evidence snapshots, `pure/programs/*` user apps, and `pure/harness/{core_gates,proposer_loop}.omni` pin-comment strings are historical/test consumers, not duplicate acceptance-rule tables, unless listed under a rule below (`spec/trusted_core.md:25` and `scripts/external_verifier_check.py:27` ARE listed).

---

## 2. Discovered Drifts Register (Report & Triage Before Fixing)

Every drift below already exists on `main @ 8aec4e4`. **None of these may be fixed silently inside a Part B restructuring PR.** Each must either be triaged and closed in its own fix PR before the affected Part B PR lands, or explicitly preserved until triaged.

| ID | Rule Family | Site A vs Site B | Observed Divergence | Runtime Impact Today & Required Action |
|---|---|---|---|---|
| **D1a** | R1 (`RESERVED` metadata) | `pure/isa/table.omni:134-307` vs `mod_check.omni:210-226` & `verifier_engine.omni:174-185` | `42` of the `47` rejected slots still carry non-zero `flags` (`1`, `3`, or `5`) in `table.omni` (`16` carry `cap != 0`; `3` carry `flags & 4` sanitizer bit `0x7F5A..=0x7F5C`); only `5` holes (`0x7F47, 0x7F49, 0x7F4F, 0x7F51, 0x7F55`) have `flags == 0`. Also `0x7F46 (32582)` is **accepted** with `flags == 0`. | Masked by `op_is_known == 0`. **Closed in Part B PR 1** by introducing bit 5 (`32 = ACCEPTED`) on the `81` known rows and zeroing `cap=0, flags=0 // RESERVED` on all `47` rejected rows. |
| **D1b** | R1 (Spec prose) | `spec/trusted_core.md:25` vs `pure/isa/table.omni` | `spec/trusted_core.md:25` states `(25 capability-gated opcodes)` in `0x7F00..=0x7F7F`; `table.omni` currently has `40` rows with `cap != 0` (`24` known + `16` rejected). | Documentation drift only; update `spec/trusted_core.md:25` alongside PR 1 `RESERVED` completion (once `47` holes zero, active count = `24`). |
| **D1c** | R1 (External TCB checker) | `scripts/external_verifier_check.py:27` | Uses contiguous `OP_BASE, OP_MAX = 0x7F00, 0x7F7F` with no `47`-hole check. | Deliberate coarse TCB trace sanity script on golden traces (`external_verifier_check.py`); either gate via **(c)** or exclude holes in a separate harness/spec decision. |
| **D2a** | R2 (`op_min_ops`) | `verifier_engine.omni:191-205` vs `mod_check.omni:210-226` | On `7` rejected slots, `op_min_ops` returns non-zero arities (`0x7F19..=0x7F1D` → `2`, `0x7F51` → `2`, `0x7F55` → `1`) whereas `m2v_ins_min_ops` returns `-1` (and `op_min_ops` returns `0` on the other `40` holes). | Masked at runtime by `op_is_known == 0`. Disappears naturally when Part B PR 2 switches both consumers to byte `+5` of `isa_op_rows` (with `0` stored on all `47` RESERVED rows). |
| **D2b** | R2 (`op_is_destless`) | `verifier_engine.omni:351-366` vs `table.omni` bit 0 | `0x7F51 (32593)` and `0x7F55 (32597)` have `table.omni` `flags == 0` (`dest=0`) while omitted from `op_is_destless` (`op_is_destless == 0`); meanwhile rejected `0x7F47, 0x7F49, 0x7F4F` ARE in `op_is_destless`. On all `81` known ops, `op_has_dest == (flags & 1)` has **0 mismatches**. | Masked on holes by `op_is_known == 0`; aligns cleanly when Part B PR 2 derives `op_has_dest` directly from `flags & 1` + `flags & 32`. |
| **D4a** | R4 (Decompiler labels) | `audit/decompile_rhs.omni:101-110` (`dc_label_id`), `audit/decompile.omni:289` (`dc_any_branch_to`) vs `m2/labels.omni:21-32` | Decompiler maps `IMM` directly (`dc_imm_val`) and `LBL` with negative shift (`-1 - (t - 32496)`), and ignores `IMM < 0` / `IMM >= 4080` boundary checks that M2 (`m2lbl_slot_of`) and `verifier_engine` (`in_range(0, 4079)`) enforce. | Separate component concern (decompiler only receives verified `.omni_bc`, though unverified inspection could collide negative `IMM` `-1..-16` with `LBL`). **Do NOT change silently in PR 4; triage separately.** |
| **D5a** | R5 (Function-table structural bounds) | `m2/verify/mod_check.omni:64-88` vs `verifier_engine.omni:1200-1347` | M2 checks `fn_cnt < 256`, `0 <= pcnt <= 32`, `icnt >= 1`, unique `fn_id`, and acyclic call graph (`m2v_check_acyclic`). `verifier_engine.omni` enforces `fn_idx in 0..=1023` (`3006`) and final `RET` (`2012`), plus its own limits (`mcnt < 512`, recursive calls allowed by verifier). | Intentional division of checks vs missing structural checks (`icnt`, `pcnt`, dup `fid`). Report for triage; Part B PR 5 unifies ONLY `fn_idx in 0..=1023` and final `RET (32586)` unless triage widens scope. |
| **D6a** | R6 (`dc_op_fixed_ty`) | `audit/decompile_rhs.omni:419` vs `table.omni:114` AND `m3/inc/lower_body.omni:476` | **`0x7F6F (32623)` `RSP_HTML` (`respond_html`)**: `dc_op_fixed_ty` returns **`32641` (`i64`)** (covered by `m2u_in(op, 32612, 32624)`), whereas BOTH `pure/isa/table.omni:114` (`tyb=4` → `32644`) AND `pure/m3/inc/lower_body.omni:476` (`let ty_tok = m2u_sel(c7, 32644, 32641)`) emit **`32644` (`TY_STR`)**! | **LIVE BUG / DRIFT.** Any real M3-compiled `respond_html(...)` instruction carries `TY_STR (32644)`, so `dc_can_rhs:451-452` currently rejects exact reconstruction (`want_ty=32641 != 32644`) and stamps it `APPROXIMATE`! **Stop & triage D6a before Part B PR 6.** |
| **D6b** | R6 (`dc_op_fixed_ty`) | `audit/decompile_rhs.omni:419` vs `table.omni` (`mn_rows`) | `0x7F66 (32614)`, `0x7F67 (32615)`, `0x7F69 (32617)`, `0x7F6A (32618)` return `32641` (`TY_I64`) in `dc_op_fixed_ty`, but have no active `isa_mn_rows` entries in `table.omni` (not emitted by M3). | Must be decided in D6 triage (assign `0` polymorphic vs document host/SQL/HTTP op fixed types in `table.omni` byte `+6`). |
| **D6c** | R6 (`dc_op_fixed_ty`) | `audit/decompile_rhs.omni:417-425` vs `table.omni` (`mn_rows`) | `10` known opcodes have a single fixed non-void type in `table.omni::isa_mn_rows` while `dc_op_fixed_ty` returns `0` (`CINT`, `MOVE`, `CSTR`, `CTRIT`, `LOOKAHEAD`, `I2S`, `SAN_SQL`, `SAN_DOM`, `SAN_PRM`, `FNV1A`). (`CINT` `0x7F06` is intentionally polymorphic in `proposer_loop.omni:169`; `0x7F56..0x7F59, 0x7F5D` are unemitted by M3.) | `0x7F06` (`CINT`) requires `0` (polymorphic exemption in decompiler byte `+6`); the other 9 need explicit triage decision before PR 6 populates table byte `+6`. |
| **D7a** | R7 (`dc_can_rhs`) | `audit/decompile_rhs.omni:435-436` vs `pure/m3/` | `0x7F56` (`i2s`, `32598`), `0x7F57` (`san_sql`, `32599`), `0x7F59` (`san_prm`, `32601`), `0x7F5D` (`fnv1a`, `32605`) are whitelisted in `dc_can_rhs` (`is_1`) and printed as builtin calls (`decompile_rhs.omni:362-374`), but **none exist in `pure/m3/`** (all fail round-trip if M3 tries to recompile them!). | **LIVE DRIFT (corpus-masked).** Exact same class as B7 `san_dom` (`0x7F58`); masked only because no corpus seed contains them. **Triage before Part B PR 7.** |
| **D7b** | R7 (`dc_can_rhs`) | `audit/decompile_rhs.omni:333, 434` vs `pure/m3/inc/lower_body.omni:928` | `0x7F21` (`BNOT`, `32545`) is in `dc_can_rhs` `is_1`, emitted as `bnot(p0)` (`decompile_rhs.omni:333`); M3 lowers unary `~x` (`aop=2`), not `bnot(...)`. | **LIVE DRIFT.** Check round-trip recompile under `m3.elf`; triage before Part B PR 7. |
| **D7c** | R7 (`dc_can_rhs`) | `audit/decompile_rhs.omni:353, 439` vs `pure/m3/inc/lower_body.omni:541` | `0x7F31` (`UNPACK5`, `32561`) is in `dc_can_rhs` `is_2`, emitted as `unpack5(p0, p1)` (`decompile_rhs.omni:353`); M3 parses `[t0, ...]` tritvec literals rather than `unpack5(...)`. | **LIVE DRIFT.** Check round-trip recompile under `m3.elf`; triage before Part B PR 7. |
| **D7d** | R7 (`dc_can_rhs`) | `audit/decompile_rhs.omni:356, 441` vs `pure/m3/inc/lower_body.omni:724` | `0x7F3F` (`TLOGIC`, `32575`) is in `dc_can_rhs` `is_3`, emitted as 3-arg `tlogic(p0, p1, p2)` (`decompile_rhs.omni:356`); M3 `icls=15` checks arity differently. | **LIVE DRIFT.** Verify exact M3 `tlogic` arity & round-trip before PR 7. |
| **D7e** | R7 (`dc_can_rhs`) | `audit/decompile_rhs.omni:380, 436` vs `pure/m3/inc/lower_body.omni:469-477` | **`0x7F6D` (`REQ_PARAM`, `32621`)**: `dc_can_rhs` puts `32621` in `is_1` (`nops == 1`) and `dc_rhs_g5b:380` prints 1-arg `req.param(p0)`. M3 `lower_body.omni:477` passes `has_op1 = 1` (`c1=1`) and emits **`nops = 2`** (`r0, op1`) even for 1-arg `req_param(...)`! | **LIVE DRIFT.** M3-compiled `req_param` (`nops=2`) fails `dc_can_rhs` (`nops==1`) and becomes `APPROXIMATE`! **Triage before PR 7.** |
| **D7f** | R7 (`dc_can_rhs`) | `audit/decompile_rhs.omni:246, 313, 444` vs `pure/m3/inc/lower_body.omni:686` | `dc_rhs_iset_st:246` prints `st_u8(p0, p2, p3)` on `wt == -1` for `ISET (32527)`, and `dc_can_rhs:444` accepts any `nops >= 2`; decoded `hlow` table in `pure/m3/inc/lower_body.omni` has no `st_u8` hash (only `st_u16`/`st_u32`/`st_u64` via `ld_st_spec`). | **LIVE DRIFT.** Check if M3 emits `wt == -1` for `ISET` or if `st_u8` is missing in M3; triage before PR 7. |

---

## 3. Per-Rule Inventory, Single-Source Design, and Acid Tests

Derivation mechanism preference order:
- **(a)** Consumer computes/reads from the designated single source's data/code at build or run time.
- **(b)** Build step generates consumer code from the single source, gated by a regenerate-and-diff check.
- **(c)** Test-pinning: copy remains, with a permanent differential gate failing on drift (**requires explicit justification why (a) and (b) do not apply**).

---

### R1 — Accepted Opcode Set (`op_is_known` + `47` `RESERVED` Slots)

- **Fact:** Which of the `128` opcode tokens `0x7F00..=0x7F7F` (`32512..=32639`) are accepted (`81` opcodes) vs rejected (`47` unassigned/reserved slots: `3001` in verifier, exit `1` in M2 `mod_check`), plus documented `RESERVED` status across spec and table.
- **Sites & backing searches:**
  - `pure/isa/table.omni:5-307` — `isa_op_rows_0..7` (128 × 8 B op rows) and `isa_mn_rows_0..7` (`81` active mnemonic rows, plus zeroed `RESERVED` rows at `:16-17, :24` and op rows `:247, :251`). Flags byte `+4` currently uses bits `0..4` (`1=dest`, `2=sink_safe`, `4=sanitizer`, `8=x86_aot`, `16=wasm_aot`).
  - `pure/m2/verify/mod_check.omni:210-226` — `m2v_ins_min_ops(op)` returns `0..4` on the `81` known opcodes, `-1` on the `47` holes (called via `m2v_step_ins:242`).
  - `pure/verifier_engine.omni:174-185` — `op_is_known(op)` hand-mirrors the `15` hole sub-ranges inside `in_range(op, 32512, 32639)` (`called at :983`, emitting error `3001`).
  - `spec/trusted_core.md:25` — `"25 capability-gated opcodes"` (**D1b**).
  - `scripts/external_verifier_check.py:27` — `OP_BASE, OP_MAX = 0x7F00, 0x7F7F` (**D1c**).
  - *Backing negative searches:* Predicate grep (`in_range|m2u_in|is_eq|m2u_eq`) and raw-range grep (`325[0-9][0-9]|326[0-3][0-9]|0x7F..`) confirm `m2/decode.omni` only checks `m2u_in(tok, 32512, 32639)` (`DECODE`, all 128 tokens decode symmetrically per `DECISIONS.md` D-13); `m2/emit_multi.omni` lowers a `51`-op subset (verified `0` unaccepted ops lowered); `m4/` does not govern acceptance.
- **Designated Single Source & Rationale:** **`pure/isa/table.omni`**. Per `spec/m2/DECISIONS.md` **D-13**, *"Opcode metadata has exactly one copy: `pure/isa/table.omni`"*. Both `m2` and `verifier_engine` already exist in the Pure-Omni build graph alongside it, and `table.omni` has zero `include` dependencies and `0` external calls.
- **Layout constraint (why bit 5 = `32` `ACCEPTED` is required, NOT `flags != 0`):**
  Full 128-op inspection shows `0x7F46 (32582)` is **accepted** (`known=1`, `min_ops=0`, `destless=1`) and currently has `flags == 0` in `pure/isa/table.omni:230` (lower 5 bits all `0`: no dest, not sink_safe, not sanitizer, not x86/wasm AOT-lowered). Meanwhile `42` of the `47` rejected holes still have non-zero lower flag bits (`D1a`). Therefore:
  1. Reserve **`flags` bit 5 (`32`) = `ACCEPTED`** in `isa_op_rows_0..7` (preserving the exact `2,320 B` table geometry, `isa_band_base`, `isa_rodata_rel = 4096`, `isa_op_off = 1296`, `isa_tbl_len = 2320`).
  2. Set `flags |= 32` on the `81` accepted slots; set `cap_lo=0, cap_hi=0, flags=0 // RESERVED (<op>)` on all `47` unassigned slots (closing **D1a** alongside the `RESERVED` sweep).
- **Derivation Mechanism:** **(a) Runtime/build-time table lookup.**
  - `pure/verifier_engine.omni` includes/links `pure/isa/table.omni` (compiled via `omnic_pure.elf compile`, which resolves `include`, keeping `pure/verifier_standalone.omni_bc` well within M2's `65,536`-word window — `16,380 w` base) and computes `op_is_known(op)` as `in_range(op, 32512, 32639) * isa_op_flag(tbl, tbl_base, op - 32512, 5)` (caching `tbl` inside workspace or module-init helper once per `omni_verify_module_with_mode` call rather than rebuilding 2,320 B per instruction!). Note: `table.omni:425` already uses 3-op `ld_u8` (`IGET -1`), so M2 lint L-4 is satisfied.
  - `pure/m2/verify/mod_check.omni` already links `pure/isa/table.omni` (`m2_standalone.omni:4` links `util`, `table`, `decode`, `labels`, `mod_check`) and checks acceptance from `isa_op_flag(..., 5)` (in PR 1, `m2v_ins_min_ops` returns `-1` when bit 5 is `0`; PR 2 removes the remaining `m2v_ins_min_ops` arity literals too).
- **Acid test (feasible):** In a scratch tree ONLY, flip bit `5` (`^= 32`) on a single row in `pure/isa/table.omni` (e.g., clear bit 5 on an accepted op `0x7F10`, or set bit 5 + minimal `cap/flags` on a hole) with **zero edits** to `verifier_engine.omni` or `mod_check.omni`; rebuild via `zero_rust_rebuild.sh` (scratch build); confirm BOTH `verifier.elf` (`3001`) and `m2.elf` (`rc=1`) flip verdict on that opcode.

---

### R2 — Operand Arity (`op_min_ops`) and Destination Requirement (`op_has_dest` / `op_is_destless`)

- **Fact:**
  1. Exact minimum operand count per opcode (`0..4` for the `81` known ops; `-1` / rejected if unknown).
  2. Whether the opcode requires a destination register (`1` for `72` known opcodes, `0` for `9` known destless opcodes: `0x7F00`, `0x7F05`, `0x7F40..=0x7F44`, `0x7F46`, `0x7F4A`; the remaining `3` entries in `op_is_destless` — `0x7F47`, `0x7F49`, `0x7F4F` — are `RESERVED` holes).
- **Sites & backing searches:**
  - `pure/m2/verify/mod_check.omni:210-226` — `m2v_ins_min_ops(op)` returns `0..4` or `-1` (**only** arity source in M2; `mod_check.omni` does not enforce `has_dest` separately beyond decoded `ins_has_dest` + `min_ops`).
  - `pure/verifier_engine.omni:191-205` — `op_min_ops(op)` duplicates `mod_check`'s arity lists (plus dead hole arities **D2a**).
  - `pure/verifier_engine.omni:208-213, 351-366` — `op_has_dest(op)` and `op_is_destless(op)` (used in `verify_ins_body:1004-1005` for error `2012`; **0 mismatches** vs `table.omni` `flags & 1` on all `81` known ops; hole drift **D2b**).
  - `pure/isa/table.omni:134-307` — `flags` bit `0` (`1`) encodes `has_dest`; bytes `+5..+7` of every `8 B` op row (`iset(o, idx, 127, cap_lo, cap_hi, flags, b5, b6, b7)`) are currently unused `0`.
  - *Backing negative searches:* Predicate and constant greps confirm neither `emit_multi`, `emit_x86`, nor `m4` holds a separate `min_ops` table (`emit_multi` only reads decoded operand fields for the `51` opcodes it lowers).
- **Designated Single Source & Rationale:** **`pure/isa/table.omni`**.
  - `has_dest` is **already** in `flags` bit `0` (exact 81/81 agreement with `op_has_dest`!).
  - `min_ops` (`0..4`) fits cleanly into unused byte **`+5`** of each `8 B` op row (with `0` on `RESERVED` rows), keeping `isa_tbl_len == 2320` unchanged and giving both `mod_check.omni` and `verifier_engine.omni` a single lookup (`isa_op_min_ops(tbl, tbl_base, idx)` = `ld_u8(tbl, tbl_base + 1296 + idx * 8 + 5)` gated by `ACCEPTED` bit 5).
- **Derivation Mechanism:** **(a) Runtime/build-time table lookup.**
  - `verifier_engine.omni`: `op_min_ops(op)` reads byte `+5` from `isa_tbl` (gated by `op_is_known`); `op_has_dest(op)` reads `(flags & 1) * ((flags >> 5) & 1)`, eliminating `op_is_destless`'s hand list.
  - `mod_check.omni`: `m2v_ins_min_ops(op)` reads `isa_op_flag(..., 5)` and byte `+5`, returning `-1` if not `ACCEPTED`.
- **Acid test (feasible):** Scratch-mutate ONLY `flags` bit `0` (`dest`) or byte `+5` (`min_ops`) on one row in `pure/isa/table.omni` with zero consumer edits; rebuild; verify `verifier.elf` rejects missing/extra operands or missing dest with `2012` AND `m2.elf` rejects too-few operands with `rc=1`.

---

### R3 — Capability Masks, Sink Taint Guards, and Sanitizers (`op_req_cap`, `op_is_sink_raw`, `op_is_sanitizer`)

- **Fact:**
  - Required 16-bit capability bitmask per opcode (`0` or bitmask in `1..=512`, `24` active gated ops after hole cleanup).
  - Whether opcode is a taint-sensitive sink (`flags & 2`, rejecting `@UNTRUSTED` operands with `1002`).
  - Whether opcode is a taint sanitizer (`flags & 4`: `0x7F57..=0x7F59, 0x7F70` = `32599..=32601, 32624`; clears taint).
- **Sites & backing searches:**
  - `pure/isa/table.omni:134-307` — stores `cap_lo` (byte `+2`), `cap_hi` (byte `+3`), `flags` bit `1` (`sink_safe`), bit `2` (`sanitizer`) across all `128` rows.
  - `pure/verifier_engine.omni:230-348` — `op_req_cap_0/1` (`L230-285`), `op_is_sink_raw_0/1` (`L288-335`), and `op_is_sanitizer` (`L338-348`).
  - *Exact 128-op check:* **`0` mismatches across all `128` slots** between `verifier_engine.omni` and `pure/isa/table.omni` (`ve_cap == tbl_cap`, `ve_sink == (tbl_flags>>1)&1`, `ve_san == (tbl_flags>>2)&1`), including the `16` cap-bearing holes and `3` sanitizer holes (`0x7F5A..=0x7F5C`, which zero in PR 1 `RESERVED` cleanup while leaving behavior unchanged since `op_is_known == 0`).
  - *Backing negative searches:* Predicate and `op_req_cap|op_is_sink|op_is_sanitizer` greps confirm M2 `mod_check.omni`, `m4/`, and `verifier.omni` (pure helper taking caller bitmask arguments) carry no second copy; `m2/decode.omni:121` (`m2d_op_meta`) already reads `table.omni`.
- **Designated Single Source & Rationale:** **`pure/isa/table.omni`** (`cap` bytes `+2..+3` via `isa_op_cap`, `flags` bits `1` & `2` via `isa_op_flag`). Replaces ~`119` lines (`L230-348`) of hand-transcribed `is_eq` accumulators in `verifier_engine.omni`.
- **Derivation Mechanism:** **(a) Runtime/build-time table lookup** in `verifier_engine.omni` using `isa_op_cap` and `isa_op_flag(..., 1)` / `isa_op_flag(..., 2)` gated on `ACCEPTED` bit 5.
- **Acid test (feasible):** Scratch-toggle a `cap` bit (`cap_lo`), `sink` bit (`flags & 2`), or `sanitizer` bit (`flags & 4`) on one active op row in `pure/isa/table.omni` with zero edits to `verifier_engine.omni`; rebuild `verifier.elf`; verify `1003` (`MISSING_CAP`), `1002` (`TAINT_SINK`), or taint clearing follows the mutated table row.

---

### R4 — Label ID Bounds & Non-Aliasing 4,096-Slot Mapping

- **Fact:** Valid bytecode `LABEL` definitions and branch targets require either `LBL` token `0x7EF0..=0x7EFF` (`32496..=32511`, mapped to slots `4080..=4095`) or signed `IMM` literal `0..=4079` (mapped directly to slots `0..=4079`). All negative `IMM`, `IMM >= 4080`, or non-label tokens are rejected (`2009` in verifier, exit `1` in M2); `LBL` and `IMM` never alias in the `4,096`-slot space.
- **Sites & backing searches:**
  - `pure/m2/labels.omni:13-32` — `m2lbl_max_slots() = 4096`, `m2lbl_imm_limit() = 4080`, `m2lbl_slot_of(packed)` (called by `m2/verify/mod_check.omni:116-168`, `m2/emit_multi.omni:204-219, 1034-1040`, `m2/emit_x86.omni:544, 620`, `m4/vm.omni:28, 138, 297-308`).
  - `pure/verifier_engine.omni:216-226, 370-400, 448-480, 671-720` — duplicates constants `32496..32511`, `0..4079`, `4080`, `4095` in `parse_operand_raw:475-477`, `op_label_slot:370-378`, `has_label:381-389`, `set_label:392-400`, `record_labels_from_ops:671-690`, `check_branch_labels:693-720` (on `ws[960..1023]`).
  - `pure/audit/decompile_rhs.omni:101-110` (`dc_label_id`), `pure/audit/decompile.omni:277-296` (`dc_any_branch_to`) — **Drift D4a** (separate decompiler policy; do not alter silently).
  - *Distinct surface rule (NOT a copy):* `pure/m3/{skim.omni:204, stmt.omni:146, inc/lower_body.omni:64-75}` enforce surface syntax `label 0..15` (`m3w_alloc_lbl`), which belongs to surface M3, not the M2 `4,096`-slot bytecode rule.
  - *Backing searches:* `4080|4079|4095|4096|m2lbl_slot_of|op_label_slot` grep + `in_range|m2u_in` grep.
- **Designated Single Source & Rationale:** **`pure/m2/labels.omni`**. It owns M2's D-3/D-8/D-11 `4,096`-slot arena mapping and already serves M2 (`mod_check`, `emit_multi`, `emit_x86`) and M4 VM.
- **Derivation Mechanism:** **(a) Shared helper extraction/linkage.** Extract pure label-rule helpers into a zero-arena slice or include `pure/m2/labels.omni` (with a thin pure predicate `m2lbl_slot_raw(is_lbl, is_imm, tok, ival)` / `m2lbl_imm_limit()` / `m2lbl_lbl_slot_base()` in `pure/m2/labels.omni`, avoiding pulling `m2/decode.omni` + `m2/arena.omni` if `m2lbl_byte_off` stays separate or if only `m2l_labels()` is stubbed/split into `labels_rules.omni` vs `labels.omni` — **concrete choice:** split pure predicate/constant functions `m2lbl_max_slots`, `m2lbl_imm_limit`, `m2lbl_slot_of_kind` cleanly inside `pure/m2/labels.omni` or a dedicated zero-dep `pure/m2/label_rules.omni` included by BOTH `labels.omni` and `verifier_engine.omni`, since `labels.omni:37` references `m2l_labels()` from `decode.omni`). Both `m2` and `verifier_engine` call the exact same slot-mapping & `imm_limit` functions (`parse_operand_raw` and `op_label_slot` derive `0..imm_limit-1` and `4080..max_slots-1` from them).
- **Acid test (feasible):** Scratch-change `m2lbl_imm_limit()` from `4080` to `4000` ONLY in the label-rule source with zero edits to `mod_check.omni` or `verifier_engine.omni`; rebuild; verify BOTH `m2.elf` (`rc=1`) and `verifier.elf` (`2009`) reject `IMM 4050` identically.

---

### R5 — Final-`RET` Rule and `FN_HDR fn_id` Bounds

- **Fact:** Every function's final decoded instruction must be `RET (0x7F4A = 32586)`, and every `FN_HDR (0x7EE3)` function ID `fn_idx` must satisfy `0 <= fn_idx <= 1023`.
- **Sites & backing searches:**
  - `pure/m2/verify/mod_check.omni:70` (`0 <= fid <= 1023`, plus `fn_cnt < 256`, `0 <= pcnt <= 32`, `icnt >= 1` — **D5a**), `:273` (`no_ret = 1 - m2u_eq(last_op, 32586)` → err `1`).
  - `pure/verifier_engine.omni:1020-1021` (`no_ret = (1 - is_eq(ws_get(w, 10), 3006)) * (1 - is_eq(last_op, 32586))` → err `2012`), `:1234, 1311` (`in_range(fn_idx, 0, 1023)` → err `3006`).
  - `scripts/external_verifier_check.py:61-63` — independent trace checker (checks `MOD_BEGIN`/`MOD_END` framing and linear/taint; checked as external oracle).
- **Designated Single Source & Rationale:** **`pure/m2/verify/mod_check.omni`** (or shared structural-rule helper module shared by `mod_check.omni` and `verifier_engine.omni`) for the scalar predicates (`m2v_fn_id_ok(fid)`, `m2v_is_term_op(op)`).
- **Derivation Mechanism:** **Hybrid (a) + (c):**
  - **(a)** Extract/share the pure scalar constants/predicates (`fn_id_max = 1023`, `term_op = 32586` / `isa_tbl` `RET`) so neither bounds literal nor terminator opcode is duplicated.
  - **(c) Test-pinning via permanent differential gate in `verify_step8_harness.sh` for the control-flow check wiring itself**, with explicit justification for why **(a)/(b) cannot eliminate the duplicated check site**: `verifier_engine.omni::verify_ins_body` and `mod_check.omni::m2v_check_fn_body` maintain completely distinct control-flow state machines and error-buffer ABIs (verifier tracks 224 SSA registers, borrow/linear state, fixed-point taint across labels, multi-error `(code, off)` output in `ws[8192..]`; M2 performs a single linear pass recording the first `u32` err/off in `m2v_state()` and checking acyclicity via Kahn's algorithm). Because checking `last_op == term_op` after an instruction loop is structural control flow rather than a table row, deriving a pure predicate `(a)` still leaves two loop sites, and generating two distinct full verifier loops from one template `(b)` would require building a bespoke code generator over two unrelated state machines. A permanent differential gate testing end-of-body non-`RET` (`JMP`, `ADD`) and boundary `fn_id` (`-1`, `0`, `1023`, `1024`, `0x7EFB`) against both `verifier.elf` and `m2.elf` pins both implementations to identical accept/reject behavior.
- **Acid test:** Scalar predicate bounds (`m2v_fn_id_ok`, `m2v_is_term_op`) **pass a source-only acid test** (scratch-mutate `1023 -> 511` on scalar predicate ONLY → both reject `fn_id=600`). Control-flow loop wiring **cannot pass a source-only acid test alone** and is classified **(c)** with the permanent differential gate.

---

### R6 — Canonical Fixed Result Type (`dc_op_fixed_ty`)

- **Fact:** Per-opcode canonical fixed return type (`TY_I64=32641`, `TY_TRIT=32643`, `TY_STR=32644`, `TY_BYTES=32646`, `TY_TVEC=32647`, `TY_ARENA=32650`), or `0` when polymorphic/multi-form (`CINT`, `CALL`, etc.), enforced by `dc_can_rhs:451-452` when deciding `RECONSTRUCTED (exact)` vs `APPROXIMATE`.
- **Sites & backing searches:**
  - `pure/isa/table.omni:5-131` — `isa_mn_rows_0..7` stores `tyb = ty - 0x7F80` (`1=i64, 3=trit, 4=str, 6=bytes, 7=tvec, 10=arena, 240..244=destless, 255=ret_ty`), plus unused byte `+6` in every `8 B` `isa_op_rows` entry.
  - `pure/audit/decompile_rhs.omni:417-425` — `dc_op_fixed_ty(op)`.
  - `pure/m3/inc/lower_body.omni` — M3 emits `ty_tok` explicitly per expression/builtin (`:476` for `RSP_HTML`, etc.).
- **Discovered drifts (BLOCKER BEFORE PR 6):** **D6a** (`0x7F6F RSP_HTML`: `dc_op_fixed_ty` returns `32641` vs `table.omni:114` & `lower_body.omni:476` `32644`), **D6b** (`0x7F66, 0x7F67, 0x7F69, 0x7F6A`), and **D6c** (`0x7F04, 0x7F06, 0x7F08, 0x7F09, 0x7F36, 0x7F56..0x7F59, 0x7F5D`). **Must be triaged in a separate issue/PR before Part B PR 6 runs.** (Behavior-preserving PR 6 requires every slot's effective `dc_op_fixed_ty` output on corpus/accepted ops to match post-triage truth cleanly.)
- **Designated Single Source & Rationale:** **`pure/isa/table.omni`**.
  - `isa_mn_rows` has alias rows (`CINT`/`MOV`, `SCAT`/`SCONC`, multi-row conventions) AND `0x7F06 (CINT)` intentionally stays `0` (polymorphic) in the decompiler even though `mn_rows` tags it `tyb=1` (`i64`). Storing the canonical **`fixed_ty_byte`** in currently-unused byte **`+6`** of `isa_op_rows_0..7` (`0` = polymorphic/exempt, `1..10` = `0x7F80 + tyb`) gives an $O(1)$ lookup per op inside the existing `2,320 B` table (no layout change, zero search across mnemonic aliases, explicit distinction between `CINT`-polymorphic `0` and fixed `1..10`), and **still puts the single authoritative metadata row in `table.omni`**.
- **Derivation Mechanism:** **(a) Runtime/build-time table lookup.** `pure/audit/decompile.omni` (built via `build_pure_driver.sh` / `omnic_pure.elf`, which expands includes) includes `../isa/table.omni`, caches `isa_tbl` in the decompiler workspace (or builds once per invocation), and `dc_op_fixed_ty(op)` reads byte `+6` from `table.omni` (`0` → `0`, `b > 0` → `32640 + b`). Note: check M2 `m2_standalone` vs `decompile` window (`decompile.omni` is already built through `m3.elf + verifier.elf + m2.elf` cleanly).
- **Acid test (feasible):** Scratch-mutate ONLY byte `+6` of one active row (e.g. change `IADD` `0x7F10` from `1` (`i64`) to `3` (`trit`)) in `pure/isa/table.omni` with zero edits to `decompile_rhs.omni`; rebuild `audit_decompile.elf`; run on a golden seed containing `IADD ... -> TY_I64` and observe `// Status: RECONSTRUCTED (exact)` flip to `// Status: APPROXIMATE` (reason code `FIXTY`).

---

### R7 — `dc_can_rhs` Whitelist of M3-Emittable Canonical Forms

- **Fact:** Which `(op, nops, operand_kinds, dest_ty)` instruction shapes `audit_decompile.elf` marks `RECONSTRUCTED (exact)` because `omnic_pure.elf` (`pure/m3/`) compiles the reconstructed Omni expression back to the matching token sequence.
- **Sites & backing searches:**
  - `pure/audit/decompile_rhs.omni:427-454` (`dc_can_rhs`) and `:165-415` (`dc_rhs_*` emitters).
  - `pure/audit/decompile.omni:69, 78-98` (`dc_emit_single`).
  - `pure/m3/{inc/lower_body.omni, expr.omni, stmt.omni}` — `59` emitted opcodes, `57` decoded `hlow` builtin hashes, special-cased forms (`[...]` tritvec `UNPACK5`, unary `-`/`~`/`not`, `ld_u*`/`st_u*` `IGET`/`ISET` `-1..-8`, `route` implicit `req` on `req.param` / status `200` on `respond_*`, `ret_slice`).
- **Discovered drifts (BLOCKER BEFORE PR 7):** **D7a** (`0x7F56, 0x7F57, 0x7F59, 0x7F5D`), **D7b** (`0x7F21` `bnot`), **D7c** (`0x7F31` `unpack5`), **D7d** (`0x7F3F` `tlogic`), **D7e** (`0x7F6D` `REQ_PARAM` arity 2 vs 1), **D7f** (`0x7F0F` `ISET` `st_u8`). Must be triaged before PR 7 pins the whitelist cleanly.
- **Designated Single Source & Rationale:** **`pure/m3/` (`expr.omni`, `stmt.omni`, `inc/lower_body.omni`) is the behavioral truth** for what M3 actually compiles cleanly; `pure/audit/decompile_rhs.omni::dc_can_rhs` remains the explicit decompiler admission whitelist.
- **Derivation Mechanism:** **(c) Test-pinning via permanent differential round-trip gate** in `scripts/verify_step8_harness.sh` — **explicit justification for why (a) and (b) do not apply:**
  1. *Why **(a)** (compute at build/run time from M3 data) is not feasible:* M3's emittable grammar is not a flat data table; it is distributed across recursive-descent precedence parsing (`expr.omni`), statement lowering (`stmt.omni`), `57` hashed identifier dispatches, implicit-argument host `route` rewrites (`lower_body.omni:460-478` inserting `req` or HTTP `200`), chunked `[...]` `ISET` expansion (`:488-545`), and packed load/store specifiers (`:686`). Computing reversibility from M3 at decompiler runtime would require embedding M3's parser and lowerer into `audit_decompile.elf` (exceeding complexity/word budgets and violating unidirectional TCB/audit separation).
  2. *Why **(b)** (generate `dc_can_rhs` from M3 source) is not feasible:* There is no declarative grammar file in `pure/m3/` from which to codegen `dc_can_rhs`; any generator would itself be a bespoke static analyzer over M3's hash table and `if_pos` lowering chains — more fragile than the code it generates and prohibited from introducing Python into `zero_rust_rebuild.sh`.
  3. *Why **(c)** closes the drift vector:* A permanent Pure-Omni differential round-trip gate in `scripts/verify_step8_harness.sh` exercises every whitelisted `(op, shape)` in `dc_can_rhs` through `audit_decompile.elf` → `m3.elf` / `omnic_pure.elf` compile → structural/opcode verification (and checks corpus negative mutants for non-whitelisted shapes), failing CI immediately whenever `dc_can_rhs` and `pure/m3/` drift apart again.
- **Acid test:** **Not derived (Mechanism (c))** — mutating `pure/m3/inc/lower_body.omni` alone does not auto-update `dc_can_rhs`; instead, the permanent differential round-trip gate in `verify_step8_harness.sh` catches any divergence automatically.

---

### R8 — Frozen / ABI Structural Constants (Documented Cross-Pins, Already Enforced)

Checked during the sweep so no silent constant copies go uninventoried:
- **Framing tokens `0x7EE0..=0x7EEF`, register band `0x7E00..=0x7EBF`, param band `0x7EC0..=0x7EDF`, type tokens `0x7F80..=0x7F8B`, taint tokens `0x7F90..=0x7F92`:** Stated in `m2/decode.omni`, `isa/table.omni`, `isa/encode.omni`, `verifier_engine.omni`, `audit/{decompile_rhs,diff_summary}.omni`, `m3/{skim,stmt,inc/lower_body}.omni`, `harness/m2_lint.omni`, `scripts/external_verifier_check.py`. Mechanism: **(c)** wire ABI contract, cross-pinned continuously by `m2-fixed-point`, `m3-modular`, `b2-pure-verifier`, `audit-view`, and `external_verifier_check.py`.
- **Capability bitmask IDs & names (`1,2,4,8,16,32,64,128,256,512`):** `m3/skim.omni:74-89`, `audit/decompile.omni:391-405`, `audit/diff_summary.omni:188-204`. Mechanism: **(c)** pinned by `audit-view` & `m3-modular` parity (candidate future cleanup if cap-name formatting ever consolidates into `table.omni` or `sys_util`).
- **Surface M3 label limit (`16`, allocator `1..15, 0`) & SSA limit (`192`/`120`):** `spec/m2/DECISIONS.md:40`, `m3/DECISIONS.md`, `m3/inc/lower_body.omni:64-75`, `harness/m2_lint.omni`. Surface/compiler invariants, distinct from M2 bytecode limits (R4).

---

## 4. Complete 47-Slot Unassigned / `RESERVED` Opcode Audit (`0x7F00..=0x7F7F`)

Every slot in `0x7F00..=0x7F7F` outside the `81` known opcodes must agree across:
1. **`pure/isa/table.omni::isa_op_rows_0..7`:** `cap_lo=0, cap_hi=0, flags=0 // RESERVED (0x7FXX)` (`(flags >> 5) & 1 == 0`) + zeroed mnemonic row or none in `isa_mn_rows_0..7`.
2. **`pure/m2/verify/mod_check.omni::m2v_ins_min_ops`:** returns `-1` (PR 1+: reads `ACCEPTED` bit 5 `== 0`).
3. **`pure/verifier_engine.omni::op_is_known`:** returns `0`, rejected `3001` (PR 1+: reads `ACCEPTED` bit 5 `== 0`).
4. **`pure/audit/decompile_rhs.omni`:** excluded from `dc_can_rhs` (`APPROXIMATE` / `LITERAL_OP` if ever reached).
5. **`spec/`:** all `47` slots documented here (`18` Step-15 + `3` B3-F1 + `24` B5-F2 + `2` B7-F5c = **`47`**).

*Current state at `main @ 8aec4e4` (`42` slots still carry stale non-zero `flags` in `table.omni`, `16` still carry stale `cap != 0`, and `3` still carry stale `flags & 4`; zero active `isa_mn_rows` exist for any of the `47` slots):*

| # | Slot (`idx`) | Hex (`Dec`) | Origin Group | `table.omni` Line & Current `(cap, flags)` | `verifier_engine` Current State `(known, min, dl, cap, snk, san)` | `mod_check` (`min`) | PR 1 Target State |
|---:|---:|---|---|---|---|---|---|
| 1 | 7 | `0x7F07` (`32519`) | Step-15 | `:143` `(cap=0, flags=1)` | `(0, 0, 0, 0, 0, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 2 | 10 | `0x7F0A` (`32522`) | Step-15 | `:146` `(cap=0, flags=1)` | `(0, 0, 0, 0, 0, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 3 | 11 | `0x7F0B` (`32523`) | Step-15 | `:147` `(cap=0, flags=1)` | `(0, 0, 0, 0, 0, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 4 | 12 | `0x7F0C` (`32524`) | Step-15 | `:148` `(cap=0, flags=1)` | `(0, 0, 0, 0, 0, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 5 | 25 | `0x7F19` (`32537`) | B5-F2 | `:167` `(cap=0, flags=1)` | `(0, 2, 0, 0, 0, 0)` *(D2a)* | `-1` | `cap=0, flags=0 // RESERVED` |
| 6 | 26 | `0x7F1A` (`32538`) | B5-F2 | `:168` `(cap=0, flags=1)` | `(0, 2, 0, 0, 0, 0)` *(D2a)* | `-1` | `cap=0, flags=0 // RESERVED` |
| 7 | 27 | `0x7F1B` (`32539`) | B5-F2 | `:169` `(cap=0, flags=1)` | `(0, 2, 0, 0, 0, 0)` *(D2a)* | `-1` | `cap=0, flags=0 // RESERVED` |
| 8 | 28 | `0x7F1C` (`32540`) | B5-F2 | `:170` `(cap=0, flags=1)` | `(0, 2, 0, 0, 0, 0)` *(D2a)* | `-1` | `cap=0, flags=0 // RESERVED` |
| 9 | 29 | `0x7F1D` (`32541`) | B5-F2 | `:171` `(cap=0, flags=1)` | `(0, 2, 0, 0, 0, 0)` *(D2a)* | `-1` | `cap=0, flags=0 // RESERVED` |
| 10 | 50 | `0x7F32` (`32562`) | B5-F2 | `:204` `(cap=16, flags=1)` | `(0, 0, 0, 16, 0, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 11 | 51 | `0x7F33` (`32563`) | B5-F2 | `:205` `(cap=16, flags=3)` | `(0, 0, 0, 16, 1, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 12 | 56 | `0x7F38` (`32568`) | B5-F2 | `:210` `(cap=16, flags=3)` | `(0, 0, 0, 16, 1, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 13 | 58 | `0x7F3A` (`32570`) | Step-15 | `:212` `(cap=0, flags=1)` | `(0, 0, 0, 0, 0, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 14 | 59 | `0x7F3B` (`32571`) | Step-15 | `:213` `(cap=0, flags=1)` | `(0, 0, 0, 0, 0, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 15 | 60 | `0x7F3C` (`32572`) | Step-15 | `:214` `(cap=0, flags=1)` | `(0, 0, 0, 0, 0, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 16 | 61 | `0x7F3D` (`32573`) | Step-15 | `:215` `(cap=0, flags=1)` | `(0, 0, 0, 0, 0, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 17 | 62 | `0x7F3E` (`32574`) | Step-15 | `:216` `(cap=0, flags=1)` | `(0, 0, 0, 0, 0, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 18 | 71 | `0x7F47` (`32583`) | B5-F2 | `:231` `(cap=0, flags=0)` | `(0, 0, 1, 0, 0, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 19 | 73 | `0x7F49` (`32585`) | B5-F2 | `:233` `(cap=0, flags=0)` | `(0, 0, 1, 0, 0, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 20 | 75 | `0x7F4B` (`32587`) | Step-15 | `:235` `(cap=0, flags=1)` | `(0, 0, 0, 0, 0, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 21 | 76 | `0x7F4C` (`32588`) | Step-15 | `:236` `(cap=0, flags=1)` | `(0, 0, 0, 0, 0, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 22 | 77 | `0x7F4D` (`32589`) | Step-15 | `:237` `(cap=0, flags=1)` | `(0, 0, 0, 0, 0, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 23 | 78 | `0x7F4E` (`32590`) | Step-15 | `:238` `(cap=0, flags=1)` | `(0, 0, 0, 0, 0, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 24 | 79 | `0x7F4F` (`32591`) | B5-F2 | `:239` `(cap=0, flags=0)` | `(0, 0, 1, 0, 0, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 25 | 81 | `0x7F51` (`32593`) | B7-F5c | `:247` `(cap=0, flags=0)` *(B7 RESERVED)* | `(0, 2, 0, 0, 0, 0)` *(D2a, D2b)* | `-1` | Already `RESERVED` (`flags=0`) |
| 26 | 82 | `0x7F52` (`32594`) | Step-15 | `:248` `(cap=0, flags=1)` | `(0, 0, 0, 0, 0, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 27 | 83 | `0x7F53` (`32595`) | Step-15 | `:249` `(cap=0, flags=1)` | `(0, 0, 0, 0, 0, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 28 | 84 | `0x7F54` (`32596`) | Step-15 | `:250` `(cap=0, flags=1)` | `(0, 0, 0, 0, 0, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 29 | 85 | `0x7F55` (`32597`) | B7-F5c | `:251` `(cap=0, flags=0)` *(B7 RESERVED)* | `(0, 1, 0, 0, 0, 0)` *(D2a, D2b)* | `-1` | Already `RESERVED` (`flags=0`) |
| 30 | 90 | `0x7F5A` (`32602`) | B3-F1 | `:256` `(cap=0, flags=5)` | `(0, 0, 0, 0, 0, 1)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 31 | 91 | `0x7F5B` (`32603`) | B3-F1 | `:257` `(cap=0, flags=5)` | `(0, 0, 0, 0, 0, 1)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 32 | 92 | `0x7F5C` (`32604`) | B3-F1 | `:258` `(cap=0, flags=5)` | `(0, 0, 0, 0, 0, 1)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 33 | 94 | `0x7F5E` (`32606`) | Step-15 | `:260` `(cap=0, flags=1)` | `(0, 0, 0, 0, 0, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 34 | 95 | `0x7F5F` (`32607`) | Step-15 | `:261` `(cap=0, flags=1)` | `(0, 0, 0, 0, 0, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 35 | 96 | `0x7F60` (`32608`) | B5-F2 | `:268` `(cap=2, flags=1)` | `(0, 0, 0, 2, 0, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 36 | 97 | `0x7F61` (`32609`) | B5-F2 | `:269` `(cap=2, flags=3)` | `(0, 0, 0, 2, 1, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 37 | 98 | `0x7F62` (`32610`) | B5-F2 | `:270` `(cap=2, flags=1)` | `(0, 0, 0, 2, 0, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 38 | 113 | `0x7F71` (`32625`) | B5-F2 | `:291` `(cap=4, flags=1)` | `(0, 0, 0, 4, 0, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 39 | 114 | `0x7F72` (`32626`) | B5-F2 | `:292` `(cap=512, flags=1)` | `(0, 0, 0, 512, 0, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 40 | 116 | `0x7F74` (`32628`) | B5-F2 | `:294` `(cap=512, flags=3)` | `(0, 0, 0, 512, 1, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 41 | 117 | `0x7F75` (`32629`) | B5-F2 | `:295` `(cap=256, flags=3)` | `(0, 0, 0, 256, 1, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 42 | 119 | `0x7F77` (`32631`) | B5-F2 | `:297` `(cap=8, flags=3)` | `(0, 0, 0, 8, 1, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 43 | 120 | `0x7F78` (`32632`) | B5-F2 | `:298` `(cap=8, flags=3)` | `(0, 0, 0, 8, 1, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 44 | 121 | `0x7F79` (`32633`) | B5-F2 | `:299` `(cap=8, flags=3)` | `(0, 0, 0, 8, 1, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 45 | 122 | `0x7F7A` (`32634`) | B5-F2 | `:300` `(cap=8, flags=3)` | `(0, 0, 0, 8, 1, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 46 | 123 | `0x7F7B` (`32635`) | B5-F2 | `:301` `(cap=8, flags=3)` | `(0, 0, 0, 8, 1, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |
| 47 | 124 | `0x7F7C` (`32636`) | B5-F2 | `:302` `(cap=64, flags=3)` | `(0, 0, 0, 64, 1, 0)` | `-1` | `cap=0, flags=0 // RESERVED` |

*(Count check: `18` Step-15 + `3` B3-F1 + `24` B5-F2 + `2` B7-F5c = **`47`**; `81 + 47 = 128`. Notice again why bit 5 `ACCEPTED = 32` is necessary: known accepted opcode `0x7F46 (32582)` at `table.omni:230` has lower 5 flag bits `0`, so after PR 1 its row carries `flags = 32` while all `47` `RESERVED` rows carry `cap = 0, flags = 0`.)*

---

## 5. Part B Implementation Order & Expected `bootstrap/MANIFEST.md` Pin Footprint

Each Part B PR owns **one rule family**, proves zero behavior change (full 128-op verifier/M2 acceptance sweep diff + decompiler corpus stamps + piped `--campaign-all` diff at `12` finds `0/0/12`), runs its acid test (when mechanism is **(a)**) on scratch builds, verifies `scripts/zero_rust_rebuild.sh` clean-room rebuild, leaves the **5 frozen hashes untouched**, and updates `bootstrap/MANIFEST.md` only for the predicted seed/binary artifacts.

> **Prerequisite triage gates:**
> - **D6a/D6b/D6c** must be triaged and resolved before **PR 6 (`dc_op_fixed_ty`)** can preserve existing behavior.
> - **D7a–D7f** must be triaged and resolved before **PR 7 (`dc_can_rhs` differential gate)** is added so the permanent round-trip gate starts green cleanly.

| Order | PR & Rule Family | Summary of Code / Gate Changes | Predicted Moved Pins | Predicted Unmoved Pins |
|---|---|---|---|---|
| **PR 1** | **R1 — `op_is_known` + 47 `RESERVED` Completion** | Add `ACCEPTED` bit 5 (`32`) to the `81` known rows in `pure/isa/table.omni`, zero `cap=0, flags=0 // RESERVED` on all `47` slots, update `spec/trusted_core.md:25`; link `table.omni` into `pure/verifier_engine.omni` so `op_is_known` reads bit 5; make `mod_check.omni::m2v_ins_min_ops` check `isa_op_flag(..., 5)`; rebuild via `zero_rust_rebuild.sh` (`S1==S2==S3`). Run scratch acid test on `table.omni` bit 5. | `pure/m2/m2.omni_bc`, `bootstrap/m2.elf` (`S1==S2==S3`), `pure/verifier_standalone.omni_bc`, `verifier.elf` (rebuilt via new `m2.elf`), `bootstrap/MANIFEST.md` | `bootstrap/m3.elf`, `m3*.omni_bc`, `elf_link.omni_bc`, `m4*.omni_bc`, `m4.elf`, all 5 frozen hashes, `omnic_pure.elf` fixed point |
| **PR 2** | **R2 — `op_min_ops` & `op_has_dest`** | Store `min_ops` (`0..4`) in byte `+5` of `isa_op_rows_0..7` in `pure/isa/table.omni` (`0` on `RESERVED`); point both `mod_check.omni::m2v_ins_min_ops` (returning `-1` when `!ACCEPTED`) and `verifier_engine.omni` (`op_min_ops`, `op_has_dest` from `flags & 1` + `ACCEPTED`) at `table.omni`; delete `op_is_destless` duplicate list. Run scratch acid tests on byte `+5` and bit `0`. | `pure/m2/m2.omni_bc`, `bootstrap/m2.elf` (`S1==S2==S3`), `pure/verifier_standalone.omni_bc`, `verifier.elf`, `bootstrap/MANIFEST.md` | `bootstrap/m3.elf`, `m3*.omni_bc`, `elf_link.omni_bc`, `m4*.omni_bc`, `m4.elf`, all 5 frozen hashes |
| **PR 3** | **R3 — `op_is_sanitizer`, `op_req_cap`, `op_is_sink_raw`** | Replace `op_req_cap_0/1`, `op_is_sink_raw_0/1`, `op_is_sanitizer` in `pure/verifier_engine.omni` with direct `table.omni` reads (`isa_op_cap`, `isa_op_flag(..., 1)`, `isa_op_flag(..., 2)` gated on `ACCEPTED` bit 5). Run scratch acid tests on `cap`, `sink`, and `sanitizer`. | `pure/verifier_standalone.omni_bc`, `verifier.elf`, `bootstrap/MANIFEST.md` | `bootstrap/m2.elf`, `pure/m2/m2.omni_bc`, `bootstrap/m3.elf`, `m4.elf`, all `.omni_bc` outside verifier, all 5 frozen hashes |
| **PR 4** | **R4 — Label ID Bounds & 4,096 Non-Aliasing Slots** | Extract pure label-rule bounds/slot helpers from `pure/m2/labels.omni` into a zero-arena helper slice (keeping `m2` lint L-4 clean and avoiding `decode`/`arena` coupling in verifier) and derive `verifier_engine.omni` (`parse_operand_raw`, `op_label_slot`, label bitset bounds) from them. Run scratch acid test on `imm_limit` (`4080 -> 4000`). | `pure/verifier_standalone.omni_bc`, `verifier.elf` (plus `pure/m2/m2.omni_bc`, `bootstrap/m2.elf` if `labels.omni` source changes), `bootstrap/MANIFEST.md` | `bootstrap/m3.elf`, `m4.elf`, all `.omni_bc` outside verifier/m2, all 5 frozen hashes |
| **PR 5** | **R5 — Final-`RET` & `FN_HDR fn_id` Bounds + Diff Gate** | Share scalar `fn_idx` / terminator-op bounds predicates between `mod_check.omni` and `verifier_engine.omni` + add permanent differential gate **(c)** in `scripts/verify_step8_harness.sh` for loop wiring (`verify_step8_harness.sh` pass total increments from `76`). Run acid test on scalar predicate; document differential gate for loop wiring. | `pure/verifier_standalone.omni_bc`, `verifier.elf`, `pure/m2/m2.omni_bc`, `bootstrap/m2.elf`, `bootstrap/MANIFEST.md` | `bootstrap/m3.elf`, `m4.elf`, all 5 frozen hashes |
| **PR 6** | **R6 — `dc_op_fixed_ty` *(After D6 Triage)*** | Populate canonical fixed-type byte (`0` polymorphic, else `ty - 0x7F80`) in unused byte `+6` of `isa_op_rows_0..7` in `pure/isa/table.omni`; include `table.omni` in `pure/audit/decompile.omni` so `dc_op_fixed_ty` reads byte `+6`. Run scratch acid test on byte `+6` verifying `audit_decompile.elf` flips `RECONSTRUCTED`/`APPROXIMATE`. | `pure/m2/m2.omni_bc`, `bootstrap/m2.elf`, `pure/verifier_standalone.omni_bc`, `verifier.elf`, `bootstrap/MANIFEST.md` *(table edits move pins for seeds that link `table.omni`; `audit_decompile.elf` is rebuilt from source)* | `bootstrap/m3.elf`, `m4.elf`, all 5 frozen hashes |
| **PR 7** | **R7 — `dc_can_rhs` Differential Round-Trip Gate *(After D7 Triage)*** | Keep `dc_can_rhs` in `pure/audit/decompile_rhs.omni` as **(c)** and add permanent differential round-trip gate in `scripts/verify_step8_harness.sh` testing every whitelisted `(op, arity/shape)` via `audit_decompile.elf` + `m3.elf`/`omnic_pure.elf` (`PASS` total increments). | **Zero bootstrap/seed pin moves** (`decompile_rhs.omni` and `verify_step8_harness.sh` are outside pinned `.omni_bc` seeds). | **All** `bootstrap/MANIFEST.md` pins & all 5 frozen hashes |
