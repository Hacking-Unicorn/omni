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

    // 3. 120-byte ELF64 header byte-exact vs Rust reference (compile_pure_u16_to_x86_64_elf_with_args)
    let ref_u16 = compile_omni_surface_strict_to_u16("#[strict]\nfn f(x: i64) -> i64 { return x + 1; }\n")?;
    let ref_elf = compile_pure_u16_to_x86_64_elf_with_args(&ref_u16, 0, 0, 0)?;
    let omni_hdr = run_h(1, ref_elf.len() as i64, 0)?;
    if omni_hdr.len() != 120 || omni_hdr.as_slice() != &ref_elf[..120] {
        return Err(format!(
            "M2-11 ELF64 header mismatch: len={} (want 120), first diff at {:?}",
            omni_hdr.len(),
            omni_hdr.iter().zip(ref_elf[..120].iter()).position(|(a, b)| a != b)
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
    if got_elf.len() != 106_242 || fnv1a_64_hash(&got_elf) != 0xEAFC81F46B482FF8 {
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

    Ok(checks)
}

/// M2-16: Self-hosted M2 compiler fixed-point verification (`pure/m2/m2.omni_bc`).
/// 1. Drift check: `RustFrontend(pure/m2/*.omni) == pure/m2/m2.omni_bc` (0-byte diff).
/// 2. Self-compile parity: `M2_vm(m2.omni_bc) == RustEmitMulti(m2.omni_bc)` (0-byte diff, S1 == V).
/// 3. Native Linux x86-64 triple fixed point: `S1(m2.omni_bc) -> S2`, `S2(m2.omni_bc) -> S3`,
///    asserting `S1 == S2 == S3` (prints explicit SKIPPED on non-Linux hosts per PLAN.md §4).
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
        use std::os::unix::fs::PermissionsExt;
        let p_bc = std::env::temp_dir().join("omni_m2_seed_fp.omni_bc");
        let p_s1 = std::env::temp_dir().join("omni_m2_s1.elf");
        let p_s2 = std::env::temp_dir().join("omni_m2_s2.elf");
        std::fs::write(&p_bc, &seed_bytes).map_err(|e| format!("write tmp bc: {}", e))?;
        std::fs::write(&p_s1, &s1).map_err(|e| format!("write tmp s1: {}", e))?;
        std::fs::set_permissions(&p_s1, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("chmod s1: {}", e))?;
        let out2 = std::process::Command::new(&p_s1)
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
        std::fs::write(&p_s2, &s2).map_err(|e| format!("write tmp s2: {}", e))?;
        std::fs::set_permissions(&p_s2, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("chmod s2: {}", e))?;
        let out3 = std::process::Command::new(&p_s2)
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
        let _ = std::fs::remove_file(&p_bc);
        let _ = std::fs::remove_file(&p_s1);
        let _ = std::fs::remove_file(&p_s2);
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
