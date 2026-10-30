//! Omni Assembler — mnemonic text-to-pure-`u16` bytecode assembler supporting
//! multi-function `FN_DEF`, `CAP_MASK`, numeric, bitwise, balanced ternary, `SEL_TRIT`,
//! `PACK5`/`UNPACK5`/`TDOT`/`EGATE`/`TLOGIC`, inter-function `CALL_*`, byte buffers,
//! and control-flow (`LABEL`, `JMP`, `BR_POS`, `BR_NEG`, `BR_TRIT`).

use omni_isa::*;
use omni_ir::pack_fn_module;

/// Maps an upper-case type name (`"I64"`, `"TRIT"`, `"BYTES"`, `"TRIT_VEC"`, etc.) to its canonical `TY_*` token.
pub fn parse_asm_ty_tok(s: &str) -> u16 {
    match s {
        "I64" => TY_I64,
        "TRIT" => TY_TRIT,
        "STR" => TY_STR,
        "UUID" => TY_UUID,
        "BYTES" => TY_BYTES,
        "ARRAY" => TY_ARRAY,
        "TRIT_VEC" => TY_TRIT_VEC,
        "ARENA" => TY_ARENA,
        "DOM" => TY_DOM,
        "STRUCT" => TY_STRUCT,
        _ => TY_I64,
    }
}

/// Encodes a single instruction packet directly into binary `u16` tokens (`[dest?, op_tok, ..., TOK_INS_END]`).
pub fn encode_asm_ins_packet_u16(line: &str, ret_ty: u16) -> Result<Vec<u16>, String> {
    let tc_safe = pack_tc(false, CAP_ID_PURE);
    let parse_op_mnemonic = |m: &str, current_ret_ty: u16| -> Result<(u16, u16), String> {
        match m {
            "CINT"     | "MOV"        => Ok((0x7F06, TY_I64)),
            "CSTR"                    => Ok((0x7F08, TY_STR)),
            "CTRIT"                   => Ok((0x7F09, TY_TRIT)),
            "ARENA"                   => Ok((0x7F01, TY_ARENA)),
            "ANEW"                    => Ok((0x7F0D, TY_BYTES)),
            "IGET"                    => Ok((0x7F0E, TY_I64)),
            "ISET"                    => Ok((0x7F0F, TY_BYTES)),
            "SLEN"                    => Ok((0x7F50, TY_I64)),
            "SCAT"     | "SCONC"      => Ok((0x7F51, TY_STR)),
            "S2I"                     => Ok((0x7F55, TY_I64)),
            "I2S"                     => Ok((0x7F56, TY_STR)),
            "IADD"                    => Ok((0x7F10, TY_I64)),
            "ISUB"                    => Ok((0x7F11, TY_I64)),
            "IMUL"                    => Ok((0x7F12, TY_I64)),
            "IDIV"                    => Ok((0x7F13, TY_I64)),
            "IMOD"                    => Ok((0x7F14, TY_I64)),
            "INEG"                    => Ok((0x7F15, TY_I64)),
            "IABS"                    => Ok((0x7F16, TY_I64)),
            "IMIN"                    => Ok((0x7F17, TY_I64)),
            "IMAX"                    => Ok((0x7F18, TY_I64)),
            "BAND"                    => Ok((0x7F1E, TY_I64)),
            "BOR"                     => Ok((0x7F1F, TY_I64)),
            "BXOR"                    => Ok((0x7F20, TY_I64)),
            "BNOT"                    => Ok((0x7F21, TY_I64)),
            "SHL"                     => Ok((0x7F22, TY_I64)),
            "SHR"                     => Ok((0x7F23, TY_I64)),
            "CEQ"                     => Ok((0x7F24, TY_TRIT)),
            "CNE"                     => Ok((0x7F25, TY_TRIT)),
            "CLT"                     => Ok((0x7F26, TY_TRIT)),
            "CGT"                     => Ok((0x7F27, TY_TRIT)),
            "TNOT"                    => Ok((0x7F28, TY_TRIT)),
            "TAND"                    => Ok((0x7F29, TY_TRIT)),
            "TOR"                     => Ok((0x7F2A, TY_TRIT)),
            "TXOR"                    => Ok((0x7F2B, TY_TRIT)),
            "TMAJ"                    => Ok((0x7F2C, TY_TRIT)),
            "TCMP"                    => Ok((0x7F2D, TY_TRIT)),
            "PACK5"    | "TPACK5"     => Ok((0x7F30, TY_BYTES)),
            "UNPACK5"  | "TUNPACK5"   => Ok((0x7F31, TY_TRIT_VEC)),
            "TDOT"                    => Ok((0x7F34, TY_I64)),
            "EGATE"                   => Ok((0x7F35, TY_TRIT)),
            "LOOKAHEAD"               => Ok((0x7F36, TY_TRIT)),
            "TLOGIC"                  => Ok((0x7F3F, TY_TRIT)),
            "SEL_TRIT" | "CMOV"       => Ok((0x7F45, current_ret_ty)),
            "CALL"     | "CALL_I64"   => Ok((0x7F48, TY_I64)),
            "CALL_TRIT"               => Ok((0x7F48, TY_TRIT)),
            "CALL_BYTES"              => Ok((0x7F48, TY_BYTES)),
            "CALL_TRIT_VEC"           => Ok((0x7F48, TY_TRIT_VEC)),
            "CALL_STR"                => Ok((0x7F48, TY_STR)),
            "BORROW_R"                => Ok((0x7F02, TY_I64)),
            "BORROW_W"                => Ok((0x7F03, TY_I64)),
            "MOVE"                    => Ok((0x7F04, TY_I64)),
            "SAN_SQL"                 => Ok((0x7F57, TY_STR)),
            "SAN_DOM"                 => Ok((0x7F58, TY_STR)),
            "SAN_PRM"                 => Ok((0x7F59, TY_STR)),
            "FNV1A"                   => Ok((0x7F5D, TY_I64)),
            other => Err(format!("Unknown assembler mnemonic '{}'", other)),
        }
    };

    let push_operand_tok = |pkt: &mut Vec<u16>, tok_str: &str| -> Result<(), String> {
        if let Some(r_idx) = tok_str.strip_prefix('%') {
            let n = r_idx.parse::<u16>().map_err(|e| e.to_string())?;
            pkt.push(REG_BASE + n);
        } else if let Some(p_idx) = tok_str.strip_prefix('$') {
            let n = p_idx.parse::<u16>().map_err(|e| e.to_string())?;
            pkt.push(PAR_BASE + n);
        } else if let Some(f_idx) = tok_str.strip_prefix("@fn") {
            let n = f_idx.parse::<u16>().map_err(|e| e.to_string())?;
            pkt.push(TOK_SYM_FN);
            pkt.push(n);
        } else if let Some(hex_s) = tok_str.strip_prefix("@str:") {
            if hex_s.len() % 2 != 0 {
                return Err("Invalid @str hex length".into());
            }
            let byte_len = hex_s.len() / 2;
            pkt.push(TOK_ENG_SPAN);
            pkt.push(byte_len as u16);
            for i in 0..byte_len {
                let b = u8::from_str_radix(&hex_s[i * 2..i * 2 + 2], 16).map_err(|e| e.to_string())?;
                pkt.push((b as u16) & ENGLISH_MAX_TOKEN);
            }
        } else if tok_str == "+1" || tok_str == "TRIT_POS" {
            pkt.push(TOK_TRIT_POS);
        } else if tok_str == "0t" || tok_str == "TRIT_ZERO" {
            pkt.push(TOK_TRIT_ZERO);
        } else if tok_str == "-1t" || tok_str == "TRIT_NEG" {
            pkt.push(TOK_TRIT_NEG);
        } else {
            let val = tok_str.parse::<i32>().map_err(|e| e.to_string())?;
            if ((i16::MIN as i32)..=(i16::MAX as i32)).contains(&val) {
                pkt.push(TOK_IMM_I16);
                pkt.push((val as i16) as u16);
            } else {
                let u = val as u32;
                pkt.push(TOK_IMM_I32);
                pkt.push(((u >> 16) & 0xFFFF) as u16);
                pkt.push((u & 0xFFFF) as u16);
            }
        }
        Ok(())
    };

    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Empty instruction packet".into());
    }
    if parts[0] == "NOP" {
        Ok(vec![0x7F00, TOK_INS_END])
    } else if parts[0] == "DROP" && parts.len() >= 2 {
        let mut pkt = vec![0x7F05];
        push_operand_tok(&mut pkt, parts[1])?;
        pkt.push(TOK_INS_END);
        Ok(pkt)
    } else if parts[0] == "RET" && parts.len() >= 2 {
        let mut pkt = vec![0x7F4A];
        push_operand_tok(&mut pkt, parts[1])?;
        pkt.push(TOK_INS_END);
        Ok(pkt)
    } else if parts[0] == "LABEL" && parts.len() >= 2 {
        let lid = parts[1].parse::<u16>().map_err(|e| e.to_string())?;
        if lid > 15 { return Err("Label id > 15".into()); }
        Ok(vec![0x7F40, LBL_BASE + lid, TOK_INS_END])
    } else if parts[0] == "JMP" && parts.len() >= 2 {
        let lid = parts[1].parse::<u16>().map_err(|e| e.to_string())?;
        if lid > 15 { return Err("Label id > 15".into()); }
        Ok(vec![0x7F41, LBL_BASE + lid, TOK_INS_END])
    } else if parts[0] == "BR_POS" && parts.len() >= 3 {
        let mut pkt = vec![0x7F43];
        push_operand_tok(&mut pkt, parts[1])?;
        let lid = parts[2].parse::<u16>().map_err(|e| e.to_string())?;
        if lid > 15 { return Err("Label id > 15".into()); }
        pkt.push(LBL_BASE + lid);
        pkt.push(TOK_INS_END);
        Ok(pkt)
    } else if parts[0] == "BR_NEG" && parts.len() >= 3 {
        let mut pkt = vec![0x7F44];
        push_operand_tok(&mut pkt, parts[1])?;
        let lid = parts[2].parse::<u16>().map_err(|e| e.to_string())?;
        if lid > 15 { return Err("Label id > 15".into()); }
        pkt.push(LBL_BASE + lid);
        pkt.push(TOK_INS_END);
        Ok(pkt)
    } else if parts[0] == "BR_TRIT" && parts.len() >= 5 {
        let mut pkt = vec![0x7F42];
        push_operand_tok(&mut pkt, parts[1])?;
        for j in 2..=4 {
            let lid = parts[j].parse::<u16>().map_err(|e| e.to_string())?;
            if lid > 15 { return Err("Label id > 15".into()); }
            pkt.push(LBL_BASE + lid);
        }
        pkt.push(TOK_INS_END);
        Ok(pkt)
    } else if parts.len() >= 4 && parts[1] == "=" {
        let d_idx = parts[0].strip_prefix('%').ok_or("Expected %N destination")?.parse::<u16>().map_err(|e| e.to_string())?;
        let (op_mnem, untr_suffix) = if let Some(base) = parts[2].strip_suffix("@UNTRUSTED") {
            (base, true)
        } else {
            (parts[2], false)
        };
        let (op_tok, ty_tok) = parse_op_mnemonic(op_mnem, ret_ty)?;
        let ins_tc = if untr_suffix { pack_tc(true, CAP_ID_PURE) } else { tc_safe };
        let mut pkt = vec![REG_BASE + d_idx, op_tok, ty_tok, ins_tc];
        for &op_s in &parts[3..] {
            push_operand_tok(&mut pkt, op_s)?;
        }
        pkt.push(TOK_INS_END);
        Ok(pkt)
    } else {
        Err(format!("Cannot parse assembler line: '{}'", line))
    }
}

/// Minimal human-readable mnemonic assembler that emits a verified pure `u16` token stream (Roadmap 3.5).
/// Supports single-function and multi-function (`FN_DEF <id>`) modules, `CAP_MASK <u16>`, `@fn<id>` call targets,
/// `@PACK5` vector opcodes (`PACK5`, `UNPACK5`, `TDOT`), and decision hooks (`EGATE`, `TLOGIC`, `LOOKAHEAD`).
pub fn assemble_omni_text_to_u16(src: &str) -> Result<Vec<u16>, String> {
    let mut fn_id: u16 = 0x0000;
    let mut cap_mask: CapMask = 0x0000;
    let mut params: Vec<(u16, u16, u16)> = Vec::new();
    let mut ret_ty: u16 = TY_I64;
    let mut ins_packets: Vec<Vec<u16>> = Vec::new();
    let mut completed_fns: Vec<(u16, Vec<(u16, u16, u16)>, u16, CapMask, Vec<Vec<u16>>)> = Vec::new();
    let mut started_fn_def = false;

    for raw_line in src.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts[0] == "FN_DEF" && parts.len() >= 2 {
            if started_fn_def || !ins_packets.is_empty() || !params.is_empty() {
                completed_fns.push((
                    fn_id,
                    std::mem::take(&mut params),
                    ret_ty,
                    cap_mask,
                    std::mem::take(&mut ins_packets),
                ));
                ret_ty = TY_I64;
                cap_mask = 0x0000;
            }
            fn_id = parts[1].parse::<u16>().map_err(|e| e.to_string())?;
            started_fn_def = true;
        } else if parts[0] == "FN_ID" && parts.len() >= 2 {
            fn_id = parts[1].parse::<u16>().map_err(|e| e.to_string())?;
        } else if parts[0] == "CAP_MASK" && parts.len() >= 2 {
            cap_mask = parts[1].parse::<u16>().map_err(|e| e.to_string())?;
        } else if parts[0] == "PARAM" && parts.len() >= 3 {
            let p_idx = parts[1].strip_prefix('$').ok_or("Expected $N in PARAM")?.parse::<u16>().map_err(|e| e.to_string())?;
            let ty = parse_asm_ty_tok(parts[2]);
            let untr = parts.get(3).copied() == Some("UNTRUSTED");
            params.push((PAR_BASE + p_idx, ty, pack_tc(untr, CAP_ID_PURE)));
        } else if parts[0] == "RET_TY" && parts.len() >= 2 {
            ret_ty = parse_asm_ty_tok(parts[1]);
        } else {
            ins_packets.push(encode_asm_ins_packet_u16(line, ret_ty)?);
        }
    }

    if completed_fns.is_empty() {
        Ok(pack_fn_module(&[], fn_id, &params, ret_ty, cap_mask, &ins_packets))
    } else {
        completed_fns.push((fn_id, params, ret_ty, cap_mask, ins_packets));
        let mut stream = vec![TOK_MOD_BEGIN];
        for (f_id, f_params, f_ret, f_cap, f_ins) in completed_fns {
            stream.push(TOK_FN_DEF);
            stream.push(f_id);
            stream.push(f_params.len() as u16);
            for (pt, pty, ptc) in f_params {
                stream.push(pt);
                stream.push(pty);
                stream.push(ptc);
            }
            stream.push(f_ret);
            stream.push(f_cap);
            stream.push(f_ins.len() as u16);
            for pkt in f_ins {
                stream.extend_from_slice(&pkt);
            }
        }
        stream.push(TOK_MOD_END);
        Ok(stream)
    }
}

// ==============================================================================

/// Assembles a custom 9-instruction non-seed program exercising arithmetic, shifts, division,
/// ternary comparison, Kleene negation, and 4-operand `SEL_TRIT` branchless selection.
pub fn build_custom_non_seed_alu_program() -> Result<Vec<u16>, String> {
    let asm_src = "
        PARAM $0 I64 SAFE
        PARAM $1 I64 SAFE
        PARAM $2 I64 SAFE
        RET_TY I64
        %0 = IMUL $0 4
        %1 = ISUB %0 $1
        %2 = SHL $2 1
        %3 = IADD %1 %2
        %4 = IDIV %3 3
        %5 = TCMP %4 35
        %6 = TNOT %5
        %7 = SEL_TRIT %6 25 50 75
        %8 = IADD %4 %7
        RET %8
    ";
    assemble_omni_text_to_u16(asm_src)
}

/// Assembles a non-seed control-flow + backward-loop program computing `sum(1..=$0)` (`55` for `$0 = 10`).
pub fn build_custom_branch_loop_program() -> Result<Vec<u16>, String> {
    let asm_src = "
        PARAM $0 I64 SAFE
        RET_TY I64
        %0 = CINT 0
        %1 = CINT $0
        LABEL 1
        %0 = IADD %0 %1
        %1 = ISUB %1 1
        %2 = TCMP %1 0
        BR_POS %2 1
        RET %0
    ";
    assemble_omni_text_to_u16(asm_src)
}

