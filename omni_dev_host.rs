// ==============================================================================
// OMNI SEED-SHRINK (PHASE B1) — OPTIONAL HTTP/KV DEV-HOST ENTRY POINT
// `omni_dev_host.rs`
// ==============================================================================
// Clearly separated optional dev-host tooling (`omni-dev-host`).
// NOT required for compile, run, verification, or self-hosting (all of which run
// on the Pure-Omni M3 -> M2/M4/M5 pipeline and `seed0/omni_seed0.rs`).
//
// Build (optional):
//   rustc -O --edition 2021 omni_dev_host.rs -o omni-dev-host
// Usage:
//   ./omni-dev-host [--port <p>] <file.omni>
// ==============================================================================

#[path = "omni_rust_bootstrap.rs"]
#[allow(dead_code)]
mod omni_bootstrap_ref;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 && (args[1] == "--help" || args[1] == "-h") {
        println!("omni-dev-host (optional HTTP/KV dev-host; not required for Pure-Omni compile/run/self-host)");
        println!("Usage: omni-dev-host [--port <p>] <file.omni>");
        return;
    }
    if args.len() >= 2 && args[1] == "--version" {
        println!("omni-dev-host 2.0.0 (optional-dev-host)");
        return;
    }
    let slice = if args.len() >= 2 && (args[1] == "serve" || args[1] == "--serve") {
        &args[2..]
    } else {
        &args[1..]
    };
    if slice.is_empty() {
        eprintln!("Usage: omni-dev-host [--port <p>] <file.omni>");
        std::process::exit(2);
    }
    std::process::exit(omni_bootstrap_ref::run_omni_serve_cli(slice));
}
