# Milestone 2 Design Prep: a real `pure/` two-pass u16 → ELF64 compiler written in Omni

Status: design only. No code was committed, pushed, or opened as a PR.
Audited tree: `origin/isa/multibyte-loads` @ `8160842` (clone at `/workspace/audit-m2`).
Line references: `A:` = `omni-workspace/crates/omni-aot/src/lib.rs`, `M:` = `omni_rust_bootstrap.rs`.
The x86 emitter and the Stage-1 builder in `A:` are byte-for-byte identical to `M:`. I checked this with `diff`; the offset is `M = A + 13832`.
Every size in §2 was checked empirically against `compile_pure_u16_to_x86_64_elf_with_args` (probe harness in `/tmp/m2probe/`, not committed).

---

## 0. TL;DR: what the code actually says (read this first)

1. **A real token-level fixed point already exists, but the compiler isn't written in Omni.**
   - `build_step7_stage1_compiler_stream` (`A:1590-2170`, `M:15422`) is a two-pass u16 → ELF64 compiler.
   - Rust *builder code* assembles it as one function: 2,248 instructions, 98 SSA registers, 83 label slots, more than 130 label ids.
   - `omnic fixed-point` (`M:18001-18028`) proves `stage1.elf == stage2.elf`: 106,242 B, FNV `0xEAFC81F46B482FF8`. I re-ran it and it passes.
   - The same stream in the VM self-compiles to the same 106,242 bytes in 214 ms (590k steps).
   - So M2 is not "invent self-hosting". M2 means **(a)** write that compiler as auditable multi-function `.omni` source under the language caps, **(b)** add verification, and **(c)** make it compile itself.
2. **Phase D "gen1 == gen2" is Rust vs Rust.**
   - `compile_omni_surface_via_pure_omni_frontend_with_sourcemap` (`M:9458`) runs a 3-argument Omni kernel that only checks `src.len() > 0`, then calls the Rust surface compiler.
   - Step 4 (README) must say this plainly.
3. **The Rust x86 emitter can't compile any multi-function program.**
   - It takes `module.functions.first()` (`A:154`) and rejects `CALL 0x7F48` (`is_x86_64_aot_supported_opcode`, isa `lib.rs:253`). Wasm does the same (`A:696`).
   - `pure/verifier_engine.omni` compiles to 58 functions with 455 CALLs. An M2 compiler written in Omni will look the same.
   - **The Rust reference emitter must gain a multi-function / CALL mode before any byte-for-byte differential or fixed point is possible.** This is the biggest prerequisite.
4. **Native x86 has exactly one byte buffer, and 2-operand `IGET` never reads it.**
   - `ANEW` = `xor r11d,r11d` (`A:302`), so `r11` is the global length.
   - `ISET` writes `[rbp + r11 - 0x20000]` (`A:352-363`).
   - Generic 2-operand `IGET` always reads a **u16 from the input-file region** `[rbp + idx*2 - 0x2E000]` (`A:346`), whatever buffer operand it's given.
   - `IGET` on an `ANEW`-created register takes the static `@PACK5` path instead: `collect_func_pack5_map` (`A:18-68`) is flow-insensitive. That path returns compile-time constants, or 0 for buffers built with typed writes.
   - Consequences:
     - **The M2 compiler must follow a "single arena" discipline**: one `ANEW`, every `bytes` value is that arena, and it is threaded linearly.
     - **Every arena read must use the pending 3-operand `IGET` form**, which must bypass the pack5 path.
     - `verifier_engine.omni` breaks the discipline as written (three live buffers: `stream`, `ws`, `out`), so it must be re-hosted, not just included.
5. **Native x86 silently drops params ≥ 4.**
   - `emit_load_op` handles only `0x7EC0..0x7EC3` (`A:227-238`). `Param(0x7EC4+)` emits 0 bytes, which is a silent miscompile.
   - Stage-1 does worse: `par_b = 0xE0 + 8k` overflows the ModRM byte for k ≥ 4 (`A:1755-1758`). Example: k = 4 gives `4C 89 00`, a store to `[rax]`.
   - The verifier's `verify_one_fn` has 8 params. The multi-function ABI must pass args on the stack.
6. **`pure/compiler.omni` is in the frozen 18-module suite**, but what gets hashed is an **embedded string copy**: `lookup_omni_stdlib_module`, `M:4469`, plus `PURE_OMNI_CORE_SUITE_MODULES`, `M:10503`.
   - Recommendation: **don't overwrite `pure/compiler.omni`.** Put M2 in new files `pure/m2/*.omni`, loaded with `include_str!` the way `verifier_engine.omni` is (`M:11458`).
   - Keep the stub until Caleb explicitly approves a deliberate suite-hash bump.
7. **Capacity cliffs in the native single-function layout. Nothing is bounds-checked.**
   - The input read is capped at `0xE000` B = 28,672 words (`A:201`).
   - The output buffer runs from `rbp-0x20000` up into the SSA slots at `rbp-8*(k+1)`. That's about 129 KB usable.
   - The current Stage-1 output is 106,242 B, about 82% of the cap.
   - Rough M2 estimate: about 20-27k words of input and about 140-170 KB of output, so **it doesn't fit**. The multi-function mode needs a bigger, checked arena (§3.3).

---

## 1. ISA facts M2 must match (real table, isa `lib.rs:112`, `M:161`)

Token partition:

| Range | Meaning |
|---|---|
| `REG 0x7E00-0x7EBF` | 192 SSA registers |
| `PAR 0x7EC0-0x7EDF` | 32 parameters |
| `0x7EE0` `MOD_BEGIN` | Module start |
| `0x7EE3` `FN_DEF` | Function header |
| `0x7EE4` `INS_END` | End of instruction |
| `0x7EE5` `IMM_I16` | i16 immediate (1 word, sign-extended) |
| `0x7EE6` `IMM_I32` | i32 immediate (2 words, hi then lo) |
| `0x7EE7-9` | `TRIT -/0/+` |
| `0x7EEA` `ENG_SPAN` | English span |
| `0x7EEC` `SYM_FN` | Function reference |
| `0x7EEE` `FIELD_KV` | Field key/value |
| `0x7EEF` `MOD_END` | Module end |
| `LBL 0x7EF0-0x7EFF` | 16 label tokens |
| `OP 0x7F00-0x7F7F` | 128 opcodes, all assigned. **No renumbering.** |
| `TY 0x7F80-0x7F8E` | Types |
| `TC 0x7FC0+` | Taint/capability tokens |

Function header (decoder `M:621-737`):

```
FN_DEF fn_idx p_cnt (par_tok ty_tok tc_tok)*p_cnt ret_ty cap_mask ins_cnt
```

Instruction:

```
[dest] op [ty tc] operands... INS_END
```

The `ty tc` pair is present only when there is a dest.

Labels can arrive as `LBL_BASE+k` (the surface compiler, k ∈ 0..15) or as `IMM_I16 id` (the hand-built stream; `extract_label_id` masks with `& 0xFFFF`, `M:523`).

Opcodes the Rust x86 emitter supports (isa `lib.rs:253-267`):

```
00, 01-06, 09, 0D-18, 1E-2D, 30-31, 34-35, 3F-45, 4A, 50, 57-59
```

Notes:
- The supported set does **not** include `46-49` (`LBOUND`/`LNEXT`/**`CALL`**/`TCALL`).
- The opcode set `verifier_engine.omni` actually uses (measured): `06 0D 0E 0F 10 11 12 13 1E 1F 20 22 23 2D 40 41 43 45 48 4A 50`.
- The M2 self-host minimum is that set plus `42` (`match_trit`), `09`, `14`, `16`, `17`, `18`, `24-27`, `28`, `2C`.
- `@PACK5` ops (`30`, `31`, `34`, `35`, `3F`) and pack5-path `IGET` depend on a static whole-function byte map. **M2 should reject them with an explicit error in v1**, and the README must list them as not yet self-hosted.

---

## 2. Exact x86-64 encodings and sizes (from the Rust emitter)

### 2.1 Operand load / store helpers

`emit_load_op`, `A:225-254` / `M:14057-14086`.

| Operand | into `rax` | into `rcx` | Size |
|---|---|---|---|
| `Param 0x7EC0+k`, k ≤ 3 (r12-r15) | `4C 89 (E0+8k)` | `4C 89 (E1+8k)` | 3 |
| `Param` k ≥ 4 | (nothing) | (nothing) | **0 (bug, §0.5)** |
| `Reg r` | `48 8B 85 disp32` | `48 8B 8D disp32` | 7 |
| `ImmInt`/`ImmTrit` | `48 B8 imm64` | `48 B9 imm64` | 10 |

- For `Reg r`, `disp32 = -8*((r-0x7E00)+1)`.
- `ImmInt`/`ImmTrit` is **always a 10-byte movabs with a sign-extended 64-bit immediate.** This resolves the blueprint's "MOV_IMM 32-bit": M2 must sign-extend `IMM_I16` (shl 48 / sar 48) or `IMM_I32` (`(hi<<16)|lo`, then shl 32 / sar 32) and append it with `iset_i64` (mode `-8`, already exists). See Stage-1 `A:1735-1749`.

`emit_store_rax` (`A:256-260`): `48 89 85 disp32`, **S = 7** bytes, emitted only when there is a dest.

Notation: `L0..L3` = load size of operand *i*.

### 2.2 Fixed preamble

| Piece | Bytes | Size | Ref |
|---|---|---|---|
| ELF64 hdr + 1 PT_LOAD | see §2.4 | 120 | `A:631-655` |
| Frame | `55 48 89 E5 48 81 EC 00 00 03 00` | 11 | `A:174-178` |
| Non-bytes ret: arg defaults | `49 BC imm64`, `49 BD imm64`, `49 BE imm64`, `49 C7 C7 00000000` | 37 | `A:212-218` |
| Non-bytes ret: 4× CLI parser | `emit_cli_arg_parser` (argc_min, disp) = (2,18), (3,20), (4,28), (5,30) | 4×65 | `A:120-141`, `A:219-222` |
| **Non-bytes prologue total** | | **308** | verified |
| Bytes ret: `r12=0`, defaults | `49 C7 C4 00000000`, `49 BD..`, `49 BE..`, `49 BF..` | 37 | `A:181-187` |
| Bytes ret: open/read/close argv[1] | read `0xE000` B to `rbp-0x2E000`, then `xor r11d,r11d` | 56 | `A:189-207` |
| Bytes ret: 3× CLI parser | (3,20), (4,28), (5,30) | 3×65 | `A:208-210` |
| **Bytes prologue total** | | **299** | verified |

So code starts at file offset 120. The first instruction is at 428 (non-bytes) or 419 (bytes).

### 2.3 Per-opcode table

Every row was checked by probe; see the "verified" column.

| Op | Name | Encoding after loads | Size | Ref (`A:`) | Verified |
|---|---|---|---|---|---|
| 7F00, 7F05 | NOP, DROP | (none) | 0 | 296 | |
| 7F01-04, 06, 09, 57-59 | ARENA, BORROW_*, MOVE, CINT, CTRIT, SAN_* | load op0 | L0+S | 297-300 | CINT imm = 17 |
| 7F0D | ANEW | `45 31 DB` | 3+S | 301-304 | 10 |
| 7F0E | IGET generic | L1, `0F B7 84 45 00 20 FD FF` | L1+8+S | 345-347 | |
| 7F0E | IGET pack5, imm idx | movabs const (+trit extract) | 10(+…)+S | 309-321 | |
| 7F0E | IGET pack5, dyn idx | `45 31 C0` + n×(10 + L1 + `48 3D imm32` + `4C 0F 44 C1`) + `4C 89 C0` | 6+n(20+L1)+S | 323-341 | |
| 7F0F | ISET `-4` (3 ops) | L2, `42 89 84 1D 00 00 FE FF 49 83 C3 04` | L2+12+D | 350-352 | |
| 7F0F | ISET `-8` (3 ops) | L2, `4A 89 84 1D 00 00 FE FF 49 83 C3 08` | L2+12+D | 353-355 | |
| 7F0F | ISET `-9` (4 ops) | L2→rcx, L3, `48 89 84 0D 00 00 FE FF` | L2+L3+8+D | 356-359 | |
| 7F0F | ISET bytes | per byte: L, `42 88 84 1D 00 00 FE FF 49 FF C3` | Σ(L+11)+D | 360-364 | |
| | ISET dest suffix D | `4C 89 D8` + store | D = 10 if dest | 366-369 | |
| 7F50 | SLEN | `4C 89 D8` | 3+S | 371-374 | |
| 7F10/11/1E/1F/20 | IADD/ISUB/BAND/BOR/BXOR | `48 01/29/21/09/31 C8` | L0+L1+3+S | 429-441 | IADD P,P = 16 |
| 7F12 | IMUL | `48 0F AF C1` | L0+L1+4+S | 435 | |
| 7F13 / 7F14 | IDIV / IMOD | 41-byte guarded `idiv` (+ `48 89 D0`) | L0+L1+41/44+S | 442-450 | 54 / 57 |
| 7F15 / 7F28 | INEG / TNOT | `48 F7 D8` | L0+3+S | 451-455 | |
| 7F16 | IABS | `48 89 C2 48 F7 D8 48 0F 48 C2` | L0+10+S | 456-460 | 20 |
| 7F21 | BNOT | `48 F7 D0` | L0+3+S | 461-465 | |
| 7F22 / 7F23 | SHL / SAR | `48 D3 E0/F8` | L0+L1+3+S | 466-471 | |
| 7F24-27 | CEQ/CNE/CLT/CGT | `48 39 C8 0F (94/95/9C/9F) C0 0F B6 C0 48 8D 44 00 FF` | L0+L1+14+S | 472-488 | 27 |
| 7F17/29, 7F18/2A | IMIN/TAND, IMAX/TOR | `48 39 C8 48 0F 4F/4C C1` | L0+L1+7+S | 489-500 | 20 |
| 7F2B | TXOR | `48 0F AF C1 48 F7 D8` | L0+L1+7+S | 501-506 | |
| 7F2C | TMAJ | `48 01 C8`, L2→rcx, `48 01 C8 48 83 F8 00 0F 9F C0 0F 9C C2 0F B6 C0 0F B6 D2 48 29 D0` | L0+L1+L2+25+S | 507-522 | 41 |
| 7F2D | TCMP | `48 39 C8 0F 9F C2 0F 9C C1 0F B6 D2 0F B6 C9 29 CA 48 63 C2` | L0+L1+20+S | 523-536 | 33 |
| 7F35 / 7F3F | EGATE / TLOGIC | pack5 sum or L0, then `48 83 F8 00 0F 9F C0 0F 9C C2 0F B6 C0 0F B6 D2 48 29 D0` | …+19+S | 404-428 | |
| 7F30/31/34 | pack5 | static-map driven | varies | 375-403 | M2 v1: reject |
| 7F40 | LABEL | records `code.len()` | 0 | 537-541 | |
| 7F41 | JMP | `E9 rel32` | 5 | 542-548 | 5 |
| 7F42 | BR_TRIT | L0, `48 83 F8 00 0F 8C r32 0F 84 r32 E9 r32` | L0+21 | 549-567 | 24 |
| 7F43 / 7F44 | BR_POS / BR_NEG | L0, `48 83 F8 01/FF 0F 84 r32` | L0+10 | 568-576 | |
| 7F45 | SEL_TRIT | L0, L1, `49 89 C0 48 89 CA`, L2, `49 83 F8 00 48 0F 44 D1`, L3, `49 83 F8 00 48 0F 4F D1 48 89 D0` | L0+L1+L2+L3+25+S | 577-586 | 65 |
| 7F4A | RET (non-bytes) | L0 + 41 B: write 8 B of rax to fd 1, exit(0) | L0+41 | 599-614 | 51 |
| 7F4A | RET (bytes) | `BF 01.. 48 8D B5 00 00 FE FF 4C 89 DA B8 01.. 0F 05 31 FF B8 3C.. 0F 05` | 31 | 588-598 | 31 |

Notes:
- **rel32** = `target_code_off - (patch_off + 4)`, with both measured from code start, i.e. ELF offset − 120 (`A:621-629`).
- **Comparison opcodes return trits.** `CEQ` and friends map 0/1 to −1/+1 with `lea rax,[rax+rax-1]`, and `BR_POS` tests `== 1`.
- **Exit sequence.** The blueprint's "SYSCALL_EXIT" is `31 FF B8 3C 00 00 00 0F 05` = **9 bytes**. It only appears inside the 41- and 31-byte RET epilogues. M2 must never size a bare exit separately (§4.2).

### 2.4 ELF header (120 bytes, `A:631-655`; Stage-1 copy `A:1646-1654`)

ELF header fields:

| Field | Value |
|---|---|
| `e_ident` | `7F 45 4C 46 02 01 01 00…` |
| `e_type` | 2 |
| `e_machine` | 0x3E |
| `e_version` | 1 |
| `e_entry` | 0x400078 |
| `e_phoff` | 64 |
| `e_shoff` | 0 |
| `e_flags` | 0 |
| `e_ehsize` | 64 (@52) |
| `e_phentsize` | 56 (@54) |
| `e_phnum` | 1 (@56) |

PT_LOAD @64:

| Field | Value |
|---|---|
| `p_type` | 1 |
| `p_flags` | 5 (R+X, no W; all data lives on the stack) |
| `p_offset` | 0 |
| `p_vaddr` / `p_paddr` | 0x400000 |
| `p_filesz` / `p_memsz` | total_size (@96 / @104) |
| `p_align` | 0x1000 |

Stage-1 writes the header with total_size = 0 and patches @96/@104 at the end with `ISET -9` (`A:2148-2150`). M2 can do the same with the existing `iset_patch_i64`.

---

## 3. Arena layout

### 3.1 Logical layout (Caleb's blueprint, made concrete)

The input stays out of the byte arena. Natively, 2-operand `IGET` on the TY_ARRAY param 0 already reads u16 words from the input region, and the VM's `IGET` on `U16Array` returns a u16 (`M:1288-1293`). That gives a **zero-new-feature, 3-backend-consistent input path**. Wasm still returns 0 today, and the parallel Wasm fix covers that.

| Logical addr | Size | Contents | Physical home | Bound checks |
|---|---|---|---|---|
| `0x00000-0x0FFFF` | 64 KiB | input u16 tokens (≤ 32,768 words) | param 0 (`array`): VM `U16Array` / native input region | `nwords ≤ 32768`. Every read checks `i < nwords`, else `error[3006]`-class decode error. |
| `0x10000-0x13FFF` | 16 KiB | **label table**: 4096 × u32 code offsets, `0xFFFFFFFF` = undefined | arena buf `+0x0000` | `0 ≤ lid`. `lbase+lid < 4096`, else `UNRESOLVED_LABEL`/`BUDGET_EXCEEDED`. |
| `0x14000-0x14FFF` | 4 KiB | function code-offset table: 1024 × u32 | arena `+0x4000` | `fn_idx < 1024` |
| `0x15000-0x157FF` | 2 KiB | function frame sizes: 1024 × u16 | arena `+0x5000` | |
| `0x15800-0x15FFF` | 2 KiB | **register/verifier slot state**: 256 × i64 (224 used: 192 REG + 32 PAR) | arena `+0x5800` | `slot = tok-0x7E00 < 224`, checked before access |
| `0x16000-0x161FF` | 512 B | verifier header words (64 × i64; `verifier_engine` words 0-31) | arena `+0x6000` | |
| `0x16200-0x165FF` | 1 KiB | error pairs: 64 × (code, off) | arena `+0x6200` | `nerr ≤ 64` |
| `0x16600-0x167FF` | 512 B | operand packs: 64 × i64 | arena `+0x6600` | `nops ≤ 64` |
| `0x16800-0x17FFF` | 6 KiB | compiler globals (pass, cursor, lbase, entry fn, 3 default args, out1/out2 starts…) | arena `+0x6800` | |
| `0x18000-…` | 2N | **output**: pass-1 image `[0x18000, +N)`, pass-2 image `[0x18000+N, +N)` | arena `+0x8000` | `0x8000 + 2N ≤ arena_cap`, checked on every append |

Arena initialization: one `ANEW`, then 4,096 `iset_i64(a, 0)` appends (0x8000 bytes), then `st_u32(a, i*4, 0xFFFFFFFF)` for the label table. All of it is O(1) with respect to the input.

Why u32 labels? 4096 × 8 bytes won't fit in 16 KiB. With i64 entries via the existing `-9` patch, the cap would be 2048 labels, which is still plenty. Surface functions have ≤ 16 labels each, so the self-image needs about 70 fns × 16 ≈ 1.1k, and the legacy Stage-1 stream needs about 130.

Label key: `lbase[fn] + lid`, where `lbase` is a running sum of `(max_lid+1)` over earlier functions. Both passes walk functions in the same order, so `lbase` never needs a table.

### 3.2 Getting the output out (needs one new operand shape)

`RET` of a bytes value writes `[0, r11)` natively (`A:588-598`), and the VM returns the whole `Vec`. With tables at the front and two images, M2 has to return a **sub-range**.

**Proposal: `RET buf, off, len`, a 3-operand RET shape.** It follows the same "new operand shape on an existing opcode" pattern Caleb approved for `IGET`/`ISET`.
- VM: `Bytes(buf[off..off+len])`.
- x86: `lea rsi,[base+off]; mov rdx,len`.
- Wasm: the equivalent.
- Hard error if `off+len > len(buf)`.
- Surface intrinsic: `ret_slice(a, off, len)`.
- The frozen suite never has a 3-operand RET, so hashes are unchanged. Verify this by scanning all frozen streams in a selftest.

Alternative: a truncate mode `ISET buf, -16, n` plus a final copy-down. That's O(N) more work, and it reinterprets an existing "append bytes 0xF0, n" shape, so I don't recommend it.

### 3.3 Native multi-function memory map

Used only when `functions.len() > 1`, so single-function mode stays byte-identical.

The entry stub reserves `0x110000` bytes on the stack:

| Region | Use |
|---|---|
| `[rbx, rbx+0x100000)` | 1 MiB arena |
| `[rbx+0x100000, +0x10000)` | 64 KiB input; `read()` up to `0x10000` B |

Register use:
- `rbx` = arena base (callee-saved, set once).
- `r11` = arena length (as today). `syscall` clobbers it, but syscalls only happen in the stub.
- 2-operand `IGET`: `0F B7 84 43 00 00 10 00`, i.e. movzx eax, word [rbx + rax*2 + 0x100000]. 8 bytes, the same size as today.
- Byte `ISET`: `42 88 84 1B 00 00 00 00 49 FF C3`, 11 bytes as today.

---

## 4. Resolving the known issues with the earlier blueprint

### 4.1 The tokenizer gap

Turning source into tokens is the 3.9k-line Rust surface frontend (`compiler/mod.rs`). A fully source-level fixed point needs a guest lexer, parser, and lowerer, which is Milestone 3 scale. **Proposal (honest and staged):**

**M2 = token-level self-hosting.**
- Commit `pure/m2/compiler.omni_bc` (= T, the u16 stream of the M2 sources) as a **seed artifact**, with its FNV-1a64 hash recorded.
- A selftest asserts `RustFrontend(pure/m2/*.omni) == T`. Any drift fails CI, so T can never silently diverge from the source.
- The fixed point is computed from T (§6.3).
- No Rust code runs while S2 or S3 is produced: they come from native ELF subprocesses that only issue open/read/close/write/exit syscalls.

**M3 = source-level.** Port lexer, parser, and lowering to `pure/frontend/*.omni`, run it natively on source text, and assert `GuestFrontend(src) == RustFrontend(src) == T`. Then Rust is only the Stage-0 seed.

The README must say "token-level self-hosted (Rust still tokenizes)" until M3 lands.

### 4.2 "Pass 1 sizes ops that Pass 2 never emits" and "SYSCALL_EXIT sized 9 but emits 10"

**Get rid of the separate size table.** Pass 1 runs the *same* emitter code as Pass 2: it writes real bytes, with rel32 = 0, into the pass-1 image. That is exactly what Stage-1 does today (`A:2145-2155`).

Sizes are then identical by construction, and blueprint-style off-by-one sizing bugs become impossible. The cost is 2× emission, which is still O(n).

Hard assertion: `len(pass2_image) == len(pass1_image)`, else `error[EMIT_SIZE_MISMATCH]`. Pass 2 knows every label and function offset, so it **writes final rel32 values directly, with no fixup list.** That covers forward jumps and forward calls without a patch pass.

### 4.3 label_id and register IDs not bounds-checked

Checks M2 must make:
- `lid` comes from `LBL_BASE+k` (0..15) or `IMM_I16`. Reject `lid < 0` and `lbase+lid ≥ 4096`.
- Reject `LABEL` defined twice in a function. The Rust emitter takes the first match (`A:622-626`); M2 must match that, or both should reject. Decide this in C3.
- `dest`: must be in `0x7E00..0x7EBF`. The verifier already enforces SSA order.
- Slot index `< 224` before any state-table access.
- Entry function: `p_cnt ≤ 4` (r12-r15).
- Callees: `p_cnt ≤ 32` (stack ABI).
- `fn_idx < 1024`.
- `CALL` targets must exist.
- Output bound: §3.1.

### 4.4 MOV_IMM 32-bit

There is no 32-bit immediate form in the real emitter. Everything uses the 10-byte `48 B8+r imm64` (§2.1). The only `imm32` forms are `49 C7 C7/C4 imm32` in the preamble and `48 3D imm32` in the pack5 path.

### 4.5 Opcodes must match the real ISA table

M2 dispatches on the real tokens in §1/§2.3. A selftest walks `OMNI_ISA_TABLE` and asserts that every opcode M2 lowers matches the Rust support list. Anything outside the list gets an explicit error naming `debug_label`, which mirrors the Rust message at `A:161-167`.

---

## 5. Function decomposition under the caps

Caps, measured in `compiler/mod.rs`:
- **192 SSA allocations per function**: `alloc_reg` `:1632`. The counter only grows, so every `let`, temp, and expression counts once, statically.
- **15 allocatable labels** (ids 1..15): `alloc_lbl` `:1644`.

Label cost per construct:

| Construct | Labels |
|---|---|
| `while` | 3 |
| `do_while` | 1 |
| `if_pos` | 2 |
| `match_trit` | 4 |
| `sel_trit` | 0 |

Rules for M2 code:
- Loops use `do_while` or `while go <=> 0`.
- Pure selection uses `sel_trit`.
- Side-effecting choices use `if_pos`/`match_trit`. **Never put a call that touches the arena inside `sel_trit` or a call argument list.** Args are eager, so every branch would run.
- Bytes are threaded linearly: `a = f(a, …)` only, and a single `ANEW` in `m2_main`. Enforce with a lint selftest.
- Arena reads use `ld_u8/u16/u32/u64` only. 2-operand `iget` is allowed **only** on the `array` input param (§0.4).
- No recursion. Call-graph depth stays well under 64; the plan's chain is about 9 levels.

Measured reference: `verifier_engine.omni` peaks at 134 SSA and 14 labels in one function. That's too close to the caps, so **target ≤ 120 SSA and ≤ 12 labels per M2 function**, enforced by a budget-report selftest (the same probe as `/tmp/m2probe/probe2.rs`).

| Module / function group | Role | Est. SSA | Labels |
|---|---|---:|---:|
| `m2/util.omni`: `is_eq`/`is_lt`/`in_range`/`sext16`/`sext32`/`tok(arr,i)` | Pure helpers (reuse verifier versions) | ≤ 12 | 0 |
| `m2/arena.omni`: `arena_init`, `out_append_*`, `chk_cap` | Single ANEW, zero-fill, label-table init, checked appends | ≤ 30 | 3-6 |
| `m2/decode.omni`: `read_operand(arr,pos,n)` → packed (kind:4, value:44, newpos:16) | REG/PAR/IMM16/IMM32/TRIT/LBL/FNREF; ENG_SPAN/FIELD_KV skipped iteratively (as in the verifier fix) | ≤ 60 | ≤ 10 |
| `m2/decode.omni`: `fn_header(arr,pos)` → packed (p_cnt, ret_ty, ins_cnt, body_pos) | Header walk and max-dest scan for frame size | ≤ 50 | 3-6 |
| `m2/verify/*.omni` | `verifier_engine` re-hosted onto arena regions `0x15800-0x167FF`. Stream reads become `tok(arr,i)` (one IGET instead of `load_u16`'s two). Same error codes. | ≤ 120 each | ≤ 12 |
| `m2/emit_ops.omni`: `emit_load(a,op,rcx)`, `emit_store(a,dst)`, `emit_imm64` | §2.1 | ≤ 40 | 4-8 |
| `m2/emit_alu.omni`: `h_alu3` (sel_trit byte pick, like Stage-1 `A:1913-1922`), `h_imul`, `h_divmod`, `h_iabs`, `h_minmax`, `h_cmp4`, `h_tcmp`, `h_txor`, `h_tmaj`, `h_sel` | Immediate byte templates via `iset(a, …)` | ≤ 60 each | ≤ 4 |
| `m2/emit_mem.omni`: `h_anew`, `h_slen`, `h_iget` (2-op and 3-op), `h_iset` (bytes, −4, −8, −9, new −2/−4 patch) | | ≤ 80 | ≤ 10 |
| `m2/emit_ctl.omni`: `h_label`, `h_jmp`, `h_br_trit`, `h_br_pos_neg`, `h_call`, `h_ret` | rel32 from the label or function table: pass 1 writes 0, pass 2 writes final | ≤ 80 | ≤ 10 |
| `m2/dispatch.omni`: `dispatch(a, arr, pos, op, …)` | **Ternary search tree on the opcode** using `match_trit op <=> pivot` (4 labels per level; 3 levels give 27 leaves). Second-level functions handle leaf groups. | ≤ 60 | ≤ 12 |
| `m2/elf.omni`: `elf_header`, `prologue_single(retty)`, `entry_stub_multi`, `fn_prologue`, `patch_sizes` | §2.2/§2.4/§3.3 | ≤ 60 | ≤ 6 |
| `m2/driver.omni`: `emit_fn(a,arr,fn_pos,pass)`, `emit_module(a,arr,pass)`, `m2_main(arr: array, d0, d1, d2) -> bytes` | Pass 0: header scan and verification. Pass 1: size and record. Pass 2: emit. Then `ret_slice`. | ≤ 100 | ≤ 12 |

Estimated size: about 70 functions and 20-27k words, against the 32,768-word input cap and the 1024-function cap.

---

## 6. Test plan

### 6.1 Prerequisites (Rust reference side)

1. **Multi-function x86 reference mode** in the Rust emitter, used only when `functions.len() > 1` or a `CALL` is present:
   - Entry stub, then the functions in order.
   - Each function: `push rbp; mov rbp,rsp; sub rsp, imm32(frame)`. Frame = 16-aligned `8*(max_dest+1)`.
   - Params: `[rbp+16+8k]` via `48 8B 85/8D disp32` (7 bytes, uniform).
   - CALL: push args in reverse (L + `50`), `E8 rel32`, `48 81 C4 imm32` (imm32 always, so sizes are fixed), then the store.
   - RET in non-entry and entry functions: L0 then `C9 C3`.
   - The stub writes the result: 8 bytes, or `(off,len)` for a bytes `ret_slice`. Then exit(0).
   - A trap stub exits with a fixed nonzero code on bounds failure.
   - Entry function choice: `fn_idx 0` if present, else the last function (same rule as `M:5579`).
2. Pending operand shapes, on VM, x86 (both modes), Wasm, the Rust verifier, and the Omni verifier:
   - 3-operand `IGET buf, mode, off` for modes `-2/-4/-8`. **Add `-1` (byte)**, because a native byte read of the arena is otherwise impossible.
   - 4-operand `ISET buf, -2/-4, off, val` patches.
   - The 3-operand `RET` slice.
   - The new `IGET` form must **bypass the pack5 static map** (`A:305-344`).
3. Surface intrinsics `ld_u8/16/32/64`, `st_u16/32`, `ret_slice` in the Rust frontend.

### 6.2 Differential tests (M2 vs Rust, byte-for-byte)

- **D1, per golden (single-function mode).** For every positive golden in `spec/acceptance/goldens/` that is single-function and AOT-supported (pack5 excluded and listed):
  - `RustEmit(T_g, d0,d1,d2) == M2_vm(T_g, d0,d1,d2) == M2_native(T_g, …)`.
  - Default args are baked in as movabs, so M2 takes them as inputs, as Stage-1 does (`A:1664-1668`).
  - Also run the resulting ELF and compare its stdout i64 against the VM.
- **D2, legacy anchor.** `M2(build_step7_stage1_compiler_stream())` must equal the existing **106,242 B ELF, FNV `0xEAFC81F46B482FF8`**. This covers `IMM_I16` labels with ids > 16, ISET −4/−8/−9, bytes RET, and 2,248 instructions.
- **D3, multi-function.** `RustEmitMulti == M2` on:
  - `pure/compiler.omni` (7 fns),
  - the re-hosted verifier,
  - new multi-function goldens: call chains with ≥ 5 params, bytes threading, forward calls.
- **D4, verification parity.** The existing 77-case Rust-vs-Omni verifier differential must hold when run through M2's pass 0. A rejected stream produces the same `(code, offset)` list and **no** ELF.
- **D5, negatives.**
  - `lid ≥ 4096`
  - an entry function with > 4 params
  - an unknown or unsupported opcode, including pack5 (explicit error)
  - a truncated stream (`MOD_END` missing, `INS_END` missing)
  - a duplicate label
  - a `CALL` to a missing function
  - more than 1024 functions
  - output over the arena capacity
  - `ret_slice`/`ld_*` out of bounds, on every backend

### 6.3 Fixed point

```
T  = RustFrontend(pure/m2/*.omni)              (committed seed; drift-checked)
S1 = RustEmitMulti(T, d0,d1,d2)                (reference compiler image)
S2 = ./S1 T.omni_bc                            (native, no Rust in-process)
S3 = ./S2 T.omni_bc
V  = VM(T) applied to T                        (4th witness, same as Stage-1 today)
ASSERT S1 == S2 == S3 == V   (byte equality; report len + FNV-1a64)
```

- S1 == S2 is the differential. **S2 == S3 is the self-host fixed point.**
- SHA-256 from the blueprint is optional. Byte equality is strictly stronger, and SHA-256 would mean writing a zero-crate implementation first.
- Run it twice to check determinism.
- Add an `omnic m2-fixed-point` CLI command and a selftest. On non-Linux-x86 hosts, skip with an explicit "SKIPPED (not x86-64 Linux)" line, never a silent pass.

### 6.4 Guards after every commit

- `omnic selftest`: 416 monolith / 378 workspace. Counts only go up on purpose.
- `cargo test`.
- `pure-bootstrap` hashes: `0x23B8D17257CCA9C4`, `0xE221085777B8AC47`, `0x78462AE4C81DF408`.
- `fixed-point` `0xEAFC81F46B482FF8`.
- 77/77 verifier differential.
- 4-backend goldens.
- The new M2 budget report: per-function SSA ≤ 120, labels ≤ 12, and no call-graph cycles.

### 6.5 O(n) evidence

Measure VM steps for M2 on 1×, 2×, and 4× synthetic inputs and assert the growth is roughly linear (within 10%). This also covers the per-function work: the frame-size scan and two passes are constant factors.

---

## 7. Missing language and backend features, in priority order

| # | Feature | Why M2 needs it | Status |
|---|---|---|---|
| F1 | Multi-function x86 mode: CALL, stack-arg ABI, `rbx` arena base, bigger checked arena/input, trap stub | M2 is necessarily multi-function | **Missing in the Rust reference** |
| F2 | 3-operand `IGET` `-1/-2/-4/-8` (bypassing pack5) | Read label, function, and state tables; read templates | Approved; pending. **`-1` is not yet in the approved list.** |
| F3 | 4-operand `ISET` `-2/-4` patch | u32 label/function tables | Approved; pending |
| F4 | `RET buf, off, len` | Return the pass-2 image | **New proposal; needs Caleb's OK** |
| F5 | Surface intrinsics `ld_*`/`st_*`/`ret_slice` | Writing M2 in surface Omni | Missing |
| F6 | Param ≥ 4 handling: error in single mode, stack ABI in multi mode | Silent miscompile today (§0.5) | Bug |
| F7 | Wasm real `IGET`/`ISET` memory, plus Wasm CALL | 4-backend parity claim for M2 | IGET/ISET fix in flight. CALL missing; can be deferred if stated honestly. |
| F8 | `@PACK5` dynamic semantics in AOT | Self-hosting pack5 programs | Out of M2 scope; explicit reject |

Not needed: new opcodes, raising any cap, structs, heap, or recursion.

---

## 8. Ordered commit plan

Each commit runs every guard in §6.4. Branch name: `m2/selfhost-emitter`.

| # | Commit | Contents |
|---|---|---|
| C0 | `docs: Milestone 2 design + decision record` | This doc, plus the open decisions: F4 shape, the `-1` mode, u32 vs i64 label entries, duplicate-label rule, trap exit code. |
| C1 | `isa: 3-op IGET (-1/-2/-4/-8) + 4-op ISET (-2/-4) on VM/x86/Wasm/verifiers` | Coordinate with the in-flight `isa/multibyte-loads` work (don't duplicate it). Add a selftest that scans the frozen streams to prove the new shapes are never present. |
| C2 | `isa: 3-op RET slice + surface ld_*/st_*/ret_slice` | After Caleb approves F4. |
| C3 | `aot: multi-function x86-64 reference mode (CALL, stack ABI, rbx arena, entry/trap stubs)` | Single-function path untouched, which the hashes prove. Add native-vs-VM multi-function goldens. Turn Param ≥ 4 into an explicit error in single mode. |
| C4 | `pure/m2: util, arena, decode` | VM differential of `read_operand`/`fn_header` against `decode_omni_token_stream` on every golden. |
| C5 | `pure/m2: single-function emitter subset` | D1 byte-for-byte on goldens (VM-executed M2). |
| C6 | `pure/m2: legacy Stage-1 stream reproduces 0xEAFC81F46B482FF8` | D2. |
| C7 | `pure/m2: pass-0 verifier (re-hosted verifier_engine)` | D4 77/77, and no ELF on reject. |
| C8 | `pure/m2: multi-function emission` | D3 against the C3 reference. |
| C9 | `pure/m2: committed seed T + drift check + omnic m2-fixed-point` | §6.3: S1 == S2 == S3 == V. |
| C10 | (optional) `aot: Wasm CALL for 4-backend parity of multi-function programs` | |
| C11 | `docs: README/roadmap honesty` | What's token-level vs source-level; that Phase D is Rust-vs-Rust; the pack5 exclusion; the M3 roadmap. |
| — | Replace the `pure/compiler.omni` stub (deliberate suite-hash bump) | **Only with Caleb's explicit approval.** Until then M2 lives in `pure/m2/`. |

---

## 9. Open decisions for Caleb

1. Approve **`RET buf, off, len`** as a new operand shape, or choose truncate + copy-down instead.
2. Add **`IGET` mode `-1`** (byte) to the approved 3-operand set.
3. Label table entries: **u32 × 4096** (needs F3) or **i64 × 2048** (works with the existing `-9`).
4. The native multi-function ABI in §6.1/§3.3: stack args, `rbx` arena base, 1 MiB stack arena.
5. M2 lives in `pure/m2/` and leaves `pure/compiler.omni` and its embedded copy alone, or replace it with a planned suite-hash bump.
6. Wasm CALL: required for M2 sign-off, or deferred with honest wording.
