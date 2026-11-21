// M2-03: mandatory Milestone 2 lint + SSA/label budget report (host-side tooling, Rust).
//
// Lives outside the monolith on purpose: `omni_rust_bootstrap.rs` only `include!`s this file
// from its M2 FOUNDATION section, so main-line ISA-A/ISA-B edits never collide with it.
// Changes no emitted byte: read-only over decoded token streams (frozen hashes untouched).
//
// Token-level lint over the Rust frontend's (or assembler's) output, run on every
// `pure/m2/*.omni` module listed in M2_PURE_MODULES. Rules (spec/m2/PLAN.md §5.2):
//   L1  EAGER_SEL_BYTES     SEL_TRIT (0x7F45) has a bytes-typed operand (all arms are evaluated eagerly)
//   L2  ANEW_COUNT          more than one ANEW (0x7F0D) in the module
//   L3  LINEAR_BYTES        a bytes handle is read after it was consumed on some path, or a CALL
//                           passes more than one bytes-typed argument
//   L4  IGET2_NON_ARRAY     2-operand IGET on anything but an ARRAY-typed operand
//   L6  PACK5_OP            pack5 opcodes (0x7F30/31/34/35/3F)
//   L7  DIV_DIVISOR         IDIV/IMOD divisor is not an immediate, or is 0 / -1 (D-9: IMOD stays, lint rejects)
//   L8  CALL_GRAPH          unknown callee, arity mismatch, recursion, or call chain > 64 frames
//   L9  READ_BEFORE_WRITE   a register is read on a path where it was never written
//   L11 BUDGET              > 120 SSA registers or > 12 labels per function (M2 targets)
//       HARD_CAP            > 192 SSA registers, or more labels than the producer's cap:
//                           16 for assembly-built functions (ids 0..15; emitter + all four backends),
//                           15 for frontend-compiled functions until main-line C4 lands (the source
//                           compiler numbers labels from 1: omni_rust_bootstrap.rs:3587, :3646-3656).
// L5 (arena reads via C1 3-op forms only) is subsumed by L4 until ISA-A/C1 lands; L10 (i32 literals)
// is already enforced by the assembler / frontend.

pub const M2_BUDGET_SSA: usize = 120;
pub const M2_BUDGET_LABELS: usize = 12;
pub const M2_HARD_SSA: usize = 192;
/// Label cap for assembly-built functions (label ids 0..15).
pub const M2_LABEL_CAP_ASM: usize = 16;
/// Label cap for frontend-compiled functions (ids 0..15 after C4).
pub const M2_LABEL_CAP_FRONTEND: usize = 16;
pub const M2_MAX_CALL_FRAMES: usize = 64;

#[derive(Clone, Debug, PartialEq)]
pub struct M2LintFinding {
    pub rule: &'static str,
    pub fn_idx: u16,
    pub offset: usize,
    pub msg: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct M2BudgetRow {
    pub fn_idx: u16,
    pub ssa: usize,
    pub labels: usize,
    pub params: usize,
    pub instructions: usize,
}

#[derive(Clone, Copy, PartialEq)]
struct M2RegSet([u64; 3]);

impl M2RegSet {
    const EMPTY: M2RegSet = M2RegSet([0; 3]);
    const FULL: M2RegSet = M2RegSet([u64::MAX; 3]);
    fn has(&self, r: usize) -> bool {
        r < 192 && (self.0[r / 64] >> (r % 64)) & 1 == 1
    }
    fn set(&mut self, r: usize) {
        if r < 192 {
            self.0[r / 64] |= 1 << (r % 64);
        }
    }
    fn clear(&mut self, r: usize) {
        if r < 192 {
            self.0[r / 64] &= !(1 << (r % 64));
        }
    }
    fn union(&self, o: &M2RegSet) -> M2RegSet {
        M2RegSet([self.0[0] | o.0[0], self.0[1] | o.0[1], self.0[2] | o.0[2]])
    }
    fn inter(&self, o: &M2RegSet) -> M2RegSet {
        M2RegSet([self.0[0] & o.0[0], self.0[1] & o.0[1], self.0[2] & o.0[2]])
    }
}

fn m2_reg_index(op: &BinOperand) -> Option<usize> {
    match *op {
        BinOperand::Reg(r) => Some((r - REG_BASE) as usize),
        _ => None,
    }
}

/// Basic blocks of a function: (start, end) instruction ranges plus successor lists.
fn m2_cfg(f: &BinFunction) -> (Vec<(usize, usize)>, Vec<Vec<usize>>) {
    let n = f.instructions.len();
    let mut starts = vec![false; n + 1];
    starts[0] = true;
    for (i, ins) in f.instructions.iter().enumerate() {
        if ins.op_tok == 0x7F40 {
            starts[i] = true;
        }
        if matches!(ins.op_tok, 0x7F41..=0x7F44 | 0x7F4A) {
            starts[i + 1] = true;
        }
    }
    let mut blocks: Vec<(usize, usize)> = Vec::new();
    let mut s = 0usize;
    for i in 1..=n {
        if starts[i] || i == n {
            if s < i {
                blocks.push((s, i));
            }
            s = i;
        }
    }
    let block_of_ins = |i: usize, blocks: &[(usize, usize)]| blocks.iter().position(|&(a, b)| a <= i && i < b);
    // First LABEL definition wins (matches the emitter's first-match relocation).
    let mut label_block: Vec<(u16, usize)> = Vec::new();
    for (i, ins) in f.instructions.iter().enumerate() {
        if ins.op_tok == 0x7F40 {
            if let Some(k) = ins.operands.first().and_then(extract_label_id) {
                if !label_block.iter().any(|(kk, _)| *kk == k) {
                    if let Some(b) = block_of_ins(i, &blocks) {
                        label_block.push((k, b));
                    }
                }
            }
        }
    }
    let target = |op: &BinOperand| extract_label_id(op).and_then(|k| label_block.iter().find(|(kk, _)| *kk == k).map(|x| x.1));
    let mut succ: Vec<Vec<usize>> = vec![Vec::new(); blocks.len()];
    for (bi, &(_, e)) in blocks.iter().enumerate() {
        let last = &f.instructions[e - 1];
        let fall = if bi + 1 < blocks.len() { Some(bi + 1) } else { None };
        match last.op_tok {
            0x7F4A => {}
            0x7F41 => {
                if let Some(t) = last.operands.first().and_then(|o| target(o)) {
                    succ[bi].push(t);
                }
            }
            0x7F42 | 0x7F43 | 0x7F44 => {
                for o in last.operands.iter().skip(1) {
                    if let Some(t) = target(o) {
                        succ[bi].push(t);
                    }
                }
                if let Some(ft) = fall {
                    succ[bi].push(ft);
                }
            }
            _ => {
                if let Some(ft) = fall {
                    succ[bi].push(ft);
                }
            }
        }
    }
    (blocks, succ)
}

fn m2_fn_by_idx(m: &BinModule, idx: u16) -> Option<&BinFunction> {
    m.functions.iter().find(|f| f.fn_idx == idx)
}

/// Entry function index, same rule as the surface runner (fn_idx 0, else the last function).
pub fn m2_entry_fn_idx(m: &BinModule) -> Option<u16> {
    if m.functions.iter().any(|f| f.fn_idx == 0) {
        Some(0)
    } else {
        m.functions.last().map(|f| f.fn_idx)
    }
}

/// Budget report: per function SSA registers used, distinct labels defined, params, instructions.
pub fn m2_budget_report(m: &BinModule) -> Vec<M2BudgetRow> {
    let mut rows = Vec::new();
    for f in &m.functions {
        let mut max_reg: Option<usize> = None;
        let mut labels: Vec<u16> = Vec::new();
        for ins in &f.instructions {
            let mut note = |r: usize| max_reg = Some(max_reg.map_or(r, |x: usize| x.max(r)));
            if let Some(d) = ins.dest_reg {
                note((d - REG_BASE) as usize);
            }
            for op in &ins.operands {
                if let Some(r) = m2_reg_index(op) {
                    note(r);
                }
            }
            if ins.op_tok == 0x7F40 {
                if let Some(k) = ins.operands.first().and_then(extract_label_id) {
                    if !labels.contains(&k) {
                        labels.push(k);
                    }
                }
            }
        }
        rows.push(M2BudgetRow {
            fn_idx: f.fn_idx,
            ssa: max_reg.map_or(0, |r| r + 1),
            labels: labels.len(),
            params: f.params.len(),
            instructions: f.instructions.len(),
        });
    }
    rows
}

/// Runs every M2 lint rule over a decoded module.
/// Runs every M2 lint rule over a decoded module.
pub fn m2_lint_module(m: &BinModule, label_cap: usize) -> Vec<M2LintFinding> {
    let mut out: Vec<M2LintFinding> = Vec::new();
    let mut push = |rule: &'static str, f: u16, off: usize, msg: String| out.push(M2LintFinding { rule, fn_idx: f, offset: off, msg });

    // L2: module-wide ANEW count.
    let anews: Vec<(u16, usize)> = m
        .functions
        .iter()
        .flat_map(|f| f.instructions.iter().filter(|i| i.op_tok == 0x7F0D).map(move |i| (f.fn_idx, i.offset)))
        .collect();
    if anews.len() > 1 {
        push("ANEW_COUNT", anews[1].0, anews[1].1, format!("{} ANEW instructions in module (max 1)", anews.len()));
    }

    for f in &m.functions {
        // A slot is bytes-typed if ANY definition is bytes-typed (join slots are re-defined by
        // untyped SEL_TRIT / MOV results); otherwise its first definition's type is used.
        let mut reg_ty = [TY_VOID; 192];
        for ins in &f.instructions {
            if let Some(d) = ins.dest_reg {
                let k = (d - REG_BASE) as usize;
                if reg_ty[k] == TY_VOID || ins.ty_tok == TY_BYTES {
                    reg_ty[k] = ins.ty_tok;
                }
            }
        }
        let ty_of = |op: &BinOperand| m2_operand_ty(op, f, &reg_ty);
        for ins in &f.instructions {
            let ops = &ins.operands;
            match ins.op_tok {
                0x7F45 => {
                    if ops.iter().any(|o| ty_of(o) == TY_BYTES) {
                        push("EAGER_SEL_BYTES", f.fn_idx, ins.offset, String::from("sel_trit with a bytes operand: every arm is evaluated eagerly"));
                    }
                }
                0x7F0E if ops.len() == 2 => {
                    if ty_of(&ops[0]) != TY_ARRAY {
                        push("IGET2_NON_ARRAY", f.fn_idx, ins.offset, String::from("2-operand IGET is only allowed on an ARRAY operand"));
                    }
                }
                0x7F30 | 0x7F31 | 0x7F34 | 0x7F35 | 0x7F3F => {
                    push("PACK5_OP", f.fn_idx, ins.offset, format!("pack5 opcode 0x{:04X} is excluded from M2", ins.op_tok));
                }
                0x7F13 | 0x7F14 => {
                    let ok = matches!(ops.get(1), Some(BinOperand::ImmInt(v)) if *v != 0 && *v != -1);
                    if !ok {
                        push("DIV_DIVISOR", f.fn_idx, ins.offset, String::from("IDIV/IMOD divisor must be an immediate other than 0 / -1"));
                    }
                }
                0x7F48 => {
                    let n_bytes = ops.iter().skip(1).filter(|o| ty_of(o) == TY_BYTES).count();
                    if n_bytes > 1 {
                        push("LINEAR_BYTES", f.fn_idx, ins.offset, format!("CALL passes {} bytes arguments (max 1)", n_bytes));
                    }
                    match ops.first() {
                        Some(BinOperand::FnRef(t)) => match m2_fn_by_idx(m, *t) {
                            None => push("CALL_GRAPH", f.fn_idx, ins.offset, format!("call to unknown fn #{}", t)),
                            Some(callee) => {
                                if callee.params.len() != ops.len() - 1 {
                                    push(
                                        "CALL_GRAPH",
                                        f.fn_idx,
                                        ins.offset,
                                        format!("arity mismatch calling fn #{}: {} args, {} params", t, ops.len() - 1, callee.params.len()),
                                    );
                                }
                            }
                        },
                        _ => push("CALL_GRAPH", f.fn_idx, ins.offset, String::from("CALL without a FnRef callee")),
                    }
                }
                _ => {}
            }
        }

        // L3 + L9: forward dataflow over the CFG.
        //   may_consumed: bytes handles consumed on SOME path (union);
        //   must_written: registers written on ALL paths (intersection).
        let (blocks, succ) = m2_cfg(f);
        let nb = blocks.len();
        if nb == 0 {
            continue;
        }
        let mut in_cons = vec![M2RegSet::EMPTY; nb];
        let mut in_wr = vec![M2RegSet::FULL; nb];
        in_wr[0] = M2RegSet::EMPTY;
        let mut reached = vec![false; nb];
        reached[0] = true;
        let transfer = |b: usize, cons: &mut M2RegSet, wr: &mut M2RegSet, report: &mut dyn FnMut(&'static str, usize, String)| {
            for ins in &f.instructions[blocks[b].0..blocks[b].1] {
                let dest = ins.dest_reg.map(|d| (d - REG_BASE) as usize);
                for op in &ins.operands {
                    if let Some(r) = m2_reg_index(op) {
                        if !wr.has(r) {
                            report("READ_BEFORE_WRITE", ins.offset, format!("%{} read before it is written on some path", r));
                        }
                        if cons.has(r) {
                            report("LINEAR_BYTES", ins.offset, format!("bytes handle %{} read after it was consumed", r));
                        }
                    }
                }
                // A bytes-producing instruction (or a call / return) consumes every bytes handle it reads.
                let consumes = ins.op_tok == 0x7F4A || ins.op_tok == 0x7F48 || ins.ty_tok == TY_BYTES;
                if consumes {
                    for op in &ins.operands {
                        if let Some(r) = m2_reg_index(op) {
                            if reg_ty[r] == TY_BYTES && Some(r) != dest {
                                cons.set(r);
                            }
                        }
                    }
                }
                if let Some(d) = dest {
                    wr.set(d);
                    cons.clear(d);
                }
            }
        };
        let mut changed = true;
        let mut guard = 0usize;
        while changed && guard < 4 * nb + 16 {
            changed = false;
            guard += 1;
            for b in 0..nb {
                if !reached[b] {
                    continue;
                }
                let (mut c, mut w) = (in_cons[b], in_wr[b]);
                transfer(b, &mut c, &mut w, &mut |_, _, _| {});
                for &s in &succ[b] {
                    let nc = in_cons[s].union(&c);
                    let nw = if reached[s] { in_wr[s].inter(&w) } else { w };
                    if !reached[s] || nc != in_cons[s] || nw != in_wr[s] {
                        reached[s] = true;
                        in_cons[s] = nc;
                        in_wr[s] = nw;
                        changed = true;
                    }
                }
            }
        }
        for b in 0..nb {
            if !reached[b] {
                continue;
            }
            let (mut c, mut w) = (in_cons[b], in_wr[b]);
            let fidx = f.fn_idx;
            transfer(b, &mut c, &mut w, &mut |rule, off, msg| push(rule, fidx, off, msg));
        }
    }

    // L8: call graph — recursion and longest chain (frames, entry = 1).
    let idxs: Vec<u16> = m.functions.iter().map(|f| f.fn_idx).collect();
    let callees = |f: &BinFunction| -> Vec<u16> {
        let mut v: Vec<u16> = f
            .instructions
            .iter()
            .filter(|i| i.op_tok == 0x7F48)
            .filter_map(|i| match i.operands.first() {
                Some(BinOperand::FnRef(t)) if idxs.contains(t) => Some(*t),
                _ => None,
            })
            .collect();
        v.sort();
        v.dedup();
        v
    };
    // Iterative DFS colouring (no recursion in the tooling either).
    let n = m.functions.len();
    let pos_of = |idx: u16| m.functions.iter().position(|f| f.fn_idx == idx);
    let mut colour = vec![0u8; n]; // 0 new, 1 on stack, 2 done
    let mut depth = vec![1usize; n];
    for root in 0..n {
        if colour[root] != 0 {
            continue;
        }
        let mut stack: Vec<(usize, usize)> = vec![(root, 0)];
        colour[root] = 1;
        while let Some(&(u, k)) = stack.last() {
            let cs = callees(&m.functions[u]);
            if k < cs.len() {
                let v = pos_of(cs[k]).unwrap_or(u);
                if let Some(top) = stack.last_mut() {
                    top.1 += 1;
                }
                match colour[v] {
                    0 => {
                        colour[v] = 1;
                        stack.push((v, 0));
                    }
                    1 => push(
                        "CALL_GRAPH",
                        m.functions[u].fn_idx,
                        0,
                        format!("recursion: fn #{} -> fn #{} closes a cycle", m.functions[u].fn_idx, m.functions[v].fn_idx),
                    ),
                    _ => {}
                }
            } else {
                let d = 1 + cs.iter().filter_map(|&c| pos_of(c)).filter(|&v| colour[v] == 2).map(|v| depth[v]).max().unwrap_or(0);
                depth[u] = d;
                colour[u] = 2;
                stack.pop();
            }
        }
    }
    if let Some(e) = m2_entry_fn_idx(m).and_then(pos_of) {
        if depth[e] > M2_MAX_CALL_FRAMES {
            push("CALL_GRAPH", m.functions[e].fn_idx, 0, format!("call chain of {} frames from the entry (max {})", depth[e], M2_MAX_CALL_FRAMES));
        }
    }

    // L11: budgets (M2 targets) and hard caps.
    for r in m2_budget_report(m) {
        if r.ssa > M2_BUDGET_SSA {
            push("BUDGET", r.fn_idx, 0, format!("{} SSA registers (M2 budget {}, hard cap {})", r.ssa, M2_BUDGET_SSA, M2_HARD_SSA));
        }
        if r.labels > M2_BUDGET_LABELS {
            push("BUDGET", r.fn_idx, 0, format!("{} labels (M2 budget {}, hard cap {})", r.labels, M2_BUDGET_LABELS, label_cap));
        }
        if r.ssa > M2_HARD_SSA {
            push("HARD_CAP", r.fn_idx, 0, format!("{} SSA registers > {}", r.ssa, M2_HARD_SSA));
        }
        if r.labels > label_cap {
            push("HARD_CAP", r.fn_idx, 0, format!("{} labels > cap {}", r.labels, label_cap));
        }
    }
    out
}

/// Compiles surface source with the Rust frontend, then lints it (label cap 15 until C4).
pub fn m2_lint_surface(src: &str) -> Result<(Vec<M2LintFinding>, Vec<M2BudgetRow>), String> {
    let u16s = compile_omni_surface_to_u16(src)?;
    let m = decode_omni_token_stream(&u16s)?;
    Ok((m2_lint_module(&m, M2_LABEL_CAP_FRONTEND), m2_budget_report(&m)))
}

/// Foreign modules linked into M2 modules but owned elsewhere. Their functions are linted
/// and reported as notes, never gated: pure/isa/table.omni (Raven, f600102) is the single
/// copy of opcode metadata and has its own budget lock (selftest 13BA). Its byte accessors
/// use 2-op IGET on bytes (L4), which is VM-only until C1's `IGET -1` lands (DECISIONS D-13).
pub const M2_FOREIGN_MODULES: &[(&str, &str)] = &[("pure/isa/table.omni", include_str!("../pure/isa/table.omni"))];

fn m2_count_fn_decls(src: &str) -> usize {
    src.lines().filter(|l| l.starts_with("fn ")).count()
}

/// Lints a frontend module; findings in functions declared by an M2_FOREIGN_MODULES source
/// (matched by position in declaration order) are split out. ANEW_COUNT only counts
/// M2-owned functions. Returns (gated findings, foreign notes, budget rows).
pub fn m2_lint_surface_scoped(src: &str) -> Result<(Vec<M2LintFinding>, Vec<M2LintFinding>, Vec<M2BudgetRow>), String> {
    let expanded = resolve_omni_surface_includes(src, None, None)?;
    let u16s = compile_omni_surface_to_u16(&expanded)?;
    let m = decode_omni_token_stream(&u16s)?;
    let total = m2_count_fn_decls(&expanded);
    if total != m.functions.len() {
        return Err(format!("M2 lint scope: {} fn declarations but {} compiled functions", total, m.functions.len()));
    }
    let mut foreign_idx: Vec<u16> = Vec::new();
    for &(_, fsrc) in M2_FOREIGN_MODULES {
        if let Some(pos) = expanded.find(fsrc) {
            let before = m2_count_fn_decls(&expanded[..pos]);
            let n = m2_count_fn_decls(fsrc);
            foreign_idx.extend(m.functions[before..before + n].iter().map(|f| f.fn_idx));
        }
    }
    // Whole-module lint (the call graph needs every function), then split by owner.
    let (notes, mut gated): (Vec<M2LintFinding>, Vec<M2LintFinding>) = m2_lint_module(&m, M2_LABEL_CAP_FRONTEND)
        .into_iter()
        .filter(|f| f.rule != "ANEW_COUNT")
        .partition(|f| foreign_idx.contains(&f.fn_idx));
    let mut own = m.clone();
    own.functions.retain(|f| !foreign_idx.contains(&f.fn_idx));
    gated.extend(m2_lint_module(&own, M2_LABEL_CAP_FRONTEND).into_iter().filter(|f| f.rule == "ANEW_COUNT"));
    Ok((gated, notes, m2_budget_report(&own)))
}

/// Lints an already-built token stream (assembly-built: label cap 16).
pub fn m2_lint_tokens(u16s: &[u16]) -> Result<(Vec<M2LintFinding>, Vec<M2BudgetRow>), String> {
    let m = decode_omni_token_stream(u16s)?;
    Ok((m2_lint_module(&m, M2_LABEL_CAP_ASM), m2_budget_report(&m)))
}

/// Assembles `.omni_asm` text, then lints it (label cap 16).
pub fn m2_lint_asm(asm: &str) -> Result<(Vec<M2LintFinding>, Vec<M2BudgetRow>), String> {
    m2_lint_tokens(&assemble_omni_text_to_u16(asm)?)
}

/// One budget-report line per function.
pub fn m2_render_budget_row(path: &str, r: &M2BudgetRow, label_cap: usize, producer: &str) -> String {
    let why = if producer == "asm" { "assembly-built" } else { "frontend-compiled, until C4; assembly-built cap 16" };
    format!(
        "{}\tfn#{}\tssa={}/{} (cap {})\tlabels={}/{} (cap {}: {})\tparams={}\tins={}",
        path, r.fn_idx, r.ssa, M2_BUDGET_SSA, M2_HARD_SSA, r.labels, M2_BUDGET_LABELS, label_cap, why, r.params, r.instructions
    )
}

/// `omni m2-lint <file.omni | file.omni_asm>...`: budget report + every finding; exit 1 on findings.
pub fn m2_lint_cli(paths: &[String]) -> i32 {
    if paths.is_empty() {
        eprintln!("m2-lint: no files given; linting the {} registered pure/m2 module(s)", M2_PURE_MODULES.len());
    }
    let mut status = 0;
    let mut items: Vec<(String, String)> = Vec::new();
    for path in paths {
        match std::fs::read_to_string(path) {
            Ok(s) => items.push((path.clone(), s)),
            Err(e) => {
                eprintln!("{}: {}", path, e);
                status = 2;
            }
        }
    }
    if paths.is_empty() {
        for &(name, src) in M2_PURE_MODULES {
            items.push((String::from(name), String::from(src)));
        }
    }
    for (path, src) in &items {
        let asm = path.ends_with(".omni_asm");
        let (cap, producer) = if asm { (M2_LABEL_CAP_ASM, "asm") } else { (M2_LABEL_CAP_FRONTEND, "frontend") };
        let res = if asm { m2_lint_asm(src).map(|(f, b)| (f, Vec::new(), b)) } else { m2_lint_surface_scoped(src) };
        match res {
            Ok((findings, notes, budget)) => {
                for f in &notes {
                    println!("{}	note[M2_LINT_FOREIGN:{}] fn#{} @{}: {} (foreign module, not gated)", path, f.rule, f.fn_idx, f.offset, f.msg);
                }
                for r in &budget {
                    println!("{}", m2_render_budget_row(path, r, cap, producer));
                }
                for f in &findings {
                    println!("{}\terror[M2_LINT:{}] fn#{} @{}: {}", path, f.rule, f.fn_idx, f.offset, f.msg);
                }
                if !findings.is_empty() {
                    status = status.max(1);
                }
            }
            Err(e) => {
                eprintln!("{}: {}", path, e);
                status = 2;
            }
        }
    }
    status
}

/// Every `pure/m2/*.omni` module (concatenation order = link order). The lint gate runs on all of
/// them (M2-owned functions gated; linked M2_FOREIGN_MODULES functions reported as notes).
pub const M2_PURE_MODULES: &[(&str, &str)] = &[
    ("pure/m2/util.omni+pure/isa/table.omni+pure/m2/decode.omni+spec/m2/decode_harness.omni", M2_DECODE_OMNI_SRC),
    ("pure/m2/util.omni+pure/isa/table.omni+pure/m2/decode.omni+spec/m2/decode_ws_harness.omni", M2_DECODE_WS_OMNI_SRC),
    ("pure/m2/util.omni+pure/isa/table.omni+pure/m2/decode.omni+pure/isa/encode.omni+spec/m2/encode_harness.omni", M3_ENCODE_OMNI_SRC),
    ("pure/m2/util.omni+pure/isa/table.omni+pure/m2/decode.omni+pure/m2/labels.omni+spec/m2/labels_harness.omni", M2_LABELS_OMNI_SRC),
    ("pure/m2/util.omni+pure/isa/table.omni+pure/m2/decode.omni+pure/m2/labels.omni+pure/m2/verify/mod_check.omni+spec/m2/verify_harness.omni", M2_VERIFY_OMNI_SRC),
    ("pure/m2/util.omni+pure/isa/table.omni+pure/m2/decode.omni+pure/m2/labels.omni+pure/m4/arena.omni+pure/m4/ops_core.omni+pure/m4/vm.omni+spec/m4/vm_harness.omni", M4_VM_OMNI_SRC),
    ("pure/m2/util.omni+pure/isa/table.omni+pure/m2/decode.omni+pure/m2/labels.omni+pure/m2/arena.omni+pure/m2/elf.omni+pure/m5/elf_sys.omni+spec/m5/sys_harness.omni", M5_SYS_OMNI_SRC),
    ("pure/m2/util.omni+pure/isa/table.omni+pure/m2/decode.omni+pure/m2/labels.omni+pure/m2/arena.omni+pure/m2/elf.omni+spec/m2/elf_harness.omni", M2_ELF_OMNI_SRC),
    ("pure/m2/util.omni+pure/isa/table.omni+pure/m2/decode.omni+pure/m2/labels.omni+pure/m2/arena.omni+pure/m2/elf.omni+pure/m2/emit_x86.omni+pure/m2/driver.omni+spec/m2/compiler_harness.omni", M2_COMPILER_OMNI_SRC),
    ("pure/m2/util.omni+pure/isa/table.omni+pure/m2/decode.omni+pure/m2/labels.omni+pure/m2/arena.omni+pure/m2/elf.omni+pure/m2/emit_x86.omni+pure/m2/emit_multi.omni+spec/m2/multi_harness.omni", M2_MULTI_OMNI_SRC),
    ("pure/m2/util.omni+pure/m3/arena.omni+pure/m3/lex.omni+spec/m3/lex_harness.omni", M3_LEX_OMNI_SRC),
    ("pure/m3/skim.omni+spec/m3/skim_harness.omni", M3_SKIM_OMNI_SRC),
    ("pure/m3/expr.omni+spec/m3/expr_harness.omni", M3_EXPR_OMNI_SRC),
    ("pure/m3/stmt.omni+spec/m3/stmt_harness.omni", M3_STMT_OMNI_SRC),
    ("pure/m3/lower.omni+spec/m3/lower_harness.omni", M3_LOWER_OMNI_SRC),
    ("pure/m7/judge.omni+spec/m7/judge_harness.omni", P7_JUDGE_OMNI_SRC),
];

/// Lint fixtures: (name, source, the exact comma-separated rule set it must trip; "" = clean).
pub const M2_LINT_FIXTURES: &[(&str, &str, &str)] = &[
    ("pos_clean", include_str!("../spec/m2/lint/pos_clean.omni"), ""),
    ("pos_depth64", include_str!("../spec/m2/lint/pos_depth64.omni"), ""),
    ("neg_eager_sel_bytes", include_str!("../spec/m2/lint/neg_eager_sel_bytes.omni"), "EAGER_SEL_BYTES"),
    ("neg_eager_sel_iset", include_str!("../spec/m2/lint/neg_eager_sel_iset.omni"), "EAGER_SEL_BYTES,LINEAR_BYTES"),
    ("neg_stale_handle", include_str!("../spec/m2/lint/neg_stale_handle.omni"), "LINEAR_BYTES"),
    ("neg_two_bytes_args", include_str!("../spec/m2/lint/neg_two_bytes_args.omni"), "LINEAR_BYTES"),
    ("neg_two_anew", include_str!("../spec/m2/lint/neg_two_anew.omni"), "ANEW_COUNT"),
    ("neg_iget_bytes", include_str!("../spec/m2/lint/neg_iget_bytes.omni"), "IGET2_NON_ARRAY"),
    ("neg_pack5", include_str!("../spec/m2/lint/neg_pack5.omni"), "PACK5_OP"),
    ("neg_div_dynamic", include_str!("../spec/m2/lint/neg_div_dynamic.omni"), "DIV_DIVISOR"),
    ("neg_div_zero", include_str!("../spec/m2/lint/neg_div_zero.omni"), "DIV_DIVISOR"),
    ("neg_recursion", include_str!("../spec/m2/lint/neg_recursion.omni"), "CALL_GRAPH"),
    ("neg_depth65", include_str!("../spec/m2/lint/neg_depth65.omni"), "CALL_GRAPH"),
    ("neg_budget_labels", include_str!("../spec/m2/lint/neg_budget_labels.omni"), "BUDGET"),
    ("neg_budget_ssa", include_str!("../spec/m2/lint/neg_budget_ssa.omni"), "BUDGET"),
];

fn m2_rule_set(findings: &[M2LintFinding]) -> String {
    let mut r: Vec<&str> = findings.iter().map(|f| f.rule).collect();
    r.sort();
    r.dedup();
    r.join(",")
}

/// One function with `n` distinct labels: ids 0..15 as LBL tokens, ids >= 16 as IMM16 label ids
/// (the assembler's text form stops at 15, so those are patched in at token level).
fn m2_lint_label_stream(n: usize) -> Result<Vec<u16>, String> {
    let mut s = String::from("PARAM $0 I64 SAFE\nRET_TY I64\n");
    for k in 0..n {
        s.push_str(&format!("LABEL {}\n", k.min(15)));
    }
    s.push_str("RET $0\n");
    let imms: Vec<[u16; 2]> = (16..n.max(16)).map(|k| m2_imm16(k as u16)).collect();
    let repl: Vec<(usize, &[u16])> = imms.iter().enumerate().map(|(i, t)| (16 + i, &t[..])).collect();
    m2_lbl_patch(&s, &repl)
}

/// selftest_m2_lint: every fixture trips exactly its rule set (positives are clean), the
/// token-level fixtures for shapes the frontend rejects first, the 16/15 label caps, and the
/// lint gate over every registered `pure/m2/*.omni` module.
pub fn selftest_m2_lint() -> Result<usize, String> {
    let mut checks = 0usize;
    for &(name, src, want) in M2_LINT_FIXTURES {
        let (findings, _) = m2_lint_surface(src).map_err(|e| format!("M2 lint fixture {} failed to compile: {}", name, e))?;
        if m2_rule_set(&findings) != want {
            return Err(format!("M2 lint fixture {} must trip exactly [{}], got {:?}", name, want, findings));
        }
        checks += 1;
    }
    // Token-level fixtures (the frontend rejects these shapes before the lint could see them).
    let tok_cases: [(&str, &str, &str); 4] = [
        (
            "tok_mutual_recursion",
            "FN_DEF 1\nPARAM $0 I64 SAFE\nRET_TY I64\n%0 = CALL_I64 @fn2 $0\nRET %0\nFN_DEF 2\nPARAM $0 I64 SAFE\nRET_TY I64\n%0 = CALL_I64 @fn1 $0\nRET %0\nFN_DEF 0\nPARAM $0 I64 SAFE\nRET_TY I64\n%0 = CALL_I64 @fn1 $0\nRET %0\n",
            "CALL_GRAPH",
        ),
        (
            "tok_arity",
            "FN_DEF 1\nPARAM $0 I64 SAFE\nRET_TY I64\nRET $0\nFN_DEF 0\nPARAM $0 I64 SAFE\nRET_TY I64\n%0 = CALL_I64 @fn1 $0 $0\nRET %0\n",
            "CALL_GRAPH",
        ),
        ("tok_unknown_callee", "PARAM $0 I64 SAFE\nRET_TY I64\n%0 = CALL_I64 @fn7 $0\nRET %0\n", "CALL_GRAPH"),
        ("tok_read_before_write", "PARAM $0 I64 SAFE\nRET_TY I64\n%0 = CINT 1\n%2 = IADD %0 %1\n%1 = CINT 2\nRET %2\n", "READ_BEFORE_WRITE"),
    ];
    for (name, asm, want) in tok_cases {
        let (f, _) = m2_lint_asm(asm).map_err(|e| format!("M2 lint token fixture {} failed: {}", name, e))?;
        if !f.iter().any(|x| x.rule == want) {
            return Err(format!("M2 lint token fixture {} must trip {}, got {:?}", name, want, f));
        }
        checks += 1;
    }
    // Label caps: assembly-built functions may use 16 labels (ids 0..15); 17 is a hard-cap error.
    // 13..16 labels still exceed the M2 budget of 12 (BUDGET only).
    for &(n, want) in &[(12usize, ""), (16, "BUDGET"), (17, "BUDGET,HARD_CAP")] {
        let (f, rows) = m2_lint_tokens(&m2_lint_label_stream(n)?).map_err(|e| format!("M2 lint label-cap fixture {}: {}", n, e))?;
        if rows.first().map(|r| r.labels) != Some(n) || m2_rule_set(&f) != want {
            return Err(format!("M2 lint asm label cap: {} labels must give [{}] (rows {:?}), got {:?}", n, want, rows, f));
        }
        checks += 1;
    }
    // Frontend-compiled functions: 15 labels (5 while loops) and 16 labels (C4) lint as BUDGET only;
    // 17+ surface labels are rejected by the frontend with BUDGET_EXCEEDED.
    let five_whiles = M2_LINT_FIXTURES.iter().find(|x| x.0 == "neg_budget_labels").map(|x| x.1).unwrap_or("");
    let (_, rows) = m2_lint_surface(five_whiles)?;
    if rows.iter().map(|r| r.labels).max() != Some(15) {
        return Err(format!("M2 lint: 5 while loops should use 15 frontend labels, got {:?}", rows));
    }
    let (f16, rows16) = m2_lint_surface(LBL16_SURFACE_SRC)?;
    if rows16.iter().map(|r| r.labels).max() != Some(M2_LABEL_CAP_FRONTEND) || m2_rule_set(&f16) != "BUDGET" {
        return Err(format!("M2 lint: 16-label surface fn after C4 should give BUDGET only, got {:?} / {:?}", rows16, f16));
    }
    let six_whiles = format!("{}\n", five_whiles).replace(
        "    return acc;",
        "    let i5: i64 = 0;\n    while i5 < n { acc = acc + 6; i5 = i5 + 1; }\n    return acc;",
    );
    match m2_lint_surface(&six_whiles) {
        Err(e) if e.contains("BUDGET_EXCEEDED") => {}
        other => return Err(format!("M2 lint: frontend must reject >16 surface labels, got {:?}", other.map(|x| x.1))),
    }
    checks += 2;
    // Lint gate: every registered pure/m2 module must be clean.
    for &(name, src) in M2_PURE_MODULES {
        let (f, _, _) = m2_lint_surface_scoped(src).map_err(|e| format!("M2 lint gate: {} failed to compile: {}", name, e))?;
        if !f.is_empty() {
            return Err(format!("M2 lint gate: {} has findings {:?}", name, f));
        }
        checks += 1;
    }
    Ok(checks)
}
