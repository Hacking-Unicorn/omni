use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};

#[allow(dead_code, unused_imports)]
#[path = "../../../omni_rust_bootstrap_verified.rs"]
mod monolith;

use omni_aot::{
    build_step7_stage1_compiler_stream, compile_pure_u16_to_wasm,
    compile_pure_u16_to_x86_64_elf_with_args, execute_wasm_module_i64, fnv1a_64_hash,
};
use omni_assembler::{
    assemble_omni_text_to_u16, build_custom_branch_loop_program, build_custom_non_seed_alu_program,
};
use omni_ir::{
    decode_bytes_to_u16_stream, decode_omni_token_stream, encode_u16_stream_to_bytes,
    format_non_human_token_stream, pack_fn_module,
};
use omni_isa::*;
use omni_lang::{
    compile_omni_surface_file_to_u16, compile_omni_surface_to_asm, compile_omni_surface_to_u16,
    compile_omni_surface_to_wasm, compile_omni_surface_to_x86_64_elf,
    compile_omni_surface_via_stage1_to_elf, execute_omni_surface_aot_first,
    execute_omni_surface_program, execute_omni_surface_program_with_budgets,
    execute_omni_surface_wasm_i64, parse_cli_omni_value, resolve_omni_surface_includes,
};
use omni_verifier::{format_debug_lens, verify_binary_function, verify_u16_stream};
use omni_vm::{OmniBinaryVm, OmniValue};

fn to_ws_val(v: &monolith::OmniValue) -> OmniValue {
    match v {
        monolith::OmniValue::Void => OmniValue::Void,
        monolith::OmniValue::Int(n) => OmniValue::Int(*n),
        monolith::OmniValue::Trit(t) => OmniValue::Trit(*t),
        monolith::OmniValue::Str(s) => OmniValue::Str(s.clone()),
        monolith::OmniValue::Bytes(b) => OmniValue::Bytes(b.clone()),
        monolith::OmniValue::U16Array(a) => OmniValue::U16Array(a.clone()),
        monolith::OmniValue::TritVec(tv) => OmniValue::TritVec(tv.clone()),
        monolith::OmniValue::Struct(m) => {
            OmniValue::Struct(m.iter().map(|(&k, val)| (k, to_ws_val(val))).collect())
        }
        monolith::OmniValue::Arena { capacity, active } => OmniValue::Arena {
            capacity: *capacity,
            active: *active,
        },
        monolith::OmniValue::SkillModel { skill_id, active } => OmniValue::SkillModel {
            skill_id: *skill_id,
            active: *active,
        },
        monolith::OmniValue::DomNode { tag_id, children } => OmniValue::DomNode {
            tag_id: *tag_id,
            children: children.iter().map(to_ws_val).collect(),
        },
    }
}

fn assert_verifier_clone_eq(stream: &[u16], expected_passed: bool, label: &str) -> Result<(), String> {
    let m_mod = monolith::decode_omni_token_stream(stream)
        .map_err(|e| format!("monolith decode failed on {}: {}", label, e))?;
    let w_mod = decode_omni_token_stream(stream)
        .map_err(|e| format!("workspace decode failed on {}: {}", label, e))?;
    let m_rep = monolith::verify_binary_function(&m_mod.functions[0]);
    let w_rep = verify_binary_function(&w_mod.functions[0]);
    if m_rep.passed != expected_passed {
        return Err(format!(
            "Monolith verifier unexpected result on {}: passed={} != {}",
            label, m_rep.passed, expected_passed
        ));
    }
    if w_rep.passed != m_rep.passed || w_rep.errors != m_rep.errors || w_rep.instruction_count != m_rep.instruction_count {
        return Err(format!(
            "Verifier mismatch on {}: monolith=({}, {:?}) vs workspace=({}, {:?})",
            label, m_rep.passed, m_rep.errors, w_rep.passed, w_rep.errors
        ));
    }
    Ok(())
}

fn assert_golden_monolith_vs_workspace(
    asm: &str,
    inputs: &[i64],
    expected: i64,
    stage1_mod: &omni_ir::BinModule,
    label: &str,
) -> Result<(), String> {
    let m_u16 = monolith::assemble_omni_text_to_u16(asm)
        .map_err(|e| format!("monolith asm failed on {}: {}", label, e))?;
    let w_u16 = assemble_omni_text_to_u16(asm)
        .map_err(|e| format!("workspace asm failed on {}: {}", label, e))?;
    if m_u16 != w_u16 {
        return Err(format!("Assembler mismatch between monolith and workspace on {}", label));
    }
    assert_verifier_clone_eq(&w_u16, true, label)?;

    let m_mod = monolith::decode_omni_token_stream(&m_u16)?;
    let w_mod = decode_omni_token_stream(&w_u16)?;
    let m_args: Vec<monolith::OmniValue> = inputs.iter().map(|&x| monolith::OmniValue::Int(x)).collect();
    let w_args: Vec<OmniValue> = inputs.iter().map(|&x| OmniValue::Int(x)).collect();

    let mut m_vm = monolith::OmniBinaryVm::new();
    let mut w_vm = OmniBinaryVm::new();
    let m_res = m_vm.execute_function(&m_mod, 0, m_args)?;
    let w_res = w_vm.execute_function(&w_mod, 0, w_args)?;
    let m_as_ws = to_ws_val(&m_res);
    if w_res != m_as_ws {
        return Err(format!("VM mismatch between monolith and workspace on {}: {:?} != {:?}", label, w_res, m_as_ws));
    }
    let got_i64 = match w_res {
        OmniValue::Int(v) => v,
        OmniValue::Trit(t) => t as i64,
        other => return Err(format!("Unexpected return type on {}: {:?}", label, other)),
    };
    if got_i64 != expected {
        return Err(format!("Golden value mismatch on {}: got {}, expected {}", label, got_i64, expected));
    }

    let a0 = inputs.first().copied().unwrap_or(0);
    let a1 = inputs.get(1).copied().unwrap_or(0);
    let a2 = inputs.get(2).copied().unwrap_or(0);
    let m_elf = monolith::compile_pure_u16_to_x86_64_elf_with_args(&m_u16, a0, a1, a2)?;
    let w_elf = compile_pure_u16_to_x86_64_elf_with_args(&w_u16, a0, a1, a2)?;
    if m_elf != w_elf {
        return Err(format!("x86-64 ELF mismatch between monolith and workspace on {}", label));
    }

    let s1_elf = match w_vm.execute_function(
        stage1_mod,
        0,
        vec![OmniValue::U16Array(w_u16.clone()), OmniValue::Int(a0), OmniValue::Int(a1), OmniValue::Int(a2)],
    )? {
        OmniValue::Bytes(b) => b,
        _ => return Err(format!("Stage-1 did not return Bytes on {}", label)),
    };
    if s1_elf != w_elf {
        return Err(format!("Stage-1 vs AOT ELF mismatch on {}", label));
    }

    let m_wasm = monolith::compile_pure_u16_to_wasm(&m_u16)?;
    let w_wasm = compile_pure_u16_to_wasm(&w_u16)?;
    if m_wasm != w_wasm {
        return Err(format!("Wasm binary mismatch between monolith and workspace on {}", label));
    }
    let m_wasm_val = monolith::execute_wasm_module_i64(&m_wasm, inputs)?;
    let w_wasm_val = execute_wasm_module_i64(&w_wasm, inputs)?;
    if m_wasm_val != w_wasm_val || w_wasm_val != expected {
        return Err(format!("Wasm execution mismatch on {}: m={} w={} exp={}", label, m_wasm_val, w_wasm_val, expected));
    }

    Ok(())
}

pub fn run_workspace_selftest() -> Result<usize, String> {
    let mut checks = 0usize;
    let tc_safe = pack_tc(false, CAP_ID_PURE);
    let tc_untr = pack_tc(true, CAP_ID_PURE);

    // 1. Pack5 round-trip + sentinel check (Monolith vs Workspace)
    let trits: Vec<i8> = (0..64).map(|i| ((i * 5 + 1) % 3) as i8 - 1).collect();
    let m_packed = monolith::pack5_encode(&trits);
    let w_packed = pack5_encode(&trits);
    let unpacked = pack5_decode(&w_packed, trits.len()).map_err(|e| e.to_string())?;
    if m_packed != w_packed || unpacked != trits || pack5_decode(&[SENTINEL_SPECIAL_TOKEN], 5).is_ok() {
        return Err("Workspace pack5 differential check failed".into());
    }
    checks += 2;

    // 2. Complete Verifier Clone Differential (Item 3: valid and invalid functions fed to BOTH verifiers)
    let bad_taint = pack_fn_module(
        &[(0x0000, vec![(TY_STR, tc_safe)])],
        0,
        &[(0x7EC0, TY_STR, tc_untr)],
        TY_STRUCT,
        cap_bit(CAP_ID_DB_W),
        &[
            vec![0x7E00, 0x7F63, TY_STRUCT, pack_tc(false, CAP_ID_DB_W), TOK_SYM_SCHEMA, 0x0000, TOK_FIELD_KV, 0x0000, 0x7EC0, TOK_INS_END],
            vec![0x7F4A, 0x7E00, TOK_INS_END],
        ],
    );
    assert_verifier_clone_eq(&bad_taint, false, "bad_taint")?;

    let bad_cap = pack_fn_module(
        &[(0x0000, vec![(TY_STR, tc_safe)])],
        0,
        &[(0x7EC0, TY_STR, tc_safe)],
        TY_STRUCT,
        0x0000,
        &[
            vec![0x7E00, 0x7F63, TY_STRUCT, pack_tc(false, CAP_ID_DB_W), TOK_SYM_SCHEMA, 0x0000, TOK_FIELD_KV, 0x0000, 0x7EC0, TOK_INS_END],
            vec![0x7F4A, 0x7E00, TOK_INS_END],
        ],
    );
    assert_verifier_clone_eq(&bad_cap, false, "bad_cap")?;

    let bad_leak = pack_fn_module(
        &[],
        0,
        &[(0x7EC0, TY_I64, tc_safe)],
        TY_I64,
        0x0000,
        &[
            vec![0x7E00, 0x7F01, TY_ARENA, tc_safe, TOK_IMM_I32, 0x0100, 0x0000, TOK_INS_END],
            vec![0x7F4A, 0x7EC0, TOK_INS_END],
        ],
    );
    assert_verifier_clone_eq(&bad_leak, false, "bad_leak")?;

    let bad_ssa = pack_fn_module(
        &[],
        0,
        &[(0x7EC0, TY_I64, tc_safe)],
        TY_I64,
        0x0000,
        &[
            vec![0x7E05, 0x7F06, TY_I64, tc_safe, 0x7EC0, TOK_INS_END],
            vec![0x7F4A, 0x7E05, TOK_INS_END],
        ],
    );
    assert_verifier_clone_eq(&bad_ssa, false, "bad_ssa")?;

    let bad_op = pack_fn_module(
        &[],
        0,
        &[(0x7EC0, TY_I64, tc_safe)],
        TY_I64,
        0x0000,
        &[
            vec![0x7E00, 0x7F99, TY_I64, tc_safe, 0x7EC0, TOK_INS_END],
            vec![0x7F4A, 0x7E00, TOK_INS_END],
        ],
    );
    assert_verifier_clone_eq(&bad_op, false, "bad_op")?;

    let bad_lbl = assemble_omni_text_to_u16("PARAM $0 I64 SAFE\nRET_TY I64\nJMP 12\nRET $0\n")?;
    assert_verifier_clone_eq(&bad_lbl, false, "bad_unresolved_label")?;

    // Active reader borrow drop violation (dropping %0 while %1 borrows %0)
    let bad_borrow_drop = pack_fn_module(
        &[],
        0,
        &[(0x7EC0, TY_I64, tc_safe)],
        TY_I64,
        0x0000,
        &[
            vec![0x7E00, 0x7F01, TY_ARENA, tc_safe, TOK_IMM_I32, 0x0100, 0x0000, TOK_INS_END],
            vec![0x7E01, 0x7F02, TY_ARENA, tc_safe, 0x7E00, TOK_INS_END],
            vec![0x7F05, 0x7E00, TOK_INS_END],
            vec![0x7F4A, 0x7EC0, TOK_INS_END],
        ],
    );
    assert_verifier_clone_eq(&bad_borrow_drop, false, "bad_borrow_drop")?;
    checks += 7;

    // 3. Fuel & memory budget differential (Monolith VM vs Workspace VM)
    let inf_u16 = assemble_omni_text_to_u16("PARAM $0 I64 SAFE\nRET_TY I64\nLABEL 1\nJMP 1\nRET $0\n")?;
    let m_inf = monolith::decode_omni_token_stream(&inf_u16)?;
    let w_inf = decode_omni_token_stream(&inf_u16)?;
    let mut m_step_vm = monolith::OmniBinaryVm::with_budgets(64, 1024);
    let mut w_step_vm = OmniBinaryVm::with_budgets(64, 1024);
    if m_step_vm.execute_function(&m_inf, 0, vec![monolith::OmniValue::Int(1)]).is_ok()
        || w_step_vm.execute_function(&w_inf, 0, vec![OmniValue::Int(1)]).is_ok()
    {
        return Err("VM failed to enforce step_limit=64".into());
    }

    let arena_64mb = pack_fn_module(
        &[],
        0,
        &[(0x7EC0, TY_I64, tc_safe)],
        TY_I64,
        0x0000,
        &[
            vec![0x7E00, 0x7F01, TY_ARENA, tc_safe, TOK_IMM_I32, 0x0400, 0x0000, TOK_INS_END],
            vec![0x7F05, 0x7E00, TOK_INS_END],
            vec![0x7F4A, 0x7EC0, TOK_INS_END],
        ],
    );
    assert_verifier_clone_eq(&arena_64mb, true, "arena_64mb")?;
    let m_mod_64mb = monolith::decode_omni_token_stream(&arena_64mb)?;
    let w_mod_64mb = decode_omni_token_stream(&arena_64mb)?;
    if monolith::OmniBinaryVm::new().execute_function(&m_mod_64mb, 0, vec![monolith::OmniValue::Int(77)])?
        != monolith::OmniValue::Int(77)
        || OmniBinaryVm::new().execute_function(&w_mod_64mb, 0, vec![OmniValue::Int(77)])? != OmniValue::Int(77)
    {
        return Err("Default 256 MiB VM failed 64 MiB ARENA execution".into());
    }
    if monolith::OmniBinaryVm::with_budgets(1000, 1024)
        .execute_function(&m_mod_64mb, 0, vec![monolith::OmniValue::Int(77)])
        .is_ok()
        || OmniBinaryVm::with_budgets(1000, 1024)
            .execute_function(&w_mod_64mb, 0, vec![OmniValue::Int(77)])
            .is_ok()
    {
        return Err("VM failed to reject 64 MiB ARENA under 1024-byte budget".into());
    }
    checks += 3;

    // 4. Full Golden Set Differential across Monolith & Workspace (Items 2, 4, 5)
    let stage1_u16 = build_step7_stage1_compiler_stream();
    let m_stage1_u16 = monolith::build_step7_stage1_compiler_stream();
    if stage1_u16 != m_stage1_u16 {
        return Err("Stage-1 compiler stream mismatch between monolith and workspace".into());
    }
    assert_verifier_clone_eq(&stage1_u16, true, "stage1_self_compiler")?;
    let stage1_mod = decode_omni_token_stream(&stage1_u16)?;

    let goldens: &[(&str, &[i64], i64, &str)] = &[
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = IABS $0\nRET %0\n", &[-42], 42, "IABS_-42"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = IABS $0\nRET %0\n", &[17], 17, "IABS_17"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = IABS $0\nRET %0\n", &[0], 0, "IABS_0"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = IABS $0\nRET %0\n", &[i64::MIN], i64::MIN, "IABS_MIN"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = IDIV $0 0\nRET %0\n", &[42], 0, "IDIV_0"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = IMOD $0 0\nRET %0\n", &[99], 0, "IMOD_0"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = IDIV $0 -1\nRET %0\n", &[i64::MIN], i64::MIN, "IDIV_MIN_NEG1"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = IMOD $0 -1\nRET %0\n", &[i64::MIN], 0, "IMOD_MIN_NEG1"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = SHL $0 65\nRET %0\n", &[1], 2, "SHL_65"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = SEL_TRIT $0 11 22 33\nRET %0\n", &[-2], 11, "SEL_EDGE_NEG2"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = SEL_TRIT $0 11 22 33\nRET %0\n", &[0], 22, "SEL_EDGE_0"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = SEL_TRIT $0 11 22 33\nRET %0\n", &[100], 33, "SEL_EDGE_100"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 10\n%1 = SEL_TRIT %0 11 22 33\nRET %1\n", &[-5], 11, "SEL_TRIT_NEG"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 10\n%1 = SEL_TRIT %0 11 22 33\nRET %1\n", &[10], 22, "SEL_TRIT_ZERO"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 10\n%1 = SEL_TRIT %0 11 22 33\nRET %1\n", &[25], 33, "SEL_TRIT_POS"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = IMUL $0 3\n%1 = TCMP %0 20\n%2 = SEL_TRIT %1 5 10 15\n%3 = IADD %0 %2\nRET %3\n", &[3], 14, "SEL_MIX_3"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = IMUL $0 3\n%1 = TCMP %0 20\n%2 = SEL_TRIT %1 5 10 15\n%3 = IADD %0 %2\nRET %3\n", &[7], 36, "SEL_MIX_7"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 0\nBR_POS %0 1\n%1 = CINT 100\nJMP 2\nLABEL 1\n%1 = CINT 200\nLABEL 2\nRET %1\n", &[5], 200, "BR_POS_5"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 0\nBR_NEG %0 1\n%1 = CINT 50\nJMP 2\nLABEL 1\n%1 = CINT 75\nLABEL 2\nRET %1\n", &[-7], 75, "BR_NEG_NEG7"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 0\nBR_NEG %0 1\n%1 = IADD $0 100\nJMP 2\nLABEL 1\n%1 = ISUB $0 50\nLABEL 2\nRET %1\n", &[-10], -60, "BR_NEG_ARITH_NEG10"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 0\nBR_NEG %0 1\n%1 = IADD $0 100\nJMP 2\nLABEL 1\n%1 = ISUB $0 50\nLABEL 2\nRET %1\n", &[10], 110, "BR_NEG_ARITH_10"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 0\nBR_POS %0 1\n%1 = CINT 10\nJMP 3\nLABEL 1\n%2 = TCMP $0 5\nBR_POS %2 2\n%1 = CINT 20\nJMP 3\nLABEL 2\n%1 = CINT 30\nLABEL 3\nRET %1\n", &[-1], 10, "NESTED_NEG1"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 0\nBR_POS %0 1\n%1 = CINT 10\nJMP 3\nLABEL 1\n%2 = TCMP $0 5\nBR_POS %2 2\n%1 = CINT 20\nJMP 3\nLABEL 2\n%1 = CINT 30\nLABEL 3\nRET %1\n", &[3], 20, "NESTED_3"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 0\nBR_POS %0 1\n%1 = CINT 10\nJMP 3\nLABEL 1\n%2 = TCMP $0 5\nBR_POS %2 2\n%1 = CINT 20\nJMP 3\nLABEL 2\n%1 = CINT 30\nLABEL 3\nRET %1\n", &[8], 30, "NESTED_8"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 10\nBR_TRIT %0 1 2 3\nLABEL 1\n%1 = ISUB $0 5\nJMP 4\nLABEL 2\n%1 = CINT 100\nJMP 4\nLABEL 3\n%1 = IADD $0 50\nLABEL 4\nRET %1\n", &[4], -1, "BR_TRIT_4"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 10\nBR_TRIT %0 1 2 3\nLABEL 1\n%1 = ISUB $0 5\nJMP 4\nLABEL 2\n%1 = CINT 100\nJMP 4\nLABEL 3\n%1 = IADD $0 50\nLABEL 4\nRET %1\n", &[10], 100, "BR_TRIT_10"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 10\nBR_TRIT %0 1 2 3\nLABEL 1\n%1 = ISUB $0 5\nJMP 4\nLABEL 2\n%1 = CINT 100\nJMP 4\nLABEL 3\n%1 = IADD $0 50\nLABEL 4\nRET %1\n", &[20], 70, "BR_TRIT_20"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = CINT 0\n%1 = CINT $0\nLABEL 1\n%0 = IADD %0 %1\n%1 = ISUB %1 1\n%2 = TCMP %1 0\nBR_POS %2 1\nRET %0\n", &[10], 55, "BRANCH_LOOP_10"),
        ("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = ANEW 0\n%0 = ISET %0 10 20 30\n%0 = ISET %0 -4 100\n%0 = ISET %0 -8 200\n%1 = SLEN %0\n%2 = TCMP $0 5\n%3 = CMOV %2 100 200 300\n%4 = IADD %1 %3\nRET %4\n", &[10], 315, "BYTE_BUF_315"),
        ("PARAM $0 I64 SAFE\nPARAM $1 I64 SAFE\nRET_TY TRIT\n%0 = IMUL $0 3\n%1 = ISUB %0 $1\n%2 = TCMP %1 75\nRET %2\n", &[30, 10], 1, "ASM_ROUNDTRIP"),
        ("PARAM $0 I64 SAFE\nPARAM $1 I64 SAFE\nPARAM $2 I64 SAFE\nRET_TY I64\n%0 = IMUL $0 4\n%1 = ISUB %0 $1\n%2 = SHL $2 1\n%3 = IADD %1 %2\n%4 = IDIV %3 3\n%5 = TCMP %4 35\n%6 = TNOT %5\n%7 = SEL_TRIT %6 25 50 75\n%8 = IADD %4 %7\nRET %8\n", &[30, 15, 9], 66, "CUSTOM_NON_SEED_ALU"),
    ];

    for &(asm, inputs, expected, label) in goldens {
        assert_golden_monolith_vs_workspace(asm, inputs, expected, &stage1_mod, label)?;
        checks += 1;
    }

    // 5. All 64 Corpus Samples Differential (Monolith Verifier + VM vs Workspace Verifier + VM)
    for idx in 0..64usize {
        let (_, stream, m_args, m_expected) = monolith::build_transpiled_seed_stream(idx);
        assert_verifier_clone_eq(&stream, true, &format!("corpus_{}", idx))?;
        let m_mod = monolith::decode_omni_token_stream(&stream)?;
        let w_mod = decode_omni_token_stream(&stream)?;
        let w_args: Vec<OmniValue> = m_args.iter().map(to_ws_val).collect();
        let w_expected = to_ws_val(&m_expected);

        let mut m_vm = monolith::OmniBinaryVm::new();
        let mut w_vm = OmniBinaryVm::new();
        let m_actual = m_vm.execute_function(&m_mod, m_mod.functions[0].fn_idx, m_args)?;
        let w_actual = w_vm.execute_function(&w_mod, w_mod.functions[0].fn_idx, w_args)?;
        if to_ws_val(&m_actual) != w_actual || w_actual != w_expected {
            return Err(format!("Corpus #{} differential mismatch: {:?} vs {:?}", idx, m_actual, w_actual));
        }
        checks += 1;
    }

    // 6. Surface Language (`omni-lang`) Differential (Monolith vs Workspace vs Assembler Vec<u16> Identity)
    let surface_goldens: &[(&str, &str, Vec<u16>)] = &[
        (
            "seed0_risk",
            "fn seed0_risk(base: i64, delta: i64) -> trit { let scaled = base * 2; let score = scaled + delta; let cmp = score <=> 50; let pos = +1t; let res = tand(cmp, pos); return res; }",
            monolith::build_transpiled_seed_stream(0).1,
        ),
        (
            "seed4_consensus",
            "#[fn_id(4)] fn seed4_consensus(s0: trit, s1: trit, s2: trit) -> trit { let m = tmaj(s0, s1, s2); let res = !m; return res; }",
            monolith::build_transpiled_seed_stream(4).1,
        ),
        (
            "seed6_clamp",
            "#[fn_id(6)] fn seed6_clamp(x: i64, lo: i64, hi: i64) -> i64 { let lower = imax(x, lo); let clamped = imin(lower, hi); return clamped; }",
            monolith::build_transpiled_seed_stream(6).1,
        ),
        (
            "custom_non_seed_alu",
            "fn custom_non_seed_alu(a: i64, b: i64, c: i64) -> i64 { let r0 = a * 4; let r1 = r0 - b; let r2 = c << 1; let r3 = r1 + r2; let r4 = r3 / 3; let r5 = r4 <=> 35; let r6 = !r5; let r7 = sel_trit(r6, 25, 50, 75); let r8 = r4 + r7; return r8; }",
            build_custom_non_seed_alu_program()?,
        ),
        (
            "asm_roundtrip",
            "fn asm_roundtrip(a: i64, b: i64) -> trit { let res = (a * 3 - b) <=> 75; return res; }",
            assemble_omni_text_to_u16("PARAM $0 I64 SAFE\nPARAM $1 I64 SAFE\nRET_TY TRIT\n%0 = IMUL $0 3\n%1 = ISUB %0 $1\n%2 = TCMP %1 75\nRET %2\n")?,
        ),
        (
            "iabs_golden",
            "fn iabs_golden(x: i64) -> i64 { let r = iabs(x); return r; }",
            assemble_omni_text_to_u16("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = IABS $0\nRET %0\n")?,
        ),
        (
            "idiv_zero",
            "fn idiv_zero(x: i64) -> i64 { let r = x / 0; return r; }",
            assemble_omni_text_to_u16("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = IDIV $0 0\nRET %0\n")?,
        ),
        (
            "shl_65",
            "fn shl_65(x: i64) -> i64 { let r = x << 65; return r; }",
            assemble_omni_text_to_u16("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = SHL $0 65\nRET %0\n")?,
        ),
        (
            "sel_mix",
            "fn sel_mix(x: i64) -> i64 { let prod = x * 3; let cmp = prod <=> 20; let bonus = sel_trit(cmp, 5, 10, 15); let res = prod + bonus; return res; }",
            assemble_omni_text_to_u16("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = IMUL $0 3\n%1 = TCMP %0 20\n%2 = SEL_TRIT %1 5 10 15\n%3 = IADD %0 %2\nRET %3\n")?,
        ),
        (
            "br_pos_golden",
            "fn br_pos_golden(x: i64) -> i64 { let c = x <=> 0; let r = if_pos c { 200 } else { 100 }; return r; }",
            assemble_omni_text_to_u16("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 0\nBR_POS %0 1\n%1 = CINT 100\nJMP 2\nLABEL 1\n%1 = CINT 200\nLABEL 2\nRET %1\n")?,
        ),
        (
            "br_neg_arith_golden",
            "fn br_neg_arith_golden(x: i64) -> i64 { let c = x <=> 0; let r = if_neg c { x - 50 } else { x + 100 }; return r; }",
            assemble_omni_text_to_u16("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 0\nBR_NEG %0 1\n%1 = IADD $0 100\nJMP 2\nLABEL 1\n%1 = ISUB $0 50\nLABEL 2\nRET %1\n")?,
        ),
        (
            "nested_branch_golden",
            "fn nested_branch_golden(x: i64) -> i64 { let c0 = x <=> 0; let r = if_pos c0 { let c1 = x <=> 5; if_pos c1 { 30 } else { 20 } } else { 10 }; return r; }",
            assemble_omni_text_to_u16("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 0\nBR_POS %0 1\n%1 = CINT 10\nJMP 3\nLABEL 1\n%2 = TCMP $0 5\nBR_POS %2 2\n%1 = CINT 20\nJMP 3\nLABEL 2\n%1 = CINT 30\nLABEL 3\nRET %1\n")?,
        ),
        (
            "match_trit_golden",
            "fn match_trit_golden(x: i64) -> i64 { let c = x <=> 10; let r = match_trit c { neg => x - 5, zero => 100, pos => x + 50 }; return r; }",
            assemble_omni_text_to_u16("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = TCMP $0 10\nBR_TRIT %0 1 2 3\nLABEL 1\n%1 = ISUB $0 5\nJMP 4\nLABEL 2\n%1 = CINT 100\nJMP 4\nLABEL 3\n%1 = IADD $0 50\nLABEL 4\nRET %1\n")?,
        ),
        (
            "branch_loop_golden",
            "fn branch_loop_golden(n: i64) -> i64 { let acc = 0; let i = n; do_while { acc = acc + i; i = i - 1; } i <=> 0; return acc; }",
            build_custom_branch_loop_program()?,
        ),
        (
            "byte_buf_golden",
            "fn byte_buf_golden(x: i64) -> i64 { let buf = anew(0); buf = iset(buf, 10, 20, 30); buf = iset_i32(buf, 100); buf = iset_i64(buf, 200); let len = slen(buf); let cmp = x <=> 5; let sel = cmov(cmp, 100, 200, 300); let total = len + sel; return total; }",
            assemble_omni_text_to_u16("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = ANEW 0\n%0 = ISET %0 10 20 30\n%0 = ISET %0 -4 100\n%0 = ISET %0 -8 200\n%1 = SLEN %0\n%2 = TCMP $0 5\n%3 = CMOV %2 100 200 300\n%4 = IADD %1 %3\nRET %4\n")?,
        ),
    ];

    for &(g_name, surf_src, ref expected_u16) in surface_goldens {
        let m_asm = monolith::compile_omni_surface_to_asm(surf_src)?;
        let w_asm = compile_omni_surface_to_asm(surf_src)?;
        if m_asm != w_asm {
            return Err(format!("Surface ASM mismatch on '{}': monolith != omni-lang", g_name));
        }
        let m_u16 = monolith::compile_omni_surface_to_u16(surf_src)?;
        let w_u16 = compile_omni_surface_to_u16(surf_src)?;
        if m_u16 != w_u16 || &w_u16 != expected_u16 {
            return Err(format!("Surface Vec<u16> mismatch on '{}'", g_name));
        }
        assert_verifier_clone_eq(&w_u16, true, g_name)?;
        checks += 1;
    }

    // 6b. Line & column error diagnostics differential (undefined vars, bad types, bad control flow)
    let bad_surface_cases: &[(&str, &str, &str)] = &[
        ("undefined_var", "fn bad(x: i64) -> i64 {\n    let r = missing_y + 1;\n    return r;\n}", "line 2, col 13: Undefined variable 'missing_y'"),
        ("unknown_intrinsic", "fn bad(x: i64) -> i64 {\n    let r = foo_bar(x);\n    return r;\n}", "line 2, col 13: Unknown surface intrinsic or function 'foo_bar'"),
        ("unknown_type", "fn bad(x: float128) -> i64 {\n    return x;\n}", "line 1, col 11: Unknown surface type 'float128'"),
        ("mutate_param", "fn bad(x: i64) -> i64 {\n    x = x + 1;\n    return x;\n}", "line 2, col 5: Cannot reassign immutable function parameter 'x'"),
        ("bad_control_flow_unresolved_goto", "fn bad(x: i64) -> i64 {\n    goto 9;\n    return x;\n}", "line 2, col 10: Unresolved branch target label 9"),
        ("bad_control_flow_missing_match_arm", "fn bad(x: trit) -> i64 {\n    let r = match_trit x { neg => 1, zero => 2 };\n    return r;\n}", "line 2, col 13: match_trit missing 'pos' arm"),
    ];
    for &(c_name, bad_src, expected_loc) in bad_surface_cases {
        let m_err = match monolith::compile_omni_surface_to_u16(bad_src) {
            Err(e) => e,
            Ok(_) => return Err(format!("Monolith surface compiler failed to reject '{}'", c_name)),
        };
        let w_err = match compile_omni_surface_to_u16(bad_src) {
            Err(e) => e,
            Ok(_) => return Err(format!("Workspace surface compiler failed to reject '{}'", c_name)),
        };
        if m_err != w_err || !w_err.contains(expected_loc) {
            return Err(format!(
                "Surface diagnostic mismatch on '{}': expected '{}', got m='{}' w='{}'",
                c_name, expected_loc, m_err, w_err
            ));
        }
        checks += 1;
    }

    // 6c. Surface-level taint & capability rejection differential
    let surf_untr_ret = "fn bad_untr_ret(x: i64 @untrusted) -> i64 { return x; }";
    let u16_untr_ret = compile_omni_surface_to_u16(surf_untr_ret)?;
    assert_verifier_clone_eq(&u16_untr_ret, false, "surf_untr_ret")?;

    let surf_untr_sink = "#[cap(AI)] fn bad_untr_sink(x: i64 @untrusted) -> trit { let g = egate(x); return g; }";
    let u16_untr_sink = compile_omni_surface_to_u16(surf_untr_sink)?;
    assert_verifier_clone_eq(&u16_untr_sink, false, "surf_untr_sink")?;

    let surf_missing_cap = "fn bad_missing_cap(x: i64) -> trit { let g = egate(x); return g; }";
    let u16_missing_cap = compile_omni_surface_to_u16(surf_missing_cap)?;
    assert_verifier_clone_eq(&u16_missing_cap, false, "surf_missing_cap")?;

    let surf_good_san = "#[cap(AI)] fn good_san(s: str @untrusted) -> str { let clean = san_prm(s); return clean; }";
    let u16_good_san = compile_omni_surface_to_u16(surf_good_san)?;
    assert_verifier_clone_eq(&u16_good_san, true, "surf_good_san")?;
    checks += 4;

    // 6d. Decision hook golden (`egate` / `0x7F35`)
    let surf_decision_hook = "#[cap(AI)] fn decision_hook_golden(a: i64, b: i64) -> trit { let delta = a - b; let gate = egate(delta); return gate; }";
    let u16_decision = compile_omni_surface_to_u16(surf_decision_hook)?;
    assert_verifier_clone_eq(&u16_decision, true, "decision_hook_golden")?;
    if execute_omni_surface_program(surf_decision_hook, vec![OmniValue::Int(30), OmniValue::Int(10)])? != OmniValue::Trit(1)
        || execute_omni_surface_program(surf_decision_hook, vec![OmniValue::Int(5), OmniValue::Int(20)])? != OmniValue::Trit(-1)
        || execute_omni_surface_program(surf_decision_hook, vec![OmniValue::Int(12), OmniValue::Int(12)])? != OmniValue::Trit(0)
    {
        return Err("Decision hook golden execution mismatch".into());
    }
    checks += 1;

    // 6e. Pack5 lowering for trit values golden (10 trits -> 2 @PACK5 bytes vs 80 i64 bytes)
    let surf_pack5 = "
        #[cap(AI)]
        fn pack5_trit_vec_golden() -> i64 {
            let v = [+1t, -1t, 0t, +1t, +1t, -1t, 0t, -1t, +1t, +1t];
            let packed = pack5(v);
            let packed_byte_len = slen(packed);
            let first_pack5_byte = iget(packed, 0);
            let second_pack5_byte = iget(packed, 1);
            let dot = tdot(v, v);
            let gate = egate(v);
            let res = packed_byte_len * 1000 + first_pack5_byte + second_pack5_byte + dot + gate;
            return res;
        }
    ";
    let u16_pack5 = compile_omni_surface_to_u16(surf_pack5)?;
    assert_verifier_clone_eq(&u16_pack5, true, "pack5_trit_vec_golden")?;
    if execute_omni_surface_program(surf_pack5, vec![])? != OmniValue::Int(2455) {
        return Err("Pack5 trit vector golden execution mismatch".into());
    }
    checks += 1;

    // 6f. Multi-function `.omni` program golden (`CALL_I64` / `0x7F48`)
    let surf_multi_fn = "
        fn weighted_term(x: i64, w: i64) -> i64 {
            let p = x * w;
            let s = p + 5;
            return s;
        }

        fn main(a: i64, b: i64) -> i64 {
            let left = weighted_term(a, 3);
            let right = weighted_term(b, 2);
            let total = left + right;
            return total;
        }
    ";
    if execute_omni_surface_program(surf_multi_fn, vec![OmniValue::Int(10), OmniValue::Int(20)])? != OmniValue::Int(80) {
        return Err("Multi-function surface program execution mismatch".into());
    }
    checks += 1;

    // 6g. Module/file `include` linker + circular include detection
    let mut vfs = BTreeMap::new();
    vfs.insert(
        String::from("trit_helper.omni"),
        String::from("fn score_to_trit(v: i64, thresh: i64) -> trit { let c = v <=> thresh; return c; }"),
    );
    let main_with_inc = "
        include \"trit_helper.omni\";
        fn main(x: i64) -> i64 {
            let t = score_to_trit(x, 50);
            let out = sel_trit(t, 10, 20, 30);
            return out;
        }
    ";
    let expanded = resolve_omni_surface_includes(main_with_inc, None, Some(&vfs))?;
    if execute_omni_surface_program(&expanded, vec![OmniValue::Int(75)])? != OmniValue::Int(30) {
        return Err("Module include execution mismatch".into());
    }
    let mut bad_vfs = BTreeMap::new();
    bad_vfs.insert(String::from("loop.omni"), String::from("include \"loop.omni\"; fn f(x: i64) -> i64 { return x; }"));
    if resolve_omni_surface_includes("include \"loop.omni\"; fn main(x: i64) -> i64 { return x; }", None, Some(&bad_vfs)).is_ok() {
        return Err("Module include failed to reject circular include".into());
    }
    checks += 2;

    // 6h. Phase 4: Pre-conditioned `while` loop + `<=` / `+=` + Surface-to-ELF / Stage-1 / Wasm AOT Differential
    let while_surf = "
        fn while_sum_even_golden(limit: i64) -> i64 {
            let acc = 0;
            let i = 2;
            while i <= limit {
                acc += i;
                i += 2;
            }
            return acc;
        }
    ";
    let m_while_u16 = monolith::compile_omni_surface_to_u16(while_surf)?;
    let w_while_u16 = compile_omni_surface_to_u16(while_surf)?;
    if m_while_u16 != w_while_u16 {
        return Err("Surface while_sum_even_golden u16 mismatch between monolith and workspace".into());
    }
    assert_verifier_clone_eq(&w_while_u16, true, "while_sum_even_golden")?;
    let m_while_elf = monolith::compile_omni_surface_to_x86_64_elf(while_surf, 10, 0, 0)?;
    let w_while_elf = compile_omni_surface_to_x86_64_elf(while_surf, 10, 0, 0)?;
    let m_while_wasm = monolith::compile_omni_surface_to_wasm(while_surf)?;
    let w_while_wasm = compile_omni_surface_to_wasm(while_surf)?;
    let w_while_wasm_val = execute_omni_surface_wasm_i64(while_surf, &[10])?;
    let mut s1_vm = OmniBinaryVm::new();
    let s1_while_elf = match s1_vm.execute_function(
        &stage1_mod,
        0,
        vec![OmniValue::U16Array(w_while_u16), OmniValue::Int(10), OmniValue::Int(0), OmniValue::Int(0)],
    )? {
        OmniValue::Bytes(b) => b,
        _ => return Err("Stage-1 did not return Bytes on while_sum_even_golden".into()),
    };
    if m_while_elf != w_while_elf
        || w_while_elf != s1_while_elf
        || m_while_wasm != w_while_wasm
        || w_while_wasm_val != 30
        || execute_omni_surface_program(while_surf, vec![OmniValue::Int(10)])? != OmniValue::Int(30)
    {
        return Err("Surface while_sum_even_golden AOT/Stage-1/Wasm differential mismatch".into());
    }
    checks += 2;

    // 6i. Phase 4: Surface Linear Memory Arena & Borrow Checker Differential
    let arena_ok_surf = "
        fn arena_lifecycle_golden(x: i64) -> i64 {
            let a = arena(4096);
            let r = borrow_r(a);
            let res = x + 42;
            drop(r);
            drop(a);
            return res;
        }
    ";
    let arena_ok_u16 = compile_omni_surface_to_u16(arena_ok_surf)?;
    assert_verifier_clone_eq(&arena_ok_u16, true, "arena_lifecycle_golden")?;
    if execute_omni_surface_program(arena_ok_surf, vec![OmniValue::Int(8)])? != OmniValue::Int(50) {
        return Err("Surface arena_lifecycle_golden execution mismatch".into());
    }

    let arena_leak_u16 = compile_omni_surface_to_u16("fn bad_surface_arena_leak(x: i64) -> i64 { let a = arena(4096); return x; }")?;
    assert_verifier_clone_eq(&arena_leak_u16, false, "bad_surface_arena_leak")?;

    let borrow_conflict_u16 = compile_omni_surface_to_u16(
        "fn bad_surface_borrow_conflict(x: i64) -> i64 { let a = arena(4096); let r = borrow_r(a); drop(a); drop(r); return x; }",
    )?;
    assert_verifier_clone_eq(&borrow_conflict_u16, false, "bad_surface_borrow_conflict")?;
    checks += 3;

    // 6j. Phase 4: Surface String Literals (`@ENG` Taint), Sanitizers, `scat`, `s2i`/`i2s`, and `fnv1a` Differential
    let bad_str_u16 = compile_omni_surface_to_u16("fn bad_unsanitized_str_lit() -> str { let s = \"raw <script> span\"; return s; }")?;
    assert_verifier_clone_eq(&bad_str_u16, false, "bad_unsanitized_str_lit")?;

    let good_str_pipeline = "
        #[cap(AI)]
        fn str_pipeline_golden(n: i64) -> i64 {
            let raw = \"120\";
            let clean = san_prm(raw);
            let parsed = s2i(clean);
            let num_s = i2s(n);
            let joined = scat(clean, num_s);
            let len = slen(joined);
            let h = fnv1a(clean);
            let cmp = h <=> 0;
            let bonus = sel_trit(cmp, 5, 0, 5);
            return parsed + len + bonus;
        }
    ";
    let good_str_u16 = compile_omni_surface_to_u16(good_str_pipeline)?;
    assert_verifier_clone_eq(&good_str_u16, true, "str_pipeline_golden")?;
    if execute_omni_surface_program(good_str_pipeline, vec![OmniValue::Int(45)])? != OmniValue::Int(130) {
        return Err("Surface str_pipeline_golden execution mismatch".into());
    }
    checks += 2;

    // 6k. Phase 4: Bare-Expression `eval` / `repl` Wrapper Differential
    if execute_omni_surface_program("egate(30 - 10)", vec![])? != OmniValue::Trit(1)
        || execute_omni_surface_program("10 <= 10", vec![])? != OmniValue::Trit(1)
        || execute_omni_surface_program("san_dom(\"<b>ok</b>\")", vec![])?
            != OmniValue::Str(String::from("&lt;b&gt;ok&lt;/b&gt;"))
    {
        return Err("Bare-expression wrap_omni_surface_snippet execution mismatch".into());
    }
    checks += 1;

    // 6l. Phase 5: Direct @PACK5 Trit Operations & Pack5/EGATE AOT (x86-64 ELF64 + 64-bit Wasm) Differential
    let p5_a = pack5_encode(&[1, -1, 0, 1, 1, -1, 0, -1, 1, 1]);
    let p5_b = pack5_encode(&[1, 1, 0, -1, 1, -1, 0, 1, 1, -1]);
    let direct_dot = pack5_tdot_direct(&p5_a, &p5_b, 10).map_err(|e| e.to_string())?;
    let m_direct_dot = monolith::pack5_tdot_direct(&p5_a, &p5_b, 10).map_err(|e| e.to_string())?;
    let direct_sum = pack5_sum_direct(&p5_a, 10).map_err(|e| e.to_string())?;
    let m_direct_sum = monolith::pack5_sum_direct(&p5_a, 10).map_err(|e| e.to_string())?;
    if direct_dot != m_direct_dot || direct_dot != 0 || direct_sum != m_direct_sum || direct_sum != 2 {
        return Err("Direct @PACK5 tdot/sum mismatch between monolith and workspace".into());
    }
    let m_p5_elf = monolith::compile_omni_surface_to_x86_64_elf(surf_pack5, 0, 0, 0)?;
    let w_p5_elf = compile_omni_surface_to_x86_64_elf(surf_pack5, 0, 0, 0)?;
    let m_p5_wasm_val = monolith::execute_omni_surface_wasm_i64(surf_pack5, &[])?;
    let w_p5_wasm_val = execute_omni_surface_wasm_i64(surf_pack5, &[])?;
    let w_dec_wasm_val = execute_omni_surface_wasm_i64(surf_decision_hook, &[30, 10])?;
    if m_p5_elf != w_p5_elf || m_p5_wasm_val != 2455 || w_p5_wasm_val != 2455 || w_dec_wasm_val != 1 {
        return Err("Phase 5 @PACK5 / EGATE AOT (ELF64 + Wasm) differential mismatch".into());
    }
    let surf_pack5_direct = "
        #[cap(AI)]
        fn pack5_direct_golden() -> i64 {
            let p = pack5_bytes([+1t, -1t, 0t, +1t, +1t, -1t, 0t, -1t, +1t, +1t]);
            let dot = pack5_tdot(p, p);
            let gate = pack5_gate(p);
            let len = pack5_len(p);
            return len * 100 + dot * 10 + gate;
        }
    ";
    if execute_omni_surface_wasm_i64(surf_pack5_direct, &[])? != 281
        || execute_omni_surface_program(surf_pack5_direct, vec![])? != OmniValue::Int(281)
    {
        return Err("Phase 5 pack5_bytes direct AOT execution mismatch".into());
    }
    checks += 3;

    // 6m. Phase 5: AOT-First Hot-Path Dispatcher & Stage-1 Surface-to-ELF Differential
    let (aot_v1, backend1) = execute_omni_surface_aot_first(surf_pack5, vec![])?;
    let (aot_v2, backend2) = execute_omni_surface_aot_first(surf_decision_hook, vec![OmniValue::Int(5), OmniValue::Int(20)])?;
    let (aot_v3, backend3) = execute_omni_surface_aot_first(arena_ok_surf, vec![OmniValue::Int(8)])?;
    if backend1 != "aot" || aot_v1 != OmniValue::Int(2455)
        || backend2 != "aot" || aot_v2 != OmniValue::Trit(-1)
        || backend3 != "aot" || aot_v3 != OmniValue::Int(50)
    {
        return Err("execute_omni_surface_aot_first failed to dispatch hot numeric/@PACK5/arena paths to AOT".into());
    }
    let s1_seed0_elf = compile_omni_surface_via_stage1_to_elf(surface_goldens[0].1, 30, 15, 0)?;
    let aot_seed0_elf = compile_omni_surface_to_x86_64_elf(surface_goldens[0].1, 30, 15, 0)?;
    if s1_seed0_elf != aot_seed0_elf {
        return Err("compile_omni_surface_via_stage1_to_elf mismatch against direct x86-64 AOT".into());
    }
    checks += 2;

    // 6n. Phase 5: Explicit Step & Memory Budgets (`#[step_budget(N)]`, `#[mem_budget(N)]`) across AOT & VM
    let inf_surface = "
        #[step_budget(150)]
        fn runaway_loop(x: i64) -> i64 {
            let acc = x;
            while +1t {
                acc += 1;
            }
            return acc;
        }
    ";
    if execute_omni_surface_program(inf_surface, vec![OmniValue::Int(1)]).is_ok()
        || execute_omni_surface_program_with_budgets(inf_surface, vec![OmniValue::Int(1)], Some(150), None).is_ok()
        || monolith::execute_omni_surface_program(inf_surface, vec![monolith::OmniValue::Int(1)]).is_ok()
    {
        return Err("Step budget failed to halt runaway surface loop in AOT/VM".into());
    }
    let mem_blowup_surface = "
        #[mem_budget(512)]
        fn runaway_alloc(x: i64) -> i64 {
            let a = arena(4096);
            drop(a);
            return x;
        }
    ";
    if execute_omni_surface_program(mem_blowup_surface, vec![OmniValue::Int(1)]).is_ok()
        || execute_omni_surface_program_with_budgets(mem_blowup_surface, vec![OmniValue::Int(1)], None, Some(512)).is_ok()
        || monolith::execute_omni_surface_program(mem_blowup_surface, vec![monolith::OmniValue::Int(1)]).is_ok()
    {
        return Err("Memory budget failed to reject oversized arena in AOT/VM".into());
    }
    let vm_buf_blowup_surface = "
        #[mem_budget(256)]
        fn runaway_buf() -> i64 {
            let b = anew(0);
            let i = 0;
            while i < 100 {
                b = iset_i64(b, i);
                i += 1;
            }
            return slen(b);
        }
    ";
    if execute_omni_surface_program_with_budgets(vm_buf_blowup_surface, vec![], None, None).is_ok() {
        return Err("VM memory budget failed to reject runaway byte-buffer growth in ISET".into());
    }
    checks += 3;

    // 6o. Phase 5: Zero Long-Lived Intermediate IR Direct Emission & Direct `verify_u16_stream`
    for &(g_name, surf_src, ref expected_u16) in surface_goldens {
        let direct_u16 = compile_omni_surface_to_u16(surf_src)?;
        let via_asm_u16 = assemble_omni_text_to_u16(&compile_omni_surface_to_asm(surf_src)?)?;
        let reps = verify_u16_stream(&direct_u16)?;
        if direct_u16 != via_asm_u16 || &direct_u16 != expected_u16 || reps.len() != 1 || !reps[0].passed {
            return Err(format!("Zero-IR direct u16 emission or verify_u16_stream mismatch on '{}'", g_name));
        }
    }
    checks += 1;

    // 6p. Deep Audit Regressions: 50-Trit (>35) @PACK5 + Dynamic Loop IGET in AOT, Sentinel 243..=255 Rejection, & Uncalled Helper Verification
    let surf_pack5_50 = "
        #[cap(AI)]
        fn pack5_50_dynamic_golden(limit: i64) -> i64 {
            let v = [
                +1t, -1t, 0t, +1t, +1t, -1t, 0t, -1t, +1t, +1t,
                +1t, -1t, 0t, +1t, +1t, -1t, 0t, -1t, +1t, +1t,
                +1t, -1t, 0t, +1t, +1t, -1t, 0t, -1t, +1t, +1t,
                +1t, -1t, 0t, +1t, +1t, -1t, 0t, -1t, +1t, +1t,
                +1t, -1t, 0t, +1t, +1t, -1t, 0t, -1t, +1t, +1t
            ];
            let dot = tdot(v, v);
            let gate = egate(v);
            let acc = 0;
            let idx = 0;
            while idx < limit {
                acc += iget(v, idx);
                idx += 1;
            }
            return dot * 100 + acc * 10 + gate;
        }
    ";
    let (v_50, b_50) = execute_omni_surface_aot_first(surf_pack5_50, vec![OmniValue::Int(50)])?;
    let vm_50 = execute_omni_surface_program_with_budgets(surf_pack5_50, vec![OmniValue::Int(50)], None, None)?;
    let m_elf_50 = monolith::compile_omni_surface_to_x86_64_elf(surf_pack5_50, 50, 0, 0)?;
    let w_elf_50 = compile_omni_surface_to_x86_64_elf(surf_pack5_50, 50, 0, 0)?;
    if b_50 != "aot" || v_50 != OmniValue::Int(4101) || vm_50 != OmniValue::Int(4101) || m_elf_50 != w_elf_50 {
        return Err("50-trit (>35) @PACK5 + dynamic loop IGET AOT/VM differential mismatch".into());
    }

    let corrupt_pack5_asm = "
        CAP_MASK 1
        RET_TY I64
        %0 = ANEW 0
        %0 = ISET %0 250
        %1 = UNPACK5 %0 5
        %2 = EGATE %1
        RET %2
    ";
    let corrupt_u16 = assemble_omni_text_to_u16(corrupt_pack5_asm)?;
    let corrupt_mod = decode_omni_token_stream(&corrupt_u16)?;
    let mut c_vm = OmniBinaryVm::new();
    if compile_pure_u16_to_wasm(&corrupt_u16).is_ok()
        || compile_pure_u16_to_x86_64_elf_with_args(&corrupt_u16, 0, 0, 0).is_ok()
        || c_vm.execute_function(&corrupt_mod, 0, vec![]).is_ok()
    {
        return Err("Corrupt @PACK5 sentinel byte (250 > 242) was not rejected across AOT and VM".into());
    }

    let uncalled_tainted_helper = "
        fn bad_helper(u: i64 @untrusted) -> i64 {
            return u;
        }
        fn main(x: i64) -> i64 {
            return x + 1;
        }
    ";
    if execute_omni_surface_program_with_budgets(uncalled_tainted_helper, vec![OmniValue::Int(5)], None, None).is_ok()
        || execute_omni_surface_program(uncalled_tainted_helper, vec![OmniValue::Int(5)]).is_ok()
    {
        return Err("Uncalled helper function with taint violation was not rejected before execution".into());
    }
    checks += 3;

    // 7. Stage-1 == Stage-2 Self-Compilation Fixed Point
    #[cfg(all(unix, target_arch = "x86_64"))]
    {
        use std::os::unix::fs::PermissionsExt;
        use std::process::Command;
        let s1_elf_bytes = compile_pure_u16_to_x86_64_elf_with_args(&stage1_u16, 45, 40, 90)?;
        let s1_bin = "/tmp/ws_stage1_self.elf";
        let s1_bc  = "/tmp/ws_stage1_self.omni_bc";
        fs::write(s1_bin, &s1_elf_bytes).map_err(|e| e.to_string())?;
        fs::write(s1_bc, encode_u16_stream_to_bytes(&stage1_u16)).map_err(|e| e.to_string())?;
        let mut perms = fs::metadata(s1_bin).map_err(|e| e.to_string())?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(s1_bin, perms).map_err(|e| e.to_string())?;
        let s2_out = Command::new(s1_bin).arg(s1_bc).output().map_err(|e| e.to_string())?;
        if !s2_out.status.success() || s2_out.stdout != s1_elf_bytes {
            return Err("Workspace Stage-1 == Stage-2 self-compilation fixed point failed".into());
        }
        checks += 1;
    }

    Ok(checks)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() >= 2 {
        match args[1].as_str() {
            "selftest" => {
                match run_workspace_selftest() {
                    Ok(n) => {
                        println!("[WORKSPACE SELFTEST PASS] {} differential checks passed (monolith vs omni-isa, omni-ir, omni-verifier, omni-vm, omni-assembler, omni-lang, omni-aot).", n);
                        return;
                    }
                    Err(e) => {
                        eprintln!("[WORKSPACE SELFTEST FAIL] {}", e);
                        std::process::exit(1);
                    }
                }
            }
            "debug-lens" | "--debug-lens" if args.len() >= 3 => {
                let raw = fs::read(&args[2]).expect("Failed to read .omni_bc");
                let u16s = decode_bytes_to_u16_stream(&raw);
                println!("{}", format_debug_lens(&u16s).expect("Decode failed"));
                return;
            }
            "hex" if args.len() >= 3 => {
                let raw = fs::read(&args[2]).expect("Failed to read .omni_bc");
                let u16s = decode_bytes_to_u16_stream(&raw);
                println!("{}", format_non_human_token_stream(&u16s));
                return;
            }
            "assemble" if args.len() >= 4 => {
                let src = fs::read_to_string(&args[2]).expect("Failed to read .omni_asm");
                let u16s = assemble_omni_text_to_u16(&src).expect("Assembly failed");
                fs::write(&args[3], encode_u16_stream_to_bytes(&u16s)).expect("Write failed");
                println!("[OMNIC ASSEMBLE] Wrote {} u16 tokens to '{}'", u16s.len(), args[3]);
                return;
            }
            "compile-surface" if args.len() >= 4 => {
                let u16s = compile_omni_surface_file_to_u16(&args[2]).expect("Surface compilation failed");
                fs::write(&args[3], encode_u16_stream_to_bytes(&u16s)).expect("Write failed");
                println!("[OMNIC SURFACE COMPILE] Wrote {} u16 tokens to '{}'", u16s.len(), args[3]);
                return;
            }
            "compile-elf" if args.len() >= 4 => {
                let path = &args[2];
                let u16s = if path.ends_with(".omni") {
                    compile_omni_surface_file_to_u16(path).expect("Surface compilation failed")
                } else {
                    let raw = fs::read(path).expect("Failed to read .omni_bc");
                    decode_bytes_to_u16_stream(&raw)
                };
                let a0 = args.get(4).and_then(|s| s.parse::<i64>().ok()).unwrap_or(45);
                let a1 = args.get(5).and_then(|s| s.parse::<i64>().ok()).unwrap_or(40);
                let a2 = args.get(6).and_then(|s| s.parse::<i64>().ok()).unwrap_or(90);
                let elf = compile_pure_u16_to_x86_64_elf_with_args(&u16s, a0, a1, a2)
                    .expect("x86-64 ELF AOT compilation failed");
                fs::write(&args[3], &elf).expect("Write failed");
                println!("[OMNIC ELF AOT] Wrote {} bytes to '{}' (FNV-1a64=0x{:016X})", elf.len(), args[3], fnv1a_64_hash(&elf));
                return;
            }
            "compile-wasm" if args.len() >= 4 => {
                let path = &args[2];
                let u16s = if path.ends_with(".omni") {
                    compile_omni_surface_file_to_u16(path).expect("Surface compilation failed")
                } else {
                    let raw = fs::read(path).expect("Failed to read .omni_bc");
                    decode_bytes_to_u16_stream(&raw)
                };
                let wasm = compile_pure_u16_to_wasm(&u16s).expect("WebAssembly AOT compilation failed");
                fs::write(&args[3], &wasm).expect("Write failed");
                println!("[OMNIC WASM AOT] Wrote {} bytes to '{}' (FNV-1a64=0x{:016X})", wasm.len(), args[3], fnv1a_64_hash(&wasm));
                return;
            }
            "run-wasm" if args.len() >= 3 => {
                let path = &args[2];
                let i64_args: Vec<i64> = args[3..]
                    .iter()
                    .map(|s| match parse_cli_omni_value(s) {
                        OmniValue::Int(n) => n,
                        OmniValue::Trit(t) => t as i64,
                        _ => 0,
                    })
                    .collect();
                if path.ends_with(".omni") {
                    let raw_src = fs::read_to_string(path).expect("Failed to read .omni surface file");
                    let parent_dir = std::path::Path::new(path).parent();
                    let merged = resolve_omni_surface_includes(&raw_src, parent_dir, None)
                        .expect("Failed to resolve .omni surface file");
                    let res = execute_omni_surface_wasm_i64(&merged, &i64_args).expect("Wasm execution failed");
                    println!("{}", res);
                    return;
                }
                let wasm_bytes = if path.ends_with(".wasm") {
                    fs::read(path).expect("Failed to read .wasm")
                } else {
                    let raw = fs::read(path).expect("Failed to read .omni_bc");
                    let u16s = decode_bytes_to_u16_stream(&raw);
                    compile_pure_u16_to_wasm(&u16s).expect("WebAssembly AOT compilation failed")
                };
                let res = execute_wasm_module_i64(&wasm_bytes, &i64_args).expect("Wasm execution failed");
                println!("{}", res);
                return;
            }
            "surface-to-asm" if args.len() >= 3 => {
                let raw_src = fs::read_to_string(&args[2]).expect("Failed to read .omni surface file");
                let parent_dir = std::path::Path::new(&args[2]).parent();
                let merged = resolve_omni_surface_includes(&raw_src, parent_dir, None).expect("Failed to resolve .omni surface file");
                let asm = compile_omni_surface_to_asm(&merged).expect("Surface-to-asm lowering failed");
                print!("{}", asm);
                return;
            }
            "compile-stage1" if args.len() >= 4 => {
                let raw_src = fs::read_to_string(&args[2]).expect("Failed to read .omni surface file");
                let parent_dir = std::path::Path::new(&args[2]).parent();
                let merged = resolve_omni_surface_includes(&raw_src, parent_dir, None)
                    .expect("Failed to resolve .omni surface file");
                let a0 = args.get(4).and_then(|s| s.parse::<i64>().ok()).unwrap_or(45);
                let a1 = args.get(5).and_then(|s| s.parse::<i64>().ok()).unwrap_or(40);
                let a2 = args.get(6).and_then(|s| s.parse::<i64>().ok()).unwrap_or(90);
                let elf = compile_omni_surface_via_stage1_to_elf(&merged, a0, a1, a2)
                    .expect("Stage-1 ELF compilation failed");
                fs::write(&args[3], &elf).expect("Write failed");
                println!(
                    "[OMNIC STAGE-1 ELF] Wrote {} bytes to '{}' (FNV-1a64=0x{:016X})",
                    elf.len(),
                    args[3],
                    fnv1a_64_hash(&elf)
                );
                return;
            }
            "run" | "run-surface" if args.len() >= 3 => {
                let path = &args[2];
                let mut vm_args = Vec::new();
                for a in &args[3..] {
                    vm_args.push(parse_cli_omni_value(a));
                }
                if path.ends_with(".omni") || args[1] == "run-surface" {
                    let raw_src = fs::read_to_string(path).expect("Failed to read .omni surface file");
                    let parent_dir = std::path::Path::new(path).parent();
                    let merged = match resolve_omni_surface_includes(&raw_src, parent_dir, None) {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("[OMNIC INCLUDE ERROR] {}", e);
                            std::process::exit(1);
                        }
                    };
                    match execute_omni_surface_program(&merged, vm_args) {
                        Ok(res) => println!("{:?}", res),
                        Err(e) => {
                            eprintln!("[OMNIC EXECUTION ERROR] {}", e);
                            std::process::exit(1);
                        }
                    }
                    return;
                }
                let raw = fs::read(path).expect("Failed to read .omni_bc");
                let u16s = decode_bytes_to_u16_stream(&raw);
                let m = decode_omni_token_stream(&u16s).expect("Decode failed");
                for func in &m.functions {
                    let rep = verify_binary_function(func);
                    if !rep.passed {
                        eprintln!("[OMNIC VERIFIER REJECTED] fn#{}: {:?}", func.fn_idx, rep.errors);
                        std::process::exit(1);
                    }
                }
                let entry_fn = if m.functions.iter().any(|f| f.fn_idx == 0) {
                    0
                } else {
                    m.functions.last().map(|f| f.fn_idx).unwrap_or(0)
                };
                let mut vm = OmniBinaryVm::new();
                let res = vm.execute_function(&m, entry_fn, vm_args).expect("Execution failed");
                println!("{:?}", res);
                return;
            }
            "eval" if args.len() >= 3 => {
                let snippet = &args[2];
                let mut vm_args = Vec::new();
                for a in &args[3..] {
                    vm_args.push(parse_cli_omni_value(a));
                }
                match execute_omni_surface_program(snippet, vm_args) {
                    Ok(val) => println!("{:?}", val),
                    Err(e) => {
                        eprintln!("[OMNIC EVAL ERROR] {}", e);
                        std::process::exit(1);
                    }
                }
                return;
            }
            "repl" => {
                println!("Omni Surface REPL (`omni-lang`). Enter a `fn ...` program or an expression (or `quit` to exit):");
                let stdin = io::stdin();
                let mut stdout = io::stdout();
                for line in stdin.lock().lines() {
                    print!("omni> ");
                    let _ = stdout.flush();
                    let l = match line {
                        Ok(s) => s,
                        Err(_) => break,
                    };
                    let trimmed = l.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    if trimmed == "quit" || trimmed == "exit" || trimmed == ":q" {
                        break;
                    }
                    match execute_omni_surface_program(trimmed, Vec::new()) {
                        Ok(v) => println!("=> {:?}", v),
                        Err(e) => println!("! {}", e),
                    }
                }
                return;
            }
            _ => {}
        }
    }
    let cnt = run_workspace_selftest().expect("Workspace selftest failed");
    println!("Omni Workspace CLI (omnic) — All {} monolith-vs-workspace differential checks PASSED.", cnt);
}
