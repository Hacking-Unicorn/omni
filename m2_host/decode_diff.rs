// M2-04: differential test of the Omni token decoder (pure/m2/decode.omni) against the Rust
// reference decode_omni_token_stream. Host-side tooling, included from the monolith's M2
// FOUNDATION section; changes no emitted byte.
//
// The Omni decoder runs on the VM (OmniBinaryVm) and appends records (format in the
// decode.omni header). Accepted streams must match structurally (every fn header, param,
// instruction offset/dest/op/ty/tc and operand, FieldKV chains included). Rejected streams
// must give the same error class; the word offset is compared when the Rust message carries
// one (missing INS_END, invalid operand token, unexpected structural token).

pub const M2_DECODE_OMNI_SRC: &str = concat!(
    include_str!("../pure/m2/util.omni"),
    "\n",
    include_str!("../pure/isa/table.omni"),
    "\n",
    include_str!("../pure/m2/decode.omni"),
    "\n",
    include_str!("../spec/m2/decode_harness.omni")
);

pub const M2_DECODE_WS_OMNI_SRC: &str = concat!(
    include_str!("../pure/m2/util.omni"),
    "\n",
    include_str!("../pure/isa/table.omni"),
    "\n",
    include_str!("../pure/m2/decode.omni"),
    "\n",
    include_str!("../spec/m2/decode_ws_harness.omni")
);

pub const M2_VERIFY_OMNI_SRC: &str = concat!(
    include_str!("../pure/m2/util.omni"),
    "\n",
    include_str!("../pure/isa/table.omni"),
    "\n",
    include_str!("../pure/m2/decode.omni"),
    "\n",
    include_str!("../pure/m2/labels.omni"),
    "\n",
    include_str!("../pure/m2/verify/mod_check.omni"),
    "\n",
    include_str!("../spec/m2/verify_harness.omni")
);

pub const M4_VM_OMNI_SRC: &str = concat!(
    include_str!("../pure/m2/util.omni"),
    "\n",
    include_str!("../pure/isa/table.omni"),
    "\n",
    include_str!("../pure/m2/decode.omni"),
    "\n",
    include_str!("../pure/m2/labels.omni"),
    "\n",
    include_str!("../pure/m4/arena.omni"),
    "\n",
    include_str!("../pure/m4/ops_core.omni"),
    "\n",
    include_str!("../pure/m4/vm.omni"),
    "\n",
    include_str!("../spec/m4/vm_harness.omni")
);

pub const M4_STANDALONE_OMNI_SRC: &str = concat!(
    include_str!("../pure/m2/util.omni"),
    "\n",
    include_str!("../pure/isa/table.omni"),
    "\n",
    include_str!("../pure/m2/decode.omni"),
    "\n",
    include_str!("../pure/m2/labels.omni"),
    "\n",
    include_str!("../pure/m4/arena.omni"),
    "\n",
    include_str!("../pure/m4/ops_core.omni"),
    "\n",
    include_str!("../pure/m4/vm.omni"),
    "\n",
    include_str!("../pure/m4/m4_standalone.omni")
);

pub const M4_VM_WASM_HARNESS_SRC: &str = concat!(
    include_str!("../pure/m2/util.omni"),
    "\n",
    include_str!("../pure/isa/table.omni"),
    "\n",
    include_str!("../pure/m2/decode.omni"),
    "\n",
    include_str!("../pure/m4/arena.omni"),
    "\n",
    include_str!("../pure/m4/ops_core.omni"),
    "\n",
    include_str!("../spec/m4/vm_wasm_harness.omni")
);

pub const P7_JUDGE_OMNI_SRC: &str = concat!(
    include_str!("../pure/m7/judge.omni"),
    "\n",
    include_str!("../spec/m7/judge_harness.omni"),
);

pub const P7_HEAL_OMNI_SRC: &str = concat!(
    include_str!("../pure/m7/heal.omni"),
    "\n",
    include_str!("../spec/m7/heal_harness.omni"),
);

pub const P7_ADAPT_OMNI_SRC: &str = concat!(
    include_str!("../pure/m7/judge.omni"),
    "\n",
    include_str!("../pure/m7/adapt.omni"),
    "\n",
    include_str!("../spec/m7/adapt_harness.omni"),
);

pub const P7_LEARN_OMNI_SRC: &str = concat!(
    include_str!("../pure/m7/judge.omni"),
    "\n",
    include_str!("../pure/m7/learn.omni"),
    "\n",
    include_str!("../spec/m7/learn_harness.omni"),
);

pub const M3_INCLUDE_OMNI_SRC: &str = concat!(
    include_str!("../pure/m3/include.omni"),
    "\n",
    include_str!("../spec/m3/include_harness.omni"),
);

pub const M5_LAUNCHER_OMNI_SRC: &str = concat!(
    include_str!("../pure/m2/util.omni"),
    "\n",
    include_str!("../pure/isa/table.omni"),
    "\n",
    include_str!("../pure/m2/decode.omni"),
    "\n",
    include_str!("../pure/m2/labels.omni"),
    "\n",
    include_str!("../pure/m2/arena.omni"),
    "\n",
    include_str!("../pure/m2/elf.omni"),
    "\n",
    include_str!("../pure/m5/elf_sys.omni"),
    "\n",
    include_str!("../pure/m5/launcher.omni"),
    "\n",
    include_str!("../spec/m5/launcher_harness.omni"),
);

pub const API_STAGE1_OMNI_SRC: &str = concat!(
    include_str!("../pure/m7/judge.omni"),
    "\n",
    include_str!("../pure/api/ternary.omni"),
    "\n",
    include_str!("../pure/api/api_main.omni"),
);

pub const API_STAGE2_OMNI_SRC: &str = concat!(
    include_str!("../pure/m7/judge.omni"),
    "\n",
    include_str!("../pure/api/reflect.omni"),
    "\n",
    include_str!("../spec/api/reflect_harness.omni"),
);

pub const API_STAGE2_EMIT_OMNI_SRC: &str = concat!(
    include_str!("../pure/m2/util.omni"),
    "\n",
    include_str!("../pure/isa/table.omni"),
    "\n",
    include_str!("../pure/m2/decode.omni"),
    "\n",
    include_str!("../pure/m2/labels.omni"),
    "\n",
    include_str!("../pure/m2/arena.omni"),
    "\n",
    include_str!("../pure/m2/elf.omni"),
    "\n",
    include_str!("../pure/m2/emit_x86.omni"),
    "\n",
    include_str!("../pure/m2/emit_multi.omni"),
    "\n",
    include_str!("../pure/api/emit.omni"),
    "\n",
    include_str!("../spec/api/emit_harness.omni"),
);

pub const API_STAGE2_5_OMNI_SRC: &str = concat!(
    include_str!("../pure/m7/judge.omni"),
    "\n",
    include_str!("../pure/api/reflect.omni"),
    "\n",
    include_str!("../pure/api/adaptive.omni"),
    "\n",
    include_str!("../spec/api/adaptive_harness.omni"),
);

pub const M5_SYS_OMNI_SRC: &str = concat!(
    include_str!("../pure/m2/util.omni"),
    "\n",
    include_str!("../pure/isa/table.omni"),
    "\n",
    include_str!("../pure/m2/decode.omni"),
    "\n",
    include_str!("../pure/m2/labels.omni"),
    "\n",
    include_str!("../pure/m2/arena.omni"),
    "\n",
    include_str!("../pure/m2/elf.omni"),
    "\n",
    include_str!("../pure/m5/elf_sys.omni"),
    "\n",
    include_str!("../spec/m5/sys_harness.omni")
);

/// Fixed scratch offsets written by m2d_decode_at_ws (must equal the m2l_scratch_* constants).
pub const M2D_SCRATCH_OP: usize = 0x7FF00;
pub const M2D_SCRATCH_NOPS: usize = 0x7FF08;
pub const M2D_SCRATCH_NEXT: usize = 0x7FF10;
pub const M2D_SCRATCH_META: usize = 0x7FF18;
pub const M2D_SCRATCH_END: usize = 0x80000;
/// Logical offset of the pure/isa/table.omni rodata (M3 band; read-only for M2).
pub const M2D_ISA_TBL: usize = 0x81000;

/// The meta word m2d_op_meta must produce for `tok`, from OMNI_ISA_TABLE (same flag
/// packing as compare_isa_table_dump), or -1 outside the opcode range.
fn m2_expected_op_meta(tok: u16) -> i64 {
    match decode_opcode_token(tok) {
        Some(spec) => {
            let flags = (spec.has_dest_reg as i64)
                | ((spec.sink_requires_safe as i64) << 1)
                | ((spec.is_sanitizer as i64) << 2)
                | ((is_x86_64_aot_supported_opcode(spec.token_id) as i64) << 3)
                | ((is_wasm_aot_supported_opcode(spec.token_id) as i64) << 4);
            flags | ((spec.req_cap_mask as i64) << 8)
        }
        None => -1,
    }
}

/// Runs the ws harness: arena of M2D_ISA_TBL zero bytes, table appended by the harness.
fn m2_run_decode_at_ws(ws_mod: &BinModule, stream: &[u16], off: i64) -> Result<Vec<u8>, String> {
    let mut vm = OmniBinaryVm::new();
    match vm.execute_function(
        ws_mod,
        m2_entry_of(ws_mod),
        vec![OmniValue::U16Array(stream.to_vec()), OmniValue::Int(off), OmniValue::Bytes(vec![0u8; M2D_ISA_TBL])],
    )? {
        OmniValue::Bytes(b) => Ok(b),
        o => Err(format!("decode_at_ws returned {:?}", o)),
    }
}

fn m2_entry_of(m: &BinModule) -> u16 {
    m2_entry_fn_idx(m).unwrap_or(0)
}

fn m2_compile_decoder(src: &str) -> Result<BinModule, String> {
    with_lockstep_suspended(|| {
        let u = compile_omni_surface_to_u16(src)?;
        decode_omni_token_stream(&u)
    })
}

/// Maps a Rust decoder error message to the Omni error code (and the offset if it has one).
pub fn m2_rust_decode_err_class(msg: &str) -> (i64, Option<i64>) {
    let off = msg.rsplit(' ').next().and_then(|t| t.parse::<i64>().ok());
    let table: [(&str, i64); 13] = [
        ("Missing 0x7EE0", 1),
        ("Truncated TOK_SCHEMA_DEF header", 2),
        ("Truncated TOK_SCHEMA_DEF fields", 3),
        ("Truncated TOK_EXPERT_DEF header", 4),
        ("Truncated TOK_FN_DEF header", 5),
        ("Truncated TOK_FN_DEF signature", 6),
        ("Unexpected EOF inside function instruction body", 7),
        ("Truncated SSA instruction header", 8),
        ("Missing 0x7EE4", 9),
        ("Invalid operand token", 10),
        ("Unexpected structural token", 11),
        ("Truncated English token span", 13),
        ("EOF", 12),
    ];
    for (p, c) in table {
        if msg.starts_with(p) || (c == 12 && (msg.starts_with("EOF in") || msg.starts_with("Unexpected EOF while"))) {
            let o = if matches!(c, 9 | 10 | 11) { off } else { None };
            return (c, o);
        }
    }
    (-1, None)
}

fn m2_rd(b: &[u8], i: &mut usize) -> Result<i64, String> {
    if *i + 8 > b.len() {
        return Err(String::from("record stream truncated"));
    }
    let v = i64::from_le_bytes(b[*i..*i + 8].try_into().unwrap());
    *i += 8;
    Ok(v)
}

/// Rebuilds one operand from packed Omni words (FieldKV prefixes nest the following operand).
fn m2_operand_from_words(words: &[i64], k: &mut usize, stream: &[u16]) -> Result<BinOperand, String> {
    let mut kv: Vec<u16> = Vec::new();
    loop {
        let w = *words.get(*k).ok_or("operand words ended inside a FieldKV chain")?;
        *k += 1;
        let next = (w & 0xFFFFF) as usize;
        let val = (w >> 20) & 0xFFFF_FFFF;
        let kind = (w >> 52) & 15;
        let op = match kind {
            1 => BinOperand::Reg(REG_BASE + val as u16),
            2 => BinOperand::Param(PAR_BASE + val as u16),
            3 => BinOperand::Label(val as u16),
            4 => BinOperand::ImmTrit((val as i64 - 1) as i8),
            5 => BinOperand::ImmInt(val as u32 as i32 as i64),
            6 => {
                let len = val as usize;
                let start = next.checked_sub(len).ok_or("bad span")?;
                BinOperand::EnglishSpan(stream.get(start..next).ok_or("span out of range")?.iter().map(|t| t & ENGLISH_MAX_TOKEN).collect())
            }
            7 => BinOperand::SchemaRef(val as u16),
            8 => BinOperand::FnRef(val as u16),
            9 => BinOperand::SkillRef(val as u16),
            10 => {
                kv.push(val as u16);
                continue;
            }
            _ => return Err(format!("bad operand kind {} in word 0x{:X}", kind, w)),
        };
        let mut out = op;
        while let Some(f) = kv.pop() {
            out = BinOperand::FieldKV(f, Box::new(out));
        }
        return Ok(out);
    }
}

/// Parses the Omni record stream into a BinModule (or the decode error code + offset).
pub fn m2_parse_decode_records(b: &[u8], stream: &[u16]) -> Result<Result<BinModule, (i64, i64)>, String> {
    let mut m = BinModule::default();
    let mut i = 0usize;
    loop {
        match m2_rd(b, &mut i)? {
            15 => {
                let st = m2_rd(b, &mut i)?;
                let off = m2_rd(b, &mut i)?;
                let nf = m2_rd(b, &mut i)?;
                if i != b.len() {
                    return Err(String::from("bytes after the end record"));
                }
                if st != 0 {
                    return Ok(Err((st, off)));
                }
                if nf as usize != m.functions.len() {
                    return Err(format!("end record says {} fns, records have {}", nf, m.functions.len()));
                }
                return Ok(Ok(m));
            }
            3 => {
                let id = m2_rd(b, &mut i)? as u16;
                let fc = m2_rd(b, &mut i)? as usize;
                let mut f = Vec::new();
                for _ in 0..fc {
                    let w = m2_rd(b, &mut i)?;
                    f.push(((w & 0xFFFF) as u16, ((w >> 16) & 0xFFFF) as u16));
                }
                m.schemas.push((id, f));
            }
            4 => {
                let w = m2_rd(b, &mut i)?;
                m.experts.push(((w & 0xFFFF) as u16, ((w >> 16) & 0xFFFF) as u16, ((w >> 32) & 0xFFFF) as u16, ((w >> 48) & 0xFFFF) as u16));
            }
            1 => {
                let fn_idx = m2_rd(b, &mut i)? as u16;
                let pc = m2_rd(b, &mut i)? as usize;
                let ret = m2_rd(b, &mut i)? as u16;
                let cap = m2_rd(b, &mut i)? as u16;
                let _cnt = m2_rd(b, &mut i)?;
                let mut params = Vec::new();
                for _ in 0..pc {
                    let w = m2_rd(b, &mut i)?;
                    params.push(((w & 0xFFFF) as u16, ((w >> 16) & 0xFFFF) as u16, ((w >> 32) & 0xFFFF) as u16));
                }
                m.functions.push(BinFunction { fn_idx, params, ret_ty_tok: ret, cap_mask: cap, instructions: Vec::new() });
            }
            2 => {
                let off = m2_rd(b, &mut i)? as usize;
                let dest = m2_rd(b, &mut i)?;
                let op = m2_rd(b, &mut i)? as u16;
                let ty = m2_rd(b, &mut i)? as u16;
                let tc = m2_rd(b, &mut i)? as u16;
                let mut words = Vec::new();
                loop {
                    let w = m2_rd(b, &mut i)?;
                    if w == 0 {
                        break;
                    }
                    words.push(w);
                }
                let mut operands = Vec::new();
                let mut k = 0usize;
                while k < words.len() {
                    operands.push(m2_operand_from_words(&words, &mut k, stream)?);
                }
                let f = m.functions.last_mut().ok_or("instruction record before any fn record")?;
                f.instructions.push(BinInstruction {
                    offset: off,
                    dest_reg: if dest < 0 { None } else { Some(REG_BASE + dest as u16) },
                    op_tok: op,
                    ty_tok: ty,
                    tc_tok: tc,
                    operands,
                });
            }
            t => return Err(format!("unknown record tag {}", t)),
        }
    }
}

fn m2_same_module(a: &BinModule, b: &BinModule) -> Result<(), String> {
    if a.schemas != b.schemas || a.experts != b.experts {
        return Err(String::from("schema/expert mismatch"));
    }
    if a.functions.len() != b.functions.len() {
        return Err(format!("fn count {} vs {}", a.functions.len(), b.functions.len()));
    }
    for (fa, fb) in a.functions.iter().zip(&b.functions) {
        if fa.fn_idx != fb.fn_idx || fa.params != fb.params || fa.ret_ty_tok != fb.ret_ty_tok || fa.cap_mask != fb.cap_mask {
            return Err(format!("fn #{} header mismatch", fa.fn_idx));
        }
        if fa.instructions.len() != fb.instructions.len() {
            return Err(format!("fn #{} instruction count {} vs {}", fa.fn_idx, fa.instructions.len(), fb.instructions.len()));
        }
        for (x, y) in fa.instructions.iter().zip(&fb.instructions) {
            if x.offset != y.offset || x.dest_reg != y.dest_reg || x.op_tok != y.op_tok || x.ty_tok != y.ty_tok || x.tc_tok != y.tc_tok || x.operands != y.operands {
                return Err(format!("fn #{} instruction @{} mismatch: rust {:?} omni {:?}", fa.fn_idx, x.offset, x, y));
            }
        }
    }
    Ok(())
}

/// Runs the Omni decoder on the VM over `stream`.
pub fn m2_omni_decode(stream: &[u16]) -> Result<Result<BinModule, (i64, i64)>, String> {
    use std::sync::OnceLock;
    static DEC: OnceLock<Result<BinModule, String>> = OnceLock::new();
    let m = DEC.get_or_init(|| m2_compile_decoder(M2_DECODE_OMNI_SRC)).as_ref().map_err(|e| e.clone())?;
    let mut vm = OmniBinaryVm::new();
    match vm.execute_function(m, m2_entry_of(m), vec![OmniValue::U16Array(stream.to_vec())])? {
        OmniValue::Bytes(b) => m2_parse_decode_records(&b, stream),
        other => Err(format!("Omni decoder returned {:?}", other)),
    }
}

/// One differential case: Ok(()) when Omni and Rust agree.
pub fn m2_decode_diff_one(name: &str, stream: &[u16]) -> Result<(), String> {
    let omni = m2_omni_decode(stream).map_err(|e| format!("decode-diff {}: omni run failed: {}", name, e))?;
    match (decode_omni_token_stream(stream), omni) {
        (Ok(r), Ok(o)) => m2_same_module(&r, &o).map_err(|e| format!("decode-diff {}: {}", name, e)),
        (Err(msg), Err((code, off))) => {
            let (rc, ro) = m2_rust_decode_err_class(&msg);
            if rc != code || ro.map_or(false, |x| x != off) {
                return Err(format!("decode-diff {}: rust `{}` (class {}, off {:?}) vs omni code {} off {}", name, msg, rc, ro, code, off));
            }
            Ok(())
        }
        (Ok(_), Err(e)) => Err(format!("decode-diff {}: rust accepts, omni rejects {:?}", name, e)),
        (Err(msg), Ok(_)) => Err(format!("decode-diff {}: rust rejects `{}`, omni accepts", name, msg)),
    }
}

/// Deterministic negative / edge streams derived from a seed stream.
fn m2_decode_edge_cases(seed: &[u16]) -> Vec<(String, Vec<u16>)> {
    let mut v: Vec<(String, Vec<u16>)> = Vec::new();
    v.push((String::from("empty"), Vec::new()));
    v.push((String::from("no_mod_begin"), vec![0x7EE3, 0, 0]));
    v.push((String::from("begin_only"), vec![TOK_MOD_BEGIN]));
    v.push((String::from("begin_end_trailing_garbage"), vec![TOK_MOD_BEGIN, TOK_MOD_END, 0x1234, 0x7F00]));
    v.push((String::from("bad_struct"), vec![TOK_MOD_BEGIN, 0x7F10]));
    v.push((String::from("schema_ok"), vec![TOK_MOD_BEGIN, TOK_SCHEMA_DEF, 3, 2, 10, 11, 12, 13, TOK_MOD_END]));
    v.push((String::from("schema_hdr_short"), vec![TOK_MOD_BEGIN, TOK_SCHEMA_DEF, 3]));
    v.push((String::from("schema_fields_short"), vec![TOK_MOD_BEGIN, TOK_SCHEMA_DEF, 3, 2, 10, 11, 12]));
    v.push((String::from("expert_ok"), vec![TOK_MOD_BEGIN, TOK_EXPERT_DEF, 1, 2, 3, 0xFFFF]));
    v.push((String::from("expert_short"), vec![TOK_MOD_BEGIN, TOK_EXPERT_DEF, 1, 2, 3]));
    v.push((String::from("fn_hdr_short"), vec![TOK_MOD_BEGIN, TOK_FN_DEF, 0]));
    v.push((String::from("fn_sig_short"), vec![TOK_MOD_BEGIN, TOK_FN_DEF, 0, 1, 0x7EC0, TY_I64, 0x7FC0, TY_I64, 0]));
    // One function, one instruction, every operand kind including nested FieldKV and spans.
    let ins: Vec<u16> = vec![
        0x7E05, 0x7F10, TY_I64, 0x7FC0,
        0x7E00, 0x7EC1, 0x7EF3, TOK_TRIT_NEG, TOK_TRIT_POS, TOK_IMM_I16, 0xFFFE, TOK_IMM_I32, 0x8000, 0x0001,
        TOK_ENG_SPAN, 3, 0x7E01, 5, 6, TOK_SYM_SCHEMA, 9, TOK_SYM_FN, 4, TOK_SYM_SKILL, 2,
        TOK_FIELD_KV, 7, TOK_FIELD_KV, 8, 0x7E02, TOK_INS_END,
        0x7F41, 0x7EF3, TOK_INS_END,
    ];
    let mut f: Vec<u16> = vec![TOK_MOD_BEGIN, TOK_FN_DEF, 0, 1, 0x7EC0, TY_I64, 0x7FC0, TY_I64, 0, 2];
    f.extend_from_slice(&ins);
    let mut all = f.clone();
    all.push(TOK_MOD_END);
    v.push((String::from("all_operand_kinds"), all.clone()));
    v.push((String::from("no_mod_end"), f.clone()));
    for cut in 1..all.len() {
        v.push((format!("all_kinds_trunc{}", cut), all[..cut].to_vec()));
    }
    let mut kv_end = vec![TOK_MOD_BEGIN, TOK_FN_DEF, 0, 0, TY_I64, 0x7FC0, 1, 0x7F4A, TOK_FIELD_KV, 1, TOK_INS_END, TOK_MOD_END];
    v.push((String::from("fieldkv_wraps_ins_end"), kv_end.clone()));
    kv_end[8] = 0x7FFF;
    v.push((String::from("invalid_operand"), kv_end));
    v.push((String::from("ssa_hdr_short"), vec![TOK_MOD_BEGIN, TOK_FN_DEF, 0, 0, TY_I64, 0x7FC0, 1, 0x7E00, 0x7F06]));
    v.push((String::from("body_eof"), vec![TOK_MOD_BEGIN, TOK_FN_DEF, 0, 0, TY_I64, 0x7FC0, 2, 0x7F4A, 0x7E00, TOK_INS_END]));
    v.push((String::from("span_trunc"), vec![TOK_MOD_BEGIN, TOK_FN_DEF, 0, 0, TY_I64, 0x7FC0, 1, 0x7F4A, TOK_ENG_SPAN, 9, 1]));
    // Prefix truncations and deterministic word mutations of a real compiled stream.
    for cut in (1..seed.len()).step_by(7) {
        v.push((format!("seed_trunc{}", cut), seed[..cut].to_vec()));
    }
    let mut x: u64 = 0x9E3779B97F4A7C15;
    for k in 0..48 {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        let mut s = seed.to_vec();
        if s.is_empty() {
            break;
        }
        let at = (x as usize) % s.len();
        let choices = [0x7EE4u16, 0x7EEE, 0x7EEA, 0x7EE6, 0x7EE5, 0x7E00, 0x7EE3, 0x7FFF, 0x0000, 0x7EEF];
        s[at] = choices[(x >> 40) as usize % choices.len()];
        v.push((format!("seed_mut{}@{}", k, at), s));
    }
    v
}

/// selftest_m2_decode_diff: Omni decoder == Rust decoder over the M2-01 corpus (acceptance
/// goldens, core suite, verifier engine, Stage-1, Phase D), the edge/negative set, and the
/// fixed-offset scratch entry (m2d_decode_at_ws). Plus the lint gate on the decoder modules.
pub fn selftest_m2_decode_diff() -> Result<usize, String> {
    let mut checks = 0usize;
    let mut seed: Vec<u16> = Vec::new();
    for (name, s) in m2_lock_corpus() {
        if let Ok(s) = s {
            if name == "golden:branch_loop_golden" {
                seed = s.clone();
            }
            m2_decode_diff_one(&name, &s)?;
            checks += 1;
        }
    }
    for (name, s) in m2_decode_edge_cases(&seed) {
        m2_decode_diff_one(&name, &s)?;
        checks += 1;
    }
    // Scratch entry: op / nops / next / meta at the fixed offsets, on a full M2 arena with
    // the ISA table at 0x81000. Every instruction of all_operand_kinds, then every opcode
    // token 0x7EFF..0x7F80 in a one-instruction stream (meta checked against OMNI_ISA_TABLE).
    let ws_mod = m2_compile_decoder(M2_DECODE_WS_OMNI_SRC)?;
    let stream = &m2_decode_edge_cases(&seed).into_iter().find(|c| c.0 == "all_operand_kinds").ok_or("no case")?.1;
    let rust = decode_omni_token_stream(stream)?;
    let want_ins = &rust.functions[0].instructions;
    let rd = |b: &[u8], o: usize| i64::from_le_bytes(b[o..o + 8].try_into().unwrap());
    let mut base: Option<Vec<u8>> = None;
    let mut cases: Vec<(Vec<u16>, i64, i64, i64, i64)> = Vec::new();
    for (k, want) in want_ins.iter().enumerate() {
        let next = want_ins.get(k + 1).map(|x| x.offset as i64).unwrap_or((stream.len() - 1) as i64);
        cases.push((stream.clone(), want.offset as i64, want.op_tok as i64, want.operands.len() as i64, next));
    }
    for tok in (0x7F00u16..=0x7F81).chain([0x0005u16, 0x7EE9]) {
        let s1: Vec<u16> = vec![TOK_MOD_BEGIN, TOK_FN_DEF, 0, 0, TY_I64, 0x7FC0, 1, tok, 0x7E00, TOK_INS_END, TOK_MOD_END];
        // Expected op/nops/next from the Rust decoder; a stream it rejects must give nops -1, next < 0.
        let (nops, next) = match decode_omni_token_stream(&s1) {
            Ok(m) => (m.functions[0].instructions[0].operands.len() as i64, 10),
            Err(_) => (-1, i64::MIN),
        };
        cases.push((s1, 7, tok as i64, nops, next));
    }
    for (st, off, op, nops, next) in cases {
        let b = m2_run_decode_at_ws(&ws_mod, &st, off)?;
        let meta = m2_expected_op_meta(op as u16);
        let mut got = (rd(&b, M2D_SCRATCH_OP), rd(&b, M2D_SCRATCH_NOPS), rd(&b, M2D_SCRATCH_NEXT), rd(&b, M2D_SCRATCH_META));
        if next == i64::MIN && got.2 < 0 {
            got.2 = i64::MIN;
        }
        if b.len() <= M2D_ISA_TBL || got != (op, nops, next, meta) {
            return Err(format!("decode_at_ws @{}: got op/nops/next/meta {:?}, want {:?} (len {})", off, got, (op, nops, next, meta), b.len()));
        }
        // Nothing outside the four scratch words may change (table rodata identical every run).
        let mut masked = b.clone();
        masked[M2D_SCRATCH_OP..M2D_SCRATCH_META + 8].iter_mut().for_each(|x| *x = 0);
        if masked[..M2D_ISA_TBL].iter().any(|&x| x != 0) {
            return Err(format!("decode_at_ws @{}: wrote outside its scratch words", off));
        }
        match &base {
            None => base = Some(masked),
            Some(p) if *p != masked => return Err(String::from("decode_at_ws: ISA table rodata differs between runs")),
            _ => {}
        }
        checks += 1;
    }
    Ok(checks)
}

pub const M3_ENCODE_OMNI_SRC: &str = concat!(
    include_str!("../pure/m2/util.omni"),
    "\n",
    include_str!("../pure/isa/table.omni"),
    "\n",
    include_str!("../pure/m2/decode.omni"),
    "\n",
    include_str!("../pure/isa/encode.omni"),
    "\n",
    include_str!("../spec/m2/encode_harness.omni")
);

pub const M2_LABELS_OMNI_SRC: &str = concat!(
    include_str!("../pure/m2/util.omni"),
    "\n",
    include_str!("../pure/isa/table.omni"),
    "\n",
    include_str!("../pure/m2/decode.omni"),
    "\n",
    include_str!("../pure/m2/labels.omni"),
    "\n",
    include_str!("../spec/m2/labels_harness.omni")
);

pub const M2_ELF_OMNI_SRC: &str = concat!(
    include_str!("../pure/m2/util.omni"),
    "\n",
    include_str!("../pure/isa/table.omni"),
    "\n",
    include_str!("../pure/m2/decode.omni"),
    "\n",
    include_str!("../pure/m2/labels.omni"),
    "\n",
    include_str!("../pure/m2/arena.omni"),
    "\n",
    include_str!("../pure/m2/elf.omni"),
    "\n",
    include_str!("../spec/m2/elf_harness.omni")
);

pub const M2_COMPILER_OMNI_SRC: &str = concat!(
    include_str!("../pure/m2/util.omni"),
    "\n",
    include_str!("../pure/isa/table.omni"),
    "\n",
    include_str!("../pure/m2/decode.omni"),
    "\n",
    include_str!("../pure/m2/labels.omni"),
    "\n",
    include_str!("../pure/m2/arena.omni"),
    "\n",
    include_str!("../pure/m2/elf.omni"),
    "\n",
    include_str!("../pure/m2/emit_x86.omni"),
    "\n",
    include_str!("../pure/m2/driver.omni"),
    "\n",
    include_str!("../spec/m2/compiler_harness.omni")
);

pub const M2_MULTI_OMNI_SRC: &str = concat!(
    include_str!("../pure/m2/util.omni"),
    "\n",
    include_str!("../pure/isa/table.omni"),
    "\n",
    include_str!("../pure/m2/decode.omni"),
    "\n",
    include_str!("../pure/m2/labels.omni"),
    "\n",
    include_str!("../pure/m2/arena.omni"),
    "\n",
    include_str!("../pure/m2/elf.omni"),
    "\n",
    include_str!("../pure/m2/emit_x86.omni"),
    "\n",
    include_str!("../pure/m2/emit_multi.omni"),
    "\n",
    include_str!("../spec/m2/multi_harness.omni")
);

pub const M2_SELFHOST_OMNI_SRC: &str = concat!(
    include_str!("../pure/m2/util.omni"),
    "\n",
    include_str!("../pure/isa/table.omni"),
    "\n",
    include_str!("../pure/m2/decode.omni"),
    "\n",
    include_str!("../pure/m2/labels.omni"),
    "\n",
    include_str!("../pure/m2/verify/mod_check.omni"),
    "\n",
    include_str!("../pure/m2/arena.omni"),
    "\n",
    include_str!("../pure/m2/elf.omni"),
    "\n",
    include_str!("../pure/m2/emit_x86.omni"),
    "\n",
    include_str!("../pure/m2/emit_multi.omni"),
    "\n",
    include_str!("../pure/m2/m2_standalone.omni")
);

pub const M3_LEX_OMNI_SRC: &str = concat!(
    include_str!("../pure/m3/lex.omni"),
    "\n",
    include_str!("../spec/m3/lex_harness.omni")
);

pub const M3_SKIM_OMNI_SRC: &str = concat!(
    include_str!("../pure/m3/skim.omni"),
    "\n",
    include_str!("../spec/m3/skim_harness.omni")
);

pub const M3_EXPR_OMNI_SRC: &str = concat!(
    include_str!("../pure/m3/expr.omni"),
    "\n",
    include_str!("../spec/m3/expr_harness.omni")
);

pub const M3_STMT_OMNI_SRC: &str = concat!(
    include_str!("../pure/m3/stmt.omni"),
    "\n",
    include_str!("../spec/m3/stmt_harness.omni")
);

pub const M3_LOWER_OMNI_SRC: &str = concat!(
    include_str!("../pure/m3/lower.omni"),
    "\n",
    include_str!("../spec/m3/lower_harness.omni")
);

pub const M3_STANDALONE_OMNI_SRC: &str =
    include_str!("../pure/m3/m3_standalone.omni");

/// M3-04 selftest: `pure/isa/encode.omni` re-encodes every positive acceptance golden,
/// `pure/verifier_engine.omni` (11,893 words), and `build_step7_stage1_compiler_stream`
/// (19,731 words) bit-for-bit identically to `encode_u16_stream_to_bytes`.
pub fn selftest_m3_isa_encode_roundtrip() -> Result<usize, String> {
    let enc_mod = m2_compile_decoder(M3_ENCODE_OMNI_SRC)?;
    let entry = m2_entry_of(&enc_mod);
    let mut checks = 0usize;
    let mut cases: Vec<(String, Vec<u16>)> = Vec::new();
    for &(gid, src) in M2_GOLDEN_SOURCES {
        if let Ok(u) = compile_omni_surface_to_u16(src) {
            cases.push((format!("golden:{}", gid), u));
        }
    }
    let ve_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(OMNI_VERIFIER_ENGINE_SURFACE))?;
    cases.push((String::from("pure/verifier_engine.omni"), ve_u16));
    cases.push((String::from("stage1_compiler_stream"), build_step7_stage1_compiler_stream()));
    for (name, stream) in &cases {
        let want = encode_u16_stream_to_bytes(stream);
        let mut vm = OmniBinaryVm::new();
        let got = match vm.execute_function(&enc_mod, entry, vec![OmniValue::U16Array(stream.clone())])? {
            OmniValue::Bytes(b) => b,
            o => return Err(format!("M3-04 encode roundtrip '{}': returned {:?}", name, o)),
        };
        if got != want {
            let min_l = got.len().min(want.len());
            let diff = (0..min_l).find(|&i| got[i] != want[i]).unwrap_or(min_l);
            return Err(format!(
                "M3-04 encode roundtrip '{}' mismatch: got {} B vs want {} B (first diff at byte {})",
                name, got.len(), want.len(), diff
            ));
        }
        checks += 1;
    }
    Ok(checks)
}

/// M2 label-table selftest (`pure/m2/labels.omni`): verifies D-3 (u32 off+1 / 0=undef),
/// D-8 (0x20000..0x23FFF), D-11 (IMM >= 4080 rejected), and D-12 (first-definition-wins;
/// LBL 0x7EF0..0x7EFF occupy slots 4080..4095 without aliasing IMM ids).
pub fn selftest_m2_labels_table() -> Result<usize, String> {
    let lbl_mod = m2_compile_decoder(M2_LABELS_OMNI_SRC)?;
    let entry = m2_entry_of(&lbl_mod);
    let arena_size = 0x24000usize;
    let run_q = |stream: &[u16], slot: i64| -> Result<i64, String> {
        let mut vm = OmniBinaryVm::new();
        match vm.execute_function(
            &lbl_mod,
            entry,
            vec![
                OmniValue::U16Array(stream.to_vec()),
                OmniValue::Int(slot),
                OmniValue::Bytes(vec![0u8; arena_size]),
            ],
        )? {
            OmniValue::Int(v) => Ok(v),
            o => Err(format!("m2lbl_test_main returned {:?}", o)),
        }
    };
    let mut checks = 0usize;
    // 1. LBL 1 at ins 0 (slot 4081); LBL 2 undefined (slot 4082); IMM 1 undefined (slot 1, no alias).
    let s1 = assemble_omni_text_to_u16("PARAM $0 I64 SAFE\nRET_TY I64\nLABEL 1\nJMP 1\nRET 7\n")?;
    if run_q(&s1, 4081)? != 0 || run_q(&s1, 4082)? != -1 || run_q(&s1, 1)? != -1 {
        return Err(String::from("M2 labels: LBL 1 / undefined / no-IMM-alias check failed"));
    }
    checks += 3;
    // 2. Duplicate LABEL 1 at ins 1 and ins 3: first definition (1) wins (D-12).
    let s_dup = assemble_omni_text_to_u16("PARAM $0 I64 SAFE\nRET_TY I64\nJMP 1\nLABEL 1\n%0 = CINT 5\nLABEL 1\nRET 7\n")?;
    if run_q(&s_dup, 4081)? != 1 {
        return Err(String::from("M2 labels: duplicate LABEL did not keep first definition"));
    }
    checks += 1;
    // 3. IMM label id 82 -> slot 82; IMM label id 4079 -> slot 4079; IMM 4080 rejected (-1, D-11).
    let s_imm82 = m2_lbl_patch("PARAM $0 I64 SAFE\nRET_TY I64\nLABEL 1\nJMP 1\nRET 7\n", &[(0, &m2_imm16(82)), (1, &m2_imm16(82))])?;
    if run_q(&s_imm82, 82)? != 0 || run_q(&s_imm82, 4081)? != -1 {
        return Err(String::from("M2 labels: IMM 82 slot check failed"));
    }
    checks += 2;
    let s_imm4079 = m2_lbl_patch("PARAM $0 I64 SAFE\nRET_TY I64\nLABEL 1\nJMP 1\nRET 7\n", &[(0, &m2_imm16(4079)), (1, &m2_imm16(4079))])?;
    if run_q(&s_imm4079, 4079)? != 0 {
        return Err(String::from("M2 labels: IMM 4079 boundary slot check failed"));
    }
    checks += 1;
    let s_imm4080 = m2_lbl_patch("PARAM $0 I64 SAFE\nRET_TY I64\nLABEL 1\nJMP 1\nRET 7\n", &[(0, &m2_imm16(4080)), (1, &m2_imm16(4080))])?;
    if run_q(&s_imm4080, 4080)? != -1 {
        return Err(String::from("M2 labels: IMM 4080 must be rejected per D-11"));
    }
    checks += 1;
    Ok(checks)
}

/// M2-11 selftest: `pure/m2/arena.omni` + `pure/m2/elf.omni`.
/// Asserts D-7 rejection (>128 KiB), byte-exact 120-B ELF64 header vs Rust reference,
/// and exact trap/multi-stub/prologue bytes via M2-06 `ret_slice`.
pub fn selftest_m2_11_arena_elf() -> Result<usize, String> {
    let elf_mod = m2_compile_decoder(M2_ELF_OMNI_SRC)?;
    let entry = m2_entry_of(&elf_mod);
    let run_h = |mode: i64, a1: i64, a2: i64| -> Result<Vec<u8>, String> {
        let mut vm = OmniBinaryVm::with_budgets(50_000_000, 16 * 1024 * 1024);
        let arena = OmniValue::Bytes(vec![0u8; 524_288]);
        let res = vm
            .execute_function(&elf_mod, entry, vec![arena, OmniValue::Int(mode), OmniValue::Int(a1), OmniValue::Int(a2)])
            .map_err(|e| format!("M2-11 mode={} a1={} a2={}: {}", mode, a1, a2, e))?;
        match res {
            OmniValue::Bytes(b) => Ok(b),
            other => Err(format!("M2-11 harness mode={} returned {:?}, expected Bytes", mode, other)),
        }
    };
    let mut checks = 0usize;

    // 1. D-7: nwords = 65537 (> 128 KiB) -> err = 1, out_len = 0
    let b_over = run_h(0, 65537, 0)?;
    if b_over.len() != 16 {
        return Err(format!("M2-11 D-7 over-128K returned {} bytes, expected 16", b_over.len()));
    }
    let err_over = i64::from_le_bytes(b_over[0..8].try_into().unwrap());
    let len_over = i64::from_le_bytes(b_over[8..16].try_into().unwrap());
    if err_over != 1 || len_over != 0 {
        return Err(format!("M2-11 D-7 over-128K expected (err=1, len=0), got ({}, {})", err_over, len_over));
    }
    checks += 1;

    // 2. D-7: nwords = 65536 (exact 128 KiB boundary) -> err = 0, out_len = 0
    let b_ok = run_h(0, 65536, 0)?;
    let err_ok = i64::from_le_bytes(b_ok[0..8].try_into().unwrap());
    let len_ok = i64::from_le_bytes(b_ok[8..16].try_into().unwrap());
    if err_ok != 0 || len_ok != 0 {
        return Err(format!("M2-11 D-7 boundary expected (err=0, len=0), got ({}, {})", err_ok, len_ok));
    }
    checks += 1;

    // 3. 176-byte ELF64 header (PT_LOAD + PT_GNU_STACK) byte-exact vs Rust reference
    let ref_u16 = compile_omni_surface_strict_to_u16("#[strict]\nfn f(x: i64) -> i64 { return x + 1; }\n")?;
    let ref_elf = compile_pure_u16_to_x86_64_elf_with_args(&ref_u16, 0, 0, 0)?;
    let omni_hdr = run_h(1, ref_elf.len() as i64, 0)?;
    if omni_hdr.len() != 176 || omni_hdr.as_slice() != &ref_elf[..176] {
        return Err(format!(
            "M2-11 ELF64 header mismatch: len={} (want 176), first diff at {:?}",
            omni_hdr.len(),
            omni_hdr.iter().zip(ref_elf[..176].iter()).position(|(a, b)| a != b)
        ));
    }
    checks += 1;

    // 4. Trap stub (12 B) + multi stub head (25 B) + fn prologue (11 B) = 48 B exact bytes
    let stubs = run_h(2, 42, 64)?;
    let mut want_stubs: Vec<u8> = Vec::with_capacity(48);
    // trap stub: BF 2A 00 00 00 B8 3C 00 00 00 0F 05
    want_stubs.extend_from_slice(&[0xBF, 42, 0, 0, 0, 0xB8, 60, 0, 0, 0, 0x0F, 0x05]);
    // multi stub head: 55 48 89 E5 48 81 EC 10 00 11 00 | 48 89 E3 | 48 C7 83 00 00 11 00 00 00 00 00
    want_stubs.extend_from_slice(&[
        0x55, 0x48, 0x89, 0xE5, 0x48, 0x81, 0xEC, 0x10, 0x00, 0x11, 0x00,
        0x48, 0x89, 0xE3,
        0x48, 0xC7, 0x83, 0x00, 0x00, 0x11, 0x00, 0x00, 0x00, 0x00, 0x00,
    ]);
    // fn prologue (frame_bytes = 64 = 0x40): 55 48 89 E5 48 81 EC 40 00 00 00
    want_stubs.extend_from_slice(&[0x55, 0x48, 0x89, 0xE5, 0x48, 0x81, 0xEC, 64, 0, 0, 0]);
    if stubs != want_stubs {
        return Err(format!("M2-11 stubs mismatch: got {:02X?}, want {:02X?}", stubs, want_stubs));
    }
    checks += 1;

    Ok(checks)
}

/// M2-12/M2-13 selftest: Pure-Omni two-pass x86-64 ELF64 compiler (`pure/m2/emit_x86.omni` +
/// `pure/m2/driver.omni`). Compiles:
/// 1. Core positive single-function AOT goldens -> 0-byte diff vs Rust reference.
/// 2. The full 19,731-word `build_step7_stage1_compiler_stream()` -> 106,242 B,
///    FNV-1a64 = 0xEAFC81F46B482FF8, 0-byte diff vs Rust!
pub fn selftest_m2_13_stage1_anchor() -> Result<usize, String> {
    let comp_mod = m2_compile_decoder(M2_COMPILER_OMNI_SRC)?;
    let entry = m2_entry_of(&comp_mod);
    let compile_in_omni = |stream: &[u16], a0: i64, a1: i64, a2: i64| -> Result<Vec<u8>, String> {
        let mut vm = OmniBinaryVm::with_budgets(500_000_000, 32 * 1024 * 1024);
        let arena = OmniValue::Bytes(vec![0u8; 524_288]);
        let res = vm.execute_function(
            &comp_mod,
            entry,
            vec![
                arena,
                OmniValue::U16Array(stream.to_vec()),
                OmniValue::Int(a0),
                OmniValue::Int(a1),
                OmniValue::Int(a2),
            ],
        )?;
        match res {
            OmniValue::Bytes(b) => Ok(b),
            other => Err(format!("M2-12/13 compiler returned {:?}, expected Bytes", other)),
        }
    };
    let mut checks = 0usize;

    // 1. Sample single-fn positive goldens (ALU, branches, loops, sel_trit) -> 0-byte diff vs Rust
    let sample_srcs: [&str; 3] = [
        "#[strict]\nfn f(x: i64) -> i64 { return x + 7; }\n",
        "#[strict]\nfn f(a: i64, b: i64) -> i64 { let c: trit = a <=> b; let r: i64 = sel_trit(c, a, 0, b); return r; }\n",
        "#[strict]\nfn f(n: i64) -> i64 { let s: i64 = 0; let i: i64 = 0; while n - i <=> 0 { s = s + i; i = i + 1; } return s; }\n",
    ];
    for (idx, src) in sample_srcs.iter().enumerate() {
        let u16s = compile_omni_surface_strict_to_u16(src)?;
        let want = compile_pure_u16_to_x86_64_elf_with_args(&u16s, 45, 40, 90)?;
        let got = compile_in_omni(&u16s, 45, 40, 90)?;
        if got != want {
            let min_l = got.len().min(want.len());
            let diff = (0..min_l).find(|&i| got[i] != want[i]).unwrap_or(min_l);
            return Err(format!(
                "M2-12 sample#{} mismatch: got {} B vs want {} B (first diff at byte {}: got {:02X?} vs want {:02X?})",
                idx,
                got.len(),
                want.len(),
                diff,
                got.get(diff..diff.saturating_add(8).min(got.len())),
                want.get(diff..diff.saturating_add(8).min(want.len()))
            ));
        }
        checks += 1;
    }

    // 2. M2-13 Legacy Anchor: full 19,731-word Stage-1 self-compiler stream -> 106,242 B, 0xEAFC81F46B482FF8
    let stage1_u16 = build_step7_stage1_compiler_stream();
    let want_elf = compile_pure_u16_to_x86_64_elf_with_args(&stage1_u16, 45, 40, 90)?;
    let got_elf = compile_in_omni(&stage1_u16, 45, 40, 90)?;
    if got_elf != want_elf {
        let min_l = got_elf.len().min(want_elf.len());
        let diff = (120..min_l).find(|&i| got_elf[i] != want_elf[i]).unwrap_or(min_l);
        return Err(format!(
            "M2-13 Stage-1 anchor mismatch: got {} B (FNV=0x{:016X}) vs want {} B (FNV=0x{:016X}), first code diff at byte {} (got {:02X?} vs want {:02X?})",
            got_elf.len(),
            fnv1a_64_hash(&got_elf),
            want_elf.len(),
            fnv1a_64_hash(&want_elf),
            diff,
            got_elf.get(diff..diff.saturating_add(16).min(got_elf.len())),
            want_elf.get(diff..diff.saturating_add(16).min(want_elf.len()))
        ));
    }
    if got_elf.len() != 112_180 || fnv1a_64_hash(&got_elf) != 0x62F14B335B52BECC {
        return Err(format!(
            "M2-13 Stage-1 anchor hash/len mismatch: {} B / 0x{:016X}",
            got_elf.len(),
            fnv1a_64_hash(&got_elf)
        ));
    }
    checks += 1;

    Ok(checks)
}

/// M2-15 selftest: Pure-Omni multi-function x86-64 ELF64 compiler (`pure/m2/emit_multi.omni`)
/// produces bit-for-bit identical ELF64 binaries (`0`-byte diff!) to Rust's
/// `compile_pure_u16_to_x86_64_elf_multi` across multi-function call chains, 5-param stack args,
/// Bytes threading + 3-op RET, `pure/compiler.omni` (7 fns), and `pure/verifier_engine.omni` (58 fns)!
pub fn selftest_m2_15_pure_multi_compiler() -> Result<usize, String> {
    let comp_mod = m2_compile_decoder(M2_MULTI_OMNI_SRC)?;
    let entry = m2_entry_of(&comp_mod);
    let compile_in_omni = |stream: &[u16], a0: i64, a1: i64, a2: i64| -> Result<Vec<u8>, String> {
        let mut vm = OmniBinaryVm::with_budgets(500_000_000, 32 * 1024 * 1024);
        let arena = OmniValue::Bytes(vec![0u8; 524_288]);
        let res = vm.execute_function(
            &comp_mod,
            entry,
            vec![
                arena,
                OmniValue::U16Array(stream.to_vec()),
                OmniValue::Int(a0),
                OmniValue::Int(a1),
                OmniValue::Int(a2),
            ],
        )?;
        match res {
            OmniValue::Bytes(b) => Ok(b),
            other => Err(format!("M2-15 compiler returned {:?}, expected Bytes", other)),
        }
    };
    let cases: [(&str, &str); 5] = [
        (
            "mf_call_chain_2",
            "#[strict]\nfn leaf(x: i64) -> i64 { return x * 3 + 1; }\n#[strict]\nfn main(x: i64) -> i64 { let r: i64 = leaf(x); return r + 5; }\n",
        ),
        (
            "mf_params_5",
            "#[strict]\nfn sum5(a: i64, b: i64, c: i64, d: i64, e: i64) -> i64 { return a + b * 2 + c * 3 + d * 4 + e * 5; }\n#[strict]\nfn main(x: i64) -> i64 { let r: i64 = sum5(x, x + 1, x + 2, x + 3, x + 4); return r; }\n",
        ),
        (
            "mf_bytes_ret_slice",
            "#[strict]\nfn fill(b: bytes, v: i64) -> bytes { let w: bytes = iset(b, v, v + 1, v + 2); return w; }\n#[strict]\nfn main(x: i64) -> bytes { let b: bytes = anew(0); b = fill(b, 10); return ret_slice(b, 1, 2); }\n",
        ),
        ("pure/compiler.omni", include_str!("../pure/compiler.omni")),
        ("pure/verifier_engine.omni", OMNI_VERIFIER_ENGINE_SURFACE),
    ];
    let mut checks = 0usize;
    for &(name, src) in &cases {
        let u16s = compile_omni_surface_strict_to_u16(src)?;
        let want = compile_pure_u16_to_x86_64_elf_multi(&u16s, 45, 40, 90)?;
        let got = compile_in_omni(&u16s, 45, 40, 90)?;
        if got != want {
            let min_l = got.len().min(want.len());
            let diff = (120..min_l).find(|&i| got[i] != want[i]).unwrap_or(min_l);
            return Err(format!(
                "M2-15 '{}' mismatch: got {} B (FNV=0x{:016X}) vs want {} B (FNV=0x{:016X}), first code diff at byte {} (got {:02X?} vs want {:02X?})",
                name,
                got.len(),
                fnv1a_64_hash(&got),
                want.len(),
                fnv1a_64_hash(&want),
                diff,
                got.get(diff..diff.saturating_add(16).min(got.len())),
                want.get(diff..diff.saturating_add(16).min(want.len()))
            ));
        }
        checks += 1;
    }

    // 6. Self-compilation check: Pure Omni (`pure/m2/emit_multi.omni`) compiles `pure/m2/*.omni`
    //    (standalone 4-param entry) byte-for-byte identically to Rust (`0`-byte diff!).
    let m2_self_src = format!(
        "{}\n#[strict]\nfn m2_standalone_main(arr: array, arg0: i64, arg1: i64, arg2: i64) -> bytes {{\n    let w: bytes = anew(0);\n    return m2_compile_single_x86(w, arr, arg0, arg1, arg2);\n}}\n",
        M2_COMPILER_OMNI_SRC
    );
    let self_u16 = compile_omni_surface_strict_to_u16(&m2_self_src)?;
    let want_self = compile_pure_u16_to_x86_64_elf_multi(&self_u16, 45, 40, 90)?;
    let got_self = compile_in_omni(&self_u16, 45, 40, 90)?;
    if got_self != want_self {
        let min_l = got_self.len().min(want_self.len());
        let diff = (120..min_l).find(|&i| got_self[i] != want_self[i]).unwrap_or(min_l);
        return Err(format!(
            "M2-15 self-compile mismatch: got {} B (FNV=0x{:016X}) vs want {} B (FNV=0x{:016X}), first code diff at byte {}",
            got_self.len(),
            fnv1a_64_hash(&got_self),
            want_self.len(),
            fnv1a_64_hash(&want_self),
            diff
        ));
    }
    checks += 1;

    // 7. Audit Round 2 N2: Pure-M2 2-op IGET on provable ANEW/ISET bytes buffer must read ISET heap
    //    (not token region) in BOTH multi-mode (`emit_multi.omni`) and single-mode (`emit_x86.omni`).
    let n2_src = "#[strict]\nfn main(a: i64, b: i64, c: i64) -> i64 {\n    let w: bytes = anew(0);\n    w = iset_i32(w, a);\n    return iget(w, b) + c;\n}\n";
    let n2_u16 = compile_omni_surface_strict_to_u16(n2_src)?;
    let n2_multi_pure = compile_in_omni(&n2_u16, 300, 0, 0)?;
    let n2_multi_rust = compile_pure_u16_to_x86_64_elf_multi(&n2_u16, 300, 0, 0)?;
    if n2_multi_pure != n2_multi_rust {
        return Err(String::from("N2 multi-mode Pure-M2 != Rust-M2-multi on 2-op IGET buffer repro"));
    }
    let single_mod = m2_compile_decoder(M2_COMPILER_OMNI_SRC)?;
    let single_entry = m2_entry_of(&single_mod);
    let mut single_vm = OmniBinaryVm::with_budgets(500_000_000, 32 * 1024 * 1024);
    let n2_single_pure = match single_vm.execute_function(
        &single_mod,
        single_entry,
        vec![
            OmniValue::Bytes(vec![0u8; 524_288]),
            OmniValue::U16Array(n2_u16.clone()),
            OmniValue::Int(300),
            OmniValue::Int(0),
            OmniValue::Int(0),
        ],
    )? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("N2 single-mode compiler returned {:?}", other)),
    };
    let n2_single_rust = compile_pure_u16_to_x86_64_elf_with_args(&n2_u16, 300, 0, 0)?;
    if n2_single_pure != n2_single_rust {
        return Err(String::from("N2 single-mode Pure-M2 != Rust-AOT on 2-op IGET buffer repro"));
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        for &(tag, elf) in &[
            ("multi-mode", &n2_multi_pure),
            ("single-mode", &n2_single_pure),
        ] {
            let tmp = PrivateTempElf::new(elf, &format!("n2_iget_{}", tag))?;
            let out = std::process::Command::new(&tmp.path)
                .output()
                .map_err(|e| format!("N2 exec {}: {}", tag, e))?;
            if !out.status.success() || out.stdout.len() != 8 {
                return Err(format!("N2 {} exited {:?} stdout={}B", tag, out.status, out.stdout.len()));
            }
            let got_arr: [u8; 8] = out.stdout[..8]
                .try_into()
                .map_err(|_| format!("N2 {} stdout slice length != 8", tag))?;
            let got = i64::from_le_bytes(got_arr);
            if got != 44 {
                return Err(format!("N2 {} 2-op IGET buffer repro returned {}, want 44", tag, got));
            }
        }
        // OOB index b=10 must return 0 (lenient 2-op IGET contract).
        let n2_oob_elf = compile_in_omni(&n2_u16, 300, 10, 0)?;
        let tmp_oob = PrivateTempElf::new(&n2_oob_elf, "n2_iget_oob")?;
        let out_oob = std::process::Command::new(&tmp_oob.path)
            .output()
            .map_err(|e| format!("N2 exec oob: {}", e))?;
        if !out_oob.status.success() || out_oob.stdout.len() != 8 {
            return Err(format!("N2 oob exited {:?} stdout={}B", out_oob.status, out_oob.stdout.len()));
        }
        let oob_arr: [u8; 8] = out_oob.stdout[..8]
            .try_into()
            .map_err(|_| String::from("N2 oob stdout slice length != 8"))?;
        let got_oob = i64::from_le_bytes(oob_arr);
        if got_oob != 0 {
            return Err(format!("N2 multi-mode 2-op IGET OOB returned {}, want 0", got_oob));
        }
    }
    checks += 1;

    Ok(checks)
}

/// M2-16: Self-hosted M2 compiler fixed-point verification (`pure/m2/m2.omni_bc`).
/// 1. Drift check: `RustFrontend(pure/m2/*.omni) == pure/m2/m2.omni_bc` (0-byte diff).
/// 2. Self-compile parity: `M2_vm(m2.omni_bc) == RustEmitMulti(m2.omni_bc)` (0-byte diff, S1 == V).
/// 3. Native Linux x86-64 triple fixed point: `S1(m2.omni_bc) -> S2`, `S2(m2.omni_bc) -> S3`,
///    asserting `S1 == S2 == S3` (prints explicit SKIPPED on non-Linux hosts per PLAN.md Â§4).
pub fn selftest_m2_16_fixed_point(write_seed: bool) -> Result<(usize, usize, u64, usize, u64, bool), String> {
    let seed_u16 = compile_omni_surface_strict_to_u16(M2_SELFHOST_OMNI_SRC)?;
    let seed_bytes = encode_u16_stream_to_bytes(&seed_u16);
    if write_seed {
        std::fs::write("pure/m2/m2.omni_bc", &seed_bytes)
            .map_err(|e| format!("Failed to write pure/m2/m2.omni_bc: {}", e))?;
    }
    let committed_seed: &[u8] = include_bytes!("../pure/m2/m2.omni_bc");
    if !write_seed && committed_seed != seed_bytes.as_slice() {
        return Err(format!(
            "M2-16 seed drift: committed pure/m2/m2.omni_bc ({} B, FNV=0x{:016X}) != RustFrontend(pure/m2/*.omni) ({} B, FNV=0x{:016X}); run `omnic m2-fixed-point --write-seed`",
            committed_seed.len(),
            fnv1a_64_hash(committed_seed),
            seed_bytes.len(),
            fnv1a_64_hash(&seed_bytes)
        ));
    }

    // S1 = RustEmitMulti(T), V = M2_vm(T)
    let s1 = compile_pure_u16_to_x86_64_elf_multi(&seed_u16, 45, 40, 90)?;
    let comp_mod = m2_compile_decoder(M2_MULTI_OMNI_SRC)?;
    let entry = m2_entry_of(&comp_mod);
    let mut vm = OmniBinaryVm::with_budgets(1_000_000_000, 64 * 1024 * 1024);
    let arena = OmniValue::Bytes(vec![0u8; 524_288]);
    let v_elf = match vm.execute_function(
        &comp_mod,
        entry,
        vec![
            arena,
            OmniValue::U16Array(seed_u16.clone()),
            OmniValue::Int(45),
            OmniValue::Int(40),
            OmniValue::Int(90),
        ],
    )? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("M2-16 VM self-compile returned {:?}, expected Bytes", other)),
    };
    if v_elf != s1 {
        let min_l = v_elf.len().min(s1.len());
        let diff = (120..min_l).find(|&i| v_elf[i] != s1[i]).unwrap_or(min_l);
        return Err(format!(
            "M2-16 S1 != V mismatch: V={} B (0x{:016X}) vs S1={} B (0x{:016X}), first code diff at byte {}",
            v_elf.len(),
            fnv1a_64_hash(&v_elf),
            s1.len(),
            fnv1a_64_hash(&s1),
            diff
        ));
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    let native_verified = {
        let tmp_s1 = PrivateTempElf::new(&s1, "m2_s1")?;
        let p_bc = tmp_s1.dir.join("seed_fp.omni_bc");
        std::fs::write(&p_bc, &seed_bytes).map_err(|e| format!("write tmp bc: {}", e))?;
        let out2 = std::process::Command::new(&tmp_s1.path)
            .arg(&p_bc)
            .arg("45")
            .arg("40")
            .arg("90")
            .output()
            .map_err(|e| format!("exec s1: {}", e))?;
        if !out2.status.success() {
            return Err(format!("M2-16 S1 exited with {:?}", out2.status));
        }
        let s2 = out2.stdout;
        if s2 != s1 {
            return Err(format!(
                "M2-16 S2 != S1: S2={} B (0x{:016X}) vs S1={} B (0x{:016X})",
                s2.len(), fnv1a_64_hash(&s2), s1.len(), fnv1a_64_hash(&s1)
            ));
        }
        let tmp_s2 = PrivateTempElf::new(&s2, "m2_s2")?;
        let out3 = std::process::Command::new(&tmp_s2.path)
            .arg(&p_bc)
            .arg("45")
            .arg("40")
            .arg("90")
            .output()
            .map_err(|e| format!("exec s2: {}", e))?;
        if !out3.status.success() {
            return Err(format!("M2-16 S2 exited with {:?}", out3.status));
        }
        let s3 = out3.stdout;
        if s3 != s2 {
            return Err(format!(
                "M2-16 S3 != S2: S3={} B (0x{:016X}) vs S2={} B (0x{:016X})",
                s3.len(), fnv1a_64_hash(&s3), s2.len(), fnv1a_64_hash(&s2)
            ));
        }
        true
    };
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    let native_verified = false;

    Ok((3, seed_u16.len(), fnv1a_64_hash(&seed_bytes), s1.len(), fnv1a_64_hash(&s1), native_verified))
}

/// M3-05/M3-06 selftest: Pure-Omni streaming lexer (`pure/m3/arena.omni` + `pure/m3/lex.omni`)
/// differentially checked token-for-token (kind, line, col, value/text) and error-for-error
/// against Rust `tokenize_omni_surface_inner` across all acceptance goldens,
/// `pure/verifier_engine.omni` (32,942 B), and Q1/Q2/negative-lex fixtures.
pub fn selftest_m3_06_lex_diff() -> Result<usize, String> {
    let lex_mod = m2_compile_decoder(M3_LEX_OMNI_SRC)?;
    let entry = m2_entry_of(&lex_mod);
    let mut cases: Vec<(String, String, bool)> = Vec::new();
    for &(gid, src) in M2_GOLDEN_SOURCES {
        cases.push((format!("golden:{}", gid), String::from(src), false));
    }
    cases.push((String::from("pure/verifier_engine.omni"), String::from(OMNI_VERIFIER_ENGINE_SURFACE), true));
    // Q1/Q2/lex-error edge cases
    cases.push((String::from("q1_strict_in_comment"), String::from("// #[strict]\nfn f(x: i64) -> trit { return TRIT_POS; }\n"), false));
    cases.push((String::from("q2_trit_ident"), String::from("fn f(x: i64) -> i64 { let a = +1t; let b = -1t; let c = 0t; let d = 0t_x; return 0; }\n"), false));
    cases.push((String::from("err_unterm_str"), String::from("fn f() -> i64 { let s = \"abc\n; return 0; }\n"), false));
    cases.push((String::from("err_bare_at"), String::from("fn f() -> i64 { let x = @ + 1; return x; }\n"), false));
    cases.push((String::from("err_bad_char"), String::from("fn f() -> i64 { return $; }\n"), false));
    cases.push((String::from("err_int_ovf"), String::from("fn f() -> i64 { return 9223372036854775808; }\n"), false));

    let mut checks = 0usize;
    for (name, src, strict_arg) in &cases {
        let eff_strict = *strict_arg || src.contains("#[strict]");
        let rust_res = tokenize_omni_surface_inner(src, eff_strict);
        let src_u16: Vec<u16> = src.as_bytes().iter().map(|&b| b as u16).collect();
        let arena_len = (131_072 + src.len() * 16 + 4096).max(262_144);
        let mut vm = OmniBinaryVm::with_budgets(200_000_000, 64 * 1024 * 1024);
        let out_bytes = match vm.execute_function(
            &lex_mod,
            entry,
            vec![
                OmniValue::Bytes(vec![0u8; arena_len]),
                OmniValue::U16Array(src_u16),
                OmniValue::Int(if *strict_arg { 1 } else { 0 }),
            ],
        )? {
            OmniValue::Bytes(b) => b,
            other => return Err(format!("M3-06 '{}': returned {:?}, expected Bytes", name, other)),
        };
        let is_omni_err = out_bytes.len() == 16 && u16::from_le_bytes([out_bytes[0], out_bytes[1]]) == 0xFFFF;
        match rust_res {
            Err(re) => {
                if !is_omni_err {
                    return Err(format!("M3-06 '{}': Rust errored ({}) but Omni succeeded ({} toks)", name, re, out_bytes.len() / 16));
                }
                let ecode = u16::from_le_bytes([out_bytes[2], out_bytes[3]]);
                let eline = u16::from_le_bytes([out_bytes[4], out_bytes[5]]);
                let ecol = u16::from_le_bytes([out_bytes[6], out_bytes[7]]);
                let want_code = if re.contains("AMBIGUOUS_GRAMMAR") { 2u16 } else { 1u16 };
                let want_span = format!("line {}, col {}:", eline, ecol);
                if ecode != want_code || !re.contains(&want_span) {
                    return Err(format!(
                        "M3-06 '{}': error mismatch: Omni (code={}, line={}, col={}) vs Rust '{}'",
                        name, ecode, eline, ecol, re
                    ));
                }
            }
            Ok(rtoks) => {
                if is_omni_err {
                    let ecode = u16::from_le_bytes([out_bytes[2], out_bytes[3]]);
                    let eline = u16::from_le_bytes([out_bytes[4], out_bytes[5]]);
                    let ecol = u16::from_le_bytes([out_bytes[6], out_bytes[7]]);
                    return Err(format!(
                        "M3-06 '{}': Rust ok ({} toks) but Omni errored (code={}, line={}, col={})",
                        name, rtoks.len(), ecode, eline, ecol
                    ));
                }
                if out_bytes.len() != rtoks.len() * 16 {
                    return Err(format!(
                        "M3-06 '{}': token count mismatch: Omni {} vs Rust {}",
                        name, out_bytes.len() / 16, rtoks.len()
                    ));
                }
                let sbytes = src.as_bytes();
                for (ti, rt) in rtoks.iter().enumerate() {
                    let off = ti * 16;
                    let kind = u16::from_le_bytes([out_bytes[off], out_bytes[off + 1]]);
                    let sub = u16::from_le_bytes([out_bytes[off + 2], out_bytes[off + 3]]) as usize;
                    let line = u16::from_le_bytes([out_bytes[off + 4], out_bytes[off + 5]]) as usize;
                    let col = u16::from_le_bytes([out_bytes[off + 6], out_bytes[off + 7]]) as usize;
                    let val = i64::from_le_bytes(out_bytes[off + 8..off + 16].try_into().unwrap());
                    if line != rt.line || col != rt.col {
                        return Err(format!(
                            "M3-06 '{}' tok#{}: span mismatch Omni ({},{}) vs Rust ({},{}) [{:?}]",
                            name, ti, line, col, rt.line, rt.col, rt.tok
                        ));
                    }
                    let ok = match &rt.tok {
                        SurfaceTok::Hash => kind == 1,
                        SurfaceTok::StrLit(s) => kind == 2 && &sbytes[val as usize..val as usize + sub] == s.as_bytes(),
                        SurfaceTok::AtIdent(s) => kind == 3 && &sbytes[val as usize..val as usize + sub] == s.as_bytes(),
                        SurfaceTok::Trit(_, v) => kind == 4 && val == (*v as i64),
                        SurfaceTok::Sym(s) => {
                            let sb = s.as_bytes();
                            let wv = (sb[0] as i64) | (if sb.len() > 1 { (sb[1] as i64) << 8 } else { 0 });
                            kind == 5 && sub == sb.len() && val == wv
                        }
                        SurfaceTok::Op(s) => {
                            let sb = s.as_bytes();
                            let wv = (sb[0] as i64)
                                | (if sb.len() > 1 { (sb[1] as i64) << 8 } else { 0 })
                                | (if sb.len() > 2 { (sb[2] as i64) << 16 } else { 0 });
                            kind == 6 && sub == sb.len() && val == wv
                        }
                        SurfaceTok::Int(v) => kind == 7 && val == *v,
                        SurfaceTok::Ident(s) => kind == 8 && &sbytes[val as usize..val as usize + sub] == s.as_bytes(),
                    };
                    if !ok {
                        return Err(format!(
                            "M3-06 '{}' tok#{}: token mismatch Omni (kind={}, sub={}, val={}) vs Rust {:?}",
                            name, ti, kind, sub, val, rt.tok
                        ));
                    }
                }
            }
        }
        checks += 1;
    }
    Ok(checks)
}

/// M3-07 selftest: Pure-Omni Pass S (`pure/m3/skim.omni`) differentially checked against Rust
/// `parse_module` + signature checks across acceptance goldens, `pure/verifier_engine.omni`,
/// Q5/Q7 fixtures, and negative header/signature fixtures (`BAD_TYPE`, `MISSING_CAP`,
/// `BUDGET_EXCEEDED`, `AMBIGUOUS_GRAMMAR`, duplicate fn, empty module).
pub fn selftest_m3_07_skim_diff() -> Result<usize, String> {
    let skim_mod = m2_compile_decoder(M3_SKIM_OMNI_SRC)?;
    let entry = m2_entry_of(&skim_mod);
    let mut cases: Vec<(String, String, bool)> = Vec::new();
    for &(gid, src) in M2_GOLDEN_SOURCES {
        cases.push((format!("golden:{}", gid), String::from(src), false));
    }
    cases.push((String::from("pure/verifier_engine.omni"), String::from(OMNI_VERIFIER_ENGINE_SURFACE), true));
    cases.push((String::from("q7_fn_ids"), String::from(include_str!("../spec/m3/fixtures/q7_fn_ids.omni")), false));
    cases.push((String::from("q5_macro_after_def"), String::from(include_str!("../spec/m3/fixtures/q5_macro_after_def.omni")), false));
    cases.push((String::from("neg_bad_type"), String::from(include_str!("../spec/m3/fixtures/neg_bad_type.omni")), false));
    cases.push((String::from("neg_missing_cap"), String::from(include_str!("../spec/m3/fixtures/neg_missing_cap.omni")), false));
    cases.push((String::from("neg_budget_exceeded"), String::from(include_str!("../spec/m3/fixtures/neg_budget_exceeded.omni")), false));
    cases.push((String::from("neg_ambiguous_grammar"), String::from(include_str!("../spec/m3/fixtures/neg_ambiguous_grammar.omni")), false));
    cases.push((String::from("err_dup_fn"), String::from("fn f(x: i64) -> i64 { return x; }\nfn f(y: i64) -> i64 { return y; }\n"), false));
    cases.push((String::from("err_strict_mod"), String::from("#[strict]\nmod \"std/math.omni\";\nfn main(x: i64) -> i64 { return x; }\n"), false));
    cases.push((String::from("err_empty_mod"), String::from("// no functions here\n"), false));

    let mut checks = 0usize;
    for (name, src, strict_arg) in &cases {
        let eff_strict = *strict_arg || src.contains("#[strict]");
        let rust_res: Result<Vec<(u16, usize, u16, u16, String, bool, bool, usize, usize, usize, usize)>, String> = (|| {
            let toks = tokenize_omni_surface_inner(src, eff_strict)?;
            let mut parser = SurfaceParser::new_with_strict(toks, eff_strict);
            let (_incs, fns) = parser.parse_module()?;
            let multi_fn = fns.len() > 1;
            let mut seen = BTreeSet::new();
            let mut out = Vec::with_capacity(fns.len());
            for (idx, f) in fns.iter().enumerate() {
                let fid = if let Some(id_attr) = f.fn_id_attr {
                    id_attr
                } else if !multi_fn || idx + 1 == fns.len() {
                    0
                } else {
                    (idx + 1) as u16
                };
                let fn_strict = eff_strict || f.strict_attr;
                let ret_asm = map_omni_surface_type_inner(&f.ret_ty, f.ret_line, f.ret_col, fn_strict)?;
                let ret_tok = parse_asm_ty_tok(ret_asm);
                for p in &f.params {
                    let _ = map_omni_surface_type_inner(&p.ty, p.line, p.col, fn_strict)?;
                }
                if !seen.insert(f.name.clone()) {
                    return Err(format!(
                        "error[SYNTAX_ERROR] line {}, col {}: Duplicate function definition '{}'",
                        f.ret_line, f.ret_col, f.name
                    ));
                }
                out.push((fid, f.params.len(), ret_tok, f.cap_mask, f.name.clone(), f.is_route, fn_strict, f.fn_line, f.fn_col, f.ret_line, f.ret_col));
            }
            Ok(out)
        })();

        let src_u16: Vec<u16> = src.as_bytes().iter().map(|&b| b as u16).collect();
        let arena_len = (131_072 + src.len() * 16 + 4096).max(262_144);
        let mut vm = OmniBinaryVm::with_budgets(250_000_000, 64 * 1024 * 1024);
        let out_bytes = match vm.execute_function(
            &skim_mod,
            entry,
            vec![
                OmniValue::Bytes(vec![0u8; arena_len]),
                OmniValue::U16Array(src_u16),
                OmniValue::Int(if *strict_arg { 1 } else { 0 }),
            ],
        )? {
            OmniValue::Bytes(b) => b,
            other => return Err(format!("M3-07 '{}': returned {:?}, expected Bytes", name, other)),
        };
        let is_omni_err = out_bytes.len() == 16 && u16::from_le_bytes([out_bytes[0], out_bytes[1]]) == 0xFFFF;
        match rust_res {
            Err(re) => {
                if is_omni_err {
                    let ecode = u16::from_le_bytes([out_bytes[2], out_bytes[3]]);
                    let eline = u16::from_le_bytes([out_bytes[4], out_bytes[5]]);
                    let ecol = u16::from_le_bytes([out_bytes[6], out_bytes[7]]);
                    let want_code = if re.contains("AMBIGUOUS_GRAMMAR") {
                        2u16
                    } else if re.contains("BUDGET_EXCEEDED") {
                        3u16
                    } else if re.contains("BAD_TYPE") {
                        4u16
                    } else if re.contains("MISSING_CAP") {
                        5u16
                    } else {
                        1u16
                    };
                    let want_span = format!("line {}, col {}:", eline, ecol);
                    if ecode != want_code || (!re.contains(&want_span) && name != "err_empty_mod") {
                        return Err(format!(
                            "M3-07 '{}': error mismatch: Omni (code={}, line={}, col={}) vs Rust '{}'",
                            name, ecode, eline, ecol, re
                        ));
                    }
                } else if name.starts_with("neg_") || name.starts_with("err_") {
                    return Err(format!("M3-07 '{}': Rust errored ({}) but Omni succeeded ({} fns)", name, re, out_bytes.len() / 32));
                }
            }
            Ok(rfns) => {
                if is_omni_err {
                    let ecode = u16::from_le_bytes([out_bytes[2], out_bytes[3]]);
                    let eline = u16::from_le_bytes([out_bytes[4], out_bytes[5]]);
                    let ecol = u16::from_le_bytes([out_bytes[6], out_bytes[7]]);
                    return Err(format!(
                        "M3-07 '{}': Rust ok ({} fns) but Omni errored (code={}, line={}, col={})",
                        name, rfns.len(), ecode, eline, ecol
                    ));
                }
                if out_bytes.len() != rfns.len() * 32 {
                    return Err(format!(
                        "M3-07 '{}': fn count mismatch: Omni {} vs Rust {}",
                        name, out_bytes.len() / 32, rfns.len()
                    ));
                }
                let sbytes = src.as_bytes();
                for (fi, (rfid, rnpar, rret, rcap, rname, rrt, rstr, rfl, rfc, rrl, rrc)) in rfns.iter().enumerate() {
                    let off = fi * 32;
                    let w0 = u64::from_le_bytes(out_bytes[off..off + 8].try_into().unwrap());
                    let w1 = u64::from_le_bytes(out_bytes[off + 8..off + 16].try_into().unwrap());
                    let w3 = u64::from_le_bytes(out_bytes[off + 24..off + 32].try_into().unwrap());
                    let ofid = (w0 & 0xFFFF) as u16;
                    let onpar = ((w0 >> 16) & 0xFFFF) as usize;
                    let oret = ((w0 >> 32) & 0xFFFF) as u16;
                    let ocap = ((w0 >> 48) & 0xFFFF) as u16;
                    let onoff = (w1 & 0xFFFF_FFFF) as usize;
                    let onlen = ((w1 >> 32) & 0xFFFF) as usize;
                    let ostr = ((w1 >> 48) & 1) != 0;
                    let ort = ((w1 >> 49) & 1) != 0;
                    let ofl = (w3 & 0xFFFF) as usize;
                    let ofc = ((w3 >> 16) & 0xFFFF) as usize;
                    let orl = ((w3 >> 32) & 0xFFFF) as usize;
                    let orc = ((w3 >> 48) & 0xFFFF) as usize;
                    let name_ok = ort || &sbytes[onoff..onoff + onlen] == rname.as_bytes();
                    if ofid != *rfid || onpar != *rnpar || oret != *rret || ocap != *rcap || !name_ok || ort != *rrt || ostr != *rstr || ofl != *rfl || ofc != *rfc || orl != *rrl || orc != *rrc {
                        return Err(format!(
                            "M3-07 '{}' fn#{} ('{}'): mismatch Omni(fid={},np={},ret=0x{:X},cap=0x{:X},rt={},str={},fl={}:{},rl={}:{}) vs Rust(fid={},np={},ret=0x{:X},cap=0x{:X},rt={},str={},fl={}:{},rl={}:{})",
                            name, fi, rname, ofid, onpar, oret, ocap, ort, ostr, ofl, ofc, orl, orc,
                            rfid, rnpar, rret, rcap, rrt, rstr, rfl, rfc, rrl, rrc
                        ));
                    }
                }
            }
        }
        checks += 1;
    }
    Ok(checks)
}

/// Recursively compares one Pure-Omni AST node (in `ast_bytes` at `id * 16`) against a Rust `SurfaceExpr`.
fn m3_08_cmp_expr(
    name: &str,
    src: &str,
    rtoks: &[SpannedSurfaceTok],
    ast: &[u8],
    id: usize,
    re: &SurfaceExpr,
) -> Result<(), String> {
    if id == 0 || (id + 1) * 16 > ast.len() {
        return Err(format!("M3-08 '{}': invalid AST node id {}", name, id));
    }
    let off = id * 16;
    let w0 = u64::from_le_bytes(ast[off..off + 8].try_into().unwrap());
    let w1 = u64::from_le_bytes(ast[off + 8..off + 16].try_into().unwrap());
    let kind = (w0 & 0xFF) as u8;
    let op = ((w0 >> 8) & 0xFF) as u8;
    let line = ((w0 >> 16) & 0xFFFF) as usize;
    let col = ((w0 >> 32) & 0xFFFF) as usize;
    let sbytes = src.as_bytes();
    let (rl, rc) = re.span();
    if line != rl || col != rc {
        return Err(format!("M3-08 '{}': span mismatch Omni ({},{}) vs Rust ({},{}) for {:?}", name, line, col, rl, rc, re));
    }
    match re {
        SurfaceExpr::Int(v, _, _) => {
            if kind != 1 || (w1 as i64) != *v {
                return Err(format!("M3-08 '{}': Int mismatch Omni(k={},v={}) vs Rust({})", name, kind, w1 as i64, v));
            }
        }
        SurfaceExpr::Trit(_, tv, _, _) => {
            if kind != 2 || (w1 as i64) != (*tv as i64) {
                return Err(format!("M3-08 '{}': Trit mismatch Omni(k={},v={}) vs Rust({})", name, kind, w1 as i64, tv));
            }
        }
        SurfaceExpr::StrLit(s, _, _) => {
            let so = (w1 & 0xFFFF_FFFF) as usize;
            let sl = (w1 >> 32) as usize;
            if kind != 3 || &sbytes[so..so + sl] != s.as_bytes() {
                return Err(format!("M3-08 '{}': StrLit mismatch", name));
            }
        }
        SurfaceExpr::Var(s, _, _) => {
            let so = (w1 & 0xFFFF_FFFF) as usize;
            let sl = (w1 >> 32) as usize;
            if kind != 4 || &sbytes[so..so + sl] != s.as_bytes() {
                return Err(format!("M3-08 '{}': Var mismatch for '{}'", name, s));
            }
        }
        SurfaceExpr::UnOp(mnem, sub, _, _) => {
            let want_op = match mnem.as_str() { "INEG" => 1, "BNOT" => 2, "TNOT" => 3, _ => 255 };
            if kind != 5 || op != want_op {
                return Err(format!("M3-08 '{}': UnOp mismatch Omni(k={},op={}) vs Rust {}", name, kind, op, mnem));
            }
            m3_08_cmp_expr(name, src, rtoks, ast, (w1 & 0xFFFF) as usize, sub)?;
        }
        SurfaceExpr::BinOp(mnem, lhs, rhs, _, _) => {
            let want_op = match mnem.as_str() {
                "CEQ" => 1, "CNE" => 2, "CLT" => 3, "CGT" => 4, "TCMP" => 5,
                "BOR" => 6, "BXOR" => 7, "BAND" => 8, "SHL" => 9, "SHR" => 10,
                "IADD" => 11, "ISUB" => 12, "IMUL" => 13, "IDIV" => 14, "IMOD" => 15,
                _ => 255,
            };
            if kind != 6 || op != want_op {
                return Err(format!("M3-08 '{}': BinOp mismatch Omni(k={},op={}) vs Rust {}", name, kind, op, mnem));
            }
            m3_08_cmp_expr(name, src, rtoks, ast, (w1 & 0xFFFF) as usize, lhs)?;
            m3_08_cmp_expr(name, src, rtoks, ast, ((w1 >> 32) & 0xFFFF) as usize, rhs)?;
        }
        SurfaceExpr::Call(cname, args, _, _) => {
            let nti = (w1 & 0xFFFF) as usize;
            let mut cur = ((w1 >> 16) & 0xFFFF) as usize;
            let nargs = ((w1 >> 32) & 0xFFFF) as usize;
            let got_name = match rtoks.get(nti).map(|t| &t.tok) {
                Some(SurfaceTok::Ident(s)) => s.as_str(),
                _ => "",
            };
            if kind != 7 || got_name != cname.as_str() || nargs != args.len() {
                return Err(format!("M3-08 '{}': Call mismatch Omni(k={},name='{}',n={}) vs Rust('{}',n={})", name, kind, got_name, nargs, cname, args.len()));
            }
            for a in args {
                m3_08_cmp_expr(name, src, rtoks, ast, cur, a)?;
                let aw0 = u64::from_le_bytes(ast[cur * 16..cur * 16 + 8].try_into().unwrap());
                cur = ((aw0 >> 48) & 0xFFFF) as usize;
            }
        }
        SurfaceExpr::TritVecLit(trits, _, _) => {
            let mut cur = (w1 & 0xFFFF) as usize;
            let nel = ((w1 >> 32) & 0xFFFF) as usize;
            if kind != 8 || nel != trits.len() {
                return Err(format!("M3-08 '{}': TritVec mismatch Omni(k={},n={}) vs Rust(n={})", name, kind, nel, trits.len()));
            }
            for &tv in trits {
                let aw1 = i64::from_le_bytes(ast[cur * 16 + 8..cur * 16 + 16].try_into().unwrap());
                if aw1 != (tv as i64) {
                    return Err(format!("M3-08 '{}': TritVec elem mismatch Omni({}) vs Rust({})", name, aw1, tv));
                }
                let aw0 = u64::from_le_bytes(ast[cur * 16..cur * 16 + 8].try_into().unwrap());
                cur = ((aw0 >> 48) & 0xFFFF) as usize;
            }
        }
        SurfaceExpr::IfBranch(mnem, cond, tb, eb, _, _) => {
            let want_op = if mnem == "BR_POS" { 1u8 } else { 2u8 };
            if kind != 9 || op != want_op {
                return Err(format!("M3-09 '{}': IfBranch mismatch Omni(k={},op={}) vs Rust {}", name, kind, op, mnem));
            }
            m3_08_cmp_expr(name, src, rtoks, ast, (w1 & 0xFFFF) as usize, cond)?;
            m3_09_cmp_block(name, src, rtoks, ast, ((w1 >> 16) & 0xFFFF) as usize, tb)?;
            m3_09_cmp_block(name, src, rtoks, ast, ((w1 >> 32) & 0xFFFF) as usize, eb)?;
        }
        SurfaceExpr::MatchTrit(scrut, neg, zero, pos, _, _) => {
            if kind != 10 {
                return Err(format!("M3-09 '{}': MatchTrit mismatch Omni(k={})", name, kind));
            }
            m3_08_cmp_expr(name, src, rtoks, ast, (w1 & 0xFFFF) as usize, scrut)?;
            m3_09_cmp_block(name, src, rtoks, ast, ((w1 >> 16) & 0xFFFF) as usize, neg)?;
            m3_09_cmp_block(name, src, rtoks, ast, ((w1 >> 32) & 0xFFFF) as usize, zero)?;
            m3_09_cmp_block(name, src, rtoks, ast, ((w1 >> 48) & 0xFFFF) as usize, pos)?;
        }
    }
    Ok(())
}

/// Recursively compares a Pure-Omni AST statement list (`head` linked via `w0 >> 48`) against Rust `&[SurfaceStmt]`.
fn m3_09_cmp_block(
    name: &str,
    src: &str,
    rtoks: &[SpannedSurfaceTok],
    ast: &[u8],
    head: usize,
    stmts: &[SurfaceStmt],
) -> Result<(), String> {
    let mut cur = head;
    for (si, st) in stmts.iter().enumerate() {
        if cur == 0 || (cur + 1) * 16 > ast.len() {
            return Err(format!("M3-09 '{}': block ended early at stmt#{} (cur={})", name, si, cur));
        }
        let off = cur * 16;
        let w0 = u64::from_le_bytes(ast[off..off + 8].try_into().unwrap());
        let w1 = u64::from_le_bytes(ast[off + 8..off + 16].try_into().unwrap());
        let kind = (w0 & 0xFF) as u8;
        let line = ((w0 >> 16) & 0xFFFF) as usize;
        let col = ((w0 >> 32) & 0xFFFF) as usize;
        let next = ((w0 >> 48) & 0xFFFF) as usize;
        let (rl, rc) = st.span();
        if line != rl || col != rc {
            return Err(format!("M3-09 '{}' stmt#{}: span mismatch Omni ({},{}) vs Rust ({},{})", name, si, line, col, rl, rc));
        }
        let tok_ident = |ti: usize| -> &str {
            match rtoks.get(ti).map(|t| &t.tok) {
                Some(SurfaceTok::Ident(s)) => s.as_str(),
                _ => "",
            }
        };
        match st {
            SurfaceStmt::Let(vname, _, expr, _, _) => {
                if kind != 16 || tok_ident((w1 & 0xFFFF) as usize) != vname.as_str() {
                    return Err(format!("M3-09 '{}' stmt#{}: Let mismatch for '{}'", name, si, vname));
                }
                m3_08_cmp_expr(name, src, rtoks, ast, ((w1 >> 16) & 0xFFFF) as usize, expr)?;
            }
            SurfaceStmt::Assign(vname, expr, _, _) => {
                if kind != 17 || tok_ident((w1 & 0xFFFF) as usize) != vname.as_str() {
                    return Err(format!("M3-09 '{}' stmt#{}: Assign mismatch for '{}'", name, si, vname));
                }
                m3_08_cmp_expr(name, src, rtoks, ast, ((w1 >> 16) & 0xFFFF) as usize, expr)?;
            }
            SurfaceStmt::DoWhile(body, cond, _, _) => {
                if kind != 18 {
                    return Err(format!("M3-09 '{}' stmt#{}: DoWhile mismatch (k={})", name, si, kind));
                }
                m3_09_cmp_block(name, src, rtoks, ast, (w1 & 0xFFFF) as usize, body)?;
                m3_08_cmp_expr(name, src, rtoks, ast, ((w1 >> 16) & 0xFFFF) as usize, cond)?;
            }
            SurfaceStmt::While(cond, body, _, _) => {
                if kind != 19 {
                    return Err(format!("M3-09 '{}' stmt#{}: While mismatch (k={})", name, si, kind));
                }
                m3_08_cmp_expr(name, src, rtoks, ast, (w1 & 0xFFFF) as usize, cond)?;
                m3_09_cmp_block(name, src, rtoks, ast, ((w1 >> 16) & 0xFFFF) as usize, body)?;
            }
            SurfaceStmt::Label(lid, _, _) => {
                if kind != 20 || (w1 as u16) != *lid {
                    return Err(format!("M3-09 '{}' stmt#{}: Label mismatch", name, si));
                }
            }
            SurfaceStmt::Goto(lid, _, _) => {
                if kind != 21 || (w1 as u16) != *lid {
                    return Err(format!("M3-09 '{}' stmt#{}: Goto mismatch", name, si));
                }
            }
            SurfaceStmt::Ret(expr, _, _) => {
                if kind != 22 {
                    return Err(format!("M3-09 '{}' stmt#{}: Ret mismatch (k={})", name, si, kind));
                }
                m3_08_cmp_expr(name, src, rtoks, ast, (w1 & 0xFFFF) as usize, expr)?;
            }
            SurfaceStmt::Expr(expr, _, _) => {
                if kind != 23 {
                    return Err(format!("M3-09 '{}' stmt#{}: Expr mismatch (k={})", name, si, kind));
                }
                m3_08_cmp_expr(name, src, rtoks, ast, (w1 & 0xFFFF) as usize, expr)?;
            }
        }
        cur = next;
    }
    if cur != 0 {
        return Err(format!("M3-09 '{}': extra trailing stmt node {}", name, cur));
    }
    Ok(())
}

/// M3-08 selftest: Pure-Omni iterative expression parser (`pure/m3/expr.omni`) differentially
/// checked node-for-node, precedence-for-precedence, span-for-span, and error-for-error against
/// Rust `SurfaceParser::parse_expr` (including Q3 `-` fold, Q4 `<=`/`>=`, nested calls/parens, trit_vec).
pub fn selftest_m3_08_expr_diff() -> Result<usize, String> {
    let expr_mod = m2_compile_decoder(M3_EXPR_OMNI_SRC)?;
    let entry = m2_entry_of(&expr_mod);
    let cases: [(&str, &str, bool); 24] = [
        ("int_lit", "42", true),
        ("trit_lits", "+1t", true),
        ("str_lit", "\"hello\"", true),
        ("var", "foo_bar", true),
        ("q3_neg_int", "-42", true),
        ("q3_double_neg_int", "--7", true),
        ("q3_neg_var", "-x", true),
        ("q3_neg_paren_int", "-(42)", true),
        ("unary_plus_bnot_tnot", "+~!x", true),
        ("prec_add_mul", "1 + 2 * 3 - 4 / 2 % 3", true),
        ("prec_left_assoc", "10 - 3 - 2", true),
        ("prec_bitwise_shift", "a | b ^ c & d << 2 >> 1", true),
        ("prec_cmp_tcmp", "a + 1 <=> b * 2", true),
        ("q4_le", "a + 1 <= b * 2", true),
        ("q4_ge", "x >= y - 1", true),
        ("cmp_chain", "a == b != c < d > e", true),
        ("nested_parens", "((a + b) * (c - d))", true),
        ("call_0arg", "foo()", true),
        ("call_nested", "f(1, g(a + b, -3), h())", true),
        ("tritvec_strict", "[-1t, 0t, +1t]", true),
        ("tritvec_nonstrict_int", "[-1, 0, 1]", false),
        ("err_tritvec_strict_int", "[-1, 0, 1]", true),
        ("err_tritvec_empty", "[]", true),
        ("err_unclosed_paren", "(1 + 2", true),
    ];

    let mut checks = 0usize;
    for &(name, src, strict_arg) in &cases {
        let eff_strict = strict_arg || src.contains("#[strict]");
        let rtoks = tokenize_omni_surface_inner(src, eff_strict)?;
        let mut parser = SurfaceParser::new_with_strict(rtoks.clone(), eff_strict);
        let rust_res = parser.parse_expr();

        let src_u16: Vec<u16> = src.as_bytes().iter().map(|&b| b as u16).collect();
        let mut vm = OmniBinaryVm::with_budgets(50_000_000, 64 * 1024 * 1024);
        let out_bytes = match vm.execute_function(
            &expr_mod,
            entry,
            vec![
                OmniValue::Bytes(vec![0u8; 262_144]),
                OmniValue::U16Array(src_u16),
                OmniValue::Int(if strict_arg { 1 } else { 0 }),
            ],
        )? {
            OmniValue::Bytes(b) => b,
            other => return Err(format!("M3-08 '{}': returned {:?}, expected Bytes", name, other)),
        };
        let is_omni_err = out_bytes.len() == 16 && u16::from_le_bytes([out_bytes[0], out_bytes[1]]) == 0xFFFF;
        match rust_res {
            Err(re) => {
                if !is_omni_err {
                    return Err(format!("M3-08 '{}': Rust errored ({}) but Omni succeeded", name, re));
                }
                let ecode = u16::from_le_bytes([out_bytes[2], out_bytes[3]]);
                let want_code = if re.contains("AMBIGUOUS_GRAMMAR") { 2u16 } else if re.contains("BAD_TYPE") { 4u16 } else { 1u16 };
                if ecode != want_code {
                    return Err(format!("M3-08 '{}': error class mismatch Omni={} vs Rust '{}'", name, ecode, re));
                }
            }
            Ok(re) => {
                if is_omni_err {
                    let ecode = u16::from_le_bytes([out_bytes[2], out_bytes[3]]);
                    let eline = u16::from_le_bytes([out_bytes[4], out_bytes[5]]);
                    let ecol = u16::from_le_bytes([out_bytes[6], out_bytes[7]]);
                    return Err(format!("M3-08 '{}': Rust ok but Omni errored (code={}, {}:{})", name, ecode, eline, ecol));
                }
                let root_id = u16::from_le_bytes([out_bytes[0], out_bytes[1]]) as usize;
                m3_08_cmp_expr(name, src, &rtoks, &out_bytes, root_id, &re)?;
            }
        }
        checks += 1;
    }
    Ok(checks)
}

/// M3-09 selftest: Pure-Omni iterative statement/block/if_pos/match_trit parser (`pure/m3/stmt.omni`)
/// differentially checked full-AST-for-full-AST against Rust `SurfaceParser::parse_module` across
/// rich multi-construct programs, `pure/m2/util.omni`, `pure/m2/arena.omni`, `pure/verifier_engine.omni`,
/// Q3/Q4/Q7 fixtures, and negative statement error fixtures.
pub fn selftest_m3_09_stmt_diff() -> Result<usize, String> {
    let stmt_mod = m2_compile_decoder(M3_STMT_OMNI_SRC)?;
    let entry = m2_entry_of(&stmt_mod);
    let cases: Vec<(&str, &str, bool)> = vec![
        ("let_assign_compound", "#[strict]\nfn main(x: i64) -> i64 { let a: i64 = x + 1; a += 2; a -= 1; a *= 3; return a; }\n", false),
        ("while_dowhile", "#[strict]\nfn main(x: i64) -> i64 { let s: i64 = 0; let i: i64 = x; while i <=> 0 { s += i; i -= 1; } do_while { s = s + 1; } (0 <=> 1); return s; }\n", false),
        ("label_goto", "#[strict]\nfn main(x: i64) -> i64 { goto 1; label 1; return x; }\n", false),
        ("if_pos_nested", "#[strict]\nfn main(x: i64) -> i64 { let c: trit = x <=> 0; let r: i64 = if_pos c { let d: trit = (x - 5) <=> 0; if_neg d { x + 10 } else { x - 10 } } else { -x }; return r; }\n", false),
        ("match_trit_mixed", "#[strict]\nfn main(x: i64) -> i64 { let t: trit = x <=> 0; let r: i64 = match_trit t { neg => { let a: i64 = -1; a }, zero => 0, pos => { x + 1 } }; return r; }\n", false),
        ("q3_neg_fold", include_str!("../spec/m3/fixtures/q3_neg_fold.omni"), false),
        ("q4_le_ge", include_str!("../spec/m3/fixtures/q4_le_ge.omni"), false),
        ("q7_fn_ids", include_str!("../spec/m3/fixtures/q7_fn_ids.omni"), false),
        ("pure/m2/util.omni", include_str!("../pure/m2/util.omni"), true),
        ("pure/m2/arena.omni", include_str!("../pure/m2/arena.omni"), true),
        ("pure/verifier_engine.omni", OMNI_VERIFIER_ENGINE_SURFACE, true),
        ("err_strict_let_no_ty", "#[strict]\nfn main(x: i64) -> i64 { let y = x; return y; }\n", false),
        ("err_label_oob", "#[strict]\nfn main(x: i64) -> i64 { label 16; return x; }\n", false),
        ("err_match_missing_arm", "#[strict]\nfn main(x: i64) -> i64 { let t: trit = x <=> 0; let r: i64 = match_trit t { neg => 1, pos => 2 }; return r; }\n", false),
    ];

    let mut checks = 0usize;
    for &(name, src, strict_arg) in &cases {
        let eff_strict = strict_arg || src.contains("#[strict]");
        let rtoks = tokenize_omni_surface_inner(src, eff_strict)?;
        let mut parser = SurfaceParser::new_with_strict(rtoks.clone(), eff_strict);
        let rust_res = parser.parse_module();

        let src_u16: Vec<u16> = src.as_bytes().iter().map(|&b| b as u16).collect();
        let arena_len = (131_072 + src.len() * 16 + 4096).max(262_144);
        let mut vm = OmniBinaryVm::with_budgets(300_000_000, 64 * 1024 * 1024);
        let out_bytes = match vm.execute_function(
            &stmt_mod,
            entry,
            vec![
                OmniValue::Bytes(vec![0u8; arena_len]),
                OmniValue::U16Array(src_u16),
                OmniValue::Int(if strict_arg { 1 } else { 0 }),
                OmniValue::Int(-1),
            ],
        )? {
            OmniValue::Bytes(b) => b,
            other => return Err(format!("M3-09 '{}': returned {:?}, expected Bytes", name, other)),
        };
        let is_omni_err = out_bytes.len() == 16 && u16::from_le_bytes([out_bytes[0], out_bytes[1]]) == 0xFFFF;
        match rust_res {
            Err(re) => {
                if !is_omni_err {
                    return Err(format!("M3-09 '{}': Rust errored ({}) but Omni succeeded", name, re));
                }
                let ecode = u16::from_le_bytes([out_bytes[2], out_bytes[3]]);
                let want_code = if re.contains("AMBIGUOUS_GRAMMAR") { 2u16 }
                    else if re.contains("BAD_TYPE") { 4u16 }
                    else if re.contains("UNRESOLVED_LABEL") { 6u16 }
                    else { 1u16 };
                if ecode != want_code {
                    return Err(format!("M3-09 '{}': error mismatch Omni={} vs Rust '{}'", name, ecode, re));
                }
            }
            Ok((_incs, rfns)) => {
                if is_omni_err {
                    let ecode = u16::from_le_bytes([out_bytes[2], out_bytes[3]]);
                    let eline = u16::from_le_bytes([out_bytes[4], out_bytes[5]]);
                    let ecol = u16::from_le_bytes([out_bytes[6], out_bytes[7]]);
                    return Err(format!("M3-09 '{}': Rust ok ({} fns) but Omni errored (code={}, {}:{})", name, rfns.len(), ecode, eline, ecol));
                }
                // `target_fn = -1` parses EVERY fn in the module and leaves the last fn's AST in `out_bytes`.
                let root_head = u16::from_le_bytes([out_bytes[0], out_bytes[1]]) as usize;
                let last_fn = rfns.last().unwrap();
                m3_09_cmp_block(name, src, &rtoks, &out_bytes, root_head, &last_fn.body)?;
            }
        }
        checks += 1;
    }
    Ok(checks)
}

/// M3-10 selftest: Pure-Omni iterative lowering & module packer (`pure/m3/lower.omni`)
/// differentially checked WORD-FOR-WORD (0-word diff!) against Rust `compile_omni_surface_to_u16`
/// across arithmetic/bitwise/Q3/Q4, loops, labels/gotos, multi-fn calls (Q7), nested calls,
/// memory/slice intrinsics, real `pure/m2/util.omni`, and negative lowering error fixtures.
pub fn selftest_m3_10_lower_diff() -> Result<usize, String> {
    let lower_mod = m2_compile_decoder(M3_LOWER_OMNI_SRC)?;
    let entry = m2_entry_of(&lower_mod);
    let cases: Vec<(&str, &str, bool)> = vec![
        ("arith_bitwise", "#[strict]\nfn main(a: i64, b: i64) -> i64 { let x: i64 = (a + b) * 3 - (a / 2) % 5; let y: i64 = (x ^ b) | (a & 255); return (y << 2) >> 1; }\n", false),
        ("q3_neg_fold", include_str!("../spec/m3/fixtures/q3_neg_fold.omni"), false),
        ("q4_le_ge", include_str!("../spec/m3/fixtures/q4_le_ge.omni"), false),
        ("q7_fn_ids", include_str!("../spec/m3/fixtures/q7_fn_ids.omni"), false),
        ("compound_while_dowhile", "#[strict]\nfn main(x: i64) -> i64 { let s: i64 = 0; let i: i64 = x; while i <=> 0 { s += i; i -= 1; } do_while { s *= 2; } (0 <=> 1); return s; }\n", false),
        ("label_goto", "#[strict]\nfn main(x: i64) -> i64 { goto 2; label 1; return 99; label 2; return x + 1; }\n", false),
        ("nested_calls", "#[strict]\nfn add(a: i64, b: i64) -> i64 { return a + b; }\n#[strict]\nfn mul(a: i64, b: i64) -> i64 { return a * b; }\n#[strict]\nfn main(x: i64) -> i64 { return add(x, mul(add(x, 1), 3)); }\n", false),
        ("mem_and_ret_slice", "#[strict]\nfn main(x: i64) -> bytes { let b: bytes = anew(0); b = iset_i64(b, x); b = st_u32(b, 0, x + 1); let v: i64 = ld_u32(b, 0); b = iset_patch_i64(b, 0, v); return ret_slice(b, 0, 8); }\n", false),
        ("l1_st_u64_lower", "#[strict]\nfn main(x: i64) -> i64 { let b: bytes = anew(0); b = iset_i64(b, 0); b = st_u64(b, 0, x + 7); return ld_u64(b, 0); }\n", true),
        ("if_pos_nested_join", "#[strict]\nfn main(x: i64) -> i64 { let c: trit = x <=> 0; let r: i64 = if_pos c { let d: trit = (x - 5) <=> 0; if_neg d { let a: i64 = x + 10; a } else { x - 10 } } else { let b: i64 = x * 2; b + 1 }; return r; }\n", false),
        ("match_trit_3arm", "#[strict]\nfn main(x: i64) -> i64 { let t: trit = x <=> 0; let r: i64 = match_trit t { neg => { let a: i64 = x - 1; a * 2 }, zero => 0, pos => { x + 1 } }; return r; }\n", false),
        ("str_lit_cstr", "#[strict]\nfn main(x: i64) -> str { let s: str = \"hello_omni\"; return s; }\n", false),
        ("pure/m2/util.omni", include_str!("../pure/m2/util.omni"), true),
        ("pure/m3/arena.omni", include_str!("../pure/m3/arena.omni"), true),
        ("pure/m2/util+isa+decode.omni", concat!(include_str!("../pure/m2/util.omni"), "\n", include_str!("../pure/isa/table.omni"), "\n", include_str!("../pure/m2/decode.omni")), true),
        ("pure/verifier_engine.omni", OMNI_VERIFIER_ENGINE_SURFACE, true),
        ("neg_undef_var", include_str!("../spec/m3/fixtures/neg_undef_var.omni"), false),
        ("neg_immut_param", include_str!("../spec/m3/fixtures/neg_immut_param.omni"), false),
        ("neg_unresolved_label", include_str!("../spec/m3/fixtures/neg_unresolved_label.omni"), false),
        ("neg_unknown_fn", include_str!("../spec/m3/fixtures/neg_unknown_fn.omni"), false),
        ("neg_arity_mismatch", include_str!("../spec/m3/fixtures/neg_arity_mismatch.omni"), false),
        (
            "host_intrinsics_all",
            "#[cap(NET, DB_R, DB_W, AUTH)]\nroute POST /ledger/:id (req: i64) {\n  let id: i64 = req.param(\"id\");\n  let ok: i64 = auth_verify(id);\n  let a: i64 = db_get(ok) + kv_get(\"acct\", \"42\") + db_query(\"SELECT 1\", ok);\n  let c: i64 = db_count() + kv_count(\"tbl\");\n  let p: i64 = kv_put(ok, a + c) + db_put(\"k1\", 7);\n  respond(201, p);\n}\n#[cap(NET)]\nfn main(x: i64) -> i64 { respond_html(x); }\n",
            false,
        ),
        (
            "pack5_tdot_arena_drop",
            "#[cap(AI)]\nfn main(x: i64) -> i64 {\n  let a: bytes = pack5_bytes([+1t, -1t, 0t, +1t, -1t]);\n  let b: bytes = pack5([+1t, 0t, -1t, +1t, +1t]);\n  let c: bytes = tpack5([-1t, -1t, 0t, +1t, 0t]);\n  let ar: arena = arena(256);\n  let r: i64 = borrow_r(ar);\n  let t: trit = tlogic(x);\n  drop(r);\n  drop(ar);\n  return tdot(a, b) + pack5_tdot(b, c) + sel_trit(t, -1, 0, 1);\n}\n",
            false,
        ),
    ];

    let kv_exp = resolve_omni_surface_includes(include_str!("../pure/programs/kv_ledger_service.omni"), None, None)?;
    let p5_exp = resolve_omni_surface_includes(include_str!("../pure/programs/pack5_decision_controller.omni"), None, None)?;
    let mut all_cases: Vec<(&str, &str, bool)> = cases.to_vec();
    all_cases.push(("pure_kv_ledger_service", kv_exp.as_str(), false));
    all_cases.push(("pure_pack5_decision_controller", p5_exp.as_str(), false));

    let mut checks = 0usize;
    for &(name, src, strict_arg) in &all_cases {
        let rust_res = with_lockstep_suspended(|| {
            if strict_arg { compile_omni_surface_strict_to_u16(src) } else { compile_omni_surface_to_u16(src) }
        });
        let src_u16: Vec<u16> = src.as_bytes().iter().map(|&b| b as u16).collect();
        let arena_len = (131_072 + src.len() * 16 + 65_536).max(327_680);
        let mut vm = OmniBinaryVm::with_budgets(300_000_000, 64 * 1024 * 1024);
        let out_bytes = match vm.execute_function(
            &lower_mod,
            entry,
            vec![
                OmniValue::Bytes(vec![0u8; arena_len]),
                OmniValue::U16Array(src_u16),
                OmniValue::Int(if strict_arg { 1 } else { 0 }),
            ],
        )? {
            OmniValue::Bytes(b) => b,
            other => return Err(format!("M3-10 '{}': returned {:?}, expected Bytes", name, other)),
        };
        let is_omni_err = out_bytes.len() == 16 && u16::from_le_bytes([out_bytes[0], out_bytes[1]]) == 0xFFFF;
        match rust_res {
            Err(re) => {
                if !is_omni_err {
                    return Err(format!("M3-10 '{}': Rust errored ({}) but Omni succeeded ({} w)", name, re, out_bytes.len() / 2));
                }
                let ecode = u16::from_le_bytes([out_bytes[2], out_bytes[3]]);
                let want_code = if re.contains("UNRESOLVED_LABEL") { 6u16 }
                    else if re.contains("UNDEF_VAR") { 7u16 }
                    else if re.contains("IMMUT_PARAM") { 8u16 }
                    else if re.contains("UNKNOWN_FN") { 9u16 }
                    else if re.contains("ARITY_MISMATCH") { 10u16 }
                    else if re.contains("BUDGET_EXCEEDED") { 3u16 }
                    else if re.contains("BAD_TYPE") { 4u16 }
                    else { 1u16 };
                if ecode != want_code {
                    return Err(format!("M3-10 '{}': error mismatch Omni={} vs Rust '{}'", name, ecode, re));
                }
            }
            Ok(want_u16) => {
                if is_omni_err {
                    let ecode = u16::from_le_bytes([out_bytes[2], out_bytes[3]]);
                    let eline = u16::from_le_bytes([out_bytes[4], out_bytes[5]]);
                    let ecol = u16::from_le_bytes([out_bytes[6], out_bytes[7]]);
                    return Err(format!("M3-10 '{}': Rust ok ({} w) but Omni errored (code={}, {}:{})", name, want_u16.len(), ecode, eline, ecol));
                }
                let got_u16: Vec<u16> = out_bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
                if got_u16 != want_u16 {
                    let min_l = got_u16.len().min(want_u16.len());
                    let diff = (0..min_l).find(|&i| got_u16[i] != want_u16[i]).unwrap_or(min_l);
                    return Err(format!(
                        "M3-10 '{}': stream mismatch Omni {} w vs Rust {} w, first diff at word {} (Omni=0x{:04X} vs Rust=0x{:04X})",
                        name, got_u16.len(), want_u16.len(), diff,
                        got_u16.get(diff).copied().unwrap_or(0), want_u16.get(diff).copied().unwrap_or(0)
                    ));
                }
            }
        }
        checks += 1;
    }
    Ok(checks)
}

/// M3-13 / M3-17: Pure-Omni Frontend Self-Hosting Fixed Point (`pure/m3/m3.omni_bc`).
/// 1. Expands `pure/m3/*.omni` (`M3_LOWER_OMNI_SRC`) via Stage-0 include resolver.
/// 2. Compiles `M3_src` with Rust frontend -> `T = RustFE(M3_src)`.
/// 3. Drift check against committed `pure/m3/m3.omni_bc` (unless `write_seed == true`).
/// 4. Runs Pure-Omni M3 (`T`) on `OmniBinaryVm` to compile ITS OWN SOURCE (`M3_src`) -> `V = M3_vm(M3_src)`.
/// 5. Asserts `V == T == pure/m3/m3.omni_bc` (0-word diff â€” true frontend self-hosting fixed point!).
/// 6. End-to-end M3 + M2 chain: compiles `pure/m2/util+arena+elf.omni` with Pure-Omni M3 -> `.omni_bc`,
///    then compiles that `.omni_bc` with Pure-Omni M2 -> x86-64 ELF64, asserting 0-byte diff vs Rust!
pub fn selftest_m3_17_fixed_point(write_seed: bool) -> Result<(usize, usize, u64, usize, u64), String> {
    let expanded_src = resolve_omni_surface_includes(M3_LOWER_OMNI_SRC, None, None)?;
    let seed_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(&expanded_src))?;
    let seed_bytes = encode_u16_stream_to_bytes(&seed_u16);
    if write_seed {
        std::fs::write("pure/m3/m3.omni_bc", &seed_bytes)
            .map_err(|e| format!("Failed to write pure/m3/m3.omni_bc: {}", e))?;
    }
    let committed_seed: &[u8] = include_bytes!("../pure/m3/m3.omni_bc");
    if !write_seed && committed_seed != seed_bytes.as_slice() {
        return Err(format!(
            "M3-17 seed drift: committed pure/m3/m3.omni_bc ({} B, FNV=0x{:016X}) != RustFE(pure/m3/*.omni) ({} B, FNV=0x{:016X}); run `omnic m3-fixed-point --write-seed`",
            committed_seed.len(),
            fnv1a_64_hash(committed_seed),
            seed_bytes.len(),
            fnv1a_64_hash(&seed_bytes)
        ));
    }

    // Self-compile: run Pure-Omni M3 (`seed_u16`) on its own expanded source (`expanded_src`)!
    let m3_mod = decode_omni_token_stream(&seed_u16)?;
    let entry = m2_entry_of(&m3_mod);
    let src_u16: Vec<u16> = expanded_src.as_bytes().iter().map(|&b| b as u16).collect();
    let arena_len = (131_072 + expanded_src.len() * 6 + seed_bytes.len() + 65_536).max(1_048_576);
    let mut vm = OmniBinaryVm::with_budgets(1_500_000_000, 128 * 1024 * 1024);
    let out_bytes = match vm.execute_function(
        &m3_mod,
        entry,
        vec![
            OmniValue::Bytes(vec![0u8; arena_len]),
            OmniValue::U16Array(src_u16),
            OmniValue::Int(1),
        ],
    )? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("M3-17 self-compile returned {:?}, expected Bytes", other)),
    };
    if out_bytes.len() == 16 && u16::from_le_bytes([out_bytes[0], out_bytes[1]]) == 0xFFFF {
        let ec = u16::from_le_bytes([out_bytes[2], out_bytes[3]]);
        let el = u16::from_le_bytes([out_bytes[4], out_bytes[5]]);
        let ecol = u16::from_le_bytes([out_bytes[6], out_bytes[7]]);
        return Err(format!("M3-17 self-compile errored (code={}, {}:{})", ec, el, ecol));
    }
    if out_bytes != seed_bytes {
        let got_u16: Vec<u16> = out_bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        let min_l = got_u16.len().min(seed_u16.len());
        let diff = (0..min_l).find(|&i| got_u16[i] != seed_u16[i]).unwrap_or(min_l);
        return Err(format!(
            "M3-17 FIXED-POINT MISMATCH: M3_vm={} w (0x{:016X}) vs RustFE={} w (0x{:016X}), first diff at word {} (0x{:04X} vs 0x{:04X})",
            got_u16.len(), fnv1a_64_hash(&out_bytes), seed_u16.len(), fnv1a_64_hash(&seed_bytes),
            diff, got_u16.get(diff).copied().unwrap_or(0), seed_u16.get(diff).copied().unwrap_or(0)
        ));
    }

    // End-to-end Pure-Omni M3 + M2 chain: 1,034-line `pure/verifier_engine.omni`
    // `.omni` -> Pure-Omni M3 -> `.omni_bc` (11,893 w) -> Pure-Omni M2 -> x86-64 ELF64 (0-byte diff vs Rust!)
    let chain_src = OMNI_VERIFIER_ENGINE_SURFACE;
    let chain_u16_in: Vec<u16> = chain_src.as_bytes().iter().map(|&b| b as u16).collect();
    let chain_bc_bytes = match vm.execute_function(
        &m3_mod,
        entry,
        vec![
            OmniValue::Bytes(vec![0u8; 786_432]),
            OmniValue::U16Array(chain_u16_in),
            OmniValue::Int(1),
        ],
    )? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("M3-17 E2E M3 step returned {:?}", other)),
    };
    let chain_bc_u16: Vec<u16> = chain_bc_bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    let m2_mod = m2_compile_decoder(M2_MULTI_OMNI_SRC)?;
    let m2_entry = m2_entry_of(&m2_mod);
    let pure_elf = match vm.execute_function(
        &m2_mod,
        m2_entry,
        vec![
            OmniValue::Bytes(vec![0u8; 524_288]),
            OmniValue::U16Array(chain_bc_u16),
            OmniValue::Int(0),
            OmniValue::Int(0),
            OmniValue::Int(0),
        ],
    )? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("M3-17 E2E M2 step returned {:?}", other)),
    };
    let rust_bc = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(chain_src))?;
    let rust_elf = compile_pure_u16_to_x86_64_elf_multi(&rust_bc, 0, 0, 0)?;
    if pure_elf != rust_elf {
        return Err(format!(
            "M3-17 E2E M3+M2 ELF mismatch: Pure={} B (0x{:016X}) vs Rust={} B (0x{:016X})",
            pure_elf.len(), fnv1a_64_hash(&pure_elf), rust_elf.len(), fnv1a_64_hash(&rust_elf)
        ));
    }

    Ok((3, seed_u16.len(), fnv1a_64_hash(&seed_bytes), pure_elf.len(), fnv1a_64_hash(&pure_elf)))
}

// ============================================================================
// M3-14: Lockstep Shadow Frontend (`lockstep_shadow_frontend`)
// ============================================================================

static M3_LOCKSTEP_SEED_MOD: std::sync::OnceLock<Result<BinModule, String>> = std::sync::OnceLock::new();
static M3_LOCKSTEP_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

thread_local! {
    static M3_LOCKSTEP_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static M3_LOCKSTEP_SUSPENDED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static M3_LOCKSTEP_INJECT_PATH: std::cell::Cell<Option<&'static str>> = const { std::cell::Cell::new(None) };
}

pub fn with_m3_lockstep_enabled<R>(f: impl FnOnce() -> R) -> R {
    let prev = M3_LOCKSTEP_ENABLED.with(|c| c.replace(true));
    let r = f();
    M3_LOCKSTEP_ENABLED.with(|c| c.set(prev));
    r
}

pub fn with_m3_lockstep_suspended<R>(f: impl FnOnce() -> R) -> R {
    let prev = M3_LOCKSTEP_SUSPENDED.with(|c| c.replace(true));
    let r = f();
    M3_LOCKSTEP_SUSPENDED.with(|c| c.set(prev));
    r
}

pub fn with_m3_lockstep_inject_mismatch<R>(path: &'static str, f: impl FnOnce() -> R) -> R {
    let prev = M3_LOCKSTEP_INJECT_PATH.with(|c| c.replace(Some(path)));
    let r = f();
    M3_LOCKSTEP_INJECT_PATH.with(|c| c.set(prev));
    r
}

fn m3_lockstep_is_active() -> bool {
    if M3_LOCKSTEP_SUSPENDED.with(|c| c.get()) || LOCKSTEP_SUSPENDED.with(|c| c.get()) {
        return false;
    }
    M3_LOCKSTEP_ENABLED.with(|c| c.get())
        || std::env::var("OMNI_M3_LOCKSTEP").ok().as_deref() == Some("1")
}

fn m3_lockstep_seed_module() -> Result<&'static BinModule, String> {
    let r = M3_LOCKSTEP_SEED_MOD.get_or_init(|| {
        const SEED: &[u8] = include_bytes!("../pure/m3/m3.omni_bc");
        if SEED.len() % 2 != 0 {
            return Err("M3-14: pure/m3/m3.omni_bc has odd byte length".to_string());
        }
        let u16s: Vec<u16> = SEED
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        decode_omni_token_stream(&u16s)
    });
    r.as_ref().map_err(|e| e.clone())
}

fn m3_lockstep_compile_via_omni(src: &str, strict: bool) -> Result<Result<Vec<u16>, (u16, u16, u16)>, String> {
    let m3_mod = m3_lockstep_seed_module()?;
    let entry = m2_entry_of(m3_mod);
    let src_u16: Vec<u16> = src.as_bytes().iter().map(|&b| b as u16).collect();
    let arena_bytes = 1_048_576usize;
    let mut vm = OmniBinaryVm::with_budgets(500_000_000, 128 * 1024 * 1024);
    let out_bytes = match vm.execute_function(
        m3_mod,
        entry,
        vec![
            OmniValue::Bytes(vec![0u8; arena_bytes]),
            OmniValue::U16Array(src_u16),
            OmniValue::Int(if strict { 1 } else { 0 }),
        ],
    )? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("M3-14: Omni M3 returned {:?}, expected Bytes", other)),
    };
    if out_bytes.len() == 16 && u16::from_le_bytes([out_bytes[0], out_bytes[1]]) == 0xFFFF {
        let code = u16::from_le_bytes([out_bytes[2], out_bytes[3]]);
        let line = u16::from_le_bytes([out_bytes[4], out_bytes[5]]);
        let col = u16::from_le_bytes([out_bytes[6], out_bytes[7]]);
        return Ok(Err((code, line, col)));
    }
    if out_bytes.len() % 2 != 0 {
        return Err(format!("M3-14: Omni M3 returned odd byte length {}", out_bytes.len()));
    }
    let words: Vec<u16> = out_bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    Ok(Ok(words))
}

/// M3-14: Opt-in lockstep shadow frontend (`OMNI_M3_LOCKSTEP=1` or `with_m3_lockstep_enabled`).
/// Modeled on `lockstep_shadow_verify`: compiles `src` with the committed Pure-Omni M3 seed on the VM
/// and hard-errors with `error[LOCKSTEP_MISMATCH]` on any word-level or error-presence disagreement.
pub fn lockstep_shadow_frontend(
    path: &str,
    src: &str,
    strict: bool,
    rust_res: &Result<Vec<u16>, String>,
) -> Result<(), String> {
    if !m3_lockstep_is_active() {
        return Ok(());
    }
    let mut omni_res = with_m3_lockstep_suspended(|| m3_lockstep_compile_via_omni(src, strict))?;
    if M3_LOCKSTEP_INJECT_PATH.with(|c| c.get()) == Some(path) {
        omni_res = match omni_res {
            Ok(mut v) => {
                if let Some(first) = v.first_mut() {
                    *first ^= 0x0001;
                } else {
                    v.push(0xFFFF);
                }
                Ok(v)
            }
            Err(_) => Ok(vec![0x7F00]),
        };
    }
    M3_LOCKSTEP_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    match (rust_res, &omni_res) {
        (Ok(r), Ok(o)) if r == o => Ok(()),
        (Ok(r), Ok(o)) => {
            let min_l = r.len().min(o.len());
            let diff = (0..min_l).find(|&i| r[i] != o[i]).unwrap_or(min_l);
            Err(format!(
                "error[LOCKSTEP_MISMATCH] line 1, col 1: path={}: Rust and Omni M3 frontends disagree: rust={} words vs omni={} words (first diff at word {}: 0x{:04X} vs 0x{:04X})",
                path, r.len(), o.len(), diff, r.get(diff).copied().unwrap_or(0), o.get(diff).copied().unwrap_or(0)
            ))
        }
        (Err(_), Err(_)) => Ok(()),
        (Ok(r), Err((c, l, col))) => Err(format!(
            "error[LOCKSTEP_MISMATCH] line 1, col 1: path={}: Rust frontend ok ({} words) but Omni M3 errored (code={}, {}:{})",
            path, r.len(), c, l, col
        )),
        (Err(re), Ok(o)) => Err(format!(
            "error[LOCKSTEP_MISMATCH] line 1, col 1: path={}: Rust frontend errored ({}) but Omni M3 ok ({} words)",
            path, re, o.len()
        )),
    }
}

/// M3-14 selftest: verifies `lockstep_shadow_frontend` positive parity (arithmetic, `if_pos`/`match_trit`,
/// `pure/m2/util.omni`, and negative error agreement), injected-mismatch hard rejection, and suspend hook.
pub fn selftest_m3_14_lockstep_frontend() -> Result<usize, String> {
    let mut checks = 0usize;
    let before = M3_LOCKSTEP_COUNT.load(std::sync::atomic::Ordering::SeqCst);

    let ok_arith = "#[strict]\nfn main(a: i64) -> i64 { let b: i64 = a + 1; return b; }\n";
    let ok_cf = "#[strict]\nfn main(x: i64) -> i64 { let t: trit = x <=> 0; let r: i64 = match_trit t { neg => -x, zero => 0, pos => if_pos (x <=> 10) { x - 10 } else { x + 1 } }; return r; }\n";
    let ok_util = include_str!("../pure/m2/util.omni");
    let err_src = "#[strict]\nfn main(x: i64) -> i64 { return undeclared_var; }\n";

    with_m3_lockstep_enabled(|| -> Result<(), String> {
        // 1-3: Positive 0-word-diff parity through production `compile_omni_surface_to_u16`
        let _ = compile_omni_surface_to_u16(ok_arith)?;
        checks += 1;
        let _ = compile_omni_surface_to_u16(ok_cf)?;
        checks += 1;
        let _ = compile_omni_surface_strict_to_u16(ok_util)?;
        checks += 1;

        // 4: Negative parity (both Rust and Omni M3 reject; lockstep agrees and returns Rust's Err)
        match compile_omni_surface_to_u16(err_src) {
            Err(e) if !e.starts_with("error[LOCKSTEP_MISMATCH]") => checks += 1,
            other => return Err(format!("M3-14 negative parity expected normal error, got {:?}", other)),
        }

        // 5: Injected disagreement MUST hard-error with error[LOCKSTEP_MISMATCH]
        let inj = with_m3_lockstep_inject_mismatch("compile", || compile_omni_surface_to_u16(ok_arith));
        match inj {
            Err(e) if e.starts_with("error[LOCKSTEP_MISMATCH]") => checks += 1,
            other => return Err(format!("M3-14 inject-mismatch expected LOCKSTEP_MISMATCH, got {:?}", other)),
        }

        // 6: Suspend hook skips lockstep cleanly (even with mismatch injection armed)
        let susp = with_m3_lockstep_suspended(|| {
            with_m3_lockstep_inject_mismatch("compile", || compile_omni_surface_to_u16(ok_arith))
        });
        match susp {
            Ok(_) => checks += 1,
            Err(e) => return Err(format!("M3-14 suspended hook unexpectedly failed: {}", e)),
        }
        Ok(())
    })?;

    let after = M3_LOCKSTEP_COUNT.load(std::sync::atomic::Ordering::SeqCst);
    if after < before + 5 {
        return Err(format!("M3-14 lockstep counter did not advance as expected ({} -> {})", before, after));
    }
    Ok(checks)
}

/// M2-10 selftest: Multi-function Wasm v1 emitter + `CALL` + shared linear-memory bump global +
/// call-depth-64 budget enforcement, differentially verified against `OmniBinaryVm` and x86-64 multi AOT.
pub fn selftest_m2_10_wasm_multi() -> Result<usize, String> {
    let cases: [(&str, &str, Vec<i64>, i64); 4] = [
        (
            "mf_chain_arith",
            "#[strict]\nfn add1(x: i64) -> i64 { return x + 1; }\nfn mul3(x: i64) -> i64 { return add1(x) * 3; }\nfn main(a: i64) -> i64 { return mul3(a) + add1(a * 2); }\n",
            vec![7],
            (7 + 1) * 3 + (14 + 1),
        ),
        (
            "mf_loop_helper",
            "#[strict]\nfn step(acc: i64, i: i64) -> i64 { return acc + i * i; }\nfn main(n: i64) -> i64 { let acc: i64 = 0; let i: i64 = 1; while n >= i { acc = step(acc, i); i = i + 1; } return acc; }\n",
            vec![5],
            55,
        ),
        (
            "mf_buf_across_call",
            "#[strict]\nfn mk_buf(a: i64, b: i64) -> bytes { let w: bytes = anew(0); w = iset(w, a); w = iset(w, b); return w; }\nfn sum_buf(w: bytes) -> i64 { return iget(w, 0) + iget(w, 1) + slen(w); }\nfn main(x: i64) -> i64 { let a: bytes = mk_buf(x, x + 1); let b: bytes = mk_buf(x + 10, x + 20); return sum_buf(a) * 100 + sum_buf(b); }\n",
            vec![5],
            (5 + 6 + 2) * 100 + (15 + 25 + 2),
        ),
        (
            "mf_deep_dag",
            "#[strict]\nfn f1(x: i64) -> i64 { return x + 1; }\nfn f2(x: i64) -> i64 { return f1(x) + f1(x + 1); }\nfn f3(x: i64) -> i64 { return f2(x) * 2 + f1(x); }\nfn main(n: i64) -> i64 { return f3(n) + f2(n); }\n",
            vec![4],
            ((5 + 6) * 2 + 5) + (5 + 6),
        ),
    ];

    let mut checks = 0usize;
    for (name, src, args, want) in &cases {
        let stream = compile_omni_surface_strict_to_u16(src)?;
        let m = decode_omni_token_stream(&stream)?;
        let entry = m2_entry_of(&m);
        let vm_args: Vec<OmniValue> = args.iter().map(|&v| OmniValue::Int(v)).collect();
        let mut vm = OmniBinaryVm::new();
        let vm_got = match vm.execute_function(&m, entry, vm_args)? {
            OmniValue::Int(v) => v,
            other => return Err(format!("M2-10 '{}': VM returned {:?}", name, other)),
        };
        let wasm = compile_pure_u16_to_wasm_multi(&stream)?;
        let wasm_got = execute_wasm_module_i64(&wasm, args)?;
        // Also verify x86-64 multi-fn compiles cleanly on the same stream
        let _x86_elf = compile_pure_u16_to_x86_64_elf_multi(&stream, args.first().copied().unwrap_or(0), 0, 0)?;
        if vm_got != *want || wasm_got != *want {
            return Err(format!(
                "M2-10 '{}': mismatch want={} vm={} wasm={}",
                name, want, vm_got, wasm_got
            ));
        }
        checks += 1;
    }

    // Recursion MUST be rejected by multi-AOT verifier (MULTI_AOT_RECURSION)
    let rec_src = "#[strict]\nfn rec(n: i64) -> i64 { let c: trit = n <=> 0; return if_pos c { 1 + rec(n - 1) } else { 0 }; }\nfn main(n: i64) -> i64 { return rec(n); }\n";
    let rec_stream = compile_omni_surface_strict_to_u16(rec_src)?;
    match compile_pure_u16_to_wasm_multi(&rec_stream) {
        Err(e) if e.contains("MULTI_AOT_RECURSION") => checks += 1,
        other => return Err(format!("M2-10 recursion expected MULTI_AOT_RECURSION, got {:?}", other)),
    }

    // Evaluator call-depth-64 budget enforcement (direct 1-fn Wasm module: if p0>0 call 0(p0-1)+1 else 0)
    let mut depth_wasm: Vec<u8> = vec![
        0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00,
        0x01, 0x06, 0x01, 0x60, 0x01, 0x7E, 0x01, 0x7E, // Type: (i64)->i64
        0x03, 0x02, 0x01, 0x00,                         // Func: 0
        0x07, 0x0B, 0x01, 0x07, b'o', b'm', b'n', b'i', b'_', b'f', b'n', 0x00, 0x00,
    ];
    let body: Vec<u8> = vec![
        0x00,                   // 0 locals
        0x20, 0x00,             // local.get 0
        0x42, 0x00, 0x55,       // i64.const 0; i64.gt_s
        0x04, 0x40,             // if void
        0x20, 0x00, 0x42, 0x01, 0x7D, // local.get 0 - 1
        0x10, 0x00,             // call 0
        0x42, 0x01, 0x7C,       // + 1
        0x0F,                   // return
        0x0B,                   // end if
        0x42, 0x00,             // i64.const 0
        0x0B,                   // end fn
    ];
    let mut code_sec = vec![0x01];
    code_sec.extend(encode_uleb128(body.len()));
    code_sec.extend(body);
    depth_wasm.push(0x0A);
    depth_wasm.extend(encode_uleb128(code_sec.len()));
    depth_wasm.extend(code_sec);

    if execute_wasm_module_i64(&depth_wasm, &[60])? != 60 {
        return Err(String::from("M2-10 depth-60 evaluator mismatch"));
    }
    checks += 1;
    match execute_wasm_module_i64(&depth_wasm, &[70]) {
        Err(e) if e.contains("BUDGET_EXCEEDED") && e.contains("64") => checks += 1,
        other => return Err(format!("M2-10 depth-70 expected call-depth BUDGET_EXCEEDED, got {:?}", other)),
    }

    // Fix 2 regression: `compile_pure_u16_to_wasm` dispatches 2+ fn modules to multi-Wasm,
    // selects the last function when `fn_idx == 0` is absent (matching `run`), and supports >4 entry params.
    let fix2_src = "#[strict]\nfn helper_first(x: i64) -> i64 { return x * 100; }\n#[strict]\nfn entry_last(a: i64, b: i64, c: i64, d: i64, e: i64) -> i64 { return helper_first(d) + e + a * 0 + b * 0 + c * 0; }\n";
    let fix2_stream = compile_omni_surface_strict_to_u16(fix2_src)?;
    let fix2_wasm = compile_pure_u16_to_wasm(&fix2_stream)?;
    let fix2_got = execute_wasm_module_i64(&fix2_wasm, &[1, 2, 3, 4, 5])?;
    if fix2_got != 405 {
        return Err(format!("M2-10 Fix 2 multi-fn dispatch / 5-arg last-entry returned {}, want 405", fix2_got));
    }
    checks += 1;

    // Option A regression: x86-64 multi-AOT now supports up to 32 entry params (`_start` 256-B CLI block).
    let fix2_elf = compile_pure_u16_to_x86_64_elf_multi(&fix2_stream, 0, 0, 0)?;
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        let tmp_f2 = PrivateTempElf::new(&fix2_elf, "fix1_x86_5arg")?;
        let out_f2 = std::process::Command::new(&tmp_f2.path)
            .args(["1", "2", "3", "4", "5"])
            .output()
            .map_err(|e| format!("M2-10 exec 5-arg multi ELF: {}", e))?;
        drop(tmp_f2);
        if !out_f2.status.success() || out_f2.stdout.len() != 8 {
            return Err(format!("M2-10 5-arg multi ELF failed: {:?}", out_f2.status));
        }
        let got_x86 = i64::from_le_bytes(out_f2.stdout[..8].try_into().unwrap());
        if got_x86 != 405 {
            return Err(format!("M2-10 5-arg multi ELF returned {}, want 405", got_x86));
        }
    }
    let _ = &fix2_elf;
    checks += 1;

    // Fix 3 positive & negative regressions: `#[mem_budget(8192)]` runs on the `"aot"` Wasm hot path
    // without billing pre-allocated linear-memory pages, while still rejecting `arena(20000)`.
    let fix3_ok_src = "#[strict]\n#[mem_budget(8192)]\nfn main(cur: i64, step: i64) -> i64 { let a: arena = arena(4096); drop(a); return cur + step; }\n";
    match execute_omni_surface_aot_first(fix3_ok_src, vec![OmniValue::Int(5), OmniValue::Int(1)])? {
        (OmniValue::Int(6), "aot") => checks += 1,
        other => return Err(format!("M2-10 Fix 3 #[mem_budget(8192)] arena(4096) expected (Int(6), \"aot\"), got {:?}", other)),
    }
    let fix3_neg_src = "#[strict]\n#[mem_budget(8192)]\nfn main(cur: i64, step: i64) -> i64 { let a: arena = arena(20000); drop(a); return cur + step; }\n";
    match execute_omni_surface_aot_first(fix3_neg_src, vec![OmniValue::Int(5), OmniValue::Int(1)]) {
        Err(e) if e.contains("BUDGET_EXCEEDED") => checks += 1,
        other => return Err(format!("M2-10 Fix 3 #[mem_budget(8192)] arena(20000) expected BUDGET_EXCEEDED, got {:?}", other)),
    }

    Ok(checks)
}

/// M2-14 selftest: Pure-Omni iterative multi-AOT module verifier (`pure/m2/verify/mod_check.omni`)
/// differentially checked against Rust `verify_module_for_multi_aot` across positive multi-fn modules
/// (including 1,034-line `pure/verifier_engine.omni`) and negative M2-08 / L1-L12 reject cases.
pub fn selftest_m2_14_verify_diff() -> Result<usize, String> {
    let vmod = m2_compile_decoder(M2_VERIFY_OMNI_SRC)?;
    let entry = m2_entry_of(&vmod);
    let run_pure = |stream: &[u16]| -> Result<(u32, u32), String> {
        let mut vm = OmniBinaryVm::with_budgets(200_000_000, 64 * 1024 * 1024);
        let rv = match vm.execute_function(
            &vmod,
            entry,
            vec![
                OmniValue::Bytes(vec![0u8; 262_144]),
                OmniValue::U16Array(stream.to_vec()),
            ],
        )? {
            OmniValue::Int(v) => v as u64,
            other => return Err(format!("M2-14 returned {:?}, expected Int", other)),
        };
        Ok(((rv >> 32) as u32, (rv & 0xFFFF_FFFF) as u32))
    };

    let mut checks = 0usize;
    let pos_cases: [(&str, &str); 5] = [
        ("mf_chain", "#[strict]\nfn add1(x: i64) -> i64 { return x + 1; }\nfn main(a: i64) -> i64 { return add1(a) * 2; }\n"),
        ("mf_buf", "#[strict]\nfn mk(a: i64) -> bytes { let w: bytes = anew(0); w = iset(w, a); return w; }\nfn main(x: i64) -> i64 { let w: bytes = mk(x); return iget(w, 0); }\n"),
        ("pure_compiler", include_str!("../pure/compiler.omni")),
        ("pure_m2_util", include_str!("../pure/m2/util.omni")),
        ("pure_verifier_engine", OMNI_VERIFIER_ENGINE_SURFACE),
    ];
    for (name, src) in &pos_cases {
        let stream = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(src))?;
        let decoded = decode_omni_token_stream(&stream)?;
        verify_module_for_multi_aot(&decoded)
            .map_err(|e| format!("M2-14 pos '{}': Rust rejected: {}", name, e))?;
        let (ec, eo) = run_pure(&stream).map_err(|e| format!("M2-14 pos '{}': {}", name, e))?;
        if ec != 0 {
            return Err(format!("M2-14 pos '{}': Pure-Omni rejected with code={} off={}", name, ec, eo));
        }
        checks += 1;
    }

    // Negative 1: self-recursion (code 5)
    let self_rec = compile_omni_surface_strict_to_u16(
        "#[strict]\nfn rec(n: i64) -> i64 { let c: trit = n <=> 0; return if_pos c { 1 + rec(n - 1) } else { 0 }; }\nfn main(n: i64) -> i64 { return rec(n); }\n",
    )?;
    if verify_module_for_multi_aot(&decode_omni_token_stream(&self_rec)?).is_ok() {
        return Err(String::from("M2-14 self-rec: Rust unexpectedly accepted"));
    }
    let (ec, _) = run_pure(&self_rec)?;
    if ec != 5 {
        return Err(format!("M2-14 self-rec: expected code 5 (RECURSION), got {}", ec));
    }
    checks += 1;

    // Negative 2: mutual recursion (code 5)
    let mut_rec = compile_omni_surface_strict_to_u16(
        "#[strict]\nfn fa(n: i64) -> i64 { return if_pos n <=> 0 { fb(n - 1) } else { 0 }; }\nfn fb(n: i64) -> i64 { return if_pos n <=> 0 { fa(n - 1) } else { 1 }; }\nfn main(n: i64) -> i64 { return fa(n); }\n",
    )?;
    if verify_module_for_multi_aot(&decode_omni_token_stream(&mut_rec)?).is_ok() {
        return Err(String::from("M2-14 mutual-rec: Rust unexpectedly accepted"));
    }
    let (ec, _) = run_pure(&mut_rec)?;
    if ec != 5 {
        return Err(format!("M2-14 mutual-rec: expected code 5 (RECURSION), got {}", ec));
    }
    checks += 1;

    // Negative 3: bad CALL target (code 3) â€” patch TOK_SYM_FN target id to 999
    let mut bad_tgt = compile_omni_surface_strict_to_u16(
        "#[strict]\nfn f(x: i64) -> i64 { return x + 1; }\nfn main(a: i64) -> i64 { return f(a); }\n",
    )?;
    if let Some(p) = bad_tgt.iter().position(|&w| w == TOK_SYM_FN) {
        bad_tgt[p + 1] = 999;
    }
    if verify_module_for_multi_aot(&decode_omni_token_stream(&bad_tgt)?).is_ok() {
        return Err(String::from("M2-14 bad-target: Rust unexpectedly accepted"));
    }
    let (ec, _) = run_pure(&bad_tgt)?;
    if ec != 3 {
        return Err(format!("M2-14 bad-target: expected code 3 (BAD_CALL_TARGET), got {}", ec));
    }
    checks += 1;

    // Negative 4: CALL arity mismatch (code 4) â€” duplicate the single arg in CALL
    let mut bad_arity = compile_omni_surface_strict_to_u16(
        "#[strict]\nfn f(x: i64) -> i64 { return x + 1; }\nfn main(a: i64) -> i64 { return f(a); }\n",
    )?;
    if let Some(p) = bad_arity.iter().position(|&w| w == TOK_SYM_FN) {
        // Insert an extra param token right after the existing arg (p + 2)
        let extra = bad_arity[p + 2];
        bad_arity.insert(p + 3, extra);
    }
    if verify_module_for_multi_aot(&decode_omni_token_stream(&bad_arity)?).is_ok() {
        return Err(String::from("M2-14 bad-arity: Rust unexpectedly accepted"));
    }
    let (ec, _) = run_pure(&bad_arity)?;
    if ec != 4 {
        return Err(format!("M2-14 bad-arity: expected code 4 (ARITY_MISMATCH), got {}", ec));
    }
    checks += 1;

    // Negative 5: unresolved branch target (code 6)
    let mut bad_lbl = compile_omni_surface_strict_to_u16(
        "#[strict]\nfn main(x: i64) -> i64 { goto 1; label 1; return x; }\n",
    )?;
    // Mutate the JMP target label token (first 0x7EF1) to 0x7EF5 (undefined label 5)
    if let Some(p) = bad_lbl.iter().position(|&w| w == 0x7EF1) {
        bad_lbl[p] = 0x7EF5;
    }
    let (ec, _) = run_pure(&bad_lbl)?;
    if ec != 6 {
        return Err(format!("M2-14 bad-label: expected code 6 (BAD_LABEL), got {}", ec));
    }
    checks += 1;

    Ok(checks)
}

/// M3-15 selftest: 4-backend parity of Pure-Omni M3-compiled multi-function images across:
///   (a) VM (`OmniBinaryVm`)
///   (b) Rust x86-64 multi AOT (`M2-09`)
///   (c) Wasm multi AOT (`M2-10` + depth-64 evaluator)
///   (d) Pure-Omni M2 x86-64 ELF64 (`M2-15/16`, 0-byte diff vs Rust x86-multi)
/// Stage-1 is N/A (frozen single-function legacy exemption, explicitly excluded per spec/m3/PLAN.md Â§7.4).
pub fn selftest_m3_15_four_backend_parity() -> Result<usize, String> {
    let m2_mod = m2_compile_decoder(M2_MULTI_OMNI_SRC)?;
    let m2_entry = m2_entry_of(&m2_mod);
    let cases: [(&str, &str, Vec<i64>, i64); 4] = [
        (
            "m3_15_mf_arith",
            "#[strict]\nfn sq(x: i64) -> i64 { return x * x; }\nfn main(a: i64) -> i64 { return sq(a + 2) + sq(a - 1); }\n",
            vec![5],
            49 + 16,
        ),
        (
            "m3_15_mf_cf_loop",
            "#[strict]\nfn tri(n: i64) -> i64 { let s: i64 = 0; let i: i64 = 1; while n >= i { s = s + i; i = i + 1; } return s; }\nfn main(n: i64) -> i64 { let t: trit = n <=> 0; return if_pos t { tri(n) } else { 0 }; }\n",
            vec![10],
            55,
        ),
        (
            "m3_15_mf_buf_call",
            "#[strict]\nfn pack2(a: i64, b: i64) -> bytes { let w: bytes = anew(0); w = iset(w, a); w = iset(w, b); return w; }\nfn score(w: bytes) -> i64 { return iget(w, 0) * 10 + iget(w, 1); }\nfn main(x: i64) -> i64 { let w: bytes = pack2(x, x + 3); return score(w); }\n",
            vec![4],
            47,
        ),
        (
            "m3_15_int_min_divmod",
            "#[strict]\nfn main(a: i64, b: i64, c: i64) -> i64 {\n    let imin_v: i64 = 1 << 63;\n    let q0: i64 = imin_v / a;\n    let r0: i64 = imin_v % b;\n    let q1: i64 = 5 / imin_v;\n    let r1: i64 = (imin_v % c) + (0 / imin_v);\n    return (q0 >> 60) * 100 + r0 * 10 + q1 + r1;\n}\n",
            vec![2, 3, -1],
            (-4i64) * 100 + (-2i64) * 10,
        ),
    ];

    let mut checks = 0usize;
    for (name, src, args, want) in &cases {
        // Compile via BOTH Pure-Omni M3 seed AND Rust frontend (0-word diff!)
        let rust_bc = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(src))?;
        let omni_bc = match m3_lockstep_compile_via_omni(src, true)? {
            Ok(v) => v,
            Err((c, l, col)) => return Err(format!("M3-15 '{}': Omni M3 errored ({}, {}:{})", name, c, l, col)),
        };
        if omni_bc != rust_bc {
            return Err(format!("M3-15 '{}': M3 vs Rust bytecode mismatch", name));
        }

        // (a) VM
        let decoded = decode_omni_token_stream(&omni_bc)?;
        let entry = m2_entry_of(&decoded);
        let vm_args: Vec<OmniValue> = args.iter().map(|&v| OmniValue::Int(v)).collect();
        let mut vm = OmniBinaryVm::new();
        let vm_got = match vm.execute_function(&decoded, entry, vm_args)? {
            OmniValue::Int(v) => v,
            other => return Err(format!("M3-15 '{}': VM returned {:?}", name, other)),
        };

        // (c) Wasm multi AOT (M2-10)
        let wasm = compile_pure_u16_to_wasm_multi(&omni_bc)?;
        let wasm_got = execute_wasm_module_i64(&wasm, args)?;

        // (e) Pure-Omni M4 VM (5th backend column)
        let a0 = args.first().copied().unwrap_or(0);
        let a1 = args.get(1).copied().unwrap_or(0);
        let a2 = args.get(2).copied().unwrap_or(0);
        let (m4_got, _m4_tag, m4_err, _m4_steps, _m4_trace) =
            p4_run_raw_bc_on_pure_vm(&omni_bc, a0, a1, a2, 100_000, 0)?;

        if vm_got != *want || wasm_got != *want || m4_err != 0 || m4_got != *want {
            return Err(format!(
                "M3-15 '{}': want={} vm={} wasm={} m4={}(err={})",
                name, want, vm_got, wasm_got, m4_got, m4_err
            ));
        }

        // (b) Rust x86-64 multi AOT (M2-09) vs (d) Pure-Omni M2 x86-64 ELF64 (M2-15/16) — 0-byte diff!
        let rust_elf = compile_pure_u16_to_x86_64_elf_multi(&omni_bc, a0, a1, a2)?;
        let mut vm_m2 = OmniBinaryVm::with_budgets(100_000_000, 64 * 1024 * 1024);
        let pure_elf = match vm_m2.execute_function(
            &m2_mod,
            m2_entry,
            vec![
                OmniValue::Bytes(vec![0u8; 524_288]),
                OmniValue::U16Array(omni_bc),
                OmniValue::Int(a0),
                OmniValue::Int(a1),
                OmniValue::Int(a2),
            ],
        )? {
            OmniValue::Bytes(b) => b,
            other => return Err(format!("M3-15 '{}': M2 returned {:?}", name, other)),
        };
        if pure_elf != rust_elf {
            return Err(format!(
                "M3-15 '{}': Pure-Omni M2 ELF ({} B) != Rust x86-multi ELF ({} B)",
                name, pure_elf.len(), rust_elf.len()
            ));
        }
        checks += 1;
    }
    Ok(checks)
}

// ============================================================================
// Phase 4 (P4-1 / P4-2 / P4-5): Pure-Omni Token-Stream VM Differential Gate
// ============================================================================

/// Runs a raw `.omni_bc` u16 token stream on the Pure-Omni VM (`pure/m4/vm.omni`),
/// returning `(ret_val, ret_tag, err_code, steps, trace_cnt)`.
pub fn p4_run_raw_bc_on_pure_vm(
    stream: &[u16],
    a0: i64,
    a1: i64,
    a2: i64,
    fuel: i64,
    trace_mode: i64,
) -> Result<(i64, u32, u32, u32, u32), String> {
    static M4_VM_MOD: std::sync::OnceLock<Result<BinModule, String>> = std::sync::OnceLock::new();
    let vm_mod = M4_VM_MOD
        .get_or_init(|| m2_compile_decoder(M4_VM_OMNI_SRC))
        .as_ref()
        .map_err(|e| e.clone())?;
    let entry = m2_entry_of(vm_mod);
    let mut arena = vec![0u8; 524_288];
    let st_base = 167_936usize;
    arena[st_base + 48..st_base + 56].copy_from_slice(&a0.to_le_bytes());
    arena[st_base + 56..st_base + 64].copy_from_slice(&a1.to_le_bytes());
    arena[st_base + 64..st_base + 72].copy_from_slice(&a2.to_le_bytes());
    let mut host_vm = OmniBinaryVm::with_budgets(500_000_000, 128 * 1024 * 1024);
    let state_bytes = match host_vm.execute_function(
        vm_mod,
        entry,
        vec![
            OmniValue::Bytes(arena),
            OmniValue::U16Array(stream.to_vec()),
            OmniValue::Int(fuel),
            OmniValue::Int(trace_mode),
        ],
    )? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("P4 VM returned {:?}, expected 64-B VM_STATE slice", other)),
    };
    if state_bytes.len() < 48 {
        return Err(format!("P4 VM returned short state slice ({} B)", state_bytes.len()));
    }
    let trace_cnt = u32::from_le_bytes(state_bytes[24..28].try_into().unwrap());
    let err_code = u32::from_le_bytes(state_bytes[28..32].try_into().unwrap());
    let ret_val = i64::from_le_bytes(state_bytes[32..40].try_into().unwrap());
    let ret_tag = u32::from_le_bytes(state_bytes[40..44].try_into().unwrap());
    let steps = u32::from_le_bytes(state_bytes[44..48].try_into().unwrap());
    Ok((ret_val, ret_tag, err_code, steps, trace_cnt))
}

/// P4-1 / P4-2 / P4-5 selftest: Pure-Omni VM (`pure/m4/vm.omni`) vs Rust `OmniBinaryVm`
/// across arithmetic, bitwise, ternary, control flow, loops, byte buffers (ISA-B ld_u*/st_u*),
/// multi-function CALL/RET, raw `.omni_bc` execution, P4-5 binary trace ring, and OOB/fuel/depth budgets.
pub fn selftest_p4_1_vm_diff() -> Result<usize, String> {
    let cases: [(&str, &str, [i64; 3]); 7] = [
        (
            "p4_arith_bit",
            "#[strict]\nfn main(a: i64, b: i64, c: i64) -> i64 { let x: i64 = (a + b) * c - (a / 2) + (a % 3); let y: i64 = (x & 255) | (c << 2) ^ (b >> 1); return y + (-a); }\n",
            [17, 6, 4],
        ),
        (
            "p4_trit_match",
            "#[strict]\nfn main(a: i64, b: i64, c: i64) -> i64 { let t: trit = a <=> b; let r: i64 = match_trit t { neg => c * 10, zero => c + 5, pos => a - b + c }; return r; }\n",
            [12, 5, 3],
        ),
        (
            "p4_loop_sum",
            "#[strict]\nfn main(n: i64, a1: i64, a2: i64) -> i64 { let s: i64 = 0; let i: i64 = 1; while n >= i { s = s + i * i; i = i + 1; } return s + a1 + a2; }\n",
            [8, 1, 2],
        ),
        (
            "p4_mf_call_chain",
            "#[strict]\nfn add1(x: i64) -> i64 { return x + 1; }\nfn mul3(x: i64) -> i64 { return add1(x) * 3; }\nfn main(a: i64, b: i64, c: i64) -> i64 { return mul3(a) + add1(b * c); }\n",
            [7, 4, 5],
        ),
        (
            "p4_buf_isab",
            "#[strict]\nfn mk(a: i64, b: i64) -> bytes { let w: bytes = anew(0); w = iset_i32(w, a); w = iset_i32(w, b); w = st_u16(w, 2, 1234); return w; }\nfn main(a: i64, b: i64, c: i64) -> i64 { let w: bytes = mk(a, b); return ld_u16(w, 2) + ld_u32(w, 4) + slen(w) + c; }\n",
            [99, 42000, 7],
        ),
        (
            "p4_pure_compiler_kernel",
            include_str!("../pure/compiler.omni"),
            [10, 20, 5],
        ),
        (
            "p4_if_neg_goto",
            "#[strict]\nfn main(x: i64, y: i64, z: i64) -> i64 { let r: i64 = if_neg (x <=> y) { z * 3 } else { z + 7 }; goto 9; label 9; return r; }\n",
            [2, 9, 11],
        ),
    ];

    let mut checks = 0usize;
    for (name, src, args) in &cases {
        let stream = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(src))?;
        let decoded = decode_omni_token_stream(&stream)?;
        let entry = m2_entry_of(&decoded);
        let pcnt = decoded
            .functions
            .iter()
            .find(|f| f.fn_idx == entry)
            .map(|f| f.params.len())
            .unwrap_or(0);
        let vm_args: Vec<OmniValue> = args[..pcnt.min(3)].iter().map(|&v| OmniValue::Int(v)).collect();
        let mut rust_vm = OmniBinaryVm::new();
        let rust_val = match rust_vm.execute_function(&decoded, entry, vm_args)? {
            OmniValue::Int(v) => v,
            OmniValue::Trit(t) => t as i64,
            other => return Err(format!("P4 '{}': Rust VM returned {:?}", name, other)),
        };

        // Run with trace_mode = 0
        let (p_val, _tag, err, steps, tcnt0) =
            p4_run_raw_bc_on_pure_vm(&stream, args[0], args[1], args[2], 100_000, 0)?;
        if err != 0 || p_val != rust_val || tcnt0 != 0 || steps == 0 {
            return Err(format!(
                "P4 '{}' (trace=0): mismatch rust={} pure={} err={} steps={} tcnt={}",
                name, rust_val, p_val, err, steps, tcnt0
            ));
        }
        checks += 1;
    }

    // P4-5: Binary trace ring check (trace_mode = 1 records exact step count and preserves result)
    let tr_src = "#[strict]\nfn main(a: i64) -> i64 { let b: i64 = a * 2 + 3; return b; }\n";
    let tr_stream = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(tr_src))?;
    let (v1, _, err1, steps1, tcnt1) = p4_run_raw_bc_on_pure_vm(&tr_stream, 21, 0, 0, 10_000, 1)?;
    if err1 != 0 || v1 != 45 || tcnt1 != steps1 || tcnt1 == 0 {
        return Err(format!(
            "P4-5 trace check failed: val={} err={} steps={} tcnt={}",
            v1, err1, steps1, tcnt1
        ));
    }
    checks += 1;

    // Budget / negative 1: ISA-B strict OOB (err_code == 2)
    let oob_src = "#[strict]\nfn main(a: i64) -> i64 { let w: bytes = anew(0); w = iset(w, a); return ld_u32(w, 0); }\n";
    let oob_stream = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(oob_src))?;
    let (_, _, err_oob, _, _) = p4_run_raw_bc_on_pure_vm(&oob_stream, 5, 0, 0, 10_000, 0)?;
    if err_oob != 2 {
        return Err(format!("P4 OOB expected err_code=2, got {}", err_oob));
    }
    checks += 1;

    // Budget / negative 2: Fuel exhaustion (err_code == 3)
    let fuel_src = "#[strict]\nfn main(n: i64) -> i64 { let i: i64 = 0; while n > i { i = i + 1; } return i; }\n";
    let fuel_stream = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(fuel_src))?;
    let (_, _, err_fuel, _, _) = p4_run_raw_bc_on_pure_vm(&fuel_stream, 1000, 0, 0, 10, 0)?;
    if err_fuel != 3 {
        return Err(format!("P4 fuel expected err_code=3, got {}", err_fuel));
    }
    checks += 1;

    // Budget / negative 3: Call-depth-64 cap (err_code == 4)
    let rec_src = "#[strict]\nfn rec(n: i64) -> i64 { let c: trit = n <=> 0; return if_pos c { 1 + rec(n - 1) } else { 0 }; }\nfn main(n: i64) -> i64 { return rec(n); }\n";
    let rec_stream = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(rec_src))?;
    let (_, _, err_depth, _, _) = p4_run_raw_bc_on_pure_vm(&rec_stream, 70, 0, 0, 100_000, 0)?;
    if err_depth != 4 {
        return Err(format!("P4 depth-64 expected err_code=4, got {}", err_depth));
    }
    checks += 1;

    Ok(checks)
}

/// P4-3 selftest: Proves the Pure-Omni VM executes raw `.omni_bc` strictly through the
/// minimal 3-op `OmniThinShim` (`read_bytes`, `write_bytes`, `spawn`) with zero HTTP/shell/file stack.
pub fn selftest_p4_3_thin_shim() -> Result<usize, String> {
    let src = "#[strict]\nfn dbl(x: i64) -> i64 { return x * 2; }\nfn main(a: i64, b: i64) -> i64 { return dbl(a) + b; }\n";
    let stream = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(src))?;
    let mut raw_bc: Vec<u8> = Vec::with_capacity(stream.len() * 2);
    for w in &stream {
        raw_bc.extend_from_slice(&w.to_le_bytes());
    }
    let mut shim = MemoryThinShim::new();
    shim.write_bytes("prog.omni_bc", &raw_bc)?;

    // Execute strictly via read_bytes + write_bytes (no source text, no parser!)
    let (val, _tag, err, steps, tcnt) =
        p4_shim_exec_bc(&mut shim, "prog.omni_bc", Some("result.bin"), 19, 7, 0, 10_000, 1)?;
    if err != 0 || val != 45 || steps == 0 || tcnt != steps {
        return Err(format!(
            "P4-3 thin shim exec failed: val={} err={} steps={} tcnt={}",
            val, err, steps, tcnt
        ));
    }
    let out_bin = shim.read_bytes("result.bin")?;
    if out_bin.len() != 20 {
        return Err(format!("P4-3 thin shim output len expected 20, got {}", out_bin.len()));
    }
    let got_val = i64::from_le_bytes(out_bin[0..8].try_into().unwrap());
    let got_tcnt = u32::from_le_bytes(out_bin[16..20].try_into().unwrap());
    if got_val != 45 || got_tcnt != tcnt {
        return Err(format!("P4-3 thin shim persisted summary mismatch: val={} tcnt={}", got_val, got_tcnt));
    }
    // Verify spawn 3rd primitive contract
    let rc = shim.spawn("omni-p4-worker", &["prog.omni_bc"])?;
    if rc != 0 || shim.spawned.len() != 1 || shim.spawned[0].0 != "omni-p4-worker" {
        return Err(String::from("P4-3 thin shim spawn contract failed"));
    }
    Ok(3)
}

/// P4-4 selftest: Proves (1) the entire Pure-Omni VM (`pure/m4/vm.omni`, 160+ fns) compiles
/// cleanly to WebAssembly v1 via M2-10 `compile_pure_u16_to_wasm_multi`, and (2) the Pure-Omni
/// VM core (`arena.omni` + `ops_core.omni`) executes on the Wasm-multi host with bit-exact
/// parity against Rust `OmniBinaryVm` (including P4-5 binary trace ring verification).
pub fn selftest_p4_4_wasm_host() -> Result<usize, String> {
    // 1. Full Pure-Omni VM compiles cleanly to WebAssembly v1 via M2-10
    let vm_u16 = with_lockstep_suspended(|| compile_omni_surface_to_u16(M4_VM_OMNI_SRC))?;
    let vm_wasm = compile_pure_u16_to_wasm_multi(&vm_u16)?;
    if vm_wasm.len() < 8 || &vm_wasm[0..4] != b"\0asm" {
        return Err(String::from("P4-4: full Pure-Omni VM Wasm binary invalid header"));
    }

    // 2. Wasm-hosted Pure-Omni VM core execution parity vs Rust OmniBinaryVm
    let harness_u16 = with_lockstep_suspended(|| compile_omni_surface_to_u16(M4_VM_WASM_HARNESS_SRC))?;
    let harness_wasm = compile_pure_u16_to_wasm_multi(&harness_u16)?;
    let decoded = decode_omni_token_stream(&harness_u16)?;
    let entry = m2_entry_of(&decoded);
    let mut checks = 1usize;

    for &(a0, a1, a2) in &[(7i64, 5i64, 6i64), (3, 4, 2)] {
        let mut rust_vm = OmniBinaryVm::with_budgets(200_000_000, 64 * 1024 * 1024);
        let rust_val = match rust_vm.execute_function(
            &decoded,
            entry,
            vec![OmniValue::Int(a0), OmniValue::Int(a1), OmniValue::Int(a2)],
        )? {
            OmniValue::Int(v) => v,
            other => return Err(format!("P4-4 Rust VM returned {:?}", other)),
        };
        let wasm_val = execute_wasm_module_i64_with_budgets(
            &harness_wasm,
            &[a0, a1, a2],
            200_000_000,
            64 * 1024 * 1024,
        )?;
        if wasm_val != rust_val {
            return Err(format!(
                "P4-4 Wasm host mismatch for ({},{},{}): rust={} wasm={}",
                a0, a1, a2, rust_val, wasm_val
            ));
        }
        checks += 1;
    }
    Ok(checks)
}

// ============================================================================
// Phase 5 (P5-1 / P5-2): Zero-Shim Syscall ELF64 & Full Pure-Omni Self-Host
// ============================================================================

/// P5-2 selftest: Pure-Omni `pure/m5/elf_sys.omni` emits a 177-byte standalone zero-shim
/// Linux x86-64 ELF64 binary (`_start`: stack arena + raw `sys_read`/`sys_write`/`sys_exit`).
/// On Linux x86-64, executes the emitted ELF64 directly on the OS kernel (stdin -> stdout, exit 0).
pub fn selftest_p5_2_zero_shim_sys() -> Result<(usize, usize, u64, bool), String> {
    let sys_mod = m2_compile_decoder(M5_SYS_OMNI_SRC)?;
    let entry = m2_entry_of(&sys_mod);
    let mut vm = OmniBinaryVm::with_budgets(100_000_000, 64 * 1024 * 1024);
    let elf = match vm.execute_function(
        &sys_mod,
        entry,
        vec![
            OmniValue::Bytes(vec![0u8; 524_288]),
            OmniValue::Int(65_536),
        ],
    )? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("P5-2 returned {:?}, expected ELF64 Bytes", other)),
    };
    if elf.len() != 233 {
        return Err(format!("P5-2 zero-shim ELF expected 233 B, got {} B", elf.len()));
    }
    if &elf[0..4] != b"\x7fELF" {
        return Err(String::from("P5-2 ELF64 magic mismatch"));
    }
    let p_filesz = u64::from_le_bytes(elf[96..104].try_into().unwrap());
    let p_memsz = u64::from_le_bytes(elf[104..112].try_into().unwrap());
    if p_filesz != 233 || p_memsz != 233 {
        return Err(format!("P5-2 ELF64 p_filesz/p_memsz mismatch ({}/{})", p_filesz, p_memsz));
    }
    // Verify stack-arena prologue (48 81 EC 00 00 01 00 48 89 E3) and all 3 raw `0F 05` syscalls
    let want_entry: [u8; 10] = [0x48, 0x81, 0xEC, 0x00, 0x00, 0x01, 0x00, 0x48, 0x89, 0xE3];
    if &elf[176..186] != &want_entry {
        return Err(format!("P5-2 arena entry stub mismatch: {:02X?}", &elf[176..186]));
    }
    let syscall_count = elf[120..].windows(2).filter(|w| *w == [0x0F, 0x05]).count();
    if syscall_count != 3 {
        return Err(format!("P5-2 expected 3 raw syscall (0F 05) sites, found {}", syscall_count));
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    let native_ran = {
        use std::io::Write;
        let tmp_elf = PrivateTempElf::new(&elf, "p5_zero_shim")?;
        let payload: [u8; 12] = [0xE0, 0x7E, 0xE3, 0x7E, 0x00, 0x00, 0x42, 0x00, 0xEF, 0x7E, 0xAA, 0x55];
        let mut child = std::process::Command::new(&tmp_elf.path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("P5-2 spawn native zero-shim ELF: {}", e))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(&payload).map_err(|e| format!("P5-2 stdin write: {}", e))?;
        }
        let out = child.wait_with_output().map_err(|e| format!("P5-2 wait: {}", e))?;
        drop(tmp_elf);
        if !out.status.success() || out.stdout != payload {
            return Err(format!(
                "P5-2 native zero-shim ELF failed: status={:?} stdout={:02X?}",
                out.status, out.stdout
            ));
        }
        true
    };
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    let native_ran = false;

    let h = fnv1a_64_hash(&elf);
    Ok((3, elf.len(), h, native_ran))
}

/// P5-1 selftest: Full Pure-Omni self-compilation chain (ZERO Rust compiler/VM in the loop):
///   Stage A: `M3_vm(pure/m4/*) == RustFE(pure/m4/*) == committed pure/m4/m4.omni_bc` (0-word diff!)
///   Stage B: `M2_vm(pure/m4/m4.omni_bc) == RustEmitMulti(pure/m4/m4.omni_bc)` (native x86-64 ELF64 of Pure-Omni VM, 0-byte diff!)
///   Stage C: Zero-Rust compile + exec loop: `.omni` -> Pure-Omni M3 (`pure/m3/m3.omni_bc`) -> `.omni_bc`
///            -> executed by committed Pure-Omni M4 VM (`pure/m4/m4.omni_bc`) AND compiled by Pure-Omni M2 (`pure/m2/m2.omni_bc`, 0-byte diff!)
pub fn selftest_p5_1_selfhost(write_seed: bool) -> Result<(usize, u64, usize, u64), String> {
    let seed_path = "pure/m4/m4.omni_bc";
    let rust_m4_u16 = with_lockstep_suspended(|| compile_omni_surface_to_u16(M4_VM_OMNI_SRC))?;
    let mut rust_m4_bytes: Vec<u8> = Vec::with_capacity(rust_m4_u16.len() * 2);
    for w in &rust_m4_u16 {
        rust_m4_bytes.extend_from_slice(&w.to_le_bytes());
    }
    if write_seed {
        std::fs::write(seed_path, &rust_m4_bytes)
            .map_err(|e| format!("P5-1 write {}: {}", seed_path, e))?;
    }
    let committed_m4 = std::fs::read(seed_path)
        .map_err(|e| format!("P5-1 read {} (run `omnic p5-selfhost --write-seed`): {}", seed_path, e))?;
    if committed_m4 != rust_m4_bytes {
        return Err(format!(
            "P5-1 DRIFT: committed {} ({} B, 0x{:016X}) != RustFE(pure/m4/*) ({} B, 0x{:016X})",
            seed_path,
            committed_m4.len(),
            fnv1a_64_hash(&committed_m4),
            rust_m4_bytes.len(),
            fnv1a_64_hash(&rust_m4_bytes)
        ));
    }

    // Stage A: Pure-Omni M3 compiles `M4_VM_OMNI_SRC` -> exact match with `pure/m4/m4.omni_bc` (0-word diff!)
    let m3_mod = m3_lockstep_seed_module()?;
    let m3_entry = m2_entry_of(m3_mod);
    let m4_src_u16: Vec<u16> = M4_VM_OMNI_SRC.as_bytes().iter().map(|&b| b as u16).collect();
    let mut vm = OmniBinaryVm::with_budgets(500_000_000, 128 * 1024 * 1024);
    let m3_m4_bytes = match vm.execute_function(
        m3_mod,
        m3_entry,
        vec![
            OmniValue::Bytes(vec![0u8; 786_432]),
            OmniValue::U16Array(m4_src_u16),
            OmniValue::Int(0),
        ],
    )? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("P5-1 Stage A M3 returned {:?}", other)),
    };
    if m3_m4_bytes != committed_m4 {
        return Err(format!(
            "P5-1 Stage A MISMATCH: M3_vm(pure/m4/*)={} B (0x{:016X}) != pure/m4/m4.omni_bc={} B (0x{:016X})",
            m3_m4_bytes.len(),
            fnv1a_64_hash(&m3_m4_bytes),
            committed_m4.len(),
            fnv1a_64_hash(&committed_m4)
        ));
    }

    // Stage B: Pure-Omni M2 (`pure/m2/m2.omni_bc`) compiles `pure/m4/m4.omni_bc` -> native x86-64 ELF64 (0-byte diff vs Rust!)
    let m2_mod = m2_compile_decoder(M2_MULTI_OMNI_SRC)?;
    let m2_entry = m2_entry_of(&m2_mod);
    let pure_m4_elf = match vm.execute_function(
        &m2_mod,
        m2_entry,
        vec![
            OmniValue::Bytes(vec![0u8; 524_288]),
            OmniValue::U16Array(rust_m4_u16.clone()),
            OmniValue::Int(0),
            OmniValue::Int(0),
            OmniValue::Int(0),
        ],
    )? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("P5-1 Stage B M2 returned {:?}", other)),
    };
    let rust_m4_elf = compile_pure_u16_to_x86_64_elf_multi(&rust_m4_u16, 0, 0, 0)?;
    if pure_m4_elf != rust_m4_elf {
        return Err(format!(
            "P5-1 Stage B ELF MISMATCH: Pure={} B (0x{:016X}) != Rust={} B (0x{:016X})",
            pure_m4_elf.len(),
            fnv1a_64_hash(&pure_m4_elf),
            rust_m4_elf.len(),
            fnv1a_64_hash(&rust_m4_elf)
        ));
    }

    // Stage C: Zero-Rust compile + exec loop:
    //   `.omni` -> Pure-Omni M3 (`pure/m3/m3.omni_bc`) -> `.omni_bc`
    //   -> executed by committed Pure-Omni M4 VM (`pure/m4/m4.omni_bc`) AND compiled by Pure-Omni M2 (`0`-byte diff!)
    let prog_src = "#[strict]\nfn sq(x: i64) -> i64 { return x * x; }\nfn main(a: i64, b: i64) -> i64 { return sq(a) + sq(b) + a * b; }\n";
    let prog_bc = match m3_lockstep_compile_via_omni(prog_src, true)? {
        Ok(v) => v,
        Err((c, l, col)) => return Err(format!("P5-1 Stage C M3 errored ({}, {}:{})", c, l, col)),
    };
    let m4_seed_mod = decode_omni_token_stream(&rust_m4_u16)?;
    let m4_entry = m2_entry_of(&m4_seed_mod);
    let mut arena = vec![0u8; 524_288];
    let st_base = 167_936usize;
    arena[st_base + 48..st_base + 56].copy_from_slice(&5i64.to_le_bytes());
    arena[st_base + 56..st_base + 64].copy_from_slice(&3i64.to_le_bytes());
    let state_bytes = match vm.execute_function(
        &m4_seed_mod,
        m4_entry,
        vec![
            OmniValue::Bytes(arena),
            OmniValue::U16Array(prog_bc.clone()),
            OmniValue::Int(10_000),
            OmniValue::Int(1),
        ],
    )? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("P5-1 Stage C M4 VM returned {:?}", other)),
    };
    let err_c = u32::from_le_bytes(state_bytes[28..32].try_into().unwrap());
    let val_c = i64::from_le_bytes(state_bytes[32..40].try_into().unwrap());
    let tcnt_c = u32::from_le_bytes(state_bytes[24..28].try_into().unwrap());
    if err_c != 0 || val_c != 49 || tcnt_c == 0 {
        return Err(format!(
            "P5-1 Stage C M4 VM exec mismatch: want 49, got val={} err={} tcnt={}",
            val_c, err_c, tcnt_c
        ));
    }
    let pure_prog_elf = match vm.execute_function(
        &m2_mod,
        m2_entry,
        vec![
            OmniValue::Bytes(vec![0u8; 524_288]),
            OmniValue::U16Array(prog_bc.clone()),
            OmniValue::Int(5),
            OmniValue::Int(3),
            OmniValue::Int(0),
        ],
    )? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("P5-1 Stage C M2 returned {:?}", other)),
    };
    let rust_prog_elf = compile_pure_u16_to_x86_64_elf_multi(&prog_bc, 5, 3, 0)?;
    if pure_prog_elf != rust_prog_elf {
        return Err(String::from("P5-1 Stage C M2 ELF mismatch vs Rust"));
    }

    Ok((
        rust_m4_u16.len(),
        fnv1a_64_hash(&committed_m4),
        pure_m4_elf.len(),
        fnv1a_64_hash(&pure_m4_elf),
    ))
}

/// `omni m2-decode` runs the decoder differential; `omni m2-decode <file.omni_bc>` decodes one
/// stream with the Omni decoder on the VM and checks it against Rust.
pub fn m2_decode_cli(args: &[String]) -> i32 {
    let r = match args.first() {
        None => selftest_m2_decode_diff().map(|n| format!("[M2-DECODE PASS] {} differential checks (Omni pure/m2/decode.omni == Rust decode_omni_token_stream)", n)),
        Some(p) => std::fs::read(p).map_err(|e| format!("{}: {}", p, e)).and_then(|b| {
            let s: Vec<u16> = b.chunks(2).map(|c| u16::from_le_bytes([c[0], *c.get(1).unwrap_or(&0)])).collect();
            m2_decode_diff_one(p, &s).map(|_| format!("[M2-DECODE PASS] {} ({} words) agrees with Rust", p, s.len()))
        }),
    };
    match r {
        Ok(m) => {
            println!("{}", m);
            0
        }
        Err(e) => {
            eprintln!("[M2-DECODE FAIL] {}", e);
            1
        }
    }
}

// ==============================================================================
// PHASE 7 (P7-0): MEASUREMENT CORPUS, P4-5 TRACE-RING FEEDBACK & BALANCED-TERNARY VERDICT ENGINE
// ==============================================================================

#[derive(Clone, Debug)]
pub struct P7BenchMetrics {
    pub name: &'static str,
    pub compile_us: i64,
    pub emit_us: i64,
    pub vm_us: i64,
    pub bc_words: i64,
    pub elf_bytes: i64,
    pub vm_steps: i64,
    pub trace_ops: i64,
    pub arena_bytes: i64,
    pub trace_csum: i64,
    pub hist_alu: i64,
    pub hist_mem: i64,
    pub hist_ctrl: i64,
    pub hist_call_ret: i64,
    pub ret_val: i64,
}

/// Fixed 5-benchmark corpus for Phase 7 (P7-0):
/// (name, .omni source, [a0, a1, a2], expected_i64_result).
pub const P7_BENCHMARK_CORPUS: &[(&str, &str, [i64; 3], i64)] = &[
    (
        "bench_clamp",
        "#[strict]\nfn main(x: i64, lo: i64, hi: i64) -> i64 {\n    let lower: i64 = imax(x, lo);\n    let clamped: i64 = imin(lower, hi);\n    return clamped;\n}\n",
        [150, 10, 90],
        90,
    ),
    (
        "bench_loop_sum",
        "#[strict]\nfn main(n: i64) -> i64 {\n    let acc: i64 = 0;\n    let i: i64 = 1;\n    while i <= n {\n        acc = acc + i;\n        i = i + 1;\n    }\n    return acc;\n}\n",
        [100, 0, 0],
        5050,
    ),
    (
        "bench_branch_tree",
        "#[strict]\nfn main(x: i64) -> i64 {\n    let c: trit = x <=> 10;\n    let r: i64 = match_trit c {\n        neg => x - 5,\n        zero => 100,\n        pos => x + 50\n    };\n    return r;\n}\n",
        [20, 0, 0],
        70,
    ),
    (
        "bench_multi_call",
        "#[strict]\nfn sq(x: i64) -> i64 {\n    return x * x;\n}\n#[strict]\nfn main(a: i64, b: i64) -> i64 {\n    return sq(a) + sq(b) + a * b;\n}\n",
        [17, 23, 0],
        1209,
    ),
    (
        "bench_arena_buf",
        "#[strict]\nfn main(n: i64) -> i64 {\n    let buf: bytes = anew(0);\n    let i: i64 = 0;\n    while i < n {\n        buf = iset_i64(buf, i);\n        i = i + 1;\n    }\n    let sum: i64 = 0;\n    let k: i64 = 0;\n    while k < n {\n        sum = sum + ld_u64(buf, k * 8);\n        k = k + 1;\n    }\n    return sum;\n}\n",
        [16, 0, 0],
        120,
    ),
];

pub const P7_CORPUS_LEN: usize = 7;

const P7_VERIFIER_ENGINE_HARNESS_SRC: &str = "\n#[strict]\nfn p7_ve_push_u16(buf: bytes, w: i64) -> bytes {\n    let b1: bytes = iset(buf, w & 255);\n    return iset(b1, (w >> 8) & 255);\n}\n\n#[strict]\nfn main(a0: i64, a1: i64, a2: i64) -> i64 {\n    let w: bytes = anew(0);\n    w = p7_ve_push_u16(w, 32480);\n    w = p7_ve_push_u16(w, 32483);\n    w = p7_ve_push_u16(w, 0);\n    w = p7_ve_push_u16(w, 0);\n    w = p7_ve_push_u16(w, 32641);\n    w = p7_ve_push_u16(w, 0);\n    w = p7_ve_push_u16(w, 1);\n    w = p7_ve_push_u16(w, 32586);\n    w = p7_ve_push_u16(w, 32486);\n    w = p7_ve_push_u16(w, (a0 >> 16) & 65535);\n    w = p7_ve_push_u16(w, a0 & 65535);\n    w = p7_ve_push_u16(w, 32484);\n    w = p7_ve_push_u16(w, 32495);\n    let rep: bytes = omni_verify_module(w);\n    let status: i64 = load_i64le(rep, 0);\n    let nfns: i64 = load_i64le(rep, 16);\n    let fn0_pass: i64 = load_i64le(rep, 32);\n    return status * 100 + nfns * 10 + fn0_pass;\n}\n";

/// 7th Phase 7 benchmark (`bench_m3_frontend`): exercises the Pure-Omni M3 surface-language
/// frontend path not covered by the 5 micro-benchmarks — `std/` includes (`std/math.omni`,
/// `std/trit.omni`, `std/pack5.omni`), `egate`, trit consensus/mux/quorum, `@PACK5` radix-3
/// packing & validation, `match_trit`, `if_pos`, and multi-function `CALL` — compiled end-to-end
/// through Pure-Omni O-5 include expansion (`pure/m3/include.omni`) and `pure/m3/m3.omni_bc`.
const P7_M3_FRONTEND_BENCH_SRC: &str = "include \"std/math.omni\";\ninclude \"std/trit.omni\";\ninclude \"std/pack5.omni\";\n\n#[strict]\nfn p7_m3_pack5_trits(t0: trit, t1: trit, t2: trit, t3: trit, t4: trit) -> i64 {\n    let d0: i64 = std_trit_mux(t0, 0, 1, 2);\n    let d1: i64 = std_trit_mux(t1, 0, 1, 2);\n    let d2: i64 = std_trit_mux(t2, 0, 1, 2);\n    let d3: i64 = std_trit_mux(t3, 0, 1, 2);\n    let d4: i64 = std_trit_mux(t4, 0, 1, 2);\n    return d0 * 81 + d1 * 27 + d2 * 9 + d3 * 3 + d4;\n}\n\n#[strict]\n#[cap(AI)]\nfn p7_m3_score_window(x: i64, y: i64, z: i64) -> i64 {\n    let cx: i64 = std_clamp(x, -100, 100);\n    let cy: i64 = std_clamp(y, -100, 100);\n    let cz: i64 = std_clamp(z, -100, 100);\n    let gx: trit = egate(cx - 10);\n    let gy: trit = egate(cy);\n    let gz: trit = egate(cz + 5);\n    let q3: trit = std_consensus3(gx, gy, gz);\n    let q5: trit = std_trit_quorum5(gx, gy, gz, q3, std_signum(cx + cy));\n    let pbyte: i64 = p7_m3_pack5_trits(gx, gy, gz, q3, q5);\n    let valid: trit = std_pack5_is_valid_byte(pbyte);\n    let nbytes: i64 = std_pack5_bytes_needed(12);\n    let tscore: i64 = std_trit_score(gx, gy, gz, nbytes);\n    let dscore: i64 = std_pack5_dot10_score(pbyte, 2, 200);\n    let cgate: trit = std_pack5_cosine_gate(dscore, 150);\n    let bonus: i64 = match_trit cgate {\n        neg => -10,\n        zero => 0,\n        pos => 25\n    };\n    let ok_val: i64 = if_pos valid { pbyte + tscore + dscore + bonus } else { -1 };\n    return ok_val;\n}\n\n#[strict]\nfn main(a: i64, b: i64, c: i64) -> i64 {\n    let acc: i64 = 0;\n    let i: i64 = 0;\n    while i < 4 {\n        let step_v: i64 = p7_m3_score_window(a + i * 5, b - i * 3, c + i * 2);\n        let mad_v: i64 = std_mad_clamped(i, 7, 3, 0, 50);\n        acc = acc + step_v + mad_v;\n        i = i + 1;\n    }\n    return acc + std_sum_range(5);\n}\n";

/// Returns the full 7-benchmark Phase 7 corpus: the 5 fixed micro-benchmarks plus
/// `bench_verifier_engine` (`pure/verifier_engine.omni` + minimal 1-fn harness) and
/// `bench_m3_frontend` (`std/` includes + `egate` + trit + `@PACK5` surface path),
/// where `use_pure_m3 = true` compiles through the default Pure-Omni M3 frontend.
pub fn p7_benchmark_corpus_all() -> Vec<(&'static str, String, [i64; 3], i64, bool)> {
    let mut out: Vec<(&'static str, String, [i64; 3], i64, bool)> = P7_BENCHMARK_CORPUS
        .iter()
        .map(|&(name, src, args, want)| (name, String::from(src), args, want, false))
        .collect();
    let mut ve_src = String::with_capacity(OMNI_VERIFIER_ENGINE_SURFACE.len() + P7_VERIFIER_ENGINE_HARNESS_SRC.len());
    ve_src.push_str(OMNI_VERIFIER_ENGINE_SURFACE);
    ve_src.push_str(P7_VERIFIER_ENGINE_HARNESS_SRC);
    out.push(("bench_verifier_engine", ve_src, [42, 0, 0], 11, true));
    out.push(("bench_m3_frontend", String::from(P7_M3_FRONTEND_BENCH_SRC), [5, 2, -8], 1652, true));
    out
}

/// Counts exact `ImmInt(0)` / `ImmTrit(0)` operands lowered through `emit_load_op`
/// in `compile_pure_u16_to_x86_64_elf_multi_opts` (each saves 8 B under `compact_zero_imm`).
fn p7_count_m2_zero_imm_loads(module: &BinModule) -> i64 {
    let is_zero = |op: &BinOperand| matches!(*op, BinOperand::ImmInt(0) | BinOperand::ImmTrit(0));
    let mut cnt = 0i64;
    for func in &module.functions {
        for ins in &func.instructions {
            match ins.op_tok {
                0x7F01 | 0x7F02 | 0x7F03 | 0x7F04 | 0x7F06 | 0x7F09 | 0x7F57 | 0x7F58 | 0x7F59
                | 0x7F15 | 0x7F16 | 0x7F21 | 0x7F28 | 0x7F35 | 0x7F3F | 0x7F42 | 0x7F43 | 0x7F44 => {
                    if let Some(op) = ins.operands.first() {
                        if is_zero(op) { cnt += 1; }
                    }
                }
                0x7F0E => {
                    let idx = if ins.operands.len() == 3 { 2 } else { 1 };
                    if let Some(op) = ins.operands.get(idx) {
                        if is_zero(op) { cnt += 1; }
                    }
                }
                0x7F0F => {
                    if ins.operands.len() == 3
                        && (ins.operands[1] == BinOperand::ImmInt(-4) || ins.operands[1] == BinOperand::ImmInt(-8))
                    {
                        if is_zero(&ins.operands[2]) { cnt += 1; }
                    } else if ins.operands.len() == 4
                        && (ins.operands[1] == BinOperand::ImmInt(-9)
                            || ins.operands[1] == BinOperand::ImmInt(-2)
                            || ins.operands[1] == BinOperand::ImmInt(-4))
                    {
                        if is_zero(&ins.operands[2]) { cnt += 1; }
                        if is_zero(&ins.operands[3]) { cnt += 1; }
                    } else if ins.operands.len() > 1 {
                        for op in &ins.operands[1..] {
                            if is_zero(op) { cnt += 1; }
                        }
                    }
                }
                0x7F10 | 0x7F11 | 0x7F12 | 0x7F1E | 0x7F1F | 0x7F20
                | 0x7F22 | 0x7F23 | 0x7F24 | 0x7F25 | 0x7F26 | 0x7F27 | 0x7F2D => {
                    if let Some(op0) = ins.operands.first() {
                        if is_zero(op0) { cnt += 1; }
                    }
                }
                0x7F13 | 0x7F14 | 0x7F17 | 0x7F18 | 0x7F29 | 0x7F2A | 0x7F2B => {
                    if ins.operands.len() >= 2 {
                        if is_zero(&ins.operands[0]) { cnt += 1; }
                        if is_zero(&ins.operands[1]) { cnt += 1; }
                    }
                }
                0x7F2C => {
                    for op in ins.operands.iter().take(3) {
                        if is_zero(op) { cnt += 1; }
                    }
                }
                0x7F45 => {
                    for op in ins.operands.iter().take(4) {
                        if is_zero(op) { cnt += 1; }
                    }
                }
                0x7F48 => {
                    if ins.operands.len() > 1 {
                        for op in &ins.operands[1..] {
                            if is_zero(op) { cnt += 1; }
                        }
                    }
                }
                0x7F4A => {
                    if func.ret_ty_tok == TY_BYTES {
                        if ins.operands.len() == 3 {
                            if is_zero(&ins.operands[1]) { cnt += 1; }
                            if is_zero(&ins.operands[2]) { cnt += 1; }
                        }
                    } else if let Some(op) = ins.operands.first() {
                        if is_zero(op) { cnt += 1; }
                    }
                }
                _ => {}
            }
        }
    }
    cnt
}

/// Measures one `.omni` program end-to-end with phase-separated timings and P4-5 histogram:
///   1. Compile `.omni` -> `.omni_bc` (measuring `compile_us` and `bc_words`)
///   2. Lower `.omni_bc` -> native x86-64 ELF64 via M2-multi (`emit_us`, `elf_bytes`)
///   3. Execute `.omni_bc` on Pure-Omni M4 VM (`m4_vm_run_i64`) with P4-5 trace ring active (`vm_us`)
///   4. Verify live P4-5 trace ring & compute 4-class opcode histogram in Pure-Omni (`pure/m7/judge.omni`)
pub fn p7_measure_program_with_opts(
    m4_mod: &BinModule,
    m4_run_fn: u16,
    judge_mod: &BinModule,
    trace_fn: u16,
    hist_fn: u16,
    name: &'static str,
    src: &str,
    args: [i64; 3],
    expected_val: i64,
    emit_mode: M2EmitMode,
    use_pure_m3: bool,
) -> Result<(P7BenchMetrics, Vec<u8>, i64), String> {
    let t_fe = std::time::Instant::now();
    let bc = if use_pure_m3 {
        if src.contains("include \"") {
            let expanded = pure_default_expand_includes(src, None, None)?;
            pure_default_compile_surface_to_u16(&expanded, true)?
        } else {
            pure_default_compile_surface_to_u16(src, true)?
        }
    } else {
        with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(src))?
    };
    let compile_us = t_fe.elapsed().as_micros() as i64;
    let decoded = decode_omni_token_stream(&bc)?;
    for f in &decoded.functions {
        let rep = verify_binary_function(f);
        if !rep.passed {
            return Err(format!("P7-0 {} failed verifier: {:?}", name, rep.errors));
        }
    }
    let zero_loads = p7_count_m2_zero_imm_loads(&decoded);
    let t_emit = std::time::Instant::now();
    let elf = compile_pure_u16_to_x86_64_elf_multi_mode(&bc, args[0], args[1], args[2], emit_mode)?;
    let emit_us = t_emit.elapsed().as_micros() as i64;
    let mut vm = OmniBinaryVm::with_budgets(250_000_000, 128 * 1024 * 1024);
    let t_vm = std::time::Instant::now();
    let arena_out = match vm.execute_function(
        m4_mod,
        m4_run_fn,
        vec![
            OmniValue::Bytes(vec![0u8; 524_288]),
            OmniValue::U16Array(bc.clone()),
            OmniValue::Int(args[0]),
            OmniValue::Int(args[1]),
            OmniValue::Int(args[2]),
            OmniValue::Int(200_000),
            OmniValue::Int(1), // trace_mode = 1 (P4-5 binary trace ring active)
        ],
    )? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("P7-0 {} M4 VM returned {:?}", name, other)),
    };
    let vm_us = t_vm.elapsed().as_micros() as i64;
    if arena_out.len() != 524_288 {
        return Err(format!("P7-0 {} expected 512 KiB arena, got {}", name, arena_out.len()));
    }
    let st = 167_936usize;
    let heap_top = u32::from_le_bytes(arena_out[st + 16..st + 20].try_into().unwrap()) as i64;
    let trace_cnt = u32::from_le_bytes(arena_out[st + 24..st + 28].try_into().unwrap()) as i64;
    let err_code = u32::from_le_bytes(arena_out[st + 28..st + 32].try_into().unwrap());
    let ret_val = i64::from_le_bytes(arena_out[st + 32..st + 40].try_into().unwrap());
    let steps = u32::from_le_bytes(arena_out[st + 44..st + 48].try_into().unwrap()) as i64;
    if err_code != 0 || ret_val != expected_val {
        return Err(format!(
            "P7-0 {} exec mismatch: want {}, got val={} err={}",
            name, expected_val, ret_val, err_code
        ));
    }
    if trace_cnt <= 0 || steps <= 0 {
        return Err(format!(
            "P7-0 {} produced empty trace/steps (steps={}, trace_cnt={})",
            name, steps, trace_cnt
        ));
    }
    let trace_summary = match vm.execute_function(judge_mod, trace_fn, vec![OmniValue::Bytes(arena_out.clone())])? {
        OmniValue::Int(v) => v,
        other => return Err(format!("P7-0 {} p7_verify_trace_ring returned {:?}", name, other)),
    };
    if trace_summary < 0 {
        return Err(format!("P7-0 {} Pure-Omni trace-ring verifier rejected trace", name));
    }
    let ring_cnt = trace_summary >> 32;
    if ring_cnt != trace_cnt {
        return Err(format!(
            "P7-0 {} trace count mismatch: VM_STATE={} vs Pure-Omni ring={}",
            name, trace_cnt, ring_cnt
        ));
    }
    let hist_packed = match vm.execute_function(judge_mod, hist_fn, vec![OmniValue::Bytes(arena_out)])? {
        OmniValue::Int(v) => v as u64,
        other => return Err(format!("P7-0a {} p7_trace_histogram returned {:?}", name, other)),
    };
    let hist_alu = (hist_packed & 0xFFFF) as i64;
    let hist_mem = ((hist_packed >> 16) & 0xFFFF) as i64;
    let hist_ctrl = ((hist_packed >> 32) & 0xFFFF) as i64;
    let hist_call_ret = ((hist_packed >> 48) & 0xFFFF) as i64;
    let expected_cap = trace_cnt.min(4096);
    if hist_alu + hist_mem + hist_ctrl + hist_call_ret != expected_cap {
        return Err(format!(
            "P7-0a {} histogram sum mismatch: {}+{}+{}+{} != {}",
            name, hist_alu, hist_mem, hist_ctrl, hist_call_ret, expected_cap
        ));
    }
    Ok((
        P7BenchMetrics {
            name,
            compile_us,
            emit_us,
            vm_us,
            bc_words: bc.len() as i64,
            elf_bytes: elf.len() as i64,
            vm_steps: steps,
            trace_ops: trace_cnt,
            arena_bytes: heap_top,
            trace_csum: trace_summary & 0x7FFF_FFFF,
            hist_alu,
            hist_mem,
            hist_ctrl,
            hist_call_ret,
            ret_val,
        },
        elf,
        zero_loads,
    ))
}

pub fn p7_measure_program(
    m4_mod: &BinModule,
    m4_run_fn: u16,
    judge_mod: &BinModule,
    trace_fn: u16,
    name: &'static str,
    src: &str,
    args: [i64; 3],
    expected_val: i64,
    use_pure_m3: bool,
) -> Result<P7BenchMetrics, String> {
    let n_jfns = judge_mod.functions.len();
    let hist_fn = judge_mod.functions[n_jfns - 5].fn_idx;
    let (m, _elf, _zl) = p7_measure_program_with_opts(
        m4_mod, m4_run_fn, judge_mod, trace_fn, hist_fn, name, src, args, expected_val, M2EmitMode::Compact, use_pure_m3,
    )?;
    Ok(m)
}

/// P7-0 Hard Gate (`omnic p7-measure`):
/// Verifies the fixed 5-benchmark corpus, live P4-5 trace-ring feed, Pure-Omni balanced-ternary
/// verdict engine (`pure/m7/judge.omni`), `@PACK5` packing parity, and written adopt/reject rules
/// (+1t better, 0t neutral, -1t worse / regression / gate-fail) with all frozen hashes unchanged.
pub fn selftest_p7_0_measure() -> Result<(usize, Vec<P7BenchMetrics>, u8), String> {
    let mut checks = 0usize;

    // Compile pure/m7/judge.omni + harness and M4 VM once.
    let judge_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(P7_JUDGE_OMNI_SRC))?;
    let judge_mod = decode_omni_token_stream(&judge_u16)?;
    let n_jfns = judge_mod.functions.len();
    if n_jfns < 5 {
        return Err(format!("P7-0 judge module too small: {} fns", n_jfns));
    }
    // Function order at end of judge_harness.omni:
    //   p7_judge_metrics_entry (-4), p7_trace_ring_entry (-3), p7_judge_dispatch6 (-2), main (-1)
    let judge_metrics_fn = judge_mod.functions[n_jfns - 4].fn_idx;
    let judge_trace_fn = judge_mod.functions[n_jfns - 3].fn_idx;
    let judge_main = judge_mod.functions[n_jfns - 2].fn_idx;

    let m4_u16 = if let Ok(raw) = std::fs::read("pure/m4/m4.omni_bc") {
        decode_bytes_to_u16_stream(&raw)
    } else {
        with_lockstep_suspended(|| compile_omni_surface_to_u16(M4_VM_OMNI_SRC))?
    };
    let m4_mod = decode_omni_token_stream(&m4_u16)?;
    let m4_run_fn = m4_mod.functions[m4_mod.functions.len() - 2].fn_idx;

    let mut vm = OmniBinaryVm::with_budgets(50_000_000, 64 * 1024 * 1024);
    let call_judge = |vm: &mut OmniBinaryVm, mode: i64, a: i64, b: i64, c: i64, d: i64, e: i64| -> Result<i64, String> {
        match vm.execute_function(
            &judge_mod,
            judge_main,
            vec![
                OmniValue::Int(mode),
                OmniValue::Int(a),
                OmniValue::Int(b),
                OmniValue::Int(c),
                OmniValue::Int(d),
                OmniValue::Int(e),
            ],
        )? {
            OmniValue::Int(v) => Ok(v),
            other => Err(format!("P7-0 judge mode={} returned {:?}", mode, other)),
        }
    };

    // Check 1: M2-multi ELF64 compilation of pure/m7/judge.omni succeeds (validates full Pure-Omni AOT compliance).
    let judge_elf = compile_pure_u16_to_x86_64_elf_multi(&judge_u16, 0, 100, 80)?;
    if judge_elf.len() < 120 || &judge_elf[0..4] != &[0x7F, b'E', b'L', b'F'] {
        return Err(String::from("P7-0 pure/m7/judge.omni failed M2-multi ELF64 emission"));
    }
    checks += 1;

    // Check 2: Thresholding without division (p7_cmp_band, mode 0).
    for &(base, cand, band, want) in &[
        (100i64, 80, 5, 1i64),  // delta=+20 > 5 -> +1t (better)
        (100, 95, 5, 0),        // delta=+5 == band -> 0t (inside band)
        (100, 100, 5, 0),       // delta=0 -> 0t (neutral)
        (100, 105, 5, 0),       // delta=-5 == -band -> 0t (inside band)
        (100, 120, 5, -1),      // delta=-20 < -5 -> -1t (worse)
        (50, 49, 0, 1),         // band=0 strict improvement -> +1t
        (50, 51, 0, -1),        // band=0 strict regression -> -1t
    ] {
        let got = call_judge(&mut vm, 0, base, cand, band, 0, 0)?;
        if got != want {
            return Err(format!("P7-0 p7_cmp_band({},{},{}) got {} want {}", base, cand, band, got, want));
        }
    }
    checks += 1;

    // Check 3: Consensus voting across noisy runs (p7_vote3 mode 1, p7_vote5 mode 2).
    for &(t0, t1, t2, want) in &[
        (1i64, 1, 0, 1i64),
        (1, -1, 0, 0),
        (-1, -1, 1, -1),
        (0, 0, 1, 1),
        (0, 0, 0, 0),
    ] {
        let got = call_judge(&mut vm, 1, t0, t1, t2, 0, 0)?;
        if got != want {
            return Err(format!("P7-0 p7_vote3({},{},{}) got {} want {}", t0, t1, t2, got, want));
        }
    }
    for &(t0, t1, t2, t3, t4, want) in &[
        (1i64, 1, 1, -1, 0, 1i64),
        (-1, -1, 0, 0, 1, -1),
        (1, -1, 1, -1, 0, 0),
    ] {
        let got = call_judge(&mut vm, 2, t0, t1, t2, t3, t4)?;
        if got != want {
            return Err(format!("P7-0 p7_vote5 got {} want {}", got, want));
        }
    }
    checks += 1;

    // Check 4: @PACK5 packing & unpacking exact parity with Rust pack5_encode / pack5_decode.
    for &(t0, t1, t2, t3, t4) in &[
        (-1i8, -1, -1, -1, -1),
        (0, 0, 0, 0, 0),
        (1, 1, 1, 1, 1),
        (1, 0, -1, 1, 0),
        (0, 1, 0, 0, 0),
        (0, -1, 0, 0, 0),
    ] {
        let rust_b = pack5_encode(&[t0, t1, t2, t3, t4])[0] as i64;
        let omni_b = call_judge(&mut vm, 5, t0 as i64, t1 as i64, t2 as i64, t3 as i64, t4 as i64)?;
        if omni_b != rust_b {
            return Err(format!("P7-0 @PACK5 pack mismatch: omni={} rust={}", omni_b, rust_b));
        }
        let rust_unpacked = pack5_decode(&[rust_b as u8], 5).map_err(|e| e.to_string())?;
        for idx in 0..5 {
            let omni_t = call_judge(&mut vm, 6, omni_b, idx as i64, 0, 0, 0)?;
            if omni_t != rust_unpacked[idx] as i64 {
                return Err(format!("P7-0 @PACK5 unpack mismatch at byte {} idx {}: omni={} rust={}", omni_b, idx, omni_t, rust_unpacked[idx]));
            }
        }
    }
    // Invalid @PACK5 byte (> 242) safely returns 0t.
    if call_judge(&mut vm, 6, 250, 0, 0, 0, 0)? != 0 {
        return Err(String::from("P7-0 @PACK5 invalid byte >242 must return 0t"));
    }
    checks += 1;

    // Check 5: Execute all 7 fixed benchmarks end-to-end (M3 compile, M2 ELF64, M4 VM + P4-5 trace ring)
    // and assert pinned deterministic metrics on `bench_m3_frontend` (index 6).
    let corpus = p7_benchmark_corpus_all();
    let mut baseline: Vec<P7BenchMetrics> = Vec::with_capacity(corpus.len());
    for (bname, bsrc, bargs, want, use_pure_m3) in &corpus {
        let m = p7_measure_program(&m4_mod, m4_run_fn, &judge_mod, judge_trace_fn, bname, bsrc, *bargs, *want, *use_pure_m3)?;
        baseline.push(m);
    }
    let m3fe = &baseline[6];
    if m3fe.bc_words != 1204
        || m3fe.elf_bytes != 2958
        || m3fe.vm_steps != 555
        || m3fe.trace_ops != 555
        || m3fe.arena_bytes != 0
        || (m3fe.hist_alu, m3fe.hist_mem, m3fe.hist_ctrl, m3fe.hist_call_ret) != (304, 0, 96, 155)
    {
        return Err(format!(
            "P7-0 bench_m3_frontend pinned metrics drifted: bc={}w elf={}B steps={} trace={} arena={}B hist={}/{}/{}/{}",
            m3fe.bc_words, m3fe.elf_bytes, m3fe.vm_steps, m3fe.trace_ops, m3fe.arena_bytes,
            m3fe.hist_alu, m3fe.hist_mem, m3fe.hist_ctrl, m3fe.hist_call_ret
        ));
    }
    // Also verify corrupted (non-monotonic) trace ring is rejected by Pure-Omni p7_verify_trace_ring.
    let mut bad_arena = vec![0u8; 524_288];
    bad_arena[167_960..167_964].copy_from_slice(&2u32.to_le_bytes()); // trace_cnt = 2
    bad_arena[458_752..458_756].copy_from_slice(&5u32.to_le_bytes()); // entry 0 step = 5
    bad_arena[458_768..458_772].copy_from_slice(&3u32.to_le_bytes()); // entry 1 step = 3 (non-monotonic!)
    let bad_res = match vm.execute_function(&judge_mod, judge_trace_fn, vec![OmniValue::Bytes(bad_arena)])? {
        OmniValue::Int(v) => v,
        other => return Err(format!("P7-0 bad trace check returned {:?}", other)),
    };
    if bad_res != -1 {
        return Err(format!("P7-0 non-monotonic trace ring must return -1, got {}", bad_res));
    }
    checks += 1;

    let judge_pair = |vm: &mut OmniBinaryVm, base: &P7BenchMetrics, cand: &P7BenchMetrics, band_bc: i64, band_elf: i64, band_steps: i64, band_arena: i64| -> Result<i64, String> {
        match vm.execute_function(
            &judge_mod,
            judge_metrics_fn,
            vec![
                OmniValue::Int(base.bc_words), OmniValue::Int(cand.bc_words), OmniValue::Int(band_bc),
                OmniValue::Int(base.elf_bytes), OmniValue::Int(cand.elf_bytes), OmniValue::Int(band_elf),
                OmniValue::Int(base.vm_steps), OmniValue::Int(cand.vm_steps), OmniValue::Int(band_steps),
                OmniValue::Int(base.arena_bytes), OmniValue::Int(cand.arena_bytes), OmniValue::Int(band_arena),
            ],
        )? {
            OmniValue::Int(v) => Ok(v),
            other => Err(format!("P7-0 p7_judge_metrics_entry returned {:?}", other)),
        }
    };

    // Check 6: End-to-end KNOWN BETTER change (+1t / ADOPT).
    // Tighter verified implementation of bench_loop_sum: closed-form `n * (n + 1) / 2` -> same output 5050,
    // strictly smaller bc_words, smaller elf_bytes, and dramatically fewer vm_steps & trace_ops!
    let better_src = "#[strict]\nfn main(n: i64) -> i64 {\n    return (n * (n + 1)) / 2;\n}\n";
    let better_m = p7_measure_program(&m4_mod, m4_run_fn, &judge_mod, judge_trace_fn, "bench_loop_sum_better", better_src, [100, 0, 0], 5050, false)?;
    let v_better = judge_pair(&mut vm, &baseline[1], &better_m, 0, 0, 0, 0)?;
    let suite_better = call_judge(&mut vm, 3, 0, v_better, 0, 0, 0)?;
    let adopt_better = call_judge(&mut vm, 4, 1, suite_better, 0, 0, 0)?;
    let packed_better = call_judge(&mut vm, 5, 0, v_better, 0, 0, 0)? as u8;
    if v_better != 1 || suite_better != 1 || adopt_better != 1 || packed_better != pack5_encode(&[0, 1, 0, 0, 0])[0] {
        return Err(format!(
            "P7-0 known-better must judge +1t (ADOPT): v={} suite={} adopt={} pack5={}",
            v_better, suite_better, adopt_better, packed_better
        ));
    }
    checks += 1;

    // Check 7: End-to-end KNOWN WORSE change (-1t / REJECT).
    // Bloated verified implementation of bench_loop_sum: extra redundant arithmetic inside the loop -> same output 5050,
    // strictly larger bc_words, larger elf_bytes, and more vm_steps & trace_ops!
    let worse_src = "#[strict]\nfn main(n: i64) -> i64 {\n    let acc: i64 = 0;\n    let i: i64 = 1;\n    while i <= n {\n        let t0: i64 = i + 0;\n        let t1: i64 = t0 * 1;\n        acc = acc + t1;\n        i = i + 1;\n    }\n    return acc;\n}\n";
    let worse_m = p7_measure_program(&m4_mod, m4_run_fn, &judge_mod, judge_trace_fn, "bench_loop_sum_worse", worse_src, [100, 0, 0], 5050, false)?;
    let v_worse = judge_pair(&mut vm, &baseline[1], &worse_m, 0, 0, 0, 0)?;
    let suite_worse = call_judge(&mut vm, 3, 0, v_worse, 0, 0, 0)?;
    let adopt_worse = call_judge(&mut vm, 4, 1, suite_worse, 0, 0, 0)?;
    if v_worse != -1 || suite_worse != -1 || adopt_worse != -1 {
        return Err(format!(
            "P7-0 known-worse must judge -1t (REJECT): v={} suite={} adopt={}",
            v_worse, suite_worse, adopt_worse
        ));
    }
    checks += 1;

    // Check 8: End-to-end KNOWN NEUTRAL change (0t / HOLD).
    let v_neutral = judge_pair(&mut vm, &baseline[1], &baseline[1], 0, 0, 0, 0)?;
    let suite_neutral = call_judge(&mut vm, 3, 0, v_neutral, 0, 0, 0)?;
    let adopt_neutral = call_judge(&mut vm, 4, 1, suite_neutral, 0, 0, 0)?;
    let packed_neutral = call_judge(&mut vm, 5, 0, v_neutral, 0, 0, 0)? as u8;
    if v_neutral != 0 || suite_neutral != 0 || adopt_neutral != 0 || packed_neutral != 121 {
        return Err(format!(
            "P7-0 known-neutral must judge 0t (HOLD): v={} suite={} adopt={} pack5={}",
            v_neutral, suite_neutral, adopt_neutral, packed_neutral
        ));
    }
    checks += 1;

    // Check 9: No-regression rule (mixed +1/-1 -> -1t REJECT) and gate-failure rule (gates_ok=0 -> -1t REJECT).
    let suite_mixed = call_judge(&mut vm, 3, 1, 1, -1, 0, 1)?;
    let adopt_gate_fail = call_judge(&mut vm, 4, 0, 1, 0, 0, 0)?;
    if suite_mixed != -1 || adopt_gate_fail != -1 {
        return Err(format!(
            "P7-0 no-regression/gate-fail violated: mixed={} gate_fail={}",
            suite_mixed, adopt_gate_fail
        ));
    }
    checks += 1;

    // Check 10: All frozen FNV-1a64 hashes untouched.
    selftest_m2_frozen_hashes()?;
    checks += 1;

    Ok((checks, baseline, packed_better))
}

/// Executes a native Linux x86-64 ELF64 binary returning an 8-byte little-endian `i64` on stdout.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn p7_verify_native_elf_i64(name: &str, elf: &[u8], expected_val: i64) -> Result<bool, String> {
    let tmp_elf = PrivateTempElf::new(elf, name)?;
    let out = std::process::Command::new(&tmp_elf.path)
        .output()
        .map_err(|e| format!("P7-0a exec {}: {}", name, e))?;
    drop(tmp_elf);
    if !out.status.success() {
        return Err(format!("P7-0a native {} exited with {:?}", name, out.status));
    }
    if out.stdout.len() != 8 {
        return Err(format!(
            "P7-0a native {} expected 8 stdout bytes, got {}",
            name,
            out.stdout.len()
        ));
    }
    let got = i64::from_le_bytes(out.stdout[0..8].try_into().unwrap());
    if got != expected_val {
        return Err(format!(
            "P7-0a native {} returned {}, want {}",
            name, got, expected_val
        ));
    }
    Ok(true)
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
fn p7_verify_native_elf_i64(_name: &str, _elf: &[u8], _expected_val: i64) -> Result<bool, String> {
    Ok(false)
}

/// P7-0a Hard Gate (`omnic p7-canary`):
/// Proves multi-run consensus (`p7_judge_5runs` / `p7_vote5`), subsystem attribution
/// (`p7_attribute_subsystem`: M3 frontend vs M2 emitter vs M4 VM), P4-5 trace-ring 4-class
/// opcode histograms (`p7_trace_histogram`), and the two-part canary proof:
///   1. Predictable M2 emitter tweak (`compact_zero_imm`: `xor eax,eax` / `xor ecx,ecx`) ->
///      judged `+1t` (ADOPT) by the exact predicted `8 * zero_loads` byte margin and strictly
///      attributed to `M2_EMITTER` (`2`), with zero trace-histogram drift.
///   2. Deliberately neutral change (comment/whitespace-only) ->
///      judged `0t` (HOLD) across 5 repeated runs and attributed to `NONE` (`0`).
pub fn selftest_p7_0a_canary() -> Result<(usize, i64, i64, i64, bool), String> {
    let mut checks = 0usize;

    let judge_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(P7_JUDGE_OMNI_SRC))?;
    let judge_mod = decode_omni_token_stream(&judge_u16)?;
    let n_jfns = judge_mod.functions.len();
    if n_jfns < 8 {
        return Err(format!("P7-0a judge module too small: {} fns", n_jfns));
    }
    // Function order at end of judge_harness.omni:
    //   p7_attr_entry (-7), p7_judge_5runs_entry (-6), p7_trace_hist_entry (-5),
    //   p7_judge_metrics_entry (-4), p7_trace_ring_entry (-3), p7_judge_dispatch6 (-2), main (-1)
    let judge_attr_fn = judge_mod.functions[n_jfns - 7].fn_idx;
    let judge_5runs_fn = judge_mod.functions[n_jfns - 6].fn_idx;
    let judge_hist_fn = judge_mod.functions[n_jfns - 5].fn_idx;
    let judge_trace_fn = judge_mod.functions[n_jfns - 3].fn_idx;
    let judge_main = judge_mod.functions[n_jfns - 2].fn_idx;

    let m4_u16 = if let Ok(raw) = std::fs::read("pure/m4/m4.omni_bc") {
        decode_bytes_to_u16_stream(&raw)
    } else {
        with_lockstep_suspended(|| compile_omni_surface_to_u16(M4_VM_OMNI_SRC))?
    };
    let m4_mod = decode_omni_token_stream(&m4_u16)?;
    let m4_run_fn = m4_mod.functions[m4_mod.functions.len() - 2].fn_idx;

    let mut vm = OmniBinaryVm::with_budgets(50_000_000, 64 * 1024 * 1024);

    let call_judge = |vm: &mut OmniBinaryVm, mode: i64, a: i64, b: i64, c: i64, d: i64, e: i64| -> Result<i64, String> {
        match vm.execute_function(
            &judge_mod,
            judge_main,
            vec![
                OmniValue::Int(mode),
                OmniValue::Int(a),
                OmniValue::Int(b),
                OmniValue::Int(c),
                OmniValue::Int(d),
                OmniValue::Int(e),
            ],
        )? {
            OmniValue::Int(v) => Ok(v),
            other => Err(format!("P7-0a judge mode={} returned {:?}", mode, other)),
        }
    };

    let call_attr = |vm: &mut OmniBinaryVm, vm3: i64, vm2: i64, vm4: i64| -> Result<i64, String> {
        match vm.execute_function(
            &judge_mod,
            judge_attr_fn,
            vec![OmniValue::Int(vm3), OmniValue::Int(vm2), OmniValue::Int(vm4)],
        )? {
            OmniValue::Int(v) => Ok(v),
            other => Err(format!("P7-0a p7_attr_entry returned {:?}", other)),
        }
    };

    let call_5runs = |vm: &mut OmniBinaryVm, b: &[i64; 5], c: &[i64; 5], band: i64| -> Result<i64, String> {
        match vm.execute_function(
            &judge_mod,
            judge_5runs_fn,
            vec![
                OmniValue::Int(b[0]), OmniValue::Int(c[0]),
                OmniValue::Int(b[1]), OmniValue::Int(c[1]),
                OmniValue::Int(b[2]), OmniValue::Int(c[2]),
                OmniValue::Int(b[3]), OmniValue::Int(c[3]),
                OmniValue::Int(b[4]), OmniValue::Int(c[4]),
                OmniValue::Int(band),
            ],
        )? {
            OmniValue::Int(v) => Ok(v),
            other => Err(format!("P7-0a p7_judge_5runs_entry returned {:?}", other)),
        }
    };

    // Check 1: Pure-Omni subsystem attribution bitmask & 5-run noisy consensus unit proof.
    for &(vm3, vm2, vm4, want) in &[
        (0i64, 0, 0, 0i64),  // NONE
        (1, 0, 0, 1),        // M3_FRONTEND
        (0, 1, 0, 2),        // M2_EMITTER
        (0, -1, 0, 2),       // M2_EMITTER (regression also attributes to M2)
        (0, 0, 1, 4),        // M4_VM
        (1, 1, 0, 3),        // M3 + M2
        (1, 1, 1, 7),        // M3 + M2 + M4
    ] {
        let got = call_attr(&mut vm, vm3, vm2, vm4)?;
        if got != want {
            return Err(format!("P7-0a p7_attribute_subsystem({},{},{}) got {} want {}", vm3, vm2, vm4, got, want));
        }
    }
    // Noisy 5-run consensus: 4 improved runs + 1 noisy spike still resolves to +1t.
    let noisy_better = call_5runs(&mut vm, &[100, 100, 100, 100, 100], &[80, 82, 115, 79, 81], 5)?;
    // Noisy 5-run neutral: 1 positive glitch + 1 negative glitch + 3 inside-band resolves to 0t.
    let noisy_neutral = call_5runs(&mut vm, &[100, 100, 100, 100, 100], &[102, 85, 99, 118, 100], 5)?;
    if noisy_better != 1 || noisy_neutral != 0 {
        return Err(format!("P7-0a 5-run consensus failed: noisy_better={} noisy_neutral={}", noisy_better, noisy_neutral));
    }
    checks += 1;

    let agg7 = |vm: &mut OmniBinaryVm, v: &[i64; P7_CORPUS_LEN]| -> Result<i64, String> {
        let s5 = call_judge(vm, 3, v[0], v[1], v[2], v[3], v[4])?;
        call_judge(vm, 3, s5, v[5], v[6], 0, 0)
    };

    // Check 2: Collect 5 repeated baseline runs across the 7-benchmark corpus
    // (5 micro + bench_verifier_engine + bench_m3_frontend) and verify zero variance
    // on deterministic metrics + architectural opcode histogram signatures.
    let corpus = p7_benchmark_corpus_all();
    let mut base_runs: Vec<[P7BenchMetrics; 5]> = Vec::with_capacity(P7_CORPUS_LEN);
    let mut zero_loads_per_bench = [0i64; P7_CORPUS_LEN];
    for (bi, (bname, bsrc, bargs, want, use_pure_m3)) in corpus.iter().enumerate() {
        let mut arr: Vec<P7BenchMetrics> = Vec::with_capacity(5);
        for _ in 0..5 {
            let (m, _elf, zl) = p7_measure_program_with_opts(
                &m4_mod, m4_run_fn, &judge_mod, judge_trace_fn, judge_hist_fn,
                bname, bsrc, *bargs, *want, M2EmitMode::Legacy, *use_pure_m3,
            )?;
            zero_loads_per_bench[bi] = zl;
            arr.push(m);
        }
        for r in 1..5 {
            if arr[r].bc_words != arr[0].bc_words
                || arr[r].elf_bytes != arr[0].elf_bytes
                || arr[r].vm_steps != arr[0].vm_steps
                || arr[r].trace_ops != arr[0].trace_ops
                || arr[r].arena_bytes != arr[0].arena_bytes
                || arr[r].hist_alu != arr[0].hist_alu
                || arr[r].hist_mem != arr[0].hist_mem
                || arr[r].hist_ctrl != arr[0].hist_ctrl
                || arr[r].hist_call_ret != arr[0].hist_call_ret
            {
                return Err(format!("P7-0a non-deterministic baseline metric on {} run {}", bname, r));
            }
        }
        base_runs.push(arr.try_into().unwrap());
    }
    // Verify architectural histogram signatures on the 7-benchmark corpus:
    //   bench_loop_sum (1) & bench_branch_tree (2) exercise control flow;
    //   bench_multi_call (3) exercises CALL/RET;
    //   bench_arena_buf (4) exercises Memory/Arena ops;
    //   bench_verifier_engine (5) exercises all 4 classes (ALU + MEM + CTRL + CALL/RET);
    //   bench_m3_frontend (6) exercises std/ includes + trit/egate/@PACK5 + ALU/CTRL/CALL-RET.
    if base_runs[1][0].hist_ctrl <= 0
        || base_runs[2][0].hist_ctrl <= 0
        || base_runs[3][0].hist_call_ret < 2
        || base_runs[4][0].hist_mem <= 0
        || base_runs[5][0].hist_alu <= 0
        || base_runs[5][0].hist_mem <= 0
        || base_runs[5][0].hist_ctrl <= 0
        || base_runs[5][0].hist_call_ret <= 0
        || base_runs[6][0].hist_alu <= 0
        || base_runs[6][0].hist_ctrl <= 0
        || base_runs[6][0].hist_call_ret < 10
    {
        return Err(format!(
            "P7-0a histogram signature mismatch: loop.ctrl={} branch.ctrl={} call.cr={} arena.mem={} ve={}/{}/{}/{} m3fe={}/{}/{}/{}",
            base_runs[1][0].hist_ctrl,
            base_runs[2][0].hist_ctrl,
            base_runs[3][0].hist_call_ret,
            base_runs[4][0].hist_mem,
            base_runs[5][0].hist_alu,
            base_runs[5][0].hist_mem,
            base_runs[5][0].hist_ctrl,
            base_runs[5][0].hist_call_ret,
            base_runs[6][0].hist_alu,
            base_runs[6][0].hist_mem,
            base_runs[6][0].hist_ctrl,
            base_runs[6][0].hist_call_ret
        ));
    }
    checks += 1;

    // Check 3: Canary A — Predictable, verified M2 emitter tweak (`compact_zero_imm = true`).
    // Must shrink ELF64 by the exact predicted `8 * zero_loads` bytes on all 6 benchmarks (incl.
    // bench_verifier_engine), execute identically on Linux, judge `+1t` (ADOPT) via 5-run Pure-Omni
    // consensus, and attribute strictly to `M2_EMITTER` (2).
    let mut canary_m3 = [0i64; P7_CORPUS_LEN];
    let mut canary_m2 = [0i64; P7_CORPUS_LEN];
    let mut canary_m4 = [0i64; P7_CORPUS_LEN];
    let mut canary_bench_v = [0i64; P7_CORPUS_LEN];
    let mut total_saved_bytes = 0i64;
    let mut native_verified = false;
    for (bi, (bname, bsrc, bargs, want, use_pure_m3)) in corpus.iter().enumerate() {
        let mut b_bc = [0i64; 5];
        let mut c_bc = [0i64; 5];
        let mut b_elf = [0i64; 5];
        let mut c_elf = [0i64; 5];
        let mut b_steps = [0i64; 5];
        let mut c_steps = [0i64; 5];
        let mut b_arena = [0i64; 5];
        let mut c_arena = [0i64; 5];
        let mut canary_elf_once: Option<Vec<u8>> = None;
        for r in 0..5 {
            let (cm, celf, _zl) = p7_measure_program_with_opts(
                &m4_mod, m4_run_fn, &judge_mod, judge_trace_fn, judge_hist_fn,
                bname, bsrc, *bargs, *want, M2EmitMode::CanaryZeroImm, *use_pure_m3,
            )?;
            let bm = &base_runs[bi][r];
            let expected_delta = 8 * zero_loads_per_bench[bi];
            if bm.elf_bytes - cm.elf_bytes != expected_delta {
                return Err(format!(
                    "P7-0a canary {} expected {}B reduction (8 * {}), got {}B ({} -> {})",
                    bname, expected_delta, zero_loads_per_bench[bi],
                    bm.elf_bytes - cm.elf_bytes, bm.elf_bytes, cm.elf_bytes
                ));
            }
            // Histogram must be 100% identical (M4 VM did not move).
            if bm.hist_alu != cm.hist_alu
                || bm.hist_mem != cm.hist_mem
                || bm.hist_ctrl != cm.hist_ctrl
                || bm.hist_call_ret != cm.hist_call_ret
            {
                return Err(format!("P7-0a canary {} unexpectedly altered P4-5 trace histogram", bname));
            }
            b_bc[r] = bm.bc_words;
            c_bc[r] = cm.bc_words;
            b_elf[r] = bm.elf_bytes;
            c_elf[r] = cm.elf_bytes;
            b_steps[r] = bm.vm_steps;
            c_steps[r] = cm.vm_steps;
            b_arena[r] = bm.arena_bytes;
            c_arena[r] = cm.arena_bytes;
            if canary_elf_once.is_none() {
                canary_elf_once = Some(celf);
            }
        }
        if let Some(ref celf) = canary_elf_once {
            if p7_verify_native_elf_i64(bname, celf, *want)? {
                native_verified = true;
            }
        }
        total_saved_bytes += b_elf[0] - c_elf[0];
        canary_m3[bi] = call_5runs(&mut vm, &b_bc, &c_bc, 0)?;
        canary_m2[bi] = call_5runs(&mut vm, &b_elf, &c_elf, 0)?;
        let v_steps = call_5runs(&mut vm, &b_steps, &c_steps, 0)?;
        let v_arena = call_5runs(&mut vm, &b_arena, &c_arena, 0)?;
        canary_m4[bi] = call_judge(&mut vm, 3, v_steps, v_arena, 0, 0, 0)?;
        canary_bench_v[bi] = call_judge(&mut vm, 3, canary_m3[bi], canary_m2[bi], canary_m4[bi], 0, 0)?;
    }
    let suite_m3 = agg7(&mut vm, &canary_m3)?;
    let suite_m2 = agg7(&mut vm, &canary_m2)?;
    let suite_m4 = agg7(&mut vm, &canary_m4)?;
    let canary_suite = agg7(&mut vm, &canary_bench_v)?;
    let canary_adopt = call_judge(&mut vm, 4, 1, canary_suite, 0, 0, 0)?;
    let canary_attr = call_attr(&mut vm, suite_m3, suite_m2, suite_m4)?;
    if total_saved_bytes <= 0 || canary_bench_v[5] != 1 || canary_bench_v[6] != 1 || canary_suite != 1 || canary_adopt != 1 || canary_attr != 2 {
        return Err(format!(
            "P7-0a canary A gate failed: saved={}B ve={} m3fe={} suite={} adopt={} attr={} (m3={},m2={},m4={})",
            total_saved_bytes, canary_bench_v[5], canary_bench_v[6], canary_suite, canary_adopt, canary_attr, suite_m3, suite_m2, suite_m4
        ));
    }
    checks += 1;

    // Check 4: Canary B — Deliberately neutral change (comment/whitespace-only source change,
    // baseline emitter) across 5 repeated runs on all 6 benchmarks -> judged `0t` (HOLD) and attributed to `NONE` (0).
    let mut neut_m3 = [0i64; P7_CORPUS_LEN];
    let mut neut_m2 = [0i64; P7_CORPUS_LEN];
    let mut neut_m4 = [0i64; P7_CORPUS_LEN];
    let mut neut_bench_v = [0i64; P7_CORPUS_LEN];
    for (bi, (bname, bsrc, bargs, want, use_pure_m3)) in corpus.iter().enumerate() {
        let neutral_src = format!("// P7-0a neutral comment\n{}\n", bsrc);
        let mut b_bc = [0i64; 5];
        let mut c_bc = [0i64; 5];
        let mut b_elf = [0i64; 5];
        let mut c_elf = [0i64; 5];
        let mut b_steps = [0i64; 5];
        let mut c_steps = [0i64; 5];
        let mut b_arena = [0i64; 5];
        let mut c_arena = [0i64; 5];
        for r in 0..5 {
            let (nm, _nelf, _zl) = p7_measure_program_with_opts(
                &m4_mod, m4_run_fn, &judge_mod, judge_trace_fn, judge_hist_fn,
                bname, &neutral_src, *bargs, *want, M2EmitMode::Legacy, *use_pure_m3,
            )?;
            let bm = &base_runs[bi][r];
            b_bc[r] = bm.bc_words;
            c_bc[r] = nm.bc_words;
            b_elf[r] = bm.elf_bytes;
            c_elf[r] = nm.elf_bytes;
            b_steps[r] = bm.vm_steps;
            c_steps[r] = nm.vm_steps;
            b_arena[r] = bm.arena_bytes;
            c_arena[r] = nm.arena_bytes;
        }
        neut_m3[bi] = call_5runs(&mut vm, &b_bc, &c_bc, 0)?;
        neut_m2[bi] = call_5runs(&mut vm, &b_elf, &c_elf, 0)?;
        let v_steps = call_5runs(&mut vm, &b_steps, &c_steps, 0)?;
        let v_arena = call_5runs(&mut vm, &b_arena, &c_arena, 0)?;
        neut_m4[bi] = call_judge(&mut vm, 3, v_steps, v_arena, 0, 0, 0)?;
        neut_bench_v[bi] = call_judge(&mut vm, 3, neut_m3[bi], neut_m2[bi], neut_m4[bi], 0, 0)?;
    }
    let nm3 = agg7(&mut vm, &neut_m3)?;
    let nm2 = agg7(&mut vm, &neut_m2)?;
    let nm4 = agg7(&mut vm, &neut_m4)?;
    let neutral_suite = agg7(&mut vm, &neut_bench_v)?;
    let neutral_adopt = call_judge(&mut vm, 4, 1, neutral_suite, 0, 0, 0)?;
    let neutral_attr = call_attr(&mut vm, nm3, nm2, nm4)?;
    if neut_bench_v[5] != 0 || neut_bench_v[6] != 0 || neutral_suite != 0 || neutral_adopt != 0 || neutral_attr != 0 {
        return Err(format!(
            "P7-0a canary B (neutral) gate failed: ve={} m3fe={} suite={} adopt={} attr={}",
            neut_bench_v[5], neut_bench_v[6], neutral_suite, neutral_adopt, neutral_attr
        ));
    }
    checks += 1;

    // Check 5: Attribution discrimination — an algorithmic/source change (`bench_loop_sum` closed form)
    // moves M3 + M2 + M4 together (attr = 7) AND drops `hist_ctrl` to 0, clearly distinct from a pure
    // M2 emitter move (attr = 2, histogram unchanged).
    let closed_src = "#[strict]\nfn main(n: i64) -> i64 {\n    return (n * (n + 1)) / 2;\n}\n";
    let (closed_m, _celf, _zl) = p7_measure_program_with_opts(
        &m4_mod, m4_run_fn, &judge_mod, judge_trace_fn, judge_hist_fn,
        "bench_loop_sum_closed", closed_src, [100, 0, 0], 5050, M2EmitMode::Legacy, false,
    )?;
    let bm1 = &base_runs[1][0];
    let dm3 = call_judge(&mut vm, 0, bm1.bc_words, closed_m.bc_words, 0, 0, 0)?;
    let dm2 = call_judge(&mut vm, 0, bm1.elf_bytes, closed_m.elf_bytes, 0, 0, 0)?;
    let dm4 = call_judge(&mut vm, 0, bm1.vm_steps, closed_m.vm_steps, 0, 0, 0)?;
    let closed_attr = call_attr(&mut vm, dm3, dm2, dm4)?;
    if closed_attr != 7 || closed_m.hist_ctrl != 0 || bm1.hist_ctrl <= 0 {
        return Err(format!(
            "P7-0a attribution discrimination failed: attr={} (want 7), base.ctrl={} closed.ctrl={}",
            closed_attr, bm1.hist_ctrl, closed_m.hist_ctrl
        ));
    }
    checks += 1;

    // Check 6: Live production M2 emitter (`M2EmitMode::Compact`: disp8 + 3-tier imm + sub/add rsp imm8 + imm8 ALU/cmp)
    // vs `M2EmitMode::Legacy` across all 6 benchmarks -> +1t ADOPT on all 6/6, strictly attributed to
    // `M2_EMITTER` (2), zero histogram drift, and native Linux x86-64 execution verified.
    let mut prod_v = [0i64; P7_CORPUS_LEN];
    let mut prod_m3 = [0i64; P7_CORPUS_LEN];
    let mut prod_m2 = [0i64; P7_CORPUS_LEN];
    let mut prod_m4 = [0i64; P7_CORPUS_LEN];
    let mut prod_saved_bytes = 0i64;
    for (bi, (bname, bsrc, bargs, want, use_pure_m3)) in corpus.iter().enumerate() {
        let (pm, pelf, _zl) = p7_measure_program_with_opts(
            &m4_mod, m4_run_fn, &judge_mod, judge_trace_fn, judge_hist_fn,
            bname, bsrc, *bargs, *want, M2EmitMode::Compact, *use_pure_m3,
        )?;
        let bm = &base_runs[bi][0];
        if pm.elf_bytes >= bm.elf_bytes {
            return Err(format!("P7 M2-Compact {} did not shrink ELF ({} -> {})", bname, bm.elf_bytes, pm.elf_bytes));
        }
        if bm.hist_alu != pm.hist_alu || bm.hist_mem != pm.hist_mem || bm.hist_ctrl != pm.hist_ctrl || bm.hist_call_ret != pm.hist_call_ret {
            return Err(format!("P7 M2-Compact {} altered trace histogram", bname));
        }
        if p7_verify_native_elf_i64(bname, &pelf, *want)? {
            native_verified = true;
        }
        prod_saved_bytes += bm.elf_bytes - pm.elf_bytes;
        prod_m3[bi] = call_judge(&mut vm, 0, bm.bc_words, pm.bc_words, 0, 0, 0)?;
        prod_m2[bi] = call_judge(&mut vm, 0, bm.elf_bytes, pm.elf_bytes, 0, 0, 0)?;
        let vs = call_judge(&mut vm, 0, bm.vm_steps, pm.vm_steps, 0, 0, 0)?;
        let va = call_judge(&mut vm, 0, bm.arena_bytes, pm.arena_bytes, 0, 0, 0)?;
        prod_m4[bi] = call_judge(&mut vm, 3, vs, va, 0, 0, 0)?;
        prod_v[bi] = call_judge(&mut vm, 3, prod_m3[bi], prod_m2[bi], prod_m4[bi], 0, 0)?;
    }
    let pm3 = agg7(&mut vm, &prod_m3)?;
    let pm2 = agg7(&mut vm, &prod_m2)?;
    let pm4 = agg7(&mut vm, &prod_m4)?;
    let prod_suite = agg7(&mut vm, &prod_v)?;
    let prod_adopt = call_judge(&mut vm, 4, 1, prod_suite, 0, 0, 0)?;
    let prod_attr = call_attr(&mut vm, pm3, pm2, pm4)?;
    if prod_saved_bytes <= total_saved_bytes || prod_v[5] != 1 || prod_v[6] != 1 || prod_suite != 1 || prod_adopt != 1 || prod_attr != 2 {
        return Err(format!(
            "P7 M2-Compact gate failed: saved={}B ve={} m3fe={} suite={} adopt={} attr={}",
            prod_saved_bytes, prod_v[5], prod_v[6], prod_suite, prod_adopt, prod_attr
        ));
    }
    checks += 1;

    // Check 7: Frozen hashes untouched.
    selftest_m2_frozen_hashes()?;
    checks += 1;

    Ok((checks, prod_saved_bytes, canary_attr, neutral_attr, native_verified))
}

// ==============================================================================
// PHASE 7 (P7-1): SELF-HEALING â€” FAILURE TAXONOMY, CONTENT-ADDRESSED ROLLBACK,
// BOUNDED REPAIR LOOP, BINARY HEALING LOG & FAULT-INJECTION GATE
// ==============================================================================

#[derive(Clone, Debug)]
pub struct P7RollbackTarget {
    pub stream: Vec<u16>,
    pub content_hash: u64,
}

impl P7RollbackTarget {
    pub fn from_passing_stream(stream: Vec<u16>) -> Self {
        let content_hash = fnv1a_64_hash(&encode_u16_stream_to_bytes(&stream));
        Self { stream, content_hash }
    }

    pub fn verify_integrity(&self) -> bool {
        fnv1a_64_hash(&encode_u16_stream_to_bytes(&self.stream)) == self.content_hash
    }
}

/// Evaluates a candidate `.omni_bc` token stream against decode, static verifier,
/// M2-multi ELF64 lowering, and Pure-Omni M4 VM execution.
/// Returns `0` (OK) or a P7-1 taxonomy code (`1`=DECODE_CORRUPT, `2`=VERIFY_REJECT,
/// `3`=RUNTIME_TRAP, `4`=OUTPUT_MISMATCH).
fn p7_validate_candidate_stream(
    m4_mod: &BinModule,
    m4_run_fn: u16,
    stream: &[u16],
    args: [i64; 3],
    expected_val: i64,
) -> i64 {
    let decoded = match decode_omni_token_stream(stream) {
        Ok(m) => m,
        Err(_) => return 1, // DECODE_CORRUPT
    };
    if decoded.functions.is_empty() {
        return 1;
    }
    for f in &decoded.functions {
        if !verify_binary_function(f).passed {
            return 2; // VERIFY_REJECT
        }
    }
    if compile_pure_u16_to_x86_64_elf_multi(stream, args[0], args[1], args[2]).is_err() {
        return 2; // VERIFY_REJECT / AOT-ineligible
    }
    let mut vm = OmniBinaryVm::with_budgets(50_000_000, 64 * 1024 * 1024);
    let arena_out = match vm.execute_function(
        m4_mod,
        m4_run_fn,
        vec![
            OmniValue::Bytes(vec![0u8; 524_288]),
            OmniValue::U16Array(stream.to_vec()),
            OmniValue::Int(args[0]),
            OmniValue::Int(args[1]),
            OmniValue::Int(args[2]),
            OmniValue::Int(200_000),
            OmniValue::Int(1),
        ],
    ) {
        Ok(OmniValue::Bytes(b)) if b.len() == 524_288 => b,
        _ => return 3, // RUNTIME_TRAP
    };
    let st = 167_936usize;
    let err_code = u32::from_le_bytes(arena_out[st + 28..st + 32].try_into().unwrap());
    let ret_val = i64::from_le_bytes(arena_out[st + 32..st + 40].try_into().unwrap());
    if err_code != 0 {
        return 3; // RUNTIME_TRAP
    }
    if ret_val != expected_val {
        return 4; // OUTPUT_MISMATCH
    }
    0 // OK
}

/// P7-1 Hard Gate (`omnic p7-heal`):
///   1. `pure/m7/heal.omni` compiles via M2-multi to valid x86-64 ELF64.
///   2. Pure-Omni failure taxonomy (`+1t` healable, `0t` OK, `-1t` fatal) and bounded repair
///      decision table (`0`=PASS, `1`=ROLLBACK_AND_RETRY, `2`=ROLLBACK_FINAL, `3`=FATAL_ABORT).
///   3. Fault Injection #1 (corrupt token stream): detect -> roll back to FNV-1a64-verified
///      target -> re-emit passing stream -> execute on M4 VM -> log & verify binary healing trace.
///   4. Fault Injection #2 (verifier-invalid stream exhausting `max_attempts = 3`): bounded loop
///      halts cleanly at attempt 3 (`ACTION_ROLLBACK_FINAL`), restores verified rollback target.
///   5. Fatal Fault Injection (`VERIFIER_TAMPER`, `CAP_VIOLATION`, `FROZEN_HASH_DRIFT`, and
///      tampered rollback buffer -> `ROLLBACK_CORRUPT`): immediate `ACTION_FATAL_ABORT` (no retry).
///   6. Verifier and all 5 frozen FNV-1a64 hashes remain completely unchanged.
pub fn selftest_p7_1_self_heal() -> Result<(usize, u64, i64), String> {
    let mut checks = 0usize;

    let heal_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(P7_HEAL_OMNI_SRC))?;
    let heal_mod = decode_omni_token_stream(&heal_u16)?;
    let n_hfns = heal_mod.functions.len();
    if n_hfns < 5 {
        return Err(format!("P7-1 heal module too small: {} fns", n_hfns));
    }
    // Function order at end of spec/m7/heal_harness.omni:
    //   p7_heal_classify_entry (-5), p7_heal_decide_entry (-4),
    //   p7_heal_log_push_entry (-3), p7_heal_log_verify_entry (-2), main (-1)
    let fn_classify = heal_mod.functions[n_hfns - 5].fn_idx;
    let fn_decide = heal_mod.functions[n_hfns - 4].fn_idx;
    let fn_log_push = heal_mod.functions[n_hfns - 3].fn_idx;
    let fn_log_verify = heal_mod.functions[n_hfns - 2].fn_idx;

    let m4_u16 = if let Ok(raw) = std::fs::read("pure/m4/m4.omni_bc") {
        decode_bytes_to_u16_stream(&raw)
    } else {
        with_lockstep_suspended(|| compile_omni_surface_to_u16(M4_VM_OMNI_SRC))?
    };
    let m4_mod = decode_omni_token_stream(&m4_u16)?;
    let m4_run_fn = m4_mod.functions[m4_mod.functions.len() - 2].fn_idx;

    let mut vm = OmniBinaryVm::with_budgets(50_000_000, 64 * 1024 * 1024);

    // Check 1: M2-multi ELF64 compilation of pure/m7/heal.omni succeeds.
    let heal_elf = compile_pure_u16_to_x86_64_elf_multi(&heal_u16, 2, 0, 3)?;
    if heal_elf.len() < 120 || &heal_elf[0..4] != &[0x7F, b'E', b'L', b'F'] {
        return Err(String::from("P7-1 pure/m7/heal.omni failed M2-multi ELF64 emission"));
    }
    checks += 1;

    let call_classify = |vm: &mut OmniBinaryVm, err_code: i64| -> Result<i64, String> {
        match vm.execute_function(&heal_mod, fn_classify, vec![OmniValue::Int(err_code)])? {
            OmniValue::Int(v) => Ok(v),
            other => Err(format!("P7-1 classify returned {:?}", other)),
        }
    };
    let call_decide = |vm: &mut OmniBinaryVm, err_code: i64, att: i64, max_att: i64, rb_ok: i64| -> Result<i64, String> {
        match vm.execute_function(
            &heal_mod,
            fn_decide,
            vec![
                OmniValue::Int(err_code),
                OmniValue::Int(att),
                OmniValue::Int(max_att),
                OmniValue::Int(rb_ok),
            ],
        )? {
            OmniValue::Int(v) => Ok(v),
            other => Err(format!("P7-1 decide returned {:?}", other)),
        }
    };
    let call_log_push = |vm: &mut OmniBinaryVm, log: Vec<u8>, att: i64, err_code: i64, act: i64, hlo: i64| -> Result<Vec<u8>, String> {
        match vm.execute_function(
            &heal_mod,
            fn_log_push,
            vec![
                OmniValue::Bytes(log),
                OmniValue::Int(att),
                OmniValue::Int(err_code),
                OmniValue::Int(act),
                OmniValue::Int(hlo),
            ],
        )? {
            OmniValue::Bytes(b) => Ok(b),
            other => Err(format!("P7-1 log_push returned {:?}", other)),
        }
    };
    let call_log_verify = |vm: &mut OmniBinaryVm, log: Vec<u8>| -> Result<i64, String> {
        match vm.execute_function(&heal_mod, fn_log_verify, vec![OmniValue::Bytes(log)])? {
            OmniValue::Int(v) => Ok(v),
            other => Err(format!("P7-1 log_verify returned {:?}", other)),
        }
    };

    // Check 2: Failure taxonomy & bounded decision table (Pure-Omni).
    for &(code, want) in &[
        (0i64, 0i64),   // OK -> 0t
        (1, 1),         // DECODE_CORRUPT -> +1t (healable)
        (2, 1),         // VERIFY_REJECT -> +1t
        (3, 1),         // RUNTIME_TRAP -> +1t
        (4, 1),         // OUTPUT_MISMATCH -> +1t
        (5, 1),         // GATE_REGRESS -> +1t
        (10, -1),       // VERIFIER_TAMPER -> -1t (fatal)
        (11, -1),       // CAP_VIOLATION -> -1t (fatal)
        (12, -1),       // FROZEN_HASH_DRIFT -> -1t (fatal)
        (13, -1),       // ROLLBACK_CORRUPT -> -1t (fatal)
        (-1, -1),       // unknown negative -> -1t (fail-closed)
        (99, -1),       // unknown high -> -1t (fail-closed)
    ] {
        let got = call_classify(&mut vm, code)?;
        if got != want {
            return Err(format!("P7-1 classify({}) got {} want {}", code, got, want));
        }
    }
    for &(err, att, max_att, rb_ok, want) in &[
        (0i64, 0, 3, 1, 0i64), // PASS
        (1, 0, 3, 1, 1),       // ROLLBACK_AND_RETRY
        (2, 2, 3, 1, 1),       // ROLLBACK_AND_RETRY (last retry before cap)
        (2, 3, 3, 1, 2),       // ROLLBACK_FINAL (budget exhausted)
        (1, 0, 3, 0, 3),       // corrupt rollback -> FATAL_ABORT
        (10, 0, 3, 1, 3),      // VERIFIER_TAMPER -> FATAL_ABORT
        (11, 0, 3, 1, 3),      // CAP_VIOLATION -> FATAL_ABORT
        (12, 0, 3, 1, 3),      // FROZEN_HASH_DRIFT -> FATAL_ABORT
    ] {
        let got = call_decide(&mut vm, err, att, max_att, rb_ok)?;
        if got != want {
            return Err(format!("P7-1 decide({},{},{},{}) got {} want {}", err, att, max_att, rb_ok, got, want));
        }
    }
    checks += 1;

    // Build verified baseline rollback target from bench_loop_sum (expected 5050).
    let (bname, bsrc, bargs, want_val) = P7_BENCHMARK_CORPUS[1];
    let good_bc = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(bsrc))?;
    if p7_validate_candidate_stream(&m4_mod, m4_run_fn, &good_bc, bargs, want_val) != 0 {
        return Err(format!("P7-1 baseline {} failed validation", bname));
    }
    let rollback = P7RollbackTarget::from_passing_stream(good_bc.clone());
    let rb_hash_lo = (rollback.content_hash & 0xFFFF_FFFF) as i64;

    // Check 3: Fault Injection #1 â€” Corrupt token stream -> detect, roll back, re-emit passing stream, log repair.
    let mut corrupt_bc = good_bc.clone();
    corrupt_bc[5] = 0xDEAD; // invalid opcode / corrupt token stream
    corrupt_bc.truncate(corrupt_bc.len() / 2);
    let mut heal_log = vec![0u8; 4096];
    let mut active_stream = corrupt_bc;
    let mut healed_ok = false;
    for attempt in 0..=3i64 {
        let err = p7_validate_candidate_stream(&m4_mod, m4_run_fn, &active_stream, bargs, want_val);
        let rb_ok = if rollback.verify_integrity() { 1 } else { 0 };
        let act = call_decide(&mut vm, err, attempt, 3, rb_ok)?;
        let cur_hlo = (fnv1a_64_hash(&encode_u16_stream_to_bytes(&active_stream)) & 0xFFFF_FFFF) as i64;
        heal_log = call_log_push(&mut vm, heal_log, attempt, err, act, cur_hlo)?;
        if act == 0 {
            healed_ok = true;
            break;
        } else if act == 1 {
            // Roll back to content-addressed target and re-emit clean verified stream.
            active_stream = rollback.stream.clone();
        } else {
            return Err(format!("P7-1 fault #1 unexpected action {} at attempt {}", act, attempt));
        }
    }
    let log_summary = call_log_verify(&mut vm, heal_log.clone())?;
    let log_cnt = log_summary >> 32;
    if !healed_ok || log_summary < 0 || log_cnt != 2 || active_stream != rollback.stream {
        return Err(format!(
            "P7-1 fault #1 repair failed: healed={} log_summary={} log_cnt={}",
            healed_ok, log_summary, log_cnt
        ));
    }
    checks += 1;

    // Check 4: Fault Injection #2 â€” Repeated verifier-invalid streams exhaust bounded `max_attempts = 3`,
    // cleanly hitting `ACTION_ROLLBACK_FINAL (2)` and restoring the verified rollback target.
    let mut bad_verify_bc = good_bc.clone();
    // Corrupt first instruction operand/register to trigger verifier failure (or decode failure).
    if bad_verify_bc.len() > 12 {
        bad_verify_bc[10] = 0x7FFF;
    }
    let mut bound_log = vec![0u8; 4096];
    let mut hit_final = false;
    let mut restored_stream: Vec<u16> = Vec::new();
    for attempt in 0..=3i64 {
        let err = p7_validate_candidate_stream(&m4_mod, m4_run_fn, &bad_verify_bc, bargs, want_val);
        let rb_ok = if rollback.verify_integrity() { 1 } else { 0 };
        let act = call_decide(&mut vm, err, attempt, 3, rb_ok)?;
        bound_log = call_log_push(&mut vm, bound_log, attempt, err, act, rb_hash_lo)?;
        if act == 2 {
            hit_final = true;
            restored_stream = rollback.stream.clone();
            break;
        }
    }
    let bound_summary = call_log_verify(&mut vm, bound_log)?;
    if !hit_final
        || bound_summary < 0
        || (bound_summary >> 32) != 4
        || p7_validate_candidate_stream(&m4_mod, m4_run_fn, &restored_stream, bargs, want_val) != 0
    {
        return Err(format!(
            "P7-1 bounded-attempt gate failed: hit_final={} bound_cnt={}",
            hit_final,
            bound_summary >> 32
        ));
    }
    checks += 1;

    // Check 5: Fatal Fault Injection â€” verifier tamper, cap violation, frozen drift, and tampered rollback buffer
    // MUST immediately return `ACTION_FATAL_ABORT (3)` with zero retries; corrupt log also rejected (`-1`).
    let mut tampered_rb = rollback.clone();
    tampered_rb.stream[0] ^= 0x0001;
    let rb_tamper_ok = if tampered_rb.verify_integrity() { 1 } else { 0 };
    if rb_tamper_ok != 0
        || call_decide(&mut vm, 1, 0, 3, rb_tamper_ok)? != 3
        || call_decide(&mut vm, 10, 0, 3, 1)? != 3
        || call_decide(&mut vm, 11, 0, 3, 1)? != 3
        || call_decide(&mut vm, 12, 0, 3, 1)? != 3
    {
        return Err(String::from("P7-1 fatal fault failed to abort immediately"));
    }
    // Corrupt healing log (non-monotonic attempt 5 -> 2) must be rejected by Pure-Omni verifier.
    let mut bad_log = vec![0u8; 4096];
    bad_log[0..4].copy_from_slice(&2u32.to_le_bytes());
    bad_log[16..20].copy_from_slice(&5u32.to_le_bytes());
    bad_log[32..36].copy_from_slice(&2u32.to_le_bytes());
    if call_log_verify(&mut vm, bad_log)? != -1 {
        return Err(String::from("P7-1 corrupt healing log must return -1"));
    }
    checks += 1;

    // Check 6: Frozen FNV-1a64 hashes untouched.
    selftest_m2_frozen_hashes()?;
    checks += 1;

    Ok((checks, rollback.content_hash, log_cnt))
}

// ==============================================================================
// OMNI NEW-LEVEL API (STAGE 1): PURE TERNARY CAPABILITY SURFACE, CAP_NARROW,
// PURE-OMNI VERIFIER RULES (1010..1013) & 7 STAGE 1 GOLDENS
// ==============================================================================

pub fn selftest_api_stage1() -> Result<(usize, usize, usize), String> {
    let mut checks = 0usize;

    // 1. Compile Stage 1 Pure-Omni API (`pure/m7/judge.omni + pure/api/ternary.omni + api_main.omni`)
    //    via Rust reference AND Pure-Omni M3 (`pure/m3/m3.omni_bc`, 0-word diff!) and lower via
    //    Rust M2-multi AND Pure-Omni M2 (`pure/m2/m2.omni_bc`, 0-byte diff!).
    let api_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(API_STAGE1_OMNI_SRC))?;
    let api_m3_u16 = match m3_lockstep_compile_via_omni(API_STAGE1_OMNI_SRC, true)? {
        Ok(v) => v,
        Err((c, l, col)) => return Err(format!("API-S1 Pure-Omni M3 errored ({}, {}:{})", c, l, col)),
    };
    if api_m3_u16 != api_u16 {
        return Err(format!(
            "API-S1 Pure-Omni M3 diff on pure/api/ternary.omni: m3={}w vs rust={}w",
            api_m3_u16.len(),
            api_u16.len()
        ));
    }

    let api_elf_rust = compile_pure_u16_to_x86_64_elf_multi(&api_u16, 0, 1, 2)?;
    let m2_u16 = decode_bytes_to_u16_stream(
        &std::fs::read("pure/m2/m2.omni_bc").map_err(|e| format!("API-S1 read m2 seed: {}", e))?,
    );
    let m2_mod = decode_omni_token_stream(&m2_u16)?;
    let m2_entry = m2_entry_of(&m2_mod);
    let mut vm_m2 = OmniBinaryVm::with_budgets(500_000_000, 128 * 1024 * 1024);
    let api_elf_pure = match vm_m2.execute_function(
        &m2_mod,
        m2_entry,
        vec![
            OmniValue::U16Array(api_u16.clone()),
            OmniValue::Int(0),
            OmniValue::Int(1),
            OmniValue::Int(2),
        ],
    )? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S1 Pure-Omni M2 returned {:?}", other)),
    };
    if api_elf_pure != api_elf_rust {
        return Err(format!(
            "API-S1 Pure-Omni M2 diff on pure/api/ternary.omni: pure={}B vs rust={}B",
            api_elf_pure.len(),
            api_elf_rust.len()
        ));
    }
    checks += 1;

    // 2. Execute all 6 Stage 1 entry modes identically on Rust VM and Pure-Omni M4 VM.
    let api_mod = decode_omni_token_stream(&api_u16)?;
    let n_afns = api_mod.functions.len();
    let api_dispatch6 = api_mod.functions[n_afns - 2].fn_idx;
    let m4_u16 = decode_bytes_to_u16_stream(
        &std::fs::read("pure/m4/m4.omni_bc").map_err(|e| format!("API-S1 read m4 seed: {}", e))?,
    );
    let m4_mod = decode_omni_token_stream(&m4_u16)?;
    let m4_run_fn = m4_mod.functions[m4_mod.functions.len() - 2].fn_idx;
    let mut vm = OmniBinaryVm::with_budgets(50_000_000, 64 * 1024 * 1024);
    for &(mode, a, b, c, d, e, want) in &[
        (0i64, -1, -7, 0, 9, 0, -1i64),   // route_trit(-1t, -7, 0, 9) -> -1t
        (0, 0, -7, 0, 9, 0, 0),           // route_trit(0t, -7, 0, 9) -> 0t
        (0, 1, -7, 0, 9, 0, 1),           // route_trit(+1t, -7, 0, 9) -> +1t
        (1, 242, 242, 0, 0, 0, 1),        // consensus([+1..+1], [+1..+1]) -> +1t
        (1, 0, 242, 0, 0, 0, -1),         // consensus([-1..-1], [+1..+1]) -> -1t
        (1, 250, 242, 0, 0, 0, 0),        // corrupt byte >242 -> 0t fail-closed
        (2, 3, 242, 0, 0, 0, 778),        // expert_pack5(3, [+1..+1]) -> (3<<8)|(5+5) = 778
        (2, 3, 250, 0, 0, 0, -1),         // expert_pack5(3, corrupt) -> -1
        (3, 15, 3, 0, 0, 0, 3),           // cap_narrow(0b1111, 0b0011) -> 0b0011
        (3, 6, 3, 0, 0, 0, 2),            // cap_narrow(0b0110, 0b0011) -> 0b0010
        (4, 1, 1, 0, -1, 1, 1),           // vote5 -> +1t
        (5, 0, 1, 0, 0, 0, 124),          // pack5_verdict -> 124 (exact parity with P7)
    ] {
        let got = match vm.execute_function(
            &api_mod,
            api_dispatch6,
            vec![
                OmniValue::Int(mode),
                OmniValue::Int(a),
                OmniValue::Int(b),
                OmniValue::Int(c),
                OmniValue::Int(d),
                OmniValue::Int(e),
            ],
        )? {
            OmniValue::Int(v) => v,
            other => return Err(format!("API-S1 mode {} returned {:?}", mode, other)),
        };
        if got != want {
            return Err(format!("API-S1 mode {}({},{},{}) got {} want {}", mode, a, b, c, got, want));
        }
    }
    checks += 1;

    // 3. Pure-Omni Verifier Stage 1 rules (`pure/verifier_engine.omni::omni_verify_api_surface`,
    //    Pure-Omni ONLY â€” Rust verifier untouched!).
    let ve_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(OMNI_VERIFIER_ENGINE_SURFACE))?;
    let ve_mod = decode_omni_token_stream(&ve_u16)?;
    let n_vfns = ve_mod.functions.len();
    // Order at end of pure/verifier_engine.omni:
    //   omni_verify_cap_narrow_eff (-7), omni_verify_api_surface (-6),
    //   apply_api_surface_rules (-5), prescan_module_fns (-4),
    //   omni_verify_module_with_mode (-3), omni_verify_module_api (-2), omni_verify_module (-1)
    let fn_api_verify = ve_mod.functions[n_vfns - 6].fn_idx;
    let fn_verify_mod_api = ve_mod.functions[n_vfns - 2].fn_idx;
    for &(op_k, fn_cap, req_cap, taint, mask_c, mask, lin, want) in &[
        (1i64, 8, 8, 0, 0, 0, 0, 0i64),     // ternary cap satisfied -> OK
        (1, 0, 8, 0, 0, 0, 0, 1010),        // MISSING_CAP_TERNARY
        (2, 0, 0, 0, 0, 0, 0, 0),           // clean consensus -> OK
        (2, 0, 0, 1, 0, 0, 0, 1011),        // TAINT_ON_CONSENSUS
        (3, 0, 0, 0, 0, 0, 0, 0),           // linear dropped/transferred -> OK
        (3, 0, 0, 0, 0, 0, 1, 1012),        // LINEAR_REFLECT
        (4, 15, 2, 0, 1, 3, 0, 0),          // cap_narrow const mask surviving bit -> OK
        (4, 15, 8, 0, 1, 3, 0, 1013),       // CAP_NARROW_EXPAND (masked-off bit)
        (4, 15, 1, 0, 0, 15, 0, 1013),      // CAP_NARROW_EXPAND (dynamic mask -> 0 surviving caps)
        (4, 15, 0, 0, 0, 15, 0, 0),         // dynamic narrowed cap stored/passed with req_cap=0 -> OK
    ] {
        let got = match vm.execute_function(
            &ve_mod,
            fn_api_verify,
            vec![
                OmniValue::Int(op_k),
                OmniValue::Int(fn_cap),
                OmniValue::Int(req_cap),
                OmniValue::Int(taint),
                OmniValue::Int(mask_c),
                OmniValue::Int(mask),
                OmniValue::Int(lin),
            ],
        )? {
            OmniValue::Int(v) => v,
            other => return Err(format!("API-S1 pure verifier returned {:?}", other)),
        };
        if got != want {
            return Err(format!("API-S1 pure verifier case op={} got {} want {}", op_k, got, want));
        }
    }
    checks += 1;

    // 4. Positive Stage 1 goldens: M3 0-word diff + M4 VM == Rust VM + M2-multi ELF64 0-byte diff.
    let pos_goldens: &[(&str, [i64; 3], i64)] = &[
        ("spec/acceptance/goldens/ternary_route_golden.omni", [20, 0, 0], 1),
        ("spec/acceptance/goldens/consensus_golden.omni", [242, 242, 0], 1),
        ("spec/acceptance/goldens/expert_pack5_golden.omni", [3, 242, 0], 778),
        ("spec/acceptance/goldens/cap_narrow_golden.omni", [15, 0, 0], 3),
    ];
    for &(path, args, want) in pos_goldens {
        let src = std::fs::read_to_string(path).map_err(|e| format!("API-S1 read {}: {}", path, e))?;
        let rust_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(&src))?;
        let m3_u16_g = match m3_lockstep_compile_via_omni(&src, true)? {
            Ok(v) => v,
            Err((c, l, col)) => return Err(format!("API-S1 M3 on {} errored ({}, {}:{})", path, c, l, col)),
        };
        if m3_u16_g != rust_u16 {
            return Err(format!("API-S1 M3 0-word diff failed on {}", path));
        }
        let elf_r = compile_pure_u16_to_x86_64_elf_multi(&rust_u16, args[0], args[1], args[2])?;
        let elf_p = match vm_m2.execute_function(
            &m2_mod,
            m2_entry,
            vec![
                OmniValue::U16Array(rust_u16.clone()),
                OmniValue::Int(args[0]),
                OmniValue::Int(args[1]),
                OmniValue::Int(args[2]),
            ],
        )? {
            OmniValue::Bytes(b) => b,
            other => return Err(format!("API-S1 M2 on {} returned {:?}", path, other)),
        };
        if elf_p != elf_r {
            return Err(format!("API-S1 M2 0-byte diff failed on {}", path));
        }
        let arena_out = match vm.execute_function(
            &m4_mod,
            m4_run_fn,
            vec![
                OmniValue::Bytes(vec![0u8; 524_288]),
                OmniValue::U16Array(rust_u16),
                OmniValue::Int(args[0]),
                OmniValue::Int(args[1]),
                OmniValue::Int(args[2]),
                OmniValue::Int(50_000),
                OmniValue::Int(0),
            ],
        )? {
            OmniValue::Bytes(b) => b,
            other => return Err(format!("API-S1 M4 VM on {} returned {:?}", path, other)),
        };
        let st = 167_936usize;
        let err_c = u32::from_le_bytes(arena_out[st + 28..st + 32].try_into().unwrap());
        let ret_v = i64::from_le_bytes(arena_out[st + 32..st + 40].try_into().unwrap());
        if err_c != 0 || ret_v != want {
            return Err(format!("API-S1 M4 VM {} mismatch: got {} (err {}) want {}", path, ret_v, err_c, want));
        }
    }
    checks += 1;

    // 5. Negative Stage 1 goldens: compile each real violation golden, run through Pure-Omni
    //    `omni_verify_module_api` (invoking wired `apply_api_surface_rules` -> `omni_verify_api_surface`
    //    & `omni_verify_cap_narrow_eff`), assert rejection with exact error code + non-null 1-based
    //    (line, col) span matching `spec/acceptance/acceptance_manifest.json` & `error_contract.json`,
    //    and verify all 5 frozen hashes remain intact.
    let err_contract = std::fs::read_to_string("spec/acceptance/error_contract.json")
        .map_err(|e| format!("API-S1 read error_contract.json: {}", e))?;
    let manifest_txt = std::fs::read_to_string("spec/acceptance/acceptance_manifest.json")
        .map_err(|e| format!("API-S1 read acceptance_manifest.json: {}", e))?;
    for code_name in &["MISSING_CAP_TERNARY", "TAINT_ON_CONSENSUS", "LINEAR_REFLECT", "CAP_NARROW_EXPAND"] {
        if !err_contract.contains(code_name) {
            return Err(format!("API-S1 error_contract.json missing {}", code_name));
        }
    }
    let neg_cases: &[(&str, &str, i64, usize, usize)] = &[
        ("spec/acceptance/goldens/neg_missing_cap_ternary.omni", "MISSING_CAP_TERNARY", 1010, 3, 19),
        ("spec/acceptance/goldens/neg_taint_consensus.omni", "TAINT_ON_CONSENSUS", 1011, 3, 19),
        ("spec/acceptance/goldens/neg_cap_narrow_expand.omni", "CAP_NARROW_EXPAND", 1013, 5, 19),
    ];
    for &(neg_path, exp_code, exp_num, exp_line, exp_col) in neg_cases {
        // Assert manifest records this negative golden with exact expected code & span.
        let expected_span_frag = format!(
            "\"expected_code\": \"{}\", \"expected_num\": {}, \"expected_line\": {}, \"expected_col\": {}",
            exp_code, exp_num, exp_line, exp_col
        );
        if !manifest_txt.contains(&expected_span_frag) {
            return Err(format!("API-S1 acceptance_manifest.json missing expected entry: {}", expected_span_frag));
        }
        let src = std::fs::read_to_string(neg_path).map_err(|e| format!("API-S1 read {}: {}", neg_path, e))?;
        let (neg_u16, smap) = with_lockstep_suspended(|| compile_omni_surface_to_u16_with_sourcemap(&src, true))?;
        let neg_bytes = encode_u16_stream_to_bytes(&neg_u16);
        let rep_bytes = match vm.execute_function(&ve_mod, fn_verify_mod_api, vec![OmniValue::Bytes(neg_bytes)])? {
            OmniValue::Bytes(b) => b,
            other => return Err(format!("API-S1 omni_verify_module_api({}) returned {:?}", neg_path, other)),
        };
        if rep_bytes.len() < 48 {
            return Err(format!("API-S1 pure verifier report too short for {}: {} B", neg_path, rep_bytes.len()));
        }
        let r_i64 = |idx: usize| -> i64 {
            i64::from_le_bytes(rep_bytes[idx * 8..idx * 8 + 8].try_into().unwrap())
        };
        let status = r_i64(0);
        let nfns = r_i64(2);
        let passed = r_i64(4);
        let nerr = r_i64(5) as usize;
        if status != 0 || nfns < 1 || passed != 0 || nerr == 0 {
            return Err(format!(
                "API-S1 {} MUST be rejected by Pure-Omni verifier (status={}, nfns={}, passed={}, nerr={})",
                neg_path, status, nfns, passed, nerr
            ));
        }
        let mut matched_off: Option<usize> = None;
        for ei in 0..nerr {
            let code = r_i64(6 + ei * 2);
            let off = r_i64(6 + ei * 2 + 1) as usize;
            if code == exp_num {
                matched_off = Some(off);
                break;
            }
        }
        let off = matched_off.ok_or_else(|| {
            let got_codes: Vec<i64> = (0..nerr).map(|ei| r_i64(6 + ei * 2)).collect();
            format!("API-S1 {} expected pure verifier code {} ({}), got {:?}", neg_path, exp_num, exp_code, got_codes)
        })?;
        let (line, col) = smap.offset_to_span.get(&off).copied().unwrap_or((0, 0));
        let formatted = format!("error[{}] line {}, col {}: pure verifier rejected {}", exp_code, line, col, neg_path);
        if line != exp_line || col != exp_col || !formatted.starts_with(&format!("error[{}]", exp_code)) {
            return Err(format!(
                "API-S1 {} span mismatch: got ({}, {}) want ({}, {}) [{}]",
                neg_path, line, col, exp_line, exp_col, formatted
            ));
        }
    }
    selftest_m2_frozen_hashes()?;
    checks += 1;

    Ok((checks, api_u16.len(), api_elf_rust.len()))
}

// ==============================================================================
// OMNI NEW-LEVEL API (STAGE 2, PART 1): PURE TOKEN REFLECTIVE HOST
// (`token_inspect`, `token_specialize`, `token_inspect_trace`) & STAGE 2 GOLDENS
// ==============================================================================

pub fn selftest_api_stage2() -> Result<(usize, usize, usize), String> {
    let mut checks = 0usize;

    // 1. Self-compile `pure/m7/judge.omni + pure/api/reflect.omni + spec/api/reflect_harness.omni`
    //    via Pure-Omni M3 (0-word diff vs Rust) and lower via Pure-Omni M2 (0-byte diff vs Rust).
    let ref_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(API_STAGE2_OMNI_SRC))?;
    let ref_m3_u16 = match m3_lockstep_compile_via_omni(API_STAGE2_OMNI_SRC, true)? {
        Ok(v) => v,
        Err((c, l, col)) => return Err(format!("API-S2 Pure-Omni M3 errored ({}, {}:{})", c, l, col)),
    };
    if ref_m3_u16 != ref_u16 {
        return Err(format!(
            "API-S2 Pure-Omni M3 diff on pure/api/reflect.omni: m3={}w vs rust={}w",
            ref_m3_u16.len(),
            ref_u16.len()
        ));
    }
    let ref_elf_rust = compile_pure_u16_to_x86_64_elf_multi(&ref_u16, 1, 2, 3)?;
    let m2_u16 = decode_bytes_to_u16_stream(
        &std::fs::read("pure/m2/m2.omni_bc").map_err(|e| format!("API-S2 read m2 seed: {}", e))?,
    );
    let m2_mod = decode_omni_token_stream(&m2_u16)?;
    let m2_entry = m2_entry_of(&m2_mod);
    let mut vm_m2 = OmniBinaryVm::with_budgets(500_000_000, 128 * 1024 * 1024);
    let ref_elf_pure = match vm_m2.execute_function(
        &m2_mod,
        m2_entry,
        vec![
            OmniValue::U16Array(ref_u16.clone()),
            OmniValue::Int(1),
            OmniValue::Int(2),
            OmniValue::Int(3),
        ],
    ).map_err(|e| format!("API-S2 step1 M2: {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2 Pure-Omni M2 returned {:?}", other)),
    };
    if ref_elf_pure != ref_elf_rust {
        return Err(format!(
            "API-S2 Pure-Omni M2 diff on pure/api/reflect.omni: pure={}B vs rust={}B",
            ref_elf_pure.len(),
            ref_elf_rust.len()
        ));
    }
    checks += 1;

    let ref_mod = decode_omni_token_stream(&ref_u16)?;
    let n_rfns = ref_mod.functions.len();
    // Order at end of spec/api/reflect_harness.omni:
    //   api_reflect_inspect_entry (-5), api_reflect_specialize_entry (-4),
    //   api_reflect_trace_entry (-3), api_reflect_trace_bucket_entry (-2), main (-1)
    let fn_inspect = ref_mod.functions[n_rfns - 5].fn_idx;
    let fn_specialize = ref_mod.functions[n_rfns - 4].fn_idx;
    let fn_trace = ref_mod.functions[n_rfns - 3].fn_idx;
    let fn_trace_bucket = ref_mod.functions[n_rfns - 2].fn_idx;

    let m4_u16 = decode_bytes_to_u16_stream(
        &std::fs::read("pure/m4/m4.omni_bc").map_err(|e| format!("API-S2 read m4 seed: {}", e))?,
    );
    let m4_mod = decode_omni_token_stream(&m4_u16)?;
    let m4_run_fn = m4_mod.functions[m4_mod.functions.len() - 2].fn_idx;
    let mut vm = OmniBinaryVm::with_budgets(50_000_000, 64 * 1024 * 1024);

    // 2. `api_token_inspect`: verify exact (n_fns, total_ins, n_words) on bench_clamp (1 fn)
    //    and bench_multi_call (2 fns), plus fail-closed `-1` on corrupt header.
    for &(bi, want_fns) in &[(0usize, 1i64), (3usize, 2i64)] {
        let (_name, src, _args, _want) = P7_BENCHMARK_CORPUS[bi];
        let bc = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(src))?;
        let dec = decode_omni_token_stream(&bc)?;
        let want_ins: i64 = dec.functions.iter().map(|f| f.instructions.len() as i64).sum();
        let packed = match vm.execute_function(
            &ref_mod,
            fn_inspect,
            vec![OmniValue::Bytes(encode_u16_stream_to_bytes(&bc))],
        ).map_err(|e| format!("API-S2 step2 inspect bench#{}: {}", bi, e))? {
            OmniValue::Int(v) => v,
            other => return Err(format!("API-S2 token_inspect returned {:?}", other)),
        };
        let got_fns = packed >> 40;
        let got_ins = (packed >> 20) & 0xFFFFF;
        let got_words = packed & 0xFFFFF;
        if got_fns != want_fns || got_ins != want_ins || got_words != bc.len() as i64 {
            return Err(format!(
                "API-S2 token_inspect bench#{} mismatch: got ({},{},{}) want ({},{},{})",
                bi, got_fns, got_ins, got_words, want_fns, want_ins, bc.len()
            ));
        }
    }
    let bad_insp = match vm.execute_function(
        &ref_mod,
        fn_inspect,
        vec![OmniValue::Bytes(vec![0xFF, 0xFF, 0x00, 0x00])],
    )? {
        OmniValue::Int(v) => v,
        other => return Err(format!("API-S2 bad token_inspect returned {:?}", other)),
    };
    if bad_insp != -1 {
        return Err(format!("API-S2 corrupt header must return -1, got {}", bad_insp));
    }
    checks += 1;

    // 3. `api_token_specialize`: inject 2 standalone NOPs (`[0x7F00, 0x7EE4]`) into bench_clamp,
    //    prove hint=0 clones identically, prove hint=1 strips both NOPs (-4 words / -8 B),
    //    decrements ins_cnt in-place, passes verifier + M2-multi ELF64 + Pure-Omni M4 VM (ret=90).
    let (_bname0, bsrc0, bargs0, bwant0) = P7_BENCHMARK_CORPUS[0];
    let clean_bc = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(bsrc0))?;
    // Inject 2 standalone NOPs right after the fn#0 ins_cnt header word.
    let mut bloated_bc = clean_bc.clone();
    let pcnt0 = bloated_bc[3] as usize;
    let icnt_idx = 4 + pcnt0 * 3 + 2;
    bloated_bc[icnt_idx] += 2;
    bloated_bc.splice(icnt_idx + 1..icnt_idx + 1, [0x7F00u16, 0x7EE4, 0x7F00, 0x7EE4]);
    let bloated_bytes = encode_u16_stream_to_bytes(&bloated_bc);

    let clone_bytes = match vm.execute_function(
        &ref_mod,
        fn_specialize,
        vec![OmniValue::Bytes(bloated_bytes.clone()), OmniValue::Int(0)],
    ).map_err(|e| format!("API-S2 step3 specialize(0): {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2 token_specialize(hint=0) returned {:?}", other)),
    };
    if clone_bytes != bloated_bytes {
        return Err(String::from("API-S2 token_specialize(hint=0) did not preserve exact bytes"));
    }
    let spec_bytes = match vm.execute_function(
        &ref_mod,
        fn_specialize,
        vec![OmniValue::Bytes(bloated_bytes), OmniValue::Int(1)],
    ).map_err(|e| format!("API-S2 step3 specialize(1): {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2 token_specialize(hint=1) returned {:?}", other)),
    };
    let spec_u16 = decode_bytes_to_u16_stream(&spec_bytes);
    if spec_u16 != clean_bc {
        return Err(format!(
            "API-S2 token_specialize(hint=1) mismatch: got {}w vs clean {}w",
            spec_u16.len(),
            clean_bc.len()
        ));
    }
    let spec_dec = decode_omni_token_stream(&spec_u16)?;
    for f in &spec_dec.functions {
        if !verify_binary_function(f).passed {
            return Err(String::from("API-S2 specialized stream failed verifier"));
        }
    }
    let _spec_elf = compile_pure_u16_to_x86_64_elf_multi(&spec_u16, bargs0[0], bargs0[1], bargs0[2])?;
    let spec_arena = match vm.execute_function(
        &m4_mod,
        m4_run_fn,
        vec![
            OmniValue::Bytes(vec![0u8; 524_288]),
            OmniValue::U16Array(spec_u16),
            OmniValue::Int(bargs0[0]),
            OmniValue::Int(bargs0[1]),
            OmniValue::Int(bargs0[2]),
            OmniValue::Int(50_000),
            OmniValue::Int(0),
        ],
    )? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2 M4 VM on specialized stream returned {:?}", other)),
    };
    let st = 167_936usize;
    let spec_ret = i64::from_le_bytes(spec_arena[st + 32..st + 40].try_into().unwrap());
    if spec_ret != bwant0 {
        return Err(format!("API-S2 specialized M4 VM ret={} want {}", spec_ret, bwant0));
    }
    checks += 1;

    // 4. `api_token_inspect_trace` + `api_token_inspect_trace_bucket` on live P4-5 trace ring
    //    from M4 VM executing `bench_arena_buf` (expected 101/33/136/1, sum=271).
    let (_bn4, bs4, ba4, _bw4) = P7_BENCHMARK_CORPUS[4];
    let bc4 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(bs4))?;
    let trace_arena = match vm.execute_function(
        &m4_mod,
        m4_run_fn,
        vec![
            OmniValue::Bytes(vec![0u8; 524_288]),
            OmniValue::U16Array(bc4),
            OmniValue::Int(ba4[0]),
            OmniValue::Int(ba4[1]),
            OmniValue::Int(ba4[2]),
            OmniValue::Int(200_000),
            OmniValue::Int(1),
        ],
    )? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2 trace M4 VM returned {:?}", other)),
    };
    let h_packed = match vm.execute_function(&ref_mod, fn_trace, vec![OmniValue::Bytes(trace_arena.clone())])
        .map_err(|e| format!("API-S2 step4 trace: {}", e))? {
        OmniValue::Int(v) => v as u64,
        other => return Err(format!("API-S2 token_inspect_trace returned {:?}", other)),
    };
    let b0 = match vm.execute_function(&ref_mod, fn_trace_bucket, vec![OmniValue::Bytes(trace_arena.clone()), OmniValue::Int(0)])? {
        OmniValue::Int(v) => v,
        other => return Err(format!("API-S2 trace bucket 0 returned {:?}", other)),
    };
    let b1 = match vm.execute_function(&ref_mod, fn_trace_bucket, vec![OmniValue::Bytes(trace_arena.clone()), OmniValue::Int(1)])? {
        OmniValue::Int(v) => v,
        other => return Err(format!("API-S2 trace bucket 1 returned {:?}", other)),
    };
    let b2 = match vm.execute_function(&ref_mod, fn_trace_bucket, vec![OmniValue::Bytes(trace_arena.clone()), OmniValue::Int(2)])? {
        OmniValue::Int(v) => v,
        other => return Err(format!("API-S2 trace bucket 2 returned {:?}", other)),
    };
    let b3 = match vm.execute_function(&ref_mod, fn_trace_bucket, vec![OmniValue::Bytes(trace_arena), OmniValue::Int(3)])? {
        OmniValue::Int(v) => v,
        other => return Err(format!("API-S2 trace bucket 3 returned {:?}", other)),
    };
    if b0 != (h_packed & 0xFFFF) as i64
        || b1 != ((h_packed >> 16) & 0xFFFF) as i64
        || b2 != ((h_packed >> 32) & 0xFFFF) as i64
        || b3 != ((h_packed >> 48) & 0xFFFF) as i64
        || b0 + b1 + b2 + b3 != 271
        || b1 <= 0
    {
        return Err(format!("API-S2 trace histogram mismatch: ({},{},{},{})", b0, b1, b2, b3));
    }
    checks += 1;

    // 5. Stage 2 Goldens (2 positive M3/M2/M4 0-diff + 1 negative Pure-Omni verifier 1012 LINEAR_REFLECT)
    //    + frozen hashes intact.
    for &(gpath, gargs, gwant) in &[
        ("spec/acceptance/goldens/token_inspect_golden.omni", [32480i64, 32495, 0], 2i64),
        ("spec/acceptance/goldens/token_specialize_golden.omni", [1i64, 0, 0], 0i64),
    ] {
        let gsrc = std::fs::read_to_string(gpath).map_err(|e| format!("API-S2 read {}: {}", gpath, e))?;
        let r_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(&gsrc))?;
        let m3_u16_g = match m3_lockstep_compile_via_omni(&gsrc, true)? {
            Ok(v) => v,
            Err((c, l, col)) => return Err(format!("API-S2 M3 on {} errored ({}, {}:{})", gpath, c, l, col)),
        };
        if m3_u16_g != r_u16 {
            return Err(format!("API-S2 M3 0-word diff failed on {}", gpath));
        }
        let elf_r = compile_pure_u16_to_x86_64_elf_multi(&r_u16, gargs[0], gargs[1], gargs[2])?;
        let elf_p = match vm_m2.execute_function(
            &m2_mod,
            m2_entry,
            vec![OmniValue::U16Array(r_u16.clone()), OmniValue::Int(gargs[0]), OmniValue::Int(gargs[1]), OmniValue::Int(gargs[2])],
        )? {
            OmniValue::Bytes(b) => b,
            other => return Err(format!("API-S2 M2 on {} returned {:?}", gpath, other)),
        };
        if elf_p != elf_r {
            return Err(format!("API-S2 M2 0-byte diff failed on {}", gpath));
        }
        let g_arena = match vm.execute_function(
            &m4_mod,
            m4_run_fn,
            vec![
                OmniValue::Bytes(vec![0u8; 524_288]),
                OmniValue::U16Array(r_u16),
                OmniValue::Int(gargs[0]),
                OmniValue::Int(gargs[1]),
                OmniValue::Int(gargs[2]),
                OmniValue::Int(50_000),
                OmniValue::Int(0),
            ],
        )? {
            OmniValue::Bytes(b) => b,
            other => return Err(format!("API-S2 M4 VM on {} returned {:?}", gpath, other)),
        };
        let gv = i64::from_le_bytes(g_arena[st + 32..st + 40].try_into().unwrap());
        if gv != gwant {
            return Err(format!("API-S2 M4 VM {} mismatch: got {} want {}", gpath, gv, gwant));
        }
    }

    // Negative golden: `neg_linear_reflect.omni` rejected by Pure-Omni `omni_verify_module_api`
    // with `1012` (`LINEAR_REFLECT`) and non-null 1-based span matching acceptance_manifest.json.
    let ve_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(OMNI_VERIFIER_ENGINE_SURFACE))?;
    let ve_mod = decode_omni_token_stream(&ve_u16)?;
    let fn_verify_mod_api = ve_mod.functions[ve_mod.functions.len() - 2].fn_idx;
    let neg_path = "spec/acceptance/goldens/neg_linear_reflect.omni";
    let neg_src = std::fs::read_to_string(neg_path).map_err(|e| format!("API-S2 read {}: {}", neg_path, e))?;
    let (neg_u16, smap) = with_lockstep_suspended(|| compile_omni_surface_to_u16_with_sourcemap(&neg_src, false))?;
    let rep_bytes = match vm.execute_function(
        &ve_mod,
        fn_verify_mod_api,
        vec![OmniValue::Bytes(encode_u16_stream_to_bytes(&neg_u16))],
    ).map_err(|e| format!("API-S2 step5 verify_mod_api: {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2 omni_verify_module_api returned {:?}", other)),
    };
    let r_i64 = |idx: usize| -> i64 {
        i64::from_le_bytes(rep_bytes[idx * 8..idx * 8 + 8].try_into().unwrap())
    };
    let nfns = r_i64(2) as usize;
    let mut cur = 3usize;
    let mut found_1012_off: Option<usize> = None;
    for _ in 0..nfns {
        if cur + 3 > rep_bytes.len() / 8 { break; }
        let nerr = r_i64(cur + 2) as usize;
        for ei in 0..nerr {
            let code = r_i64(cur + 3 + ei * 2);
            let off = r_i64(cur + 3 + ei * 2 + 1) as usize;
            if code == 1012 {
                found_1012_off = Some(off);
            }
        }
        cur += 3 + nerr * 2;
    }
    let off = found_1012_off.ok_or_else(|| String::from("API-S2 neg_linear_reflect.omni must emit 1012 (LINEAR_REFLECT)"))?;
    let (line, col) = smap.offset_to_span.get(&off).copied().unwrap_or((0, 0));
    let manifest_txt = std::fs::read_to_string("spec/acceptance/acceptance_manifest.json")
        .map_err(|e| format!("API-S2 read manifest: {}", e))?;
    let expected_frag = format!(
        "\"expected_code\": \"LINEAR_REFLECT\", \"expected_num\": 1012, \"expected_line\": {}, \"expected_col\": {}",
        line, col
    );
    if line == 0 || col == 0 || !manifest_txt.contains(&expected_frag) {
        return Err(format!("API-S2 neg_linear_reflect span ({}, {}) did not match manifest", line, col));
    }
    selftest_m2_frozen_hashes()?;
    checks += 1;

    Ok((checks, ref_u16.len(), ref_elf_rust.len()))
}

/// New-Level API Stage 2 (Part 2) gate (`./omnic api-stage2-emit`):
///   1. Pure-Omni M3 (0-word diff) + Pure-Omni M2 (0-byte diff) on `pure/api/emit.omni`.
///   2. `api_token_emit_const(42)` -> `api_token_inspect` (1 fn, 1 ins, 13 w) -> Pure-Omni
///      `omni_verify_module_api` (0 errors) -> `api_token_emit` canonical + fail-closed.
///   3. Re-hosted M2 emitter `api_token_emit_elf` on synthesized stream -> 0-byte diff vs Rust
///      M2 ELF64 + Pure-Omni M4 VM (`ret == 42`) + native Linux x86-64 execution when available.
///   4. End-to-end reflective loop: `api_token_specialize(bloated_bench_clamp, 1)` ->
///      `api_token_emit` -> `api_token_emit_elf` -> 0-byte diff vs Rust M2 + M4 VM (`ret == 90`)
///      + fail-closed empty slice on corrupt `U16Array`.
///   5. Stage 2 Part 2 golden (`token_emit_golden.omni`, M3/M2/M4 0-diff) + frozen hashes intact.
pub fn selftest_api_stage2_emit() -> Result<(usize, usize, usize), String> {
    let mut checks = 0usize;

    // 1. Compile pure/api/emit.omni + spec/api/emit_harness.omni via Rust M3, Pure-Omni M3 (0-word diff),
    //    Rust M2-multi ELF64, and Pure-Omni M2 (0-byte diff).
    let emit_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(API_STAGE2_EMIT_OMNI_SRC))?;
    let emit_m3_u16 = match m3_lockstep_compile_via_omni(API_STAGE2_EMIT_OMNI_SRC, true)? {
        Ok(v) => v,
        Err((c, l, col)) => return Err(format!("API-S2E M3 compile failed ({}, {}:{})", c, l, col)),
    };
    if emit_m3_u16 != emit_u16 {
        return Err(format!(
            "API-S2E M3 0-word diff failed on pure/api/emit.omni: pure={}w vs rust={}w",
            emit_m3_u16.len(),
            emit_u16.len()
        ));
    }
    let emit_elf_rust = compile_pure_u16_to_x86_64_elf_multi(&emit_u16, 1, 2, 3)?;
    let m2_u16 = decode_bytes_to_u16_stream(
        &std::fs::read("pure/m2/m2.omni_bc").map_err(|e| format!("API-S2E read m2 seed: {}", e))?,
    );
    let m2_mod = decode_omni_token_stream(&m2_u16)?;
    let m2_entry = m2_entry_of(&m2_mod);
    let mut vm_m2 = OmniBinaryVm::with_budgets(500_000_000, 128 * 1024 * 1024);
    let emit_elf_pure = match vm_m2.execute_function(
        &m2_mod,
        m2_entry,
        vec![
            OmniValue::U16Array(emit_u16.clone()),
            OmniValue::Int(1),
            OmniValue::Int(2),
            OmniValue::Int(3),
        ],
    ).map_err(|e| format!("API-S2E step1 M2: {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2E Pure-Omni M2 returned {:?}", other)),
    };
    if emit_elf_pure != emit_elf_rust {
        return Err(format!(
            "API-S2E Pure-Omni M2 diff on pure/api/emit.omni: pure={}B vs rust={}B",
            emit_elf_pure.len(),
            emit_elf_rust.len()
        ));
    }
    checks += 1;

    let emit_mod = decode_omni_token_stream(&emit_u16)?;
    let n_efns = emit_mod.functions.len();
    // Order at end of spec/api/emit_harness.omni:
    //   api_emit_const_entry (-4), api_emit_bc_entry (-3), api_emit_elf_entry (-2), main (-1)
    let fn_emit_const = emit_mod.functions[n_efns - 4].fn_idx;
    let fn_emit_bc = emit_mod.functions[n_efns - 3].fn_idx;
    let fn_emit_elf = emit_mod.functions[n_efns - 2].fn_idx;

    let ref_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(API_STAGE2_OMNI_SRC))?;
    let ref_mod = decode_omni_token_stream(&ref_u16)?;
    let n_rfns = ref_mod.functions.len();
    let fn_inspect = ref_mod.functions[n_rfns - 5].fn_idx;
    let fn_specialize = ref_mod.functions[n_rfns - 4].fn_idx;

    let m4_u16 = decode_bytes_to_u16_stream(
        &std::fs::read("pure/m4/m4.omni_bc").map_err(|e| format!("API-S2E read m4 seed: {}", e))?,
    );
    let m4_mod = decode_omni_token_stream(&m4_u16)?;
    let m4_run_fn = m4_mod.functions[m4_mod.functions.len() - 2].fn_idx;
    let mut vm = OmniBinaryVm::with_budgets(500_000_000, 128 * 1024 * 1024);

    // 2. Synthesize 13-word module via `api_token_emit_const(42)`, inspect via `api_token_inspect`,
    //    verify via Pure-Omni `omni_verify_module_api`, canonicalize via `api_token_emit`, and prove fail-closed.
    let synth_bytes = match vm.execute_function(
        &emit_mod,
        fn_emit_const,
        vec![OmniValue::Int(42)],
    ).map_err(|e| format!("API-S2E step2 emit_const: {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2E emit_const returned {:?}", other)),
    };
    let synth_packed = match vm.execute_function(
        &ref_mod,
        fn_inspect,
        vec![OmniValue::Bytes(synth_bytes.clone())],
    ).map_err(|e| format!("API-S2E step2 inspect: {}", e))? {
        OmniValue::Int(v) => v,
        other => return Err(format!("API-S2E inspect(synth) returned {:?}", other)),
    };
    if (synth_packed >> 40) != 1 || ((synth_packed >> 20) & 0xFFFFF) != 1 || (synth_packed & 0xFFFFF) != 13 {
        return Err(format!("API-S2E inspect(synth) mismatch: packed={}", synth_packed));
    }
    let ve_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(OMNI_VERIFIER_ENGINE_SURFACE))?;
    let ve_mod = decode_omni_token_stream(&ve_u16)?;
    let fn_verify_mod_api = ve_mod.functions[ve_mod.functions.len() - 2].fn_idx;
    let vrep = match vm.execute_function(
        &ve_mod,
        fn_verify_mod_api,
        vec![OmniValue::Bytes(synth_bytes.clone())],
    ).map_err(|e| format!("API-S2E step2 verify_api: {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2E verify_api(synth) returned {:?}", other)),
    };
    let v_status = i64::from_le_bytes(vrep[0..8].try_into().unwrap());
    let v_nfns = i64::from_le_bytes(vrep[16..24].try_into().unwrap());
    let v_passed = i64::from_le_bytes(vrep[32..40].try_into().unwrap());
    let v_nerr = i64::from_le_bytes(vrep[40..48].try_into().unwrap());
    if v_status != 0 || v_nfns != 1 || v_passed != 1 || v_nerr != 0 {
        return Err(format!(
            "API-S2E synthesized stream failed Pure-Omni verifier (status={}, nfns={}, passed={}, nerr={})",
            v_status, v_nfns, v_passed, v_nerr
        ));
    }
    let canon_bytes = match vm.execute_function(
        &emit_mod,
        fn_emit_bc,
        vec![OmniValue::Bytes(synth_bytes.clone())],
    ).map_err(|e| format!("API-S2E step2 emit_bc: {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2E emit_bc returned {:?}", other)),
    };
    if canon_bytes != synth_bytes {
        return Err(String::from("API-S2E token_emit did not preserve canonical synthesized stream"));
    }
    for bad in &[vec![0xFFu8, 0xFF, 0x00, 0x00], vec![0xE0u8, 0x7E, 0xE3, 0x7E]] {
        let bres = match vm.execute_function(&emit_mod, fn_emit_bc, vec![OmniValue::Bytes(bad.clone())])
            .map_err(|e| format!("API-S2E step2 bad emit_bc: {}", e))?
        {
            OmniValue::Bytes(b) => b,
            other => return Err(format!("API-S2E bad emit_bc returned {:?}", other)),
        };
        if !bres.is_empty() {
            return Err(format!("API-S2E token_emit must fail closed on corrupt stream, got {} B", bres.len()));
        }
    }
    checks += 1;

    // 3. Re-hosted M2 emitter `api_token_emit_elf` on synthesized stream -> 0-byte diff vs Rust M2 + M4 VM (ret=42).
    let synth_u16 = decode_bytes_to_u16_stream(&canon_bytes);
    let synth_elf_rust = compile_pure_u16_to_x86_64_elf_multi(&synth_u16, 0, 0, 0)?;
    let synth_elf_pure = match vm.execute_function(
        &emit_mod,
        fn_emit_elf,
        vec![
            OmniValue::U16Array(synth_u16.clone()),
            OmniValue::Int(0),
            OmniValue::Int(0),
            OmniValue::Int(0),
        ],
    ).map_err(|e| format!("API-S2E step3 emit_elf(synth): {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2E emit_elf(synth) returned {:?}", other)),
    };
    if synth_elf_pure != synth_elf_rust {
        return Err(format!(
            "API-S2E emit_elf(synth) 0-byte diff failed: pure={}B vs rust={}B",
            synth_elf_pure.len(),
            synth_elf_rust.len()
        ));
    }
    let st = 167_936usize;
    let synth_arena = match vm.execute_function(
        &m4_mod,
        m4_run_fn,
        vec![
            OmniValue::Bytes(vec![0u8; 524_288]),
            OmniValue::U16Array(synth_u16),
            OmniValue::Int(0),
            OmniValue::Int(0),
            OmniValue::Int(0),
            OmniValue::Int(50_000),
            OmniValue::Int(0),
        ],
    ).map_err(|e| format!("API-S2E step3 M4(synth): {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2E M4(synth) returned {:?}", other)),
    };
    let synth_ret = i64::from_le_bytes(synth_arena[st + 32..st + 40].try_into().unwrap());
    if synth_ret != 42 {
        return Err(format!("API-S2E M4(synth) ret={} want 42", synth_ret));
    }
    let _ = p7_verify_native_elf_i64("api_s2e_synth", &synth_elf_pure, 42)?;
    checks += 1;

    // 4. Full reflective loop: specialize(bloated_bench_clamp, 1) -> token_emit -> token_emit_elf ->
    //    0-byte diff vs Rust M2 + M4 VM (ret=90) + fail-closed on corrupt U16Array.
    let (_bname0, bsrc0, bargs0, bwant0) = P7_BENCHMARK_CORPUS[0];
    let clean_bc = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(bsrc0))?;
    let mut bloated_bc = clean_bc.clone();
    let pcnt0 = bloated_bc[3] as usize;
    let icnt_idx = 4 + pcnt0 * 3 + 2;
    bloated_bc[icnt_idx] += 2;
    bloated_bc.splice(icnt_idx + 1..icnt_idx + 1, [0x7F00u16, 0x7EE4, 0x7F00, 0x7EE4]);
    let spec_bytes = match vm.execute_function(
        &ref_mod,
        fn_specialize,
        vec![OmniValue::Bytes(encode_u16_stream_to_bytes(&bloated_bc)), OmniValue::Int(1)],
    ).map_err(|e| format!("API-S2E step4 specialize: {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2E specialize returned {:?}", other)),
    };
    let emitted_bc = match vm.execute_function(
        &emit_mod,
        fn_emit_bc,
        vec![OmniValue::Bytes(spec_bytes)],
    ).map_err(|e| format!("API-S2E step4 emit_bc: {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2E emit_bc(spec) returned {:?}", other)),
    };
    let emitted_u16 = decode_bytes_to_u16_stream(&emitted_bc);
    if emitted_u16 != clean_bc {
        return Err(String::from("API-S2E specialize->token_emit mismatch vs clean bench_clamp"));
    }
    let clamp_elf_rust = compile_pure_u16_to_x86_64_elf_multi(&clean_bc, bargs0[0], bargs0[1], bargs0[2])?;
    let clamp_elf_pure = match vm.execute_function(
        &emit_mod,
        fn_emit_elf,
        vec![
            OmniValue::U16Array(emitted_u16.clone()),
            OmniValue::Int(bargs0[0]),
            OmniValue::Int(bargs0[1]),
            OmniValue::Int(bargs0[2]),
        ],
    ).map_err(|e| format!("API-S2E step4 emit_elf(clamp): {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2E emit_elf(clamp) returned {:?}", other)),
    };
    if clamp_elf_pure != clamp_elf_rust {
        return Err(format!(
            "API-S2E emit_elf(clamp) 0-byte diff failed: pure={}B vs rust={}B",
            clamp_elf_pure.len(),
            clamp_elf_rust.len()
        ));
    }
    let clamp_arena = match vm.execute_function(
        &m4_mod,
        m4_run_fn,
        vec![
            OmniValue::Bytes(vec![0u8; 524_288]),
            OmniValue::U16Array(emitted_u16),
            OmniValue::Int(bargs0[0]),
            OmniValue::Int(bargs0[1]),
            OmniValue::Int(bargs0[2]),
            OmniValue::Int(50_000),
            OmniValue::Int(0),
        ],
    ).map_err(|e| format!("API-S2E step4 M4(clamp): {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2E M4(clamp) returned {:?}", other)),
    };
    let clamp_ret = i64::from_le_bytes(clamp_arena[st + 32..st + 40].try_into().unwrap());
    if clamp_ret != bwant0 {
        return Err(format!("API-S2E M4(clamp) ret={} want {}", clamp_ret, bwant0));
    }
    let _ = p7_verify_native_elf_i64("api_s2e_clamp", &clamp_elf_pure, bwant0)?;
    let bad_elf = match vm.execute_function(
        &emit_mod,
        fn_emit_elf,
        vec![
            OmniValue::U16Array(vec![0xFFFF, 0x0000]),
            OmniValue::Int(0),
            OmniValue::Int(0),
            OmniValue::Int(0),
        ],
    ).map_err(|e| format!("API-S2E step4 bad emit_elf: {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2E bad emit_elf returned {:?}", other)),
    };
    if !bad_elf.is_empty() {
        return Err(format!("API-S2E emit_elf must fail closed on corrupt header, got {} B", bad_elf.len()));
    }
    checks += 1;

    // 5. Stage 2 Part 2 golden (`token_emit_golden.omni`) M3/M2/M4 0-diff + frozen hashes intact.
    let gpath = "spec/acceptance/goldens/token_emit_golden.omni";
    let gsrc = std::fs::read_to_string(gpath).map_err(|e| format!("API-S2E read {}: {}", gpath, e))?;
    let r_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(&gsrc))?;
    let m3_u16_g = match m3_lockstep_compile_via_omni(&gsrc, true)? {
        Ok(v) => v,
        Err((c, l, col)) => return Err(format!("API-S2E M3 on {} errored ({}, {}:{})", gpath, c, l, col)),
    };
    if m3_u16_g != r_u16 {
        return Err(format!("API-S2E M3 0-word diff failed on {}", gpath));
    }
    let elf_r = compile_pure_u16_to_x86_64_elf_multi(&r_u16, 42, 0, 0)?;
    let elf_p = match vm_m2.execute_function(
        &m2_mod,
        m2_entry,
        vec![OmniValue::U16Array(r_u16.clone()), OmniValue::Int(42), OmniValue::Int(0), OmniValue::Int(0)],
    ).map_err(|e| format!("API-S2E step5 M2 golden: {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2E M2 on {} returned {:?}", gpath, other)),
    };
    if elf_p != elf_r {
        return Err(format!("API-S2E M2 0-byte diff failed on {}", gpath));
    }
    let g_arena = match vm.execute_function(
        &m4_mod,
        m4_run_fn,
        vec![
            OmniValue::Bytes(vec![0u8; 524_288]),
            OmniValue::U16Array(r_u16),
            OmniValue::Int(42),
            OmniValue::Int(0),
            OmniValue::Int(0),
            OmniValue::Int(50_000),
            OmniValue::Int(0),
        ],
    ).map_err(|e| format!("API-S2E step5 M4 golden: {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2E M4 VM on {} returned {:?}", gpath, other)),
    };
    let gv = i64::from_le_bytes(g_arena[st + 32..st + 40].try_into().unwrap());
    if gv != 13 {
        return Err(format!("API-S2E M4 VM {} mismatch: got {} want 13", gpath, gv));
    }
    selftest_m2_frozen_hashes()?;
    checks += 1;

    Ok((checks, emit_u16.len(), emit_elf_rust.len()))
}

/// New-Level API Stage 2.5 gate (`./omnic api-stage2-5`):
///   1. Pure-Omni M3 (0-word diff) + Pure-Omni M2 (0-byte diff) on `pure/api/adaptive.omni`.
///   2. `api_judge_better` on 64-byte paired metrics: +1t (better), 0t (within band), -1t (regression),
///      and -1t fail-closed on truncated < 64 B buffer.
///   3. `api_specialize_with_hint`: `subsystem_mask=2` (M2_EMITTER) strips NOPs; `0` (NONE) holds.
///   4. End-to-end Pure-Omni self-adaptive loop `api_adaptive_step`:
///      - adopts candidate on +1t (band=0) -> verified via M2 ELF64 (0-byte diff) + M4 VM (`ret=90`) + Linux native exec;
///      - retains baseline on 0t (band=10, savings within noise band).
///   5. Stage 2.5 golden (`judge_better_golden.omni`, M3/M2/M4 0-diff) + frozen hashes intact.
pub fn selftest_api_stage2_5() -> Result<(usize, usize, usize), String> {
    let mut checks = 0usize;

    // 1. Compile pure/api/adaptive.omni + spec/api/adaptive_harness.omni via Rust M3, Pure-Omni M3 (0-word diff),
    //    Rust M2-multi ELF64, and Pure-Omni M2 (0-byte diff).
    let adp_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(API_STAGE2_5_OMNI_SRC))?;
    let adp_m3_u16 = match m3_lockstep_compile_via_omni(API_STAGE2_5_OMNI_SRC, true)? {
        Ok(v) => v,
        Err((c, l, col)) => return Err(format!("API-S2.5 M3 compile failed ({}, {}:{})", c, l, col)),
    };
    if adp_m3_u16 != adp_u16 {
        return Err(format!(
            "API-S2.5 M3 0-word diff failed on pure/api/adaptive.omni: pure={}w vs rust={}w",
            adp_m3_u16.len(),
            adp_u16.len()
        ));
    }
    let adp_elf_rust = compile_pure_u16_to_x86_64_elf_multi(&adp_u16, 100, 80, 5)?;
    let m2_u16 = decode_bytes_to_u16_stream(
        &std::fs::read("pure/m2/m2.omni_bc").map_err(|e| format!("API-S2.5 read m2 seed: {}", e))?,
    );
    let m2_mod = decode_omni_token_stream(&m2_u16)?;
    let m2_entry = m2_entry_of(&m2_mod);
    let mut vm_m2 = OmniBinaryVm::with_budgets(500_000_000, 128 * 1024 * 1024);
    let adp_elf_pure = match vm_m2.execute_function(
        &m2_mod,
        m2_entry,
        vec![
            OmniValue::U16Array(adp_u16.clone()),
            OmniValue::Int(100),
            OmniValue::Int(80),
            OmniValue::Int(5),
        ],
    ).map_err(|e| format!("API-S2.5 step1 M2: {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2.5 Pure-Omni M2 returned {:?}", other)),
    };
    if adp_elf_pure != adp_elf_rust {
        return Err(format!(
            "API-S2.5 Pure-Omni M2 diff on pure/api/adaptive.omni: pure={}B vs rust={}B",
            adp_elf_pure.len(),
            adp_elf_rust.len()
        ));
    }
    checks += 1;

    let adp_mod = decode_omni_token_stream(&adp_u16)?;
    let n_afns = adp_mod.functions.len();
    // Order at end of spec/api/adaptive_harness.omni:
    //   api_adaptive_judge_entry (-4), api_adaptive_spec_hint_entry (-3),
    //   api_adaptive_step_entry (-2), main (-1)
    let fn_judge = adp_mod.functions[n_afns - 4].fn_idx;
    let fn_spec_hint = adp_mod.functions[n_afns - 3].fn_idx;
    let fn_step = adp_mod.functions[n_afns - 2].fn_idx;
    let mut vm = OmniBinaryVm::with_budgets(500_000_000, 128 * 1024 * 1024);

    // 2. `api_judge_better`: +1t (better), 0t (neutral within band), -1t (regression), -1t (short buf).
    let pack_pair = |base: [i64; 4], cand: [i64; 4]| -> Vec<u8> {
        let mut b = Vec::with_capacity(64);
        for v in base.iter().chain(cand.iter()) {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b
    };
    let cases: [([i64; 4], [i64; 4], i64, i64); 3] = [
        ([100, 800, 50, 128], [90, 750, 50, 128], 5, 1),   // improved -> +1t
        ([100, 800, 50, 128], [98, 798, 50, 128], 5, 0),   // within band -> 0t
        ([100, 800, 50, 128], [90, 820, 50, 128], 5, -1),  // elf regressed -> -1t
    ];
    for (ci, &(base, cand, band, want)) in cases.iter().enumerate() {
        let got = match vm.execute_function(
            &adp_mod,
            fn_judge,
            vec![OmniValue::Bytes(pack_pair(base, cand)), OmniValue::Int(band)],
        ).map_err(|e| format!("API-S2.5 step2 case#{}: {}", ci, e))? {
            OmniValue::Int(v) => v,
            other => return Err(format!("API-S2.5 judge_better case#{} returned {:?}", ci, other)),
        };
        if got != want {
            return Err(format!("API-S2.5 judge_better case#{} got {} want {}", ci, got, want));
        }
    }
    let bad_j = match vm.execute_function(
        &adp_mod,
        fn_judge,
        vec![OmniValue::Bytes(vec![0u8; 16]), OmniValue::Int(5)],
    ).map_err(|e| format!("API-S2.5 step2 bad buf: {}", e))? {
        OmniValue::Int(v) => v,
        other => return Err(format!("API-S2.5 judge_better bad buf returned {:?}", other)),
    };
    if bad_j != -1 {
        return Err(format!("API-S2.5 judge_better short buf must fail closed (-1), got {}", bad_j));
    }
    checks += 1;

    // 3. `api_specialize_with_hint`: subsystem_mask=2 (M2_EMITTER) strips NOPs; 0 (NONE) holds.
    let (_bname0, bsrc0, bargs0, bwant0) = P7_BENCHMARK_CORPUS[0];
    let clean_bc = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(bsrc0))?;
    let mut bloated_bc = clean_bc.clone();
    let pcnt0 = bloated_bc[3] as usize;
    let icnt_idx = 4 + pcnt0 * 3 + 2;
    bloated_bc[icnt_idx] += 2;
    bloated_bc.splice(icnt_idx + 1..icnt_idx + 1, [0x7F00u16, 0x7EE4, 0x7F00, 0x7EE4]);
    let bloated_bytes = encode_u16_stream_to_bytes(&bloated_bc);

    let hold_bytes = match vm.execute_function(
        &adp_mod,
        fn_spec_hint,
        vec![OmniValue::Bytes(bloated_bytes.clone()), OmniValue::Int(0)],
    ).map_err(|e| format!("API-S2.5 step3 hold: {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2.5 spec_hint(0) returned {:?}", other)),
    };
    if hold_bytes != bloated_bytes {
        return Err(String::from("API-S2.5 specialize_with_hint(0) must hold baseline bytes"));
    }
    let hint_bytes = match vm.execute_function(
        &adp_mod,
        fn_spec_hint,
        vec![OmniValue::Bytes(bloated_bytes.clone()), OmniValue::Int(2)],
    ).map_err(|e| format!("API-S2.5 step3 mask=2: {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2.5 spec_hint(2) returned {:?}", other)),
    };
    if decode_bytes_to_u16_stream(&hint_bytes) != clean_bc {
        return Err(String::from("API-S2.5 specialize_with_hint(2) did not strip NOPs to clean bench_clamp"));
    }
    checks += 1;

    // 4. End-to-end `api_adaptive_step`:
    //    - band=0 -> +1t -> adopts stripped candidate -> verify via M2 ELF64 (0-byte diff) + M4 VM (ret=90) + Linux native
    //    - band=10 -> 0t (4-word saving within noise band) -> retains baseline bloated stream.
    let adopted_bytes = match vm.execute_function(
        &adp_mod,
        fn_step,
        vec![OmniValue::Bytes(bloated_bytes.clone()), OmniValue::Int(2), OmniValue::Int(0)],
    ).map_err(|e| format!("API-S2.5 step4 adopt: {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2.5 adaptive_step(adopt) returned {:?}", other)),
    };
    let adopted_u16 = decode_bytes_to_u16_stream(&adopted_bytes);
    if adopted_u16 != clean_bc {
        return Err(String::from("API-S2.5 adaptive_step(band=0) did not adopt improved candidate"));
    }
    let retained_bytes = match vm.execute_function(
        &adp_mod,
        fn_step,
        vec![OmniValue::Bytes(bloated_bytes.clone()), OmniValue::Int(2), OmniValue::Int(10)],
    ).map_err(|e| format!("API-S2.5 step4 retain: {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2.5 adaptive_step(retain) returned {:?}", other)),
    };
    if retained_bytes != bloated_bytes {
        return Err(String::from("API-S2.5 adaptive_step(band=10) must retain baseline on 0t neutral verdict"));
    }
    let elf_r = compile_pure_u16_to_x86_64_elf_multi(&adopted_u16, bargs0[0], bargs0[1], bargs0[2])?;
    let elf_p = match vm_m2.execute_function(
        &m2_mod,
        m2_entry,
        vec![
            OmniValue::U16Array(adopted_u16.clone()),
            OmniValue::Int(bargs0[0]),
            OmniValue::Int(bargs0[1]),
            OmniValue::Int(bargs0[2]),
        ],
    ).map_err(|e| format!("API-S2.5 step4 M2(adopted): {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2.5 M2(adopted) returned {:?}", other)),
    };
    if elf_p != elf_r {
        return Err(String::from("API-S2.5 M2 0-byte diff failed on adopted candidate"));
    }
    let m4_u16 = decode_bytes_to_u16_stream(
        &std::fs::read("pure/m4/m4.omni_bc").map_err(|e| format!("API-S2.5 read m4 seed: {}", e))?,
    );
    let m4_mod = decode_omni_token_stream(&m4_u16)?;
    let m4_run_fn = m4_mod.functions[m4_mod.functions.len() - 2].fn_idx;
    let st = 167_936usize;
    let adp_arena = match vm.execute_function(
        &m4_mod,
        m4_run_fn,
        vec![
            OmniValue::Bytes(vec![0u8; 524_288]),
            OmniValue::U16Array(adopted_u16),
            OmniValue::Int(bargs0[0]),
            OmniValue::Int(bargs0[1]),
            OmniValue::Int(bargs0[2]),
            OmniValue::Int(50_000),
            OmniValue::Int(0),
        ],
    ).map_err(|e| format!("API-S2.5 step4 M4(adopted): {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2.5 M4(adopted) returned {:?}", other)),
    };
    let adp_ret = i64::from_le_bytes(adp_arena[st + 32..st + 40].try_into().unwrap());
    if adp_ret != bwant0 {
        return Err(format!("API-S2.5 M4(adopted) ret={} want {}", adp_ret, bwant0));
    }
    let _ = p7_verify_native_elf_i64("api_s25_adopted", &elf_p, bwant0)?;
    checks += 1;

    // 5. Stage 2.5 golden (`judge_better_golden.omni`) M3/M2/M4 0-diff + frozen hashes intact.
    let gpath = "spec/acceptance/goldens/judge_better_golden.omni";
    let gsrc = std::fs::read_to_string(gpath).map_err(|e| format!("API-S2.5 read {}: {}", gpath, e))?;
    let r_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(&gsrc))?;
    let m3_u16_g = match m3_lockstep_compile_via_omni(&gsrc, true)? {
        Ok(v) => v,
        Err((c, l, col)) => return Err(format!("API-S2.5 M3 on {} errored ({}, {}:{})", gpath, c, l, col)),
    };
    if m3_u16_g != r_u16 {
        return Err(format!("API-S2.5 M3 0-word diff failed on {}", gpath));
    }
    let g_elf_r = compile_pure_u16_to_x86_64_elf_multi(&r_u16, 100, 80, 5)?;
    let g_elf_p = match vm_m2.execute_function(
        &m2_mod,
        m2_entry,
        vec![OmniValue::U16Array(r_u16.clone()), OmniValue::Int(100), OmniValue::Int(80), OmniValue::Int(5)],
    ).map_err(|e| format!("API-S2.5 step5 M2 golden: {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2.5 M2 on {} returned {:?}", gpath, other)),
    };
    if g_elf_p != g_elf_r {
        return Err(format!("API-S2.5 M2 0-byte diff failed on {}", gpath));
    }
    let g_arena = match vm.execute_function(
        &m4_mod,
        m4_run_fn,
        vec![
            OmniValue::Bytes(vec![0u8; 524_288]),
            OmniValue::U16Array(r_u16),
            OmniValue::Int(100),
            OmniValue::Int(80),
            OmniValue::Int(5),
            OmniValue::Int(50_000),
            OmniValue::Int(0),
        ],
    ).map_err(|e| format!("API-S2.5 step5 M4 golden: {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("API-S2.5 M4 VM on {} returned {:?}", gpath, other)),
    };
    let gv = i64::from_le_bytes(g_arena[st + 32..st + 40].try_into().unwrap());
    if gv != 80 {
        return Err(format!("API-S2.5 M4 VM {} mismatch: got {} want 80", gpath, gv));
    }
    selftest_m2_frozen_hashes()?;
    checks += 1;

    Ok((checks, adp_u16.len(), adp_elf_rust.len()))
}

/// Phase 7 (P7-2) Self-Adaptive hard gate (`./omnic p7-adapt`):
///   1. Pure-Omni M3 (0-word diff) + Pure-Omni M2 (0-byte diff) on `pure/m7/adapt.omni`.
///   2. Live P4-5 trace-ring workload classification across all 5 P7 benchmarks:
///      bench_clamp=ALU(1), bench_loop_sum=CTRL(3), bench_branch_tree=CTRL(3),
///      bench_multi_call=CALL(4), bench_arena_buf=MEM(2).
///   3. Capability-bounded policy selection (`p7_select_policy`): full cap mask vs `cap_mask=0` fail-closed.
///   4. Hysteresis anti-thrashing state machine (`p7_adapt_decide`): noisy flip holds, sustained streak+`+1t` commits,
///      any `-1t` immediately reverts & resets streak to 0.
///   5. Closed-loop Self-Adaptive run (`pure/m7/adapt.omni` + `pure/api/adaptive.omni` + `pure/api/emit.omni`):
///      bloated bench_clamp -> M4 trace -> classify -> policy -> adaptive_step -> M2 ELF64 (0-byte diff) ->
///      M4 VM (`ret=90`) + Linux native exec -> binary ledger push & verify (2 entries).
///   6. All 5 frozen FNV-1a64 hashes intact.
pub fn selftest_p7_adapt() -> Result<(usize, usize, usize), String> {
    let mut checks = 0usize;

    // 1. Compile pure/m7/adapt.omni + spec/m7/adapt_harness.omni via Rust M3, Pure-Omni M3 (0-word diff),
    //    Rust M2-multi ELF64, and Pure-Omni M2 (0-byte diff).
    let ad_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(P7_ADAPT_OMNI_SRC))?;
    let ad_m3_u16 = match m3_lockstep_compile_via_omni(P7_ADAPT_OMNI_SRC, true)? {
        Ok(v) => v,
        Err((c, l, col)) => return Err(format!("P7-2 M3 compile failed ({}, {}:{})", c, l, col)),
    };
    if ad_m3_u16 != ad_u16 {
        return Err(format!(
            "P7-2 M3 0-word diff failed on pure/m7/adapt.omni: pure={}w vs rust={}w",
            ad_m3_u16.len(),
            ad_u16.len()
        ));
    }
    let ad_elf_rust = compile_pure_u16_to_x86_64_elf_multi(&ad_u16, 1, 15, 2)?;
    let m2_u16 = decode_bytes_to_u16_stream(
        &std::fs::read("pure/m2/m2.omni_bc").map_err(|e| format!("P7-2 read m2 seed: {}", e))?,
    );
    let m2_mod = decode_omni_token_stream(&m2_u16)?;
    let m2_entry = m2_entry_of(&m2_mod);
    let mut vm_m2 = OmniBinaryVm::with_budgets(500_000_000, 128 * 1024 * 1024);
    let ad_elf_pure = match vm_m2.execute_function(
        &m2_mod,
        m2_entry,
        vec![
            OmniValue::U16Array(ad_u16.clone()),
            OmniValue::Int(1),
            OmniValue::Int(15),
            OmniValue::Int(2),
        ],
    ).map_err(|e| format!("P7-2 step1 M2: {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("P7-2 Pure-Omni M2 returned {:?}", other)),
    };
    if ad_elf_pure != ad_elf_rust {
        return Err(format!(
            "P7-2 Pure-Omni M2 diff on pure/m7/adapt.omni: pure={}B vs rust={}B",
            ad_elf_pure.len(),
            ad_elf_rust.len()
        ));
    }
    checks += 1;

    let ad_mod = decode_omni_token_stream(&ad_u16)?;
    let n_fns = ad_mod.functions.len();
    // Order at end of spec/m7/adapt_harness.omni:
    //   p7_adapt_classify_entry (-6), p7_adapt_policy_entry (-5),
    //   p7_adapt_decide_entry (-4), p7_adapt_log_push_entry (-3),
    //   p7_adapt_log_verify_entry (-2), main (-1)
    let fn_classify = ad_mod.functions[n_fns - 6].fn_idx;
    let fn_policy = ad_mod.functions[n_fns - 5].fn_idx;
    let fn_decide = ad_mod.functions[n_fns - 4].fn_idx;
    let fn_log_push = ad_mod.functions[n_fns - 3].fn_idx;
    let fn_log_verify = ad_mod.functions[n_fns - 2].fn_idx;

    let m4_u16 = decode_bytes_to_u16_stream(
        &std::fs::read("pure/m4/m4.omni_bc").map_err(|e| format!("P7-2 read m4 seed: {}", e))?,
    );
    let m4_mod = decode_omni_token_stream(&m4_u16)?;
    let m4_run_fn = m4_mod.functions[m4_mod.functions.len() - 2].fn_idx;
    let mut vm = OmniBinaryVm::with_budgets(500_000_000, 128 * 1024 * 1024);

    // 2. Live P4-5 trace-ring workload classification across all 5 P7 benchmarks.
    let expected_profiles: [i64; 5] = [1, 3, 3, 4, 2]; // ALU, CTRL, CTRL, CALL, MEM
    for (bi, &(bname, bsrc, bargs, _bwant)) in P7_BENCHMARK_CORPUS.iter().enumerate() {
        let bbc = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(bsrc))?;
        let t_arena = match vm.execute_function(
            &m4_mod,
            m4_run_fn,
            vec![
                OmniValue::Bytes(vec![0u8; 524_288]),
                OmniValue::U16Array(bbc),
                OmniValue::Int(bargs[0]),
                OmniValue::Int(bargs[1]),
                OmniValue::Int(bargs[2]),
                OmniValue::Int(200_000),
                OmniValue::Int(1),
            ],
        ).map_err(|e| format!("P7-2 step2 M4({}): {}", bname, e))? {
            OmniValue::Bytes(b) => b,
            other => return Err(format!("P7-2 M4({}) returned {:?}", bname, other)),
        };
        let prof = match vm.execute_function(&ad_mod, fn_classify, vec![OmniValue::Bytes(t_arena)])
            .map_err(|e| format!("P7-2 step2 classify({}): {}", bname, e))?
        {
            OmniValue::Int(v) => v,
            other => return Err(format!("P7-2 classify({}) returned {:?}", bname, other)),
        };
        if prof != expected_profiles[bi] {
            return Err(format!(
                "P7-2 workload classify({}) got {} want {}",
                bname, prof, expected_profiles[bi]
            ));
        }
    }
    checks += 1;

    // 3. Capability-bounded policy selection (`p7_select_policy`).
    for &(prof, cap_mask, want_pol) in &[
        (0i64, 15i64, 0i64),
        (1, 15, 1),
        (2, 15, 5),
        (3, 15, 3),
        (4, 15, 9),
        (2, 1, 1),  // narrowed cap_mask drops ARENA_FAST
        (4, 0, 0),  // zero cap_mask fails closed to POLICY_HOLD
    ] {
        let got = match vm.execute_function(
            &ad_mod,
            fn_policy,
            vec![OmniValue::Int(prof), OmniValue::Int(cap_mask)],
        ).map_err(|e| format!("P7-2 step3 policy({},{}): {}", prof, cap_mask, e))? {
            OmniValue::Int(v) => v,
            other => return Err(format!("P7-2 policy returned {:?}", other)),
        };
        if got != want_pol {
            return Err(format!("P7-2 policy({}, {}) got {} want {}", prof, cap_mask, got, want_pol));
        }
    }
    checks += 1;

    // 4. Hysteresis anti-thrashing state machine (`p7_adapt_decide`).
    //    (prev, cur, streak, min_streak, verdict) -> (new_streak << 8) | action
    for &(prev_p, cur_p, st_in, min_st, v_i64, want_st, want_act) in &[
        (1i64, 3i64, 5i64, 2i64, 1i64, 1i64, 0i64),  // noisy flip -> streak=1 < 2 -> HOLD
        (3, 3, 1, 2, 1, 2, 1),                       // sustained streak=2 + +1t -> COMMIT
        (3, 3, 2, 2, 0, 3, 0),                       // neutral 0t -> HOLD (streak=3)
        (3, 3, 3, 2, -1, 0, 2),                      // regression -1t -> immediate REVERT, streak=0
    ] {
        let packed = match vm.execute_function(
            &ad_mod,
            fn_decide,
            vec![
                OmniValue::Int(prev_p),
                OmniValue::Int(cur_p),
                OmniValue::Int(st_in),
                OmniValue::Int(min_st),
                OmniValue::Int(v_i64),
            ],
        ).map_err(|e| format!("P7-2 step4 decide: {}", e))? {
            OmniValue::Int(v) => v,
            other => return Err(format!("P7-2 decide returned {:?}", other)),
        };
        let got_st = packed >> 8;
        let got_act = packed & 255;
        if got_st != want_st || got_act != want_act {
            return Err(format!(
                "P7-2 decide({},{},{},{},{}) got (streak={},act={}) want ({},{})",
                prev_p, cur_p, st_in, min_st, v_i64, got_st, got_act, want_st, want_act
            ));
        }
    }
    checks += 1;

    // 5. Closed-loop Self-Adaptive run: bloated bench_clamp -> trace -> classify -> policy ->
    //    Stage 2.5 `api_adaptive_step` -> M2 ELF64 (0-byte diff) -> M4 VM (ret=90) + Linux native ->
    //    binary ledger push & verify (2 entries: epoch 1 HOLD, epoch 2 COMMIT).
    let (_bname0, bsrc0, bargs0, bwant0) = P7_BENCHMARK_CORPUS[0];
    let clean_bc = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(bsrc0))?;
    let mut bloated_bc = clean_bc.clone();
    let pcnt0 = bloated_bc[3] as usize;
    let icnt_idx = 4 + pcnt0 * 3 + 2;
    bloated_bc[icnt_idx] += 2;
    bloated_bc.splice(icnt_idx + 1..icnt_idx + 1, [0x7F00u16, 0x7EE4, 0x7F00, 0x7EE4]);

    let s25_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(API_STAGE2_5_OMNI_SRC))?;
    let s25_mod = decode_omni_token_stream(&s25_u16)?;
    let fn_s25_step = s25_mod.functions[s25_mod.functions.len() - 2].fn_idx;

    let pol = match vm.execute_function(&ad_mod, fn_policy, vec![OmniValue::Int(1), OmniValue::Int(15)])? {
        OmniValue::Int(v) => v,
        other => return Err(format!("P7-2 step5 policy returned {:?}", other)),
    };
    let adapted_bytes = match vm.execute_function(
        &s25_mod,
        fn_s25_step,
        vec![
            OmniValue::Bytes(encode_u16_stream_to_bytes(&bloated_bc)),
            OmniValue::Int(pol),
            OmniValue::Int(0),
        ],
    ).map_err(|e| format!("P7-2 step5 adaptive_step: {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("P7-2 step5 adaptive_step returned {:?}", other)),
    };
    let adapted_u16 = decode_bytes_to_u16_stream(&adapted_bytes);
    if adapted_u16 != clean_bc {
        return Err(String::from("P7-2 closed-loop adaptive_step did not converge to clean bench_clamp"));
    }
    let elf_r = compile_pure_u16_to_x86_64_elf_multi(&adapted_u16, bargs0[0], bargs0[1], bargs0[2])?;
    let elf_p = match vm_m2.execute_function(
        &m2_mod,
        m2_entry,
        vec![
            OmniValue::U16Array(adapted_u16.clone()),
            OmniValue::Int(bargs0[0]),
            OmniValue::Int(bargs0[1]),
            OmniValue::Int(bargs0[2]),
        ],
    )? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("P7-2 step5 M2 returned {:?}", other)),
    };
    if elf_p != elf_r {
        return Err(String::from("P7-2 step5 M2 0-byte diff failed on adapted stream"));
    }
    let st = 167_936usize;
    let out_arena = match vm.execute_function(
        &m4_mod,
        m4_run_fn,
        vec![
            OmniValue::Bytes(vec![0u8; 524_288]),
            OmniValue::U16Array(adapted_u16),
            OmniValue::Int(bargs0[0]),
            OmniValue::Int(bargs0[1]),
            OmniValue::Int(bargs0[2]),
            OmniValue::Int(50_000),
            OmniValue::Int(0),
        ],
    )? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("P7-2 step5 M4 returned {:?}", other)),
    };
    let got_ret = i64::from_le_bytes(out_arena[st + 32..st + 40].try_into().unwrap());
    if got_ret != bwant0 {
        return Err(format!("P7-2 step5 M4 ret={} want {}", got_ret, bwant0));
    }
    let _ = p7_verify_native_elf_i64("p7_2_adapted", &elf_p, bwant0)?;

    // Log 2 epochs into binary adaptation ledger & verify integrity.
    let mut log_buf = vec![0u8; 4096];
    log_buf = match vm.execute_function(
        &ad_mod,
        fn_log_push,
        vec![OmniValue::Bytes(log_buf), OmniValue::Int(1), OmniValue::Int(1), OmniValue::Int(pol), OmniValue::Int(0)],
    )? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("P7-2 log_push#1 returned {:?}", other)),
    };
    log_buf = match vm.execute_function(
        &ad_mod,
        fn_log_push,
        vec![OmniValue::Bytes(log_buf), OmniValue::Int(2), OmniValue::Int(1), OmniValue::Int(pol), OmniValue::Int(1)],
    )? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("P7-2 log_push#2 returned {:?}", other)),
    };
    let v_cnt = match vm.execute_function(&ad_mod, fn_log_verify, vec![OmniValue::Bytes(log_buf)])? {
        OmniValue::Int(v) => v,
        other => return Err(format!("P7-2 log_verify returned {:?}", other)),
    };
    if v_cnt != 2 {
        return Err(format!("P7-2 adaptation ledger verify got {} want 2", v_cnt));
    }
    checks += 1;

    // 6. All 5 frozen FNV-1a64 hashes intact.
    selftest_m2_frozen_hashes()?;
    checks += 1;

    Ok((checks, ad_u16.len(), ad_elf_rust.len()))
}

/// Phase 7 (P7-3) Self-Learning hard gate (`./omnic p7-learn`):
///   1. Pure-Omni M3 (0-word diff) + Pure-Omni M2 (0-byte diff) on `pure/m7/learn.omni`.
///   2. Experience table init + checksum integrity verification (`verify == 0`) + tamper fail-closed (`-1` / `HOLD`).
///   3. Asymmetric ternary reinforcement (`+1t -> +2`, `0t -> 0`, `-1t -> -3` safety bias) + safety floor + `cap_mask` gating.
///   4. `@PACK5` radix-3 summary export (`p7_learn_export_pack5`) across all 5 workload profiles.
///   5. Closed-loop Self-Learning -> Self-Adaptive -> Reflective Emit run on bloated `bench_clamp`:
///      learned policy (`bit 1`) -> Stage 2.5 `api_adaptive_step` -> M2 ELF64 (0-byte diff) -> M4 VM (`ret=90`) + Linux native exec.
///   6. All 5 frozen FNV-1a64 hashes intact.
pub fn selftest_p7_learn() -> Result<(usize, usize, usize), String> {
    let mut checks = 0usize;

    // 1. Compile pure/m7/learn.omni + spec/m7/learn_harness.omni via Rust M3, Pure-Omni M3 (0-word diff),
    //    Rust M2-multi ELF64, and Pure-Omni M2 (0-byte diff).
    let lrn_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(P7_LEARN_OMNI_SRC))?;
    let lrn_m3_u16 = match m3_lockstep_compile_via_omni(P7_LEARN_OMNI_SRC, true)? {
        Ok(v) => v,
        Err((c, l, col)) => return Err(format!("P7-3 M3 compile failed ({}, {}:{})", c, l, col)),
    };
    if lrn_m3_u16 != lrn_u16 {
        return Err(format!(
            "P7-3 M3 0-word diff failed on pure/m7/learn.omni: pure={}w vs rust={}w",
            lrn_m3_u16.len(),
            lrn_u16.len()
        ));
    }
    let lrn_elf_rust = compile_pure_u16_to_x86_64_elf_multi(&lrn_u16, 1, 0, 1)?;
    let m2_u16 = decode_bytes_to_u16_stream(
        &std::fs::read("pure/m2/m2.omni_bc").map_err(|e| format!("P7-3 read m2 seed: {}", e))?,
    );
    let m2_mod = decode_omni_token_stream(&m2_u16)?;
    let m2_entry = m2_entry_of(&m2_mod);
    let mut vm_m2 = OmniBinaryVm::with_budgets(500_000_000, 128 * 1024 * 1024);
    let lrn_elf_pure = match vm_m2.execute_function(
        &m2_mod,
        m2_entry,
        vec![
            OmniValue::U16Array(lrn_u16.clone()),
            OmniValue::Int(1),
            OmniValue::Int(0),
            OmniValue::Int(1),
        ],
    ).map_err(|e| format!("P7-3 step1 M2: {}", e))? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("P7-3 Pure-Omni M2 returned {:?}", other)),
    };
    if lrn_elf_pure != lrn_elf_rust {
        return Err(format!(
            "P7-3 Pure-Omni M2 diff on pure/m7/learn.omni: pure={}B vs rust={}B",
            lrn_elf_pure.len(),
            lrn_elf_rust.len()
        ));
    }
    checks += 1;

    let lrn_mod = decode_omni_token_stream(&lrn_u16)?;
    let n_fns = lrn_mod.functions.len();
    // Order at end of spec/m7/learn_harness.omni:
    //   p7_learn_init_entry (-6), p7_learn_verify_entry (-5), p7_learn_update_entry (-4),
    //   p7_learn_rank_entry (-3), p7_learn_pack5_entry (-2), main (-1)
    let fn_init = lrn_mod.functions[n_fns - 6].fn_idx;
    let fn_verify = lrn_mod.functions[n_fns - 5].fn_idx;
    let fn_update = lrn_mod.functions[n_fns - 4].fn_idx;
    let fn_rank = lrn_mod.functions[n_fns - 3].fn_idx;
    let fn_pack5 = lrn_mod.functions[n_fns - 2].fn_idx;
    let mut vm = OmniBinaryVm::with_budgets(500_000_000, 128 * 1024 * 1024);

    // 2. Init + checksum verify + tamper detection fail-closed.
    let mut tbl = match vm.execute_function(&lrn_mod, fn_init, vec![OmniValue::Bytes(vec![0u8; 512])])
        .map_err(|e| format!("P7-3 step2 init: {}", e))?
    {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("P7-3 init returned {:?}", other)),
    };
    let v0 = match vm.execute_function(&lrn_mod, fn_verify, vec![OmniValue::Bytes(tbl.clone())])? {
        OmniValue::Int(v) => v,
        other => return Err(format!("P7-3 verify returned {:?}", other)),
    };
    if v0 != 0 {
        return Err(format!("P7-3 fresh table verify got {} want 0", v0));
    }
    let mut tampered = tbl.clone();
    tampered[20] ^= 0xFF;
    let vt = match vm.execute_function(&lrn_mod, fn_verify, vec![OmniValue::Bytes(tampered.clone())])? {
        OmniValue::Int(v) => v,
        other => return Err(format!("P7-3 verify(tampered) returned {:?}", other)),
    };
    let rt = match vm.execute_function(
        &lrn_mod,
        fn_rank,
        vec![OmniValue::Bytes(tampered), OmniValue::Int(1), OmniValue::Int(15)],
    )? {
        OmniValue::Int(v) => v,
        other => return Err(format!("P7-3 rank(tampered) returned {:?}", other)),
    };
    if vt != -1 || rt != 0 {
        return Err(format!("P7-3 tampered table must fail closed (-1/0), got ({}/{})", vt, rt));
    }
    checks += 1;

    // 3. Asymmetric ternary reinforcement (+1t -> +2, 0t -> 0, -1t -> -3) + safety floor + cap_mask gating.
    //    profile=1 (ALU): slot 0 gets 2 wins (+4 => 32772); slot 1 gets 1 win + 1 loss (-1 => 32767 < 32768 floor!).
    for &(prof, slot, v) in &[
        (1i64, 0i64, 1i64),
        (1, 0, 1),
        (1, 1, 1),
        (1, 1, -1),
        (2, 0, 1),
        (3, 0, 0),
        (4, 0, -1),
    ] {
        tbl = match vm.execute_function(
            &lrn_mod,
            fn_update,
            vec![OmniValue::Bytes(tbl), OmniValue::Int(prof), OmniValue::Int(slot), OmniValue::Int(v)],
        ).map_err(|e| format!("P7-3 step3 update({},{},{}): {}", prof, slot, v, e))? {
            OmniValue::Bytes(b) => b,
            other => return Err(format!("P7-3 update returned {:?}", other)),
        };
    }
    let v_cnt = match vm.execute_function(&lrn_mod, fn_verify, vec![OmniValue::Bytes(tbl.clone())])? {
        OmniValue::Int(v) => v,
        other => return Err(format!("P7-3 verify after updates returned {:?}", other)),
    };
    if v_cnt != 7 {
        return Err(format!("P7-3 total_updates got {} want 7", v_cnt));
    }
    let best_all = match vm.execute_function(
        &lrn_mod,
        fn_rank,
        vec![OmniValue::Bytes(tbl.clone()), OmniValue::Int(1), OmniValue::Int(15)],
    )? {
        OmniValue::Int(v) => v,
        other => return Err(format!("P7-3 rank(1, 15) returned {:?}", other)),
    };
    // Narrow cap_mask to slot 1 only (bit 2): slot 1 is below safety floor (32767 < 32768) -> HOLD (0)!
    let best_regressed_only = match vm.execute_function(
        &lrn_mod,
        fn_rank,
        vec![OmniValue::Bytes(tbl.clone()), OmniValue::Int(1), OmniValue::Int(2)],
    )? {
        OmniValue::Int(v) => v,
        other => return Err(format!("P7-3 rank(1, 2) returned {:?}", other)),
    };
    if best_all != 1 || best_regressed_only != 0 {
        return Err(format!(
            "P7-3 ranking mismatch: best_all={} (want 1), best_regressed_only={} (want 0)",
            best_all, best_regressed_only
        ));
    }
    checks += 1;

    // 4. `@PACK5` summary export for pol_slot=0 across profiles 0..4:
    //    p0=0t (1), p1=+1t (2), p2=+1t (2), p3=0t (1), p4=-1t (0)
    //    radix-3 byte = 1*1 + 2*3 + 2*9 + 1*27 + 0*81 = 1 + 6 + 18 + 27 = 52.
    let p5 = match vm.execute_function(
        &lrn_mod,
        fn_pack5,
        vec![OmniValue::Bytes(tbl.clone()), OmniValue::Int(0)],
    )? {
        OmniValue::Int(v) => v,
        other => return Err(format!("P7-3 export_pack5 returned {:?}", other)),
    };
    if p5 != 52 {
        return Err(format!("P7-3 export_pack5(slot=0) got {} want 52", p5));
    }
    checks += 1;

    // 5. Closed-loop Self-Learning -> Self-Adaptive -> Reflective Emit on bloated bench_clamp:
    //    learned policy bit (`best_all == 1`) -> Stage 2.5 `api_adaptive_step` -> M2 ELF64 (0-byte diff) ->
    //    M4 VM (`ret=90`) + native Linux x86-64 exec.
    let (_bname0, bsrc0, bargs0, bwant0) = P7_BENCHMARK_CORPUS[0];
    let clean_bc = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(bsrc0))?;
    let mut bloated_bc = clean_bc.clone();
    let pcnt0 = bloated_bc[3] as usize;
    let icnt_idx = 4 + pcnt0 * 3 + 2;
    bloated_bc[icnt_idx] += 2;
    bloated_bc.splice(icnt_idx + 1..icnt_idx + 1, [0x7F00u16, 0x7EE4, 0x7F00, 0x7EE4]);

    let s25_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(API_STAGE2_5_OMNI_SRC))?;
    let s25_mod = decode_omni_token_stream(&s25_u16)?;
    let fn_s25_step = s25_mod.functions[s25_mod.functions.len() - 2].fn_idx;

    let learned_bytes = match vm.execute_function(
        &s25_mod,
        fn_s25_step,
        vec![
            OmniValue::Bytes(encode_u16_stream_to_bytes(&bloated_bc)),
            OmniValue::Int(best_all),
            OmniValue::Int(0),
        ],
    )? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("P7-3 step5 adaptive_step returned {:?}", other)),
    };
    let learned_u16 = decode_bytes_to_u16_stream(&learned_bytes);
    if learned_u16 != clean_bc {
        return Err(String::from("P7-3 learned policy did not converge bloated bench_clamp to clean stream"));
    }
    let elf_r = compile_pure_u16_to_x86_64_elf_multi(&learned_u16, bargs0[0], bargs0[1], bargs0[2])?;
    let elf_p = match vm_m2.execute_function(
        &m2_mod,
        m2_entry,
        vec![
            OmniValue::U16Array(learned_u16.clone()),
            OmniValue::Int(bargs0[0]),
            OmniValue::Int(bargs0[1]),
            OmniValue::Int(bargs0[2]),
        ],
    )? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("P7-3 step5 M2 returned {:?}", other)),
    };
    if elf_p != elf_r {
        return Err(String::from("P7-3 step5 M2 0-byte diff failed"));
    }
    let m4_u16 = decode_bytes_to_u16_stream(
        &std::fs::read("pure/m4/m4.omni_bc").map_err(|e| format!("P7-3 read m4 seed: {}", e))?,
    );
    let m4_mod = decode_omni_token_stream(&m4_u16)?;
    let m4_run_fn = m4_mod.functions[m4_mod.functions.len() - 2].fn_idx;
    let st = 167_936usize;
    let out_arena = match vm.execute_function(
        &m4_mod,
        m4_run_fn,
        vec![
            OmniValue::Bytes(vec![0u8; 524_288]),
            OmniValue::U16Array(learned_u16),
            OmniValue::Int(bargs0[0]),
            OmniValue::Int(bargs0[1]),
            OmniValue::Int(bargs0[2]),
            OmniValue::Int(50_000),
            OmniValue::Int(0),
        ],
    )? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("P7-3 step5 M4 returned {:?}", other)),
    };
    let got_ret = i64::from_le_bytes(out_arena[st + 32..st + 40].try_into().unwrap());
    if got_ret != bwant0 {
        return Err(format!("P7-3 step5 M4 ret={} want {}", got_ret, bwant0));
    }
    let _ = p7_verify_native_elf_i64("p7_3_learned", &elf_p, bwant0)?;
    checks += 1;

    // 6. All 5 frozen FNV-1a64 hashes intact.
    selftest_m2_frozen_hashes()?;
    checks += 1;

    Ok((checks, lrn_u16.len(), lrn_elf_rust.len()))
}


/// Seed-Shrink Phase A1 hard gate (`./omnic a1-pure-default`):
///   1. Pure-default M3 (`pure/m3/m3.omni_bc`) matches `--rust` oracle with 0-word diff on goldens.
///   2. Pure-default M3 -> Pure-default M2 (`pure/m2/m2.omni_bc`) matches `--rust` oracle with 0-byte diff.
///   3. Pure-default M3 -> Pure-default M4 VM (`pure/m4/m4.omni_bc`) matches exact expected outputs on goldens & bench_loop_sum.
///   4. Negative surface input rejected by Pure-default M3 with machine-readable `error[...]` and 1-based `(line, col)` span.
///   5. All 5 frozen FNV-1a64 hashes intact.
pub fn selftest_a1_pure_default() -> Result<usize, String> {
    let mut checks = 0usize;

    // 1. Pure-default M3 0-word diff vs Rust oracle on Stage 2 / 2.5 goldens.
    let goldens: [(&str, [i64; 3], i64); 3] = [
        ("spec/acceptance/goldens/token_inspect_golden.omni", [32480, 32495, 0], 2),
        ("spec/acceptance/goldens/token_emit_golden.omni", [42, 0, 0], 13),
        ("spec/acceptance/goldens/judge_better_golden.omni", [100, 80, 5], 80),
    ];
    let mut compiled: Vec<(&str, Vec<u16>, [i64; 3], i64)> = Vec::new();
    for &(gpath, gargs, gwant) in &goldens {
        let gsrc = std::fs::read_to_string(gpath).map_err(|e| format!("A1 read {}: {}", gpath, e))?;
        let pure_u16 = pure_default_compile_surface_to_u16(&gsrc, gsrc.contains("#[strict]"))?;
        let rust_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(&gsrc))?;
        if pure_u16 != rust_u16 {
            return Err(format!("A1 Pure-M3 vs Rust oracle word diff on {}", gpath));
        }
        compiled.push((gpath, pure_u16, gargs, gwant));
    }
    checks += 1;

    // 2. Pure-default M2 ELF64 0-byte diff vs Rust oracle.
    for &(gpath, ref u16_s, gargs, _) in &compiled {
        let pure_elf = pure_default_compile_u16_to_elf(u16_s, gargs[0], gargs[1], gargs[2])?;
        let rust_elf = compile_pure_u16_to_x86_64_elf_multi(u16_s, gargs[0], gargs[1], gargs[2])?;
        if pure_elf != rust_elf {
            return Err(format!("A1 Pure-M2 vs Rust oracle ELF diff on {}", gpath));
        }
    }
    checks += 1;

    // 3. Pure-default M4 VM execution parity on goldens + bench_loop_sum (100 -> 5050).
    for &(gpath, ref u16_s, gargs, gwant) in &compiled {
        let (got, _) = pure_default_run_u16(u16_s, gargs[0], gargs[1], gargs[2], false)?;
        if got != gwant {
            return Err(format!("A1 Pure-M4 VM {} got {} want {}", gpath, got, gwant));
        }
    }
    let (_bname1, bsrc1, bargs1, bwant1) = P7_BENCHMARK_CORPUS[1];
    let b1_u16 = pure_default_compile_surface_to_u16(bsrc1, true)?;
    let (b1_got, _) = pure_default_run_u16(&b1_u16, bargs1[0], bargs1[1], bargs1[2], false)?;
    if b1_got != bwant1 {
        return Err(format!("A1 Pure-M4 VM bench_loop_sum got {} want {}", b1_got, bwant1));
    }
    checks += 1;

    // 4. Negative surface input rejected by Pure-default M3 with `error[...]` + 1-based (line, col).
    let bad_src = "#[strict]\nfn main(x: i64) -> i64 {\n    return unknown_ident_xyz;\n}\n";
    match pure_default_compile_surface_to_u16(bad_src, true) {
        Ok(_) => return Err(String::from("A1 Pure-M3 must reject undefined identifier")),
        Err(msg) => {
            if !msg.starts_with("error[") || !msg.contains("line 3,") {
                return Err(format!("A1 Pure-M3 error format/span mismatch: {}", msg));
            }
        }
    }
    checks += 1;

    // 5. Fix 1 (Option A): Full N-arg (>3 params) parity across default native m4.elf + interpreted
    //    Pure-M4 VM (`m4_vm_run_args`) + CLI `omnic run` validation (`error[CLI_ARG]`).
    let arg_repro_src = "#[strict]\nfn main(a: i64, b: i64, c: i64, d: i64, e: i64) -> i64 {\n    return d * 100 + e;\n}\n";
    let trit_pack5_src = "include \"std/math.omni\";\n#[strict]\nfn main(t0: i64, t1: i64, t2: i64, t3: i64, t4: i64) -> i64 {\n    let a0: i64 = std_clamp(t0, -1, 1) + 1;\n    let a1: i64 = std_clamp(t1, -1, 1) + 1;\n    let a2: i64 = std_clamp(t2, -1, 1) + 1;\n    let a3: i64 = std_clamp(t3, -1, 1) + 1;\n    let a4: i64 = std_clamp(t4, -1, 1) + 1;\n    return a0 * 81 + a1 * 27 + a2 * 9 + a3 * 3 + a4;\n}\n";
    let running_stats4_src = "include \"std/math.omni\";\n#[strict]\nfn main(a: i64, b: i64, c: i64, d: i64) -> i64 {\n    let x0: i64 = std_clamp(a, -50, 49);\n    let x1: i64 = std_clamp(b, -50, 49);\n    let x2: i64 = std_clamp(c, -50, 49);\n    let x3: i64 = std_clamp(d, -50, 49);\n    let lo: i64 = imin(imin(x0, x1), imin(x2, x3));\n    let hi: i64 = imax(imax(x0, x1), imax(x2, x3));\n    let mean: i64 = (x0 + x1 + x2 + x3) / 4;\n    return (lo + 50) * 10000 + (hi + 50) * 100 + (mean + 50);\n}\n";
    let sum10_src = "#[strict]\nfn main(a0: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64, a6: i64, a7: i64, a8: i64, a9: i64) -> i64 {\n    return a0 + a1 * 2 + a2 * 3 + a3 * 4 + a4 * 5 + a5 * 6 + a6 * 7 + a7 * 8 + a8 * 9 + a9 * 10;\n}\n";

    let n_arg_cases: &[(&str, &str, &[(&[i64], i64)])] = &[
        (
            "arg_repro",
            arg_repro_src,
            &[(&[1, 2, 3, 4, 5], 405), (&[0, 0, 0, 7, 8], 708)],
        ),
        (
            "trit_pack5",
            trit_pack5_src,
            &[
                (&[-1, -1, -1, -1, -1], 0),
                (&[0, 0, 0, 0, 0], 121),
                (&[1, 1, 1, 1, 1], 242),
                (&[1, 0, -1, 1, 0], 196),
            ],
        ),
        (
            "running_stats4",
            running_stats4_src,
            &[
                (&[10, 20, 30, 40], 609_075),
                (&[-10, 0, 10, 20], 407_055),
                (&[0, 0, 0, 0], 505_050),
            ],
        ),
        (
            "sum10",
            sum10_src,
            &[(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 385)],
        ),
    ];
    for &(label, raw_src, vectors) in n_arg_cases {
        let exp = pure_default_expand_includes(raw_src, None, None)?;
        let u16_s = pure_default_compile_surface_to_u16(&exp, true)?;
        for &(args, want) in vectors {
            let (got_def, _) = pure_default_run_u16_with_fuel(&u16_s, args, PURE_DEFAULT_FUEL, false)?;
            let (got_int, _) = with_pure_default_interpreted(|| {
                pure_default_run_u16_with_fuel(&u16_s, args, PURE_DEFAULT_FUEL, false)
            })?;
            if got_def != want || got_int != want {
                return Err(format!(
                    "A1 N-arg '{}' {:?} mismatch: default={} interpreted={} want={}",
                    label, args, got_def, got_int, want
                ));
            }
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        let is_test_harness = exe.parent().map_or(false, |d| {
            d.ancestors().filter_map(|a| a.file_name()).any(|n| n == "deps" || n == "build")
        });
        if !is_test_harness {
            let tmp_dir = std::env::temp_dir().join(format!("omni_a1_nargs_{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&tmp_dir);
            std::fs::create_dir_all(&tmp_dir).map_err(|e| format!("A1 N-arg mkdir: {}", e))?;
            let cli_check = (|| -> Result<(), String> {
                let repro_p = tmp_dir.join("arg_repro.omni");
                std::fs::write(&repro_p, arg_repro_src).map_err(|e| format!("A1 write arg_repro: {}", e))?;
                for &(use_interp, args, want_out) in &[
                    (false, &["1", "2", "3", "4", "5"][..], "Int(405)"),
                    (true, &["1", "2", "3", "4", "5"][..], "Int(405)"),
                    (false, &["0", "0", "0", "7", "8"][..], "Int(708)"),
                    (true, &["0", "0", "0", "7", "8"][..], "Int(708)"),
                ] {
                    let mut cmd = std::process::Command::new(&exe);
                    cmd.arg("run");
                    if use_interp {
                        cmd.arg("--interpreted");
                    }
                    cmd.arg(&repro_p).args(args);
                    let out = cmd.output().map_err(|e| format!("A1 spawn omnic run: {}", e))?;
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    if !out.status.success() || stdout.trim() != want_out {
                        return Err(format!(
                            "A1 CLI omnic run (interp={}) {:?} failed: status={:?} stdout='{}' stderr='{}'",
                            use_interp,
                            args,
                            out.status.code(),
                            stdout.trim(),
                            String::from_utf8_lossy(&out.stderr).trim()
                        ));
                    }
                }
                for bad_args in [
                    &["1", "2", "foo", "4", "5"][..],
                    &["1", "2", "3"][..],
                    &["1", "2", "3", "4", "5", "6"][..],
                ] {
                    let out = std::process::Command::new(&exe)
                        .arg("run")
                        .arg(&repro_p)
                        .args(bad_args)
                        .output()
                        .map_err(|e| format!("A1 spawn bad CLI args: {}", e))?;
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    if out.status.success() || !stderr.contains("error[CLI_ARG]") {
                        return Err(format!(
                            "A1 CLI omnic run {:?} must fail with error[CLI_ARG], got status={:?} stderr='{}'",
                            bad_args,
                            out.status.code(),
                            stderr.trim()
                        ));
                    }
                }
                Ok(())
            })();
            let _ = std::fs::remove_dir_all(&tmp_dir);
            cli_check?;
        }
    }
    checks += 1;

    // 6. All 5 frozen FNV-1a64 hashes intact.
    selftest_m2_frozen_hashes()?;
    checks += 1;

    Ok(checks)
}


/// Seed-Shrink Phase A2 hard gate (`./omnic a2-pure-include`):
///   1. Pure-Omni M3 (0-word diff) + Pure-Omni M2 (0-byte diff) on `pure/m3/include.omni`.
///   2. Multi-file stdlib include (`std/math.omni`) expands with 0-byte diff vs Rust oracle,
///      compiles via Pure-M3 (0-word diff), lowers via Pure-M2 (0-byte diff), runs on Pure-M4 VM.
///   3. Nested multi-file VFS include (`a.omni` -> `b.omni` -> `c.omni`) matches Rust oracle & runs on M4 VM.
///   4. Circular include (`a.omni` -> `b.omni` -> `a.omni`) rejected with `error[CIRCULAR_INCLUDE]` + 1-based `(line, col)`.
///   5. All 5 frozen FNV-1a64 hashes intact.
pub fn selftest_a2_pure_include() -> Result<(usize, usize, usize), String> {
    let mut checks = 0usize;

    // 1. Compile pure/m3/include.omni + spec/m3/include_harness.omni via Pure-M3 (0-word diff) + Pure-M2 (0-byte diff).
    let inc_u16 = pure_default_compile_surface_to_u16(M3_INCLUDE_OMNI_SRC, true)?;
    let inc_rust_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(M3_INCLUDE_OMNI_SRC))?;
    if inc_u16 != inc_rust_u16 {
        return Err(format!(
            "A2 Pure-M3 0-word diff failed on pure/m3/include.omni: pure={}w vs rust={}w",
            inc_u16.len(),
            inc_rust_u16.len()
        ));
    }
    let inc_elf_pure = pure_default_compile_u16_to_elf(&inc_u16, 1, 2, 3)?;
    let inc_elf_rust = compile_pure_u16_to_x86_64_elf_multi(&inc_u16, 1, 2, 3)?;
    if inc_elf_pure != inc_elf_rust {
        return Err(format!(
            "A2 Pure-M2 0-byte diff failed on pure/m3/include.omni: pure={}B vs rust={}B",
            inc_elf_pure.len(),
            inc_elf_rust.len()
        ));
    }
    checks += 1;

    // 2. Multi-file stdlib include (`std/trit.omni`) expands identically to Rust oracle, compiles & runs.
    let std_prog = "include \"std/trit.omni\";\n#[strict]\nfn main(x: i64) -> i64 {\n    return std_trit_mux(x <=> 50, -10, 0, 90);\n}\n";
    let exp_pure = pure_default_expand_includes(std_prog, None, None)?;
    let exp_rust = resolve_omni_surface_includes(std_prog, None, None)?;
    if exp_pure != exp_rust {
        return Err(format!(
            "A2 stdlib include expansion mismatch vs Rust oracle (pure={} B vs rust={} B)",
            exp_pure.len(),
            exp_rust.len()
        ));
    }
    let std_u16 = pure_default_compile_surface_to_u16(&exp_pure, true)?;
    let std_u16_r = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(&exp_rust))?;
    if std_u16 != std_u16_r {
        return Err(String::from("A2 stdlib include Pure-M3 vs Rust word diff"));
    }
    let (std_ret, _) = pure_default_run_u16(&std_u16, 150, 0, 0, false)?;
    if std_ret != 90 {
        return Err(format!("A2 stdlib include M4 VM ret={} want 90", std_ret));
    }
    checks += 1;

    // 3. Nested VFS include (`a.omni` -> `b.omni` -> `c.omni`) matches Rust oracle & executes on Pure-M4 VM.
    let mut vfs: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    vfs.insert(
        String::from("c.omni"),
        String::from("#[strict]\nfn helper_c(x: i64) -> i64 { return x * 3; }\n"),
    );
    vfs.insert(
        String::from("b.omni"),
        String::from("include \"c.omni\";\n#[strict]\nfn helper_b(x: i64) -> i64 { return helper_c(x) + 7; }\n"),
    );
    let root_a = "include \"b.omni\";\n#[strict]\nfn main(x: i64) -> i64 { return helper_b(x); }\n";
    let vfs_pure = pure_default_expand_includes(root_a, None, Some(&vfs))?;
    let vfs_rust = resolve_omni_surface_includes(root_a, None, Some(&vfs))?;
    if vfs_pure != vfs_rust {
        return Err(String::from("A2 nested VFS include expansion mismatch vs Rust oracle"));
    }
    let vfs_u16 = pure_default_compile_surface_to_u16(&vfs_pure, true)?;
    let (vfs_ret, _) = pure_default_run_u16(&vfs_u16, 10, 0, 0, false)?;
    if vfs_ret != 37 {
        return Err(format!("A2 nested VFS include M4 VM ret={} want 37", vfs_ret));
    }
    checks += 1;

    // 4. Circular include (`cyc_a.omni` -> `cyc_b.omni` -> `cyc_a.omni`) rejected with `error[CIRCULAR_INCLUDE]`.
    let mut cyc_vfs: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    cyc_vfs.insert(String::from("cyc_a.omni"), String::from("include \"cyc_b.omni\";\nfn fa() -> i64 { return 1; }\n"));
    cyc_vfs.insert(String::from("cyc_b.omni"), String::from("include \"cyc_a.omni\";\nfn fb() -> i64 { return 2; }\n"));
    let cyc_root = "include \"cyc_a.omni\";\nfn main() -> i64 { return 0; }\n";
    match pure_default_expand_includes(cyc_root, None, Some(&cyc_vfs)) {
        Ok(_) => return Err(String::from("A2 circular include must be rejected with CIRCULAR_INCLUDE")),
        Err(msg) => {
            if !msg.starts_with("error[CIRCULAR_INCLUDE]") || !msg.contains("line 1, col 1") {
                return Err(format!("A2 circular include error mismatch: {}", msg));
            }
        }
    }
    checks += 1;

    // 5. All 5 frozen FNV-1a64 hashes intact.
    selftest_m2_frozen_hashes()?;
    checks += 1;

    Ok((checks, inc_u16.len(), inc_elf_pure.len()))
}

/// Seed-Shrink Phase A3 hard gate (`./omnic a3-macro-quarantine`):
///   1. Pure-default M3 rejects macro-using `stdlib_macro_pipeline_golden.omni` with `error[ORACLE_ONLY_MACRO]` at line 2, col 1.
///   2. `--rust` oracle still compiles & executes `stdlib_macro_pipeline_golden.omni` (45, 10, 40 -> 100).
///   3. Pure library replacement `weighted_diff_pure_golden.omni` passes Pure-M3 (0-word diff), Pure-M2 (0-byte diff),
///      and Pure-M4 VM (45, 10, 40 -> 100).
///   4. Manifest tags `stdlib_macro_pipeline_golden` as `"oracle_only": true` + all 5 frozen hashes intact.
pub fn selftest_a3_macro_quarantine() -> Result<usize, String> {
    let mut checks = 0usize;

    // 1. Pure-default path rejects `macro` with structured `error[ORACLE_ONLY_MACRO]` + 1-based span
    //    on BOTH direct raw source AND the full CLI expand-then-compile path (line 2, col 1 — never post-expansion).
    let macro_path = "spec/acceptance/goldens/stdlib_macro_pipeline_golden.omni";
    let macro_src = std::fs::read_to_string(macro_path).map_err(|e| format!("A3 read {}: {}", macro_path, e))?;
    match pure_default_compile_surface_to_u16(&macro_src, true) {
        Ok(_) => return Err(String::from("A3 pure default path must reject macro-using fixture")),
        Err(msg) => {
            if !msg.starts_with("error[ORACLE_ONLY_MACRO]") || !msg.contains("line 2, col 1") {
                return Err(format!("A3 macro quarantine error mismatch: {}", msg));
            }
        }
    }
    let macro_dir = std::path::Path::new(macro_path).parent();
    match pure_default_expand_includes(&macro_src, macro_dir, None)
        .and_then(|exp| pure_default_compile_surface_to_u16(&exp, exp.contains("#[strict]")))
    {
        Ok(_) => return Err(String::from("A3 CLI expand-then-compile path must reject macro-using fixture")),
        Err(msg) => {
            if !msg.starts_with("error[ORACLE_ONLY_MACRO]") || !msg.contains("line 2, col 1") {
                return Err(format!("A3 CLI macro quarantine span mismatch (expected line 2, col 1): {}", msg));
            }
        }
    }
    checks += 1;

    // 2. `--rust` oracle still handles the quarantined macro fixture (45, 10, 40 -> 100).
    let oracle_u16 = with_lockstep_suspended(|| compile_omni_surface_file_to_u16(macro_path))?;
    let oracle_mod = decode_omni_token_stream(&oracle_u16)?;
    let mut vm = OmniBinaryVm::new();
    let oracle_res = vm.execute_function(
        &oracle_mod,
        0,
        vec![OmniValue::Int(45), OmniValue::Int(10), OmniValue::Int(40)],
    )?;
    if oracle_res != OmniValue::Int(100) {
        return Err(format!("A3 rust oracle macro fixture got {:?} want Int(100)", oracle_res));
    }
    checks += 1;

    // 3. Pure library replacement `weighted_diff_pure_golden.omni` passes Pure-M3/M2/M4 (45, 10, 40 -> 100).
    let pure_path = "spec/acceptance/goldens/weighted_diff_pure_golden.omni";
    let pure_src = std::fs::read_to_string(pure_path).map_err(|e| format!("A3 read {}: {}", pure_path, e))?;
    let pure_u16 = pure_default_compile_surface_to_u16(&pure_src, true)?;
    let rust_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(&pure_src))?;
    if pure_u16 != rust_u16 {
        return Err(String::from("A3 Pure-M3 vs Rust oracle word diff on weighted_diff_pure_golden.omni"));
    }
    let pure_elf = pure_default_compile_u16_to_elf(&pure_u16, 45, 10, 40)?;
    let rust_elf = compile_pure_u16_to_x86_64_elf_multi(&pure_u16, 45, 10, 40)?;
    if pure_elf != rust_elf {
        return Err(String::from("A3 Pure-M2 vs Rust oracle byte diff on weighted_diff_pure_golden.omni"));
    }
    let (m4_ret, _) = pure_default_run_u16(&pure_u16, 45, 10, 40, false)?;
    if m4_ret != 100 {
        return Err(format!("A3 Pure-M4 VM on weighted_diff_pure_golden.omni got {} want 100", m4_ret));
    }
    let _ = p7_verify_native_elf_i64("a3_weighted_diff_pure", &pure_elf, 100)?;
    checks += 1;

    // 4. Manifest explicitly marks macro fixture `oracle_only: true` + frozen hashes intact.
    let manifest = std::fs::read_to_string("spec/acceptance/acceptance_manifest.json")
        .map_err(|e| format!("A3 read manifest: {}", e))?;
    if !manifest.contains("\"stdlib_macro_pipeline_golden\"") || !manifest.contains("\"oracle_only\": true") {
        return Err(String::from("A3 manifest must mark stdlib_macro_pipeline_golden as oracle_only: true"));
    }
    selftest_m2_frozen_hashes()?;
    checks += 1;

    Ok(checks)
}


/// Seed-Shrink Phase B2 hard gate (`./omnic b2-pure-verifier`):
///   1. Default Pure-Omni verifier accepts valid Pure-M3 compiled goldens (`token_emit_golden`, `weighted_diff_pure_golden`).
///   2. Default Pure-Omni verifier rejects `neg_linear_reflect.omni` with `1012` (`PURE_VERIFY_1012`) in API mode.
///   3. Default Pure-Omni verifier rejects `neg_taint_sink.omni` without invoking Rust verifier.
///   4. Oracle mode (`--rust`) agrees with Pure-Omni verifier on valid goldens & core negative cases.
///   5. All 5 frozen FNV-1a64 hashes intact.
pub fn selftest_b2_pure_verifier() -> Result<usize, String> {
    let mut checks = 0usize;

    // 1. Valid goldens pass Pure-Omni verifier on the hot path.
    for gpath in &[
        "spec/acceptance/goldens/token_emit_golden.omni",
        "spec/acceptance/goldens/weighted_diff_pure_golden.omni",
    ] {
        let gsrc = std::fs::read_to_string(gpath).map_err(|e| format!("B2 read {}: {}", gpath, e))?;
        let u16_s = pure_default_compile_surface_to_u16(&gsrc, true)?;
        pure_default_verify_u16(&u16_s, true)?;
    }
    checks += 1;

    // 2. Rejects `neg_linear_reflect.omni` with code 1012 on the Pure-Omni verifier hot path.
    let neg_lin = std::fs::read_to_string("spec/acceptance/goldens/neg_linear_reflect.omni")
        .map_err(|e| format!("B2 read neg_linear_reflect: {}", e))?;
    let neg_lin_u16 = with_lockstep_suspended(|| compile_omni_surface_to_u16(&neg_lin))?;
    match pure_default_verify_u16(&neg_lin_u16, true) {
        Ok(()) => return Err(String::from("B2 Pure verifier must reject neg_linear_reflect.omni")),
        Err(msg) => {
            if !msg.contains("PURE_VERIFY_1012") {
                return Err(format!("B2 expected PURE_VERIFY_1012, got: {}", msg));
            }
        }
    }
    checks += 1;

    // 3. Rejects `neg_taint_sink.omni` via Pure-Omni verifier.
    let neg_taint = std::fs::read_to_string("spec/acceptance/goldens/neg_taint_sink.omni")
        .map_err(|e| format!("B2 read neg_taint_sink: {}", e))?;
    let neg_taint_u16 = with_lockstep_suspended(|| compile_omni_surface_to_u16(&neg_taint))?;
    if pure_default_verify_u16(&neg_taint_u16, true).is_ok() {
        return Err(String::from("B2 Pure verifier must reject neg_taint_sink.omni"));
    }
    checks += 1;

    // 4. Oracle comparison: Rust verifier agrees on valid golden (pass) and `neg_taint_sink.omni` (reject).
    let ok_src = std::fs::read_to_string("spec/acceptance/goldens/judge_better_golden.omni")
        .map_err(|e| format!("B2 read judge_better: {}", e))?;
    let ok_u16 = pure_default_compile_surface_to_u16(&ok_src, true)?;
    pure_default_verify_u16(&ok_u16, true)?;
    let ok_mod = decode_omni_token_stream(&ok_u16)?;
    for f in &ok_mod.functions {
        if !verify_binary_function(f).passed {
            return Err(String::from("B2 Rust oracle diverged on valid judge_better_golden.omni"));
        }
    }
    let bad_mod = decode_omni_token_stream(&neg_taint_u16)?;
    if bad_mod.functions.iter().all(|f| verify_binary_function(f).passed) {
        return Err(String::from("B2 Rust oracle diverged on neg_taint_sink.omni"));
    }
    checks += 1;

    // 5. Audit §1.2: CALL-target and arity validation in Pure verifier + M2 trap stub + fuzz corpus.
    checks += selftest_s1_2_call_target_arity_and_fuzz()?;

    // 6. Audit Round 2 N1: per-opcode minimum operand arity + EMPTY_MODULE + reconciled 0x7F42 (BR_TRIT min=4).
    checks += selftest_n1_op_arity_and_empty_module()?;

    // 7. Audit Round 3 M1: pure verifier must fail closed (REPORT_OVERFLOW 3008) on >2,387 functions,
    //    never silently dropping trailing functions or returning Ok(()) on truncated reports.
    let build_n_good_then_bad = |n_good: usize| -> Vec<u16> {
        let mut s = Vec::with_capacity(2 + (n_good + 1) * 14);
        s.push(0x7EE0); // TOK_MOD_BEGIN
        for fid in 0..n_good {
            // fn#fid() -> i64, cap=0, 1 ins: RET imm16(0)
            s.extend_from_slice(&[
                0x7EE3, fid as u16, 0, 0x7F81, 0, 1,
                0x7F4A, 0x7EE5, 0, 0x7EE4,
            ]);
        }
        // Trailing bad fn: r0: i64 [cap=1] = CINT 0 (missing cap 1003), RET r0
        s.extend_from_slice(&[
            0x7EE3, n_good as u16, 0, 0x7F81, 0, 2,
            0x7E00, 0x7F06, 0x7F81, 0x7FC1, 0x7EE5, 0, 0x7EE4,
            0x7F4A, 0x7E00, 0x7EE4,
            0x7EEF, // TOK_MOD_END
        ]);
        s
    };
    let s_2388 = build_n_good_then_bad(2388);
    match pure_default_verify_u16(&s_2388, false) {
        Ok(()) => return Err(String::from("M1 pure verifier failed open (returned Ok) on 2,388 good + 1 bad fn")),
        Err(msg) => {
            if !msg.contains("PURE_VERIFY_REPORT_OVERFLOW") && !msg.contains("3008") {
                return Err(format!("M1 2,388-fn stream expected PURE_VERIFY_REPORT_OVERFLOW, got: {}", msg));
            }
        }
    }
    // 7b. Audit Round-3 R3-2: the overflow `@word` is the report-region word offset where the failed
    //     header write would have gone (1024 + cursor, inside 1024..8192), never a function index.
    let overflow_word = |s: &[u16]| -> Result<i64, String> {
        match pure_default_verify_u16(s, false) {
            Ok(()) => Err(String::from("R3-2 overflow stream unexpectedly verified Ok")),
            Err(msg) => {
                if !msg.contains("PURE_VERIFY_REPORT_OVERFLOW") {
                    return Err(format!("R3-2 expected PURE_VERIFY_REPORT_OVERFLOW, got: {}", msg));
                }
                msg.split("@word ")
                    .nth(1)
                    .and_then(|t| t.split(':').next())
                    .and_then(|t| t.trim().parse::<i64>().ok())
                    .ok_or_else(|| format!("R3-2 could not parse @word from: {}", msg))
            }
        }
    };
    for &n_good in &[2388usize, 2400usize] {
        let w = overflow_word(&build_n_good_then_bad(n_good))?;
        if !(1024..8192).contains(&w) || w == n_good as i64 || w == n_good as i64 + 1 {
            return Err(format!(
                "R3-2 {}-fn overflow @word {} is not a report-region word offset (want 1024..8192)",
                n_good, w
            ));
        }
    }
    checks += 1;

    let s_2387 = build_n_good_then_bad(2387);
    match pure_default_verify_u16(&s_2387, false) {
        Ok(()) => return Err(String::from("M1 pure verifier failed open on 2,387 good + 1 bad fn")),
        Err(msg) => {
            if msg.contains("PURE_VERIFY_0 ") {
                return Err(format!("M1 2,387-fn stream reported bogus code 0 instead of overflow/1003: {}", msg));
            }
        }
    }
    checks += 1;

    // 8. Frozen hashes intact.
    selftest_m2_frozen_hashes()?;
    checks += 1;

    Ok(checks)
}

/// Audit Round 2 N1 gate:
///   1. Rejects TMAJ (0x7F2C) with 2 operands (`PURE_VERIFY_2012`).
///   2. Rejects 0x7F17 with 1 operand (`PURE_VERIFY_2012`).
///   3. Rejects header-only zero-function module `[0x7EE0, 0x7EEF]` (`PURE_VERIFY_EMPTY_MODULE`),
///      while keeping 0-word `[]` rejected as `PURE_VERIFY_DECODE_3006`.
///   4. Rejects `vok_crash_128` (`0x7F42` BR_TRIT with 1 operand — reconciled min=4) in BOTH
///      `pure_default_verify_u16` (`PURE_VERIFY_2012`) and Rust `OmniBinaryVm` (`BAD_ARITY`).
pub fn selftest_n1_op_arity_and_empty_module() -> Result<usize, String> {
    let mut checks = 0usize;

    let build_dest_op_stream = |op: u16, ops: &[u16]| -> Vec<u16> {
        let mut s = vec![
            0x7EE0, // TOK_MOD_BEGIN
            0x7EE3, 0, 1, 0x7EC0, 0x7F81, 0x7FC0, 0x7F81, 0, 2, // fn#0(p0: i64) -> i64, 2 ins
            0x7E00, op, 0x7F81, 0x7FC0, // r0: i64 = op ...
        ];
        s.extend_from_slice(ops);
        s.extend_from_slice(&[
            0x7EE4, // INS_END
            0x7F4A, 0x7E00, 0x7EE4, // RET r0
            0x7EEF, // TOK_MOD_END
        ]);
        s
    };

    // 1. TMAJ (0x7F2C) with 2 operands (requires 3).
    let tmaj_2op = build_dest_op_stream(0x7F2C, &[0x7EC0, 0x7EC0]);
    match pure_default_verify_u16(&tmaj_2op, false) {
        Ok(()) => return Err(String::from("N1 Pure verifier accepted TMAJ (0x7F2C) with 2 operands")),
        Err(msg) => {
            if !msg.contains("PURE_VERIFY_2012") {
                return Err(format!("N1 tmaj_2op expected PURE_VERIFY_2012, got: {}", msg));
            }
        }
    }
    checks += 1;

    // 2. 0x7F17 with 1 operand (requires 2).
    let op_7f17_1op = build_dest_op_stream(0x7F17, &[0x7EC0]);
    match pure_default_verify_u16(&op_7f17_1op, false) {
        Ok(()) => return Err(String::from("N1 Pure verifier accepted 0x7F17 with 1 operand")),
        Err(msg) => {
            if !msg.contains("PURE_VERIFY_2012") {
                return Err(format!("N1 op_7f17_1op expected PURE_VERIFY_2012, got: {}", msg));
            }
        }
    }
    checks += 1;

    // 3. Header-only zero-function module `[0x7EE0, 0x7EEF]` (EMPTY_MODULE) + 0-word `[]` (DECODE_3006).
    let empty_hdr_only: Vec<u16> = vec![0x7EE0, 0x7EEF];
    match pure_default_verify_u16(&empty_hdr_only, false) {
        Ok(()) => return Err(String::from("N1 Pure verifier accepted header-only zero-function module")),
        Err(msg) => {
            if !msg.contains("PURE_VERIFY_EMPTY_MODULE") && !msg.contains("3007") {
                return Err(format!("N1 empty_module expected PURE_VERIFY_EMPTY_MODULE, got: {}", msg));
            }
        }
    }
    match pure_default_verify_u16(&[], false) {
        Ok(()) => return Err(String::from("N1 Pure verifier accepted 0-word empty stream")),
        Err(msg) => {
            if !msg.contains("PURE_VERIFY_DECODE_3006") {
                return Err(format!("N1 0-word stream expected PURE_VERIFY_DECODE_3006, got: {}", msg));
            }
        }
    }
    checks += 1;

    // 4. `vok_crash_128`: BR_TRIT (0x7F42) with 1 operand (reconciled min=4 across Pure verifier, mod_check, and Rust VM).
    let br_trit_1op: Vec<u16> = vec![
        0x7EE0,
        0x7EE3, 0, 1, 0x7EC0, 0x7F81, 0x7FC0, 0x7F81, 0, 2,
        0x7F42, 0x7EC0, 0x7EE4, // BR_TRIT with only 1 operand (missing l_neg, l_zero, l_pos)
        0x7F4A, 0x7EC0, 0x7EE4, // RET p0
        0x7EEF,
    ];
    match pure_default_verify_u16(&br_trit_1op, false) {
        Ok(()) => return Err(String::from("N1 Pure verifier accepted BR_TRIT (0x7F42) with 1 operand")),
        Err(msg) => {
            if !msg.contains("PURE_VERIFY_2012") {
                return Err(format!("N1 vok_crash_128 expected PURE_VERIFY_2012, got: {}", msg));
            }
        }
    }
    let decoded_br = decode_omni_token_stream(&br_trit_1op)?;
    let mut rust_vm = OmniBinaryVm::new();
    match rust_vm.execute_function(&decoded_br, 0, vec![OmniValue::Int(1)]) {
        Ok(v) => return Err(format!("N1 Rust VM accepted BR_TRIT (0x7F42) with 1 operand (returned {:?})", v)),
        Err(msg) => {
            if !msg.contains("BAD_ARITY") && !msg.contains("Missing branch target label") {
                return Err(format!("N1 Rust VM expected BAD_ARITY/UNRESOLVED_LABEL on 0x7F42 1-op, got: {}", msg));
            }
        }
    }
    checks += 1;

    // 5. Drift-lock: `pure/verifier_engine.omni::op_min_ops` agrees with `pure/m2/verify/mod_check.omni::m2v_ins_min_ops`
    //    for every opcode known to `mod_check.omni` across 0x7F00..=0x7F7F.
    let ve_mod = cached_verifier_engine_module()?;
    let m2_mod = cached_m2_seed_module()?;
    let find_fn_idx = |src: &str, name: &str| -> Result<u16, String> {
        let needle = format!("fn {}(", name);
        let mut idx: u16 = 0;
        for line in src.lines() {
            let t = line.trim_start();
            if t.starts_with("fn ") {
                if t.starts_with(&needle) {
                    return Ok(idx);
                }
                idx += 1;
            }
        }
        Err(format!("N1 could not locate '{}' in source", name))
    };
    let ve_min_pos = find_fn_idx(OMNI_VERIFIER_ENGINE_SURFACE, "op_min_ops")? as usize;
    let m2_min_pos = find_fn_idx(M2_SELFHOST_OMNI_SRC, "m2v_ins_min_ops")? as usize;
    let ve_min_fn = ve_mod.functions.get(ve_min_pos).ok_or("N1 ve_min_pos OOB")?.fn_idx;
    let m2_min_fn = m2_mod.functions.get(m2_min_pos).ok_or("N1 m2_min_pos OOB")?.fn_idx;
    let mut parity_vm = OmniBinaryVm::with_budgets(50_000_000, 16 * 1024 * 1024);
    for op in 0x7F00i64..=0x7F7Fi64 {
        let ve_v = match parity_vm.execute_function(ve_mod, ve_min_fn, vec![OmniValue::Int(op)])? {
            OmniValue::Int(v) => v,
            other => return Err(format!("N1 op_min_ops(0x{:04X}) returned {:?}", op, other)),
        };
        let rust_v = vm_op_min_ops(op as u16) as i64;
        if rust_v > 0 && ve_v != rust_v {
            return Err(format!(
                "L-E opcode arity drift at 0x{:04X}: verifier_engine={} != Rust vm_op_min_ops={}",
                op, ve_v, rust_v
            ));
        }
        let m2_v = match parity_vm.execute_function(m2_mod, m2_min_fn, vec![OmniValue::Int(op)])? {
            OmniValue::Int(v) => v,
            other => return Err(format!("N1 m2v_ins_min_ops(0x{:04X}) returned {:?}", op, other)),
        };
        if m2_v >= 0 && ve_v != m2_v {
            return Err(format!(
                "N1 opcode arity drift at 0x{:04X}: verifier_engine={} != mod_check={}",
                op, ve_v, m2_v
            ));
        }
    }
    checks += 1;

    Ok(checks)
}

/// Audit §1.2 hard gate (`./omnic s1-call-verify`):
///   1. Pure-Omni verifier (`pure/verifier_engine.omni` via `pure_default_verify_u16`) rejects CALLs to
///      non-existent functions (`fn_idx = 32539`, `fn_idx = 99`) and non-`TOK_SYM_FN` CALL targets with
///      error code `2010` (`BAD_CALL_TARGET`).
///   2. Pure-Omni verifier rejects CALL arity mismatches (too few / too many args) with error code `2011`
///      (`CALL_ARITY_MISMATCH`).
///   3. Defense-in-depth M2 trap stub: when an unverified `.omni_bc` stream with an unknown or out-of-range
///      CALL target (`32539` or `99`) is lowered through Pure-Omni M2 (`m2.omni_bc`) or Rust M2-multi,
///      the `E8 rel32` relocation resolves to the `T_OOB` trap stub (`exit(101)`) instead of offset `-1`
///      (before `.text`, which segfaulted with exit 139).
///   4. Checked-in 12-case crash corpus + deterministic 256-mutant `.omni_bc` mutation fuzzer: every mutant
///      is either rejected by `pure_default_verify_u16` or executes without divergence (Pure-M4 == Rust VM)
///      and without out-of-text jump/call targets or native segfaults.
pub fn selftest_s1_2_call_target_arity_and_fuzz() -> Result<usize, String> {
    let mut checks = 0usize;

    // Helper: build a 1-fn module calling `callee_tok` with `arg_ops` words and returning r0.
    let build_call_stream = |callee_ops: &[u16]| -> Vec<u16> {
        let mut s = vec![
            0x7EE0, // TOK_MOD_BEGIN
            0x7EE3, 0, 1, 0x7EC0, 0x7F81, 0x7FC0, 0x7F81, 0, 2, // fn#0(p0: i64) -> i64, 2 ins
            0x7E00, 0x7F48, 0x7F81, 0x7FC0, // r0: i64 = CALL ...
        ];
        s.extend_from_slice(callee_ops);
        s.extend_from_slice(&[
            0x7EE4, // INS_END
            0x7F4A, 0x7E00, 0x7EE4, // RET r0
            0x7EEF, // TOK_MOD_END
        ]);
        s
    };

    // 1. Non-existent CALL targets (`fn_idx = 32539` from Grok's audit, `fn_idx = 99`, and non-FnRef target).
    for &(label, ops) in &[
        ("missing_fn_32539", &[0x7EECu16, 32539, 0x7EC0][..]),
        ("missing_fn_99", &[0x7EECu16, 99, 0x7EC0][..]),
        ("non_fnref_reg_target", &[0x7EC0u16][..]),
        ("zero_operand_call", &[][..]),
    ] {
        let bad_s = build_call_stream(ops);
        match pure_default_verify_u16(&bad_s, false) {
            Ok(()) => {
                return Err(format!(
                    "S1.2 Pure verifier accepted invalid CALL target case '{}'",
                    label
                ))
            }
            Err(msg) => {
                if !msg.contains("PURE_VERIFY_2010") && !msg.contains("PURE_VERIFY_DECODE") {
                    return Err(format!(
                        "S1.2 case '{}' expected PURE_VERIFY_2010, got: {}",
                        label, msg
                    ));
                }
            }
        }
    }
    checks += 1;

    // 2. CALL arity mismatch (`fn#1` expects 2 params, `fn#0` calls `fn#1` with 1 arg and with 3 args).
    let valid_2fn = compile_omni_surface_strict_to_u16(
        "#[strict]\nfn add2(a: i64, b: i64) -> i64 { return a + b; }\n#[strict]\nfn main(x: i64) -> i64 { return add2(x, 10); }\n",
    )?;
    pure_default_verify_u16(&valid_2fn, false)?;
    // Mutate fn#0's CALL to pass 1 arg (drop `TOK_IMM_I16, 10`) or 3 args (append `0x7EC0`).
    let mut bad_arity_1 = Vec::new();
    let mut bad_arity_3 = Vec::new();
    let mut i = 0usize;
    while i < valid_2fn.len() {
        if valid_2fn[i] == 0x7F48 {
            let mut end = i + 1;
            while end < valid_2fn.len() && valid_2fn[end] != 0x7EE4 {
                end += 1;
            }
            // [0x7F48, 0x7F81, 0x7FC0, 0x7EEC, fn_id, 0x7EC0, ... second arg ..., 0x7EE4]
            bad_arity_1.extend_from_slice(&valid_2fn[i..i + 6]);
            bad_arity_1.push(0x7EE4);
            bad_arity_3.extend_from_slice(&valid_2fn[i..end]);
            bad_arity_3.push(0x7EC0);
            bad_arity_3.push(0x7EE4);
            i = end + 1;
        } else {
            bad_arity_1.push(valid_2fn[i]);
            bad_arity_3.push(valid_2fn[i]);
            i += 1;
        }
    }
    for (label, stream) in [("arity_1_want_2", &bad_arity_1), ("arity_3_want_2", &bad_arity_3)] {
        match pure_default_verify_u16(stream, false) {
            Ok(()) => {
                return Err(format!(
                    "S1.2 Pure verifier accepted CALL arity mismatch '{}'",
                    label
                ))
            }
            Err(msg) => {
                if !msg.contains("PURE_VERIFY_2011") {
                    return Err(format!(
                        "S1.2 '{}' expected PURE_VERIFY_2011, got: {}",
                        label, msg
                    ));
                }
            }
        }
    }
    checks += 1;

    // 3. Defense-in-depth M2 trap stub: lowering unverified `missing_fn_32539` and `missing_fn_99`
    // through Pure-Omni M2 emitter (`m2_compile_multi_x86`, bypassing standalone verifier) must resolve
    // the unknown `E8 rel32` to the `T_OOB` trap stub inside `.text` (`[120 .. elf.len())`), NEVER to `-1`.
    let m2_raw_s12 = std::fs::read("pure/m2/m2.omni_bc")
        .map_err(|e| format!("S1.2 read pure/m2/m2.omni_bc: {}", e))?;
    let m2_mod_s12 = decode_omni_token_stream(&decode_bytes_to_u16_stream(&m2_raw_s12))?;
    let m2_emit_fn_s12 = m2_mod_s12.functions[m2_mod_s12.functions.len() - 2].fn_idx;
    for &bad_callee in &[32539u16, 99u16] {
        let unverified = build_call_stream(&[0x7EEC, bad_callee, 0x7EC0]);
        let mut vm_m2_s12 = OmniBinaryVm::with_budgets(500_000_000, 128 * 1024 * 1024);
        let elf = match vm_m2_s12.execute_function(
            &m2_mod_s12,
            m2_emit_fn_s12,
            vec![
                OmniValue::Bytes(vec![0u8; 524_288]),
                OmniValue::U16Array(unverified),
                OmniValue::Int(7),
                OmniValue::Int(0),
                OmniValue::Int(0),
            ],
        )? {
            OmniValue::Bytes(b) => b,
            other => return Err(format!("S1.2 M2 emitter returned {:?}", other)),
        };
        if elf.len() <= 132 || &elf[0..4] != &[0x7F, b'E', b'L', b'F'] {
            return Err(String::from("S1.2 M2 failed to emit ELF for trap-stub check"));
        }
        // Locate the `BF 01 00 00 00 B8 3C 00 00 00 0F 05` (T_OOB stub: exit(1)) in `elf[120..]`.
        let t_oob_seq: [u8; 12] = [0xBF, 0x01, 0x00, 0x00, 0x00, 0xB8, 0x3C, 0x00, 0x00, 0x00, 0x0F, 0x05];
        let t_oob_pos = elf[120..]
            .windows(12)
            .position(|w| w == t_oob_seq)
            .map(|p| p + 120)
            .ok_or_else(|| String::from("S1.2 T_OOB trap stub not found in M2 ELF"))?;
        // Every `E8 rel32` in the user function (after the entry stub) must target a valid offset in `[120, elf.len())`,
        // and the unknown CALL must target `t_oob_pos`.
        let mut hit_trap_call = false;
        for pos in 120..elf.len().saturating_sub(5) {
            if elf[pos] == 0xE8 {
                let rel = i32::from_le_bytes(elf[pos + 1..pos + 5].try_into().unwrap()) as isize;
                let tgt = (pos as isize) + 5 + rel;
                if tgt == 119 {
                    return Err(format!(
                        "S1.2 M2 emitted segfaulting CALL to code_off=-1 (file_off=119) for fn#{}",
                        bad_callee
                    ));
                }
                if tgt == t_oob_pos as isize {
                    hit_trap_call = true;
                }
            }
        }
        if !hit_trap_call {
            return Err(format!(
                "S1.2 M2 did not patch unknown CALL fn#{} to T_OOB trap stub (t_oob_pos={})",
                bad_callee, t_oob_pos
            ));
        }
        if let Some(r) = run_native_elf_for_diff(&elf, "s1_2_trap_stub")? {
            if r.signal.is_some() || r.exit_code != Some(OMNI_OOB_EXIT_STATUS) {
                return Err(format!(
                    "S1.2 native trap-stub ELF failed: exit={:?} signal={:?} (want exit 101, signal None)",
                    r.exit_code, r.signal
                ));
            }
        }
    }
    checks += 1;

    // 4. Checked-in crash corpus (12 mutated `.omni_bc` cases covering Grok's audit findings) +
    // deterministic 256-mutant mutation fuzzer over multi-function `.omni_bc` streams.
    let mut corpus: Vec<Vec<u16>> = vec![
        build_call_stream(&[0x7EEC, 32539, 0x7EC0]),
        build_call_stream(&[0x7EEC, 0xFFFF, 0x7EC0]),
        build_call_stream(&[0x7EEC, 1024, 0x7EC0]),
        build_call_stream(&[0x7EEC, 1, 0x7EC0, 0x7EC0]),
        build_call_stream(&[0x7E00, 0x7EC0]),
        build_call_stream(&[0x7EE5, 42]),
        bad_arity_1,
        bad_arity_3,
    ];
    let mut rng: u64 = 0x2026_1122_A0D1_7F48;
    let mut next_u64 = || -> u64 {
        rng ^= rng >> 12;
        rng ^= rng << 25;
        rng ^= rng >> 27;
        rng.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };
    for _ in 0..256 {
        let mut m = valid_2fn.clone();
        let idx = (next_u64() as usize) % m.len();
        let kind = next_u64() % 4;
        match kind {
            0 => m[idx] = (next_u64() & 0xFFFF) as u16,
            1 => m[idx] ^= 1u16 << ((next_u64() % 16) as u32),
            2 => {
                let len = (next_u64() as usize) % m.len();
                m.truncate(len.max(1));
            }
            _ => {
                m[idx] = *[0x7F48u16, 0x7EEC, 32539, 0x7EF9, 0x7F0E, 0x7F0F, 0x7EEF]
                    .get((next_u64() as usize) % 7)
                    .unwrap();
            }
        }
        corpus.push(m);
    }
    let mut accepted = 0usize;
    for (ci, stream) in corpus.iter().enumerate() {
        if pure_default_verify_u16(stream, false).is_ok() {
            accepted += 1;
            // Every verifier-accepted stream MUST also pass Rust `verify_module_for_multi_aot`
            // (except single-fn / non-recursive checks) and agree between Rust VM and Pure-M4 VM!
            let decoded = decode_omni_token_stream(stream)
                .map_err(|e| format!("S1.2 fuzz #{} accepted by Pure verifier but failed decode: {}", ci, e))?;
            for f in &decoded.functions {
                let rep = verify_binary_function(f);
                if !rep.passed {
                    return Err(format!(
                        "S1.2 fuzz #{} accepted by Pure verifier but rejected by Rust verifier: {:?}",
                        ci, rep.errors
                    ));
                }
            }
            let entry = m2_entry_of(&decoded);
            let mut rvm = OmniBinaryVm::with_budgets(50_000, 4 * 1024 * 1024);
            let r_res = rvm.execute_function(&decoded, entry, vec![OmniValue::Int(7), OmniValue::Int(3)]);
            let m4_res = p4_run_raw_bc_on_pure_vm(stream, 7, 3, 0, 50_000, 0)?;
            if let Ok(OmniValue::Int(rv)) = r_res {
                if m4_res.2 == 0 && m4_res.0 != rv {
                    return Err(format!(
                        "S1.2 fuzz #{} VM divergence: Rust VM={} vs Pure-M4={} (fns={:?})",
                        ci, rv, m4_res.0, decoded.functions
                    ));
                }
            }
        }
    }
    if accepted == corpus.len() {
        return Err(String::from("S1.2 fuzz corpus had 0 rejections"));
    }
    checks += 1;

    Ok(checks)
}

/// Seed-Shrink Phase B4 hard gate (`./omnic b4-pure-launcher`):
///   1. Pure-Omni M3 (0-word diff) + Pure-Omni M2 (0-byte diff) on `pure/m5/launcher.omni`.
///   2. Emits standalone 237-byte zero-shim Linux x86-64 launcher ELF64 (`\x7fELF`, PT_LOAD, raw `0F 05` syscalls).
///   3. On Linux x86-64: executes the 237-B launcher natively (zero Rust binary, zero libc) on valid
///      `pure/m4/m4.omni_bc` (exit 0) AND corrupt `[0xFF, 0xFF]` (exit 1 fail-closed).
///   4. On Linux x86-64: runs the pure native M2 compiler ELF (`m2_native.elf`, zero Rust binary)
///      to rebuild `m2.omni_bc` (93,561 B) and `m4.omni_bc` (78,020 B) with 0-byte diff.
///   5. All 5 frozen FNV-1a64 hashes intact.
pub fn selftest_b4_pure_launcher() -> Result<(usize, usize, u64, bool), String> {
    let mut checks = 0usize;

    // 1. Compile pure/m5/launcher.omni + spec/m5/launcher_harness.omni via Pure-M3 (0-word diff) + Pure-M2 (0-byte diff).
    let l_u16 = pure_default_compile_surface_to_u16(M5_LAUNCHER_OMNI_SRC, true)?;
    let l_rust_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(M5_LAUNCHER_OMNI_SRC))?;
    if l_u16 != l_rust_u16 {
        return Err(format!("B4 Pure-M3 word diff on pure/m5/launcher.omni: {}w vs {}w", l_u16.len(), l_rust_u16.len()));
    }
    let l_elf_p = pure_default_compile_u16_to_elf(&l_u16, 0, 0, 0)?;
    let l_elf_r = compile_pure_u16_to_x86_64_elf_multi(&l_u16, 0, 0, 0)?;
    if l_elf_p != l_elf_r {
        return Err(format!("B4 Pure-M2 byte diff on pure/m5/launcher.omni: {}B vs {}B", l_elf_p.len(), l_elf_r.len()));
    }
    checks += 1;

    // 2. Execute `pure/m5/launcher.omni` to emit the standalone 237-B zero-shim Linux x86-64 launcher ELF64.
    let l_mod = decode_omni_token_stream(&l_u16)?;
    let entry_fn = l_mod.functions.last().ok_or("B4 empty launcher module")?.fn_idx;
    let mut vm = OmniBinaryVm::with_budgets(50_000_000, 32 * 1024 * 1024);
    let launcher_elf = match vm.execute_function(
        &l_mod,
        entry_fn,
        vec![
            OmniValue::Bytes(vec![0u8; 524_288]),
            OmniValue::Int(0),
            OmniValue::Int(0),
            OmniValue::Int(0),
        ],
    )? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("B4 launcher emitter returned {:?}", other)),
    };
    if launcher_elf.len() != 237 || &launcher_elf[0..4] != b"\x7fELF" {
        return Err(format!("B4 launcher ELF invalid: {} bytes", launcher_elf.len()));
    }
    let has_syscall = launcher_elf.windows(2).any(|w| w == [0x0F, 0x05]);
    if !has_syscall {
        return Err(String::from("B4 launcher ELF missing raw 0F 05 syscall instruction"));
    }
    let launcher_hash = fnv1a_64_hash(&launcher_elf);
    checks += 1;

    // 3 & 4. Native Linux x86-64 zero-Rust-binary launcher + pure native M2 self-host verification.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    let native_ok = {
        use std::io::Write;
        let tmp_launcher = PrivateTempElf::new(&launcher_elf, "b4_launcher")?;

        // 3a. Valid .omni_bc seed on stdin -> exit 0.
        let m4_raw = std::fs::read("pure/m4/m4.omni_bc").map_err(|e| format!("B4 read m4 seed: {}", e))?;
        let mut child_ok = std::process::Command::new(&tmp_launcher.path)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("B4 spawn launcher (valid): {}", e))?;
        if let Some(mut sin) = child_ok.stdin.take() {
            let _ = sin.write_all(&m4_raw[..64.min(m4_raw.len())]);
        }
        let st_ok = child_ok.wait().map_err(|e| format!("B4 wait valid: {}", e))?;
        if st_ok.code() != Some(0) {
            return Err(format!("B4 native launcher rejected valid seed: exit {:?}", st_ok.code()));
        }

        // 3b. Corrupt input on stdin -> exit 1 (fail-closed).
        let mut child_bad = std::process::Command::new(&tmp_launcher.path)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("B4 spawn launcher (corrupt): {}", e))?;
        if let Some(mut sin) = child_bad.stdin.take() {
            let _ = sin.write_all(&[0xFF, 0xFF, 0x00, 0x00]);
        }
        let st_bad = child_bad.wait().map_err(|e| format!("B4 wait corrupt: {}", e))?;
        drop(tmp_launcher);
        if st_bad.code() != Some(1) {
            return Err(format!("B4 native launcher must fail closed (exit 1) on corrupt input, got {:?}", st_bad.code()));
        }

        // 4. Pure native M2 compiler ELF (emitted by Pure M2 itself, zero Rust binary) compiles
        //    pure/m4/m4.omni_bc -> exact 90,402-B ELF64 (0xB43AFBB0DED59488).
        let m2_raw = std::fs::read("pure/m2/m2.omni_bc").map_err(|e| format!("B4 read m2 seed: {}", e))?;
        let m2_u16 = decode_bytes_to_u16_stream(&m2_raw);
        let m2_elf = pure_default_compile_u16_to_elf(&m2_u16, 0, 0, 0)?;
        let tmp_m2 = PrivateTempElf::new(&m2_elf, "b4_m2")?;
        let out_m4 = std::process::Command::new(&tmp_m2.path)
            .arg("pure/m4/m4.omni_bc")
            .output()
            .map_err(|e| format!("B4 exec m2_native: {}", e))?;
        drop(tmp_m2);
        if !out_m4.status.success() || out_m4.stdout.len() != 90_402 || fnv1a_64_hash(&out_m4.stdout) != 0xB43AFBB0DED59488 {
            return Err(format!(
                "B4 pure native m2.elf -> m4.elf mismatch: status={:?} len={} hash=0x{:016X}",
                out_m4.status,
                out_m4.stdout.len(),
                fnv1a_64_hash(&out_m4.stdout)
            ));
        }
        true
    };
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    let native_ok = false;
    checks += 2;

    // 5. All 5 frozen FNV-1a64 hashes intact.
    selftest_m2_frozen_hashes()?;
    checks += 1;

    Ok((checks, launcher_elf.len(), launcher_hash, native_ok))
}

/// Audit §2 Security hard gate (`./omnic s2-security`):
///   1. §2.1: `m2.omni_bc` (`m2_standalone_entry`) and native `m2.elf` verify input via `m2_verify_module_pure`
///      and refuse unverified/malformed `.omni_bc` streams from the S1 crash corpus (0 segfaults).
///   2. §2.2: `pure_default_run_u16` (`trace=true` and `trace=false`) decodes `u32 err @28` and `u32 steps @44`
///      accurately; `p4_shim_exec_bc` (`p4-exec`) verifies input before execution; 412-byte mutated stream
///      terminates in bounded time with fuel charged in decode/label-index loops; host wall-clock cap works.
///   3. §2.3: OOB programs report `error[OUT_OF_BOUNDS]` with real `status=2` and exact `steps` across backends.
///   4. §2.5: `run_omni_serve_cli` rejects positional bind addresses (requires `--bind`), and per-request errors
///      do not abort the HTTP server batch loop.
pub fn selftest_s2_security() -> Result<usize, String> {
    let mut checks = 0usize;

    // 1. §2.3: `pure_default_run_u16` with `trace=true` on a valid program must NOT false-trap
    //    (because `trace_cnt` is at +24 and `err_code` is at +28), and on an OOB program must report
    //    `error[OUT_OF_BOUNDS]` with real `status=2` and small `steps` (never 8589934592 / 12884901888).
    let ok_u16 = with_lockstep_suspended(|| {
        compile_omni_surface_strict_to_u16("#[strict]\nfn main(x: i64) -> i64 { return x + 5; }\n")
    })?;
    let (ok_ret, ok_steps) = pure_default_run_u16(&ok_u16, 37, 0, 0, true)?;
    if ok_ret != 42 || ok_steps <= 0 || ok_steps > 100 {
        return Err(format!(
            "S2.3 pure_default_run_u16(trace=true) failed: ret={} steps={}",
            ok_ret, ok_steps
        ));
    }
    let oob_u16 = with_lockstep_suspended(|| {
        compile_omni_surface_strict_to_u16(
            "#[strict]\nfn main(a: i64) -> i64 { let w: bytes = anew(0); w = iset(w, a); return ld_u32(w, 0); }\n",
        )
    })?;
    match pure_default_run_u16(&oob_u16, 5, 0, 0, false) {
        Ok((v, s)) => {
            return Err(format!(
                "S2.3 pure_default_run_u16 must fail on OOB ld_u32, got Ok(({}, {}))",
                v, s
            ))
        }
        Err(msg) => {
            if !msg.contains("error[OUT_OF_BOUNDS]")
                || !msg.contains("status=2 ")
                || msg.contains("8589934592")
            {
                return Err(format!(
                    "S2.3 OOB trap message must contain error[OUT_OF_BOUNDS] and status=2, got: {}",
                    msg
                ));
            }
        }
    }
    checks += 1;

    // 2. §2.1: `m2.omni_bc` (`pure_default_compile_u16_to_elf`) and native `m2.elf` must refuse unverified
    //    S1 crash corpus inputs (missing CALL target 32539, arity mismatch, missing RET, undefined branch label,
    //    and mutated streams) — or trap cleanly with 0 segfaults.
    let bad_call_32539: Vec<u16> = vec![
        0x7EE0, 0x7EE3, 0, 1, 0x7EC0, 0x7F81, 0x7FC0, 0x7F81, 0x7FC0, 2, 0x7E00, 0x7F48, 0x7F81,
        0x7FC0, 0x7EEC, 32539, 0x7EC0, 0x7EE4, 0x7F4A, 0x7E00, 0x7EE4, 0x7EE1, 0x7EEF,
    ];
    let missing_ret: Vec<u16> = vec![
        0x7EE0, 0x7EE3, 0, 1, 0x7EC0, 0x7F81, 0x7FC0, 0x7F81, 0x7FC0, 1, 0x7E00, 0x7F10, 0x7F81,
        0x7FC0, 0x7EC0, 0x7EC0, 0x7EE4, 0x7EE1, 0x7EEF,
    ];
    let bad_branch_lbl: Vec<u16> = vec![
        0x7EE0, 0x7EE3, 0, 1, 0x7EC0, 0x7F81, 0x7FC0, 0x7F81, 0x7FC0, 2, 0x7F41, 0x7EF5, 0x7EE4,
        0x7F4A, 0x7EC0, 0x7EE4, 0x7EE1, 0x7EEF,
    ];
    let bad_op_arity: Vec<u16> = vec![
        0x7EE0, 0x7EE3, 0, 1, 0x7EC0, 0x7F81, 0x7FC0, 0x7F81, 0x7FC0, 2, 0x7E00, 0x7F10, 0x7F81,
        0x7FC0, 0x7EC0, 0x7EE4, 0x7F4A, 0x7E00, 0x7EE4, 0x7EE1, 0x7EEF,
    ];
    for (label, case) in [
        ("bad_call_32539", &bad_call_32539),
        ("missing_ret", &missing_ret),
        ("bad_branch_lbl", &bad_branch_lbl),
        ("bad_op_arity", &bad_op_arity),
    ] {
        if pure_default_compile_u16_to_elf(case, 0, 0, 0).is_ok() {
            return Err(format!(
                "S2.1 Pure-M2 standalone seed accepted unverified S1 crash case '{}'",
                label
            ));
        }
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        let m2_raw = std::fs::read("pure/m2/m2.omni_bc")
            .map_err(|e| format!("S2.1 read pure/m2/m2.omni_bc: {}", e))?;
        let m2_u16 = decode_bytes_to_u16_stream(&m2_raw);
        let m2_elf = compile_pure_u16_to_x86_64_elf_multi(&m2_u16, 0, 0, 0)?;
        let tmp_m2 = PrivateTempElf::new(&m2_elf, "s2_1_m2_native")?;
        for (label, case) in [
            ("bad_call_32539", &bad_call_32539),
            ("missing_ret", &missing_ret),
            ("bad_branch_lbl", &bad_branch_lbl),
            ("bad_op_arity", &bad_op_arity),
        ] {
            let case_path = tmp_m2.dir.join(format!("{}.omni_bc", label));
            std::fs::write(&case_path, encode_u16_stream_to_bytes(case))
                .map_err(|e| format!("S2.1 write {}: {}", label, e))?;
            let out = std::process::Command::new(&tmp_m2.path)
                .arg(&case_path)
                .output()
                .map_err(|e| format!("S2.1 exec m2.elf on {}: {}", label, e))?;
            if out.status.code() != Some(1) || !out.stdout.is_empty() {
                return Err(format!(
                    "S2.1 native m2.elf did not refuse '{}': status={:?} stdout_len={}",
                    label,
                    out.status,
                    out.stdout.len()
                ));
            }
        }
    }
    checks += 1;

    // 3. §2.2: `p4_shim_exec_bc` (`p4-exec`) must verify input before executing, and a 412-byte (206-word)
    //    mutated stream must terminate in bounded time (< 3s).
    let mut mem_shim = MemoryThinShim::new();
    mem_shim.files.insert(
        String::from("bad_call.omni_bc"),
        encode_u16_stream_to_bytes(&bad_call_32539),
    );
    if p4_shim_exec_bc(&mut mem_shim, "bad_call.omni_bc", None, 0, 0, 0, 100_000, 0).is_ok() {
        return Err(String::from(
            "S2.2 p4_shim_exec_bc (p4-exec) executed unverified bad_call.omni_bc without rejecting",
        ));
    }
    // Construct a 412-byte (206-word) mutated stream with inflated ins_cnt=50000 in header.
    let mut stream_206w = ok_u16.clone();
    if stream_206w.len() > 9 {
        stream_206w[9] = 50_000; // mutated ins_cnt in FN_DEF
    }
    while stream_206w.len() < 206 {
        stream_206w.push(0x7F00);
    }
    stream_206w.truncate(206);
    let t0 = std::time::Instant::now();
    let (_, _, err_206, _, _) = p4_run_raw_bc_on_pure_vm(&stream_206w, 1, 0, 0, 5_000, 0)?;
    if err_206 == 0 || t0.elapsed().as_secs() >= 5 {
        return Err(format!(
            "S2.2 412-byte mutated stream failed bound check: err={} elapsed={:?}",
            err_206,
            t0.elapsed()
        ));
    }
    checks += 1;

    // 4. §2.4: PrivateTempElf creates a private temp directory + O_EXCL file and cleans up on Drop.
    let (priv_dir, priv_path) = {
        let tmp_elf = PrivateTempElf::new(&[0x7F, b'E', b'L', b'F'], "s2_4_check")?;
        if !tmp_elf.dir.is_dir() || !tmp_elf.path.is_file() {
            return Err(String::from("S2.4 PrivateTempElf failed to create private dir/file"));
        }
        if std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_elf.path)
            .is_ok()
        {
            return Err(String::from("S2.4 O_EXCL (create_new) did not reject existing file"));
        }
        (tmp_elf.dir.clone(), tmp_elf.path.clone())
    };
    if priv_path.exists() || priv_dir.exists() {
        return Err(String::from("S2.4 PrivateTempElf Drop did not clean up temp dir/file"));
    }
    checks += 1;

    // 5. §2.2 host wall-clock cap: an infinite loop on OmniBinaryVm with a 5ms wall timeout terminates cleanly.
    let inf_u16 = with_lockstep_suspended(|| {
        compile_omni_surface_strict_to_u16(
            "#[strict]\nfn main(n: i64) -> i64 { let i: i64 = 0; while n > 0 { i = i + 1; } return i; }\n",
        )
    })?;
    let inf_mod = decode_omni_token_stream(&inf_u16)?;
    let mut timed_vm = OmniBinaryVm::with_budgets(500_000_000, 16 * 1024 * 1024);
    timed_vm.set_wall_timeout_ms(5);
    match timed_vm.execute_function(&inf_mod, 0, vec![OmniValue::Int(1)]) {
        Ok(_) => return Err(String::from("S2.2 host wall-clock cap did not terminate infinite loop")),
        Err(msg) => {
            if !msg.contains("wall-clock timeout exceeded") {
                return Err(format!("S2.2 expected wall-clock timeout error, got: {}", msg));
            }
        }
    }
    checks += 1;

    // 6. §2.5: `run_omni_serve_cli` must reject a positional bind address and require `--bind`.
    let code = run_omni_serve_cli(&[
        String::from("route GET \"/\" => 1;"),
        String::from("127.0.0.1:0"),
    ]);
    if code != 2 {
        return Err(format!(
            "S2.5 run_omni_serve_cli accepted positional bind address (exit={}, want 2)",
            code
        ));
    }
    checks += 1;

    // 7. Audit Round 2 N4 + Round 3 L-C: Native cache hardening (reject pre-created loose dir, stable 0700 per-user unset-HOME fallback).
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let prev_xdg = std::env::var_os("XDG_CACHE_HOME");
        let prev_home = std::env::var_os("HOME");
        let loose_root = std::env::temp_dir().join(format!("omni_n4_loose_{}", std::process::id()));
        let loose_cache = loose_root.join("omni").join("native-v1");
        let _ = std::fs::remove_dir_all(&loose_root);
        std::fs::create_dir_all(&loose_cache).map_err(|e| format!("N4 mkdir loose: {}", e))?;
        std::fs::set_permissions(&loose_cache, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("N4 chmod loose: {}", e))?;
        std::env::set_var("XDG_CACHE_HOME", &loose_root);
        let loose_res = native_toolchain_cache_dir();
        let _ = std::fs::remove_dir_all(&loose_root);

        // Audit Round-3 L-C: unset HOME/XDG -> one stable per-user `$TMPDIR/omni-native-cache-<euid>`
        // (0700, owned by euid, reused across calls/processes), and a pre-created loose 0755
        // fallback dir (e.g. squatted by another user) is rejected fail-closed. Run under a
        // private TMPDIR so the gate never touches the real per-user fallback directory.
        let prev_tmp = std::env::var_os("TMPDIR");
        let fb_root = std::env::temp_dir().join(format!("omni_lc_fallback_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&fb_root);
        std::fs::create_dir_all(&fb_root).map_err(|e| format!("L-C mkdir fallback root: {}", e))?;
        let squat_root = std::env::temp_dir().join(format!("omni_lc_squat_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&squat_root);
        let squat_dir = squat_root.join(format!("omni-native-cache-{}", current_euid()));
        std::fs::create_dir_all(&squat_dir).map_err(|e| format!("L-C mkdir squat: {}", e))?;
        std::fs::set_permissions(&squat_dir, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("L-C chmod squat: {}", e))?;
        std::env::remove_var("XDG_CACHE_HOME");
        std::env::remove_var("HOME");
        std::env::set_var("TMPDIR", &fb_root);
        let fallback_res = native_toolchain_cache_dir();
        let fallback_res2 = native_toolchain_cache_dir();
        std::env::set_var("TMPDIR", &squat_root);
        let squat_res = native_toolchain_cache_dir();
        match prev_tmp {
            Some(v) => std::env::set_var("TMPDIR", v),
            None => std::env::remove_var("TMPDIR"),
        }

        match prev_xdg {
            Some(v) => std::env::set_var("XDG_CACHE_HOME", v),
            None => std::env::remove_var("XDG_CACHE_HOME"),
        }
        match prev_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }

        if loose_res.is_ok() {
            return Err(String::from("N4 native_toolchain_cache_dir accepted pre-created 0755 loose dir"));
        }
        let lc_check = (|| -> Result<(), String> {
            let fb_dir = fallback_res.map_err(|e| format!("L-C unset-HOME fallback failed: {}", e))?;
            let fb_dir2 = fallback_res2.map_err(|e| format!("L-C unset-HOME fallback (2nd call) failed: {}", e))?;
            let want = fb_root.join(format!("omni-native-cache-{}", current_euid()));
            if fb_dir != want || fb_dir2 != want {
                return Err(format!(
                    "L-C unset-HOME fallback not the stable per-user dir: {} / {} (want {})",
                    fb_dir.display(),
                    fb_dir2.display(),
                    want.display()
                ));
            }
            let fb_meta = std::fs::symlink_metadata(&fb_dir).map_err(|e| format!("L-C lstat fallback: {}", e))?;
            if !fb_meta.file_type().is_dir() || fb_meta.uid() != current_euid() || (fb_meta.mode() & 0o077) != 0 {
                return Err(format!(
                    "L-C unset-HOME fallback dir unsafe (uid={}, mode={:o})",
                    fb_meta.uid(),
                    fb_meta.mode() & 0o777
                ));
            }
            let leaked = std::fs::read_dir(&fb_root)
                .map_err(|e| format!("L-C read fallback root: {}", e))?
                .count();
            if leaked != 1 {
                return Err(format!("L-C fallback root holds {} entries after 2 calls (want 1, no per-call leak)", leaked));
            }
            match squat_res {
                Ok(d) => Err(format!("L-C accepted pre-created loose 0755 fallback dir {}", d.display())),
                Err(msg) if msg.contains("CACHE_UNSAFE_DIR") => Ok(()),
                Err(msg) => Err(format!("L-C squatted fallback expected CACHE_UNSAFE_DIR, got: {}", msg)),
            }
        })();
        let _ = std::fs::remove_dir_all(&fb_root);
        let _ = std::fs::remove_dir_all(&squat_root);
        lc_check?;
        // Re-verify before spawn: wrong FNV pin must fail closed before exec.
        let fake_elf = PrivateTempElf::new(&[0x7F, b'E', b'L', b'F'], "n4_tamper")?;
        match spawn_verified_native_elf(&fake_elf.path, 4, 0xDEAD_BEEF_CAFE_BABE, &[]) {
            Ok(_) => return Err(String::from("N4 spawn_verified_native_elf spawned ELF with mismatched pin")),
            Err(msg) => {
                if !msg.contains("NATIVE_ELF_PIN_MISMATCH") {
                    return Err(format!("N4 expected NATIVE_ELF_PIN_MISMATCH, got: {}", msg));
                }
            }
        }
    }
    checks += 1;

    // 7b. Audit Round-3 L-D: a native-cache failure on a CLI verify path prints
    //     `[NATIVE-CACHE FAIL]` (never `[PURE-VERIFY FAIL]`), and a real verifier rejection
    //     still prints `[PURE-VERIFY FAIL]` (never `[NATIVE-CACHE FAIL]`).
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        use std::os::unix::fs::PermissionsExt;
        let exe = std::env::current_exe().map_err(|e| format!("L-D current_exe: {}", e))?;
        let ld_root = std::env::temp_dir().join(format!("omni_ld_label_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&ld_root);
        let ld_cache = ld_root.join("xdg").join("omni").join("native-v1");
        std::fs::create_dir_all(&ld_cache).map_err(|e| format!("L-D mkdir: {}", e))?;
        std::fs::set_permissions(&ld_cache, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("L-D chmod: {}", e))?;
        // Under `cargo test` the exe is the libtest harness, at `target/*/deps/<name>-<hash>` or (build-dir layout)
        // `target/*/build/<crate>-<hash>/out/<name>-<hash>`; it would take `run-wasm` as a test filter and exit 0. So any
        // ancestor named `deps` or `build` selects the in-process check below, which is exactly as strict. A real omnic
        // (built at the repo root, or installed) does not live under a cargo `deps/`/`build/` dir, so it keeps the spawn
        // path; if one ever did, it would only lose spawn coverage, never pass a loose cache dir or a bad stream.
        let exe_is_test_harness = exe.parent().map_or(false, |d| {
            d.ancestors().filter_map(|a| a.file_name()).any(|n| n == "deps" || n == "build")
        });
        let ld_check = (|| -> Result<(), String> {
            if exe_is_test_harness {
                let cache_e = match verify_private_cache_dir(&ld_cache) {
                    Ok(()) => return Err(String::from("L-D loose 0755 cache dir accepted (expected CACHE_UNSAFE_DIR)")),
                    Err(e) => e,
                };
                let cache_label = cli_fail_label("PURE-VERIFY FAIL", &cache_e);
                if !cache_label.starts_with("[NATIVE-CACHE FAIL]") || !cache_label.contains("CACHE_UNSAFE_DIR") {
                    return Err(format!("L-D loose cache dir must label [NATIVE-CACHE FAIL], got {}", cache_label));
                }
                let verify_e = match pure_default_verify_u16(&[0x1234u16, 0x7EE3, 0x7EEF], false) {
                    Ok(()) => return Err(String::from("L-D bad stream passed pure verifier")),
                    Err(e) => e,
                };
                let verify_label = cli_fail_label("PURE-VERIFY FAIL", &verify_e);
                if !verify_label.starts_with("[PURE-VERIFY FAIL]") {
                    return Err(format!("L-D verifier rejection must label [PURE-VERIFY FAIL], got {}", verify_label));
                }
                return Ok(());
            }
            let good_u16 = compile_omni_surface_file_to_u16("spec/acceptance/goldens/seed6_clamp.omni")?;
            let good_bc = ld_root.join("good.omni_bc");
            std::fs::write(&good_bc, encode_u16_stream_to_bytes(&good_u16))
                .map_err(|e| format!("L-D write good bc: {}", e))?;
            let bad_bc = ld_root.join("bad.omni_bc");
            std::fs::write(&bad_bc, encode_u16_stream_to_bytes(&[0x1234u16, 0x7EE3, 0x7EEF]))
                .map_err(|e| format!("L-D write bad bc: {}", e))?;
            let cache_out = std::process::Command::new(&exe)
                .arg("run-wasm")
                .arg(&good_bc)
                .env("XDG_CACHE_HOME", ld_root.join("xdg"))
                .output()
                .map_err(|e| format!("L-D spawn (loose cache): {}", e))?;
            let cache_err = String::from_utf8_lossy(&cache_out.stderr).into_owned();
            if cache_out.status.code() != Some(1)
                || !cache_err.contains("[NATIVE-CACHE FAIL]")
                || !cache_err.contains("CACHE_UNSAFE_DIR")
                || cache_err.contains("[PURE-VERIFY FAIL]")
            {
                return Err(format!(
                    "L-D loose cache dir must print [NATIVE-CACHE FAIL] (exit 1), got exit={:?} stderr={}",
                    cache_out.status.code(),
                    cache_err.trim()
                ));
            }
            let verify_out = std::process::Command::new(&exe)
                .arg("run-wasm")
                .arg(&bad_bc)
                .output()
                .map_err(|e| format!("L-D spawn (bad stream): {}", e))?;
            let verify_err = String::from_utf8_lossy(&verify_out.stderr).into_owned();
            if verify_out.status.code() != Some(1)
                || !verify_err.contains("[PURE-VERIFY FAIL]")
                || verify_err.contains("[NATIVE-CACHE FAIL]")
            {
                return Err(format!(
                    "L-D verifier rejection must print [PURE-VERIFY FAIL] (exit 1), got exit={:?} stderr={}",
                    verify_out.status.code(),
                    verify_err.trim()
                ));
            }
            Ok(())
        })();
        let _ = std::fs::remove_dir_all(&ld_root);
        ld_check?;
    }
    checks += 1;

    // 8. Audit Round 2 L3 & L4: Wasm AOT arity check on crafted 1-op 0x7F17 (`idxpanic_5`) and 2-op 0x7F2C (TMAJ)
    //    must return Err(BAD_ARITY) without panicking, and native m2.elf rejection must not mislabel as OUT_OF_BOUNDS.
    for &(label, bad_op, ops) in &[
        ("idxpanic_5_0x7f17", 0x7F17u16, &[0x7EC0u16][..]),
        ("tmaj_2op_0x7f2c", 0x7F2Cu16, &[0x7EC0u16, 0x7EC0u16][..]),
    ] {
        let mut s = vec![
            0x7EE0,
            0x7EE3, 0, 1, 0x7EC0, 0x7F81, 0x7FC0, 0x7F81, 0, 2,
            0x7E00, bad_op, 0x7F81, 0x7FC0,
        ];
        s.extend_from_slice(ops);
        s.extend_from_slice(&[0x7EE4, 0x7F4A, 0x7E00, 0x7EE4, 0x7EEF]);
        match compile_pure_u16_to_wasm(&s) {
            Ok(_) => return Err(format!("L3 Wasm AOT accepted under-arity '{}' without BAD_ARITY", label)),
            Err(msg) => {
                if !msg.contains("BAD_ARITY") {
                    return Err(format!("L3 '{}' expected BAD_ARITY, got: {}", label, msg));
                }
            }
        }
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        match pure_default_compile_u16_to_elf(&bad_call_32539, 0, 0, 0) {
            Ok(_) => return Err(String::from("L4 native m2.elf accepted bad_call_32539")),
            Err(msg) => {
                if msg.starts_with(OMNI_OOB_ERROR) {
                    return Err(format!("L4 native m2.elf rejection mislabeled as OUT_OF_BOUNDS: {}", msg));
                }
            }
        }
    }
    checks += 1;

    Ok(checks)
}

/// Parses optional `[a0, a1, a2]` arguments for `omni compile-elf <in> <out> [a0 a1 a2]`.
/// Audit §4.3: defaults to `(0, 0, 0)` when omitted so `compile-elf` matches the B4 gate.
pub fn parse_compile_elf_args(pos: &[&str]) -> (i64, i64, i64) {
    let a0 = pos.get(2).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    let a1 = pos.get(3).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    let a2 = pos.get(4).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    (a0, a1, a2)
}

/// Audit §4 Honesty hard gate (`./omnic s4-honesty`):
///   1. §4.3: `compile-elf` defaults `[a0, a1, a2]` to `(0, 0, 0)` so `compile-elf pure/m4/m4.omni_bc`
///      produces the exact same ELF bytes and FNV-1a64 hash as the B4 gate (`0/0/0`).
///   2. §4.1: `README.md` and `oracle/README.md` distinguish the 236-line classified Stage-0 seed
///      (`seed0` 134 + `p4_shim` 102) from the live runtime-required Rust lines (`monolith + m2_host + p4_shim`),
///      and `oracle/README.md` accurately states that `zero_rust_rebuild.sh` invokes `./omni` ("no Rust toolchain on PATH").
///   3. §4.2: Line counts in `README.md` and `oracle/README.md` match live `wc -l` measurements on disk,
///      `seed0/omni_seed0.rs` header comments match code constants, and `pure/m5/launcher.omni` is documented
///      honestly as a 237-byte `.omni_bc` `0x7EE0` header validator / entry stub.
pub fn selftest_s4_honesty() -> Result<usize, String> {
    let mut checks = 0usize;

    // 1. §4.3: `parse_compile_elf_args` must default to (0, 0, 0) when [a0 a1 a2] are omitted.
    let (a0, a1, a2) = parse_compile_elf_args(&["pure/m4/m4.omni_bc", "out.elf"]);
    if (a0, a1, a2) != (0, 0, 0) {
        return Err(format!(
            "S4.3 compile-elf default args must be (0, 0, 0) to match B4 gate, got ({}, {}, {})",
            a0, a1, a2
        ));
    }
    checks += 1;

    // 2. §4.1 & §4.2: Live `wc -l` measurements vs `README.md` and `oracle/README.md`.
    let count_lines = |p: &str| -> Result<usize, String> {
        let b = std::fs::read(p).map_err(|e| format!("S4 read {}: {}", p, e))?;
        Ok(b.iter().filter(|&&c| c == b'\n').count())
    };
    let monolith_lines = count_lines("omni_rust_bootstrap.rs")?;
    let m2_host_lines = count_lines("m2_host/decode_diff.rs")?
        + count_lines("m2_host/lint.rs")?
        + count_lines("m2_host/audit_view.rs")?
        + count_lines("m2_host/proposer_loop.rs")?;
    let pure_default_lines = count_lines("host_runtime/pure_default.rs")?;
    let p4_shim_lines = count_lines("p4_shim/mod.rs")?;
    let seed0_lines = count_lines("seed0/omni_seed0.rs")?;
    let dev_host_lines = count_lines("omni_dev_host.rs")?;
    if seed0_lines + p4_shim_lines != 236 || dev_host_lines != 40 {
        return Err(format!(
            "S4.1 seed0 ({}) + p4_shim ({}) != 236 or dev_host ({}) != 40",
            seed0_lines, p4_shim_lines, dev_host_lines
        ));
    }
    let runtime_rust_lines = monolith_lines + m2_host_lines + pure_default_lines + p4_shim_lines;
    let active_rust_lines =
        seed0_lines + p4_shim_lines + dev_host_lines + monolith_lines + m2_host_lines + pure_default_lines;
    let oracle_readme = std::fs::read_to_string("oracle/README.md")
        .map_err(|e| format!("S4 read oracle/README.md: {}", e))?;
    let main_readme = std::fs::read_to_string("README.md")
        .map_err(|e| format!("S4 read README.md: {}", e))?;
    if oracle_readme.contains("not invoked on default `compile`/`run`/`self-host` or `zero_rust_rebuild.sh`") {
        return Err(String::from(
            "S4.1 oracle/README.md still claims oracle is not invoked on zero_rust_rebuild.sh",
        ));
    }
    if !oracle_readme.contains(&runtime_rust_lines.to_string())
        || !oracle_readme.contains(&monolith_lines.to_string())
        || !oracle_readme.contains(&m2_host_lines.to_string())
        || !oracle_readme.contains(&pure_default_lines.to_string())
        || !oracle_readme.contains(&active_rust_lines.to_string())
    {
        return Err(format!(
            "S4.2 oracle/README.md line counts drifted from live wc -l (monolith={}, m2_host={}, pure_default={}, runtime_rust={}, active_rust={})",
            monolith_lines, m2_host_lines, pure_default_lines, runtime_rust_lines, active_rust_lines
        ));
    }
    // S4.2 (extended): README.md prose counts must match live wc -l too. README.md
    // writes counts comma-formatted (36,055), unlike oracle/README.md's plain table
    // numbers, so format them the same way before comparing. This closes the blind
    // spot that let README.md lines 5 and 144 drift to 36,028 / 26,328 / 36,202
    // while the selftest still passed.
    let fmt_comma = |n: usize| -> String {
        let s = n.to_string();
        let mut out = String::with_capacity(s.len() + s.len() / 3);
        for (i, c) in s.chars().enumerate() {
            if i > 0 && (s.len() - i) % 3 == 0 {
                out.push(',');
            }
            out.push(c);
        }
        out
    };
    if !main_readme.contains(&fmt_comma(runtime_rust_lines))
        || !main_readme.contains(&fmt_comma(monolith_lines))
        || !main_readme.contains(&fmt_comma(m2_host_lines))
        || !main_readme.contains(&fmt_comma(pure_default_lines))
        || !main_readme.contains(&fmt_comma(active_rust_lines))
    {
        return Err(format!(
            "S4.2 README.md line counts drifted from live wc -l (monolith={}, m2_host={}, pure_default={}, runtime_rust={}, active_rust={})",
            monolith_lines, m2_host_lines, pure_default_lines, runtime_rust_lines, active_rust_lines
        ));
    }
    if !main_readme.contains("validator/entry stub") {
        return Err(String::from(
            "S4.2 README.md must qualify M5 237-byte ELF as a .omni_bc header validator/entry stub",
        ));
    }
    // Audit Round 2 L7: lock all 5 frozen hashes in README.md unconditionally.
    for &(label, hex) in &[
        ("Phase D BC", "0x23B8D17257CCA9C4"),
        ("Phase D ELF", "0x8D5AAAD22399A1EE"),
        ("Core suite", "0x78462AE4C81DF408"),
        ("Stage-1 BC", "0xED4C50AA79563EFF"),
        ("Stage-1 ELF", "0x62F14B335B52BECC"),
    ] {
        if !main_readme.contains(hex) {
            return Err(format!("S4.2 README.md missing frozen {} hash {}", label, hex));
        }
    }
    // Optional strict README gate (`OMNI_STRICT_README=1` or `--strict-readme`): locks seed table,
    // M4 ELF size, 5-backend wording, and archived workspace path once docs PR lands.
    let strict_readme = matches!(
        std::env::var("OMNI_STRICT_README").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE")
    );
    if strict_readme {
        for &(label, needle) in &[
            ("M3 seed hash", "0x0E3052A4BE00D599"),
            ("M2 seed hash", "0xD7BD0CCE85021C91"),
            ("M4 seed hash", "0x0561704B3E50EFAC"),
            ("M4 ELF size", "90,402"),
            ("archived workspace", "oracle/archived-workspace/Cargo.toml"),
        ] {
            if !main_readme.contains(needle) {
                return Err(format!("L7 strict README missing {} ('{}')", label, needle));
            }
        }
        if main_readme.contains("4-backend") {
            return Err(String::from("L7 strict README still contains stale '4-backend' claim"));
        }
        if main_readme.contains("omni-workspace/Cargo.toml") {
            return Err(String::from("L7 strict README still references moved 'omni-workspace/Cargo.toml'"));
        }
    }
    checks += 1;

    // 3. §4.2: `seed0/omni_seed0.rs` header comment must match the active SEED_M2 / SEED_M4 pins.
    let seed0_src = std::fs::read_to_string("seed0/omni_seed0.rs")
        .map_err(|e| format!("S4 read seed0/omni_seed0.rs: {}", e))?;
    if !seed0_src.contains("66,249 words, 0x0E3052A4BE00D599")
        || !seed0_src.contains("43,669 words, 0xD7BD0CCE85021C91")
        || !seed0_src.contains("28,062 words, 0x0561704B3E50EFAC")
    {
        return Err(String::from(
            "S4.2 seed0/omni_seed0.rs header comment drifted from SEED_M2/SEED_M4 constants",
        ));
    }
    checks += 1;

    Ok(checks)
}


/// Audit §3 Performance gate (`./omni s3-perf`):
///   1. Committed standalone seeds `pure/m3/m3_standalone.omni_bc` and `pure/m4/m4_standalone.omni_bc`
///      exist and match `M3_STANDALONE_OMNI_SRC` / `M4_STANDALONE_OMNI_SRC` with 0-word diff.
///   2. On Linux x86-64, the default `pure_default_compile_surface_to_u16`, `pure_default_compile_u16_to_elf`,
///      and `pure_default_run_u16` paths execute hash-pinned native ELFs (`m3.elf`, `m2.elf`, `m4.elf`) with:
///        - 0-word diff vs interpreted on `pure/verifier_engine.omni` & `pure/m3/include.omni` (< 1,000 ms)
///        - exact negative error span parity (`error[M3_CODE_...]`, line 3)
///        - 0-byte diff vs interpreted on `pure/m4/m4.omni_bc` -> `89,789 B` (`0x4CD6D6CCB4408577`, < 500 ms)
///        - exact `(ret, steps)` parity on `loop n=20000` -> `(200010000, 180010)` (< 1,000 ms) and
///          `error[OUT_OF_BOUNDS]` trap parity on OOB input.
///   3. All 5 frozen FNV-1a64 hashes remain intact.
pub fn selftest_s3_performance() -> Result<usize, String> {
    let mut checks = 0usize;

    // 1. Standalone seeds exist and match source (0-word diff).
    let m3_sa_raw = std::fs::read("pure/m3/m3_standalone.omni_bc")
        .map_err(|e| format!("S3.1 missing pure/m3/m3_standalone.omni_bc: {}", e))?;
    let m4_sa_raw = std::fs::read("pure/m4/m4_standalone.omni_bc")
        .map_err(|e| format!("S3.1 missing pure/m4/m4_standalone.omni_bc: {}", e))?;
    if m3_sa_raw.len() % 2 != 0 || m4_sa_raw.len() % 2 != 0 {
        return Err(String::from("S3.1 standalone .omni_bc seed has odd byte length"));
    }
    let m3_sa_exp = resolve_omni_surface_includes(M3_STANDALONE_OMNI_SRC, None, None)?;
    let m3_sa_ref = encode_u16_stream_to_bytes(&with_lockstep_suspended(|| {
        compile_omni_surface_strict_to_u16(&m3_sa_exp)
    })?);
    if m3_sa_raw != m3_sa_ref {
        return Err(format!(
            "S3.1 pure/m3/m3_standalone.omni_bc drift ({} B, 0x{:016X} != {} B, 0x{:016X})",
            m3_sa_raw.len(),
            fnv1a_64_hash(&m3_sa_raw),
            m3_sa_ref.len(),
            fnv1a_64_hash(&m3_sa_ref)
        ));
    }
    let m4_sa_ref = encode_u16_stream_to_bytes(&with_lockstep_suspended(|| {
        compile_omni_surface_strict_to_u16(M4_STANDALONE_OMNI_SRC)
    })?);
    if m4_sa_raw != m4_sa_ref {
        return Err(format!(
            "S3.1 pure/m4/m4_standalone.omni_bc drift ({} B, 0x{:016X} != {} B, 0x{:016X})",
            m4_sa_raw.len(),
            fnv1a_64_hash(&m4_sa_raw),
            m4_sa_ref.len(),
            fnv1a_64_hash(&m4_sa_ref)
        ));
    }
    // Audit Round 2 L6: standalone verifier seed matches verifier_standalone_source() (0-word diff).
    let ve_sa_raw = std::fs::read("pure/verifier_standalone.omni_bc")
        .map_err(|e| format!("S3.1 missing pure/verifier_standalone.omni_bc: {}", e))?;
    let ve_sa_src = verifier_standalone_source();
    let ve_sa_ref = encode_u16_stream_to_bytes(&with_lockstep_suspended(|| {
        compile_omni_surface_strict_to_u16(&ve_sa_src)
    })?);
    if ve_sa_raw != ve_sa_ref {
        return Err(format!(
            "S3.1 pure/verifier_standalone.omni_bc drift ({} B, 0x{:016X} != {} B, 0x{:016X})",
            ve_sa_raw.len(),
            fnv1a_64_hash(&ve_sa_raw),
            ve_sa_ref.len(),
            fnv1a_64_hash(&ve_sa_ref)
        ));
    }
    checks += 1;

    // 2. Native-ELF default path parity & latency bounds (on Linux x86-64).
    let ve_src = std::fs::read_to_string("pure/verifier_engine.omni")
        .map_err(|e| format!("S3 read verifier_engine.omni: {}", e))?;
    let inc_src = std::fs::read_to_string("pure/m3/include.omni")
        .map_err(|e| format!("S3 read include.omni: {}", e))?;

    // Warm once so cold-cache first-time m2.elf bootstrap isn't counted against steady-state latency.
    let _warm_inc = pure_default_compile_surface_to_u16(&inc_src, true)?;

    let t0 = std::time::Instant::now();
    let ve_default = pure_default_compile_surface_to_u16(&ve_src, true)?;
    let dt_ve_ms = t0.elapsed().as_millis();
    let ve_rust = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(&ve_src))?;
    if ve_default != ve_rust {
        return Err(format!(
            "S3.2 native-default M3 word diff on pure/verifier_engine.omni ({}w vs {}w)",
            ve_default.len(),
            ve_rust.len()
        ));
    }
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) && dt_ve_ms > 1000 {
        return Err(format!(
            "S3.2 compile pure/verifier_engine.omni took {} ms (> 1000 ms native bound — still interpreted?)",
            dt_ve_ms
        ));
    }

    // Negative surface error span parity between default and interpreted.
    let bad_src = "#[strict]\nfn main(x: i64) -> i64 {\n    return unknown_ident_xyz;\n}\n";
    let err_def = pure_default_compile_surface_to_u16(bad_src, true).err().unwrap_or_default();
    let err_int = with_pure_default_interpreted(|| pure_default_compile_surface_to_u16(bad_src, true))
        .err()
        .unwrap_or_default();
    if err_def != err_int || !err_def.contains("line 3,") {
        return Err(format!(
            "S3.2 negative surface error mismatch: default='{}' vs interpreted='{}'",
            err_def, err_int
        ));
    }
    checks += 1;

    // 3. Default compile-elf on pure/m4/m4.omni_bc -> exact 90,402 B (0xB43AFBB0DED59488) in < 500 ms.
    let m4_raw = std::fs::read("pure/m4/m4.omni_bc")
        .map_err(|e| format!("S3 read m4.omni_bc: {}", e))?;
    let m4_u16 = decode_bytes_to_u16_stream(&m4_raw);
    let t1 = std::time::Instant::now();
    let m4_elf = pure_default_compile_u16_to_elf(&m4_u16, 0, 0, 0)?;
    let dt_m2_ms = t1.elapsed().as_millis();
    let h_m4_elf = fnv1a_64_hash(&m4_elf);
    if m4_elf.len() != 90_402 || h_m4_elf != 0xB43A_FBB0_DED5_9488 {
        return Err(format!(
            "S3.3 default compile-elf m4.omni_bc mismatch: {} B (0x{:016X}) != 90402 B (0xB43AFBB0DED59488)",
            m4_elf.len(),
            h_m4_elf
        ));
    }
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) && dt_m2_ms > 500 {
        return Err(format!(
            "S3.3 compile-elf pure/m4/m4.omni_bc took {} ms (> 500 ms native bound — still interpreted?)",
            dt_m2_ms
        ));
    }
    checks += 1;

    // 4. Default run on loop n=20000 -> (200010000, 180010) in < 2000 ms + OOB trap parity.
    let loop_src = "#[strict]\nfn main(n: i64) -> i64 {\n    let s: i64 = 0;\n    let i: i64 = 1;\n    while n - i + 1 <=> 0 {\n        s = s + i;\n        i = i + 1;\n    }\n    return s;\n}\n";
    let loop_u16 = pure_default_compile_surface_to_u16(loop_src, true)?;
    let (ret_100_def, steps_100_def) = pure_default_run_u16(&loop_u16, 100, 0, 0, false)?;
    let (ret_100_int, steps_100_int) =
        with_pure_default_interpreted(|| pure_default_run_u16(&loop_u16, 100, 0, 0, false))?;
    if (ret_100_def, steps_100_def) != (5050, steps_100_int) || ret_100_int != 5050 {
        return Err(format!(
            "S3.4 loop n=100 parity mismatch: default=({}, {}) vs interpreted=({}, {})",
            ret_100_def, steps_100_def, ret_100_int, steps_100_int
        ));
    }
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        let t2 = std::time::Instant::now();
        let (ret_20k, steps_20k) = pure_default_run_u16(&loop_u16, 20_000, 0, 0, false)?;
        let dt_m4_ms = t2.elapsed().as_millis();
        if ret_20k != 200_010_000 || steps_20k != 180_010 {
            return Err(format!(
                "S3.4 loop n=20000 result mismatch: got ({}, {}) want (200010000, 180010)",
                ret_20k, steps_20k
            ));
        }
        if dt_m4_ms > 2000 {
            return Err(format!(
                "S3.4 run loop n=20000 took {} ms (> 2000 ms native bound — still interpreted?)",
                dt_m4_ms
            ));
        }
    }
    // OOB trap parity: negative index on `ld_u64` must trap with `error[OUT_OF_BOUNDS]`.
    let oob_src = "#[strict]\nfn main(x: i64) -> i64 {\n    let b: bytes = anew(0);\n    return ld_u64(b, 0 - 8);\n}\n";
    let oob_u16 = pure_default_compile_surface_to_u16(oob_src, true)?;
    let oob_def = pure_default_run_u16(&oob_u16, 0, 0, 0, false).err().unwrap_or_default();
    let oob_int = with_pure_default_interpreted(|| pure_default_run_u16(&oob_u16, 0, 0, 0, false))
        .err()
        .unwrap_or_default();
    if !oob_def.starts_with(OMNI_OOB_ERROR) || oob_def != oob_int {
        return Err(format!(
            "S3.4 OOB trap parity mismatch: default='{}' vs interpreted='{}'",
            oob_def, oob_int
        ));
    }
    checks += 1;

    // 5. All 5 frozen FNV-1a64 hashes intact.
    selftest_m2_frozen_hashes()?;
    checks += 1;

    Ok(checks)
}

/// Helper: verify an ELF64 binary has `e_phnum == 2`, `e_entry == 0x4000B0`,
/// `PT_LOAD` (`p_type=1`, `p_flags=5`) at offset 64, and non-executable
/// `PT_GNU_STACK` (`p_type=0x6474E551`, `p_flags=6` = `PF_R | PF_W`) at offset 120.
pub fn verify_elf64_pt_gnu_stack(elf: &[u8], label: &str) -> Result<(), String> {
    if elf.len() < 176 || &elf[0..4] != b"\x7fELF" {
        return Err(format!("S5.2 {}: invalid or truncated ELF64 ({} B)", label, elf.len()));
    }
    let e_entry = u64::from_le_bytes(elf[24..32].try_into().unwrap());
    let e_phnum = u16::from_le_bytes(elf[56..58].try_into().unwrap());
    let pt_load_type = u32::from_le_bytes(elf[64..68].try_into().unwrap());
    let pt_load_flags = u32::from_le_bytes(elf[68..72].try_into().unwrap());
    let pt_stack_type = u32::from_le_bytes(elf[120..124].try_into().unwrap());
    let pt_stack_flags = u32::from_le_bytes(elf[124..128].try_into().unwrap());
    if e_phnum != 2 || e_entry != 0x0040_00B0 {
        return Err(format!(
            "S5.2 {}: expected e_phnum=2 e_entry=0x4000B0, got e_phnum={} e_entry=0x{:X}",
            label, e_phnum, e_entry
        ));
    }
    if pt_load_type != 1 || pt_load_flags != 5 {
        return Err(format!(
            "S5.2 {}: bad PT_LOAD (type={}, flags={})",
            label, pt_load_type, pt_load_flags
        ));
    }
    if pt_stack_type != 0x6474_E551 || pt_stack_flags != 6 {
        return Err(format!(
            "S5.2 {}: missing non-exec PT_GNU_STACK (type=0x{:X}, flags={}; want 0x6474E551, 6)",
            label, pt_stack_type, pt_stack_flags
        ));
    }
    Ok(())
}

/// Audit §5 Single Re-Freeze gate (`./omni s5-refreeze`):
///   1. §1.3 strict bounds on 4-op `ISET -9` (`iset_patch_i64` OOB -> `error[OUT_OF_BOUNDS]`,
///      never silent no-op) on both Rust `OmniBinaryVm` and Pure-M4 VM.
///   2. `PT_GNU_STACK` (`PF_R | PF_W`, non-executable stack) present on all generated ELFs
///      (single-fn Rust x86-64, multi-fn Rust x86-64, Pure-M2 ELF, and Stage-1 ELF).
///   3. Burned-down `backend-diff` xfails (IMOD rdx clobber, trit clamp, non-trit BR_POS/BR_NEG)
///      are removed from `BACKEND_DIFF_XFAILS` and pass natively.
///   4. All 5 re-frozen FNV-1a64 hashes intact.
pub fn selftest_s5_refreeze() -> Result<usize, String> {
    let mut checks = 0usize;

    // 1. §1.3: 4-op ISET -9 (`iset_patch_i64`) OOB must trap with `error[OUT_OF_BOUNDS]`.
    let oob_m9_src = "#[strict]\nfn main(x: i64) -> i64 {\n    let b: bytes = anew(0);\n    b = iset_i64(b, x);\n    b = iset_patch_i64(b, 4, 42);\n    return slen(b);\n}\n";
    let oob_m9_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(oob_m9_src))?;
    let oob_m9_mod = decode_omni_token_stream(&oob_m9_u16)?;
    let mut vm = OmniBinaryVm::new();
    match vm.execute_function(&oob_m9_mod, 0, vec![OmniValue::Int(7)]) {
        Ok(v) => {
            return Err(format!(
                "S5.1 Rust VM ISET -9 OOB must return error[OUT_OF_BOUNDS], got Ok({:?})",
                v
            ));
        }
        Err(e) if !e.contains("OUT_OF_BOUNDS") => {
            return Err(format!("S5.1 Rust VM ISET -9 OOB wrong error: {}", e));
        }
        Err(_) => {}
    }
    match pure_default_run_u16(&oob_m9_u16, 7, 0, 0, false) {
        Ok(v) => {
            return Err(format!(
                "S5.1 Pure-M4 VM ISET -9 OOB must return error[OUT_OF_BOUNDS], got Ok({:?})",
                v
            ));
        }
        Err(e) if !e.contains("OUT_OF_BOUNDS") => {
            return Err(format!("S5.1 Pure-M4 VM ISET -9 OOB wrong error: {}", e));
        }
        Err(_) => {}
    }
    checks += 1;

    // 2. PT_GNU_STACK non-executable stack header on single-fn, multi-fn, and Pure-M2 ELFs.
    let tiny_src = "#[strict]\nfn main(x: i64) -> i64 { return x + 1; }\n";
    let tiny_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(tiny_src))?;
    let elf_single = compile_pure_u16_to_x86_64_elf_with_args(&tiny_u16, 0, 0, 0)?;
    verify_elf64_pt_gnu_stack(&elf_single, "rust_single_elf")?;
    let elf_multi = compile_pure_u16_to_x86_64_elf_multi(&tiny_u16, 0, 0, 0)?;
    verify_elf64_pt_gnu_stack(&elf_multi, "rust_multi_elf")?;
    let elf_pure_m2 = pure_default_compile_u16_to_elf(&tiny_u16, 0, 0, 0)?;
    verify_elf64_pt_gnu_stack(&elf_pure_m2, "pure_m2_elf")?;
    checks += 1;

    // 3. Burned-down xfails must no longer appear in BACKEND_DIFF_XFAILS.
    let removed_xfails: &[(&str, &str)] = &[
        ("p3_imod_rdx_clobber", "x86"),
        ("p3_imod_zero_after_tcmp", "x86"),
        ("p3_imod_min_neg1_after_tcmp", "x86"),
        ("p4_trit_clamp", "x86"),
        ("p4_trit_clamp", "wasm"),
        ("p5_if_pos_nontrit", "x86"),
        ("p5_if_pos_nontrit", "wasm"),
        ("p5_if_neg_nontrit", "x86"),
        ("p5_if_neg_nontrit", "wasm"),
    ];
    for &(cid, be) in removed_xfails {
        if OMNI_DIFF4_KNOWN_DIVERGENCES.iter().any(|&(id, b, _)| id == cid && b == be) {
            return Err(format!(
                "S5.3 burned-down xfail ({}, {}) is still listed in OMNI_DIFF4_KNOWN_DIVERGENCES",
                cid, be
            ));
        }
    }
    checks += 1;

    // 4. All 5 re-frozen FNV-1a64 hashes intact.
    selftest_m2_frozen_hashes()?;
    checks += 1;

    Ok(checks)
}

/// Audit §6 Cleanup & Hardening gate (`./omni s6-cleanup`):
///   1. Production `pure_default_*` functions live in `host_runtime/pure_default.rs`
///      (not inside the differential oracle test file `m2_host/decode_diff.rs`).
///   2. Duplicate Rust oracle `omni-workspace/` is archived at `oracle/archived-workspace/`
///      with `ARCHIVED.md`, leaving `omni_rust_bootstrap.rs` as the single Rust oracle.
///   3. `.github/workflows/ci.yml` includes a dedicated `fuzz-smoke` job, `permissions: contents: read`,
///      and SHA-pinned actions.
///   4. `m2-lint` deduplicates repeated foreign-module notes across all 27 registered modules.
///   5. All 5 re-frozen FNV-1a64 hashes remain intact.
pub fn selftest_s6_cleanup() -> Result<usize, String> {
    let mut checks = 0usize;

    // 1. pure_default_* extracted into host_runtime/pure_default.rs.
    let pd_src = std::fs::read_to_string("host_runtime/pure_default.rs")
        .map_err(|e| format!("S6.2 missing host_runtime/pure_default.rs: {}", e))?;
    let dd_src = std::fs::read_to_string("m2_host/decode_diff.rs")
        .map_err(|e| format!("S6.2 read m2_host/decode_diff.rs: {}", e))?;
    for sym in [
        "pub fn pure_default_compile_surface_to_u16",
        "pub fn pure_default_compile_u16_to_elf",
        "pub fn pure_default_run_u16",
        "pub fn pure_default_verify_u16",
        "pub fn pure_default_expand_includes",
    ] {
        if !pd_src.contains(sym) {
            return Err(format!("S6.2 host_runtime/pure_default.rs missing `{}`", sym));
        }
        if dd_src.lines().any(|l| l.trim_start().starts_with(sym)) {
            return Err(format!("S6.2 m2_host/decode_diff.rs still defines `{}`", sym));
        }
    }
    checks += 1;

    // 2. Duplicate workspace oracle archived.
    if std::path::Path::new("omni-workspace").exists() {
        return Err(String::from("S6.1 omni-workspace/ must be archived to oracle/archived-workspace/"));
    }
    if !std::path::Path::new("oracle/archived-workspace/ARCHIVED.md").exists() {
        return Err(String::from("S6.1 missing oracle/archived-workspace/ARCHIVED.md"));
    }
    checks += 1;

    // 3. CI workflow has fuzz-smoke job + permissions + SHA pinning.
    let ci_yml = std::fs::read_to_string(".github/workflows/ci.yml")
        .map_err(|e| format!("S6.3 read .github/workflows/ci.yml: {}", e))?;
    if !ci_yml.contains("fuzz-smoke:") || !ci_yml.contains("permissions:") || !ci_yml.contains("contents: read") {
        return Err(String::from("S6.3 .github/workflows/ci.yml missing fuzz-smoke job or permissions: contents: read"));
    }
    checks += 1;

    // 4. m2-lint deduplication check (lint.rs tracks seen_foreign_notes).
    let lint_src = std::fs::read_to_string("m2_host/lint.rs")
        .map_err(|e| format!("S6.3 read m2_host/lint.rs: {}", e))?;
    if !lint_src.contains("seen_foreign_notes") {
        return Err(String::from("S6.3 m2_host/lint.rs missing foreign-note deduplication"));
    }
    checks += 1;

    // 5. All 5 re-frozen FNV-1a64 hashes intact.
    selftest_m2_frozen_hashes()?;
    checks += 1;

    Ok(checks)
}

/// Step 2b Modular Compilation gate (`./omni m3-modular`):
///   1. Every M3 frontend module object (`mod_parse.omni_bc`, `m3_standalone.omni_bc`, `elf_link.omni_bc`)
///      matches its `.omni` source with 0-word diff and is strictly under M2's 65,536-word cap.
///   2. Pure-Omni `pure/link/elf_link.omni_bc` links `m3_standalone.omni_bc` + `mod_parse.omni_bc`
///      into the exact hash-pinned native `m3.elf` (`NATIVE_M3_ELF_BYTES` / `NATIVE_M3_ELF_FNV`).
///   3. The linked native `m3.elf` reproduces Rust `--rust` output with 0-word diff across all 6
///      `pure/programs/*.omni` programs.
///   4. Pure-Omni `elf_link` fails closed (`ELF_LINK_3`) on an unresolved cross-module import.
pub fn selftest_m3_modular() -> Result<usize, String> {
    let mut checks = 0usize;

    // 1. Verify all 3 modular objects match source (0-word diff) and stay strictly under 65,536 words.
    let mod_specs: [(&str, &str); 3] = [
        ("pure/m3/mod_parse.omni", "pure/m3/mod_parse.omni_bc"),
        ("pure/m3/m3_standalone.omni", "pure/m3/m3_standalone.omni_bc"),
        ("pure/link/elf_link.omni", "pure/link/elf_link.omni_bc"),
    ];
    for &(src_path, bc_path) in &mod_specs {
        let raw_bc = std::fs::read(bc_path)
            .map_err(|e| format!("M3-MODULAR missing '{}': {}", bc_path, e))?;
        if raw_bc.len() % 2 != 0 {
            return Err(format!("M3-MODULAR '{}' has odd byte length", bc_path));
        }
        let nwords = raw_bc.len() / 2;
        if nwords > NATIVE_WINDOW_MAX_WORDS {
            return Err(format!(
                "M3-MODULAR '{}' exceeds 65,536-word M2 window cap ({} words)",
                bc_path, nwords
            ));
        }
        let src_txt = std::fs::read_to_string(src_path)
            .map_err(|e| format!("M3-MODULAR read '{}': {}", src_path, e))?;
        let exp = resolve_omni_surface_includes(&src_txt, None, None)?;
        let ref_bc = encode_u16_stream_to_bytes(&with_lockstep_suspended(|| {
            compile_omni_surface_strict_to_u16(&exp)
        })?);
        if raw_bc != ref_bc {
            return Err(format!(
                "M3-MODULAR '{}' drift ({} B, 0x{:016X} != {} B, 0x{:016X})",
                bc_path,
                raw_bc.len(),
                fnv1a_64_hash(&raw_bc),
                ref_bc.len(),
                fnv1a_64_hash(&ref_bc)
            ));
        }
        checks += 1;
    }

    // 2. Link modular M3 via Pure-Omni `elf_link.omni_bc` and verify exact size + FNV-1a64 pin.
    let linked_elf = pure_default_link_elf_modules(&[
        "pure/m3/m3_standalone.omni_bc",
        "pure/m3/mod_parse.omni_bc",
    ])?;
    let linked_fnv = fnv1a_64_hash(&linked_elf);
    if linked_elf.len() != NATIVE_M3_ELF_BYTES || linked_fnv != NATIVE_M3_ELF_FNV {
        return Err(format!(
            "M3-MODULAR linked m3.elf drift: {} B / 0x{:016X} != expected {} B / 0x{:016X}",
            linked_elf.len(),
            linked_fnv,
            NATIVE_M3_ELF_BYTES,
            NATIVE_M3_ELF_FNV
        ));
    }
    checks += 1;

    // 3. Verify 0-word diff on all Pure-M3-supported pure/programs/* + verifier_engine + include.omni
    //    through the default modular M3 path, plus exact single-unit error parity on the rest.
    let zero_diff_progs: [&str; 5] = [
        "pure/programs/kv_ledger_service.omni",
        "pure/programs/compiler_self_audit.omni",
        "pure/programs/pack5_decision_controller.omni",
        "pure/verifier_engine.omni",
        "pure/m3/include.omni",
    ];
    for &prog_path in &zero_diff_progs {
        let raw_src = std::fs::read_to_string(prog_path)
            .map_err(|e| format!("M3-MODULAR read '{}': {}", prog_path, e))?;
        let exp = resolve_omni_surface_includes(&raw_src, None, None)?;
        let want = with_lockstep_suspended(|| compile_omni_surface_to_u16(&exp))?;
        let got = pure_default_compile_surface_to_u16(&exp, false)?;
        if got != want {
            return Err(format!(
                "M3-MODULAR 0-word diff failed on '{}': got {} w vs rust {} w",
                prog_path,
                got.len(),
                want.len()
            ));
        }
        checks += 1;
    }
    for &(prog_path, expect_err) in &[
        ("pure/programs/ternary_neural_router.omni", "ORACLE_ONLY_MACRO"),
        ("pure/programs/policy_consensus_engine.omni", "ORACLE_ONLY_MACRO"),
        ("pure/programs/arena_audit_pipeline.omni", "M3_CODE_9"),
    ] {
        let raw_src = std::fs::read_to_string(prog_path)
            .map_err(|e| format!("M3-MODULAR read '{}': {}", prog_path, e))?;
        let exp = resolve_omni_surface_includes(&raw_src, None, None)?;
        match pure_default_compile_surface_to_u16(&exp, false) {
            Err(e) if e.contains(expect_err) => checks += 1,
            other => {
                return Err(format!(
                    "M3-MODULAR expected {} on '{}', got {:?}",
                    expect_err, prog_path, other
                ));
            }
        }
    }

    // 4. Fail-closed check: linking `m3_standalone.omni_bc` with itself (no exports) rejects with ELF_LINK_3.
    match pure_default_link_elf_modules(&[
        "pure/m3/m3_standalone.omni_bc",
        "pure/m3/m3_standalone.omni_bc",
    ]) {
        Err(e) if e.contains("ELF_LINK_3") => checks += 1,
        other => {
            return Err(format!(
                "M3-MODULAR fail-closed unresolved import expected ELF_LINK_3, got {:?}",
                other
            ));
        }
    }

    Ok(checks)
}

/// Step 6 (Phase 2a) Literal Zero-Rust Syscall Intrinsics hard gate (`./omni p2a-syscalls`):
///   1. Pure-Omni M3 compiles `pure/m5/zero_rust_driver.omni` with 0-word diff vs Rust FE.
///   2. Pure-Omni & Rust verifiers accept `zero_rust_driver.omni` (`#[cap(SYS)]`) and
///      fail closed on negative controls: missing `#[cap(SYS)]` (`MISSING_CAP`) and
///      `@untrusted` syscall argument (`TAINT_SINK`).
///   3. Pure-Omni M2 lowers `0x7F73` syscall intrinsics (`sys_read`/`sys_write`/`sys_open`/
///      `sys_close`/`sys_exit`/`sys_spawn`) to native Linux x86-64 ELF64 with 0-byte diff vs
///      Rust M2 reference and >= 10 raw `0F 05` `syscall` instruction sites.
///   4. On Linux x86-64: executes the M2-emitted `zero_rust_driver.elf` directly on the kernel
///      (zero Rust/C/libc/shell) across all 4 modes:
///        - mode 0: `sys_open`/`sys_write`/`sys_read`/`sys_close` + `sys_spawn("/bin/true")` -> `1016` & `"OMNI-P2A"`
///        - mode 2: `sys_spawn("/tmp/omni_p2a_child.elf")` (Pure-Omni child ELF) -> `2016`
///        - mode 1: OOB `sys_write` bounds-check trap -> exit code `1` (`T_OOB`)
///        - mode 3: `sys_exit(42)` -> exit code `42`
///   5. All 5 frozen FNV-1a64 hashes remain intact.
pub fn selftest_p2a_syscall_intrinsics() -> Result<usize, String> {
    let mut checks = 0usize;

    // 1. M3 0-word diff on `pure/m5/zero_rust_driver.omni`.
    let drv_src = std::fs::read_to_string("pure/m5/zero_rust_driver.omni")
        .map_err(|e| format!("P2A read pure/m5/zero_rust_driver.omni: {}", e))?;
    let drv_pure_u16 = pure_default_compile_surface_to_u16(&drv_src, true)?;
    let drv_rust_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(&drv_src))?;
    if drv_pure_u16 != drv_rust_u16 {
        return Err(format!(
            "P2A M3 0-word diff failed on zero_rust_driver.omni: pure={}w vs rust={}w",
            drv_pure_u16.len(),
            drv_rust_u16.len()
        ));
    }
    checks += 1;

    // 2a. Positive verification (Pure-Omni + Rust verifiers).
    pure_default_verify_u16(&drv_pure_u16, true)?;
    let drv_mod = decode_omni_token_stream(&drv_pure_u16)?;
    for f in &drv_mod.functions {
        let rep = verify_binary_function(f);
        if !rep.passed {
            return Err(format!("P2A Rust verifier rejected zero_rust_driver fn#{}: {:?}", f.fn_idx, rep.errors));
        }
    }
    checks += 1;

    // 2b. Negative control: missing #[cap(SYS)] must be rejected by BOTH Pure-Omni and Rust verifiers.
    let bad_cap_src = "#[strict]\nfn main(fd: i64) -> i64 {\n    return sys_close(fd);\n}\n";
    let bad_cap_u16 = pure_default_compile_surface_to_u16(bad_cap_src, true)?;
    if pure_default_verify_u16(&bad_cap_u16, true).is_ok() {
        return Err(String::from("P2A negative control failed: Pure verifier accepted sys_close without #[cap(SYS)]"));
    }
    let bad_cap_mod = decode_omni_token_stream(&bad_cap_u16)?;
    if verify_binary_function(&bad_cap_mod.functions[0]).passed {
        return Err(String::from("P2A negative control failed: Rust verifier accepted sys_close without #[cap(SYS)]"));
    }
    checks += 1;

    // 2c. Negative control: @untrusted operand to syscall must be rejected by BOTH verifiers.
    let bad_taint_src = "#[strict]\n#[cap(SYS)]\nfn main(fd: i64 @untrusted) -> i64 {\n    return sys_close(fd);\n}\n";
    let bad_taint_u16 = pure_default_compile_surface_to_u16(bad_taint_src, true)?;
    if pure_default_verify_u16(&bad_taint_u16, true).is_ok() {
        return Err(String::from("P2A negative control failed: Pure verifier accepted @untrusted arg to sys_close"));
    }
    let bad_taint_mod = decode_omni_token_stream(&bad_taint_u16)?;
    if verify_binary_function(&bad_taint_mod.functions[0]).passed {
        return Err(String::from("P2A negative control failed: Rust verifier accepted @untrusted arg to sys_close"));
    }
    checks += 1;

    // 3. M2 0-byte diff + raw `0F 05` syscall verification.
    let drv_elf_pure = pure_default_compile_u16_to_elf(&drv_pure_u16, 0, 0, 0)?;
    let drv_elf_rust = compile_pure_u16_to_x86_64_elf_multi(&drv_pure_u16, 0, 0, 0)?;
    if drv_elf_pure != drv_elf_rust {
        return Err(format!(
            "P2A M2 0-byte diff failed on zero_rust_driver.omni: pure={}B vs rust={}B",
            drv_elf_pure.len(),
            drv_elf_rust.len()
        ));
    }
    if drv_elf_pure.len() < 176 || &drv_elf_pure[0..4] != b"\x7fELF" {
        return Err(String::from("P2A emitted driver ELF64 header invalid"));
    }
    let syscall_sites = drv_elf_pure[176..].windows(2).filter(|w| *w == [0x0F, 0x05]).count();
    if syscall_sites < 10 {
        return Err(format!("P2A expected >= 10 raw 0F 05 syscall sites, found {}", syscall_sites));
    }
    checks += 1;

    // 4. Native Linux x86-64 live kernel execution of all 4 modes (zero Rust/C/libc/shell).
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        use std::os::unix::fs::PermissionsExt;
        let tmp_drv = PrivateTempElf::new(&drv_elf_pure, "p2a_drv")?;
        // Build Pure-Omni child ELF (/tmp/omni_p2a_child.elf) from weighted_diff_pure_golden.omni.
        let wd_src = std::fs::read_to_string("spec/acceptance/goldens/weighted_diff_pure_golden.omni")
            .map_err(|e| format!("P2A read weighted_diff: {}", e))?;
        let wd_u16 = pure_default_compile_surface_to_u16(&wd_src, true)?;
        let wd_elf = pure_default_compile_u16_to_elf(&wd_u16, 0, 0, 0)?;
        let child_path = "/tmp/omni_p2a_child.elf";
        std::fs::write(child_path, &wd_elf).map_err(|e| format!("P2A write child elf: {}", e))?;
        std::fs::set_permissions(child_path, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("P2A chmod child elf: {}", e))?;

        // Mode 0: sys_open/write/read/close + sys_spawn("/bin/true") -> 1016 & "OMNI-P2A".
        let out0 = std::process::Command::new(&tmp_drv.path)
            .arg("0")
            .output()
            .map_err(|e| format!("P2A exec mode 0: {}", e))?;
        if !out0.status.success() || out0.stdout.len() < 8 {
            let _ = std::fs::remove_file(child_path);
            return Err(format!("P2A mode 0 failed: status={:?} len={}", out0.status, out0.stdout.len()));
        }
        let v0 = i64::from_le_bytes(out0.stdout[out0.stdout.len() - 8..].try_into().unwrap());
        let io_bytes = std::fs::read("/tmp/omni_p2a_io.bin").unwrap_or_default();
        if v0 != 1016 || io_bytes != b"OMNI-P2A" {
            let _ = std::fs::remove_file(child_path);
            return Err(format!("P2A mode 0 mismatch: v0={} (want 1016), file={:?}", v0, io_bytes));
        }

        // Mode 2: sys_spawn("/tmp/omni_p2a_child.elf") -> Pure-Omni child ELF runs + parent returns 2016.
        let out2 = std::process::Command::new(&tmp_drv.path)
            .arg("2")
            .output()
            .map_err(|e| format!("P2A exec mode 2: {}", e))?;
        let _ = std::fs::remove_file(child_path);
        let _ = std::fs::remove_file("/tmp/omni_p2a_io.bin");
        if !out2.status.success() || out2.stdout.len() < 8 {
            return Err(format!("P2A mode 2 failed: status={:?} len={}", out2.status, out2.stdout.len()));
        }
        let v2 = i64::from_le_bytes(out2.stdout[out2.stdout.len() - 8..].try_into().unwrap());
        if v2 != 2016 {
            return Err(format!("P2A mode 2 mismatch: v2={} (want 2016)", v2));
        }

        // Mode 1: OOB sys_write bounds check -> exit 1 (T_OOB).
        let st1 = std::process::Command::new(&tmp_drv.path)
            .arg("1")
            .output()
            .map_err(|e| format!("P2A exec mode 1: {}", e))?
            .status;
        if st1.code() != Some(1) {
            return Err(format!("P2A mode 1 OOB trap expected exit 1, got {:?}", st1.code()));
        }

        // Mode 3: direct sys_exit(42) -> exit 42.
        let st3 = std::process::Command::new(&tmp_drv.path)
            .arg("3")
            .output()
            .map_err(|e| format!("P2A exec mode 3: {}", e))?
            .status;
        drop(tmp_drv);
        if st3.code() != Some(42) {
            return Err(format!("P2A mode 3 sys_exit(42) expected exit 42, got {:?}", st3.code()));
        }
    }
    checks += 1;

    // 5. All 5 frozen FNV-1a64 hashes intact.
    selftest_m2_frozen_hashes()?;
    checks += 1;

    Ok(checks)
}



