//! Omni VM — complete executing virtual machine with full control flow (`LABEL`, `JMP`,
//! `BR_TRIT`, `BR_POS`, `BR_NEG`), balanced ternary, wrapping 64-bit integer ALU,
//! byte/u16 buffers, ternary decision hooks (`EGATE`/`TLOGIC`), inter-function calls (`CALL`),
//! relational tables, and configurable fuel (`step_limit`) and memory (`memory_budget_bytes`) resource budgets.

use std::collections::{BTreeMap, HashMap};
use omni_isa::*;
use omni_ir::*;
use omni_verifier::{token_to_slot, verify_binary_function};

#[derive(Clone, Debug, PartialEq)]
pub enum OmniValue {
    Void,
    Int(i64),
    Trit(i8),
    Str(String),
    Bytes(Vec<u8>),
    U16Array(Vec<u16>),
    TritVec(Vec<i8>),
    Struct(BTreeMap<u16, OmniValue>),
    Arena { capacity: usize, active: bool },
    SkillModel { skill_id: u16, active: bool },
    DomNode { tag_id: u16, children: Vec<OmniValue> },
}

pub const DEFAULT_VM_STEP_LIMIT: u64 = 50_000_000;
pub const DEFAULT_VM_MEMORY_BUDGET_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_VM_CALL_DEPTH: usize = 64;

pub struct OmniBinaryVm {
    pub tables: HashMap<u16, Vec<BTreeMap<u16, OmniValue>>>,
    pub step_limit: u64,
    pub steps_executed: u64,
    pub memory_budget_bytes: usize,
    pub allocated_bytes: usize,
    pub call_depth: usize,
}

impl OmniBinaryVm {
    pub fn new() -> Self {
        Self {
            tables: HashMap::new(),
            step_limit: DEFAULT_VM_STEP_LIMIT,
            steps_executed: 0,
            memory_budget_bytes: DEFAULT_VM_MEMORY_BUDGET_BYTES,
            allocated_bytes: 0,
            call_depth: 0,
        }
    }

    pub fn with_budgets(step_limit: u64, memory_budget_bytes: usize) -> Self {
        Self {
            tables: HashMap::new(),
            step_limit,
            steps_executed: 0,
            memory_budget_bytes,
            allocated_bytes: 0,
            call_depth: 0,
        }
    }

    pub fn clear_tables(&mut self) {
        self.tables.clear();
        self.steps_executed = 0;
        self.allocated_bytes = 0;
        self.call_depth = 0;
    }

    #[inline]
    fn charge_memory(&mut self, bytes: usize) -> Result<(), String> {
        let next_alloc = self.allocated_bytes.saturating_add(bytes);
        if next_alloc > self.memory_budget_bytes {
            return Err(format!(
                "VM memory budget exceeded ({} > {} bytes)",
                next_alloc, self.memory_budget_bytes
            ));
        }
        self.allocated_bytes = next_alloc;
        Ok(())
    }

    fn eval_op(op: &BinOperand, regs: &[OmniValue; 224]) -> OmniValue {
        match op {
            BinOperand::Reg(t) | BinOperand::Param(t) => {
                token_to_slot(*t).map(|s| regs[s].clone()).unwrap_or(OmniValue::Void)
            }
            BinOperand::ImmInt(v) => OmniValue::Int(*v),
            BinOperand::ImmTrit(t) => OmniValue::Trit(*t),
            BinOperand::EnglishSpan(toks) => {
                let bytes: Vec<u8> = toks.iter().map(|&u| (u & 0xFF) as u8).collect();
                OmniValue::Str(String::from_utf8_lossy(&bytes).into_owned())
            }
            _ => OmniValue::Void,
        }
    }

    fn eval_as_i64(op: &BinOperand, regs: &[OmniValue; 224]) -> Option<i64> {
        match Self::eval_op(op, regs) {
            OmniValue::Int(v) => Some(v),
            OmniValue::Trit(t) => Some(t as i64),
            _ => None,
        }
    }

    fn eval_as_trit(op: &BinOperand, regs: &[OmniValue; 224]) -> Option<i8> {
        match Self::eval_op(op, regs) {
            OmniValue::Trit(t) => Some(t.clamp(-1, 1)),
            OmniValue::Int(v) => Some(v.clamp(-1, 1) as i8),
            _ => None,
        }
    }

    pub fn execute_function(
        &mut self,
        module: &BinModule,
        fn_idx: u16,
        args: Vec<OmniValue>,
    ) -> Result<OmniValue, String> {
        let func = module
            .functions
            .iter()
            .find(|f| f.fn_idx == fn_idx)
            .ok_or_else(|| format!("Function index {} not found", fn_idx))?;

        let rep = verify_binary_function(func);
        if !rep.passed {
            return Err(format!("Binary verification failed: {:?}", rep.errors));
        }

        let mut regs: [OmniValue; 224] = std::array::from_fn(|_| OmniValue::Void);
        for (i, &(par_tok, _, _)) in func.params.iter().enumerate() {
            if let (Some(slot), Some(val)) = (token_to_slot(par_tok), args.get(i)) {
                regs[slot] = val.clone();
            }
        }

        let mut label_pcs = [usize::MAX; 256];
        for (idx_i, cand) in func.instructions.iter().enumerate() {
            if cand.op_tok == 0x7F40 {
                if let Some(lid) = cand.operands.first().and_then(extract_label_id) {
                    if (lid as usize) < 256 {
                        label_pcs[lid as usize] = idx_i + 1;
                    }
                }
            }
        }

        let mut pc = 0usize;
        while pc < func.instructions.len() {
            self.steps_executed = self.steps_executed.saturating_add(1);
            if self.steps_executed > self.step_limit {
                return Err(format!("VM fuel/step budget exceeded ({} > {})", self.steps_executed, self.step_limit));
            }
            let ins = &func.instructions[pc];
            match ins.op_tok {
                0x7F00 | 0x7F40 | 0x7F46 => { pc += 1; continue; }
                0x7F41 => {
                    if let Some(target_lbl) = ins.operands.first().and_then(extract_label_id) {
                        let next_pc = if (target_lbl as usize) < 256 && label_pcs[target_lbl as usize] != usize::MAX {
                            Some(label_pcs[target_lbl as usize])
                        } else {
                            func.instructions.iter().position(|cand| {
                                cand.op_tok == 0x7F40 && cand.operands.first().and_then(extract_label_id) == Some(target_lbl)
                            }).map(|p| p + 1)
                        };
                        if let Some(tp) = next_pc {
                            pc = tp;
                            continue;
                        }
                    }
                    pc += 1;
                    continue;
                }
                0x7F42 => {
                    if let (Some(c), Some(l_neg), Some(l_zero), Some(l_pos)) = (
                        Self::eval_as_trit(&ins.operands[0], &regs),
                        ins.operands.get(1).and_then(extract_label_id),
                        ins.operands.get(2).and_then(extract_label_id),
                        ins.operands.get(3).and_then(extract_label_id),
                    ) {
                        let chosen = if c < 0 { l_neg } else if c == 0 { l_zero } else { l_pos };
                        let next_pc = if (chosen as usize) < 256 && label_pcs[chosen as usize] != usize::MAX {
                            Some(label_pcs[chosen as usize])
                        } else {
                            func.instructions.iter().position(|cand| {
                                cand.op_tok == 0x7F40 && cand.operands.first().and_then(extract_label_id) == Some(chosen)
                            }).map(|p| p + 1)
                        };
                        if let Some(tp) = next_pc {
                            pc = tp;
                            continue;
                        }
                    }
                    pc += 1;
                    continue;
                }
                0x7F43 | 0x7F44 => {
                    let want: i8 = if ins.op_tok == 0x7F43 { 1 } else { -1 };
                    if let (Some(c), Some(target_lbl)) = (
                        Self::eval_as_trit(&ins.operands[0], &regs),
                        ins.operands.get(1).and_then(extract_label_id),
                    ) {
                        if c == want {
                            let next_pc = if (target_lbl as usize) < 256 && label_pcs[target_lbl as usize] != usize::MAX {
                                Some(label_pcs[target_lbl as usize])
                            } else {
                                func.instructions.iter().position(|cand| {
                                    cand.op_tok == 0x7F40 && cand.operands.first().and_then(extract_label_id) == Some(target_lbl)
                                }).map(|p| p + 1)
                            };
                            if let Some(tp) = next_pc {
                                pc = tp;
                                continue;
                            }
                        }
                    }
                    pc += 1;
                    continue;
                }
                0x7F05 => {
                    if let Some(BinOperand::Reg(r_tok)) = ins.operands.first() {
                        if let Some(slot) = token_to_slot(*r_tok) {
                            if let OmniValue::Arena { capacity, active: true } = regs[slot] {
                                self.allocated_bytes = self.allocated_bytes.saturating_sub(capacity);
                            }
                            regs[slot] = OmniValue::Void;
                        }
                    }
                    pc += 1;
                    continue;
                }
                0x7F4A => return Ok(Self::eval_op(&ins.operands[0], &regs)),
                _ => {}
            }

            let out_val = match ins.op_tok {
                0x7F01 => {
                    if let OmniValue::Int(cap) = Self::eval_op(&ins.operands[0], &regs) {
                        let req = if cap > 0 { cap as usize } else { 0 };
                        self.charge_memory(req)?;
                        OmniValue::Arena { capacity: req, active: true }
                    } else { OmniValue::Void }
                }
                0x7F02 | 0x7F03 => match Self::eval_op(&ins.operands[0], &regs) {
                    OmniValue::Arena { .. } => OmniValue::Arena { capacity: 0, active: true },
                    other => other,
                },
                0x7F04 | 0x7F06 | 0x7F09 => Self::eval_op(&ins.operands[0], &regs),
                0x7F08 => {
                    let v = Self::eval_op(&ins.operands[0], &regs);
                    if let OmniValue::Str(ref s) = v {
                        self.charge_memory(s.len().max(1))?;
                    }
                    v
                }
                0x7F0D => {
                    self.charge_memory(64)?;
                    OmniValue::Bytes(Vec::with_capacity(64))
                }
                0x7F0E => {
                    let arr_val = Self::eval_op(&ins.operands[0], &regs);
                    let idx_val = Self::eval_op(&ins.operands[1], &regs);
                    match (arr_val, idx_val) {
                        (OmniValue::U16Array(arr), OmniValue::Int(i)) if i >= 0 && (i as usize) < arr.len() => {
                            OmniValue::Int(arr[i as usize] as i64)
                        }
                        (OmniValue::Bytes(buf), OmniValue::Int(i)) if i >= 0 && (i as usize) < buf.len() => {
                            OmniValue::Int(buf[i as usize] as i64)
                        }
                        (OmniValue::TritVec(tv), OmniValue::Int(i)) if i >= 0 && (i as usize) < tv.len() => {
                            let packed = pack5_encode(&tv);
                            OmniValue::Int(pack5_get_trit(&packed, i as usize).map_err(String::from)? as i64)
                        }
                        _ => OmniValue::Int(0),
                    }
                }
                0x7F0F => {
                    let mut buf = match Self::eval_op(&ins.operands[0], &regs) {
                        OmniValue::Bytes(b) => b,
                        _ => Vec::new(),
                    };
                    if ins.operands.len() == 3 && ins.operands[1] == BinOperand::ImmInt(-4) {
                        self.charge_memory(4)?;
                        if let Some(v) = Self::eval_as_i64(&ins.operands[2], &regs) {
                            buf.extend_from_slice(&(v as i32).to_le_bytes());
                        }
                    } else if ins.operands.len() == 3 && ins.operands[1] == BinOperand::ImmInt(-8) {
                        self.charge_memory(8)?;
                        if let Some(v) = Self::eval_as_i64(&ins.operands[2], &regs) {
                            buf.extend_from_slice(&v.to_le_bytes());
                        }
                    } else if ins.operands.len() == 4 && ins.operands[1] == BinOperand::ImmInt(-9) {
                        if let (Some(off), Some(v)) = (
                            Self::eval_as_i64(&ins.operands[2], &regs),
                            Self::eval_as_i64(&ins.operands[3], &regs),
                        ) {
                            let u_off = off as usize;
                            if u_off + 8 <= buf.len() {
                                buf[u_off..u_off + 8].copy_from_slice(&(v as u64).to_le_bytes());
                            }
                        }
                    } else {
                        self.charge_memory(ins.operands.len().saturating_sub(1))?;
                        for op_b in &ins.operands[1..] {
                            let b_val = Self::eval_as_i64(op_b, &regs).unwrap_or(0);
                            buf.push((b_val & 0xFF) as u8);
                        }
                    }
                    OmniValue::Bytes(buf)
                }
                0x7F10 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Int(a.wrapping_add(b)) } else { OmniValue::Int(0) },
                0x7F11 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Int(a.wrapping_sub(b)) } else { OmniValue::Int(0) },
                0x7F12 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Int(a.wrapping_mul(b)) } else { OmniValue::Int(0) },
                0x7F13 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Int(if b == 0 { 0 } else { a.wrapping_div(b) }) } else { OmniValue::Int(0) },
                0x7F14 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Int(if b == 0 { 0 } else { a.wrapping_rem(b) }) } else { OmniValue::Int(0) },
                0x7F15 => if let Some(a) = Self::eval_as_i64(&ins.operands[0], &regs) { OmniValue::Int(a.wrapping_neg()) } else { OmniValue::Int(0) },
                0x7F16 => if let Some(a) = Self::eval_as_i64(&ins.operands[0], &regs) { OmniValue::Int(a.wrapping_abs()) } else { OmniValue::Int(0) },
                0x7F17 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Int(a.min(b)) } else { OmniValue::Int(0) },
                0x7F18 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Int(a.max(b)) } else { OmniValue::Int(0) },
                0x7F1E => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Int(a & b) } else { OmniValue::Int(0) },
                0x7F1F => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Int(a | b) } else { OmniValue::Int(0) },
                0x7F20 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Int(a ^ b) } else { OmniValue::Int(0) },
                0x7F21 => if let Some(a) = Self::eval_as_i64(&ins.operands[0], &regs) { OmniValue::Int(!a) } else { OmniValue::Int(0) },
                0x7F22 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Int(a.wrapping_shl((b & 63) as u32)) } else { OmniValue::Int(0) },
                0x7F23 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Int(a.wrapping_shr((b & 63) as u32)) } else { OmniValue::Int(0) },
                0x7F50 => match Self::eval_op(&ins.operands[0], &regs) {
                    OmniValue::Bytes(b) => OmniValue::Int(b.len() as i64),
                    OmniValue::Str(s) => OmniValue::Int(s.len() as i64),
                    OmniValue::U16Array(a) => OmniValue::Int(a.len() as i64),
                    OmniValue::TritVec(t) => OmniValue::Int(t.len() as i64),
                    _ => OmniValue::Int(0),
                },
                0x7F51 => {
                    if let (OmniValue::Str(a), OmniValue::Str(b)) = (Self::eval_op(&ins.operands[0], &regs), Self::eval_op(&ins.operands[1], &regs)) {
                        self.charge_memory(a.len().saturating_add(b.len()).max(1))?;
                        OmniValue::Str(format!("{}{}", a, b))
                    } else { OmniValue::Str(String::new()) }
                }
                0x7F55 => {
                    if let OmniValue::Str(s) = Self::eval_op(&ins.operands[0], &regs) {
                        OmniValue::Int(s.trim().parse::<i64>().unwrap_or(0))
                    } else { OmniValue::Int(0) }
                }
                0x7F56 => {
                    if let Some(v) = Self::eval_as_i64(&ins.operands[0], &regs) {
                        let s = v.to_string();
                        self.charge_memory(s.len().max(1))?;
                        OmniValue::Str(s)
                    } else { OmniValue::Str(String::new()) }
                }
                0x7F24 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Trit(if a == b { 1 } else { -1 }) } else { OmniValue::Trit(-1) },
                0x7F25 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Trit(if a != b { 1 } else { -1 }) } else { OmniValue::Trit(-1) },
                0x7F26 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Trit(if a < b { 1 } else { -1 }) } else { OmniValue::Trit(-1) },
                0x7F27 => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Trit(if a > b { 1 } else { -1 }) } else { OmniValue::Trit(-1) },
                0x7F28 => if let Some(a) = Self::eval_as_trit(&ins.operands[0], &regs) { OmniValue::Trit(tnot(a)) } else { OmniValue::Trit(0) },
                0x7F29 => if let (Some(a), Some(b)) = (Self::eval_as_trit(&ins.operands[0], &regs), Self::eval_as_trit(&ins.operands[1], &regs)) { OmniValue::Trit(tand(a, b)) } else { OmniValue::Trit(0) },
                0x7F2A => if let (Some(a), Some(b)) = (Self::eval_as_trit(&ins.operands[0], &regs), Self::eval_as_trit(&ins.operands[1], &regs)) { OmniValue::Trit(tor(a, b)) } else { OmniValue::Trit(0) },
                0x7F2B => if let (Some(a), Some(b)) = (Self::eval_as_trit(&ins.operands[0], &regs), Self::eval_as_trit(&ins.operands[1], &regs)) { OmniValue::Trit(txor(a, b)) } else { OmniValue::Trit(0) },
                0x7F2C => if let (Some(a), Some(b), Some(c)) = (Self::eval_as_trit(&ins.operands[0], &regs), Self::eval_as_trit(&ins.operands[1], &regs), Self::eval_as_trit(&ins.operands[2], &regs)) { OmniValue::Trit(tmaj(a, b, c)) } else { OmniValue::Trit(0) },
                0x7F2D => if let (Some(a), Some(b)) = (Self::eval_as_i64(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) { OmniValue::Trit(if a < b { -1 } else if a > b { 1 } else { 0 }) } else { OmniValue::Trit(0) },
                0x7F30 => {
                    if let OmniValue::TritVec(tv) = Self::eval_op(&ins.operands[0], &regs) {
                        let packed = pack5_encode(&tv);
                        self.charge_memory(packed.len())?;
                        OmniValue::Bytes(packed)
                    } else { OmniValue::Bytes(Vec::new()) }
                }
                0x7F31 => {
                    if let (OmniValue::Bytes(b), Some(len)) = (Self::eval_op(&ins.operands[0], &regs), Self::eval_as_i64(&ins.operands[1], &regs)) {
                        let n = len.max(0) as usize;
                        self.charge_memory(pack5_byte_len(n))?;
                        OmniValue::TritVec(pack5_decode(&b, n).map_err(String::from)?)
                    } else { OmniValue::TritVec(Vec::new()) }
                }
                0x7F34 => {
                    match (Self::eval_op(&ins.operands[0], &regs), Self::eval_op(&ins.operands[1], &regs)) {
                        (OmniValue::Bytes(pa), OmniValue::Bytes(pb)) => {
                            let n_trits = ins
                                .operands
                                .get(2)
                                .and_then(|op| Self::eval_as_i64(op, &regs))
                                .map(|n| n.max(0) as usize)
                                .unwrap_or(pa.len().min(pb.len()) * 5);
                            OmniValue::Int(pack5_tdot_direct(&pa, &pb, n_trits).map_err(String::from)?)
                        }
                        (OmniValue::TritVec(a), OmniValue::TritVec(b)) => {
                            let pa = pack5_encode(&a);
                            let pb = pack5_encode(&b);
                            let n_trits = a.len().min(b.len());
                            OmniValue::Int(pack5_tdot_direct(&pa, &pb, n_trits).map_err(String::from)?)
                        }
                        (OmniValue::Bytes(pa), OmniValue::TritVec(b)) => {
                            let pb = pack5_encode(&b);
                            let n_trits = (pa.len() * 5).min(b.len());
                            OmniValue::Int(pack5_tdot_direct(&pa, &pb, n_trits).map_err(String::from)?)
                        }
                        (OmniValue::TritVec(a), OmniValue::Bytes(pb)) => {
                            let pa = pack5_encode(&a);
                            let n_trits = a.len().min(pb.len() * 5);
                            OmniValue::Int(pack5_tdot_direct(&pa, &pb, n_trits).map_err(String::from)?)
                        }
                        _ => OmniValue::Int(0),
                    }
                }
                0x7F35 | 0x7F3F => {
                    let sum_i64: i64 = match Self::eval_op(&ins.operands[0], &regs) {
                        OmniValue::TritVec(tv) => {
                            let packed = pack5_encode(&tv);
                            pack5_sum_direct(&packed, tv.len()).map_err(String::from)?
                        }
                        OmniValue::Bytes(b) => {
                            let n_trits = ins
                                .operands
                                .get(1)
                                .and_then(|op| Self::eval_as_i64(op, &regs))
                                .map(|n| n.max(0) as usize)
                                .unwrap_or(b.len() * 5);
                            pack5_sum_direct(&b, n_trits).map_err(String::from)?
                        }
                        OmniValue::Int(v) => v,
                        OmniValue::Trit(t) => t as i64,
                        _ => 0,
                    };
                    OmniValue::Trit(if sum_i64 > 0 { 1 } else if sum_i64 < 0 { -1 } else { 0 })
                }
                0x7F36 => OmniValue::Trit(-1),
                0x7F37 => OmniValue::Struct(BTreeMap::new()),
                0x7F39 => {
                    let mut m = BTreeMap::new();
                    if let Some(t) = Self::eval_as_trit(&ins.operands[2], &regs) {
                        m.insert(0, OmniValue::Int(t as i64));
                    }
                    m.insert(1, OmniValue::Int(64));
                    OmniValue::Struct(m)
                }
                0x7F45 => {
                    if let Some(c) = Self::eval_as_trit(&ins.operands[0], &regs) {
                        if c < 0 { Self::eval_op(&ins.operands[1], &regs) }
                        else if c == 0 { Self::eval_op(&ins.operands[2], &regs) }
                        else { Self::eval_op(&ins.operands[3], &regs) }
                    } else { OmniValue::Void }
                }
                0x7F48 => {
                    if let BinOperand::FnRef(callee_idx) = ins.operands[0] {
                        if self.call_depth >= MAX_VM_CALL_DEPTH {
                            return Err(format!("VM call depth budget exceeded ({} >= {})", self.call_depth, MAX_VM_CALL_DEPTH));
                        }
                        let call_args: Vec<OmniValue> = ins.operands[1..].iter().map(|op| Self::eval_op(op, &regs)).collect();
                        self.call_depth += 1;
                        let call_res = self.execute_function(module, callee_idx, call_args);
                        self.call_depth = self.call_depth.saturating_sub(1);
                        call_res?
                    } else { OmniValue::Void }
                }
                0x7F57 => if let OmniValue::Str(s) = Self::eval_op(&ins.operands[0], &regs) { OmniValue::Str(s.replace('\'', "''").replace(';', "")) } else { OmniValue::Str(String::new()) },
                0x7F58 => if let OmniValue::Str(s) = Self::eval_op(&ins.operands[0], &regs) { OmniValue::Str(s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")) } else { OmniValue::Str(String::new()) },
                0x7F59 => Self::eval_op(&ins.operands[0], &regs),
                0x7F5D => {
                    if let OmniValue::Str(s) = Self::eval_op(&ins.operands[0], &regs) {
                        let mut h: u64 = 0xcbf29ce484222325;
                        for b in s.bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
                        OmniValue::Int(h as i64)
                    } else { OmniValue::Int(0) }
                }
                0x7F63 => {
                    if let BinOperand::SchemaRef(s_idx) = ins.operands[0] {
                        self.charge_memory(64 * ins.operands.len())?;
                        let mut row = BTreeMap::new();
                        for op in &ins.operands[1..] {
                            if let BinOperand::FieldKV(f_id, inner) = op {
                                row.insert(*f_id, Self::eval_op(inner, &regs));
                            }
                        }
                        self.tables.entry(s_idx).or_default().push(row.clone());
                        OmniValue::Struct(row)
                    } else { OmniValue::Void }
                }
                0x7F6A => {
                    if let (BinOperand::SchemaRef(s_idx), OmniValue::TritVec(q_vec)) = (&ins.operands[0], Self::eval_op(&ins.operands[1], &regs)) {
                        let q_packed = pack5_encode(&q_vec);
                        let mut best = OmniValue::Void;
                        let mut best_score = i64::MIN;
                        if let Some(rows) = self.tables.get(s_idx) {
                            for row in rows {
                                for val in row.values() {
                                    if let OmniValue::TritVec(r_vec) = val {
                                        let r_packed = pack5_encode(r_vec);
                                        let score: i64 = pack5_tdot_direct(&q_packed, &r_packed, q_vec.len().min(r_vec.len())).unwrap_or(0);
                                        if score > best_score { best_score = score; best = OmniValue::Struct(row.clone()); }
                                    }
                                }
                            }
                        }
                        if best == OmniValue::Void {
                            let mut m = BTreeMap::new();
                            m.insert(0, OmniValue::Int(*s_idx as i64));
                            m.insert(1, OmniValue::Int(q_vec.len() as i64));
                            OmniValue::Struct(m)
                        } else {
                            best
                        }
                    } else { OmniValue::Void }
                }
                0x7F7D => {
                    if let OmniValue::Str(s) = Self::eval_op(&ins.operands[0], &regs) {
                        self.charge_memory(pack5_byte_len(64))?;
                        let mut trits = vec![0i8; 64];
                        let mut h: u32 = 2166136261;
                        for (idx, b) in s.bytes().enumerate() {
                            h = (h ^ (b as u32)).wrapping_mul(16777619);
                            trits[idx % 64] = ((h % 3) as i8) - 1;
                        }
                        OmniValue::TritVec(trits)
                    } else { OmniValue::TritVec(vec![0; 64]) }
                }
                0x7F7E => if let BinOperand::SkillRef(sk_id) = ins.operands[0] { OmniValue::SkillModel { skill_id: sk_id, active: true } } else { OmniValue::Void },
                0x7F7F => if let (OmniValue::SkillModel { skill_id, .. }, OmniValue::Str(prompt)) = (Self::eval_op(&ins.operands[0], &regs), Self::eval_op(&ins.operands[1], &regs)) { OmniValue::Str(format!("[Skill#{}->'{}']", skill_id, prompt)) } else { OmniValue::Void },
                0x7F76 => if let BinOperand::ImmInt(tag_id) = ins.operands[0] {
                    self.charge_memory(32 * ins.operands.len())?;
                    OmniValue::DomNode { tag_id: tag_id as u16, children: ins.operands[1..].iter().map(|op| Self::eval_op(op, &regs)).collect() }
                } else { OmniValue::Void },
                _ => OmniValue::Void,
            };

            if let Some(d_tok) = ins.dest_reg {
                if let Some(slot) = token_to_slot(d_tok) {
                    regs[slot] = out_val;
                }
            }
            pc += 1;
        }
        Ok(OmniValue::Void)
    }
}


/// Ergonomic single-function VM wrapper compatible with `OmniVm::new().execute(&func, &args)`.
pub struct OmniVm {
    pub inner: OmniBinaryVm,
}

impl OmniVm {
    pub fn new() -> Self {
        Self { inner: OmniBinaryVm::new() }
    }

    pub fn with_limit(step_limit: u64) -> Self {
        Self {
            inner: OmniBinaryVm::with_budgets(step_limit, DEFAULT_VM_MEMORY_BUDGET_BYTES),
        }
    }

    pub fn with_budgets(step_limit: u64, memory_budget_bytes: usize) -> Self {
        Self {
            inner: OmniBinaryVm::with_budgets(step_limit, memory_budget_bytes),
        }
    }

    pub fn execute(&mut self, func: &BinFunction, args: &[OmniValue]) -> Result<OmniValue, String> {
        let module = BinModule {
            schemas: Vec::new(),
            experts: Vec::new(),
            functions: vec![func.clone()],
        };
        self.inner.execute_function(&module, func.fn_idx, args.to_vec())
    }
}
