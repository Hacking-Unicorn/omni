#![allow(warnings)]
#[path = "/workspace/audit-vera/omni_rust_bootstrap.rs"]
mod omni;
use omni::*;
use std::process::Command;
use std::io::Write;

fn run_elf(elf: &[u8], tag: &str) -> String {
    let p = format!("/workspace/vera-build/h/tmp_{}.elf", tag);
    std::fs::write(&p, elf).unwrap();
    Command::new("chmod").arg("+x").arg(&p).status().unwrap();
    let out = Command::new(&p).output().unwrap();
    let st = out.status;
    let so = &out.stdout;
    if so.len() == 8 {
        let v = i64::from_le_bytes(so[..8].try_into().unwrap());
        format!("{} (exit={:?})", v, st.code())
    } else {
        format!("raw{:?} (exit={:?} sig={:?})", so, st.code(), std::os::unix::process::ExitStatusExt::signal(&st))
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&a[1]).unwrap();
    let args: Vec<i64> = a[2..].iter().map(|s| s.parse().unwrap()).collect();
    let mut a3 = args.clone(); while a3.len() < 3 { a3.push(0); }
    let wrapped = wrap_omni_surface_snippet(&src);
    let u16s = match compile_omni_surface_to_u16(&wrapped) { Ok(u) => u, Err(e) => { println!("FRONTEND ERR {}", e); return; } };
    // VM
    match decode_omni_token_stream(&u16s) {
        Ok(m) => {
            let entry = if m.functions.iter().any(|f| f.fn_idx == 0) { 0 } else { m.functions.last().unwrap().fn_idx };
            let mut vm = OmniBinaryVm::new();
            let r = vm.execute_function(&m, entry, args.iter().map(|v| OmniValue::Int(*v)).collect());
            println!("VM      : {:?}", r);
        }
        Err(e) => println!("VM decode err {}", e),
    }
    match execute_omni_surface_aot_first(&src, args.iter().map(|v| OmniValue::Int(*v)).collect()) {
        Ok((v, t)) => println!("AOTFIRST: {:?} via {}", v, t),
        Err(e) => println!("AOTFIRST: ERR {}", e),
    }
    match compile_pure_u16_to_wasm(&u16s) {
        Ok(w) => match execute_wasm_module_i64(&w, &args) { Ok(v) => println!("WASM    : {}", v), Err(e) => println!("WASM    : RUN ERR {}", e) },
        Err(e) => println!("WASM    : COMPILE ERR {}", e),
    }
    let host = compile_pure_u16_to_x86_64_elf_with_args(&u16s, a3[0], a3[1], a3[2]);
    match &host { Ok(e) => println!("X86     : {} ({} B, fnv={:016X})", run_elf(e, "host"), e.len(), fnv1a_64_hash(e)), Err(e) => println!("X86     : COMPILE ERR {}", e) }
    let s1 = compile_omni_surface_via_stage1_to_elf(&src, a3[0], a3[1], a3[2]);
    match &s1 { Ok(e) => println!("STAGE1  : {} ({} B, fnv={:016X})", run_elf(e, "s1"), e.len(), fnv1a_64_hash(e)), Err(e) => println!("STAGE1  : COMPILE ERR {}", e) }
    if let (Ok(h), Ok(s)) = (&host, &s1) { println!("ELF_EQ  : {}", h == s); }
    if std::env::var("ASM").is_ok() { println!("{}", compile_omni_surface_to_asm(&wrapped).unwrap_or_else(|e| e)); }
}
