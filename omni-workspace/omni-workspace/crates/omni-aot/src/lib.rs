//! Omni AOT — standalone two-pass x86-64 ELF64 compiler (64-bit stdout channel),
//! trap-free WebAssembly v1 compiler & 64-bit stack-machine evaluator, and Two-Pass Stage-1 compiler stream.

use std::collections::BTreeMap;
use omni_isa::*;
use omni_ir::*;
use omni_verifier::verify_binary_function;

// 12. STEP 6 (PURE RUST): TWO-PASS x86-64 ELF & REAL INSTRUCTION-LOWERING WASM AOT
// ==============================================================================

#[derive(Clone, Copy, Debug)]
struct RelocPatch {
    patch_offset: usize,
    target_label: u16,
}

fn collect_func_pack5_map(func: &BinFunction) -> BTreeMap<u16, (Vec<u8>, usize, bool)> {
    let mut reg_pack5: BTreeMap<u16, (Vec<u8>, usize, bool)> = BTreeMap::new();
    for ins in &func.instructions {
        match ins.op_tok {
            0x7F0D => {
                if let Some(d) = ins.dest_reg {
                    reg_pack5.insert(d, (Vec::new(), 0, false));
                }
            }
            0x7F0F => {
                let is_typed_write = (ins.operands.len() == 3
                    && (ins.operands[1] == BinOperand::ImmInt(-4) || ins.operands[1] == BinOperand::ImmInt(-8)))
                    || (ins.operands.len() == 4 && ins.operands[1] == BinOperand::ImmInt(-9));
                if !is_typed_write {
                    let mut bytes = match ins.operands.first() {
                        Some(BinOperand::Reg(s)) => reg_pack5.get(s).map(|(b, _, _)| b.clone()).unwrap_or_default(),
                        _ => Vec::new(),
                    };
                    for op_b in &ins.operands[1..] {
                        if let BinOperand::ImmInt(v) = *op_b {
                            bytes.push((v & 0xFF) as u8);
                        }
                    }
                    let n_trits = bytes.len() * 5;
                    if let Some(d) = ins.dest_reg {
                        reg_pack5.insert(d, (bytes, n_trits, false));
                    }
                }
            }
            0x7F31 => {
                if let (Some(d), Some(BinOperand::Reg(s))) = (ins.dest_reg, ins.operands.first()) {
                    let bytes = reg_pack5.get(s).map(|(b, _, _)| b.clone()).unwrap_or_default();
                    let n_trits = match ins.operands.get(1) {
                        Some(BinOperand::ImmInt(n)) => (*n).max(0) as usize,
                        _ => bytes.len() * 5,
                    };
                    reg_pack5.insert(d, (bytes, n_trits, true));
                }
            }
            0x7F30 => {
                if let (Some(d), Some(BinOperand::Reg(s))) = (ins.dest_reg, ins.operands.first()) {
                    if let Some((bytes, n_trits, _)) = reg_pack5.get(s).cloned() {
                        reg_pack5.insert(d, (bytes, n_trits, false));
                    }
                }
            }
            _ => {}
        }
    }
    reg_pack5
}

#[inline]
fn pack5_word_from_bytes(bytes: &[u8], n_trits: usize) -> i64 {
    let mut word: u64 = (n_trits & 0xFF) as u64;
    for (idx, &b) in bytes.iter().take(7).enumerate() {
        word |= (b as u64) << ((idx + 1) * 8);
    }
    word as i64
}

/// Returns `true` if `module` is a hot numeric, ternary, or `@PACK5` path eligible for direct AOT execution.
pub fn is_module_aot_hot_path(module: &BinModule) -> bool {
    if module.functions.len() != 1 {
        return false;
    }
    let func = &module.functions[0];
    if !matches!(func.ret_ty_tok, TY_I64 | TY_TRIT) {
        return false;
    }
    if !func.params.iter().all(|&(_, ty, _)| matches!(ty, TY_I64 | TY_TRIT | TY_ARENA)) {
        return false;
    }
    func.instructions
        .iter()
        .all(|ins| is_wasm_aot_supported_opcode(ins.op_tok) && is_x86_64_aot_supported_opcode(ins.op_tok))
}

/// Emits a 65-byte x86-64 CLI decimal integer parser supporting optional leading `'-'` (`0x2D`)
/// so both positive and negative integer/trit arguments can be passed on the command line.
fn emit_cli_arg_parser(mc: &mut Vec<u8>, argc_min: u8, disp_argv: u8, target_reg_code: u8) {
    mc.extend_from_slice(&[0x48, 0x83, 0x7D, 0x08, argc_min]);
    mc.extend_from_slice(&[0x7C, 0x3A]);
    mc.extend_from_slice(&[0x48, 0x8B, 0x75, disp_argv]);
    mc.extend_from_slice(&[0x31, 0xC0]);
    mc.extend_from_slice(&[0xBF, 0x01, 0x00, 0x00, 0x00]);
    mc.extend_from_slice(&[0x0F, 0xB6, 0x0E]);
    mc.extend_from_slice(&[0x80, 0xF9, 0x2D]);
    mc.extend_from_slice(&[0x75, 0x0A]);
    mc.extend_from_slice(&[0x48, 0xC7, 0xC7, 0xFF, 0xFF, 0xFF, 0xFF]);
    mc.extend_from_slice(&[0x48, 0xFF, 0xC6]);
    mc.extend_from_slice(&[0x0F, 0xB6, 0x0E]);
    mc.extend_from_slice(&[0x84, 0xC9]);
    mc.extend_from_slice(&[0x74, 0x0F]);
    mc.extend_from_slice(&[0x83, 0xE9, 0x30]);
    mc.extend_from_slice(&[0x48, 0x6B, 0xC0, 0x0A]);
    mc.extend_from_slice(&[0x48, 0x01, 0xC8]);
    mc.extend_from_slice(&[0x48, 0xFF, 0xC6]);
    mc.extend_from_slice(&[0xEB, 0xEA]);
    mc.extend_from_slice(&[0x48, 0x0F, 0xAF, 0xC7]);
    mc.extend_from_slice(&[0x49, 0x89, 0xC0 + target_reg_code]);
}

pub fn compile_pure_u16_to_x86_64_elf(stream: &[u16], arg0: i64, arg1: i64) -> Result<Vec<u8>, String> {
    compile_pure_u16_to_x86_64_elf_with_args(stream, arg0, arg1, 90)
}

pub fn compile_pure_u16_to_x86_64_elf_with_args(
    stream: &[u16],
    arg0: i64,
    arg1: i64,
    arg2: i64,
) -> Result<Vec<u8>, String> {
    let module = decode_omni_token_stream(stream)?;
    let func = module.functions.first().ok_or("No function in module")?;
    let rep = verify_binary_function(func);
    if !rep.passed {
        return Err(format!("Cannot AOT-compile unverified binary stream: {:?}", rep.errors));
    }

    for ins in &func.instructions {
        if !is_x86_64_aot_supported_opcode(ins.op_tok) {
            let lbl = decode_opcode_token(ins.op_tok).map(|s| s.debug_label).unwrap_or("OP_UNKNOWN");
            return Err(format!(
                "Opcode 0x{:04X} ({}) requires high-level VM runtime state and is not lowered by standalone x86-64 AOT",
                ins.op_tok, lbl
            ));
        }
    }

    let mut code: Vec<u8> = Vec::with_capacity(2048);
    let mut label_offsets: Vec<(u16, usize)> = Vec::new();
    let mut relocs: Vec<RelocPatch> = Vec::new();

    code.extend_from_slice(&[
        0x55,
        0x48, 0x89, 0xE5,
        0x48, 0x81, 0xEC, 0x00, 0x00, 0x03, 0x00,
    ]);

    if func.ret_ty_tok == TY_BYTES {
        code.extend_from_slice(&[0x49, 0xC7, 0xC4, 0x00, 0x00, 0x00, 0x00]);
        code.extend_from_slice(&[0x49, 0xBD]);
        code.extend_from_slice(&(arg0 as u64).to_le_bytes());
        code.extend_from_slice(&[0x49, 0xBE]);
        code.extend_from_slice(&(arg1 as u64).to_le_bytes());
        code.extend_from_slice(&[0x49, 0xBF]);
        code.extend_from_slice(&(arg2 as u64).to_le_bytes());

        code.extend_from_slice(&[
            0x48, 0x83, 0x7D, 0x08, 0x02,
            0x7C, 0x2E,
            0x48, 0x8B, 0x7D, 0x18,
            0x31, 0xF6,
            0x31, 0xD2,
            0xB8, 0x02, 0x00, 0x00, 0x00,
            0x0F, 0x05,
            0x48, 0x85, 0xC0,
            0x78, 0x1A,
            0x48, 0x89, 0xC7,
            0x48, 0x8D, 0xB5, 0x00, 0x20, 0xFD, 0xFF,
            0xBA, 0x00, 0xE0, 0x00, 0x00,
            0x31, 0xC0,
            0x0F, 0x05,
            0xB8, 0x03, 0x00, 0x00, 0x00,
            0x0F, 0x05,
            0x45, 0x31, 0xDB,
        ]);
        emit_cli_arg_parser(&mut code, 3, 0x20, 5);
        emit_cli_arg_parser(&mut code, 4, 0x28, 6);
        emit_cli_arg_parser(&mut code, 5, 0x30, 7);
    } else {
        code.extend_from_slice(&[0x49, 0xBC]);
        code.extend_from_slice(&(arg0 as u64).to_le_bytes());
        code.extend_from_slice(&[0x49, 0xBD]);
        code.extend_from_slice(&(arg1 as u64).to_le_bytes());
        code.extend_from_slice(&[0x49, 0xBE]);
        code.extend_from_slice(&(arg2 as u64).to_le_bytes());
        code.extend_from_slice(&[0x49, 0xC7, 0xC7, 0x00, 0x00, 0x00, 0x00]);
        emit_cli_arg_parser(&mut code, 2, 0x18, 4);
        emit_cli_arg_parser(&mut code, 3, 0x20, 5);
        emit_cli_arg_parser(&mut code, 4, 0x28, 6);
        emit_cli_arg_parser(&mut code, 5, 0x30, 7);
    }

    let emit_load_op = |c: &mut Vec<u8>, op: &BinOperand, into_rcx: bool| {
        match *op {
            BinOperand::Param(0x7EC0) => {
                c.extend_from_slice(&[0x4C, 0x89, if into_rcx { 0xE1 } else { 0xE0 }]);
            }
            BinOperand::Param(0x7EC1) => {
                c.extend_from_slice(&[0x4C, 0x89, if into_rcx { 0xE9 } else { 0xE8 }]);
            }
            BinOperand::Param(0x7EC2) => {
                c.extend_from_slice(&[0x4C, 0x89, if into_rcx { 0xF1 } else { 0xF0 }]);
            }
            BinOperand::Param(0x7EC3) => {
                c.extend_from_slice(&[0x4C, 0x89, if into_rcx { 0xF9 } else { 0xF8 }]);
            }
            BinOperand::Reg(r_tok) => {
                let disp: i32 = -8 * (((r_tok - REG_BASE) as i32) + 1);
                c.extend_from_slice(&[0x48, 0x8B, if into_rcx { 0x8D } else { 0x85 }]);
                c.extend_from_slice(&disp.to_le_bytes());
            }
            BinOperand::ImmInt(v) => {
                c.extend_from_slice(&[0x48, if into_rcx { 0xB9 } else { 0xB8 }]);
                c.extend_from_slice(&(v as u64).to_le_bytes());
            }
            BinOperand::ImmTrit(t) => {
                c.extend_from_slice(&[0x48, if into_rcx { 0xB9 } else { 0xB8 }]);
                c.extend_from_slice(&((t as i64) as u64).to_le_bytes());
            }
            _ => {}
        }
    };

    let emit_store_rax = |c: &mut Vec<u8>, d_tok: u16| {
        let disp: i32 = -8 * (((d_tok - REG_BASE) as i32) + 1);
        c.extend_from_slice(&[0x48, 0x89, 0x85]);
        c.extend_from_slice(&disp.to_le_bytes());
    };

    let reg_pack5 = collect_func_pack5_map(func);
    let emit_extract_pack5_trit_x86 = |c: &mut Vec<u8>, op: &BinOperand, trit_idx: usize, into_rcx: bool| {
        if let BinOperand::Reg(r) = *op {
            if let Some((bytes, n, _)) = reg_pack5.get(&r) {
                emit_load_op(c, &BinOperand::ImmInt(pack5_word_from_bytes(bytes, *n)), false);
            } else {
                emit_load_op(c, op, false);
            }
        } else {
            emit_load_op(c, op, false);
        }
        let shift: u8 = (((trit_idx / 5) + 1) * 8) as u8;
        c.extend_from_slice(&[0x48, 0xC1, 0xE8, shift]); // shr rax, shift
        c.extend_from_slice(&[0x0F, 0xB6, 0xC0]);       // movzx eax, al
        c.extend_from_slice(&[0x31, 0xD2]);             // xor edx, edx
        let pow3: u8 = [1u8, 3, 9, 27, 81][trit_idx % 5];
        if pow3 > 1 {
            c.extend_from_slice(&[0xBE, pow3, 0, 0, 0]); // mov esi, pow3
            c.extend_from_slice(&[0x48, 0xF7, 0xF6]);    // div rsi
            c.extend_from_slice(&[0x31, 0xD2]);          // xor edx, edx
        }
        c.extend_from_slice(&[0xBE, 3, 0, 0, 0]);       // mov esi, 3
        c.extend_from_slice(&[0x48, 0xF7, 0xF6]);       // div rsi (rdx = byte / 3^k % 3)
        c.extend_from_slice(&[0x48, 0x8D, 0x42, 0xFF]); // lea rax, [rdx - 1]
        if into_rcx {
            c.extend_from_slice(&[0x48, 0x89, 0xC1]);   // mov rcx, rax
        }
    };

    for ins in &func.instructions {
        match ins.op_tok {
            0x7F00 | 0x7F05 => {}
            0x7F01 | 0x7F02 | 0x7F03 | 0x7F04 | 0x7F06 | 0x7F09 | 0x7F57 | 0x7F58 | 0x7F59 => {
                emit_load_op(&mut code, &ins.operands[0], false);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F0D => {
                code.extend_from_slice(&[0x45, 0x31, 0xDB]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F0E => {
                if let (Some(BinOperand::Reg(r_arr)), Some(BinOperand::ImmInt(idx))) =
                    (ins.operands.first(), ins.operands.get(1))
                {
                    if *idx >= 0 {
                        if let Some((bytes, _, is_tv)) = reg_pack5.get(r_arr) {
                            if *is_tv {
                                emit_extract_pack5_trit_x86(&mut code, &ins.operands[0], *idx as usize, false);
                            } else {
                                let b_val = bytes.get(*idx as usize).copied().unwrap_or(0) as i64;
                                emit_load_op(&mut code, &BinOperand::ImmInt(b_val), false);
                            }
                            if let Some(d) = ins.dest_reg {
                                emit_store_rax(&mut code, d);
                            }
                            continue;
                        }
                    }
                }
                emit_load_op(&mut code, &ins.operands[1], false);
                code.extend_from_slice(&[0x0F, 0xB7, 0x84, 0x45, 0x00, 0x20, 0xFD, 0xFF]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F0F => {
                if ins.operands.len() == 3 && ins.operands[1] == BinOperand::ImmInt(-4) {
                    emit_load_op(&mut code, &ins.operands[2], false);
                    code.extend_from_slice(&[0x42, 0x89, 0x84, 0x1D, 0x00, 0x00, 0xFE, 0xFF, 0x49, 0x83, 0xC3, 0x04]);
                } else if ins.operands.len() == 3 && ins.operands[1] == BinOperand::ImmInt(-8) {
                    emit_load_op(&mut code, &ins.operands[2], false);
                    code.extend_from_slice(&[0x4A, 0x89, 0x84, 0x1D, 0x00, 0x00, 0xFE, 0xFF, 0x49, 0x83, 0xC3, 0x08]);
                } else if ins.operands.len() == 4 && ins.operands[1] == BinOperand::ImmInt(-9) {
                    emit_load_op(&mut code, &ins.operands[2], true);
                    emit_load_op(&mut code, &ins.operands[3], false);
                    code.extend_from_slice(&[0x48, 0x89, 0x84, 0x0D, 0x00, 0x00, 0xFE, 0xFF]);
                } else {
                    for op_b in &ins.operands[1..] {
                        emit_load_op(&mut code, op_b, false);
                        code.extend_from_slice(&[0x42, 0x88, 0x84, 0x1D, 0x00, 0x00, 0xFE, 0xFF, 0x49, 0xFF, 0xC3]);
                    }
                }
                if let Some(d) = ins.dest_reg {
                    code.extend_from_slice(&[0x4C, 0x89, 0xD8]);
                    emit_store_rax(&mut code, d);
                }
            }
            0x7F50 => {
                code.extend_from_slice(&[0x4C, 0x89, 0xD8]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F30 | 0x7F31 => {
                if let Some(d) = ins.dest_reg {
                    let word = reg_pack5
                        .get(&d)
                        .map(|(b, n, _)| pack5_word_from_bytes(b, *n))
                        .unwrap_or(0);
                    emit_load_op(&mut code, &BinOperand::ImmInt(word), false);
                    emit_store_rax(&mut code, d);
                }
            }
            0x7F34 => {
                let n_trits = match (ins.operands.first(), ins.operands.get(1)) {
                    (Some(BinOperand::Reg(ra)), Some(BinOperand::Reg(rb))) => {
                        let na = reg_pack5.get(ra).map(|(_, n, _)| *n).unwrap_or(0);
                        let nb = reg_pack5.get(rb).map(|(_, n, _)| *n).unwrap_or(0);
                        na.min(nb).min(35)
                    }
                    _ => 0,
                };
                code.extend_from_slice(&[0x45, 0x31, 0xC0]); // xor r8d, r8d
                for idx in 0..n_trits {
                    emit_extract_pack5_trit_x86(&mut code, &ins.operands[0], idx, true);
                    emit_extract_pack5_trit_x86(&mut code, &ins.operands[1], idx, false);
                    code.extend_from_slice(&[0x48, 0x0F, 0xAF, 0xC1]); // imul rax, rcx
                    code.extend_from_slice(&[0x49, 0x01, 0xC0]);       // add r8, rax
                }
                code.extend_from_slice(&[0x4C, 0x89, 0xC0]); // mov rax, r8
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F35 | 0x7F3F => {
                let pack5_len = match ins.operands.first() {
                    Some(BinOperand::Reg(r)) => reg_pack5.get(r).map(|(_, n, _)| (*n).min(35)),
                    _ => None,
                };
                if let Some(n_trits) = pack5_len {
                    code.extend_from_slice(&[0x45, 0x31, 0xC0]); // xor r8d, r8d
                    for idx in 0..n_trits {
                        emit_extract_pack5_trit_x86(&mut code, &ins.operands[0], idx, false);
                        code.extend_from_slice(&[0x49, 0x01, 0xC0]); // add r8, rax
                    }
                    code.extend_from_slice(&[0x4C, 0x89, 0xC0]); // mov rax, r8
                } else {
                    emit_load_op(&mut code, &ins.operands[0], false);
                }
                code.extend_from_slice(&[
                    0x48, 0x83, 0xF8, 0x00, // cmp rax, 0
                    0x0F, 0x9F, 0xC0,       // setg al
                    0x0F, 0x9C, 0xC2,       // setl dl
                    0x0F, 0xB6, 0xC0,       // movzx eax, al
                    0x0F, 0xB6, 0xD2,       // movzx edx, dl
                    0x48, 0x29, 0xD0,       // sub rax, rdx
                ]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F10 | 0x7F11 | 0x7F12 | 0x7F1E | 0x7F1F | 0x7F20 => {
                emit_load_op(&mut code, &ins.operands[0], false);
                emit_load_op(&mut code, &ins.operands[1], true);
                match ins.op_tok {
                    0x7F10 => code.extend_from_slice(&[0x48, 0x01, 0xC8]),
                    0x7F11 => code.extend_from_slice(&[0x48, 0x29, 0xC8]),
                    0x7F12 => code.extend_from_slice(&[0x48, 0x0F, 0xAF, 0xC1]),
                    0x7F1E => code.extend_from_slice(&[0x48, 0x21, 0xC8]),
                    0x7F1F => code.extend_from_slice(&[0x48, 0x09, 0xC8]),
                    _      => code.extend_from_slice(&[0x48, 0x31, 0xC8]),
                }
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F13 | 0x7F14 => {
                emit_load_op(&mut code, &ins.operands[0], false);
                emit_load_op(&mut code, &ins.operands[1], true);
                code.extend_from_slice(&[0x49, 0x89, 0xC0, 0x48, 0x85, 0xC9, 0x74, 0x1E, 0x48, 0x83, 0xF9, 0xFF, 0x75, 0x11, 0x49, 0xB9, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x4D, 0x39, 0xC8, 0x75, 0x02, 0xEB, 0x0A, 0x48, 0x99, 0x48, 0xF7, 0xF9, 0xEB, 0x03, 0x48, 0x31, 0xC0]);
                if ins.op_tok == 0x7F14 {
                    code.extend_from_slice(&[0x48, 0x89, 0xD0]);
                }
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F15 | 0x7F28 => {
                emit_load_op(&mut code, &ins.operands[0], false);
                code.extend_from_slice(&[0x48, 0xF7, 0xD8]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F16 => {
                emit_load_op(&mut code, &ins.operands[0], false);
                code.extend_from_slice(&[0x48, 0x89, 0xC2, 0x48, 0xF7, 0xD8, 0x48, 0x0F, 0x48, 0xC2]); // fixed IABS: cmovs
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F21 => {
                emit_load_op(&mut code, &ins.operands[0], false);
                code.extend_from_slice(&[0x48, 0xF7, 0xD0]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F22 | 0x7F23 => {
                emit_load_op(&mut code, &ins.operands[0], false);
                emit_load_op(&mut code, &ins.operands[1], true);
                code.extend_from_slice(&[0x48, 0xD3, if ins.op_tok == 0x7F22 { 0xE0 } else { 0xF8 }]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F24 | 0x7F25 | 0x7F26 | 0x7F27 => {
                emit_load_op(&mut code, &ins.operands[0], false);
                emit_load_op(&mut code, &ins.operands[1], true);
                let setcc: u8 = match ins.op_tok {
                    0x7F24 => 0x94,
                    0x7F25 => 0x95,
                    0x7F26 => 0x9C,
                    _      => 0x9F,
                };
                code.extend_from_slice(&[
                    0x48, 0x39, 0xC8,
                    0x0F, setcc, 0xC0,
                    0x0F, 0xB6, 0xC0,
                    0x48, 0x8D, 0x44, 0x00, 0xFF,
                ]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F29 | 0x7F17 => {
                emit_load_op(&mut code, &ins.operands[0], false);
                emit_load_op(&mut code, &ins.operands[1], true);
                code.extend_from_slice(&[0x48, 0x39, 0xC8, 0x48, 0x0F, 0x4F, 0xC1]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F2A | 0x7F18 => {
                emit_load_op(&mut code, &ins.operands[0], false);
                emit_load_op(&mut code, &ins.operands[1], true);
                code.extend_from_slice(&[0x48, 0x39, 0xC8, 0x48, 0x0F, 0x4C, 0xC1]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F2B => {
                emit_load_op(&mut code, &ins.operands[0], false);
                emit_load_op(&mut code, &ins.operands[1], true);
                code.extend_from_slice(&[0x48, 0x0F, 0xAF, 0xC1, 0x48, 0xF7, 0xD8]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F2C => {
                emit_load_op(&mut code, &ins.operands[0], false);
                emit_load_op(&mut code, &ins.operands[1], true);
                code.extend_from_slice(&[0x48, 0x01, 0xC8]);
                emit_load_op(&mut code, &ins.operands[2], true);
                code.extend_from_slice(&[
                    0x48, 0x01, 0xC8,
                    0x48, 0x83, 0xF8, 0x00,
                    0x0F, 0x9F, 0xC0,
                    0x0F, 0x9C, 0xC2,
                    0x0F, 0xB6, 0xC0,
                    0x0F, 0xB6, 0xD2,
                    0x48, 0x29, 0xD0,
                ]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F2D => {
                emit_load_op(&mut code, &ins.operands[0], false);
                emit_load_op(&mut code, &ins.operands[1], true);
                code.extend_from_slice(&[
                    0x48, 0x39, 0xC8,
                    0x0F, 0x9F, 0xC2,
                    0x0F, 0x9C, 0xC1,
                    0x0F, 0xB6, 0xD2,
                    0x0F, 0xB6, 0xC9,
                    0x29, 0xCA,
                    0x48, 0x63, 0xC2,
                ]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F40 => {
                if let Some(lbl) = ins.operands.first().and_then(extract_label_id) {
                    label_offsets.push((lbl, code.len()));
                }
            }
            0x7F41 => {
                if let Some(lbl) = ins.operands.first().and_then(extract_label_id) {
                    code.push(0xE9);
                    relocs.push(RelocPatch { patch_offset: code.len(), target_label: lbl });
                    code.extend_from_slice(&[0, 0, 0, 0]);
                }
            }
            0x7F42 => {
                if let (Some(l_neg), Some(l_zero), Some(l_pos)) = (
                    ins.operands.get(1).and_then(extract_label_id),
                    ins.operands.get(2).and_then(extract_label_id),
                    ins.operands.get(3).and_then(extract_label_id),
                ) {
                    emit_load_op(&mut code, &ins.operands[0], false);
                    code.extend_from_slice(&[0x48, 0x83, 0xF8, 0x00]);
                    code.extend_from_slice(&[0x0F, 0x8C]);
                    relocs.push(RelocPatch { patch_offset: code.len(), target_label: l_neg });
                    code.extend_from_slice(&[0, 0, 0, 0]);
                    code.extend_from_slice(&[0x0F, 0x84]);
                    relocs.push(RelocPatch { patch_offset: code.len(), target_label: l_zero });
                    code.extend_from_slice(&[0, 0, 0, 0]);
                    code.push(0xE9);
                    relocs.push(RelocPatch { patch_offset: code.len(), target_label: l_pos });
                    code.extend_from_slice(&[0, 0, 0, 0]);
                }
            }
            0x7F43 | 0x7F44 => {
                if let Some(lbl) = ins.operands.get(1).and_then(extract_label_id) {
                    emit_load_op(&mut code, &ins.operands[0], false);
                    code.extend_from_slice(&[0x48, 0x83, 0xF8, if ins.op_tok == 0x7F43 { 0x01 } else { 0xFF }]);
                    code.extend_from_slice(&[0x0F, 0x84]);
                    relocs.push(RelocPatch { patch_offset: code.len(), target_label: lbl });
                    code.extend_from_slice(&[0, 0, 0, 0]);
                }
            }
            0x7F45 => {
                emit_load_op(&mut code, &ins.operands[0], false);
                emit_load_op(&mut code, &ins.operands[1], true);
                code.extend_from_slice(&[0x49, 0x89, 0xC0, 0x48, 0x89, 0xCA]);
                emit_load_op(&mut code, &ins.operands[2], true);
                code.extend_from_slice(&[0x49, 0x83, 0xF8, 0x00, 0x48, 0x0F, 0x44, 0xD1]);
                emit_load_op(&mut code, &ins.operands[3], true);
                code.extend_from_slice(&[0x49, 0x83, 0xF8, 0x00, 0x48, 0x0F, 0x4F, 0xD1, 0x48, 0x89, 0xD0]);
                if let Some(d) = ins.dest_reg { emit_store_rax(&mut code, d); }
            }
            0x7F4A => {
                if func.ret_ty_tok == TY_BYTES {
                    code.extend_from_slice(&[
                        0xBF, 0x01, 0x00, 0x00, 0x00,
                        0x48, 0x8D, 0xB5, 0x00, 0x00, 0xFE, 0xFF,
                        0x4C, 0x89, 0xDA,
                        0xB8, 0x01, 0x00, 0x00, 0x00,
                        0x0F, 0x05,
                        0x31, 0xFF,
                        0xB8, 0x3C, 0x00, 0x00, 0x00,
                        0x0F, 0x05,
                    ]);
                } else {
                    emit_load_op(&mut code, &ins.operands[0], false);
                    // 64-bit result channel: write raw little-endian i64 to stdout, then exit(0)
                    code.extend_from_slice(&[
                        0x48, 0x83, 0xEC, 0x08,             // sub rsp, 8
                        0x48, 0x89, 0x04, 0x24,             // mov [rsp], rax
                        0xB8, 0x01, 0x00, 0x00, 0x00,       // mov eax, 1 (write)
                        0xBF, 0x01, 0x00, 0x00, 0x00,       // mov edi, 1 (stdout)
                        0x48, 0x89, 0xE6,                   // mov rsi, rsp
                        0xBA, 0x08, 0x00, 0x00, 0x00,       // mov edx, 8
                        0x0F, 0x05,                         // syscall
                        0x48, 0x83, 0xC4, 0x08,             // add rsp, 8
                        0x31, 0xFF,                         // xor edi, edi
                        0xB8, 0x3C, 0x00, 0x00, 0x00,       // mov eax, 60 (exit)
                        0x0F, 0x05,                         // syscall
                    ]);
                }
            }
            _ => {}
        }
    }

    for r in &relocs {
        let target_off = label_offsets
            .iter()
            .find(|&&(lbl, _)| lbl == r.target_label)
            .map(|&(_, off)| off)
            .ok_or_else(|| format!("Unresolved label 0x{:04X}", r.target_label))?;
        let rel32: i32 = (target_off as i32) - ((r.patch_offset + 4) as i32);
        code[r.patch_offset..r.patch_offset + 4].copy_from_slice(&rel32.to_le_bytes());
    }

    let base_vaddr: u64 = 0x400000;
    let entry_vaddr: u64 = base_vaddr + 120;
    let total_size: u64 = 120 + (code.len() as u64);

    let mut elf = vec![0u8; 120];
    elf[0..4].copy_from_slice(&[0x7F, b'E', b'L', b'F']);
    elf[4] = 2;
    elf[5] = 1;
    elf[6] = 1;
    elf[16..18].copy_from_slice(&2u16.to_le_bytes());
    elf[18..20].copy_from_slice(&0x3Eu16.to_le_bytes());
    elf[20..24].copy_from_slice(&1u32.to_le_bytes());
    elf[24..32].copy_from_slice(&entry_vaddr.to_le_bytes());
    elf[32..40].copy_from_slice(&64u64.to_le_bytes());
    elf[52..54].copy_from_slice(&64u16.to_le_bytes());
    elf[54..56].copy_from_slice(&56u16.to_le_bytes());
    elf[56..58].copy_from_slice(&1u16.to_le_bytes());

    elf[64..68].copy_from_slice(&1u32.to_le_bytes());
    elf[68..72].copy_from_slice(&5u32.to_le_bytes());
    elf[80..88].copy_from_slice(&base_vaddr.to_le_bytes());
    elf[88..96].copy_from_slice(&base_vaddr.to_le_bytes());
    elf[96..104].copy_from_slice(&total_size.to_le_bytes());
    elf[104..112].copy_from_slice(&total_size.to_le_bytes());
    elf[112..120].copy_from_slice(&0x1000u64.to_le_bytes());

    elf.extend_from_slice(&code);
    Ok(elf)
}

fn encode_uleb128(mut val: usize) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let mut byte = (val & 0x7F) as u8;
        val >>= 7;
        if val != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if val == 0 {
            break;
        }
    }
    out
}

fn encode_sleb128(mut val: i64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let mut byte = (val & 0x7F) as u8;
        val >>= 7;
        let done = (val == 0 && (byte & 0x40) == 0) || (val == -1 && (byte & 0x40) != 0);
        if !done {
            byte |= 0x80;
        }
        out.push(byte);
        if done {
            break;
        }
    }
    out
}

pub fn compile_pure_u16_to_wasm(stream: &[u16]) -> Result<Vec<u8>, String> {
    let module = decode_omni_token_stream(stream)?;
    let func = module.functions.first().ok_or("No function in module")?;
    let rep = verify_binary_function(func);
    if !rep.passed {
        return Err(format!("Cannot Wasm-compile unverified binary stream: {:?}", rep.errors));
    }

    for ins in &func.instructions {
        if !is_wasm_aot_supported_opcode(ins.op_tok) {
            let lbl = decode_opcode_token(ins.op_tok).map(|s| s.debug_label).unwrap_or("OP_UNKNOWN");
            return Err(format!(
                "Opcode 0x{:04X} ({}) requires high-level VM runtime state and is not lowered by standalone WebAssembly AOT",
                ins.op_tok, lbl
            ));
        }
    }

    let num_params = func.params.len();
    let max_reg_used = func
        .instructions
        .iter()
        .filter_map(|ins| ins.dest_reg)
        .map(|r| (r - REG_BASE) as usize + 1)
        .max()
        .unwrap_or(0);
    let num_locals = max_reg_used + 4;
    let scratch0 = num_params + max_reg_used;
    let scratch1 = num_params + max_reg_used + 1;
    let buf_len_local = num_params + max_reg_used + 2;
    let pc_local = num_params + max_reg_used + 3;

    let has_control_flow = func
        .instructions
        .iter()
        .any(|ins| (0x7F40..=0x7F44).contains(&ins.op_tok));

    let mut label_pcs: Vec<(u16, i64)> = Vec::new();
    for (idx, ins) in func.instructions.iter().enumerate() {
        if ins.op_tok == 0x7F40 {
            if let Some(lbl) = ins.operands.first().and_then(extract_label_id) {
                label_pcs.push((lbl, idx as i64));
            }
        }
    }
    let lookup_lbl_pc = |lbl: u16| -> Result<i64, String> {
        label_pcs
            .iter()
            .find(|&&(l, _)| l == lbl)
            .map(|&(_, pc)| pc)
            .ok_or_else(|| format!("Unresolved Wasm label 0x{:04X}", lbl))
    };

    let mut body: Vec<u8> = Vec::with_capacity(512);
    body.extend(encode_uleb128(1));
    body.extend(encode_uleb128(num_locals));
    body.push(0x7E);

    let emit_wasm_op = |b: &mut Vec<u8>, op: &BinOperand| {
        match *op {
            BinOperand::Param(p_tok) => {
                b.push(0x20);
                b.extend(encode_uleb128((p_tok - PAR_BASE) as usize));
            }
            BinOperand::Reg(r_tok) => {
                b.push(0x20);
                b.extend(encode_uleb128(num_params + (r_tok - REG_BASE) as usize));
            }
            BinOperand::ImmInt(v) => {
                b.push(0x42);
                b.extend(encode_sleb128(v));
            }
            BinOperand::ImmTrit(t) => {
                b.push(0x42);
                b.extend(encode_sleb128(t as i64));
            }
            _ => {
                b.extend_from_slice(&[0x42, 0x00]);
            }
        }
    };

    let emit_wasm_set_dest = |b: &mut Vec<u8>, dest: Option<u16>| {
        if let Some(d_tok) = dest {
            b.push(0x21);
            b.extend(encode_uleb128(num_params + (d_tok - REG_BASE) as usize));
        }
    };

    let reg_pack5 = collect_func_pack5_map(func);
    let emit_extract_pack5_trit_wasm = |b: &mut Vec<u8>, op: &BinOperand, trit_idx: usize| {
        if let BinOperand::Reg(r) = *op {
            if let Some((bytes, n, _)) = reg_pack5.get(&r) {
                b.push(0x42);
                b.extend(encode_sleb128(pack5_word_from_bytes(bytes, *n)));
            } else {
                emit_wasm_op(b, op);
            }
        } else {
            emit_wasm_op(b, op);
        }
        let shift: i64 = (((trit_idx / 5) + 1) * 8) as i64;
        b.push(0x42);
        b.extend(encode_sleb128(shift));
        b.push(0x87); // i64.shr_s
        b.push(0x42);
        b.extend(encode_sleb128(255));
        b.push(0x83); // i64.and
        let pow3: i64 = [1i64, 3, 9, 27, 81][trit_idx % 5];
        if pow3 > 1 {
            b.push(0x42);
            b.extend(encode_sleb128(pow3));
            b.push(0x7F); // i64.div_s
        }
        b.extend_from_slice(&[0x42, 0x03, 0x81, 0x42, 0x01, 0x7D]); // % 3 - 1
    };

    let emit_straight_ins = |b: &mut Vec<u8>, ins: &BinInstruction| {
        match ins.op_tok {
            0x7F00 | 0x7F05 => {}
            0x7F01 | 0x7F02 | 0x7F03 | 0x7F04 | 0x7F06 | 0x7F09 | 0x7F57 | 0x7F58 | 0x7F59 => {
                emit_wasm_op(b, &ins.operands[0]);
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F30 | 0x7F31 => {
                let word = ins
                    .dest_reg
                    .and_then(|d| reg_pack5.get(&d).map(|(bytes, n, _)| pack5_word_from_bytes(bytes, *n)))
                    .unwrap_or(0);
                b.push(0x42);
                b.extend(encode_sleb128(word));
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F34 => {
                let n_trits = match (ins.operands.first(), ins.operands.get(1)) {
                    (Some(BinOperand::Reg(ra)), Some(BinOperand::Reg(rb))) => {
                        let na = reg_pack5.get(ra).map(|(_, n, _)| *n).unwrap_or(0);
                        let nb = reg_pack5.get(rb).map(|(_, n, _)| *n).unwrap_or(0);
                        na.min(nb).min(35)
                    }
                    _ => 0,
                };
                b.extend_from_slice(&[0x42, 0x00]); // acc = 0
                for idx in 0..n_trits {
                    emit_extract_pack5_trit_wasm(b, &ins.operands[0], idx);
                    emit_extract_pack5_trit_wasm(b, &ins.operands[1], idx);
                    b.extend_from_slice(&[0x7E, 0x7C]); // i64.mul, i64.add
                }
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F35 | 0x7F3F => {
                let pack5_len = match ins.operands.first() {
                    Some(BinOperand::Reg(r)) => reg_pack5.get(r).map(|(_, n, _)| (*n).min(35)),
                    _ => None,
                };
                if let Some(n_trits) = pack5_len {
                    b.extend_from_slice(&[0x42, 0x00]); // sum = 0
                    for idx in 0..n_trits {
                        emit_extract_pack5_trit_wasm(b, &ins.operands[0], idx);
                        b.push(0x7C); // i64.add
                    }
                } else {
                    emit_wasm_op(b, &ins.operands[0]);
                }
                b.push(0x22); b.extend(encode_uleb128(scratch0));
                b.extend_from_slice(&[0x42, 0x00, 0x55, 0xAD]);
                b.push(0x20); b.extend(encode_uleb128(scratch0));
                b.extend_from_slice(&[0x42, 0x00, 0x53, 0xAD, 0x7D]);
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F0D => {
                b.extend_from_slice(&[0x42, 0x00]);
                b.push(0x21);
                b.extend(encode_uleb128(buf_len_local));
                if ins.dest_reg.is_some() {
                    b.extend_from_slice(&[0x42, 0x00]);
                    emit_wasm_set_dest(b, ins.dest_reg);
                }
            }
            0x7F0E => {
                let mut handled = false;
                if let (Some(BinOperand::Reg(r_arr)), Some(BinOperand::ImmInt(idx))) =
                    (ins.operands.first(), ins.operands.get(1))
                {
                    if *idx >= 0 {
                        if let Some((bytes, _, is_tv)) = reg_pack5.get(r_arr) {
                            if *is_tv {
                                emit_extract_pack5_trit_wasm(b, &ins.operands[0], *idx as usize);
                            } else {
                                let b_val = bytes.get(*idx as usize).copied().unwrap_or(0) as i64;
                                b.push(0x42);
                                b.extend(encode_sleb128(b_val));
                            }
                            emit_wasm_set_dest(b, ins.dest_reg);
                            handled = true;
                        }
                    }
                }
                if !handled {
                    b.extend_from_slice(&[0x42, 0x00]);
                    emit_wasm_set_dest(b, ins.dest_reg);
                }
            }
            0x7F0F => {
                let delta: i64 = if ins.operands.len() == 3 && ins.operands[1] == BinOperand::ImmInt(-4) {
                    4
                } else if ins.operands.len() == 3 && ins.operands[1] == BinOperand::ImmInt(-8) {
                    8
                } else if ins.operands.len() == 4 && ins.operands[1] == BinOperand::ImmInt(-9) {
                    0
                } else {
                    ins.operands.len().saturating_sub(1) as i64
                };
                if delta != 0 {
                    b.push(0x20);
                    b.extend(encode_uleb128(buf_len_local));
                    b.push(0x42);
                    b.extend(encode_sleb128(delta));
                    b.push(0x7C);
                    b.push(0x21);
                    b.extend(encode_uleb128(buf_len_local));
                }
                if ins.dest_reg.is_some() {
                    b.push(0x20);
                    b.extend(encode_uleb128(buf_len_local));
                    emit_wasm_set_dest(b, ins.dest_reg);
                }
            }
            0x7F50 => {
                if ins.dest_reg.is_some() {
                    b.push(0x20);
                    b.extend(encode_uleb128(buf_len_local));
                    emit_wasm_set_dest(b, ins.dest_reg);
                }
            }
            0x7F10 | 0x7F11 | 0x7F12 | 0x7F1E | 0x7F1F | 0x7F20 | 0x7F22 | 0x7F23 => {
                emit_wasm_op(b, &ins.operands[0]);
                emit_wasm_op(b, &ins.operands[1]);
                let wasm_op = match ins.op_tok {
                    0x7F10 => 0x7C,
                    0x7F11 => 0x7D,
                    0x7F12 => 0x7E,
                    0x7F1E => 0x83,
                    0x7F1F => 0x84,
                    0x7F20 => 0x85,
                    0x7F22 => 0x86,
                    _      => 0x87,
                };
                b.push(wasm_op);
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F13 | 0x7F14 => {
                emit_wasm_op(b, &ins.operands[0]);
                b.push(0x21); b.extend(encode_uleb128(scratch0));
                emit_wasm_op(b, &ins.operands[1]);
                b.push(0x21); b.extend(encode_uleb128(scratch1));
                if ins.op_tok == 0x7F13 {
                    b.extend_from_slice(&[0x42, 0x00]);
                }
                b.push(0x20); b.extend(encode_uleb128(scratch0));
                b.extend_from_slice(&[0x42, 0x01]);
                b.push(0x20); b.extend(encode_uleb128(scratch1));
                b.push(0x20); b.extend(encode_uleb128(scratch1));
                b.push(0x50);
                b.push(0x20); b.extend(encode_uleb128(scratch0));
                b.push(0x42); b.extend(encode_sleb128(i64::MIN));
                b.push(0x51);
                b.push(0x20); b.extend(encode_uleb128(scratch1));
                b.extend_from_slice(&[0x42, 0x7F, 0x51]);
                b.extend_from_slice(&[0x71, 0x72, 0x1B]);
                if ins.op_tok == 0x7F13 {
                    b.push(0x7F);
                    b.push(0x20); b.extend(encode_uleb128(scratch1));
                    b.extend_from_slice(&[0x50, 0x1B]);
                } else {
                    b.push(0x81);
                }
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F15 | 0x7F28 => {
                b.extend_from_slice(&[0x42, 0x00]);
                emit_wasm_op(b, &ins.operands[0]);
                b.push(0x7D);
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F16 => {
                b.extend_from_slice(&[0x42, 0x00]);
                emit_wasm_op(b, &ins.operands[0]);
                b.push(0x7D);
                emit_wasm_op(b, &ins.operands[0]);
                emit_wasm_op(b, &ins.operands[0]);
                b.extend_from_slice(&[0x42, 0x00, 0x53, 0x1B]);
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F21 => {
                emit_wasm_op(b, &ins.operands[0]);
                b.extend_from_slice(&[0x42, 0x7F, 0x85]);
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F17 | 0x7F29 | 0x7F18 | 0x7F2A => {
                emit_wasm_op(b, &ins.operands[0]);
                emit_wasm_op(b, &ins.operands[1]);
                emit_wasm_op(b, &ins.operands[0]);
                emit_wasm_op(b, &ins.operands[1]);
                b.push(if matches!(ins.op_tok, 0x7F17 | 0x7F29) { 0x53 } else { 0x55 });
                b.push(0x1B);
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F2B => {
                b.extend_from_slice(&[0x42, 0x00]);
                emit_wasm_op(b, &ins.operands[0]);
                emit_wasm_op(b, &ins.operands[1]);
                b.extend_from_slice(&[0x7E, 0x7D]);
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F24 | 0x7F25 | 0x7F26 | 0x7F27 => {
                emit_wasm_op(b, &ins.operands[0]);
                emit_wasm_op(b, &ins.operands[1]);
                let cmp_op = match ins.op_tok {
                    0x7F24 => 0x51,
                    0x7F25 => 0x52,
                    0x7F26 => 0x53,
                    _      => 0x55,
                };
                b.extend_from_slice(&[cmp_op, 0xAD, 0x42, 0x02, 0x7E, 0x42, 0x01, 0x7D]);
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F2C => {
                emit_wasm_op(b, &ins.operands[0]);
                emit_wasm_op(b, &ins.operands[1]);
                b.push(0x7C);
                emit_wasm_op(b, &ins.operands[2]);
                b.push(0x7C);
                b.push(0x22); b.extend(encode_uleb128(scratch0));
                b.extend_from_slice(&[0x42, 0x00, 0x55, 0xAD]);
                b.push(0x20); b.extend(encode_uleb128(scratch0));
                b.extend_from_slice(&[0x42, 0x00, 0x53, 0xAD, 0x7D]);
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F2D => {
                emit_wasm_op(b, &ins.operands[0]);
                emit_wasm_op(b, &ins.operands[1]);
                b.extend_from_slice(&[0x55, 0xAD]);
                emit_wasm_op(b, &ins.operands[0]);
                emit_wasm_op(b, &ins.operands[1]);
                b.extend_from_slice(&[0x53, 0xAD, 0x7D]);
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F45 => {
                emit_wasm_op(b, &ins.operands[1]);
                emit_wasm_op(b, &ins.operands[2]);
                emit_wasm_op(b, &ins.operands[0]);
                b.extend_from_slice(&[0x42, 0x00, 0x53, 0x1B]);
                b.push(0x21); b.extend(encode_uleb128(scratch1));
                emit_wasm_op(b, &ins.operands[3]);
                b.push(0x20); b.extend(encode_uleb128(scratch1));
                emit_wasm_op(b, &ins.operands[0]);
                b.extend_from_slice(&[0x42, 0x00, 0x55, 0x1B]);
                emit_wasm_set_dest(b, ins.dest_reg);
            }
            0x7F4A => {
                emit_wasm_op(b, &ins.operands[0]);
                b.push(0x0F);
            }
            _ => {}
        }
    };

    if !has_control_flow {
        for ins in &func.instructions {
            emit_straight_ins(&mut body, ins);
        }
    } else {
        body.extend_from_slice(&[0x03, 0x40]); // loop (empty blocktype)
        for (ins_idx, ins) in func.instructions.iter().enumerate() {
            body.push(0x20);
            body.extend(encode_uleb128(pc_local));
            body.push(0x42);
            body.extend(encode_sleb128(ins_idx as i64));
            body.extend_from_slice(&[0x51, 0x04, 0x40]); // i64.eq, if (empty blocktype)

            match ins.op_tok {
                0x7F40 => {
                    body.push(0x42);
                    body.extend(encode_sleb128((ins_idx + 1) as i64));
                    body.push(0x21);
                    body.extend(encode_uleb128(pc_local));
                    body.extend_from_slice(&[0x0C, 0x01]);
                }
                0x7F41 => {
                    let lbl = ins.operands.first().and_then(extract_label_id).ok_or("Missing JMP label")?;
                    let target_pc = lookup_lbl_pc(lbl)?;
                    body.push(0x42);
                    body.extend(encode_sleb128(target_pc));
                    body.push(0x21);
                    body.extend(encode_uleb128(pc_local));
                    body.extend_from_slice(&[0x0C, 0x01]);
                }
                0x7F43 | 0x7F44 => {
                    let lbl = ins.operands.get(1).and_then(extract_label_id).ok_or("Missing BR label")?;
                    let target_pc = lookup_lbl_pc(lbl)?;
                    body.push(0x42);
                    body.extend(encode_sleb128(target_pc));
                    body.push(0x42);
                    body.extend(encode_sleb128((ins_idx + 1) as i64));
                    emit_wasm_op(&mut body, &ins.operands[0]);
                    body.push(0x42);
                    body.extend(encode_sleb128(if ins.op_tok == 0x7F43 { 1 } else { -1 }));
                    body.extend_from_slice(&[0x51, 0x1B]);
                    body.push(0x21);
                    body.extend(encode_uleb128(pc_local));
                    body.extend_from_slice(&[0x0C, 0x01]);
                }
                0x7F42 => {
                    let l_neg = ins.operands.get(1).and_then(extract_label_id).ok_or("Missing BR_TRIT neg")?;
                    let l_zero = ins.operands.get(2).and_then(extract_label_id).ok_or("Missing BR_TRIT zero")?;
                    let l_pos = ins.operands.get(3).and_then(extract_label_id).ok_or("Missing BR_TRIT pos")?;
                    let pc_neg = lookup_lbl_pc(l_neg)?;
                    let pc_zero = lookup_lbl_pc(l_zero)?;
                    let pc_pos = lookup_lbl_pc(l_pos)?;
                    body.push(0x42);
                    body.extend(encode_sleb128(pc_neg));
                    body.push(0x42);
                    body.extend(encode_sleb128(pc_zero));
                    emit_wasm_op(&mut body, &ins.operands[0]);
                    body.extend_from_slice(&[0x42, 0x00, 0x53, 0x1B]);
                    body.push(0x21);
                    body.extend(encode_uleb128(scratch1));
                    body.push(0x42);
                    body.extend(encode_sleb128(pc_pos));
                    body.push(0x20);
                    body.extend(encode_uleb128(scratch1));
                    emit_wasm_op(&mut body, &ins.operands[0]);
                    body.extend_from_slice(&[0x42, 0x00, 0x55, 0x1B]);
                    body.push(0x21);
                    body.extend(encode_uleb128(pc_local));
                    body.extend_from_slice(&[0x0C, 0x01]);
                }
                0x7F4A => {
                    emit_wasm_op(&mut body, &ins.operands[0]);
                    body.push(0x0F);
                }
                _ => {
                    emit_straight_ins(&mut body, ins);
                    body.push(0x42);
                    body.extend(encode_sleb128((ins_idx + 1) as i64));
                    body.push(0x21);
                    body.extend(encode_uleb128(pc_local));
                    body.extend_from_slice(&[0x0C, 0x01]);
                }
            }
            body.push(0x0B); // end if
        }
        body.push(0x0B); // end loop
        body.extend_from_slice(&[0x42, 0x00]); // fallback i64.const 0
    }
    body.push(0x0B);

    let mut type_sec = vec![0x01, 0x60];
    type_sec.extend(encode_uleb128(num_params));
    type_sec.resize(type_sec.len() + num_params, 0x7E);
    type_sec.extend_from_slice(&[0x01, 0x7E]);

    let func_sec = vec![0x01, 0x00];
    let exp_sec = vec![0x01, 0x07, b'o', b'm', b'n', b'i', b'_', b'f', b'n', 0x00, 0x00];
    let mut code_sec = vec![0x01];
    code_sec.extend(encode_uleb128(body.len()));
    code_sec.extend(body);

    let mut wasm = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
    for (sec_id, payload) in [(1u8, type_sec), (3u8, func_sec), (7u8, exp_sec), (10u8, code_sec)] {
        wasm.push(sec_id);
        wasm.extend(encode_uleb128(payload.len()));
        wasm.extend(payload);
    }
    Ok(wasm)
}

fn decode_uleb128(bytes: &[u8], pos: &mut usize) -> Result<usize, String> {
    let mut result: usize = 0;
    let mut shift: u32 = 0;
    loop {
        if *pos >= bytes.len() {
            return Err(String::from("Unexpected EOF in Wasm ULEB128"));
        }
        let byte = bytes[*pos];
        *pos += 1;
        result |= ((byte & 0x7F) as usize) << shift;
        if (byte & 0x80) == 0 {
            break;
        }
        shift += 7;
        if shift >= 64 {
            return Err(String::from("Wasm ULEB128 overflow"));
        }
    }
    Ok(result)
}

fn decode_sleb128(bytes: &[u8], pos: &mut usize) -> Result<i64, String> {
    let mut result: i64 = 0;
    let mut shift: u32 = 0;
    let mut byte: u8;
    loop {
        if *pos >= bytes.len() {
            return Err(String::from("Unexpected EOF in Wasm SLEB128"));
        }
        byte = bytes[*pos];
        *pos += 1;
        result |= ((byte & 0x7F) as i64) << shift;
        shift += 7;
        if (byte & 0x80) == 0 {
            break;
        }
        if shift >= 64 {
            break;
        }
    }
    if shift < 64 && (byte & 0x40) != 0 {
        result |= !0i64 << shift;
    }
    Ok(result)
}

fn skip_wasm_to_matching_end(body: &[u8], mut pos: usize) -> Result<usize, String> {
    let mut depth = 1usize;
    while pos < body.len() {
        let op = body[pos];
        pos += 1;
        match op {
            0x02 | 0x03 | 0x04 => {
                if pos >= body.len() {
                    return Err(String::from("Truncated Wasm block header during skip"));
                }
                pos += 1;
                depth += 1;
            }
            0x0B => {
                depth -= 1;
                if depth == 0 {
                    return Ok(pos);
                }
            }
            0x0C | 0x20 | 0x21 | 0x22 => {
                let _ = decode_uleb128(body, &mut pos)?;
            }
            0x42 => {
                let _ = decode_sleb128(body, &mut pos)?;
            }
            _ => {}
        }
    }
    Err(String::from("Unclosed Wasm block while skipping to end"))
}

pub const DEFAULT_WASM_STEP_LIMIT: u64 = 50_000_000;
pub const DEFAULT_WASM_MEMORY_BUDGET_BYTES: usize = 64 * 1024 * 1024;

/// Self-contained 64-bit WebAssembly v1 binary validator and stack-machine evaluator for
/// differential verification of `compile_pure_u16_to_wasm` against `OmniBinaryVm` and x86-64 ELF.
pub fn execute_wasm_module_i64(wasm: &[u8], args: &[i64]) -> Result<i64, String> {
    execute_wasm_module_i64_with_budgets(
        wasm,
        args,
        DEFAULT_WASM_STEP_LIMIT,
        DEFAULT_WASM_MEMORY_BUDGET_BYTES,
    )
}

/// Executes a 64-bit WebAssembly v1 binary under explicit `step_limit` and `memory_budget_bytes` caps.
pub fn execute_wasm_module_i64_with_budgets(
    wasm: &[u8],
    args: &[i64],
    step_limit: u64,
    memory_budget_bytes: usize,
) -> Result<i64, String> {
    if wasm.len() < 8 || &wasm[0..8] != &[0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00] {
        return Err(String::from("Invalid WebAssembly '\0asm' v1 header"));
    }
    let mut pos = 8usize;
    let mut num_params = 0usize;
    let mut code_body: Option<&[u8]> = None;

    while pos < wasm.len() {
        let sec_id = wasm[pos];
        pos += 1;
        let sec_len = decode_uleb128(wasm, &mut pos)?;
        if pos + sec_len > wasm.len() {
            return Err(String::from("Truncated Wasm section"));
        }
        let sec_end = pos + sec_len;
        if sec_id == 1 {
            let mut p = pos;
            let type_cnt = decode_uleb128(wasm, &mut p)?;
            if type_cnt == 0 || p >= sec_end || wasm[p] != 0x60 {
                return Err(String::from("Invalid Wasm Type section"));
            }
            p += 1;
            num_params = decode_uleb128(wasm, &mut p)?;
        } else if sec_id == 10 {
            let mut p = pos;
            let fn_cnt = decode_uleb128(wasm, &mut p)?;
            if fn_cnt == 0 {
                return Err(String::from("Empty Wasm Code section"));
            }
            let body_len = decode_uleb128(wasm, &mut p)?;
            if p + body_len > sec_end {
                return Err(String::from("Truncated Wasm function body"));
            }
            code_body = Some(&wasm[p..p + body_len]);
        }
        pos = sec_end;
    }

    let body = code_body.ok_or_else(|| String::from("Missing Wasm Code section"))?;
    let mut bpos = 0usize;
    let decl_cnt = decode_uleb128(body, &mut bpos)?;
    let mut total_locals = num_params;
    for _ in 0..decl_cnt {
        let cnt = decode_uleb128(body, &mut bpos)?;
        if bpos >= body.len() {
            return Err(String::from("Truncated Wasm local decl"));
        }
        bpos += 1;
        total_locals += cnt;
    }

    let frame_bytes = wasm.len().saturating_add(total_locals.max(args.len()).saturating_mul(8));
    if frame_bytes > memory_budget_bytes {
        return Err(format!(
            "Wasm AOT memory budget exceeded ({} > {} bytes)",
            frame_bytes, memory_budget_bytes
        ));
    }

    let mut locals = vec![0i64; total_locals.max(args.len())];
    for (i, &v) in args.iter().enumerate() {
        if i < locals.len() {
            locals[i] = v;
        }
    }
    let mut stack: Vec<i64> = Vec::with_capacity(32);
    let mut ctrl_stack: Vec<(u8, usize)> = Vec::with_capacity(8);
    let mut steps_executed: u64 = 0;

    while bpos < body.len() {
        steps_executed = steps_executed.saturating_add(1);
        if steps_executed > step_limit {
            return Err(format!(
                "Wasm AOT step budget exceeded ({} > {})",
                steps_executed, step_limit
            ));
        }
        let runtime_bytes = frame_bytes.saturating_add(stack.len().saturating_add(ctrl_stack.len()).saturating_mul(8));
        if runtime_bytes > memory_budget_bytes {
            return Err(format!(
                "Wasm AOT memory budget exceeded ({} > {} bytes)",
                runtime_bytes, memory_budget_bytes
            ));
        }
        let op = body[bpos];
        bpos += 1;
        match op {
            0x02 | 0x03 => {
                if bpos >= body.len() {
                    return Err(String::from("Truncated Wasm block/loop header"));
                }
                bpos += 1;
                ctrl_stack.push((op, bpos));
            }
            0x04 => {
                if bpos >= body.len() {
                    return Err(String::from("Truncated Wasm if header"));
                }
                bpos += 1;
                let cond = stack.pop().ok_or("Wasm stack underflow on if")?;
                if cond != 0 {
                    ctrl_stack.push((0x04, bpos));
                } else {
                    bpos = skip_wasm_to_matching_end(body, bpos)?;
                }
            }
            0x0C => {
                let depth = decode_uleb128(body, &mut bpos)?;
                if depth >= ctrl_stack.len() {
                    return Err(String::from("Wasm br depth out of bounds"));
                }
                for _ in 0..depth {
                    ctrl_stack.pop();
                }
                let &(target_op, target_pos) = ctrl_stack.last().unwrap();
                if target_op == 0x03 {
                    bpos = target_pos;
                } else {
                    ctrl_stack.pop();
                    bpos = skip_wasm_to_matching_end(body, target_pos)?;
                }
            }
            0x20 => {
                let idx = decode_uleb128(body, &mut bpos)?;
                stack.push(*locals.get(idx).ok_or("Wasm local.get out of bounds")?);
            }
            0x21 => {
                let idx = decode_uleb128(body, &mut bpos)?;
                let v = stack.pop().ok_or("Wasm stack underflow on local.set")?;
                if idx >= locals.len() {
                    return Err(String::from("Wasm local.set out of bounds"));
                }
                locals[idx] = v;
            }
            0x22 => {
                let idx = decode_uleb128(body, &mut bpos)?;
                let v = *stack.last().ok_or("Wasm stack underflow on local.tee")?;
                if idx >= locals.len() {
                    return Err(String::from("Wasm local.tee out of bounds"));
                }
                locals[idx] = v;
            }
            0x42 => {
                let v = decode_sleb128(body, &mut bpos)?;
                stack.push(v);
            }
            0x50 => {
                let a = stack.pop().ok_or("Wasm stack underflow on i64.eqz")?;
                stack.push(if a == 0 { 1 } else { 0 });
            }
            0x51 | 0x52 | 0x53 | 0x55 => {
                let b = stack.pop().ok_or("Wasm stack underflow on cmp b")?;
                let a = stack.pop().ok_or("Wasm stack underflow on cmp a")?;
                let r = match op {
                    0x51 => a == b,
                    0x52 => a != b,
                    0x53 => a < b,
                    _    => a > b,
                };
                stack.push(if r { 1 } else { 0 });
            }
            0x71 => {
                let b = stack.pop().ok_or("Wasm stack underflow on i32.and")?;
                let a = stack.pop().ok_or("Wasm stack underflow on i32.and")?;
                stack.push((a & b) & 1);
            }
            0x72 => {
                let b = stack.pop().ok_or("Wasm stack underflow on i32.or")?;
                let a = stack.pop().ok_or("Wasm stack underflow on i32.or")?;
                stack.push((a | b) & 1);
            }
            0x7C | 0x7D | 0x7E | 0x83 | 0x84 | 0x85 | 0x86 | 0x87 => {
                let b = stack.pop().ok_or("Wasm stack underflow on binop b")?;
                let a = stack.pop().ok_or("Wasm stack underflow on binop a")?;
                let r = match op {
                    0x7C => a.wrapping_add(b),
                    0x7D => a.wrapping_sub(b),
                    0x7E => a.wrapping_mul(b),
                    0x83 => a & b,
                    0x84 => a | b,
                    0x85 => a ^ b,
                    0x86 => a.wrapping_shl((b & 63) as u32),
                    _    => a.wrapping_shr((b & 63) as u32),
                };
                stack.push(r);
            }
            0x7F => {
                let b = stack.pop().ok_or("Wasm stack underflow on i64.div_s b")?;
                let a = stack.pop().ok_or("Wasm stack underflow on i64.div_s a")?;
                if b == 0 || (a == i64::MIN && b == -1) {
                    return Err(String::from("Wasm hardware trap on unguarded i64.div_s"));
                }
                stack.push(a.wrapping_div(b));
            }
            0x81 => {
                let b = stack.pop().ok_or("Wasm stack underflow on i64.rem_s b")?;
                let a = stack.pop().ok_or("Wasm stack underflow on i64.rem_s a")?;
                if b == 0 {
                    return Err(String::from("Wasm hardware trap on unguarded i64.rem_s"));
                }
                stack.push(if a == i64::MIN && b == -1 { 0 } else { a.wrapping_rem(b) });
            }
            0xAD => {
                let a = stack.pop().ok_or("Wasm stack underflow on i64.extend_i32_u")?;
                stack.push((a as u32) as i64);
            }
            0x1B => {
                let c = stack.pop().ok_or("Wasm stack underflow on select c")?;
                let v2 = stack.pop().ok_or("Wasm stack underflow on select v2")?;
                let v1 = stack.pop().ok_or("Wasm stack underflow on select v1")?;
                stack.push(if c != 0 { v1 } else { v2 });
            }
            0x0F => {
                return stack.pop().ok_or_else(|| String::from("Wasm stack empty on return"));
            }
            0x0B => {
                if ctrl_stack.pop().is_none() {
                    break;
                }
            }
            other => {
                return Err(format!("Unsupported Wasm opcode 0x{:02X} in evaluator", other));
            }
        }
    }
    stack.pop().ok_or_else(|| String::from("Wasm execution finished with empty stack"))
}

// ==============================================================================
// 13. STEP 7 (PURE RUST): LOOP-BASED STAGE-1 PURE-`u16` COMPILER LOWERING `.omni_bc` -> ELF64
// ==============================================================================

fn alloc_ssa_ins(
    ins: &mut Vec<Vec<u16>>,
    next_reg: &mut u16,
    tc_safe: u16,
    op: u16,
    ty: u16,
    ops: Vec<u16>,
) -> u16 {
    let r = *next_reg;
    *next_reg += 1;
    let mut pkt = vec![r, op, ty, tc_safe];
    pkt.extend(ops);
    pkt.push(TOK_INS_END);
    ins.push(pkt);
    r
}

fn set_ssa_reg(
    ins: &mut Vec<Vec<u16>>,
    tc_safe: u16,
    r: u16,
    op: u16,
    ty: u16,
    ops: Vec<u16>,
) {
    let mut pkt = vec![r, op, ty, tc_safe];
    pkt.extend(ops);
    pkt.push(TOK_INS_END);
    ins.push(pkt);
}

fn emit_lbl_ins(ins: &mut Vec<Vec<u16>>, lbl_id: u16) {
    ins.push(vec![0x7F40, TOK_IMM_I16, lbl_id, TOK_INS_END]);
}

fn emit_jmp_ins(ins: &mut Vec<Vec<u16>>, lbl_id: u16) {
    ins.push(vec![0x7F41, TOK_IMM_I16, lbl_id, TOK_INS_END]);
}

fn emit_br_pos_ins(ins: &mut Vec<Vec<u16>>, cond_reg: u16, lbl_id: u16) {
    ins.push(vec![0x7F43, cond_reg, TOK_IMM_I16, lbl_id, TOK_INS_END]);
}

fn alloc_lbl(next_lbl: &mut u16) -> u16 {
    let l = *next_lbl;
    *next_lbl += 1;
    l
}

fn emit_iset_bytes_in_place(
    ins: &mut Vec<Vec<u16>>,
    tc_safe: u16,
    r_buf: u16,
    bytes: &[u8],
) {
    for chunk in bytes.chunks(24) {
        let mut ops = vec![r_buf];
        for &b in chunk {
            ops.push(TOK_IMM_I16);
            ops.push(b as u16);
        }
        set_ssa_reg(ins, tc_safe, r_buf, 0x7F0F, TY_BYTES, ops);
    }
}

pub fn build_step7_stage1_compiler_stream() -> Vec<u16> {
    let tc_safe = pack_tc(false, CAP_ID_PURE);
    let mut ins: Vec<Vec<u16>> = Vec::new();
    let mut next_reg: u16 = REG_BASE;
    let mut next_lbl: u16 = 0;

    let r_buf      = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F0D, TY_BYTES, vec![TOK_IMM_I16, 0]);
    let r_pos      = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 1]);
    let r_cur      = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 0]);
    let r_tok0     = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 0]);
    let r_op       = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 0]);
    let r_disp_dst = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 0]);
    let r_otok     = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 0]);
    let r_tmp      = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 0]);
    let r_tmp2     = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 0]);
    let r_tmp3     = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 0]);
    let r_cond     = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F09, TY_TRIT,  vec![TOK_TRIT_NEG]);
    let r_pass     = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 0]);
    let r_lid      = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 0]);
    let r_loff     = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 0]);
    let r_retty    = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 0]);

    const MAX_LABELS: usize = 83;
    let mut r_lbl = [0u16; MAX_LABELS];
    for k in 0..MAX_LABELS {
        r_lbl[k] = alloc_ssa_ins(&mut ins, &mut next_reg, tc_safe, 0x7F06, TY_I64, vec![TOK_IMM_I16, 0]);
    }

    let lbl_pass_entry  = alloc_lbl(&mut next_lbl);
    let lbl_find_fn     = alloc_lbl(&mut next_lbl);
    let lbl_fn_found    = alloc_lbl(&mut next_lbl);
    let lbl_ty_bytes_pr = alloc_lbl(&mut next_lbl);
    let lbl_pro_done    = alloc_lbl(&mut next_lbl);
    let lbl_ins_loop    = alloc_lbl(&mut next_lbl);

    emit_lbl_ins(&mut ins, lbl_pass_entry);
    set_ssa_reg(&mut ins, tc_safe, r_buf, 0x7F0D, TY_BYTES, vec![TOK_IMM_I16, 0]);
    set_ssa_reg(&mut ins, tc_safe, r_pos, 0x7F06, TY_I64,   vec![TOK_IMM_I16, 1]);

    // Locate TOK_FN_DEF and extract ret_ty_tok before emitting the prologue
    emit_lbl_ins(&mut ins, lbl_find_fn);
    set_ssa_reg(&mut ins, tc_safe, r_tok0, 0x7F0E, TY_I64,  vec![0x7EC0, r_pos]);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_tok0, TOK_IMM_I16, TOK_FN_DEF]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_fn_found);
    set_ssa_reg(&mut ins, tc_safe, r_pos,  0x7F10, TY_I64,  vec![r_pos, TOK_IMM_I16, 1]);
    emit_jmp_ins(&mut ins, lbl_find_fn);

    emit_lbl_ins(&mut ins, lbl_fn_found);
    set_ssa_reg(&mut ins, tc_safe, r_tmp,   0x7F10, TY_I64, vec![r_pos, TOK_IMM_I16, 2]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp,   0x7F0E, TY_I64, vec![0x7EC0, r_tmp]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp,   0x7F12, TY_I64, vec![r_tmp, TOK_IMM_I16, 3]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp2,  0x7F10, TY_I64, vec![r_pos, TOK_IMM_I16, 3]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp2,  0x7F10, TY_I64, vec![r_tmp2, r_tmp]);
    set_ssa_reg(&mut ins, tc_safe, r_retty, 0x7F0E, TY_I64, vec![0x7EC0, r_tmp2]);
    set_ssa_reg(&mut ins, tc_safe, r_pos,   0x7F10, TY_I64, vec![r_tmp2, TOK_IMM_I16, 3]);

    let mut elf_hdr = [0u8; 120];
    elf_hdr[0..4].copy_from_slice(&[0x7F, b'E', b'L', b'F']);
    elf_hdr[4] = 2; elf_hdr[5] = 1; elf_hdr[6] = 1;
    elf_hdr[16] = 2; elf_hdr[18] = 0x3E; elf_hdr[20] = 1;
    elf_hdr[24] = 0x78; elf_hdr[25] = 0x00; elf_hdr[26] = 0x40;
    elf_hdr[32] = 64; elf_hdr[52] = 64; elf_hdr[54] = 56; elf_hdr[56] = 1;
    elf_hdr[64] = 1; elf_hdr[68] = 5;
    elf_hdr[82] = 0x40; elf_hdr[90] = 0x40;
    elf_hdr[113] = 0x10;
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &elf_hdr);

    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x55, 0x48, 0x89, 0xE5, 0x48, 0x81, 0xEC, 0x00, 0x00, 0x03, 0x00]);

    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_retty, TOK_IMM_I16, TY_BYTES]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_ty_bytes_pr);

    // Non-TY_BYTES prologue
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x49, 0xBC]);
    set_ssa_reg(&mut ins, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, (-8i16) as u16, 0x7EC1]);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x49, 0xBD]);
    set_ssa_reg(&mut ins, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, (-8i16) as u16, 0x7EC2]);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x49, 0xBE]);
    set_ssa_reg(&mut ins, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, (-8i16) as u16, 0x7EC3]);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x49, 0xC7, 0xC7, 0x00, 0x00, 0x00, 0x00]);
    for (argc_min, disp_argv, reg_code) in [(2u8, 0x18u8, 4u8), (3, 0x20, 5), (4, 0x28, 6), (5, 0x30, 7)] {
        let mut cli_mc = Vec::new();
        emit_cli_arg_parser(&mut cli_mc, argc_min, disp_argv, reg_code);
        emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &cli_mc);
    }
    emit_jmp_ins(&mut ins, lbl_pro_done);

    // TY_BYTES prologue
    emit_lbl_ins(&mut ins, lbl_ty_bytes_pr);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x49, 0xC7, 0xC4, 0x00, 0x00, 0x00, 0x00]);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x49, 0xBD]);
    set_ssa_reg(&mut ins, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, (-8i16) as u16, 0x7EC1]);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x49, 0xBE]);
    set_ssa_reg(&mut ins, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, (-8i16) as u16, 0x7EC2]);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x49, 0xBF]);
    set_ssa_reg(&mut ins, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, (-8i16) as u16, 0x7EC3]);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[
        0x48, 0x83, 0x7D, 0x08, 0x02,
        0x7C, 0x2E,
        0x48, 0x8B, 0x7D, 0x18,
        0x31, 0xF6,
        0x31, 0xD2,
        0xB8, 0x02, 0x00, 0x00, 0x00,
        0x0F, 0x05,
        0x48, 0x85, 0xC0,
        0x78, 0x1A,
        0x48, 0x89, 0xC7,
        0x48, 0x8D, 0xB5, 0x00, 0x20, 0xFD, 0xFF,
        0xBA, 0x00, 0xE0, 0x00, 0x00,
        0x31, 0xC0,
        0x0F, 0x05,
        0xB8, 0x03, 0x00, 0x00, 0x00,
        0x0F, 0x05,
        0x45, 0x31, 0xDB,
    ]);
    for (argc_min, disp_argv, reg_code) in [(3u8, 0x20u8, 5u8), (4, 0x28, 6), (5, 0x30, 7)] {
        let mut cli_mc = Vec::new();
        emit_cli_arg_parser(&mut cli_mc, argc_min, disp_argv, reg_code);
        emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &cli_mc);
    }

    emit_lbl_ins(&mut ins, lbl_pro_done);

    let emit_decode_operand = |ins_v: &mut Vec<Vec<u16>>, next_l: &mut u16, into_rcx: bool| {
        let l_imm  = alloc_lbl(next_l);
        let l_par  = alloc_lbl(next_l);
        let l_done = alloc_lbl(next_l);
        let reg_ld: [u8; 3] = [0x48, 0x8B, if into_rcx { 0x8D } else { 0x85 }];
        let imm_ld: [u8; 2] = [0x48, if into_rcx { 0xB9 } else { 0xB8 }];
        let par_b: u16      = if into_rcx { 0xE1 } else { 0xE0 };

        set_ssa_reg(ins_v, tc_safe, r_otok, 0x7F0E, TY_I64,  vec![0x7EC0, r_cur]);
        set_ssa_reg(ins_v, tc_safe, r_cond, 0x7F27, TY_TRIT, vec![r_otok, TOK_IMM_I16, TOK_INS_END]);
        emit_br_pos_ins(ins_v, r_cond, l_imm);
        set_ssa_reg(ins_v, tc_safe, r_cond, 0x7F27, TY_TRIT, vec![r_otok, TOK_IMM_I16, REG_MAX]);
        emit_br_pos_ins(ins_v, r_cond, l_par);

        set_ssa_reg(ins_v, tc_safe, r_tmp, 0x7F11, TY_I64,   vec![TOK_IMM_I16, 0x7DFF, r_otok]);
        set_ssa_reg(ins_v, tc_safe, r_tmp, 0x7F12, TY_I64,   vec![r_tmp, TOK_IMM_I16, 8]);
        emit_iset_bytes_in_place(ins_v, tc_safe, r_buf, &reg_ld);
        set_ssa_reg(ins_v, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, (-4i16) as u16, r_tmp]);
        set_ssa_reg(ins_v, tc_safe, r_cur, 0x7F10, TY_I64,   vec![r_cur, TOK_IMM_I16, 1]);
        emit_jmp_ins(ins_v, l_done);

        emit_lbl_ins(ins_v, l_imm);
        set_ssa_reg(ins_v, tc_safe, r_cond, 0x7F2D, TY_TRIT, vec![r_otok, TOK_IMM_I16, TOK_IMM_I32]);
        set_ssa_reg(ins_v, tc_safe, r_tmp,  0x7F10, TY_I64,  vec![r_cur, TOK_IMM_I16, 1]);
        set_ssa_reg(ins_v, tc_safe, r_tmp,  0x7F0E, TY_I64,  vec![0x7EC0, r_tmp]);
        set_ssa_reg(ins_v, tc_safe, r_tmp2, 0x7F22, TY_I64,  vec![r_tmp, TOK_IMM_I16, 48]);
        set_ssa_reg(ins_v, tc_safe, r_tmp2, 0x7F23, TY_I64,  vec![r_tmp2, TOK_IMM_I16, 48]);
        set_ssa_reg(ins_v, tc_safe, r_tmp3, 0x7F10, TY_I64,  vec![r_cur, TOK_IMM_I16, 2]);
        set_ssa_reg(ins_v, tc_safe, r_tmp3, 0x7F0E, TY_I64,  vec![0x7EC0, r_tmp3]);
        set_ssa_reg(ins_v, tc_safe, r_tmp,  0x7F22, TY_I64,  vec![r_tmp, TOK_IMM_I16, 16]);
        set_ssa_reg(ins_v, tc_safe, r_tmp3, 0x7F1F, TY_I64,  vec![r_tmp, r_tmp3]);
        set_ssa_reg(ins_v, tc_safe, r_tmp3, 0x7F22, TY_I64,  vec![r_tmp3, TOK_IMM_I16, 32]);
        set_ssa_reg(ins_v, tc_safe, r_tmp3, 0x7F23, TY_I64,  vec![r_tmp3, TOK_IMM_I16, 32]);
        set_ssa_reg(ins_v, tc_safe, r_tmp,  0x7F11, TY_I64,  vec![r_otok, TOK_IMM_I16, TOK_TRIT_ZERO]);
        set_ssa_reg(ins_v, tc_safe, r_tmp,  0x7F45, TY_I64,  vec![r_cond, r_tmp2, r_tmp3, r_tmp]);
        emit_iset_bytes_in_place(ins_v, tc_safe, r_buf, &imm_ld);
        set_ssa_reg(ins_v, tc_safe, r_buf,  0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, (-8i16) as u16, r_tmp]);
        set_ssa_reg(ins_v, tc_safe, r_tmp2, 0x7F45, TY_I64,  vec![r_cond, TOK_IMM_I16, 2, TOK_IMM_I16, 3, TOK_IMM_I16, 1]);
        set_ssa_reg(ins_v, tc_safe, r_cur,  0x7F10, TY_I64,  vec![r_cur, r_tmp2]);
        emit_jmp_ins(ins_v, l_done);

        emit_lbl_ins(ins_v, l_par);
        set_ssa_reg(ins_v, tc_safe, r_tmp, 0x7F11, TY_I64,   vec![r_otok, TOK_IMM_I16, PAR_BASE]);
        set_ssa_reg(ins_v, tc_safe, r_tmp, 0x7F12, TY_I64,   vec![r_tmp, TOK_IMM_I16, 8]);
        set_ssa_reg(ins_v, tc_safe, r_tmp, 0x7F10, TY_I64,   vec![r_tmp, TOK_IMM_I16, par_b]);
        set_ssa_reg(ins_v, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, 0x4C, TOK_IMM_I16, 0x89, r_tmp]);
        set_ssa_reg(ins_v, tc_safe, r_cur, 0x7F10, TY_I64,   vec![r_cur, TOK_IMM_I16, 1]);

        emit_lbl_ins(ins_v, l_done);
    };

    let emit_read_label_id = |ins_v: &mut Vec<Vec<u16>>| {
        set_ssa_reg(ins_v, tc_safe, r_otok, 0x7F0E, TY_I64,  vec![0x7EC0, r_cur]);
        set_ssa_reg(ins_v, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_otok, TOK_IMM_I16, TOK_IMM_I16]);
        set_ssa_reg(ins_v, tc_safe, r_tmp,  0x7F10, TY_I64,  vec![r_cur, TOK_IMM_I16, 1]);
        set_ssa_reg(ins_v, tc_safe, r_tmp,  0x7F0E, TY_I64,  vec![0x7EC0, r_tmp]);
        set_ssa_reg(ins_v, tc_safe, r_tmp2, 0x7F11, TY_I64,  vec![r_otok, TOK_IMM_I16, LBL_BASE]);
        set_ssa_reg(ins_v, tc_safe, r_lid,  0x7F45, TY_I64,  vec![r_cond, r_tmp2, r_tmp2, r_tmp]);
        set_ssa_reg(ins_v, tc_safe, r_tmp3, 0x7F45, TY_I64,  vec![r_cond, TOK_IMM_I16, 1, TOK_IMM_I16, 1, TOK_IMM_I16, 2]);
        set_ssa_reg(ins_v, tc_safe, r_cur,  0x7F10, TY_I64,  vec![r_cur, r_tmp3]);
    };

    let emit_rel32_for_lid = |ins_v: &mut Vec<Vec<u16>>| {
        set_ssa_reg(ins_v, tc_safe, r_loff, 0x7F06, TY_I64, vec![r_lbl[0]]);
        for k in 1..MAX_LABELS {
            set_ssa_reg(ins_v, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_lid, TOK_IMM_I16, k as u16]);
            set_ssa_reg(ins_v, tc_safe, r_loff, 0x7F45, TY_I64,  vec![r_cond, r_loff, r_loff, r_lbl[k]]);
        }
        set_ssa_reg(ins_v, tc_safe, r_tmp, 0x7F50, TY_I64,   vec![r_buf]);
        set_ssa_reg(ins_v, tc_safe, r_tmp, 0x7F11, TY_I64,   vec![r_tmp, TOK_IMM_I16, 116]);
        set_ssa_reg(ins_v, tc_safe, r_tmp, 0x7F11, TY_I64,   vec![r_loff, r_tmp]);
        set_ssa_reg(ins_v, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, (-4i16) as u16, r_tmp]);
    };

    let lbl_nop        = alloc_lbl(&mut next_lbl);
    let lbl_label      = alloc_lbl(&mut next_lbl);
    let lbl_jmp        = alloc_lbl(&mut next_lbl);
    let lbl_br_trit    = alloc_lbl(&mut next_lbl);
    let lbl_br_pos     = alloc_lbl(&mut next_lbl);
    let lbl_br_neg     = alloc_lbl(&mut next_lbl);
    let lbl_ret        = alloc_lbl(&mut next_lbl);
    let lbl_ret_bytes  = alloc_lbl(&mut next_lbl);
    let lbl_anew       = alloc_lbl(&mut next_lbl);
    let lbl_slen       = alloc_lbl(&mut next_lbl);
    let lbl_iget       = alloc_lbl(&mut next_lbl);
    let lbl_iset       = alloc_lbl(&mut next_lbl);
    let lbl_iset_m4    = alloc_lbl(&mut next_lbl);
    let lbl_iset_m8    = alloc_lbl(&mut next_lbl);
    let lbl_iset_m9    = alloc_lbl(&mut next_lbl);
    let lbl_iset_bloop = alloc_lbl(&mut next_lbl);
    let lbl_iset_bdone = alloc_lbl(&mut next_lbl);
    let lbl_dispatch_1 = alloc_lbl(&mut next_lbl);
    let lbl_tmaj       = alloc_lbl(&mut next_lbl);
    let lbl_sel_trit   = alloc_lbl(&mut next_lbl);
    let lbl_store_dst  = alloc_lbl(&mut next_lbl);
    let lbl_loop_end   = alloc_lbl(&mut next_lbl);
    let lbl_pass0_end  = alloc_lbl(&mut next_lbl);

    emit_lbl_ins(&mut ins, lbl_ins_loop);
    set_ssa_reg(&mut ins, tc_safe, r_tok0, 0x7F0E, TY_I64,  vec![0x7EC0, r_pos]);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_tok0, TOK_IMM_I16, TOK_MOD_END]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_loop_end);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_tok0, TOK_IMM_I16, 0]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_loop_end);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_tok0, TOK_IMM_I16, 0x7F00]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_nop);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_tok0, TOK_IMM_I16, 0x7F40]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_label);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_tok0, TOK_IMM_I16, 0x7F41]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_jmp);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_tok0, TOK_IMM_I16, 0x7F42]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_br_trit);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_tok0, TOK_IMM_I16, 0x7F43]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_br_pos);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_tok0, TOK_IMM_I16, 0x7F44]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_br_neg);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_tok0, TOK_IMM_I16, 0x7F4A]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_ret);

    set_ssa_reg(&mut ins, tc_safe, r_tmp,      0x7F10, TY_I64, vec![r_pos, TOK_IMM_I16, 1]);
    set_ssa_reg(&mut ins, tc_safe, r_op,       0x7F0E, TY_I64, vec![0x7EC0, r_tmp]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp,      0x7F11, TY_I64, vec![TOK_IMM_I16, 0x7DFF, r_tok0]);
    set_ssa_reg(&mut ins, tc_safe, r_disp_dst, 0x7F12, TY_I64, vec![r_tmp, TOK_IMM_I16, 8]);
    set_ssa_reg(&mut ins, tc_safe, r_cur,      0x7F10, TY_I64, vec![r_pos, TOK_IMM_I16, 4]);

    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F0D]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_anew);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F50]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_slen);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F0E]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_iget);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F0F]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_iset);

    emit_decode_operand(&mut ins, &mut next_lbl, false);

    set_ssa_reg(&mut ins, tc_safe, r_otok, 0x7F0E, TY_I64,  vec![0x7EC0, r_cur]);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_otok, TOK_IMM_I16, TOK_INS_END]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_dispatch_1);

    emit_decode_operand(&mut ins, &mut next_lbl, true);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F2C]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_tmaj);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F45]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_sel_trit);

    emit_lbl_ins(&mut ins, lbl_dispatch_1);
    for copy_op in [0x7F02u16, 0x7F03, 0x7F04, 0x7F06, 0x7F09, 0x7F57, 0x7F58, 0x7F59] {
        set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, copy_op]);
        emit_br_pos_ins(&mut ins, r_cond, lbl_store_dst);
    }

    let lbl_emit_3b   = alloc_lbl(&mut next_lbl);
    let lbl_imul      = alloc_lbl(&mut next_lbl);
    let lbl_idiv      = alloc_lbl(&mut next_lbl);
    let lbl_imod      = alloc_lbl(&mut next_lbl);
    let lbl_iabs      = alloc_lbl(&mut next_lbl);
    let lbl_imin_imax = alloc_lbl(&mut next_lbl);
    let lbl_txor      = alloc_lbl(&mut next_lbl);
    let lbl_cmp4      = alloc_lbl(&mut next_lbl);
    let lbl_tcmp      = alloc_lbl(&mut next_lbl);

    let ops_3b: &[(u16, u16, u16)] = &[
        (0x7F10, 0x01, 0xC8),
        (0x7F11, 0x29, 0xC8),
        (0x7F1E, 0x21, 0xC8),
        (0x7F1F, 0x09, 0xC8),
        (0x7F20, 0x31, 0xC8),
        (0x7F15, 0xF7, 0xD8),
        (0x7F28, 0xF7, 0xD8),
        (0x7F21, 0xF7, 0xD0),
        (0x7F22, 0xD3, 0xE0),
        (0x7F23, 0xD3, 0xF8),
    ];
    for &(op_c, _, _) in ops_3b {
        set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, op_c]);
        emit_br_pos_ins(&mut ins, r_cond, lbl_emit_3b);
    }
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F12]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_imul);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F13]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_idiv);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F14]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_imod);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F16]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_iabs);
    for op_mm in [0x7F17u16, 0x7F29, 0x7F18, 0x7F2A] {
        set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, op_mm]);
        emit_br_pos_ins(&mut ins, r_cond, lbl_imin_imax);
    }
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F2B]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_txor);
    for op_cm in [0x7F24u16, 0x7F25, 0x7F26, 0x7F27] {
        set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, op_cm]);
        emit_br_pos_ins(&mut ins, r_cond, lbl_cmp4);
    }
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F2D]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_tcmp);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_emit_3b);
    set_ssa_reg(&mut ins, tc_safe, r_tmp,  0x7F06, TY_I64, vec![TOK_IMM_I16, 0x01]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp2, 0x7F06, TY_I64, vec![TOK_IMM_I16, 0xC8]);
    for &(op_c, b1, b2) in &ops_3b[1..] {
        set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, op_c]);
        set_ssa_reg(&mut ins, tc_safe, r_tmp,  0x7F45, TY_I64,  vec![r_cond, r_tmp, r_tmp, TOK_IMM_I16, b1]);
        set_ssa_reg(&mut ins, tc_safe, r_tmp2, 0x7F45, TY_I64,  vec![r_cond, r_tmp2, r_tmp2, TOK_IMM_I16, b2]);
    }
    set_ssa_reg(&mut ins, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, 0x48, r_tmp, r_tmp2]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_imul);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x48, 0x0F, 0xAF, 0xC1]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_idiv);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x49, 0x89, 0xC0, 0x48, 0x85, 0xC9, 0x74, 0x1E, 0x48, 0x83, 0xF9, 0xFF, 0x75, 0x11, 0x49, 0xB9, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x4D, 0x39, 0xC8, 0x75, 0x02, 0xEB, 0x0A, 0x48, 0x99, 0x48, 0xF7, 0xF9, 0xEB, 0x03, 0x48, 0x31, 0xC0]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_imod);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x49, 0x89, 0xC0, 0x48, 0x85, 0xC9, 0x74, 0x1E, 0x48, 0x83, 0xF9, 0xFF, 0x75, 0x11, 0x49, 0xB9, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x4D, 0x39, 0xC8, 0x75, 0x02, 0xEB, 0x0A, 0x48, 0x99, 0x48, 0xF7, 0xF9, 0xEB, 0x03, 0x48, 0x31, 0xC0, 0x48, 0x89, 0xD0]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_iabs);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x48, 0x89, 0xC2, 0x48, 0xF7, 0xD8, 0x48, 0x0F, 0x48, 0xC2]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_imin_imax);
    set_ssa_reg(&mut ins, tc_safe, r_tmp,  0x7F06, TY_I64,  vec![TOK_IMM_I16, 0x4F]);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F18]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp,  0x7F45, TY_I64,  vec![r_cond, r_tmp, r_tmp, TOK_IMM_I16, 0x4C]);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, 0x7F2A]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp,  0x7F45, TY_I64,  vec![r_cond, r_tmp, r_tmp, TOK_IMM_I16, 0x4C]);
    set_ssa_reg(&mut ins, tc_safe, r_buf,  0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, 0x48, TOK_IMM_I16, 0x39, TOK_IMM_I16, 0xC8, TOK_IMM_I16, 0x48, TOK_IMM_I16, 0x0F, r_tmp, TOK_IMM_I16, 0xC1]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_txor);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x48, 0x0F, 0xAF, 0xC1, 0x48, 0xF7, 0xD8]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_cmp4);
    set_ssa_reg(&mut ins, tc_safe, r_tmp, 0x7F06, TY_I64, vec![TOK_IMM_I16, 0x94]);
    for (op_cm, scc) in [(0x7F25u16, 0x95u16), (0x7F26, 0x9C), (0x7F27, 0x9F)] {
        set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_op, TOK_IMM_I16, op_cm]);
        set_ssa_reg(&mut ins, tc_safe, r_tmp,  0x7F45, TY_I64,  vec![r_cond, r_tmp, r_tmp, TOK_IMM_I16, scc]);
    }
    set_ssa_reg(&mut ins, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![
        r_buf,
        TOK_IMM_I16, 0x48, TOK_IMM_I16, 0x39, TOK_IMM_I16, 0xC8,
        TOK_IMM_I16, 0x0F, r_tmp, TOK_IMM_I16, 0xC0,
        TOK_IMM_I16, 0x0F, TOK_IMM_I16, 0xB6, TOK_IMM_I16, 0xC0,
        TOK_IMM_I16, 0x48, TOK_IMM_I16, 0x8D, TOK_IMM_I16, 0x44, TOK_IMM_I16, 0x00, TOK_IMM_I16, 0xFF,
    ]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_tcmp);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x48, 0x39, 0xC8, 0x0F, 0x9F, 0xC2, 0x0F, 0x9C, 0xC1, 0x0F, 0xB6, 0xD2, 0x0F, 0xB6, 0xC9, 0x29, 0xCA, 0x48, 0x63, 0xC2]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_tmaj);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x48, 0x01, 0xC8]);
    emit_decode_operand(&mut ins, &mut next_lbl, true);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[
        0x48, 0x01, 0xC8, 0x48, 0x83, 0xF8, 0x00,
        0x0F, 0x9F, 0xC0, 0x0F, 0x9C, 0xC2,
        0x0F, 0xB6, 0xC0, 0x0F, 0xB6, 0xD2, 0x48, 0x29, 0xD0,
    ]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_sel_trit);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x49, 0x89, 0xC0, 0x48, 0x89, 0xCA]);
    emit_decode_operand(&mut ins, &mut next_lbl, true);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x49, 0x83, 0xF8, 0x00, 0x48, 0x0F, 0x44, 0xD1]);
    emit_decode_operand(&mut ins, &mut next_lbl, true);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x49, 0x83, 0xF8, 0x00, 0x48, 0x0F, 0x4F, 0xD1, 0x48, 0x89, 0xD0]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_anew);
    set_ssa_reg(&mut ins, tc_safe, r_cur, 0x7F10, TY_I64, vec![r_cur, TOK_IMM_I16, 2]);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x45, 0x31, 0xDB]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_slen);
    set_ssa_reg(&mut ins, tc_safe, r_cur, 0x7F10, TY_I64, vec![r_cur, TOK_IMM_I16, 1]);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x4C, 0x89, 0xD8]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_iget);
    set_ssa_reg(&mut ins, tc_safe, r_cur, 0x7F10, TY_I64, vec![r_cur, TOK_IMM_I16, 1]);
    emit_decode_operand(&mut ins, &mut next_lbl, false);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x0F, 0xB7, 0x84, 0x45, 0x00, 0x20, 0xFD, 0xFF]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_iset);
    set_ssa_reg(&mut ins, tc_safe, r_cur,  0x7F10, TY_I64,  vec![r_cur, TOK_IMM_I16, 1]);
    set_ssa_reg(&mut ins, tc_safe, r_otok, 0x7F0E, TY_I64,  vec![0x7EC0, r_cur]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp,  0x7F10, TY_I64,  vec![r_cur, TOK_IMM_I16, 1]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp,  0x7F0E, TY_I64,  vec![0x7EC0, r_tmp]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp,  0x7F22, TY_I64,  vec![r_tmp, TOK_IMM_I16, 48]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp,  0x7F23, TY_I64,  vec![r_tmp, TOK_IMM_I16, 48]);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_otok, TOK_IMM_I16, TOK_IMM_I16]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp2, 0x7F24, TY_TRIT, vec![r_tmp, TOK_IMM_I16, (-4i16) as u16]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp2, 0x7F29, TY_TRIT, vec![r_cond, r_tmp2]);
    emit_br_pos_ins(&mut ins, r_tmp2, lbl_iset_m4);
    set_ssa_reg(&mut ins, tc_safe, r_tmp2, 0x7F24, TY_TRIT, vec![r_tmp, TOK_IMM_I16, (-8i16) as u16]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp2, 0x7F29, TY_TRIT, vec![r_cond, r_tmp2]);
    emit_br_pos_ins(&mut ins, r_tmp2, lbl_iset_m8);
    set_ssa_reg(&mut ins, tc_safe, r_tmp2, 0x7F24, TY_TRIT, vec![r_tmp, TOK_IMM_I16, (-9i16) as u16]);
    set_ssa_reg(&mut ins, tc_safe, r_tmp2, 0x7F29, TY_TRIT, vec![r_cond, r_tmp2]);
    emit_br_pos_ins(&mut ins, r_tmp2, lbl_iset_m9);

    emit_lbl_ins(&mut ins, lbl_iset_bloop);
    set_ssa_reg(&mut ins, tc_safe, r_otok, 0x7F0E, TY_I64,  vec![0x7EC0, r_cur]);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_otok, TOK_IMM_I16, TOK_INS_END]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_iset_bdone);
    emit_decode_operand(&mut ins, &mut next_lbl, false);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x42, 0x88, 0x84, 0x1D, 0x00, 0x00, 0xFE, 0xFF, 0x49, 0xFF, 0xC3]);
    emit_jmp_ins(&mut ins, lbl_iset_bloop);

    emit_lbl_ins(&mut ins, lbl_iset_m4);
    set_ssa_reg(&mut ins, tc_safe, r_cur, 0x7F10, TY_I64, vec![r_cur, TOK_IMM_I16, 2]);
    emit_decode_operand(&mut ins, &mut next_lbl, false);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x42, 0x89, 0x84, 0x1D, 0x00, 0x00, 0xFE, 0xFF, 0x49, 0x83, 0xC3, 0x04]);
    emit_jmp_ins(&mut ins, lbl_iset_bdone);

    emit_lbl_ins(&mut ins, lbl_iset_m8);
    set_ssa_reg(&mut ins, tc_safe, r_cur, 0x7F10, TY_I64, vec![r_cur, TOK_IMM_I16, 2]);
    emit_decode_operand(&mut ins, &mut next_lbl, false);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x4A, 0x89, 0x84, 0x1D, 0x00, 0x00, 0xFE, 0xFF, 0x49, 0x83, 0xC3, 0x08]);
    emit_jmp_ins(&mut ins, lbl_iset_bdone);

    emit_lbl_ins(&mut ins, lbl_iset_m9);
    set_ssa_reg(&mut ins, tc_safe, r_cur, 0x7F10, TY_I64, vec![r_cur, TOK_IMM_I16, 2]);
    emit_decode_operand(&mut ins, &mut next_lbl, true);
    emit_decode_operand(&mut ins, &mut next_lbl, false);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x48, 0x89, 0x84, 0x0D, 0x00, 0x00, 0xFE, 0xFF]);

    emit_lbl_ins(&mut ins, lbl_iset_bdone);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x4C, 0x89, 0xD8]);
    emit_jmp_ins(&mut ins, lbl_store_dst);

    emit_lbl_ins(&mut ins, lbl_nop);
    set_ssa_reg(&mut ins, tc_safe, r_pos, 0x7F10, TY_I64, vec![r_pos, TOK_IMM_I16, 2]);
    emit_jmp_ins(&mut ins, lbl_ins_loop);

    emit_lbl_ins(&mut ins, lbl_label);
    set_ssa_reg(&mut ins, tc_safe, r_cur,  0x7F10, TY_I64, vec![r_pos, TOK_IMM_I16, 1]);
    emit_read_label_id(&mut ins);
    set_ssa_reg(&mut ins, tc_safe, r_loff, 0x7F50, TY_I64, vec![r_buf]);
    set_ssa_reg(&mut ins, tc_safe, r_loff, 0x7F11, TY_I64, vec![r_loff, TOK_IMM_I16, 120]);
    for k in 0..MAX_LABELS {
        set_ssa_reg(&mut ins, tc_safe, r_cond,   0x7F24, TY_TRIT, vec![r_lid, TOK_IMM_I16, k as u16]);
        set_ssa_reg(&mut ins, tc_safe, r_lbl[k], 0x7F45, TY_I64,  vec![r_cond, r_lbl[k], r_lbl[k], r_loff]);
    }
    set_ssa_reg(&mut ins, tc_safe, r_pos, 0x7F10, TY_I64, vec![r_cur, TOK_IMM_I16, 1]);
    emit_jmp_ins(&mut ins, lbl_ins_loop);

    emit_lbl_ins(&mut ins, lbl_jmp);
    set_ssa_reg(&mut ins, tc_safe, r_cur, 0x7F10, TY_I64, vec![r_pos, TOK_IMM_I16, 1]);
    emit_read_label_id(&mut ins);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0xE9]);
    emit_rel32_for_lid(&mut ins);
    set_ssa_reg(&mut ins, tc_safe, r_pos, 0x7F10, TY_I64, vec![r_cur, TOK_IMM_I16, 1]);
    emit_jmp_ins(&mut ins, lbl_ins_loop);

    emit_lbl_ins(&mut ins, lbl_br_trit);
    set_ssa_reg(&mut ins, tc_safe, r_cur, 0x7F10, TY_I64, vec![r_pos, TOK_IMM_I16, 1]);
    emit_decode_operand(&mut ins, &mut next_lbl, false);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x48, 0x83, 0xF8, 0x00, 0x0F, 0x8C]);
    emit_read_label_id(&mut ins);
    emit_rel32_for_lid(&mut ins);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x0F, 0x84]);
    emit_read_label_id(&mut ins);
    emit_rel32_for_lid(&mut ins);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0xE9]);
    emit_read_label_id(&mut ins);
    emit_rel32_for_lid(&mut ins);
    set_ssa_reg(&mut ins, tc_safe, r_pos, 0x7F10, TY_I64, vec![r_cur, TOK_IMM_I16, 1]);
    emit_jmp_ins(&mut ins, lbl_ins_loop);

    emit_lbl_ins(&mut ins, lbl_br_pos);
    set_ssa_reg(&mut ins, tc_safe, r_cur, 0x7F10, TY_I64, vec![r_pos, TOK_IMM_I16, 1]);
    emit_decode_operand(&mut ins, &mut next_lbl, false);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x48, 0x83, 0xF8, 0x01, 0x0F, 0x84]);
    emit_read_label_id(&mut ins);
    emit_rel32_for_lid(&mut ins);
    set_ssa_reg(&mut ins, tc_safe, r_pos, 0x7F10, TY_I64, vec![r_cur, TOK_IMM_I16, 1]);
    emit_jmp_ins(&mut ins, lbl_ins_loop);

    emit_lbl_ins(&mut ins, lbl_br_neg);
    set_ssa_reg(&mut ins, tc_safe, r_cur, 0x7F10, TY_I64, vec![r_pos, TOK_IMM_I16, 1]);
    emit_decode_operand(&mut ins, &mut next_lbl, false);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x48, 0x83, 0xF8, 0xFF, 0x0F, 0x84]);
    emit_read_label_id(&mut ins);
    emit_rel32_for_lid(&mut ins);
    set_ssa_reg(&mut ins, tc_safe, r_pos, 0x7F10, TY_I64, vec![r_cur, TOK_IMM_I16, 1]);
    emit_jmp_ins(&mut ins, lbl_ins_loop);

    emit_lbl_ins(&mut ins, lbl_store_dst);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[0x48, 0x89, 0x85]);
    set_ssa_reg(&mut ins, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, (-4i16) as u16, r_disp_dst]);
    set_ssa_reg(&mut ins, tc_safe, r_pos, 0x7F10, TY_I64,   vec![r_cur, TOK_IMM_I16, 1]);
    emit_jmp_ins(&mut ins, lbl_ins_loop);

    emit_lbl_ins(&mut ins, lbl_ret);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_retty, TOK_IMM_I16, TY_BYTES]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_ret_bytes);
    set_ssa_reg(&mut ins, tc_safe, r_cur, 0x7F10, TY_I64, vec![r_pos, TOK_IMM_I16, 1]);
    emit_decode_operand(&mut ins, &mut next_lbl, false);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[
        0x48, 0x83, 0xEC, 0x08, 0x48, 0x89, 0x04, 0x24,
        0xB8, 0x01, 0x00, 0x00, 0x00, 0xBF, 0x01, 0x00, 0x00, 0x00,
        0x48, 0x89, 0xE6, 0xBA, 0x08, 0x00, 0x00, 0x00, 0x0F, 0x05,
        0x48, 0x83, 0xC4, 0x08, 0x31, 0xFF, 0xB8, 0x3C, 0x00, 0x00, 0x00, 0x0F, 0x05,
    ]);
    set_ssa_reg(&mut ins, tc_safe, r_pos, 0x7F10, TY_I64, vec![r_cur, TOK_IMM_I16, 1]);
    emit_jmp_ins(&mut ins, lbl_ins_loop);

    emit_lbl_ins(&mut ins, lbl_ret_bytes);
    emit_iset_bytes_in_place(&mut ins, tc_safe, r_buf, &[
        0xBF, 0x01, 0x00, 0x00, 0x00,
        0x48, 0x8D, 0xB5, 0x00, 0x00, 0xFE, 0xFF,
        0x4C, 0x89, 0xDA,
        0xB8, 0x01, 0x00, 0x00, 0x00,
        0x0F, 0x05,
        0x31, 0xFF,
        0xB8, 0x3C, 0x00, 0x00, 0x00,
        0x0F, 0x05,
    ]);
    set_ssa_reg(&mut ins, tc_safe, r_pos, 0x7F10, TY_I64, vec![r_pos, TOK_IMM_I16, 3]);
    emit_jmp_ins(&mut ins, lbl_ins_loop);

    emit_lbl_ins(&mut ins, lbl_loop_end);
    set_ssa_reg(&mut ins, tc_safe, r_cond, 0x7F24, TY_TRIT, vec![r_pass, TOK_IMM_I16, 0]);
    emit_br_pos_ins(&mut ins, r_cond, lbl_pass0_end);
    set_ssa_reg(&mut ins, tc_safe, r_tmp, 0x7F50, TY_I64,   vec![r_buf]);
    set_ssa_reg(&mut ins, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, (-9i16) as u16, TOK_IMM_I16, 96,  r_tmp]);
    set_ssa_reg(&mut ins, tc_safe, r_buf, 0x7F0F, TY_BYTES, vec![r_buf, TOK_IMM_I16, (-9i16) as u16, TOK_IMM_I16, 104, r_tmp]);
    ins.push(vec![0x7F4A, r_buf, TOK_INS_END]);

    emit_lbl_ins(&mut ins, lbl_pass0_end);
    set_ssa_reg(&mut ins, tc_safe, r_pass, 0x7F06, TY_I64, vec![TOK_IMM_I16, 1]);
    emit_jmp_ins(&mut ins, lbl_pass_entry);

    pack_fn_module(
        &[],
        0x0000,
        &[
            (0x7EC0, TY_ARRAY, tc_safe),
            (0x7EC1, TY_I64,   tc_safe),
            (0x7EC2, TY_I64,   tc_safe),
            (0x7EC3, TY_I64,   tc_safe),
        ],
        TY_BYTES,
        0x0000,
        &ins,
    )
}

pub fn fnv1a_64_hash(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub fn assert_elf_bit_exact(native_elf: &[u8], vm_elf: &[u8], label: &str) {
    if native_elf != vm_elf {
        let min_len = native_elf.len().min(vm_elf.len());
        let first_diff = (0..min_len).find(|&i| native_elf[i] != vm_elf[i]).unwrap_or(min_len);
        let s_start = first_diff.saturating_sub(8);
        let s_end = (first_diff + 16).min(min_len);
        panic!(
            "Step 7 ELF mismatch on '{}': native_len={} vs vm_len={}, first diff at byte offset {} (0x{:04X}):\n  native[{}..{}]: {:02X?}\n  vm    [{}..{}]: {:02X?}",
            label,
            native_elf.len(),
            vm_elf.len(),
            first_diff,
            first_diff,
            s_start, s_end, &native_elf[s_start..s_end],
            s_start, s_end, &vm_elf[s_start..s_end],
        );
    }
}

