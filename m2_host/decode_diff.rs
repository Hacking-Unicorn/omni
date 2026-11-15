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
