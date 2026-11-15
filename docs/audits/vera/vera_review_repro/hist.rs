#![allow(warnings)]
#[path = "/workspace/audit-vera/omni_rust_bootstrap.rs"]
mod omni;
use omni::*;
use std::collections::BTreeMap;
fn hist(name: &str, s: &[u16]) {
    let m = decode_omni_token_stream(s).unwrap();
    let mut h: BTreeMap<u16, usize> = BTreeMap::new();
    for f in &m.functions { for i in &f.instructions { *h.entry(i.op_tok).or_default() += 1; } }
    let v: Vec<String> = h.iter().map(|(k, c)| format!("{:04X}x{}", k, c)).collect();
    println!("{} ({} words, fnv={:016X}): {}", name, s.len(), fnv1a_64_hash(&encode_u16_stream_to_bytes(s)), v.join(" "));
}
fn main() {
    hist("stage1_compiler", &build_step7_stage1_compiler_stream());
    hist("phaseD_gen1", &compile_omni_surface_strict_to_u16(PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE).unwrap());
    let d = build_phase_b_pure_omni_driver_stream().unwrap(); println!("driver words={} fnv={:016X}", d.len(), fnv1a_64_hash(&encode_u16_stream_to_bytes(&d)));
    let suite = build_pure_omni_core_suite_stream().unwrap();
    println!("suite words={} fnv={:016X}", suite.len(), fnv1a_64_hash(&encode_u16_stream_to_bytes(&suite)));
}
