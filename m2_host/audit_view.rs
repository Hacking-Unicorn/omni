// ==============================================================================
// STEP 4: AUDIT VIEW — DETERMINISTIC M3 DECOMPILER & PROPOSER DIFF SUMMARY
// ==============================================================================
// Read-only audit view over raw `.omni_bc` token streams (`Vec<u16>`).
// Never enters the compile/verify loop and never modifies any pin or frozen hash.

/// Structured result of decompiling a raw `.omni_bc` token stream into readable M3 surface.
#[derive(Clone, Debug)]
pub struct DecompileReport {
    pub m3_source: String,
    pub is_exact: bool,
    pub functions_exact: usize,
    pub functions_approximate: usize,
    pub markers: Vec<String>,
}

fn audit_ty_name(ty_tok: u16) -> Option<&'static str> {
    match ty_tok {
        TY_I64 => Some("i64"),
        TY_TRIT => Some("trit"),
        TY_STR => Some("str"),
        TY_BYTES => Some("bytes"),
        TY_TRIT_VEC => Some("trit_vec"),
        TY_ARRAY => Some("array"),
        TY_ARENA => Some("arena"),
        TY_VOID => Some("void"),
        _ => None,
    }
}

fn audit_cap_names(mask: CapMask) -> Vec<&'static str> {
    const CAPS: &[(u16, &str)] = &[
        (CAP_ID_DB_R, "DB_R"),
        (CAP_ID_DB_W, "DB_W"),
        (CAP_ID_NET, "NET"),
        (CAP_ID_DOM, "DOM"),
        (CAP_ID_AI, "AI"),
        (CAP_ID_VISION, "VISION"),
        (CAP_ID_AUDIO, "AUDIO"),
        (CAP_ID_AUTH, "AUTH"),
        (CAP_ID_FS, "FS"),
        (CAP_ID_SYS, "SYS"),
    ];
    let mut out = Vec::new();
    for &(id, name) in CAPS {
        if (mask & cap_bit(id)) != 0 {
            out.push(name);
        }
    }
    out
}

fn audit_format_operand(op: &BinOperand) -> String {
    match op {
        BinOperand::Reg(r) => format!("v{}", r.saturating_sub(REG_BASE)),
        BinOperand::Param(p) => format!("p{}", p.saturating_sub(PAR_BASE)),
        BinOperand::Label(l) => format!("{}", l.saturating_sub(LBL_BASE)),
        BinOperand::ImmInt(v) => format!("{}", v),
        BinOperand::ImmTrit(t) => match *t {
            -1 => String::from("-1t"),
            0 => String::from("0t"),
            _ => String::from("+1t"),
        },
        BinOperand::EnglishSpan(toks) => {
            let bytes: Vec<u8> = toks.iter().map(|&t| (t & 0xFF) as u8).collect();
            let raw = String::from_utf8_lossy(&bytes);
            let mut esc = String::with_capacity(raw.len() + 2);
            esc.push('"');
            for ch in raw.chars() {
                match ch {
                    '\\' => esc.push_str("\\\\"),
                    '"' => esc.push_str("\\\""),
                    '\n' => esc.push_str("\\n"),
                    '\r' => esc.push_str("\\r"),
                    '\t' => esc.push_str("\\t"),
                    c => esc.push(c),
                }
            }
            esc.push('"');
            esc
        }
        BinOperand::FnRef(id) => format!("fn_{}", id),
        BinOperand::SchemaRef(id) => format!("@schema_{}", id),
        BinOperand::SkillRef(id) => format!("@skill_{}", id),
        BinOperand::FieldKV(k, v) => format!("field_{}:{}", k, audit_format_operand(v)),
    }
}

/// Formats an instruction's RHS as an M3 expression, returning `Some(expr)` when exact
/// or `None` when the opcode requires literal opcode fallback.
fn audit_format_rhs(ins: &BinInstruction) -> Option<String> {
    let ops = &ins.operands;
    let op0 = || ops.first().map(audit_format_operand);
    let op1 = || ops.get(1).map(audit_format_operand);
    let op2 = || ops.get(2).map(audit_format_operand);
    let op3 = || ops.get(3).map(audit_format_operand);
    match ins.op_tok {
        0x7F06 => op0(), // CINT / MOV
        0x7F08 => op0(), // CSTR
        0x7F09 => op0(), // CTRIT
        0x7F01 => Some(format!("arena({})", op0()?)),
        0x7F02 => Some(format!("borrow_r({})", op0()?)),
        0x7F03 => Some(format!("borrow_w({})", op0()?)),
        0x7F0D => Some(format!("anew({})", op0()?)),
        0x7F0E => {
            // IGET: 2-op iget(b, i) or 3-op ld_u8/ld_u16/ld_u32/ld_u64(b, off)
            if ops.len() == 2 {
                Some(format!("iget({}, {})", op0()?, op1()?))
            } else if ops.len() == 3 {
                if let BinOperand::ImmInt(w) = ops[2] {
                    let name = match w {
                        -1 => "ld_u8",
                        -2 => "ld_u16",
                        -4 => "ld_u32",
                        -8 => "ld_u64",
                        _ => return None,
                    };
                    Some(format!("{}({}, {})", name, op0()?, op1()?))
                } else {
                    None
                }
            } else {
                None
            }
        }
        0x7F0F => {
            // ISET: 3-op iset_i32(-4)/iset_i64(-8), 4-op st_u8/16/32/64, or variadic iset(b, ...)
            if ops.len() == 3 {
                if let BinOperand::ImmInt(tag) = ops[1] {
                    if tag == -4 {
                        return Some(format!("iset_i32({}, {})", op0()?, op2()?));
                    } else if tag == -8 {
                        return Some(format!("iset_i64({}, {})", op0()?, op2()?));
                    }
                }
            }
            if ops.len() == 4 {
                if let BinOperand::ImmInt(tag) = ops[1] {
                    let name = match tag {
                        -1 => Some("st_u8"),
                        -2 => Some("st_u16"),
                        -4 => Some("st_u32"),
                        -8 => Some("st_u64"),
                        _ => None,
                    };
                    if let Some(st_name) = name {
                        return Some(format!("{}({}, {}, {})", st_name, op0()?, op2()?, op3()?));
                    }
                }
            }
            if ops.len() >= 2 {
                let args: Vec<String> = ops.iter().map(audit_format_operand).collect();
                Some(format!("iset({})", args.join(", ")))
            } else {
                None
            }
        }
        0x7F10 => Some(format!("{} + {}", op0()?, op1()?)),
        0x7F11 => Some(format!("{} - {}", op0()?, op1()?)),
        0x7F12 => Some(format!("{} * {}", op0()?, op1()?)),
        0x7F13 => Some(format!("{} / {}", op0()?, op1()?)),
        0x7F14 => Some(format!("{} % {}", op0()?, op1()?)),
        0x7F15 => Some(format!("0 - {}", op0()?)),
        0x7F16 => Some(format!("iabs({})", op0()?)),
        0x7F17 => Some(format!("imin({}, {})", op0()?, op1()?)),
        0x7F18 => Some(format!("imax({}, {})", op0()?, op1()?)),
        0x7F1E => Some(format!("{} & {}", op0()?, op1()?)),
        0x7F1F => Some(format!("{} | {}", op0()?, op1()?)),
        0x7F20 => Some(format!("{} ^ {}", op0()?, op1()?)),
        0x7F21 => Some(format!("bnot({})", op0()?)),
        0x7F22 => Some(format!("{} << {}", op0()?, op1()?)),
        0x7F23 => Some(format!("{} >> {}", op0()?, op1()?)),
        0x7F24 => Some(format!("{} == {}", op0()?, op1()?)),
        0x7F25 => Some(format!("{} != {}", op0()?, op1()?)),
        0x7F26 => Some(format!("{} < {}", op0()?, op1()?)),
        0x7F27 => Some(format!("{} > {}", op0()?, op1()?)),
        0x7F28 => Some(format!("!{}", op0()?)),
        0x7F29 => Some(format!("tand({}, {})", op0()?, op1()?)),
        0x7F2A => Some(format!("tor({}, {})", op0()?, op1()?)),
        0x7F2B => Some(format!("txor({}, {})", op0()?, op1()?)),
        0x7F2C => Some(format!("tmaj({}, {}, {})", op0()?, op1()?, op2()?)),
        0x7F2D => Some(format!("{} <=> {}", op0()?, op1()?)),
        0x7F30 => Some(format!("pack5({})", op0()?)),
        0x7F31 => Some(format!("unpack5({}, {})", op0()?, op1()?)),
        0x7F34 => Some(format!("tdot({}, {})", op0()?, op1()?)),
        0x7F35 => Some(format!("egate({})", op0()?)),
        0x7F3F => Some(format!("tlogic({}, {}, {})", op0()?, op1()?, op2()?)),
        0x7F45 => Some(format!("sel_trit({}, {}, {}, {})", op0()?, op1()?, op2()?, op3()?)),
        0x7F48 => {
            // CALL: first operand is FnRef(id), remaining operands are call arguments
            if let Some(BinOperand::FnRef(fid)) = ops.first() {
                let args: Vec<String> = ops[1..].iter().map(audit_format_operand).collect();
                Some(format!("fn_{}({})", fid, args.join(", ")))
            } else {
                None
            }
        }
        0x7F50 => Some(format!("slen({})", op0()?)),
        0x7F51 => Some(format!("scat({}, {})", op0()?, op1()?)),
        0x7F55 => Some(format!("s2i({})", op0()?)),
        0x7F56 => Some(format!("i2s({})", op0()?)),
        0x7F57 => Some(format!("san_sql({})", op0()?)),
        0x7F58 => Some(format!("san_dom({})", op0()?)),
        0x7F59 => Some(format!("san_prm({})", op0()?)),
        0x7F5D => Some(format!("fnv1a({})", op0()?)),
        0x7F64 => Some(format!("db_get({})", op0()?)),
        0x7F65 => Some(format!("db_query({}, {})", op0()?, op1()?)),
        0x7F68 => Some(format!("db_count({})", op0()?)),
        0x7F6B => Some(format!("kv_put({}, {})", op0()?, op1()?)),
        0x7F6D => Some(format!("req.param({})", op0()?)),
        0x7F6E => Some(format!("respond({}, {})", op0()?, op1()?)),
        0x7F6F => Some(format!("respond_html({}, {})", op0()?, op1()?)),
        0x7F70 => Some(format!("auth_verify({})", op0()?)),
        _ => None,
    }
}

fn audit_label_id(ins: &BinInstruction, op_idx: usize) -> Option<u16> {
    ins.operands.get(op_idx).and_then(|o| match *o {
        BinOperand::Label(l) => Some(l.saturating_sub(LBL_BASE)),
        BinOperand::ImmInt(v) => Some((v & 0xFFFF) as u16),
        _ => None,
    })
}

fn audit_find_label(insns: &[BinInstruction], start: usize, end: usize, lbl: u16) -> Option<usize> {
    (start..end).find(|&k| insns[k].op_tok == 0x7F40 && audit_label_id(&insns[k], 0) == Some(lbl))
}

/// Recognizes a tail expression for an `if_pos`/`if_neg` arm writing `dst`:
/// either a single tail instruction writing `dst`, OR a nested `if_pos`/`if_neg`
/// whose `JMP` targets `outer_end_lbl` and whose two arms both write `dst`.
fn audit_arm_tail_expr(
    insns: &[BinInstruction],
    start: usize,
    end: usize,
    dst: u16,
    outer_end_lbl: Option<u16>,
) -> Option<(usize, String)> {
    if start >= end {
        return None;
    }
    // Check nested tail `if_pos`/`if_neg` sharing `outer_end_lbl`:
    //   BR_POS/BR_NEG cond L_inner_then
    //   <inner_else: br+1 .. jmp_idx> (writes dst)
    //   JMP outer_end_lbl
    //   LABEL L_inner_then
    //   <inner_then: lbl+1 .. end> (writes dst)
    if let Some(l_outer) = outer_end_lbl {
        for br_idx in start..end {
            let b = &insns[br_idx];
            if (b.op_tok == 0x7F43 || b.op_tok == 0x7F44) && b.operands.len() >= 2 {
                if let Some(l_then) = audit_label_id(b, 1) {
                    if let Some(then_lbl_idx) = audit_find_label(insns, br_idx + 1, end, l_then) {
                        if then_lbl_idx >= br_idx + 2
                            && insns[then_lbl_idx - 1].op_tok == 0x7F41
                            && audit_label_id(&insns[then_lbl_idx - 1], 0) == Some(l_outer)
                        {
                            let ie_start = br_idx + 1;
                            let ie_end = then_lbl_idx - 1;
                            let it_start = then_lbl_idx + 1;
                            let it_end = end;
                            if ie_start + 1 == ie_end && it_start + 1 == it_end {
                                if insns[ie_start].dest_reg == Some(dst)
                                    && insns[it_start].dest_reg == Some(dst)
                                {
                                    if let (Some(e_rhs), Some(t_rhs)) = (
                                        audit_format_rhs(&insns[ie_start]),
                                        audit_format_rhs(&insns[it_start]),
                                    ) {
                                        let kw = if b.op_tok == 0x7F43 { "if_pos" } else { "if_neg" };
                                        let cond_s = audit_format_operand(&b.operands[0]);
                                        return Some((
                                            br_idx,
                                            format!("{} {} {{ {} }} else {{ {} }}", kw, cond_s, t_rhs, e_rhs),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let last = &insns[end - 1];
    if last.dest_reg == Some(dst) {
        let rhs = audit_format_rhs(last)?;
        Some((end - 1, rhs))
    } else {
        None
    }
}

fn audit_emit_single_ins(
    ins: &BinInstruction,
    declared: &mut BTreeSet<u16>,
    indent: &str,
    out: &mut String,
    approx: &mut bool,
) {
    match ins.op_tok {
        0x7F40 => {
            // LABEL
            if let Some(lbl) = audit_label_id(ins, 0) {
                out.push_str(&format!("{}label {};\n", indent, lbl));
            } else {
                *approx = true;
                out.push_str(&format!("{}// [LITERAL_OP] LABEL {:?}\n", indent, ins.operands));
            }
        }
        0x7F41 => {
            // JMP
            if let Some(lbl) = audit_label_id(ins, 0) {
                out.push_str(&format!("{}goto {};\n", indent, lbl));
            } else {
                *approx = true;
                out.push_str(&format!("{}// [LITERAL_OP] JMP {:?}\n", indent, ins.operands));
            }
        }
        0x7F05 => {
            // DROP
            if let Some(op) = ins.operands.first() {
                out.push_str(&format!("{}drop({});\n", indent, audit_format_operand(op)));
            } else {
                *approx = true;
                out.push_str(&format!("{}// [LITERAL_OP] DROP {:?}\n", indent, ins.operands));
            }
        }
        0x7F4A => {
            // RET
            if let Some(op) = ins.operands.first() {
                out.push_str(&format!("{}return {};\n", indent, audit_format_operand(op)));
            } else {
                out.push_str(&format!("{}return;\n", indent));
            }
        }
        0x7F42 | 0x7F43 | 0x7F44 => {
            // Unstructured branch fallback — never guess silently!
            *approx = true;
            let op_name = decode_opcode_token(ins.op_tok)
                .map(|s| s.debug_label)
                .unwrap_or("BR_UNKNOWN");
            let args: Vec<String> = ins.operands.iter().map(audit_format_operand).collect();
            out.push_str(&format!(
                "{}// [LITERAL_OP @0x{:04X}] {} {}\n",
                indent,
                ins.offset,
                op_name,
                args.join(" ")
            ));
        }
        _ => {
            if let (Some(dst), Some(rhs)) = (ins.dest_reg, audit_format_rhs(ins)) {
                let reg_idx = dst.saturating_sub(REG_BASE);
                if declared.insert(dst) {
                    let ty_s = audit_ty_name(ins.ty_tok).unwrap_or("i64");
                    out.push_str(&format!("{}let v{}: {} = {};\n", indent, reg_idx, ty_s, rhs));
                } else {
                    out.push_str(&format!("{}v{} = {};\n", indent, reg_idx, rhs));
                }
            } else {
                *approx = true;
                let op_name = decode_opcode_token(ins.op_tok)
                    .map(|s| s.debug_label)
                    .unwrap_or("OP_UNKNOWN");
                let dst_s = ins
                    .dest_reg
                    .map(|d| format!("v{} = ", d.saturating_sub(REG_BASE)))
                    .unwrap_or_default();
                let args: Vec<String> = ins.operands.iter().map(audit_format_operand).collect();
                out.push_str(&format!(
                    "{}// [LITERAL_OP @0x{:04X}] {}{} (0x{:04X}) {}\n",
                    indent,
                    ins.offset,
                    dst_s,
                    op_name,
                    ins.op_tok,
                    args.join(", ")
                ));
            }
        }
    }
}

/// Recursively reconstructs a structured instruction slice `[start..end)`.
/// Never elides or hides any register assignment (including B6 `vX = 0;` clobbers).
fn audit_emit_block(
    insns: &[BinInstruction],
    start: usize,
    end: usize,
    declared: &mut BTreeSet<u16>,
    depth: usize,
    out: &mut String,
    approx: &mut bool,
) {
    let indent = "    ".repeat(depth);
    let mut i = start;
    while i < end {
        let ins = &insns[i];

        // Pattern 1 & 2: structured `if_pos` / `if_neg`
        //   BR_POS/BR_NEG cond L_then
        //   <else_arm: i+1 .. jmp_idx>
        //   JMP L_end
        //   LABEL L_then
        //   <then_arm: then_lbl_idx+1 .. end_lbl_idx>
        //   LABEL L_end
        if (ins.op_tok == 0x7F43 || ins.op_tok == 0x7F44) && ins.operands.len() >= 2 {
            let kw = if ins.op_tok == 0x7F43 { "if_pos" } else { "if_neg" };
            let cond_s = audit_format_operand(&ins.operands[0]);
            if let Some(l_then) = audit_label_id(ins, 1) {
                if let Some(then_lbl_idx) = audit_find_label(insns, i + 1, end, l_then) {
                    if then_lbl_idx >= i + 2 && insns[then_lbl_idx - 1].op_tok == 0x7F41 {
                        let jmp_idx = then_lbl_idx - 1;
                        if let Some(l_end) = audit_label_id(&insns[jmp_idx], 0) {
                            if let Some(end_lbl_idx) =
                                audit_find_label(insns, then_lbl_idx + 1, end, l_end)
                            {
                                let else_start = i + 1;
                                let else_end = jmp_idx;
                                let then_start = then_lbl_idx + 1;
                                let then_end = end_lbl_idx;

                                // Check if both arms end by assigning the same join register `dst`
                                let join_dst = if else_start < else_end && then_start < then_end {
                                    let ed = insns[else_end - 1].dest_reg;
                                    let td = insns[then_end - 1].dest_reg;
                                    if ed.is_some() && ed == td { ed } else { None }
                                } else {
                                    None
                                };

                                if let Some(dst) = join_dst {
                                    if let (Some((t_tail_idx, t_rhs)), Some((e_tail_idx, e_rhs))) = (
                                        audit_arm_tail_expr(insns, then_start, then_end, dst, Some(l_end)),
                                        audit_arm_tail_expr(insns, else_start, else_end, dst, Some(l_end)),
                                    ) {
                                        let reg_idx = dst.saturating_sub(REG_BASE);
                                        let ty_s = audit_ty_name(insns[then_end - 1].ty_tok)
                                            .unwrap_or("i64");
                                        let is_new = declared.insert(dst);
                                        let lhs = if is_new {
                                            format!("let v{}: {} = ", reg_idx, ty_s)
                                        } else {
                                            format!("v{} = ", reg_idx)
                                        };
                                        let mut decl_then = declared.clone();
                                        let mut decl_else = declared.clone();
                                        if then_start == t_tail_idx && else_start == e_tail_idx {
                                            out.push_str(&format!(
                                                "{}{}{} {} {{ {} }} else {{ {} }};\n",
                                                indent, lhs, kw, cond_s, t_rhs, e_rhs
                                            ));
                                        } else {
                                            out.push_str(&format!(
                                                "{}{}{} {} {{\n",
                                                indent, lhs, kw, cond_s
                                            ));
                                            audit_emit_block(
                                                insns,
                                                then_start,
                                                t_tail_idx,
                                                &mut decl_then,
                                                depth + 1,
                                                out,
                                                approx,
                                            );
                                            out.push_str(&format!(
                                                "{}    {}\n{}}} else {{\n",
                                                indent, t_rhs, indent
                                            ));
                                            audit_emit_block(
                                                insns,
                                                else_start,
                                                e_tail_idx,
                                                &mut decl_else,
                                                depth + 1,
                                                out,
                                                approx,
                                            );
                                            out.push_str(&format!(
                                                "{}    {}\n{}}};\n",
                                                indent, e_rhs, indent
                                            ));
                                        }
                                        i = end_lbl_idx + 1;
                                        continue;
                                    }
                                }

                                // Statement-form `if_pos`/`if_neg` (preserves every write verbatim)
                                out.push_str(&format!("{}{} {} {{\n", indent, kw, cond_s));
                                let mut decl_then = declared.clone();
                                audit_emit_block(
                                    insns,
                                    then_start,
                                    then_end,
                                    &mut decl_then,
                                    depth + 1,
                                    out,
                                    approx,
                                );
                                out.push_str(&format!("{}}} else {{\n", indent));
                                let mut decl_else = declared.clone();
                                audit_emit_block(
                                    insns,
                                    else_start,
                                    else_end,
                                    &mut decl_else,
                                    depth + 1,
                                    out,
                                    approx,
                                );
                                out.push_str(&format!("{}}}\n", indent));
                                i = end_lbl_idx + 1;
                                continue;
                            }
                        }
                    }
                }
            }
        }

        // Pattern 3: structured `match_trit cond { neg => ..., zero => ..., pos => ... }`
        //   BR_TRIT cond L_neg L_zero L_pos
        //   LABEL L_neg <neg_arm> JMP L_end
        //   LABEL L_zero <zero_arm> JMP L_end
        //   LABEL L_pos <pos_arm>
        //   LABEL L_end
        if ins.op_tok == 0x7F42 && ins.operands.len() >= 4 {
            let cond_s = audit_format_operand(&ins.operands[0]);
            if let (Some(l_neg), Some(l_zero), Some(l_pos)) = (
                audit_label_id(ins, 1),
                audit_label_id(ins, 2),
                audit_label_id(ins, 3),
            ) {
                if let (Some(neg_lbl), Some(zero_lbl), Some(pos_lbl)) = (
                    audit_find_label(insns, i + 1, end, l_neg),
                    audit_find_label(insns, i + 1, end, l_zero),
                    audit_find_label(insns, i + 1, end, l_pos),
                ) {
                    if neg_lbl == i + 1
                        && zero_lbl > neg_lbl + 1
                        && pos_lbl > zero_lbl + 1
                        && insns[zero_lbl - 1].op_tok == 0x7F41
                        && insns[pos_lbl - 1].op_tok == 0x7F41
                    {
                        let l_end_a = audit_label_id(&insns[zero_lbl - 1], 0);
                        let l_end_b = audit_label_id(&insns[pos_lbl - 1], 0);
                        if l_end_a.is_some() && l_end_a == l_end_b {
                            if let Some(end_lbl) =
                                audit_find_label(insns, pos_lbl + 1, end, l_end_a.unwrap())
                            {
                                let neg_s = neg_lbl + 1;
                                let neg_e = zero_lbl - 1;
                                let zero_s = zero_lbl + 1;
                                let zero_e = pos_lbl - 1;
                                let pos_s = pos_lbl + 1;
                                let pos_e = end_lbl;
                                let join_dst = if neg_s < neg_e
                                    && zero_s < zero_e
                                    && pos_s < pos_e
                                {
                                    let d0 = insns[neg_e - 1].dest_reg;
                                    let d1 = insns[zero_e - 1].dest_reg;
                                    let d2 = insns[pos_e - 1].dest_reg;
                                    if d0.is_some() && d0 == d1 && d1 == d2 { d0 } else { None }
                                } else {
                                    None
                                };
                                if let Some(dst) = join_dst {
                                    if let (
                                        Some((nt, nr)),
                                        Some((zt, zr)),
                                        Some((pt, pr)),
                                    ) = (
                                        audit_arm_tail_expr(insns, neg_s, neg_e, dst, None),
                                        audit_arm_tail_expr(insns, zero_s, zero_e, dst, None),
                                        audit_arm_tail_expr(insns, pos_s, pos_e, dst, None),
                                    ) {
                                        if neg_s == nt && zero_s == zt && pos_s == pt {
                                            let reg_idx = dst.saturating_sub(REG_BASE);
                                            let ty_s = audit_ty_name(insns[neg_e - 1].ty_tok)
                                                .unwrap_or("i64");
                                            let is_new = declared.insert(dst);
                                            let lhs = if is_new {
                                                format!("let v{}: {} = ", reg_idx, ty_s)
                                            } else {
                                                format!("v{} = ", reg_idx)
                                            };
                                            out.push_str(&format!(
                                                "{}{}match_trit {} {{ neg => {}, zero => {}, pos => {} }};\n",
                                                indent, lhs, cond_s, nr, zr, pr
                                            ));
                                            i = end_lbl + 1;
                                            continue;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Pattern 4: structured `do_while { body } cond_expr;`
        //   LABEL L_head
        //   <body>
        //   vC = TCMP a b
        //   BR_POS vC L_head
        if ins.op_tok == 0x7F40 {
            if let Some(l_head) = audit_label_id(ins, 0) {
                if let Some(br_idx) = (i + 1..end).find(|&k| {
                    insns[k].op_tok == 0x7F43
                        && insns[k].operands.len() >= 2
                        && audit_label_id(&insns[k], 1) == Some(l_head)
                }) {
                    // Ensure no other branch targets L_head after br_idx
                    let later_target = (br_idx + 1..end).any(|k| {
                        matches!(insns[k].op_tok, 0x7F41 | 0x7F42 | 0x7F43 | 0x7F44)
                            && insns[k].operands.iter().any(|o| {
                                matches!(*o, BinOperand::Label(l) if l.saturating_sub(LBL_BASE) == l_head)
                            })
                    });
                    if !later_target && br_idx >= i + 2 {
                        let cond_ins = &insns[br_idx - 1];
                        if let (Some(cond_dst), BinOperand::Reg(br_reg)) =
                            (cond_ins.dest_reg, &insns[br_idx].operands[0])
                        {
                            if cond_dst == *br_reg {
                                if let Some(cond_rhs) = audit_format_rhs(cond_ins) {
                                    out.push_str(&format!("{}do_while {{\n", indent));
                                    audit_emit_block(
                                        insns,
                                        i + 1,
                                        br_idx - 1,
                                        declared,
                                        depth + 1,
                                        out,
                                        approx,
                                    );
                                    out.push_str(&format!("{}}} {};\n", indent, cond_rhs));
                                    i = br_idx + 1;
                                    continue;
                                }
                            }
                        }
                    }
                }
            }
        }

        audit_emit_single_ins(ins, declared, &indent, out, approx);
        i += 1;
    }
}

/// Decompiles a raw `.omni_bc` token stream into deterministic, readable M3 surface text.
/// No source map is required. Marks reconstructed vs approximate sections explicitly.
pub fn decompile_u16_stream_to_m3(stream: &[u16]) -> Result<DecompileReport, String> {
    let module = decode_omni_token_stream(stream)?;
    let stream_hash = fnv1a_64_hash(&encode_u16_stream_to_bytes(stream));
    let mut fn_blocks: Vec<(bool, String, String)> = Vec::with_capacity(module.functions.len());
    let mut functions_exact = 0usize;
    let mut functions_approximate = 0usize;
    let mut markers: Vec<String> = Vec::new();

    for (pos_idx, func) in module.functions.iter().enumerate() {
        let rep = verify_binary_function(func);
        let mut approx = false;
        let mut body = String::new();
        let mut declared: BTreeSet<u16> = BTreeSet::new();

        audit_emit_block(
            &func.instructions,
            0,
            func.instructions.len(),
            &mut declared,
            1,
            &mut body,
            &mut approx,
        );

        let mut header = String::new();
        let marker = if approx {
            functions_approximate += 1;
            format!(
                "fn_{}:APPROXIMATE (literal opcode/branch fallback)",
                func.fn_idx
            )
        } else {
            functions_exact += 1;
            format!("fn_{}:RECONSTRUCTED", func.fn_idx)
        };
        header.push_str(&format!(
            "// [AUDIT-VIEW: {}] fn_idx={} ins={} cap_mask=0x{:04X} verified={}\n",
            if approx {
                "APPROXIMATE — contains literal opcode/branch fallback"
            } else {
                "RECONSTRUCTED"
            },
            func.fn_idx,
            func.instructions.len(),
            func.cap_mask,
            rep.passed
        ));
        if func.cap_mask != 0 {
            let caps = audit_cap_names(func.cap_mask);
            if !caps.is_empty() {
                header.push_str(&format!("#[cap({})]\n", caps.join(", ")));
            }
        }
        if func.fn_idx as usize != pos_idx {
            header.push_str(&format!("#[fn_id({})]\n", func.fn_idx));
        }

        let mut param_strs = Vec::with_capacity(func.params.len());
        for &(p_tok, ty_tok, tc_tok) in &func.params {
            let p_idx = p_tok.saturating_sub(PAR_BASE);
            let ty_s = audit_ty_name(ty_tok).unwrap_or("i64");
            let (untrusted, _, _) = unpack_tc(tc_tok);
            if untrusted {
                param_strs.push(format!("p{}: {} @untrusted", p_idx, ty_s));
            } else {
                param_strs.push(format!("p{}: {}", p_idx, ty_s));
            }
        }
        let ret_s = audit_ty_name(func.ret_ty_tok).unwrap_or("i64");
        header.push_str(&format!(
            "fn fn_{}({}) -> {} {{\n{}}}\n",
            func.fn_idx,
            param_strs.join(", "),
            ret_s,
            body
        ));
        markers.push(marker.clone());
        fn_blocks.push((approx, marker, header));
    }

    let is_exact = functions_approximate == 0;
    let mut m3_source = String::new();
    m3_source.push_str(&format!(
        "// === OMNI AUDIT VIEW (M3 DECOMPILER) ===\n// Status: {} | words={} | fnv1a64=0x{:016X} | functions={} (exact={}, approximate={})\n",
        if is_exact {
            "RECONSTRUCTED (exact)"
        } else {
            "APPROXIMATE (contains literal fallback)"
        },
        stream.len(),
        stream_hash,
        module.functions.len(),
        functions_exact,
        functions_approximate
    ));
    for (_, _, block) in fn_blocks {
        m3_source.push('\n');
        m3_source.push_str(&block);
    }

    Ok(DecompileReport {
        m3_source,
        is_exact,
        functions_exact,
        functions_approximate,
        markers,
    })
}

// ==============================================================================
// PLAIN-LANGUAGE PROPOSER DIFF SUMMARY & GATEABLE INVARIANTS
// ==============================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FnAuditSnapshot {
    pub fn_idx: u16,
    pub ins_count: usize,
    pub cap_mask: CapMask,
    pub tainted_param_mask: u32,
    pub arena_bytes: i64,
    pub buf_ops: usize,
    pub verified: bool,
    pub body_hash: u64,
}

fn audit_snapshot_function(func: &BinFunction) -> FnAuditSnapshot {
    let rep = verify_binary_function(func);
    let mut tainted_param_mask = 0u32;
    for (idx, &(_, _, tc_tok)) in func.params.iter().enumerate() {
        let (untr, _, _) = unpack_tc(tc_tok);
        if untr && idx < 32 {
            tainted_param_mask |= 1u32 << idx;
        }
    }
    let mut arena_bytes = 0i64;
    let mut buf_ops = 0usize;
    let mut hash_buf: Vec<u8> = Vec::with_capacity(func.instructions.len() * 12);
    hash_buf.extend_from_slice(&func.fn_idx.to_le_bytes());
    hash_buf.extend_from_slice(&func.ret_ty_tok.to_le_bytes());
    hash_buf.extend_from_slice(&func.cap_mask.to_le_bytes());
    for &(pt, ty, tc) in &func.params {
        hash_buf.extend_from_slice(&pt.to_le_bytes());
        hash_buf.extend_from_slice(&ty.to_le_bytes());
        hash_buf.extend_from_slice(&tc.to_le_bytes());
    }
    for ins in &func.instructions {
        if ins.op_tok == 0x7F01 {
            if let Some(BinOperand::ImmInt(sz)) = ins.operands.first() {
                arena_bytes = arena_bytes.saturating_add(*sz);
            }
        }
        if matches!(ins.op_tok, 0x7F0D | 0x7F0E | 0x7F0F) {
            buf_ops += 1;
        }
        hash_buf.extend_from_slice(&ins.dest_reg.unwrap_or(0xFFFF).to_le_bytes());
        hash_buf.extend_from_slice(&ins.op_tok.to_le_bytes());
        hash_buf.extend_from_slice(&ins.ty_tok.to_le_bytes());
        hash_buf.extend_from_slice(&ins.tc_tok.to_le_bytes());
        for op in &ins.operands {
            match op {
                BinOperand::Reg(v) => {
                    hash_buf.push(1);
                    hash_buf.extend_from_slice(&v.to_le_bytes());
                }
                BinOperand::Param(v) => {
                    hash_buf.push(2);
                    hash_buf.extend_from_slice(&v.to_le_bytes());
                }
                BinOperand::Label(v) => {
                    hash_buf.push(3);
                    hash_buf.extend_from_slice(&v.to_le_bytes());
                }
                BinOperand::ImmInt(v) => {
                    hash_buf.push(4);
                    hash_buf.extend_from_slice(&v.to_le_bytes());
                }
                BinOperand::ImmTrit(v) => {
                    hash_buf.push(5);
                    hash_buf.push(*v as u8);
                }
                BinOperand::EnglishSpan(ts) => {
                    hash_buf.push(6);
                    for &t in ts {
                        hash_buf.extend_from_slice(&t.to_le_bytes());
                    }
                }
                BinOperand::SchemaRef(v) => {
                    hash_buf.push(7);
                    hash_buf.extend_from_slice(&v.to_le_bytes());
                }
                BinOperand::FnRef(v) => {
                    hash_buf.push(8);
                    hash_buf.extend_from_slice(&v.to_le_bytes());
                }
                BinOperand::SkillRef(v) => {
                    hash_buf.push(9);
                    hash_buf.extend_from_slice(&v.to_le_bytes());
                }
                BinOperand::FieldKV(k, _) => {
                    hash_buf.push(10);
                    hash_buf.extend_from_slice(&k.to_le_bytes());
                }
            }
        }
    }
    FnAuditSnapshot {
        fn_idx: func.fn_idx,
        ins_count: func.instructions.len(),
        cap_mask: func.cap_mask,
        tainted_param_mask,
        arena_bytes,
        buf_ops,
        verified: rep.passed,
        body_hash: fnv1a_64_hash(&hash_buf),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposerDiffSummary {
    pub before_label: String,
    pub after_label: String,
    pub before_words: usize,
    pub after_words: usize,
    pub delta_words: isize,
    pub before_fnv1a64: u64,
    pub after_fnv1a64: u64,
    pub hash_changed: bool,
    pub functions_added: Vec<u16>,
    pub functions_removed: Vec<u16>,
    pub functions_modified: Vec<u16>,
    pub functions_unchanged: Vec<u16>,
    pub caps_added: Vec<&'static str>,
    pub caps_removed: Vec<&'static str>,
    pub cap_escalation: bool,
    pub taint_changed: bool,
    pub before_verified: bool,
    pub after_verified: bool,
    pub ins_delta: isize,
    pub arena_bytes_delta: i64,
    pub buf_ops_delta: isize,
    pub step_budget_delta: i64,
    pub mem_budget_delta: i64,
}

/// Builds a deterministic plain-language + gateable diff summary between two `.omni_bc` streams.
pub fn summarize_proposer_diff(
    before: &[u16],
    after: &[u16],
    before_label: &str,
    after_label: &str,
) -> Result<ProposerDiffSummary, String> {
    let before_mod = decode_omni_token_stream(before)?;
    let after_mod = decode_omni_token_stream(after)?;

    let before_fnv1a64 = fnv1a_64_hash(&encode_u16_stream_to_bytes(before));
    let after_fnv1a64 = fnv1a_64_hash(&encode_u16_stream_to_bytes(after));

    let mut before_map: BTreeMap<u16, FnAuditSnapshot> = BTreeMap::new();
    let mut after_map: BTreeMap<u16, FnAuditSnapshot> = BTreeMap::new();
    for f in &before_mod.functions {
        before_map.insert(f.fn_idx, audit_snapshot_function(f));
    }
    for f in &after_mod.functions {
        after_map.insert(f.fn_idx, audit_snapshot_function(f));
    }

    let mut all_ids: BTreeSet<u16> = BTreeSet::new();
    all_ids.extend(before_map.keys().copied());
    all_ids.extend(after_map.keys().copied());

    let mut functions_added = Vec::new();
    let mut functions_removed = Vec::new();
    let mut functions_modified = Vec::new();
    let mut functions_unchanged = Vec::new();
    let mut before_caps_union: CapMask = 0;
    let mut after_caps_union: CapMask = 0;
    let mut cap_escalation = false;
    let mut taint_changed = false;
    let mut before_ins = 0usize;
    let mut after_ins = 0usize;
    let mut before_arena = 0i64;
    let mut after_arena = 0i64;
    let mut before_buf = 0usize;
    let mut after_buf = 0usize;
    let mut before_verified = true;
    let mut after_verified = true;

    for id in all_ids {
        match (before_map.get(&id), after_map.get(&id)) {
            (None, Some(a)) => {
                functions_added.push(id);
                after_caps_union |= a.cap_mask;
                if a.cap_mask != 0 {
                    cap_escalation = true;
                }
                if a.tainted_param_mask != 0 {
                    taint_changed = true;
                }
                after_ins += a.ins_count;
                after_arena = after_arena.saturating_add(a.arena_bytes);
                after_buf += a.buf_ops;
                after_verified &= a.verified;
            }
            (Some(b), None) => {
                functions_removed.push(id);
                before_caps_union |= b.cap_mask;
                if b.tainted_param_mask != 0 {
                    taint_changed = true;
                }
                before_ins += b.ins_count;
                before_arena = before_arena.saturating_add(b.arena_bytes);
                before_buf += b.buf_ops;
                before_verified &= b.verified;
            }
            (Some(b), Some(a)) => {
                before_caps_union |= b.cap_mask;
                after_caps_union |= a.cap_mask;
                if (a.cap_mask & !b.cap_mask) != 0 {
                    cap_escalation = true;
                }
                if a.tainted_param_mask != b.tainted_param_mask {
                    taint_changed = true;
                }
                if b.body_hash == a.body_hash {
                    functions_unchanged.push(id);
                } else {
                    functions_modified.push(id);
                }
                before_ins += b.ins_count;
                after_ins += a.ins_count;
                before_arena = before_arena.saturating_add(b.arena_bytes);
                after_arena = after_arena.saturating_add(a.arena_bytes);
                before_buf += b.buf_ops;
                after_buf += a.buf_ops;
                before_verified &= b.verified;
                after_verified &= a.verified;
            }
            (None, None) => {}
        }
    }

    let caps_added = audit_cap_names(after_caps_union & !before_caps_union);
    let caps_removed = audit_cap_names(before_caps_union & !after_caps_union);
    let ins_delta = after_ins as isize - before_ins as isize;
    let arena_bytes_delta = after_arena - before_arena;
    let buf_ops_delta = after_buf as isize - before_buf as isize;

    Ok(ProposerDiffSummary {
        before_label: before_label.to_string(),
        after_label: after_label.to_string(),
        before_words: before.len(),
        after_words: after.len(),
        delta_words: after.len() as isize - before.len() as isize,
        before_fnv1a64,
        after_fnv1a64,
        hash_changed: before_fnv1a64 != after_fnv1a64 || before != after,
        functions_added,
        functions_removed,
        functions_modified,
        functions_unchanged,
        caps_added,
        caps_removed,
        cap_escalation,
        taint_changed,
        before_verified,
        after_verified,
        ins_delta,
        arena_bytes_delta,
        buf_ops_delta,
        // Static proxy deltas for step/mem budgets when no explicit #[step_budget]/#[mem_budget] override is supplied
        step_budget_delta: ins_delta as i64,
        mem_budget_delta: arena_bytes_delta,
    })
}

impl ProposerDiffSummary {
    /// Renders a human-readable plain-language audit summary for a proposer diff.
    pub fn render_plain_language(&self) -> String {
        let fmt_ids = |ids: &[u16]| -> String {
            if ids.is_empty() {
                String::from("none")
            } else {
                ids.iter()
                    .map(|id| format!("fn_{}", id))
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        };
        let fmt_caps = |caps: &[&str]| -> String {
            if caps.is_empty() {
                String::from("none")
            } else {
                caps.join(", ")
            }
        };
        format!(
            "=== OMNI PROPOSER DIFF SUMMARY ===\n\
             Before:          {} ({} words, FNV-1a64=0x{:016X}, verified={})\n\
             After:           {} ({} words, FNV-1a64=0x{:016X}, verified={})\n\
             Seed/Hash Delta: {:+} words (hash_changed={})\n\
             Functions:       added=[{}], removed=[{}], modified=[{}], unchanged={}\n\
             Capabilities:    added=[{}], removed=[{}], escalation={}\n\
             Taint Surface:   changed={}\n\
             Budget Delta:    ins={:+}, step_budget={:+}, arena_bytes={:+}, mem_budget={:+}, buf_ops={:+}\n",
            self.before_label,
            self.before_words,
            self.before_fnv1a64,
            self.before_verified,
            self.after_label,
            self.after_words,
            self.after_fnv1a64,
            self.after_verified,
            self.delta_words,
            self.hash_changed,
            fmt_ids(&self.functions_added),
            fmt_ids(&self.functions_removed),
            fmt_ids(&self.functions_modified),
            self.functions_unchanged.len(),
            fmt_caps(&self.caps_added),
            fmt_caps(&self.caps_removed),
            self.cap_escalation,
            self.taint_changed,
            self.ins_delta,
            self.step_budget_delta,
            self.arena_bytes_delta,
            self.mem_budget_delta,
            self.buf_ops_delta,
        )
    }

    /// Renders a deterministic, machine-checkable canonical TSV form for gating.
    pub fn render_canonical_tsv(&self) -> String {
        let fmt_ids = |ids: &[u16]| -> String {
            ids.iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(",")
        };
        format!(
            "words_before\t{}\n\
             words_after\t{}\n\
             delta_words\t{}\n\
             fnv1a64_before\t0x{:016X}\n\
             fnv1a64_after\t0x{:016X}\n\
             hash_changed\t{}\n\
             fn_added\t{}\n\
             fn_removed\t{}\n\
             fn_modified\t{}\n\
             fn_unchanged_count\t{}\n\
             caps_added\t{}\n\
             caps_removed\t{}\n\
             cap_escalation\t{}\n\
             taint_changed\t{}\n\
             verified_before\t{}\n\
             verified_after\t{}\n\
             ins_delta\t{}\n\
             step_budget_delta\t{}\n\
             arena_bytes_delta\t{}\n\
             mem_budget_delta\t{}\n\
             buf_ops_delta\t{}\n",
            self.before_words,
            self.after_words,
            self.delta_words,
            self.before_fnv1a64,
            self.after_fnv1a64,
            self.hash_changed,
            fmt_ids(&self.functions_added),
            fmt_ids(&self.functions_removed),
            fmt_ids(&self.functions_modified),
            self.functions_unchanged.len(),
            self.caps_added.join(","),
            self.caps_removed.join(","),
            self.cap_escalation,
            self.taint_changed,
            self.before_verified,
            self.after_verified,
            self.ins_delta,
            self.step_budget_delta,
            self.arena_bytes_delta,
            self.mem_budget_delta,
            self.buf_ops_delta,
        )
    }
}

fn audit_load_stream_arg(target: &str) -> Result<Vec<u16>, String> {
    if target.ends_with(".omni_bc") && std::path::Path::new(target).is_file() {
        let raw = std::fs::read(target).map_err(|e| format!("failed to read '{}': {}", target, e))?;
        return Ok(decode_bytes_to_u16_stream(&raw));
    }
    if target.ends_with(".omni") && std::path::Path::new(target).is_file() {
        let raw = std::fs::read_to_string(target)
            .map_err(|e| format!("failed to read '{}': {}", target, e))?;
        let parent = std::path::Path::new(target).parent();
        let expanded = resolve_omni_surface_includes(&raw, parent, None)?;
        return compile_omni_surface_to_u16(&expanded);
    }
    compile_omni_surface_to_u16(target)
}

/// CLI handler for `omnic decompile <file.omni_bc | file.omni | "source">` (and `inspect --m3`).
pub fn run_omni_decompile_cli(argv: &[String]) -> i32 {
    if argv.is_empty() {
        eprintln!("usage: omnic decompile <file.omni_bc | file.omni | \"source\">");
        return 2;
    }
    let stream = match audit_load_stream_arg(&argv[0]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[DECOMPILE ERROR] {}", e);
            return 1;
        }
    };
    match decompile_u16_stream_to_m3(&stream) {
        Ok(rep) => {
            print!("{}", rep.m3_source);
            0
        }
        Err(e) => {
            eprintln!("[DECOMPILE ERROR] {}", e);
            1
        }
    }
}

/// CLI handler for `omnic audit-diff <before> <after> [--tsv]`.
pub fn run_omni_audit_diff_cli(argv: &[String]) -> i32 {
    let tsv = argv.iter().any(|a| a == "--tsv");
    let pos: Vec<&String> = argv.iter().filter(|a| !a.starts_with("--")).collect();
    if pos.len() < 2 {
        eprintln!("usage: omnic audit-diff <before.omni_bc|before.omni> <after.omni_bc|after.omni> [--tsv]");
        return 2;
    }
    let before = match audit_load_stream_arg(pos[0]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[AUDIT-DIFF ERROR] {}", e);
            return 1;
        }
    };
    let after = match audit_load_stream_arg(pos[1]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[AUDIT-DIFF ERROR] {}", e);
            return 1;
        }
    };
    match summarize_proposer_diff(&before, &after, pos[0], pos[1]) {
        Ok(sum) => {
            if tsv {
                print!("{}", sum.render_canonical_tsv());
            } else {
                print!("{}", sum.render_plain_language());
            }
            0
        }
        Err(e) => {
            eprintln!("[AUDIT-DIFF ERROR] {}", e);
            1
        }
    }
}

/// Step 4 permanent selftest gate (`omnic audit-view`):
/// 1. Exact round-trip byte-identity on existing goldens (compile -> decompile -> recompile == original Vec<u16>).
/// 2. Round-trip & determinism on `pure/verifier_standalone.omni_bc` (15,402w verifier engine).
/// 3. B6 miscompile shape: verifies decompiler shows the clobbered `v0 = 0;` assignment, never hides it.
/// 4. Approximate fallback marking on irreducible CFG / literal opcode streams.
/// 5. Proposer diff summary invariants: reproducibility, identity, cap escalation, taint change,
///    budget/arena/word/hash deltas, and antisymmetry.
pub fn selftest_audit_view() -> Result<usize, String> {
    let mut checks = 0usize;

    // 1. Exact round-trip byte-identity on existing surface goldens (no source map required).
    let roundtrip_goldens: &[(&str, &str)] = &[
        (
            "seed0_risk",
            "fn seed0_risk(base: i64, delta: i64) -> trit { let scaled = base * 2; let score = scaled + delta; let cmp = score <=> 50; let pos = +1t; let res = tand(cmp, pos); return res; }",
        ),
        (
            "seed4_consensus",
            "#[fn_id(4)] fn seed4_consensus(s0: trit, s1: trit, s2: trit) -> trit { let m = tmaj(s0, s1, s2); let res = !m; return res; }",
        ),
        (
            "seed6_clamp",
            "#[fn_id(6)] fn seed6_clamp(x: i64, lo: i64, hi: i64) -> i64 { let lower = imax(x, lo); let clamped = imin(lower, hi); return clamped; }",
        ),
        (
            "custom_non_seed_alu",
            "fn custom_non_seed_alu(a: i64, b: i64, c: i64) -> i64 { let r0 = a * 4; let r1 = r0 - b; let r2 = c << 1; let r3 = r1 + r2; let r4 = r3 / 3; let r5 = r4 <=> 35; let r6 = !r5; let r7 = sel_trit(r6, 25, 50, 75); let r8 = r4 + r7; return r8; }",
        ),
        (
            "iabs_golden",
            "fn iabs_golden(x: i64) -> i64 { let r = iabs(x); return r; }",
        ),
        (
            "idiv_zero",
            "fn idiv_zero(x: i64) -> i64 { let r = x / 0; return r; }",
        ),
        (
            "shl_65",
            "fn shl_65(x: i64) -> i64 { let r = x << 65; return r; }",
        ),
        (
            "sel_mix",
            "fn sel_mix(x: i64) -> i64 { let prod = x * 3; let cmp = prod <=> 20; let bonus = sel_trit(cmp, 5, 10, 15); let res = prod + bonus; return res; }",
        ),
        (
            "br_pos_golden",
            "fn br_pos_golden(x: i64) -> i64 { let c = x <=> 0; let r = if_pos c { 200 } else { 100 }; return r; }",
        ),
        (
            "br_neg_arith_golden",
            "fn br_neg_arith_golden(x: i64) -> i64 { let c = x <=> 0; let r = if_neg c { x - 50 } else { x + 100 }; return r; }",
        ),
        (
            "nested_branch_golden",
            "fn nested_branch_golden(x: i64) -> i64 { let c0 = x <=> 0; let r = if_pos c0 { let c1 = x <=> 5; if_pos c1 { 30 } else { 20 } } else { 10 }; return r; }",
        ),
        (
            "match_trit_golden",
            "fn match_trit_golden(x: i64) -> i64 { let c = x <=> 10; let r = match_trit c { neg => x - 5, zero => 100, pos => x + 50 }; return r; }",
        ),
        (
            "branch_loop_golden",
            "fn branch_loop_golden(n: i64) -> i64 { let acc = 0; let i = n; do_while { acc = acc + i; i = i - 1; } i <=> 0; return acc; }",
        ),
        (
            "byte_buf_golden",
            "fn byte_buf_golden(x: i64) -> i64 { let buf = anew(0); buf = iset(buf, 10, 20, 30); buf = iset_i32(buf, 100); buf = iset_i64(buf, 200); let len = slen(buf); let cmp = x <=> 5; let sel = cmov(cmp, 100, 200, 300); let total = len + sel; return total; }",
        ),
        (
            "decision_hook_golden",
            "#[cap(AI)] fn decision_hook_golden(a: i64, b: i64) -> trit { let delta = a - b; let gate = egate(delta); return gate; }",
        ),
        (
            "arena_lifecycle_golden",
            "fn arena_lifecycle_golden(x: i64) -> i64 { let a = arena(4096); let r = borrow_r(a); let res = x + 42; drop(r); drop(a); return res; }",
        ),
    ];

    for &(name, src) in roundtrip_goldens {
        let orig_u16 = compile_omni_surface_to_u16(src)
            .map_err(|e| format!("audit-view golden '{}' compile failed: {}", name, e))?;
        let rep1 = decompile_u16_stream_to_m3(&orig_u16)
            .map_err(|e| format!("audit-view golden '{}' decompile failed: {}", name, e))?;
        let rep2 = decompile_u16_stream_to_m3(&orig_u16)?;
        if rep1.m3_source != rep2.m3_source {
            return Err(format!("audit-view golden '{}' decompile non-deterministic", name));
        }
        if !rep1.is_exact || !rep1.m3_source.contains("RECONSTRUCTED") {
            return Err(format!(
                "audit-view golden '{}' expected exact reconstruction, got:\n{}",
                name, rep1.m3_source
            ));
        }
        let recompiled = compile_omni_surface_to_u16(&rep1.m3_source).map_err(|e| {
            format!(
                "audit-view golden '{}' decompiled M3 failed to recompile: {}\n--- DECOMPILED ---\n{}",
                name, e, rep1.m3_source
            )
        })?;
        if recompiled != orig_u16 {
            return Err(format!(
                "audit-view golden '{}' round-trip stream mismatch (orig={}w, recompiled={}w)\n--- DECOMPILED ---\n{}",
                name,
                orig_u16.len(),
                recompiled.len(),
                rep1.m3_source
            ));
        }
        checks += 1;
    }

    // 2. Round-trip / determinism on `pure/verifier_standalone.omni_bc` (the full verifier engine).
    let ve_raw = std::fs::read("pure/verifier_standalone.omni_bc")
        .map_err(|e| format!("audit-view: read pure/verifier_standalone.omni_bc: {}", e))?;
    let ve_u16 = decode_bytes_to_u16_stream(&ve_raw);
    let ve_rep1 = decompile_u16_stream_to_m3(&ve_u16)?;
    let ve_rep2 = decompile_u16_stream_to_m3(&ve_u16)?;
    if ve_rep1.m3_source != ve_rep2.m3_source {
        return Err(String::from("audit-view: verifier_engine decompile non-deterministic"));
    }
    let ve_mod = decode_omni_token_stream(&ve_u16)?;
    if ve_rep1.functions_exact + ve_rep1.functions_approximate != ve_mod.functions.len()
        || ve_mod.functions.is_empty()
    {
        return Err(String::from("audit-view: verifier_engine function count mismatch"));
    }
    if ve_rep1.is_exact {
        // When marked exact, it must re-compile cleanly to a byte-identical stream.
        let ve_recompiled = compile_omni_surface_to_u16(&ve_rep1.m3_source).map_err(|e| {
            format!("audit-view: verifier_engine marked exact but failed to recompile: {}", e)
        })?;
        if ve_recompiled != ve_u16 {
            return Err(format!(
                "audit-view: verifier_engine marked exact but recompile differed ({}w vs {}w)",
                ve_recompiled.len(),
                ve_u16.len()
            ));
        }
    } else if !ve_rep1.m3_source.contains("APPROXIMATE") || !ve_rep1.m3_source.contains("LITERAL_OP") {
        return Err(String::from(
            "audit-view: verifier_engine approximate output missing explicit APPROXIMATE/LITERAL_OP markers",
        ));
    }
    checks += 1;

    // 3. B6 miscompile shape: clobbered `v0 = 0;` (`CINT 0`) inside `else` arm before `fn_0(v0, 1024 + p1)`.
    //    The decompiled view MUST show `v0 = 0;` explicitly and NEVER hide the clobber.
    let tc_safe = pack_tc(false, CAP_ID_PURE);
    let b6_miscompile = pack_fn_module(
        &[],
        0,
        &[(PAR_BASE, TY_I64, tc_safe), (PAR_BASE + 1, TY_I64, tc_safe)],
        TY_I64,
        0,
        &[
            // v0 = p0 + 9000  (live variable `w`)
            vec![REG_BASE, 0x7F10, TY_I64, tc_safe, PAR_BASE, TOK_IMM_I16, 9000u16, TOK_INS_END],
            // v1 = p1 <=> 0
            vec![REG_BASE + 1, 0x7F2D, TY_TRIT, tc_safe, PAR_BASE + 1, TOK_IMM_I16, 0, TOK_INS_END],
            // BR_POS v1 L1
            vec![0x7F43, REG_BASE + 1, LBL_BASE + 1, TOK_INS_END],
            // [BUGGY B6 ELSE ARM]: v0 = CINT 0  <-- clobbers live v0!
            vec![REG_BASE, 0x7F06, TY_I64, tc_safe, TOK_IMM_I16, 0, TOK_INS_END],
            // v2 = 1024 + p1
            vec![REG_BASE + 2, 0x7F10, TY_I64, tc_safe, TOK_IMM_I16, 1024, PAR_BASE + 1, TOK_INS_END],
            // v0 = v0 + v2  (reads clobbered v0 == 0 instead of 9000!)
            vec![REG_BASE, 0x7F10, TY_I64, tc_safe, REG_BASE, REG_BASE + 2, TOK_INS_END],
            // JMP L2
            vec![0x7F41, LBL_BASE + 2, TOK_INS_END],
            // LABEL 1 (then arm)
            vec![0x7F40, LBL_BASE + 1, TOK_INS_END],
            // v0 = v0 + 1
            vec![REG_BASE, 0x7F10, TY_I64, tc_safe, REG_BASE, TOK_IMM_I16, 1, TOK_INS_END],
            // LABEL 2
            vec![0x7F40, LBL_BASE + 2, TOK_INS_END],
            // RET v0
            vec![0x7F4A, REG_BASE, TOK_INS_END],
        ],
    );
    let b6_rep = decompile_u16_stream_to_m3(&b6_miscompile)?;
    if !b6_rep.m3_source.contains("v0 = 0;") || !b6_rep.m3_source.contains("v0 + v2") {
        return Err(format!(
            "audit-view: B6 miscompile view hid the clobbered variable 'v0 = 0;'!\n{}",
            b6_rep.m3_source
        ));
    }
    checks += 1;

    // 4. Approximate fallback test: irreducible branch / non-surface opcode must be marked APPROXIMATE.
    let approx_stream = pack_fn_module(
        &[],
        0,
        &[(PAR_BASE, TY_I64, tc_safe)],
        TY_I64,
        0,
        &[
            vec![REG_BASE, 0x7F7D, TY_I64, tc_safe, PAR_BASE, TOK_INS_END], // EMBED (non-surface)
            vec![0x7F4A, REG_BASE, TOK_INS_END],
        ],
    );
    let approx_rep = decompile_u16_stream_to_m3(&approx_stream)?;
    if approx_rep.is_exact
        || approx_rep.functions_approximate != 1
        || !approx_rep.m3_source.contains("APPROXIMATE")
        || !approx_rep.m3_source.contains("LITERAL_OP")
    {
        return Err(format!(
            "audit-view: approximate fallback was not explicitly marked:\n{}",
            approx_rep.m3_source
        ));
    }
    checks += 1;

    // 5. Proposer diff summary gateable invariants:
    //    5a. Reproducibility (two runs -> byte-identical plain-language & TSV)
    //    5b. Identity invariance (diff(X, X) == 0 deltas, hash_changed=false)
    //    5c. Cap escalation detection
    //    5d. Taint surface change detection
    //    5e. Arena / instruction / word / hash delta + antisymmetry
    let base_src = "fn main(x: i64) -> i64 { let r: i64 = x + 1; return r; }";
    let cap_src = "#[cap(AI)] fn main(x: i64) -> trit { let g: trit = egate(x); return g; }";
    let taint_src = "fn main(x: i64 @untrusted) -> i64 { return x; }";
    let arena_src = "fn main(x: i64) -> i64 { let a: arena = arena(4096); let r: i64 = x + 2; drop(a); return r; }";

    let base_u16 = compile_omni_surface_to_u16(base_src)?;
    let cap_u16 = compile_omni_surface_to_u16(cap_src)?;
    let taint_u16 = compile_omni_surface_to_u16(taint_src)?;
    let arena_u16 = compile_omni_surface_to_u16(arena_src)?;

    // 5a. Reproducibility
    let s1 = summarize_proposer_diff(&base_u16, &arena_u16, "base", "arena")?;
    let s2 = summarize_proposer_diff(&base_u16, &arena_u16, "base", "arena")?;
    if s1.render_plain_language() != s2.render_plain_language()
        || s1.render_canonical_tsv() != s2.render_canonical_tsv()
    {
        return Err(String::from("audit-view: proposer diff summary is not reproducible"));
    }
    checks += 1;

    // 5b. Identity invariance
    let id_sum = summarize_proposer_diff(&base_u16, &base_u16, "base", "base")?;
    if id_sum.hash_changed
        || id_sum.delta_words != 0
        || !id_sum.functions_added.is_empty()
        || !id_sum.functions_removed.is_empty()
        || !id_sum.functions_modified.is_empty()
        || id_sum.cap_escalation
        || id_sum.taint_changed
        || id_sum.ins_delta != 0
        || id_sum.arena_bytes_delta != 0
    {
        return Err(format!("audit-view: identity diff non-zero:\n{}", id_sum.render_canonical_tsv()));
    }
    checks += 1;

    // 5c. Cap escalation detection
    let cap_sum = summarize_proposer_diff(&base_u16, &cap_u16, "base", "cap_ai")?;
    if !cap_sum.cap_escalation || cap_sum.caps_added != vec!["AI"] {
        return Err(format!("audit-view: cap escalation missed:\n{}", cap_sum.render_canonical_tsv()));
    }
    checks += 1;

    // 5d. Taint change detection
    let taint_sum = summarize_proposer_diff(&base_u16, &taint_u16, "base", "taint")?;
    if !taint_sum.taint_changed || taint_sum.after_verified {
        return Err(format!("audit-view: taint change missed:\n{}", taint_sum.render_canonical_tsv()));
    }
    checks += 1;

    // 5e. Budget/arena delta + antisymmetry
    if s1.arena_bytes_delta != 4096 || s1.mem_budget_delta != 4096 || !s1.hash_changed || s1.functions_modified != vec![0] {
        return Err(format!("audit-view: arena/budget delta wrong:\n{}", s1.render_canonical_tsv()));
    }
    let rev = summarize_proposer_diff(&arena_u16, &base_u16, "arena", "base")?;
    if rev.delta_words != -s1.delta_words
        || rev.ins_delta != -s1.ins_delta
        || rev.arena_bytes_delta != -s1.arena_bytes_delta
    {
        return Err(String::from("audit-view: proposer diff summary antisymmetry failed"));
    }
    checks += 1;

    Ok(checks)
}
