fn main() {
    let args: Vec<String> = std::env::args().collect();
    std::process::exit(omni_lang::run_omni_harness_cli(&args[1..]));
}
