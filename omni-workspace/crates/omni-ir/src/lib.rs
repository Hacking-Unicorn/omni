//! Omni IR — binary AST structures, bounds-checked pure-u16 decoder, and module framing.

use omni_isa::*;

// 3. PURE NON-HUMAN TOKEN-STREAM DECODER (`Vec<u16>` -> BINARY AST)
// ==============================================================================

#[derive(Clone, Debug, PartialEq)]
pub enum BinOperand {
    Reg(u16),
    Param(u16),
    Label(u16),
    ImmInt(i64),
    ImmTrit(i8),
    EnglishSpan(Vec<u16>),
    SchemaRef(u16),
    FnRef(u16),
    SkillRef(u16),
    FieldKV(u16, Box<BinOperand>),
}

#[inline(always)]
pub fn extract_label_id(op: &BinOperand) -> Option<u16> {
    match *op {
        BinOperand::Label(lbl) => Some(lbl),
        BinOperand::ImmInt(v)  => Some((v & 0xFFFF) as u16),
        _ => None,
    }
}

#[derive(Clone, Debug)]
pub struct BinInstruction {
    pub offset: usize,
    pub dest_reg: Option<u16>,
    pub op_tok: u16,
    pub ty_tok: u16,
    pub tc_tok: u16,
    pub operands: Vec<BinOperand>,
}

#[derive(Clone, Debug)]
pub struct BinFunction {
    pub fn_idx: u16,
    pub params: Vec<(u16, u16, u16)>,
    pub ret_ty_tok: u16,
    pub cap_mask: CapMask,
    pub instructions: Vec<BinInstruction>,
}

#[derive(Clone, Debug, Default)]
pub struct BinModule {
    pub schemas: Vec<(u16, Vec<(u16, u16)>)>,
    pub experts: Vec<(u16, u16, u16, u16)>,
    pub functions: Vec<BinFunction>,
}

fn parse_one_operand(stream: &[u16], pos: &mut usize) -> Result<BinOperand, String> {
    if *pos >= stream.len() {
        return Err(String::from("Unexpected EOF while decoding binary operand"));
    }
    let tok = stream[*pos];
    *pos += 1;

    match tok {
        REG_BASE..=REG_MAX => Ok(BinOperand::Reg(tok)),
        PAR_BASE..=PAR_MAX => Ok(BinOperand::Param(tok)),
        LBL_BASE..=LBL_MAX => Ok(BinOperand::Label(tok)),
        TOK_TRIT_NEG  => Ok(BinOperand::ImmTrit(-1)),
        TOK_TRIT_ZERO => Ok(BinOperand::ImmTrit(0)),
        TOK_TRIT_POS  => Ok(BinOperand::ImmTrit(1)),
        TOK_IMM_I16 => {
            let v = *stream.get(*pos).ok_or("EOF in TOK_IMM_I16")? as i16;
            *pos += 1;
            Ok(BinOperand::ImmInt(v as i64))
        }
        TOK_IMM_I32 => {
            let hi = *stream.get(*pos).ok_or("EOF in TOK_IMM_I32 hi")? as u32;
            let lo = *stream.get(*pos + 1).ok_or("EOF in TOK_IMM_I32 lo")? as u32;
            *pos += 2;
            let combined = ((hi << 16) | lo) as i32;
            Ok(BinOperand::ImmInt(combined as i64))
        }
        TOK_ENG_SPAN => {
            let len = *stream.get(*pos).ok_or("EOF in TOK_ENG_SPAN len")? as usize;
            *pos += 1;
            if *pos + len > stream.len() {
                return Err(String::from("Truncated English token span"));
            }
            let mut span = Vec::with_capacity(len);
            for &t in &stream[*pos..*pos + len] {
                span.push(t & ENGLISH_MAX_TOKEN);
            }
            *pos += len;
            Ok(BinOperand::EnglishSpan(span))
        }
        TOK_SYM_SCHEMA => {
            let idx = *stream.get(*pos).ok_or("EOF in TOK_SYM_SCHEMA")?;
            *pos += 1;
            Ok(BinOperand::SchemaRef(idx))
        }
        TOK_SYM_FN => {
            let idx = *stream.get(*pos).ok_or("EOF in TOK_SYM_FN")?;
            *pos += 1;
            Ok(BinOperand::FnRef(idx))
        }
        TOK_SYM_SKILL => {
            let idx = *stream.get(*pos).ok_or("EOF in TOK_SYM_SKILL")?;
            *pos += 1;
            Ok(BinOperand::SkillRef(idx))
        }
        TOK_FIELD_KV => {
            let f_idx = *stream.get(*pos).ok_or("EOF in TOK_FIELD_KV")?;
            *pos += 1;
            let inner = parse_one_operand(stream, pos)?;
            Ok(BinOperand::FieldKV(f_idx, Box::new(inner)))
        }
        other => Err(format!("Invalid operand token 0x{:04X} at offset {}", other, *pos - 1)),
    }
}

pub fn decode_omni_token_stream(stream: &[u16]) -> Result<BinModule, String> {
    if stream.is_empty() || stream[0] != TOK_MOD_BEGIN {
        return Err(String::from("Missing 0x7EE0 (TOK_MOD_BEGIN) header token"));
    }
    let mut module = BinModule::default();
    let mut pos = 1usize;

    while pos < stream.len() {
        let tag = stream[pos];
        pos += 1;
        match tag {
            TOK_MOD_END => break,
            TOK_SCHEMA_DEF => {
                if pos + 2 > stream.len() {
                    return Err(String::from("Truncated TOK_SCHEMA_DEF header"));
                }
                let s_id = stream[pos];
                let f_cnt = stream[pos + 1] as usize;
                pos += 2;
                if pos + f_cnt * 2 > stream.len() {
                    return Err(String::from("Truncated TOK_SCHEMA_DEF fields"));
                }
                let mut fields = Vec::with_capacity(f_cnt);
                for _ in 0..f_cnt {
                    fields.push((stream[pos], stream[pos + 1]));
                    pos += 2;
                }
                module.schemas.push((s_id, fields));
            }
            TOK_EXPERT_DEF => {
                if pos + 4 > stream.len() {
                    return Err(String::from("Truncated TOK_EXPERT_DEF header"));
                }
                let e_id = stream[pos];
                let d_in = stream[pos + 1];
                let d_out = stream[pos + 2];
                let rank = stream[pos + 3];
                pos += 4;
                module.experts.push((e_id, d_in, d_out, rank));
            }
            TOK_FN_DEF => {
                if pos + 2 > stream.len() {
                    return Err(String::from("Truncated TOK_FN_DEF header"));
                }
                let fn_idx = stream[pos];
                let p_cnt = stream[pos + 1] as usize;
                pos += 2;
                if pos + p_cnt * 3 + 3 > stream.len() {
                    return Err(String::from("Truncated TOK_FN_DEF signature"));
                }
                let mut params = Vec::with_capacity(p_cnt);
                for _ in 0..p_cnt {
                    params.push((stream[pos], stream[pos + 1], stream[pos + 2]));
                    pos += 3;
                }
                let ret_ty_tok = stream[pos];
                let cap_mask = stream[pos + 1];
                let ins_cnt = stream[pos + 2] as usize;
                pos += 3;

                let mut instructions = Vec::with_capacity(ins_cnt);
                for _ in 0..ins_cnt {
                    if pos >= stream.len() {
                        return Err(String::from("Unexpected EOF inside function instruction body"));
                    }
                    let ins_offset = pos;
                    let first_tok = stream[pos];
                    let (dest_reg, op_tok, ty_tok, tc_tok) = if (REG_BASE..=REG_MAX).contains(&first_tok) {
                        if pos + 4 > stream.len() {
                            return Err(String::from("Truncated SSA instruction header"));
                        }
                        let d = first_tok;
                        let op = stream[pos + 1];
                        let ty = stream[pos + 2];
                        let tc = stream[pos + 3];
                        pos += 4;
                        (Some(d), op, ty, tc)
                    } else {
                        let op = first_tok;
                        pos += 1;
                        (None, op, TY_VOID, pack_tc(false, CAP_ID_PURE))
                    };

                    let mut operands = Vec::new();
                    while pos < stream.len() && stream[pos] != TOK_INS_END {
                        operands.push(parse_one_operand(stream, &mut pos)?);
                    }
                    if pos >= stream.len() || stream[pos] != TOK_INS_END {
                        return Err(format!("Missing 0x7EE4 (TOK_INS_END) for instruction at {}", ins_offset));
                    }
                    pos += 1;

                    instructions.push(BinInstruction {
                        offset: ins_offset,
                        dest_reg,
                        op_tok,
                        ty_tok,
                        tc_tok,
                        operands,
                    });
                }

                module.functions.push(BinFunction {
                    fn_idx,
                    params,
                    ret_ty_tok,
                    cap_mask,
                    instructions,
                });
            }
            other => {
                return Err(format!("Unexpected structural token 0x{:04X} at offset {}", other, pos - 1));
            }
        }
    }
    Ok(module)
}

// ==============================================================================

pub fn encode_u16_stream_to_bytes(stream: &[u16]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(stream.len() * 2);
    for &t in stream {
        bytes.extend_from_slice(&t.to_le_bytes());
    }
    bytes
}

pub fn decode_bytes_to_u16_stream(bytes: &[u8]) -> Vec<u16> {
    bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect()
}

pub fn format_non_human_token_stream(stream: &[u16]) -> String {
    let mut out = String::new();
    for (i, &tok) in stream.iter().enumerate() {
        if i > 0 {
            if i % 12 == 0 { out.push('\n'); } else { out.push(' '); }
        }
        out.push_str(&format!("{:04X}", tok));
    }
    out
}


pub fn pack_fn_module(
    schemas: &[(u16, Vec<(u16, u16)>)],
    fn_id: u16,
    params: &[(u16, u16, u16)],
    ret_ty: u16,
    cap_mask: CapMask,
    ins_packets: &[Vec<u16>],
) -> Vec<u16> {
    let mut stream = vec![TOK_MOD_BEGIN];
    for (s_id, fields) in schemas {
        stream.push(TOK_SCHEMA_DEF);
        stream.push(*s_id);
        stream.push(fields.len() as u16);
        for &(f_ty, f_tc) in fields {
            stream.push(f_ty);
            stream.push(f_tc);
        }
    }
    stream.push(TOK_FN_DEF);
    stream.push(fn_id);
    stream.push(params.len() as u16);
    for &(p_tok, p_ty, p_tc) in params {
        stream.push(p_tok);
        stream.push(p_ty);
        stream.push(p_tc);
    }
    stream.push(ret_ty);
    stream.push(cap_mask);
    stream.push(ins_packets.len() as u16);
    for pkt in ins_packets {
        stream.extend_from_slice(pkt);
    }
    stream.push(TOK_MOD_END);
    stream
}

