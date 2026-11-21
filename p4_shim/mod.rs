// Phase 4 (P4-3): Minimal 3-operation thin host shim (`read_bytes`, `write_bytes`, `spawn`).
// Deliberately contains NO HTTP server, NO shell execution (`sh -c` / `cmd /C`),
// NO source parser, and NO general file/IO stack. Placeholder for Phase 5/6
// syscall intrinsics + minimal assembly entry stub.

pub trait OmniThinShim {
    fn read_bytes(&self, path: &str) -> Result<Vec<u8>, String>;
    fn write_bytes(&mut self, path: &str, data: &[u8]) -> Result<(), String>;
    fn spawn(&mut self, program: &str, args: &[&str]) -> Result<i32, String>;
}

/// Native OS thin shim exposing strictly `read_bytes`, `write_bytes`, and direct-argv `spawn`.
pub struct StdThinShim;

impl OmniThinShim for StdThinShim {
    fn read_bytes(&self, path: &str) -> Result<Vec<u8>, String> {
        std::fs::read(path).map_err(|e| format!("p4_shim read_bytes('{}'): {}", path, e))
    }

    fn write_bytes(&mut self, path: &str, data: &[u8]) -> Result<(), String> {
        std::fs::write(path, data).map_err(|e| format!("p4_shim write_bytes('{}'): {}", path, e))
    }

    fn spawn(&mut self, program: &str, args: &[&str]) -> Result<i32, String> {
        let status = std::process::Command::new(program)
            .args(args)
            .status()
            .map_err(|e| format!("p4_shim spawn('{}'): {}", program, e))?;
        Ok(status.code().unwrap_or(-1))
    }
}

/// Deterministic in-memory thin shim for P4-3 selftests (proves Pure-Omni VM needs
/// strictly read_bytes / write_bytes / spawn and zero other host capabilities).
pub struct MemoryThinShim {
    pub files: std::collections::BTreeMap<String, Vec<u8>>,
    pub spawned: Vec<(String, Vec<String>)>,
}

impl MemoryThinShim {
    pub fn new() -> Self {
        Self {
            files: std::collections::BTreeMap::new(),
            spawned: Vec::new(),
        }
    }
}

impl OmniThinShim for MemoryThinShim {
    fn read_bytes(&self, path: &str) -> Result<Vec<u8>, String> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| format!("p4_shim memory read_bytes('{}'): not found", path))
    }

    fn write_bytes(&mut self, path: &str, data: &[u8]) -> Result<(), String> {
        self.files.insert(path.to_string(), data.to_vec());
        Ok(())
    }

    fn spawn(&mut self, program: &str, args: &[&str]) -> Result<i32, String> {
        self.spawned.push((
            program.to_string(),
            args.iter().map(|s| s.to_string()).collect(),
        ));
        Ok(0)
    }
}

/// Runs a raw `.omni_bc` file strictly through the 3-op `OmniThinShim` + Pure-Omni VM (`pure/m4/vm.omni`),
/// optionally persisting the 16-byte execution summary (`[ret_val: i64, err: u32, steps: u32]`) via `write_bytes`.
pub fn p4_shim_exec_bc(
    shim: &mut dyn OmniThinShim,
    bc_path: &str,
    out_path: Option<&str>,
    a0: i64,
    a1: i64,
    a2: i64,
    fuel: i64,
    trace_mode: i64,
) -> Result<(i64, u32, u32, u32, u32), String> {
    let raw = shim.read_bytes(bc_path)?;
    if raw.len() % 2 != 0 {
        return Err(format!("p4_shim: '{}' has odd byte length {}", bc_path, raw.len()));
    }
    let words: Vec<u16> = raw
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let (val, tag, err, steps, tcnt) =
        p4_run_raw_bc_on_pure_vm(&words, a0, a1, a2, fuel, trace_mode)?;
    if let Some(op) = out_path {
        let mut summary = Vec::with_capacity(20);
        summary.extend_from_slice(&val.to_le_bytes());
        summary.extend_from_slice(&err.to_le_bytes());
        summary.extend_from_slice(&steps.to_le_bytes());
        summary.extend_from_slice(&tcnt.to_le_bytes());
        shim.write_bytes(op, &summary)?;
    }
    Ok((val, tag, err, steps, tcnt))
}
