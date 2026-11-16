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
/// Documented `DB_QRY` (0x7F65) not-found value (Boss Lady decision; HANDOFF.md §6.7). Never `key*10`.
pub const OMNI_DB_QRY_MISS: i64 = -1;
/// Documented `DB_GET` (0x7F64) not-found value (Boss Lady decision 2026-11-14). Never `key*10`.
pub const OMNI_DB_GET_MISS: i64 = -1;
/// Fixed KV key cap (Boss Lady 2026-11-14): 8192 distinct keys, the `pure/host_kv.omni` arena size
/// (0x1B0000..0x1EFFFF at 32 B). The 8193rd new key is `BUDGET_EXCEEDED`; overwrites still succeed.
pub const OMNI_VM_KV_MAX_KEYS: usize = 8192;

pub struct OmniBinaryVm {
    pub tables: HashMap<u16, Vec<BTreeMap<u16, OmniValue>>>,
    pub kv_store: BTreeMap<i64, i64>,
    pub req_params: BTreeMap<i64, i64>,
    pub last_http_response: Option<(u16, String, String)>,
    pub step_limit: u64,
    pub steps_executed: u64,
    pub memory_budget_bytes: usize,
    pub allocated_bytes: usize,
    pub call_depth: usize,
    pub verified_fns: u64,
}

impl OmniBinaryVm {
    pub fn new() -> Self {
        Self {
            tables: HashMap::new(),
            kv_store: BTreeMap::new(),
            req_params: BTreeMap::new(),
            last_http_response: None,
            step_limit: DEFAULT_VM_STEP_LIMIT,
            steps_executed: 0,
            memory_budget_bytes: DEFAULT_VM_MEMORY_BUDGET_BYTES,
            allocated_bytes: 0,
            call_depth: 0,
            verified_fns: 0,
        }
    }

    pub fn with_budgets(step_limit: u64, memory_budget_bytes: usize) -> Self {
        Self {
            tables: HashMap::new(),
            kv_store: BTreeMap::new(),
            req_params: BTreeMap::new(),
            last_http_response: None,
            step_limit,
            steps_executed: 0,
            memory_budget_bytes,
            allocated_bytes: 0,
            call_depth: 0,
            verified_fns: 0,
        }
    }

    pub fn clear_tables(&mut self) {
        self.tables.clear();
        self.kv_store.clear();
        self.req_params.clear();
        self.last_http_response = None;
        self.steps_executed = 0;
        self.allocated_bytes = 0;
        self.call_depth = 0;
        self.verified_fns = 0;
    }

    #[inline]
    fn charge_memory(&mut self, bytes: usize) -> Result<(), String> {
        let next_alloc = self.allocated_bytes.saturating_add(bytes);
        if next_alloc > self.memory_budget_bytes {
            return Err(format!(
                "error[BUDGET_EXCEEDED]: VM memory budget exceeded ({} > {} bytes)",
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

    #[inline]
    fn eval_op_ref<'a>(op: &BinOperand, regs: &'a [OmniValue; 224]) -> Option<&'a OmniValue> {
        match op {
            BinOperand::Reg(t) | BinOperand::Param(t) => token_to_slot(*t).map(|s| &regs[s]),
            _ => None,
        }
    }

    #[inline]
    fn eval_as_i64(op: &BinOperand, regs: &[OmniValue; 224]) -> Option<i64> {
        match op {
            BinOperand::Reg(t) | BinOperand::Param(t) => match token_to_slot(*t).map(|s| &regs[s]) {
                Some(OmniValue::Int(v)) => Some(*v),
                Some(OmniValue::Trit(t)) => Some(*t as i64),
                _ => None,
            },
            BinOperand::ImmInt(v) => Some(*v),
            BinOperand::ImmTrit(t) => Some(*t as i64),
            _ => None,
        }
    }

    #[inline]
    fn eval_as_trit(op: &BinOperand, regs: &[OmniValue; 224]) -> Option<i8> {
        match op {
            BinOperand::Reg(t) | BinOperand::Param(t) => match token_to_slot(*t).map(|s| &regs[s]) {
                Some(OmniValue::Trit(t)) => Some((*t).clamp(-1, 1)),
                Some(OmniValue::Int(v)) => Some((*v).clamp(-1, 1) as i8),
                _ => None,
            },
            BinOperand::ImmTrit(t) => Some((*t).clamp(-1, 1)),
            BinOperand::ImmInt(v) => Some((*v).clamp(-1, 1) as i8),
            _ => None,
        }
    }

    pub fn execute_function(
        &mut self,
        module: &BinModule,
        fn_idx: u16,
        args: Vec<OmniValue>,
    ) -> Result<OmniValue, String> {
        let mut no_restore: [(usize, OmniValue); 0] = [];
        self.execute_function_inner(module, fn_idx, args, 0, &mut no_restore)
    }

    fn execute_function_inner(
        &mut self,
        module: &BinModule,
        fn_idx: u16,
        args: Vec<OmniValue>,
        owned_params: u32,
        restore_borrowed: &mut [(usize, OmniValue)],
    ) -> Result<OmniValue, String> {
        if self.call_depth == 0 {
            self.verified_fns = 0;
        }
        let func = if let Some(f) = module.functions.get(fn_idx as usize).filter(|f| f.fn_idx == fn_idx) {
            f
        } else {
            module
                .functions
                .iter()
                .find(|f| f.fn_idx == fn_idx)
                .ok_or_else(|| format!("Function index {} not found", fn_idx))?
        };

        let fn_bit = if (fn_idx as usize) < 64 { 1u64 << fn_idx } else { 0 };
        if fn_bit == 0 || (self.verified_fns & fn_bit) == 0 {
            let rep = verify_binary_function(func);
            if !rep.passed {
                return Err(format!("Binary verification failed: {:?}", rep.errors));
            }
            self.verified_fns |= fn_bit;
        }

        let mut regs: [OmniValue; 224] = std::array::from_fn(|_| OmniValue::Void);
        for (&(par_tok, _, _), val) in func.params.iter().zip(args.into_iter()) {
            if let Some(slot) = token_to_slot(par_tok) {
                regs[slot] = val;
            }
        }

        let mut label_pcs: Option<[usize; 256]> = None;
        let resolve_lbl = |lbl: u16, lp: &mut Option<[usize; 256]>| -> Option<usize> {
            let table = lp.get_or_insert_with(|| {
                let mut t = [usize::MAX; 256];
                for (idx_i, cand) in func.instructions.iter().enumerate() {
                    if cand.op_tok == 0x7F40 {
                        if let Some(lid) = cand.operands.first().and_then(extract_label_id) {
                            if (lid as usize) < 256 {
                                t[lid as usize] = idx_i + 1;
                            }
                        }
                    }
                }
                t
            });
            if (lbl as usize) < 256 && table[lbl as usize] != usize::MAX {
                Some(table[lbl as usize])
            } else {
                func.instructions
                    .iter()
                    .position(|cand| cand.op_tok == 0x7F40 && cand.operands.first().and_then(extract_label_id) == Some(lbl))
                    .map(|p| p + 1)
            }
        };

        let mut pc = 0usize;
        while pc < func.instructions.len() {
            self.steps_executed = self.steps_executed.saturating_add(1);
            if self.steps_executed > self.step_limit {
                for &mut (p_idx, ref mut out_v) in restore_borrowed.iter_mut() {
                    *out_v = std::mem::replace(&mut regs[192 + p_idx], OmniValue::Void);
                }
                return Err(format!("error[BUDGET_EXCEEDED]: VM fuel/step budget exceeded ({} > {})", self.steps_executed, self.step_limit));
            }
            let ins = &func.instructions[pc];
            match ins.op_tok {
                0x7F00 | 0x7F40 | 0x7F46 => { pc += 1; continue; }
                0x7F41 => {
                    if let Some(target_lbl) = ins.operands.first().and_then(extract_label_id) {
                        if let Some(tp) = resolve_lbl(target_lbl, &mut label_pcs) {
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
                        if let Some(tp) = resolve_lbl(chosen, &mut label_pcs) {
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
                            if let Some(tp) = resolve_lbl(target_lbl, &mut label_pcs) {
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
                0x7F4A => {
                    let ret_val = match &ins.operands[0] {
                        BinOperand::Reg(t) => {
                            token_to_slot(*t).map(|s| std::mem::replace(&mut regs[s], OmniValue::Void)).unwrap_or(OmniValue::Void)
                        }
                        BinOperand::Param(t) if (owned_params & (1u32 << (*t - PAR_BASE))) != 0 => {
                            token_to_slot(*t).map(|s| std::mem::replace(&mut regs[s], OmniValue::Void)).unwrap_or(OmniValue::Void)
                        }
                        other => Self::eval_op(other, &regs),
                    };
                    for &mut (p_idx, ref mut out_v) in restore_borrowed.iter_mut() {
                        *out_v = std::mem::replace(&mut regs[192 + p_idx], OmniValue::Void);
                    }
                    return Ok(ret_val);
                }
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
                0x7F04 | 0x7F06 | 0x7F09 => match &ins.operands[0] {
                    BinOperand::Reg(t)
                        if !func.instructions[..=pc].iter().any(|c| c.op_tok == 0x7F40)
                            && !func.instructions[pc + 1..].iter().any(|later| {
                                later.operands.iter().any(|op| matches!(op, BinOperand::Reg(ot) if ot == t))
                            }) =>
                    {
                        token_to_slot(*t).map(|s| std::mem::replace(&mut regs[s], OmniValue::Void)).unwrap_or(OmniValue::Void)
                    }
                    BinOperand::Param(t)
                        if (owned_params & (1u32 << (*t - PAR_BASE))) != 0
                            && !func.instructions[..=pc].iter().any(|c| c.op_tok == 0x7F40)
                            && !func.instructions[pc + 1..].iter().any(|later| {
                                later.operands.iter().any(|op| matches!(op, BinOperand::Param(ot) if ot == t))
                            }) =>
                    {
                        token_to_slot(*t).map(|s| std::mem::replace(&mut regs[s], OmniValue::Void)).unwrap_or(OmniValue::Void)
                    }
                    other => Self::eval_op(other, &regs),
                },
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
                0x7F0E if ins.operands.len() == 3 => {
                    // ISA-B 3-op IGET buf, mode, off: LE zero-extended load of 2/4/8 bytes.
                    // Strict bounds: 0 <= off && off + width <= len (checked add); otherwise a
                    // deterministic runtime error[OUT_OF_BOUNDS] (never read-as-0).
                    let width = match ins.operands[1] {
                        BinOperand::ImmInt(-2) => 2usize,
                        BinOperand::ImmInt(-4) => 4usize,
                        BinOperand::ImmInt(-8) => 8usize,
                        _ => return Err(String::from("error[BAD_TYPE]: 3-op IGET mode must be -2, -4, or -8")),
                    };
                    let off = Self::eval_as_i64(&ins.operands[2], &regs);
                    match (Self::eval_op_ref(&ins.operands[0], &regs), off) {
                        (Some(OmniValue::Bytes(buf)), Some(off)) if off >= 0 => {
                            match (off as u64).checked_add(width as u64) {
                                Some(end) if end <= buf.len() as u64 => {
                                    let o = off as usize;
                                    let mut le = [0u8; 8];
                                    le[..width].copy_from_slice(&buf[o..o + width]);
                                    OmniValue::Int(u64::from_le_bytes(le) as i64)
                                }
                                _ => return Err(String::from(OMNI_OOB_ERROR)),
                            }
                        }
                        _ => return Err(String::from(OMNI_OOB_ERROR)),
                    }
                }
                0x7F0E => {
                    let idx_opt = Self::eval_as_i64(&ins.operands[1], &regs);
                    match (Self::eval_op_ref(&ins.operands[0], &regs), idx_opt) {
                        (Some(OmniValue::U16Array(arr)), Some(i)) if i >= 0 && (i as usize) < arr.len() => {
                            OmniValue::Int(arr[i as usize] as i64)
                        }
                        (Some(OmniValue::Bytes(buf)), Some(i)) if i >= 0 && (i as usize) < buf.len() => {
                            OmniValue::Int(buf[i as usize] as i64)
                        }
                        (Some(OmniValue::TritVec(tv)), Some(i)) if i >= 0 && (i as usize) < tv.len() => {
                            let packed = pack5_encode(tv);
                            OmniValue::Int(pack5_get_trit(&packed, i as usize).map_err(String::from)? as i64)
                        }
                        _ => OmniValue::Int(0),
                    }
                }
                0x7F0F => {
                    let src_tok = match ins.operands.first() {
                        Some(BinOperand::Reg(t) | BinOperand::Param(t)) => Some(*t),
                        _ => None,
                    };
                    let not_in_rest_ops = !ins.operands[1..].iter().any(|op| match op {
                        BinOperand::Reg(t) | BinOperand::Param(t) => Some(*t) == src_tok,
                        _ => false,
                    });
                    let can_take_owned_param = match src_tok {
                        Some(pt) if (PAR_BASE..=PAR_MAX).contains(&pt) && (owned_params & (1u32 << (pt - PAR_BASE))) != 0 => {
                            not_in_rest_ops
                                && !func.instructions[pc + 1..].iter().any(|later| {
                                    later.operands.iter().any(|op| matches!(op, BinOperand::Param(t) if *t == pt))
                                })
                        }
                        _ => false,
                    };
                    let can_take_last_use = match src_tok {
                        Some(t) if (REG_BASE..=REG_MAX).contains(&t) || ((PAR_BASE..=PAR_MAX).contains(&t) && (owned_params & (1u32 << (t - PAR_BASE))) != 0) => {
                            not_in_rest_ops
                                && !func.instructions[..=pc].iter().any(|c| c.op_tok == 0x7F40)
                                && !func.instructions[pc + 1..].iter().any(|later| {
                                    later.operands.iter().any(|op| matches!(op, BinOperand::Reg(ot) | BinOperand::Param(ot) if *ot == t))
                                })
                        }
                        _ => false,
                    };
                    let can_take_in_place = (ins.dest_reg.is_some() && ins.dest_reg == src_tok && not_in_rest_ops) || can_take_owned_param || can_take_last_use;
                    let mut buf = if can_take_in_place {
                        if let Some(slot) = src_tok.and_then(token_to_slot) {
                            match std::mem::replace(&mut regs[slot], OmniValue::Void) {
                                OmniValue::Bytes(b) => b,
                                _ => Vec::new(),
                            }
                        } else {
                            Vec::new()
                        }
                    } else {
                        match Self::eval_op(&ins.operands[0], &regs) {
                            OmniValue::Bytes(b) => b,
                            _ => Vec::new(),
                        }
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
                            if off >= 0 {
                                let u_off = off as usize;
                                if u_off.checked_add(8).map(|end| end <= buf.len()).unwrap_or(false) {
                                    buf[u_off..u_off + 8].copy_from_slice(&(v as u64).to_le_bytes());
                                }
                            }
                        }
                    } else if ins.operands.len() == 4
                        && (ins.operands[1] == BinOperand::ImmInt(-2) || ins.operands[1] == BinOperand::ImmInt(-4))
                    {
                        // ISA-B 4-op ISET buf, mode, off, val: LE patch of 2/4 bytes. Strict bounds
                        // (0 <= off && off + width <= len, checked add); out of bounds is a
                        // deterministic runtime error[OUT_OF_BOUNDS] (never a silent no-op).
                        let width = if ins.operands[1] == BinOperand::ImmInt(-2) { 2usize } else { 4usize };
                        if let (Some(off), Some(v)) = (
                            Self::eval_as_i64(&ins.operands[2], &regs),
                            Self::eval_as_i64(&ins.operands[3], &regs),
                        ) {
                            let in_bounds = off >= 0
                                && matches!((off as u64).checked_add(width as u64), Some(end) if end <= buf.len() as u64);
                            if !in_bounds {
                                return Err(String::from(OMNI_OOB_ERROR));
                            }
                            let o = off as usize;
                            buf[o..o + width].copy_from_slice(&(v as u64).to_le_bytes()[..width]);
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
                0x7F50 => match Self::eval_op_ref(&ins.operands[0], &regs) {
                    Some(OmniValue::Bytes(b)) => OmniValue::Int(b.len() as i64),
                    Some(OmniValue::Str(s)) => OmniValue::Int(s.len() as i64),
                    Some(OmniValue::U16Array(a)) => OmniValue::Int(a.len() as i64),
                    Some(OmniValue::TritVec(t)) => OmniValue::Int(t.len() as i64),
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
                            return Err(format!("error[BUDGET_EXCEEDED]: VM call depth budget exceeded ({} >= {})", self.call_depth, MAX_VM_CALL_DEPTH));
                        }
                        let dest_tok = ins.dest_reg;
                        let n_call_ops = ins.operands.len().saturating_sub(1);
                        let mut call_args: Vec<OmniValue> = Vec::with_capacity(n_call_ops);
                        let mut callee_owned: u32 = 0;
                        let mut restore_pairs: [(usize, OmniValue); 8] = std::array::from_fn(|_| (0, OmniValue::Void));
                        let mut caller_slots: [usize; 8] = [0; 8];
                        let mut n_borrowed = 0usize;
                        for (idx_op, op) in ins.operands[1..].iter().enumerate() {
                            let arg_val = match op {
                                BinOperand::Reg(t) | BinOperand::Param(t)
                                    if idx_op < 32
                                        && !ins.operands[1..].iter().enumerate().any(|(j, other)| {
                                            j != idx_op && matches!(other, BinOperand::Reg(ot) | BinOperand::Param(ot) if ot == t)
                                        }) =>
                                {
                                    if let Some(s) = token_to_slot(*t) {
                                        if Some(*t) == dest_tok {
                                            callee_owned |= 1u32 << idx_op;
                                            std::mem::replace(&mut regs[s], OmniValue::Void)
                                        } else if matches!(regs[s], OmniValue::Bytes(_) | OmniValue::U16Array(_)) {
                                            let is_owned_src = (REG_BASE..=REG_MAX).contains(t)
                                                || ((PAR_BASE..=PAR_MAX).contains(t) && (owned_params & (1u32 << (*t - PAR_BASE))) != 0);
                                            let dead_after = is_owned_src
                                                && !func.instructions[..=pc].iter().any(|c| c.op_tok == 0x7F40)
                                                && !func.instructions[pc + 1..].iter().any(|later| {
                                                    later.operands.iter().any(|op_l| matches!(op_l, BinOperand::Reg(ot) | BinOperand::Param(ot) if ot == t))
                                                });
                                            if dead_after {
                                                callee_owned |= 1u32 << idx_op;
                                                std::mem::replace(&mut regs[s], OmniValue::Void)
                                            } else if n_borrowed < 8 {
                                                let moved = std::mem::replace(&mut regs[s], OmniValue::Void);
                                                restore_pairs[n_borrowed] = (idx_op, OmniValue::Void);
                                                caller_slots[n_borrowed] = s;
                                                n_borrowed += 1;
                                                moved
                                            } else {
                                                regs[s].clone()
                                            }
                                        } else {
                                            regs[s].clone()
                                        }
                                    } else {
                                        OmniValue::Void
                                    }
                                }
                                _ => Self::eval_op(op, &regs),
                            };
                            call_args.push(arg_val);
                        }
                        self.call_depth += 1;
                        let call_res = self.execute_function_inner(module, callee_idx, call_args, callee_owned, &mut restore_pairs[..n_borrowed]);
                        self.call_depth = self.call_depth.saturating_sub(1);
                        for k in 0..n_borrowed {
                            regs[caller_slots[k]] = std::mem::replace(&mut restore_pairs[k].1, OmniValue::Void);
                        }
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
                0x7F64 | 0x7F65 => {
                    // DB_QRY (0x7F65) miss = OMNI_DB_QRY_MISS (-1); DB_GET (0x7F64) miss = OMNI_DB_GET_MISS (-1).
                    let is_qry = ins.op_tok == 0x7F65;
                    if ins.operands.len() == 1 {
                        let key = Self::eval_as_i64(&ins.operands[0], &regs).unwrap_or(0);
                        let miss = if is_qry { OMNI_DB_QRY_MISS } else { OMNI_DB_GET_MISS };
                        let val = self.kv_store.get(&key).copied().unwrap_or(miss);
                        OmniValue::Int(val)
                    } else if ins.operands.len() >= 2 {
                        let tbl = Self::eval_as_i64(&ins.operands[0], &regs).unwrap_or(0);
                        let key = Self::eval_as_i64(&ins.operands[1], &regs).unwrap_or(0);
                        let composite_key = tbl.wrapping_mul(1_000_000).wrapping_add(key);
                        let miss = if is_qry { OMNI_DB_QRY_MISS } else { OMNI_DB_GET_MISS };
                        let val = self
                            .kv_store
                            .get(&composite_key)
                            .or_else(|| self.kv_store.get(&key))
                            .copied()
                            .unwrap_or(miss);
                        OmniValue::Int(val)
                    } else {
                        OmniValue::Int(0)
                    }
                }
                0x7F68 => {
                    let cnt = self.kv_store.len() + self.tables.values().map(|v| v.len()).sum::<usize>();
                    OmniValue::Int(cnt as i64)
                }
                0x7F6B => {
                    if ins.operands.len() >= 2 {
                        let key = Self::eval_as_i64(&ins.operands[0], &regs).unwrap_or(0);
                        let val = Self::eval_as_i64(&ins.operands[1], &regs).unwrap_or(0);
                        // A new distinct key past OMNI_VM_KV_MAX_KEYS fails before the charge; overwrites still work.
                        if self.kv_store.len() >= OMNI_VM_KV_MAX_KEYS && !self.kv_store.contains_key(&key) {
                            return Err(format!(
                                "error[BUDGET_EXCEEDED]: VM KV store key cap exceeded ({} distinct keys, max {})",
                                self.kv_store.len(), OMNI_VM_KV_MAX_KEYS
                            ));
                        }
                        self.charge_memory(16)?;
                        self.kv_store.insert(key, val);
                        OmniValue::Int(val)
                    } else {
                        self.charge_memory(16)?;
                        OmniValue::Int(0)
                    }
                }
                0x7F6C => {
                    let method_id = ins.operands.first().and_then(|op| Self::eval_as_i64(op, &regs)).unwrap_or(1);
                    let route_id = ins.operands.get(1).and_then(|op| Self::eval_as_i64(op, &regs)).unwrap_or(0);
                    OmniValue::Int(method_id.wrapping_mul(1000).wrapping_add(route_id & 0x3FF))
                }
                0x7F6D => {
                    if let Some(op1) = ins.operands.get(1) {
                        if let Some(key_hash) = Self::eval_as_i64(op1, &regs) {
                            if let Some(&val) = self.req_params.get(&key_hash) {
                                OmniValue::Int(val)
                            } else if let Some(op0) = ins.operands.first() {
                                match op0 {
                                    BinOperand::ImmInt(p_idx) if (0..32).contains(p_idx) => {
                                        let slot = 192 + (*p_idx as usize);
                                        if regs[slot] != OmniValue::Void {
                                            regs[slot].clone()
                                        } else {
                                            OmniValue::Int(*p_idx)
                                        }
                                    }
                                    other => Self::eval_op(other, &regs),
                                }
                            } else {
                                OmniValue::Int(0)
                            }
                        } else if let Some(op0) = ins.operands.first() {
                            Self::eval_op(op0, &regs)
                        } else {
                            OmniValue::Int(0)
                        }
                    } else if let Some(op0) = ins.operands.first() {
                        if let Some(key_hash) = Self::eval_as_i64(op0, &regs) {
                            if let Some(&val) = self.req_params.get(&key_hash) {
                                OmniValue::Int(val)
                            } else {
                                match op0 {
                                    BinOperand::ImmInt(p_idx) if (0..32).contains(p_idx) => {
                                        let slot = 192 + (*p_idx as usize);
                                        if regs[slot] != OmniValue::Void {
                                            regs[slot].clone()
                                        } else {
                                            OmniValue::Int(*p_idx)
                                        }
                                    }
                                    other => Self::eval_op(other, &regs),
                                }
                            }
                        } else {
                            Self::eval_op(op0, &regs)
                        }
                    } else {
                        OmniValue::Int(0)
                    }
                }
                0x7F6E => {
                    let status = ins.operands.first().and_then(|op| Self::eval_as_i64(op, &regs)).unwrap_or(200);
                    let body = ins.operands.get(1).and_then(|op| Self::eval_as_i64(op, &regs)).unwrap_or(0);
                    let json_body = format!("{{\"status\":{},\"data\":{}}}", status, body);
                    self.last_http_response = Some((
                        status.clamp(100, 599) as u16,
                        String::from("application/json"),
                        json_body,
                    ));
                    OmniValue::Int(status.wrapping_mul(1_000_000).wrapping_add(body))
                }
                0x7F6F => {
                    let status = ins.operands.first().and_then(|op| Self::eval_as_i64(op, &regs)).unwrap_or(200);
                    let body = ins.operands.get(1).map(|op| Self::eval_op(op, &regs)).unwrap_or(OmniValue::Void);
                    let out_s = format!("<http status=\"{}\">{:?}</http>", status, body);
                    self.charge_memory(out_s.len())?;
                    self.last_http_response = Some((
                        status.clamp(100, 599) as u16,
                        String::from("text/html; charset=utf-8"),
                        out_s.clone(),
                    ));
                    OmniValue::Str(out_s)
                }
                0x7F70 => {
                    if let Some(op0) = ins.operands.first() {
                        Self::eval_op(op0, &regs)
                    } else {
                        OmniValue::Int(0)
                    }
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
        for &mut (p_idx, ref mut out_v) in restore_borrowed.iter_mut() {
            *out_v = std::mem::replace(&mut regs[192 + p_idx], OmniValue::Void);
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
