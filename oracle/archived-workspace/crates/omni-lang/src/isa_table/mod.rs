//! M3-03 shared ISA table drift lock: `pure/isa/table.omni` vs `omni_isa::OMNI_ISA_TABLE` and the
//! `omni_assembler::encode_asm_ins_packet_u16` mnemonic map (`omni-assembler/src/lib.rs:29-97`, `:146-191`).

use omni_isa::*;
use omni_ir::*;
use omni_verifier::*;
use omni_vm::*;
use omni_assembler::*;
use crate::compiler::*;

pub const OMNI_ISA_TABLE_SURFACE: &str = include_str!("../../../../../../pure/isa/table.omni");
pub const OMNI_ISA_TABLE_SELFTEST_SURFACE: &str = include_str!("../../../../../../pure/isa/table_selftest.omni");
const OMNI_ISA_TABLE_ASM_SCAN_SRC: &str = include_str!("../../../omni-assembler/src/lib.rs");

pub const OMNI_ISA_MNEMONIC_ID_GOLDEN: [&str; 81] = [
    "CINT", "MOV", "CSTR", "CTRIT", "ARENA", "ANEW", "IGET", "ISET", "SLEN",
    "SCAT", "SCONC", "S2I", "I2S", "IADD", "ISUB", "IMUL", "IDIV", "IMOD",
    "INEG", "IABS", "IMIN", "IMAX", "BAND", "BOR", "BXOR", "BNOT", "SHL",
    "SHR", "CEQ", "CNE", "CLT", "CGT", "TNOT", "TAND", "TOR", "TXOR",
    "TMAJ", "TCMP", "PACK5", "TPACK5", "UNPACK5", "TUNPACK5", "TDOT", "EGATE", "LOOKAHEAD",
    "TLOGIC", "SEL_TRIT", "CMOV", "CALL", "CALL_I64", "CALL_TRIT", "CALL_BYTES", "CALL_TRIT_VEC", "CALL_STR",
    "BORROW_R", "BORROW_W", "MOVE", "SAN_SQL", "SAN_DOM", "SAN_PRM", "FNV1A", "DB_GET", "KV_GET",
    "DB_QRY", "DB_CNT", "KV_PUT", "DB_PUT", "ROUTE", "HTTP_SRV", "REQ_PAR", "RSP_JSON", "RSP_HTML",
    "AUTH_VER", "NOP", "DROP", "RET", "LABEL", "JMP", "BR_POS", "BR_NEG", "BR_TRIT",
];

/// Mnemonic-id freeze (M3 intrinsic table stores these ids): ids 0..80 are pinned to
/// OMNI_ISA_MNEMONIC_ID_GOLDEN; new mnemonics may only be appended (ids >= 81), never duplicated.
pub fn check_isa_mnemonic_id_freeze(names: &[String]) -> Result<(), String> {
    let g = &OMNI_ISA_MNEMONIC_ID_GOLDEN;
    for (id, want) in g.iter().enumerate() {
        match names.get(id) {
            Some(got) if got == want => {}
            got => {
                return Err(format!(
                    "error[LOCKSTEP_MISMATCH] line 1, col 1: isa-table mnemonic id {id} frozen as '{want}' but found {:?} (ids are append-only)",
                    got
                ))
            }
        }
    }
    for (id, n) in names.iter().enumerate().skip(g.len()) {
        if names[..id].contains(n) {
            return Err(format!("error[LOCKSTEP_MISMATCH] line 1, col 1: isa-table appended mnemonic id {id} '{n}' duplicates an existing id"));
        }
    }
    Ok(())
}

/// Rust-side image of one `pure/isa/table.omni` mnemonic row: (name, opcode, ty byte).
fn isa_table_rust_mnemonic_rows() -> Result<Vec<(String, u16, u16)>, String> {
    let src = OMNI_ISA_TABLE_ASM_SCAN_SRC;
    let a = src
        .find("let parse_op_mnemonic = |m: &str, current_ret_ty: u16|")
        .ok_or("isa-table: parse_op_mnemonic not found in assembler source")?;
    let b = a + src[a..]
        .find("other => Err(format!(\"Unknown assembler mnemonic")
        .ok_or("isa-table: end of parse_op_mnemonic not found")?;
    let c = b + src[b..]
        .find("Cannot parse assembler line")
        .ok_or("isa-table: end of encode_asm_ins_packet_u16 not found")?;
    let quoted = |s: &str| -> Vec<String> {
        let mut v = Vec::new();
        let mut it = s.split('"');
        it.next();
        while let (Some(q), Some(_)) = (it.next(), it.next()) {
            v.push(String::from(q));
        }
        v
    };
    let mut rows: Vec<(String, u16, u16)> = Vec::new();
    for name in quoted(&src[a..b]) {
        let probe = |ret: u16| encode_asm_ins_packet_u16(&format!("%0 = {} 1", name), ret);
        let p1 = probe(TY_TRIT)?;
        let p2 = probe(TY_BYTES)?;
        let want_tail = [pack_tc(false, CAP_ID_PURE), TOK_IMM_I16, 1, TOK_INS_END];
        if p1.len() != 7 || p1[0] != REG_BASE || p1[3..] != want_tail || p1[1] != p2[1] {
            return Err(format!("isa-table: unexpected packet shape for '{}': {:04X?}", name, p1));
        }
        let ty = if p1[2] != p2[2] && p1[2] == TY_TRIT && p2[2] == TY_BYTES { 255 } else { p1[2] - TY_VOID };
        rows.push((name, p1[1], ty));
    }
    let mut tail = &src[b..c];
    while let Some(k) = tail.find("parts[0] == \"") {
        tail = &tail[k + 13..];
        let e = tail.find('"').ok_or("isa-table: unterminated structural mnemonic")?;
        let name = String::from(&tail[..e]);
        let shapes: [(u16, String, Vec<u16>); 5] = [
            (240, name.clone(), vec![]),
            (241, format!("{} %3", name), vec![REG_BASE + 3]),
            (242, format!("{} 7", name), vec![LBL_BASE + 7]),
            (243, format!("{} %3 7", name), vec![REG_BASE + 3, LBL_BASE + 7]),
            (244, format!("{} %3 7 8 9", name), vec![REG_BASE + 3, LBL_BASE + 7, LBL_BASE + 8, LBL_BASE + 9]),
        ];
        let mut found: Option<(u16, u16)> = None;
        for (shape, line, ops) in shapes.iter().rev() {
            if let Ok(p) = encode_asm_ins_packet_u16(line, TY_I64) {
                if p.len() == ops.len() + 2 && p[1..p.len() - 1] == ops[..] && p[p.len() - 1] == TOK_INS_END {
                    found = Some((p[0], *shape));
                    break;
                }
            }
        }
        let (op, shape) = found.ok_or_else(|| format!("isa-table: no operand shape fits structural mnemonic '{}'", name))?;
        rows.push((name, op, shape));
    }
    Ok(rows)
}

fn isa_table_rd_i64(buf: &[u8], word: usize) -> Result<i64, String> {
    let s = buf.get(word * 8..word * 8 + 8).ok_or("isa-table: Omni dump truncated")?;
    let mut a = [0u8; 8];
    a.copy_from_slice(s);
    Ok(i64::from_le_bytes(a))
}

/// Compares the Omni `pure/isa/table.omni` dump against the Rust ISA table and assembler mnemonic map entry by entry.
pub fn compare_isa_table_dump(dump: &[u8], mn: &[(String, u16, u16)]) -> Result<usize, String> {
    let bad = |what: String, rust: i64, omni: i64| -> String {
        format!("error[LOCKSTEP_MISMATCH] line 1, col 1: isa-table {what}: rust={rust:#X} omni={omni:#X}")
    };
    let n_mn = mn.len();
    const HDR: usize = 12;
    // M3 band (design §3): rodata slot rel 0x01000-0x01FFF; mnemonics @+0, opcodes @+16*n_mn, intrinsics after, >= 110 x 16 B.
    let op_off = 16 * n_mn;
    let tbl_len = op_off + 128 * 8;
    let intr_rel = 0x1000 + tbl_len;
    if (0x2000 - intr_rel) / 16 < 110 {
        return Err(bad(String::from("intrinsic room in rel 0x01000-0x01FFF"), 110, ((0x2000 - intr_rel) / 16) as i64));
    }
    let words = HDR + 128 * 4 + n_mn * 6;
    let raw = dump.get(words * 8..).ok_or("isa-table: Omni dump truncated")?;
    let hdr = [128i64, n_mn as i64, -1, -1, 0x80000, 0x1000, op_off as i64, tbl_len as i64, intr_rel as i64, 0x2000, intr_rel as i64, 0x1000];
    for (k, &h) in hdr.iter().enumerate() {
        let v = isa_table_rd_i64(dump, k)?;
        if v != h {
            return Err(bad(format!("header[{k}]"), h, v));
        }
    }
    if raw.len() != tbl_len {
        return Err(bad(String::from("rodata bytes"), tbl_len as i64, raw.len() as i64));
    }
    let mut cmp = 0usize;
    for (i, spec) in OMNI_ISA_TABLE.iter().enumerate() {
        let flags = (spec.has_dest_reg as i64)
            | ((spec.sink_requires_safe as i64) << 1)
            | ((spec.is_sanitizer as i64) << 2)
            | ((is_x86_64_aot_supported_opcode(spec.token_id) as i64) << 3)
            | ((is_wasm_aot_supported_opcode(spec.token_id) as i64) << 4);
        let tok = (OP_BASE as usize + i) as i64;
        let want = [spec.token_id as i64, spec.req_cap_mask as i64, flags, i as i64];
        let r = &raw[op_off + i * 8..op_off + i * 8 + 8];
        let rawv = [
            r[0] as i64 | ((r[1] as i64) << 8),
            r[2] as i64 | ((r[3] as i64) << 8),
            r[4] as i64 | ((r[5] as i64) << 8) | ((r[6] as i64) << 16) | ((r[7] as i64) << 24),
            i as i64,
        ];
        if spec.token_id as i64 != tok {
            return Err(bad(format!("opcode[{i}] renumbered"), tok, spec.token_id as i64));
        }
        for f in 0..4 {
            let v = isa_table_rd_i64(dump, HDR + i * 4 + f)?;
            if v != want[f] || rawv[f] != want[f] {
                return Err(bad(format!("opcode[{tok:#06X}] field {f}"), want[f], if v != want[f] { v } else { rawv[f] }));
            }
            cmp += 1;
        }
    }
    let base = HDR + 128 * 4;
    for (m, (name, op, tyb)) in mn.iter().enumerate() {
        let r = &raw[m * 16..m * 16 + 16];
        let nb = name.as_bytes();
        let mut want_name = [0u8; 13];
        if nb.len() > 13 {
            return Err(bad(format!("mnemonic[{m}] '{name}' longer than 13 bytes"), 13, nb.len() as i64));
        }
        want_name[..nb.len()].copy_from_slice(nb);
        if r[3..16] != want_name[..] || r[2] as usize != nb.len() {
            return Err(format!(
                "error[LOCKSTEP_MISMATCH] line 1, col 1: isa-table mnemonic[{m}] name: rust='{name}' omni='{}'",
                String::from_utf8_lossy(&r[3..3 + (r[2] as usize).min(13)])
            ));
        }
        let ty_fixed = if *tyb < 240 { TY_VOID as i64 + *tyb as i64 } else { 0 };
        let ty_t = if *tyb == 255 { TY_TRIT as i64 } else { ty_fixed };
        let ty_b = if *tyb == 255 { TY_BYTES as i64 } else { ty_fixed };
        let shape = if (240..255).contains(tyb) { *tyb as i64 - 239 } else { 0 };
        let want = [*op as i64, ty_t, ty_b, shape, nb.len() as i64, m as i64];
        let rawv = [OP_BASE as i64 + r[0] as i64, *tyb as i64, r[1] as i64];
        if rawv[1] != rawv[2] || rawv[0] != want[0] {
            return Err(bad(format!("mnemonic[{m}] '{name}' rodata op/ty"), (*op as i64) << 8 | *tyb as i64, rawv[0] << 8 | rawv[2]));
        }
        for f in 0..6 {
            let v = isa_table_rd_i64(dump, base + m * 6 + f)?;
            if v != want[f] {
                return Err(bad(format!("mnemonic[{m}] '{name}' field {f}"), want[f], v));
            }
            cmp += 1;
        }
    }
    if dump.len() != words * 8 + raw.len() {
        return Err(bad(String::from("dump length"), (words * 8 + raw.len()) as i64, dump.len() as i64));
    }
    Ok(cmp)
}

/// M3-03 drift lock: `pure/isa/table.omni` (run on the VM) == `OMNI_ISA_TABLE` + assembler mnemonic map.
/// Also asserts every table function stays within the M2 lint target (<= 12 labels, <= 120 SSA).
/// Returns (positive comparisons, negative injected-drift checks).
pub fn run_isa_table_drift_selftests() -> Result<(usize, usize), String> {
    let src = format!("{}\n{}", OMNI_ISA_TABLE_SURFACE, OMNI_ISA_TABLE_SELFTEST_SURFACE);
    let stream = compile_omni_surface_strict_to_u16(&src)?;
    let module = decode_omni_token_stream(&stream)?;
    for f in &module.functions {
        let rep = verify_binary_function(f);
        if !rep.passed {
            return Err(format!("isa-table fn #{} failed Rust verification: {:?}", f.fn_idx, rep.errors));
        }
        let ssa = f.instructions.iter().filter_map(|i| i.dest_reg).map(|r| r - REG_BASE + 1).max().unwrap_or(0);
        let lbl = f.instructions.iter().filter(|i| i.op_tok == 0x7F40).count();
        if ssa > 120 || lbl > 12 {
            return Err(format!("isa-table fn #{} exceeds lint target: ssa={} labels={}", f.fn_idx, ssa, lbl));
        }
    }
    let mut vm = OmniBinaryVm::new();
    let dump = match vm.execute_function(&module, 0, vec![OmniValue::Int(0)])? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("isa-table: Omni selftest returned non-bytes {:?}", other)),
    };
    let mn = isa_table_rust_mnemonic_rows()?;
    let mn_names: Vec<String> = mn.iter().map(|r| r.0.clone()).collect();
    check_isa_mnemonic_id_freeze(&mn_names)?;
    let n = compare_isa_table_dump(&dump, &mn)?;
    let mut neg = 0usize;
    let base = (12 + 128 * 4 + mn.len() * 6) * 8;
    let op_raw = base + 16 * mn.len();
    for off in [op_raw + 5 * 8 + 2, base + 47 * 16 + 1, base + 80 * 16 + 9, 5 * 8] {
        let mut d = dump.clone();
        d[off] ^= 1;
        match compare_isa_table_dump(&d, &mn) {
            Err(e) if e.starts_with("error[LOCKSTEP_MISMATCH]") => neg += 1,
            other => return Err(format!("isa-table: injected drift at byte {off} not caught: {:?}", other)),
        }
    }
    let mut mn_bad = mn.clone();
    mn_bad[0].1 = 0x7F07;
    match compare_isa_table_dump(&dump, &mn_bad) {
        Err(e) if e.starts_with("error[LOCKSTEP_MISMATCH]") => neg += 1,
        other => return Err(format!("isa-table: Rust-side drift not caught: {:?}", other)),
    }
    let mut swap = mn_names.clone();
    swap.swap(0, 1);
    let mut removed = mn_names.clone();
    removed.remove(40);
    let mut inserted = mn_names.clone();
    inserted.insert(10, String::from("ZZ_NEW"));
    for (what, v) in [("swap", swap), ("removal", removed), ("mid-insertion", inserted)] {
        match check_isa_mnemonic_id_freeze(&v) {
            Err(e) if e.starts_with("error[LOCKSTEP_MISMATCH]") => neg += 1,
            other => return Err(format!("isa-table: mnemonic-id {what} not caught: {:?}", other)),
        }
    }
    let mut appended = mn_names.clone();
    appended.push(String::from("ZZ_NEW"));
    check_isa_mnemonic_id_freeze(&appended)
        .map_err(|e| format!("isa-table: mnemonic-id append (id 81) wrongly rejected: {e}"))?;
    let mut dup = appended.clone();
    dup.push(String::from("CINT"));
    match check_isa_mnemonic_id_freeze(&dup) {
        Err(e) if e.starts_with("error[LOCKSTEP_MISMATCH]") => neg += 1,
        other => return Err(format!("isa-table: duplicated appended mnemonic not caught: {:?}", other)),
    }
    Ok((n, neg))
}
