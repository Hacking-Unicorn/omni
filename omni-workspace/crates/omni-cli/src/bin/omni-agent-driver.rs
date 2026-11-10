//! Standalone `omni-agent-driver` binary — version-locked AI emit + repair driver beside `omni-harness`.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(omni_lang::run_omni_agent_driver_cli(&args));
}
