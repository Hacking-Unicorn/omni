//! Omni Static Verifier — full monolith-equivalent O(n) ownership, taint, capability,
//! SSA ordering, linear arena leak, and branch-label target verifier.

use std::time::Instant;
use omni_isa::*;
use omni_ir::*;

// 4. O(n) BINARY TOKEN-STREAM VERIFIER & COMPLETE EXECUTING VM
// ==============================================================================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RegState {
    Uninit,
    Owned,
    BorrowedR(u8),
    BorrowedW,
    Moved,
    Dropped,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Taint {
    Safe,
    Untrusted,
}

#[derive(Clone, Copy, Debug)]
pub struct RegSlot {
    pub state: RegState,
    pub taint: Taint,
    pub is_heap_arena: bool,
    pub parent_slot: Option<u8>,
}

impl Default for RegSlot {
    fn default() -> Self {
        Self {
            state: RegState::Uninit,
            taint: Taint::Safe,
            is_heap_arena: false,
            parent_slot: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BinVerificationReport {
    pub passed: bool,
    pub fn_idx: u16,
    pub instruction_count: usize,
    pub errors: Vec<String>,
    pub elapsed_ns: u128,
    pub rl_reward: f32,
}

#[inline(always)]
pub fn token_to_slot(tok: u16) -> Option<usize> {
    if (REG_BASE..=REG_MAX).contains(&tok) {
        Some((tok - REG_BASE) as usize)
    } else if (PAR_BASE..=PAR_MAX).contains(&tok) {
        Some(192 + (tok - PAR_BASE) as usize)
    } else {
        None
    }
}

pub fn collect_operand_slots(op: &BinOperand, out: &mut Vec<u16>) {
    match op {
        BinOperand::Reg(t) | BinOperand::Param(t) => out.push(*t),
        BinOperand::FieldKV(_, inner) => collect_operand_slots(inner, out),
        _ => {}
    }
}

pub fn verify_binary_function(func: &BinFunction) -> BinVerificationReport {
    let t0 = Instant::now();
    let mut errors = Vec::new();
    let mut regs = [RegSlot::default(); 224];
    let mut next_ssa_tok = REG_BASE;
    let mut defined_labels: Vec<u16> = Vec::new();
    for ins in &func.instructions {
        if ins.op_tok == 0x7F40 {
            if let Some(lid) = ins.operands.first().and_then(extract_label_id) {
                defined_labels.push(lid);
            }
        }
    }

    for &(par_tok, ty_tok, tc_tok) in &func.params {
        if let Some(slot) = token_to_slot(par_tok) {
            let (untrusted, _, _) = unpack_tc(tc_tok);
            regs[slot] = RegSlot {
                state: RegState::Owned,
                taint: if untrusted { Taint::Untrusted } else { Taint::Safe },
                is_heap_arena: ty_tok == TY_ARENA || ty_tok == TY_MODEL_T,
                parent_slot: None,
            };
        }
    }

    for ins in &func.instructions {
        let spec = match decode_opcode_token(ins.op_tok) {
            Some(s) => s,
            None => {
                errors.push(format!("Offset {}: Unknown opcode token 0x{:04X}", ins.offset, ins.op_tok));
                continue;
            }
        };

        if ins.op_tok == 0x7F05 {
            if let Some(BinOperand::Reg(r_tok)) = ins.operands.first() {
                if let Some(slot) = token_to_slot(*r_tok) {
                    match regs[slot].state {
                        RegState::Moved | RegState::Dropped | RegState::Uninit => {
                            errors.push(format!("Offset {}: Invalid DROP on 0x{:04X}", ins.offset, r_tok));
                        }
                        RegState::BorrowedR(k) if k > 0 => {
                            errors.push(format!("Offset {}: Cannot DROP 0x{:04X} with {} active readers", ins.offset, r_tok, k));
                        }
                        _ => {
                            if let Some(p) = regs[slot].parent_slot {
                                regs[p as usize].state = match regs[p as usize].state {
                                    RegState::BorrowedR(k) if k > 1 => RegState::BorrowedR(k - 1),
                                    _ => RegState::Owned,
                                };
                            }
                            regs[slot].state = RegState::Dropped;
                        }
                    }
                }
            }
            continue;
        }

        if ins.op_tok == 0x7F4A {
            if let Some(op0) = ins.operands.first() {
                let mut ref_toks = Vec::new();
                collect_operand_slots(op0, &mut ref_toks);
                for r_tok in ref_toks {
                    if let Some(slot) = token_to_slot(r_tok) {
                        if regs[slot].taint == Taint::Untrusted {
                            errors.push(format!("Offset {}: Taint violation on RET 0x{:04X}", ins.offset, r_tok));
                        }
                        regs[slot].state = RegState::Moved;
                    }
                }
            }
            for idx in 0..192usize {
                if regs[idx].is_heap_arena && !matches!(regs[idx].state, RegState::Moved | RegState::Dropped | RegState::Uninit) {
                    errors.push(format!("Offset {}: Linear memory leak on 0x{:04X}", ins.offset, REG_BASE + (idx as u16)));
                }
            }
            continue;
        }

        if matches!(ins.op_tok, 0x7F41 | 0x7F42 | 0x7F43 | 0x7F44) {
            let lbl_ops = if ins.op_tok == 0x7F41 {
                &ins.operands[..]
            } else if ins.operands.len() > 1 {
                &ins.operands[1..]
            } else {
                &[]
            };
            if lbl_ops.is_empty() {
                errors.push(format!("Offset {}: Missing branch target label on 0x{:04X}", ins.offset, ins.op_tok));
            }
            for lop in lbl_ops {
                match extract_label_id(lop) {
                    Some(lid) if defined_labels.contains(&lid) => {}
                    other => {
                        errors.push(format!("Offset {}: Unresolved branch target label {:?}", ins.offset, other));
                    }
                }
            }
        }

        if !spec.has_dest_reg {
            continue;
        }

        if let Some(d_tok) = ins.dest_reg {
            if d_tok > next_ssa_tok {
                errors.push(format!(
                    "Offset {}: Non-canonical SSA token 0x{:04X}, expected <= 0x{:04X}",
                    ins.offset, d_tok, next_ssa_tok
                ));
                next_ssa_tok = d_tok + 1;
            } else if d_tok == next_ssa_tok {
                next_ssa_tok += 1;
            } else if let Some(d_slot) = token_to_slot(d_tok) {
                if regs[d_slot].is_heap_arena && !matches!(regs[d_slot].state, RegState::Dropped | RegState::Moved) {
                    errors.push(format!("Offset {}: Cannot overwrite active heap arena 0x{:04X}", ins.offset, d_tok));
                } else if !matches!(regs[d_slot].state, RegState::Owned) {
                    errors.push(format!("Offset {}: Cannot reassign register 0x{:04X} in state {:?}", ins.offset, d_tok, regs[d_slot].state));
                }
            }

            let (ins_untrusted, _, ins_cap_mask) = unpack_tc(ins.tc_tok);
            if (ins_cap_mask & !func.cap_mask) != 0 || (spec.req_cap_mask & !func.cap_mask) != 0 {
                errors.push(format!("Offset {}: Capability violation on opcode 0x{:04X}", ins.offset, ins.op_tok));
            }

            let mut any_untrusted = false;
            let mut parent_slot: Option<u8> = None;
            let mut op_toks = Vec::new();
            for op in &ins.operands {
                if matches!(op, BinOperand::EnglishSpan(_)) {
                    any_untrusted = true;
                }
                collect_operand_slots(op, &mut op_toks);
            }

            for r_tok in op_toks {
                if let Some(slot) = token_to_slot(r_tok) {
                    let info = regs[slot];
                    if matches!(info.state, RegState::Moved | RegState::Dropped | RegState::Uninit | RegState::BorrowedW) {
                        errors.push(format!("Offset {}: Invalid state {:?} on token 0x{:04X}", ins.offset, info.state, r_tok));
                    }
                    if info.taint == Taint::Untrusted {
                        any_untrusted = true;
                    }
                    if ins.op_tok == 0x7F03 && info.state == RegState::Owned {
                        regs[slot].state = RegState::BorrowedW;
                        parent_slot = Some(slot as u8);
                    } else if ins.op_tok == 0x7F02 && matches!(info.state, RegState::Owned | RegState::BorrowedR(_)) {
                        let k = match info.state { RegState::BorrowedR(c) => c, _ => 0 };
                        regs[slot].state = RegState::BorrowedR(k + 1);
                        parent_slot = Some(slot as u8);
                    } else if ins.op_tok == 0x7F04 && info.state == RegState::Owned {
                        regs[slot].state = RegState::Moved;
                    }
                }
            }

            if spec.sink_requires_safe && any_untrusted {
                errors.push(format!("Offset {}: Sink opcode 0x{:04X} received @UNTRUSTED operand", ins.offset, ins.op_tok));
            }
            let out_taint = if ins_untrusted { Taint::Untrusted } else { Taint::Safe };
            if !spec.is_sanitizer && any_untrusted && out_taint == Taint::Safe {
                errors.push(format!("Offset {}: Illegal taint upgrade to @SAFE in opcode 0x{:04X}", ins.offset, ins.op_tok));
            }

            if let Some(d_slot) = token_to_slot(d_tok) {
                regs[d_slot] = RegSlot {
                    state: RegState::Owned,
                    taint: out_taint,
                    is_heap_arena: matches!(ins.op_tok, 0x7F01 | 0x7F7E),
                    parent_slot,
                };
            }
        }
    }

    let elapsed_ns = t0.elapsed().as_nanos();
    let passed = errors.is_empty();
    let rl_reward = if passed {
        1.0
    } else {
        (0.40 - 0.30 * (errors.len() as f32)).max(-1.0)
    };

    BinVerificationReport {
        passed,
        fn_idx: func.fn_idx,
        instruction_count: func.instructions.len(),
        errors,
        elapsed_ns,
        rl_reward,
    }
}


/// Pretty-prints a pure `u16` stream using the `OMNI_ISA_TABLE` opcode labels for human debugging (`--debug-lens`, Roadmap 3.3).
pub fn format_debug_lens(stream: &[u16]) -> Result<String, String> {
    let module = decode_omni_token_stream(stream)?;
    let mut out = String::new();
    out.push_str(&format!(
        "=== OMNI DEBUG LENS ({} u16 tokens, {} functions, {} schemas) ===\n",
        stream.len(),
        module.functions.len(),
        module.schemas.len()
    ));
    for func in &module.functions {
        let rep = verify_binary_function(func);
        out.push_str(&format!(
            "FN #0x{:04X} (params={}, ret_ty=0x{:04X}, cap_mask=0x{:04X}, ins={}, verified={}):\n",
            func.fn_idx,
            func.params.len(),
            func.ret_ty_tok,
            func.cap_mask,
            func.instructions.len(),
            rep.passed
        ));
        for (idx, ins) in func.instructions.iter().enumerate() {
            let op_name = decode_opcode_token(ins.op_tok)
                .map(|s| s.debug_label)
                .unwrap_or("OP_UNKNOWN");
            let dst_str = ins
                .dest_reg
                .map(|d| format!("%{} (0x{:04X}) = ", d - REG_BASE, d))
                .unwrap_or_default();
            out.push_str(&format!(
                "  [{:03} @0x{:04X}] {}{} (0x{:04X}) {:?}\n",
                idx, ins.offset, dst_str, op_name, ins.op_tok, ins.operands
            ));
        }
    }
    Ok(out)
}

