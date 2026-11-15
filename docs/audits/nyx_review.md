# Nyx review: `milestone2_design.md` (Milestone 2, two-pass u16 → ELF64 in Omni)

Reviewer: Nyx Fixpoint. Review only. Nothing was committed, pushed, branched, PR'd, or commented.
Tree: `/workspace/audit-m2` @ `origin/isa/multibyte-loads` `8160842` (read-only; `git status` clean after the review).
`/workspace/omni` was not touched. C1 was not implemented. Zero crates: every probe is a single `rustc` file that `#[path]`-includes `omni_rust_bootstrap.rs`.
Probes and artifacts are in `/tmp/m2probe/nyx/`: `enc.rs`, `imod.rs`, `lbl.rs`, `lbldiv.rs`, `budget.rs`, `sizemodel.rs`, `vmcost.rs`, `cliff.rs`, `fp.rs`, `proto/m2proto.omni`.
Refs: `A:` = `omni-workspace/crates/omni-aot/src/lib.rs`, `M:` = `omni_rust_bootstrap.rs`, `ISA:` = `omni-workspace/crates/omni-isa/src/lib.rs`, `FE:` = `omni-workspace/crates/omni-lang/src/compiler/mod.rs`.
Caleb hasn't decided the 6 open questions (§9 of the design). §6 below gives options only; I don't assume any answer.

---

## 0. Verdict in one screen

- **Encodings are solid.** I checked 55 encoding/size rows against the code and probed 51 of them by actually emitting bytes with `compile_pure_u16_to_x86_64_elf_with_args`, plus objdump for the non-trivial ones. **There are 2 encoding mismatches, both in the pack5 `IGET` rows. Every other opcode size and byte string matches.** There are also 3 factual mismatches outside the encodings (A≡M identity, the Stage-1 label count, and the output-size estimate). See §1.
- **The budgets are under-estimated, and two places are at the cap today.**
  - The verifier's `omni_verify_module` uses 14 of the 15 allocatable labels.
  - `handle_dest` uses 134 SSA, over the doc's own 120 target.
  - My prototype `m2_main` already uses 12 labels with no error paths.
  - A single dispatch function can't hold more than 3 `match_trit`s.
  - `read_operand` measures 84 SSA, against the doc's estimate of ≤ 60. See §2.
- **The capacity arithmetic holds, but the framing is wrong in two places.**
  - 28,672 words is the *single-function native read window* (`0xE000` B), not "0x00000 up to 0x10000".
  - The ~129 KB output cap applies to single-function binaries such as Stage-1, not to M2, which runs in multi-function mode.
  - I reproduced both cliffs empirically:
    - **Input over 28,672 words is silently truncated, giving a wrong ELF with exit 0.**
    - **Output over 131,072 B crashes with SIGBUS.** Between 130,288 and 131,072 B it overwrites SSA slots.
  - The doc's 140–170 KB M2 output estimate is probably about 2× high. My model gives about 70–100 KB. See §3.
- **The ABI is under-specified in ways that break byte-exactness or VM/native parity.**
  - Native code has no way to learn `nwords`.
  - The label-key semantics differ between the Rust emitter and Stage-1 (3 divergences measured).
  - The entry-param channel is contradictory.
  - The `ret_slice` return channel is undefined.
  - Native IMOD returns stale `rdx`, measured `1234 % 0 = 1234`.
  - Eager-arg misuse passes the Rust verifier.
  - See §4.
- **What blocks a fixed point** (§5): F1 (no Rust multi-function reference), the `nwords` channel, and the label-key decision. None of the three violates an invariant.

---

## 1. Task 1: per-opcode encodings vs the real emitter

Method:
- `enc.rs` assembles one-instruction programs (`assemble_omni_text_to_u16`, `M:1952`) and compiles them with the real emitter. It then slices out the instruction bytes: non-bytes programs `[428, len-51)`, bytes programs `[429, len-31)`.
- The base programs measure 479 B and 460 B, which confirms the doc's preamble and RET totals.
- `objdump -D -b binary -mi386:x86-64` confirms the semantics of every multi-byte template.
- A≡M check: for `A:102-659` vs `M:13934-14491`, and for the Stage-1 builder `A:1590-2199` vs `M:15422-16031`, `diff` is empty.

Notation: `P` = param (r12-r15), `R` = SSA slot, `i` = imm, `S` = store (7).

### 1.1 Helpers, preamble, ELF

| item | doc bytes | emitter bytes (measured) | file:line | status |
|---|---|---|---|---|
| load Param k≤3 → rax/rcx | `4C 89 E0+8k` / `E1+8k`, 3 | `4C 89 E0/E8/F0/F8`, `4C 89 E1/E9/F1/F9`, 3 | A:227-238 | MATCH |
| load Param k≥4 | nothing, 0 | 0 bytes. **The store still runs (7 B), so it stores stale rax** (`MOVE $4` = `48 89 85 F8 FF FF FF`) | A:252 (`_ => {}`) | MATCH (+caveat) |
| load Reg | `48 8B 85/8D disp32`, 7 | `48 8B 85 F8 FF FF FF` (`mov rax,[rbp-8]`) | A:239-243 | MATCH |
| load Imm/Trit | `48 B8/B9 imm64`, 10 | `48 B8 05 00…`, 10 | A:244-251 | MATCH |
| store rax | `48 89 85 disp32`, 7 | same | A:256-260 | MATCH |
| frame | `55 48 89 E5 48 81 EC 00 00 03 00`, 11 | same; objdump `sub rsp,0x30000` | A:174-178 | MATCH |
| non-bytes defaults | 37 | 10+10+10+7 = 37 | A:212-218 | MATCH |
| CLI parser | 65 ×4 | 65 (summed per line) | A:120-141, A:219-222 | MATCH |
| non-bytes prologue | 308 | base ELF 479 = 120+308+10+41 | A:174-222 | MATCH |
| bytes defaults | 37 | 7+10+10+10 | A:181-187 | MATCH |
| open/read/close | 56 | 56; objdump: `mov edx,0xe000` → `[rbp-0x2e000]`. **The read count is discarded** (`mov eax,3` overwrites it) | A:189-207 | MATCH (+caveat §4.2) |
| bytes prologue | 299 | base ELF 460 = 120+299+10+31 | A:180-210 | MATCH |
| ELF64 + 1 PT_LOAD | 120, fields per §2.4 | Hex dump matches every field. `p_filesz`=`p_memsz`=0x1DF (479) for the base. e_shentsize/shnum/shstrndx = 0 (not listed in the doc; harmless) | A:631-655 | MATCH |
| rel32 | `target-(patch+4)`, code-relative | JMP back `E9 FB FF FF FF`, JMP fwd `E9 00 00 00 00` | A:621-629 | MATCH |

### 1.2 Opcodes

| opcode | doc bytes | emitter bytes (measured, incl. S) | file:line | status |
|---|---|---|---|---|
| 7F00 NOP / 7F05 DROP | 0 | 0 / 0 | A:296 | MATCH |
| 7F06 CINT imm | L0+S = 17 | 17 | A:297-300 | MATCH |
| 7F04 MOVE / 7F02 BORROW_R / 7F03 BORROW_W (P) | L0+S | 10 / 10 / 10 | A:297-300 | MATCH |
| 7F09 CTRIT +1 | L0+S | 17 | A:297-300 | MATCH |
| 7F01 ARENA, 7F57-59 SAN_* | L0+S | same arm in code; not probed (cap/taint setup) | A:297-300 | MATCH (code only) |
| 7F0D ANEW | `45 31 DB`+S = 10 | `45 31 DB 48 89 85 F8 FF FF FF` (stores **stale rax**) | A:301-304 | MATCH |
| 7F0E IGET generic | L1+8+S | 18 (P idx), 25 (imm idx); `movzx eax,[rbp+rax*2-0x2e000]` | A:345-347 | MATCH |
| 7F0E IGET pack5, imm idx | 10+S | 17 for idx ≥ 0. **`IGET %anew, -1` takes the generic input-read path (25 B)** because of the `*idx >= 0` guard. The doc omits this condition. | A:310-311 | **MISMATCH** (missing condition) |
| 7F0E IGET pack5, dyn idx | 6+n(20+L1)+S | Empty ANEW: 13 (n=0). After a 3-byte ISET: **358 B = 3 + 15×23 + 3 + 7, so n = n_trits = 5×bytes, not bytes.** The per-iteration cost is 20+L1 only for byte maps; the trit-vec path uses `emit_extract_pack5_trit_x86` (24–37 B). | A:323-341, A:264-292, A:41 | **MISMATCH** (n undefined; tv case wrong) |
| 7F0F ISET -4 (3-op) | L2+12+D | 32 (imm + 12 + 10); objdump `mov [rbp+r11-0x20000],eax; add r11,4` | A:350-352 | MATCH |
| 7F0F ISET -8 (3-op) | L2+12+D | 25 (P) | A:353-355 | MATCH |
| 7F0F ISET -9 (4-op) | L2+L3+8+D | 24 (P,P); `mov [rbp+rcx-0x20000],rax` | A:356-359 | MATCH |
| 7F0F ISET bytes | Σ(L+11)+D | 73 for 3 imm bytes = 3×21+10; 24 for one P | A:360-364 | MATCH |
| ISET dest suffix D | 10 | `4C 89 D8` + store | A:366-369 | MATCH |
| 7F50 SLEN | 3+S | 10, `mov rax,r11` (**ignores its operand**) | A:371-374 | MATCH |
| 7F10/11/1E/1F/20 | L0+L1+3+S | 16 each (P,P); 41 = CINT + IADD R,R | A:429-441 | MATCH |
| 7F12 IMUL | +4 | 17 | A:435 | MATCH |
| 7F13 / 7F14 IDIV / IMOD | 54 / 57 (P,P) | 54 / 57; template disassembled (see §4.6 for the stale-rdx bug) | A:442-450 | MATCH |
| 7F15 INEG / 7F28 TNOT | L0+3+S | 13 / 13 | A:451-455 | MATCH |
| 7F16 IABS | 20 | 20 | A:456-460 | MATCH |
| 7F21 BNOT | L0+3+S | 13 | A:461-465 | MATCH |
| 7F22 SHL / 7F23 | `48 D3 E0/F8` | 16 / 16. **ISA calls 7F23 `OP_7F23_SHR` (ISA:148) but the bytes are `sar`.** The bytes match the doc. | A:466-471 | MATCH (naming note) |
| 7F24-27 CEQ/CNE/CLT/CGT | 27 | 27 each, setcc 94/95/9C/9F | A:472-488 | MATCH |
| 7F17 IMIN / 7F29 TAND | `…4F C1`, 20 | 20 / 20 (cmovg ⇒ min) | A:489-494 | MATCH |
| 7F18 IMAX / 7F2A TOR | `…4C C1`, 20 | 20 / 20 | A:495-500 | MATCH |
| 7F2B TXOR | +7 | 20 | A:501-506 | MATCH |
| 7F2C TMAJ | 41 | 41 | A:507-522 | MATCH |
| 7F2D TCMP | 33 | 33 | A:523-536 | MATCH |
| 7F3F TLOGIC (non-pack5) / 7F35 EGATE | L0+19+S | 29 (P). EGATE is the same arm; not probed (AI cap) | A:404-428 | MATCH |
| 7F30/31/34 pack5 | varies / reject | static-map driven | A:375-403 | MATCH (agree: reject in v1) |
| 7F40 LABEL | 0 | 0 | A:537-541 | MATCH |
| 7F41 JMP | 5 | 5. **0 bytes if the operand isn't a label/imm** (`if let Some`) | A:542-548 | MATCH (+caveat) |
| 7F42 BR_TRIT | L0+21 | 24 (P) | A:549-567 | MATCH |
| 7F43 / 7F44 BR_POS / BR_NEG | L0+10 | 13 / 13 | A:568-576 | MATCH |
| 7F45 SEL_TRIT | L0..L3+25+S | 65 (P,i,i,i), 44 (P,P,P,P) | A:577-586 | MATCH |
| 7F4A RET non-bytes | L0+41 | 51 (imm), 44 (P) | A:599-614 | MATCH |
| 7F4A RET bytes | 31 | 31. **Ignores its operand**; always writes `[rbp-0x20000, +r11)` | A:588-598 | MATCH |
| "SYSCALL_EXIT" | 9 | `31 FF B8 3C 00 00 00 0F 05` = 9 | A:595-597 | MATCH |
| 7F48 CALL | not supported | not in the support list | ISA:253-267 | MATCH |
| multi-mode 2-op IGET (proposal) | `0F B7 84 43 00 00 10 00`, 8 | objdump: `movzx eax,WORD PTR [rbx+rax*2+0x100000]` | design §3.3 | MATCH (as described) |
| multi-mode byte ISET (proposal) | `42 88 84 1B 00 00 00 00 49 FF C3`, 11 | objdump: `mov [rbx+r11*1+0x0],al; inc r11` | design §3.3 | MATCH (as described) |

**Encoding mismatches: 2.** Both are pack5 `IGET` rows. M2 v1 rejects pack5 ops but **not** `IGET` on an ANEW register, so the imm-idx guard matters:
- `IGET a, <negative imm>` on the arena reads the *input* region today.
- Any M2-emitted `IGET` on its own arena with an immediate index ≥ 0 compiles to a **compile-time constant** (`movabs 0`): 17 B, measured.

**Not encoding rows, but wrong as written:**
1. "The x86 emitter and the Stage-1 builder in `A:` are byte-for-byte identical to `M:` … `M = A + 13832`" (§ header).
   - It's true from `A:102` on.
   - It's false for `A:18-101`. `validate_func_pack5_ops` differs: A checks `30|31|34|35|3F`, while M checks `31|34|35|3F` plus `IGET` on trit-vec regs. The offset there is +13818.
   - Which one is the reference matters for pack5 *reject* parity.
2. "83 label slots, more than 130 label ids" / "legacy Stage-1 stream needs about 130" (§0.1, §3.1).
   - Measured: 83 `LABEL` defs, 83 distinct ids `0..82`, all in `IMM_I16` form, 0 duplicates, 147 uses (`lbl.rs`).
   - `MAX_LABELS = 83` (A:1612).
   - The estimate errs on the safe side, but it's wrong.
3. M2 output of "about 140–170 KB" (§0.7). See §3.4; my model gives about 70–100 KB.

---

## 2. Task 2: SSA / label budgets

### 2.1 The real caps

- **SSA:** `alloc_reg` errors at `next_reg >= 192` (FE:1632-1642). It's a static, monotonic counter, so loops don't add to it.
- **Labels:** `alloc_lbl` errors at `next_lbl > 15` (FE:1644-1654), and ids start at 1. **The effective cap is 15, not the 16 label tokens.** "Within 10%" therefore means ≥ 14 labels.
- **Label cost per construct**, measured from FE:2163-2296 and the prototypes:

| Construct | Labels |
|---|---|
| value `if_pos` | 2 (then + join) |
| `match_trit` | 4 |
| `while` | 3 |
| `do_while` | 1 |
| `sel_trit` | 0 |

- **Literals:** integer literals are immediates and cost no SSA (FE:1656-1658). **But any literal outside i32 fails to assemble** (`tok_str.parse::<i32>()`, M:1863).
  - The doc's `0xFFFFFFFF` label sentinel and 44-bit masks can't be literals.
  - Each one costs 1–2 SSA if computed, or needs a different encoding (see §3.5).
- **Entry function:** the **last-declared surface function becomes fn#0, the entry** (FE:3069-3075). `#[fn_id(n)]` is limited to `0..=15` (FE:780). `m2_main` must be last in the concatenated `pure/m2/*.omni`, or carry `#[fn_id(0)]`.

### 2.2 Measured: the code that will actually be re-hosted (`pure/verifier_engine.omni`, `lbl.rs`)

| fn (decl) | SSA | labels | params | vs 192 / 15 | vs doc target 120 / 12 |
|---|---:|---:|---:|---|---|
| `handle_dest` (#53) | **134** | 7 | 6 | 70% / 47% | **over 120** |
| `parse_operand_raw` (#35) | 96 | 2 | 3 | 50% | ok |
| `omni_verify_module` (#0) | 89 | **14** | 1 | **93% of label cap** | **over 12** |
| `verify_ins_body` (#55) | 54 | 9 | 5 | | ok |
| `verify_one_fn` (#57) | small | | **8** | | params > 4 |

The re-host adds `arr` and the arena `a` to most signatures. That's more params, still fine under a stack ABI, and it doesn't reduce labels.

### 2.3 Measured: M2 prototypes written to the design (`proto/m2proto.omni`, compiled with the Rust frontend, all verify)

These are *minimal* faithful versions. Sub-emitters are stubbed, so treat every number as a **lower bound**.

| prototype | SSA | labels | params | doc estimate | note |
|---|---:|---:|---:|---|---|
| `read_operand` (ENG/KV skip loop, 7 kinds, packed result) | **84** | 3 | 3 | ≤ 60 | 40% over the doc estimate; 44% of cap |
| `emit_load` (single+multi, P/R/I) | 28 | 6 | 5 | ≤ 40 / 4-8 | ok |
| `h_iset` (mode select + bytes loop + dest) | 57 | **11** | 5 | ≤ 80 / ≤ 10 | **over the doc's label estimate even with sub-emitters stubbed** |
| `emit_ins` (decode + operand-pack loop + dispatch) | 38 | 5 | 5 | n/a | ok |
| `dispatch0` (3 nested `match_trit`s) | 4 | **12** | 8 | ≤ 12 | **at the ceiling; a 4th `match_trit` = 16 > 15** |
| `h_br_trit` (3 lookups, pass-aware) | 46 | 3 | 3 | ≤ 80 | ok |
| `emit_fn` (header, frame scan, prologue, ins loop) | **96** | 8 | 5 | driver ≤ 100 | `skip_ins` stubbed, no verify call |
| `m2_main` (zero-fill, label init, 3-pass loop, fn loop) | 33 | **12** | 4 | ≤ 12 | **no error-return path, no `ret_slice`, no pass-0 verify yet** |

### 2.4 Hot spots and splits (no recursion, args stay eager)

1. **`omni_verify_module` (14/15).**
   - Split the per-function loop body into `verify_next_fn(a, arr) -> bytes`, so the loop keeps 3 labels.
   - Hoist the decode-error `if_pos` chains into `finish_status(a)`.
2. **`handle_dest` (134).**
   - Split by concern: `dest_ssa_check`, `dest_taint`, `dest_arena_state`, each `(a, …) -> bytes`, called in sequence.
   - Re-hosting `load_u16`'s two IGETs into one `tok()` saves a few SSA but not 14+.
3. **`m2_main` (12+).**
   - Make it straight-line, with 0 labels: `a = m2_init(a)` (6 labels: 2 loops), `a = m2_pass(a, arr, 0)`, `a = m2_pass(a, arr, 1)`, `a = m2_pass(a, arr, 2)`, then `ret_slice`.
   - That removes the pass loop entirely. Unrolling 3 calls is free under eager args, because each is a full statement.
4. **Dispatch.**
   - The doc's "3 levels, 27 leaves" can't live in one function: a 3-level tree is 13 `match_trit`s = 52 labels.
   - **Option A:** one level per function, 4 labels each, ~5 functions.
   - **Option B (preferred):** a 128-byte `op → class` table written once into the arena, then one `ld_u8` plus a 2-level tree over ~12 classes (≤ 8 labels). It's cheaper and table-driven.
5. **`h_iset`.** Keep `emit_iset_small/m8/patch` as separate functions, already in the prototype. Never inline the bytes loop with the mode `match_trit` *and* the dest `if_pos` in a function that also checks bounds.
6. **`emit_fn`.**
   - Move the frame/max-label scan to `fn_scan(a, arr, pos) -> i64`, packing `(maxd:8 | maxl:8 | end:16)`.
   - Move the prologue to `emit_fn_prologue`.
   - This keeps `emit_fn` under 70 SSA.
7. **Lint is load-bearing, not optional.**
   - My `h_br_trit` prototype contains `w = sel_trit(c, w, w, iset(w, 15, nxt))`, and **the Rust verifier accepts it** (`verified=true`).
   - Natively, `iset` always appends, because `r11` is global and args are eager. The VM discards the unselected value. So VM and native diverge.
   - The "single-arena / no arena-touching call inside `sel_trit` or call args / single ANEW" lint must land in C4, before any emitter code.

**Budget conclusion.**
- Nothing in the design forces a function past 192 SSA.
- The label cap (15) is the real constraint: 4 places are already at 11–14.
- The doc's ≤ 120 / ≤ 12 targets are right, but its per-group estimates are low. `read_operand` is +40%, and `h_iset` labels are 11 vs ≤ 10.

---

## 3. Task 3: capacity math (arithmetic shown)

### 3.1 The "28,672-word input cap"

- The source is `mov edx, 0xE000` (A:201), reading into `lea rsi,[rbp-0x2E000]` (A:200).
- **Units:** 0xE000 = 57,344 **bytes**. One token = one u16 = 2 bytes. 57,344 / 2 = **28,672 words**. ✔
- **Region:** `[rbp-0x2E000, rbp-0x20000)` = 0xE000 B, which ends exactly where the output buffer starts.
  - `[rbp-0x30000, rbp-0x2E000)` (8 KiB) is unused.
- **It is not "tokens 0x00000 up to labels 0x10000".**
  - 0x10000 B = 65,536 B = **32,768 words**. That's the doc's *logical* input region (§3.1) and the proposed *multi-function* read window (§3.3).
  - So there are two different caps: **28,672 words for single-function native binaries** (Stage-1 and D1 goldens), and **32,768 words for M2 in multi mode**.
- **Empirical (`cliff.rs`, NOP-padded streams):**
  - 28,630 words: byte-identical to Rust.
  - 28,830 words: **43,450 B vs Rust's 43,460 B with exit 0.** The tail (RET) was silently dropped.
  - This is the silent-truncation failure mode.
- **Largest real input today:** the Stage-1 stream is 19,731 words = 39,462 B, which is 68.8% of 28,672 (31% margin).
- **M2's own token stream (T):**
  - The doc estimates 20–27k words.
  - The re-hosted verifier alone is 11,893 words.
  - My partial, stub-heavy prototype is 5,170 words.
  - Against 32,768 that's 61–82% utilisation; at 27k the margin is **17.6%**.
  - Against 28,672 it would be 94% (5.8% margin), which is why M2 must never be built single-function.

### 3.2 Output region, single-function native

- The output runs from `rbp-0x20000` upward. SSA slot k is at `rbp-8(k+1)` (A:240, A:257).
- Usable bytes = 0x20000 − 8·R, where R = number of SSA regs used.
- **R = 98 (Stage-1):** 131,072 − 784 = **130,288 B**.
- **R = 192 (worst):** 131,072 − 1,536 = **129,536 B**. The doc's "about 129 KB" is right for R = 192.
- **The ELF header lives inside the same buffer.** Stage-1 writes the 120-byte header first (A:1646-1654): 64 B Ehdr + 1×56 B Phdr = 120.
  - Code capacity for Stage-1 = 130,288 − 120 = 130,168 B.
- **Stage-1 self-image:** 106,242 B = **81.5%** of 130,288, a margin of 24,046 B (18.5%).
  - Re-verified natively: `stage1.elf` → `stage2.elf` → `stage3.elf` are all byte-identical, 106,242 B, FNV `0xEAFC81F46B482FF8` (`fp.rs`).
  - The VM gives the same result: 590,227 steps, 209 ms.
- **Empirical cliff (`cliff.rs`, ISET-heavy streams run through native Stage-1):**
  - Output ≤ 130,750 B: matches Rust. Slots of high-numbered regs are clobbered but not re-read in this program, so it's silently unsafe.
  - **131,180 B: SIGBUS (exit 135).**
  - So the hard wall is 0x20000 = 131,072 B. 130,288–131,072 is a silent-corruption zone. Nothing is bounds-checked. ✔ (matches the doc's claim)

### 3.3 Arena layout arithmetic (design §3.1)

Every region checks out:

| Region | Arithmetic | Range |
|---|---|---|
| Labels | 4,096×4 = 16,384 = 0x4000 | 0x10000–0x13FFF ✔ |
| Function code offsets | 1,024×4 = 0x1000 | 0x14000–0x14FFF ✔ |
| Frame sizes | 1,024×2 = 0x800 | 0x15000–0x157FF ✔ |
| State | 256×8 = 0x800 | 0x15800–0x15FFF ✔ |
| Verifier header | 64×8 = 0x200 | → 0x16200 ✔ |
| Error pairs | 64×16 = 0x400 | → 0x16600 ✔ |
| Operand packs | 64×8 = 0x200 | → 0x16800 ✔ |
| Globals | 0x1800 | → 0x18000 ✔ |
| Init | 4,096 × `iset_i64` = 32,768 = 0x8000 | arena offset 0x8000 ↔ logical 0x18000 ✔ |

- The frame-size table entries must hold 16-aligned 8×193 = 1,552, which fits in u16 ✔.
- ⚠ **The brief's shorthand ("register state 0x14000") differs from the doc.** The doc puts the *function table* at 0x14000 and register state at 0x15800. The doc is internally consistent; just pick one and record it in C0.

### 3.4 Output region, M2 multi-function mode

- The constraint is 0x8000 + 2N ≤ 0x100000, so **N ≤ (1,048,576 − 32,768)/2 = 507,904 B**.
- **Size model** (`sizemodel.rs`): §1 sizes, with multi-mode Param = 7 B, CALL = Σ(L+1)+5+7+S, fn prologue 11, RET L0+2, plus about 200 B stub and 120 B header.
  - `verifier_engine`: 40.8 KB at **3.43 B/word**.
  - My prototype: 3.50 B/word.
  - Stage-1's 5.38 B/word comes from its hand-built 18–25-operand ISET byte lists, so it isn't representative.
  - **M2 at 20–27k words ⇒ roughly 69–95 KB** (my model, ±30%), vs the doc's 140–170 KB.
- **Either way it fits.** Worst case (doc's 170 KB) uses 32 KiB + 340 KB = 36% of 1 MiB. My model puts it under 20%.
- Native stack total: 0x110000 + 64 frames × (≤ 1,552 + 16 + 32×8 args) ≈ 1.06 MiB + 117 KB ≈ **1.18 MiB**. That's under the default 8 MiB `ulimit -s`, but it **fails under `ulimit -s` < ~1.2 MiB**, so document it.
- VM memory: the budget is 256 MiB (M:1025). The arena is under 0.5 MB.
- VM time is **superlinear in arena size**, because every register read clones the value (`regs[s].clone()`, M:1095).
  - Measured (`vmcost.rs`): zero-filling 32 / 128 / 256 KiB took 1.8 / ~210 / ~720 ms.
  - 10k call+iget+iset rounds on a 256 KiB arena cost about 50 ms more.
  - It's acceptable for M2, but **§6.5's "VM steps roughly linear" check must also record wall time**, or it will pass while wall time goes quadratic.

### 3.5 Capacity recommendations (options, not decisions)

- **Input window:** keep 0x10000 B with an overflow check, or use 0x20000 B (65,536 words), which is cheap because it's stack. The stub should read 1 byte past the window and trap if it gets one.
- **Label sentinel:** store `offset+1` with `0 = undefined`. The zero-filled arena then needs **no** label-init loop: it saves 4,096 `st_u32` iterations, 3 labels in `m2_init`, and the non-i32 literal.

---

## 4. Task 4: challenging the multi-function ABI

### 4.1 What the Rust emitter's convention actually is

There's only one convention today, the entry convention:
- **Params** live in r12–r15. They're loaded from movabs defaults, then overwritten by the CLI parser (A:181-187, A:208-222, `49 89 C0+reg` at A:140).
- **Bytes-return layout differs from non-bytes:**
  - Bytes return: r12 = 0, defaults go to r13–r15, 3 CLI parsers.
  - Non-bytes: defaults go to r12–r14, r15 = 0, 4 parsers.
- **Locals** are at `[rbp-8(k+1)]` (A:240, A:257).
- **Arena** length is the global r11 (A:206, A:302, A:352-372). The arena base is the fixed `rbp-0x20000`.
- **Input** is at the fixed `rbp-0x2E000` (A:200, A:346).
- **No CALL, no callee frames, no return-to-caller.** RET always exits the process (A:587-615).
- The design's §6.1 convention (stack args at `[rbp+16+8k]`, `rbx` = arena base) is therefore a **new** convention. Nothing in the emitter confirms or contradicts it. It must be specified fully, because the Rust reference (C3) and M2 both have to emit it byte-identically.

### 4.2 Concrete failure modes

1. **No `nwords` channel natively. This blocks D5 and parity.**
   - The read count is discarded (A:189-207).
   - `SLEN` returns r11 whatever its operand (A:371-374). The VM's `SLEN` on `U16Array` returns the array length (M:1355-1361).
   - The VM's out-of-bounds `IGET` returns 0 (M:1302). Native reads whatever is in the stack window.
   - So `slen(arr)` in M2 gives different answers on VM and native.
   - Options:
     - (a) Detect the end by `MOD_END` plus a "token 0 = truncated" error. This works on both: fresh stack pages are zero, and VM OOB is 0.
     - (b) Have the multi-mode stub pass `nread/2` as a hidden first argument, and have the VM pass `len(arr)`. This changes the entry signature, so the frozen-suite scan must cover it.
     - (c) Add a new `SLEN`-on-array semantic natively, which is new behaviour on an existing shape.
2. **Label-key semantics: three measured divergences** (`lbldiv.rs`).
   - Rust keys are the raw `LBL_BASE+k` token or `(imm & 0xFFFF)` (M:523-529), with first match winning (A:621-626).
   - Stage-1 normalises LBL to `k`, keeps the **last** definition, and has only 83 slots (A:1764-1772, A:2058-2066, A:1612).
   - Results:
     - (A) `LABEL LBL1` + `JMP IMM 1`: Rust rejects it as `UNRESOLVED_LABEL`; Stage-1 accepts it (484 B).
     - (B) A duplicate `LABEL 1`: same length, different bytes (`E9 00 00 00 00` vs `E9 11 00 00 00`).
     - (C) `IMM` label 5000: same length, different bytes. Stage-1 silently miscompiles ids ≥ 83.
   - **The design's `lid = k or IMM value` normalisation copies Stage-1's behaviour, not Rust's.** Its `lbase+lid < 4096` / reject-`lid<0` rule also rejects ids Rust accepts (Rust accepts 4096–65535 and negatives masked to u16).
   - D1 ("RustEmit == M2") will fail on any such stream. D2 is safe, because Stage-1's own stream is clean: IMM-only, ids 0..82, no duplicates.
   - Options:
     - (i) M2 follows Rust exactly. Use separate key spaces for LBL and IMM, first match wins, and a 64K→table mapping (e.g. a per-function open-addressing table of 16–128 entries).
     - (ii) Tighten Rust (verifier and emitter) to reject mixed forms, duplicates, and ids ≥ N, scanning all goldens and frozen streams. Successful outputs don't change, but reject behaviour does.
     - (iii) M2 rejects the divergent cases and D5 lists them explicitly.
3. **Entry params are contradictory.**
   - §4.3 says entry `p_cnt ≤ 4 (r12-r15)`.
   - §6.1 says params live uniformly at `[rbp+16+8k]` and RET in the entry is `C9 C3`.
   - Pick one:
     - The stub pushes r15..r12 and CALLs fn#0. That's uniform, and its cost is the 4 pushes.
     - Or the entry function keeps the r12–r15 loads, 3 B each. Then the same function's code differs depending on whether it's the entry.
   - Either way, the stub must reproduce **both** default/CLI layouts (bytes vs non-bytes) from §4.1.
4. **`ret_slice` has no return channel.**
   - In multi mode, the entry `RET` is `L0; C9 C3` into the stub, and rax carries one value.
   - `(off, len)` needs a second register, e.g. rdx, or the entry RET must jump straight to a write-and-exit stub.
   - Specify it in C2/C3, or the reference and M2 will choose different bytes.
5. **Multi-mode encodings are unspecified for most memory ops.** The doc gives only 2-op `IGET` and byte `ISET`. Still needed:
   - `ISET -4/-8/-9`
   - the new `-2/-4` patch, and 3-op `IGET -1/-2/-4/-8`
   - `SLEN`, `ANEW`, and `RET` bytes
   - `[rbx+…]` forms differ in size from `[rbp+…-0x20000]` only by SIB/disp choices. They must be fixed-width, always disp32, or pass 1 and pass 2 sizes can drift.
6. **Native IMOD is wrong on edge cases, while the VM is right.**
   - Native `x % 0` and `INT_MIN % -1` return stale `rdx`. The template's zero path jumps to `xor rax,rax` and then `mov rax,rdx` (A:445-447).
   - Measured after `IABS`: `1234 % 0` → **1234** natively vs **0** in the VM. `INT_MIN % -1` → **INT_MIN** vs **0**. The VM uses `wrapping_rem` with a zero guard (M:1344).
   - M2's bytes stay identical, but **V (VM) ≠ S2 (native)** if M2 ever executes `%` with a zero or −1 divisor.
   - Lint against non-constant divisors in M2, or fix the template in the emitter.
     - The Stage-1 stream has no IDIV or IMOD (opcode histogram in `lbl.rs`), so a fix wouldn't change `0xEAFC81F46B482FF8`.
     - Whether it changes the pure-bootstrap hashes is **UNVERIFIED**.
     - This one is Vera's beat; I'm flagging it for her.
7. **Eager args and non-linear bytes give VM ≠ native.**
   - The VM passes `Bytes` by value: `eval_op` clones (M:1095), and CALL clones its args (M:1468-1470). Native has one global r11.
   - `let b = f(a); let c = g(a);`: the VM's `g` sees the old `a`, while native `g` sees `f`'s appends.
   - The `sel_trit` case is in §2.4 item 7.
   - The Rust verifier doesn't catch either, so lint is required (see §2.4).
8. **ANEW in a callee resets r11 natively** (A:302), truncating the caller's arena. Lint: exactly one ANEW, in `m2_main` or `m2_init`.
9. **Call depth.**
   - The VM errors at 64 (M:1026, M:1129-1134). Native has no check.
   - Core forbids recursion, so depth is static. **Pass 0 must compute the longest call chain and reject cycles or depth ≥ 64**, and the Rust reference must do the same. Otherwise VM and native disagree on deep chains.
   - Whether the surface frontend already rejects recursion is **UNVERIFIED**.
10. **Arity.**
    - The VM CALL passes whatever operands exist. A missing param is `Void`.
    - Native callees read `[rbp+16+8k]` beyond the pushed args, i.e. caller-frame garbage.
    - Pass 0 must reject `CALL` arg count ≠ callee `p_cnt`. Whether the Rust verifier checks this is **UNVERIFIED**.
11. **Caller/callee save.**
    - All values live in stack slots. rax/rcx/rdx/r8/r9 are scratch, and rbx is set once.
    - **r11 must *not* be saved or restored.** It's a global out-parameter; saving it would drop callee appends.
    - Say this explicitly in C3. A "helpful" save in either implementation would break the fixed point.
12. **Forward calls in two passes.**
    - This is sound if (a) every encoding is fixed-width, which holds (always disp32, imm32, imm64, rel32 per §1), (b) the mode (single vs multi) is decided in pass 0, before pass 1, and (c) function order is stream order, while the fn table is keyed by `fn_idx`.
    - Note that fn#0 is the *last* function physically (FE:3069-3075), so the stub's `call fn#0` is always a forward call. That's fine, because the stub size is fixed.
    - Assert `len(pass2) == len(pass1)` **and** `fn_off_pass2[i] == fn_off_pass1[i]` for every i, as my `emit_fn` prototype does.
13. **The `IGET`/`ISET` shapes in F2/F3 aren't "new"; they reinterpret valid shapes** (measured, `enc.rs`).
    - Today `ISET a,-4,x,y` is a 3-byte append (59 B).
    - Today `IGET a,-4,i` is a u16 input read at index −4 (25 B).
    - Today a 3-op RET is a 1-op RET.
    - The frozen-stream scan in C1/C2 must cover all three, plus the VM's current semantics of those shapes. C1 is in flight on `isa/multibyte-loads`; I'm flagging this, not touching it.

### 4.3 Does S2 == S3 survive without Rust host intrinsics?

**Yes, conditionally.**
- S2 and S3 are produced by native binaries that use only `open/read/close/write/exit`, with no host calls, and all code is rbp/rbx-relative, so ASLR is harmless.
- S2 == S3 needs M2 to be deterministic in (T, d0, d1, d2). The risks:
  - **Uninitialised reads.** Slots never written, e.g. a non-assigning `if_pos` arm, read stack garbage. That's deterministic per binary, but it differs between S1 (Rust-emitted) and S2 if the layouts ever differ, and after S1 == S2 they don't. Still, require M2 to initialise every slot it reads: the FE join-slot fix in `8160842` is relevant.
  - **The default triple.** S1 bakes in `(d0, d1, d2)`. `./S1 T` with no args passes those to M2, which bakes them into S2. **The fixed-point command must never pass CLI args**, or the stages diverge by construction.
  - **I/O.** There's a single `read` and a single `write` with no retry loops (A:588-598). Regular files are fine. Pipes and short writes are a theoretical risk, so redirect to files in the selftest.
- **S1 == S2** additionally needs failure modes 2, 3, 4, 5, 11 and 12 resolved identically in C3 and M2.
- **V == S2** additionally needs 1, 6, 7, 8, 9 and 10.

### 4.4 Alternatives to the proposed ABI (tradeoffs only)

| Option | Shape | Pro | Con |
|---|---|---|---|
| A (doc) | push args reverse, `call`, `add rsp,imm32`; params at `[rbp+16+8k]` | simple, unlimited params | Param load grows 3 → 7 B; `add rsp` per call; rsp moves |
| B | **Callee-reserved outgoing area**: the frame includes `8·max_args`; the caller does `mov [rsp+8k],rax`; no push/add | fixed rsp per function, fewer bytes per call, easier 16-alignment | the frame scan must also find the max outgoing args |
| C | r12–r15 for k ≤ 3 (as today), stack for k ≥ 4 | entry and callee param encodings unify with single mode | must push/pop r12–r15 around every call (more bytes, more ways to get it wrong) |
| D | arena-resident arg blocks per depth | no stack args | more bytes, and it mixes ABI state into M2's arena layout |
| Memory | 1 MiB arena on the stack (doc) vs a 2nd `PT_LOAD` RW with `p_memsz` = 1 MiB (bss) | bss is kernel-zeroed, so it needs no zero-fill loop and no `ulimit -s` dependency | the header becomes 176 B with 2 phdrs, and `p_flags` differ (multi mode only) |

---

## 5. Task 5: top risks and fixed-point blockers

**Blockers** (S1 == S2 == S3 == V is impossible until each is resolved):
1. **F1: Rust multi-function reference (C3).** The emitter takes `functions.first()` (A:154) and rejects CALL (ISA:253-267). It must be specified to the byte first (§4.2 items 3, 4, 5, 11, 12).
2. **The `nwords` channel** (§4.2-1). Without one, VM and native disagree on truncated input, and the D5 negatives can't be identical.
3. **The label-key semantics decision** (§4.2-2). D1 is guaranteed to fail on mixed, duplicate, or large label ids under the doc's normalisation.

**High risks:**
4. **Label budget.** Four functions are at 11–14 of 15 (§2.4). Plan the splits in C4/C7, not afterwards.
5. **Silent native cliffs.**
   - Input over 28,672 words gives a wrong ELF with exit 0.
   - Output over ~130 KB corrupts state, and over 131,072 B it crashes.
   - Single mode is unchanged by design, so the D1 harness must reject inputs or outputs over those caps explicitly.
   - Multi mode needs checked reads and appends (doc §3.1 already proposes them; make them mandatory).
6. **VM/native semantic gaps M2 can trip over:** IMOD (§4.2-6), eager-arg appends (§4.2-7), callee ANEW (§4.2-8). Lint them in C4.
7. **F2/F3/F4 reinterpret existing valid shapes** (§4.2-13). Hash safety rests entirely on the frozen-stream scan.

**Medium risks:**
8. The pack5 `IGET` constant-folding on ANEW registers (§1): any arena read must use the 3-op form, as the doc says. Make it a lint, not a convention.
9. The `A:` vs `M:` pack5 validator divergence (§1, non-encoding item 1). Choose the reference.
10. Literals over i32 (M:1863) and `fn_id` limited to 0..15 (FE:780). These are small, but they'll surprise whoever writes M2.

**None of the above requires violating an invariant.** 192 SSA, 15/16 labels, 64-deep calls, eager args, flat arena, no recursion, the opcode table and the frozen hashes are all preserved by every option listed.

---

## 6. The 6 open decisions: options and tradeoffs (no answers assumed)

1. **`RET buf, off, len` vs truncate + copy-down.**
   - The 3-op RET needs a defined multi-mode return channel (§4.2-4) and reinterprets the 3-op RET shape (§4.2-13).
   - Copy-down costs O(N) once (about 100 KB, negligible natively). It's quadratic-ish in the VM only through clones.
   - A third option: emit pass 2 at arena offset 0 by keeping the tables in a second region. That needs no new shape, but it means two buffers, which conflicts with the single-arena discipline.
2. **`IGET -1`.**
   - Without it, byte reads use `-2`/`-4` plus a mask. That works, but costs 1–2 SSA per read.
   - With it, there's one more reinterpreted shape to scan.
3. **u32 × 4096 vs i64 × 2048 labels.**
   - i64 works today with `-9`.
   - u32 needs F3.
   - Either is fine for capacity: M2 needs about 93–1,120 entries, and the verifier is 93 measured.
   - Using `off+1`, 0 = undefined (§3.5) helps either way.
4. **Native multi ABI.** See §4.4 options A–D and the stack-vs-bss memory choice.
5. **`pure/m2/` vs replacing `pure/compiler.omni`.**
   - `pure/m2/` keeps every frozen hash.
   - Replacing means a deliberate suite-hash bump, and the embedded copy (M:4469) must change in lockstep.
6. **Wasm CALL.**
   - Requiring it makes 4-backend parity real for M2.
   - Deferring it needs explicit README wording: "M2 verified on VM + x86-64 only".

---

## Appendix: reproduce

```
cd /tmp/m2probe/nyx
./enc                               # §1 every row: name|size|bytes
./imod                              # §4.2-6 (prints VM; run imod0.elf 1234 0 for native)
./lbl                               # Stage-1 label ids, verifier per-fn budgets
./lbldiv                            # §4.2-2 three label divergences
./budget proto/m2proto.omni         # §2.3 prototype budgets
./sizemodel /workspace/audit-m2/pure/verifier_engine.omni proto/m2proto.omni   # §3.4
./cliff 304 && ./stage1.elf cliff.omni_bc >/dev/null   # §3.2 SIGBUS
./cliff 100 12100 && ./stage1.elf cliff.omni_bc | cmp - cliff_ref.elf  # §3.1 silent truncation
```
