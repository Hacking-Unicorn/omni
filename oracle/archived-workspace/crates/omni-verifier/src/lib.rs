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
                            errors.push(format!("Offset {}: error[BORROW_CONFLICT]: Invalid DROP on 0x{:04X}", ins.offset, r_tok));
                        }
                        RegState::BorrowedR(k) if k > 0 => {
                            errors.push(format!("Offset {}: error[BORROW_CONFLICT]: Cannot DROP 0x{:04X} with {} active readers", ins.offset, r_tok, k));
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
            for (oi, op) in ins.operands.iter().enumerate() {
                let mut ref_toks = Vec::new();
                collect_operand_slots(op, &mut ref_toks);
                for r_tok in ref_toks {
                    if let Some(slot) = token_to_slot(r_tok) {
                        if regs[slot].taint == Taint::Untrusted {
                            errors.push(format!("Offset {}: error[TAINT_ON_RET]: Taint violation on RET 0x{:04X}", ins.offset, r_tok));
                        }
                        if oi == 0 {
                            regs[slot].state = RegState::Moved;
                        }
                    }
                }
            }
            for idx in 0..192usize {
                if regs[idx].is_heap_arena && !matches!(regs[idx].state, RegState::Moved | RegState::Dropped | RegState::Uninit) {
                    errors.push(format!("Offset {}: error[LINEAR_LEAK]: Linear memory leak on 0x{:04X}", ins.offset, REG_BASE + (idx as u16)));
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
                errors.push(format!("Offset {}: error[UNRESOLVED_LABEL]: Missing branch target label on 0x{:04X}", ins.offset, ins.op_tok));
            }
            for lop in lbl_ops {
                match extract_label_id(lop) {
                    Some(lid) if defined_labels.contains(&lid) => {}
                    other => {
                        errors.push(format!("Offset {}: error[UNRESOLVED_LABEL]: Unresolved branch target label {:?}", ins.offset, other));
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
                errors.push(format!("Offset {}: error[MISSING_CAP]: Capability violation on opcode 0x{:04X}", ins.offset, ins.op_tok));
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
                        errors.push(format!("Offset {}: error[BORROW_CONFLICT]: Invalid state {:?} on token 0x{:04X}", ins.offset, info.state, r_tok));
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
                errors.push(format!("Offset {}: error[TAINT_SINK]: Sink opcode 0x{:04X} received @UNTRUSTED operand", ins.offset, ins.op_tok));
            }
            let out_taint = if ins_untrusted { Taint::Untrusted } else { Taint::Safe };
            if !spec.is_sanitizer && any_untrusted && out_taint == Taint::Safe {
                errors.push(format!("Offset {}: error[TAINT_SINK]: Illegal taint upgrade to @SAFE in opcode 0x{:04X}", ins.offset, ins.op_tok));
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

/// Verifies a raw `&[u16]` binary token stream directly in a single ephemeral pass without retaining intermediate IR.
pub fn verify_u16_stream(stream: &[u16]) -> Result<Vec<BinVerificationReport>, String> {
    let module = decode_omni_token_stream(stream)?;
    Ok(module.functions.iter().map(verify_binary_function).collect())
}

/// M2-08: Module-level pre-check required before multi-function AOT emission.
pub fn verify_module_for_multi_aot(module: &BinModule) -> Result<(), String> {
    let n = module.functions.len();
    if n == 0 {
        return Err(String::from("error[MULTI_AOT_EMPTY]: module contains no functions"));
    }
    if n > 1024 {
        return Err(format!("error[MULTI_AOT_FN_CAP]: module has {} functions (max 1024)", n));
    }

    let mut fn_map: std::collections::HashMap<u16, usize> = std::collections::HashMap::with_capacity(n);
    for (pos, func) in module.functions.iter().enumerate() {
        if fn_map.insert(func.fn_idx, pos).is_some() {
            return Err(format!("error[MULTI_AOT_DUP_FN]: duplicate function id fn#{}", func.fn_idx));
        }
    }

    let entry_pos = module.functions.iter().position(|f| f.fn_idx == 0).unwrap_or(n - 1);
    let entry_fn = &module.functions[entry_pos];
    if entry_fn.params.len() > 4 {
        return Err(format!(
            "error[MULTI_AOT_ENTRY_PARAMS]: entry fn#{} has {} params (max 4)",
            entry_fn.fn_idx,
            entry_fn.params.len()
        ));
    }
    for func in &module.functions {
        if func.params.len() > 32 {
            return Err(format!(
                "error[MULTI_AOT_CALLEE_PARAMS]: fn#{} has {} params (max 32)",
                func.fn_idx,
                func.params.len()
            ));
        }
    }

    let mut edges: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut in_deg: Vec<usize> = vec![0usize; n];
    let mut total_label_slots: usize = 0;

    for (caller_pos, func) in module.functions.iter().enumerate() {
        let mut max_imm_key: Option<u16> = None;
        for ins in &func.instructions {
            if ins.op_tok == 0x7F48 {
                let callee_idx = match ins.operands.first() {
                    Some(BinOperand::FnRef(idx)) => *idx,
                    _ => {
                        return Err(format!(
                            "error[MULTI_AOT_BAD_CALL]: fn#{} @{} CALL operand 0 is not a FnRef",
                            func.fn_idx, ins.offset
                        ));
                    }
                };
                let callee_pos = match fn_map.get(&callee_idx) {
                    Some(&p) => p,
                    None => {
                        return Err(format!(
                            "error[MULTI_AOT_MISSING_FN]: fn#{} @{} CALL targets missing fn#{}",
                            func.fn_idx, ins.offset, callee_idx
                        ));
                    }
                };
                let arg_cnt = ins.operands.len() - 1;
                let expected = module.functions[callee_pos].params.len();
                if arg_cnt != expected {
                    return Err(format!(
                        "error[MULTI_AOT_ARITY]: fn#{} @{} CALL fn#{} passed {} args, expected {}",
                        func.fn_idx, ins.offset, callee_idx, arg_cnt, expected
                    ));
                }
                edges[caller_pos].push(callee_pos);
                in_deg[callee_pos] += 1;
            } else if matches!(ins.op_tok, 0x7F40 | 0x7F41 | 0x7F42 | 0x7F43 | 0x7F44) {
                let ops: &[BinOperand] = if ins.op_tok == 0x7F40 || ins.op_tok == 0x7F41 {
                    &ins.operands[..]
                } else if ins.operands.len() > 1 {
                    &ins.operands[1..]
                } else {
                    &[]
                };
                for op in ops {
                    if let Some(key) = extract_label_id(op) {
                        if !(0x7EF0..=0x7EFF).contains(&key) {
                            if key < 4080 {
                                max_imm_key = Some(max_imm_key.map_or(key, |m| m.max(key)));
                            } else {
                                return Err(format!(
                                    "error[LABEL_ID_RANGE]: fn#{} @{} label key 0x{:04X} outside M2 slot domain",
                                    func.fn_idx, ins.offset, key
                                ));
                            }
                        }
                    }
                }
            }
        }
        let span = match max_imm_key {
            Some(m) => 16 + (m as usize) + 1,
            None => 16,
        };
        total_label_slots += span;
        if total_label_slots > 4096 {
            return Err(format!(
                "error[MULTI_AOT_LABEL_BUDGET]: module requires {} label slots (max 4096)",
                total_label_slots
            ));
        }
    }

    let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    for i in 0..n {
        if in_deg[i] == 0 {
            queue.push_back(i);
        }
    }
    let mut topo: Vec<usize> = Vec::with_capacity(n);
    while let Some(u) = queue.pop_front() {
        topo.push(u);
        for &v in &edges[u] {
            in_deg[v] -= 1;
            if in_deg[v] == 0 {
                queue.push_back(v);
            }
        }
    }
    if topo.len() != n {
        return Err(String::from(
            "error[MULTI_AOT_RECURSION]: call graph contains a cycle (recursion is forbidden)"
        ));
    }

    let mut max_chain: Vec<usize> = vec![1usize; n];
    for &u in topo.iter().rev() {
        let mut best = 1usize;
        for &v in &edges[u] {
            let cand = 1 + max_chain[v];
            if cand > best {
                best = cand;
            }
        }
        max_chain[u] = best;
    }
    if max_chain[entry_pos] > 64 {
        return Err(format!(
            "error[MULTI_AOT_DEPTH]: max call chain from entry fn#{} is {} frames (max 64)",
            entry_fn.fn_idx, max_chain[entry_pos]
        ));
    }
    Ok(())
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



#[cfg(test)]
mod cap_bit_overflow_tests {
    use super::*;

    /// All defined caps (ids 1..=10, bits 0..=9). No function holds bit 15.
    const ALL_DEFINED_CAPS: CapMask = 0x03FF;

    fn cint_with_cap(cap_id: u16, fn_cap_mask: CapMask) -> BinFunction {
        BinFunction {
            fn_idx: 0,
            params: vec![],
            ret_ty_tok: TY_I64,
            cap_mask: fn_cap_mask,
            instructions: vec![BinInstruction {
                offset: 0,
                dest_reg: Some(REG_BASE),
                op_tok: 0x7F06, // CINT: has dest, requires no cap of its own
                ty_tok: TY_I64,
                tc_tok: pack_tc(false, cap_id),
                operands: vec![BinOperand::ImmInt(0)],
            }],
        }
    }

    fn report_for(cap_id: u16, fn_cap_mask: CapMask) -> BinVerificationReport {
        verify_binary_function(&cint_with_cap(cap_id, fn_cap_mask))
    }

    fn assert_missing_cap(cap_id: u16, fn_cap_mask: CapMask) {
        let rep = report_for(cap_id, fn_cap_mask);
        assert!(
            !rep.passed,
            "cap_id {cap_id} on fn mask 0x{fn_cap_mask:04X} should fail, errors={:?}",
            rep.errors
        );
        assert!(
            rep.errors.iter().any(|e| e.contains("MISSING_CAP")),
            "cap_id {cap_id} on fn mask 0x{fn_cap_mask:04X} should report MISSING_CAP, errors={:?}",
            rep.errors
        );
    }

    #[test]
    fn defined_cap_token_accepted_when_function_holds_it() {
        let rep = report_for(CAP_ID_PURE, 0);
        assert!(rep.passed, "PURE CINT on a PURE fn should pass: {:?}", rep.errors);
        let rep = report_for(CAP_ID_DB_R, cap_bit(CAP_ID_DB_R));
        assert!(rep.passed, "DB_R CINT on a DB_R fn should pass: {:?}", rep.errors);
        let rep = report_for(CAP_ID_DB_R, 0);
        assert!(!rep.passed && rep.errors.iter().any(|e| e.contains("MISSING_CAP")));
    }

    #[test]
    fn undefined_cap_token_reports_missing_cap() {
        // Before: unpack_tc -> cap_bit(17) panics in debug; in release cap 17 aliases
        // DB_R (bit 0) so a function holding 0x03FF would silently pass.
        // After: ids 11..=31 are fail-closed MISSING_CAP even when the function
        // holds every defined cap, and the verifier does not panic.
        for cap_id in 11u16..=31 {
            assert_missing_cap(cap_id, 0);
            assert_missing_cap(cap_id, ALL_DEFINED_CAPS);
        }
        // Explicit pins from the overflow report.
        for cap_id in [17u16, 31] {
            assert_missing_cap(cap_id, ALL_DEFINED_CAPS);
        }
    }
}
