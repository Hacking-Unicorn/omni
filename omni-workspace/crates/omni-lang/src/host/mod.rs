#![allow(dead_code, unused_imports)]
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use omni_isa::*;
use omni_ir::*;
use omni_verifier::*;
use omni_vm::*;
use omni_assembler::*;
use omni_aot::*;

use crate::compiler::*;
use crate::ternary::*;
use crate::verifier::*;
use crate::repair_agent::*;
use crate::harness::*;
use crate::inspect::*;
use crate::acceptance::*;
use crate::pure_omni::*;

// ==============================================================================
// 7E. NARROW CAPABILITY-GATED HOST INTERFACE (`OmniNarrowHost`) & LIVE HTTP/DB HOST
// ==============================================================================

pub(crate) fn check_narrow_host_cap(cap_mask: CapMask, req_mask: CapMask, syscall_name: &str) -> Result<(), String> {
    if (cap_mask & req_mask) != req_mask {
        return Err(format!(
            "error[MISSING_CAP] line 1, col 1: Narrow host syscall '{}' requires capability mask 0x{:04X}, got 0x{:04X}",
            syscall_name, req_mask, cap_mask
        ));
    }
    Ok(())
}

/// The frozen 10-operation narrow capability-gated host boundary (`spec/host_interface.md`, `omni-narrow-host-v1.0.0`).
/// All OS network, file-backed KV/DB persistence, and monotonic clock operations route through this trait.
pub trait OmniNarrowHost {
    fn host_tcp_listen(&mut self, cap_mask: CapMask, bind_addr: &str) -> Result<(u64, String), String>;
    fn host_tcp_accept(&mut self, cap_mask: CapMask, listener_handle: u64) -> Result<u64, String>;
    fn host_tcp_read(&mut self, cap_mask: CapMask, conn_handle: u64, max_bytes: usize) -> Result<Vec<u8>, String>;
    fn host_tcp_write(&mut self, cap_mask: CapMask, conn_handle: u64, data: &[u8]) -> Result<usize, String>;
    fn host_tcp_close(&mut self, cap_mask: CapMask, handle: u64) -> Result<(), String>;
    fn host_file_open(&mut self, cap_mask: CapMask, path: &str, write: bool) -> Result<u64, String>;
    fn host_file_read(&mut self, cap_mask: CapMask, file_handle: u64) -> Result<Vec<u8>, String>;
    fn host_file_write(&mut self, cap_mask: CapMask, file_handle: u64, data: &[u8]) -> Result<usize, String>;
    fn host_file_close(&mut self, cap_mask: CapMask, file_handle: u64) -> Result<(), String>;
    fn host_time_ns(&mut self, cap_mask: CapMask) -> Result<u64, String>;
}

/// Resolves a host FS/DB path against an explicit, canonicalized sandbox root.
///
/// Rejects empty or NUL paths, `..` traversal, absolute paths outside the root,
/// and symlink escapes before any file is opened for read or write.
pub fn resolve_sandboxed_host_path(root: &std::path::Path, path: &str) -> Result<std::path::PathBuf, String> {
    const ESC: &str = "error[PATH_ESCAPE] line 1, col 1";
    if path.is_empty() {
        return Err(format!("{}: Host file path must be non-empty", ESC));
    }
    if path.contains('\0') {
        return Err(format!("{}: Host file path must not contain NUL", ESC));
    }
    let requested = std::path::Path::new(path);
    if requested.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(format!("{}: Host file path must not contain '..' traversal", ESC));
    }
    let root_canon = root.canonicalize().map_err(|e| {
        format!("{}: Sandbox root '{}' cannot be canonicalized: {}", ESC, root.display(), e)
    })?;
    let candidate = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        root_canon.join(requested)
    };
    let resolved = canonicalize_host_path_under_root(&root_canon, &candidate)?;
    if !resolved.starts_with(&root_canon) {
        return Err(format!("{}: Host file path escapes the sandbox root", ESC));
    }
    Ok(resolved)
}

fn canonicalize_host_path_under_root(
    root: &std::path::Path,
    candidate: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    const ESC: &str = "error[PATH_ESCAPE] line 1, col 1";
    // Walk every component, canonicalizing existing prefixes (including symlink
    // ancestors such as /tmp). Only the *final* resolved path is checked against
    // the sandbox root so intermediate ancestors of the root are not rejected.
    let mut cur = std::path::PathBuf::new();
    for comp in candidate.components() {
        match comp {
            std::path::Component::Prefix(_) | std::path::Component::RootDir => {
                cur.push(comp);
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                return Err(format!("{}: Host file path must not contain '..' traversal", ESC));
            }
            std::path::Component::Normal(_) => {
                cur.push(comp);
                if std::fs::symlink_metadata(&cur).is_ok() {
                    cur = cur.canonicalize().map_err(|e| {
                        format!("{}: Host file path could not be canonicalized: {}", ESC, e)
                    })?;
                }
            }
        }
    }
    if !cur.starts_with(root) {
        return Err(format!(
            "{}: Host file path escapes the sandbox root via a symlink or absolute path",
            ESC
        ));
    }
    Ok(cur)
}

fn reject_unsandboxed_virtual_path(path: &str) -> Result<(), String> {
    const ESC: &str = "error[PATH_ESCAPE] line 1, col 1";
    if path.is_empty() {
        return Err(format!("{}: Host file path must be non-empty", ESC));
    }
    if path.contains('\0') {
        return Err(format!("{}: Host file path must not contain NUL", ESC));
    }
    let requested = std::path::Path::new(path);
    if requested.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(format!("{}: Host file path must not contain '..' traversal", ESC));
    }
    if requested.is_absolute() {
        return Err(format!(
            "{}: Host file path must not be an absolute path outside the sandbox",
            ESC
        ));
    }
    Ok(())
}

/// Live OS reference implementation of `OmniNarrowHost` backed by `std::net`, `std::fs`, and `std::time::Instant`.
pub struct StdNarrowHostAdapter {
    next_handle: u64,
    listeners: BTreeMap<u64, std::net::TcpListener>,
    streams: BTreeMap<u64, std::net::TcpStream>,
    files: BTreeMap<u64, (std::path::PathBuf, bool, Vec<u8>)>,
    start_instant: std::time::Instant,
    sandbox_root: std::path::PathBuf,
}

impl Default for StdNarrowHostAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl StdNarrowHostAdapter {
    pub fn new() -> Self {
        Self::with_sandbox_root(std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")))
    }

    pub fn with_sandbox_root(root: impl AsRef<std::path::Path>) -> Self {
        let given = root.as_ref().to_path_buf();
        let sandbox_root = given.canonicalize().unwrap_or(given);
        Self {
            next_handle: 1,
            listeners: BTreeMap::new(),
            streams: BTreeMap::new(),
            files: BTreeMap::new(),
            start_instant: std::time::Instant::now(),
            sandbox_root,
        }
    }

    pub(crate) fn alloc_handle(&mut self) -> u64 {
        let h = self.next_handle;
        self.next_handle += 1;
        h
    }
}

impl OmniNarrowHost for StdNarrowHostAdapter {
    fn host_tcp_listen(&mut self, cap_mask: CapMask, bind_addr: &str) -> Result<(u64, String), String> {
        check_narrow_host_cap(cap_mask, cap_bit(CAP_ID_NET), "host_tcp_listen")?;
        let listener = std::net::TcpListener::bind(bind_addr)
            .map_err(|e| format!("Failed to bind TCP listener on '{}': {}", bind_addr, e))?;
        let addr_str = listener
            .local_addr()
            .map_err(|e| format!("Failed to query local socket addr: {}", e))?
            .to_string();
        let h = self.alloc_handle();
        self.listeners.insert(h, listener);
        Ok((h, addr_str))
    }

    fn host_tcp_accept(&mut self, cap_mask: CapMask, listener_handle: u64) -> Result<u64, String> {
        check_narrow_host_cap(cap_mask, cap_bit(CAP_ID_NET), "host_tcp_accept")?;
        let listener = self
            .listeners
            .get(&listener_handle)
            .ok_or_else(|| format!("Invalid TCP listener handle {}", listener_handle))?;
        let (stream, _) = listener.accept().map_err(|e| format!("TCP accept failed: {}", e))?;
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
        let ch = self.alloc_handle();
        self.streams.insert(ch, stream);
        Ok(ch)
    }

    fn host_tcp_read(&mut self, cap_mask: CapMask, conn_handle: u64, max_bytes: usize) -> Result<Vec<u8>, String> {
        use std::io::Read;
        check_narrow_host_cap(cap_mask, cap_bit(CAP_ID_NET), "host_tcp_read")?;
        let stream = self
            .streams
            .get_mut(&conn_handle)
            .ok_or_else(|| format!("Invalid TCP connection handle {}", conn_handle))?;
        let mut buf = vec![0u8; max_bytes.clamp(1, 65536)];
        let n = stream.read(&mut buf).map_err(|e| format!("TCP read failed: {}", e))?;
        buf.truncate(n);
        Ok(buf)
    }

    fn host_tcp_write(&mut self, cap_mask: CapMask, conn_handle: u64, data: &[u8]) -> Result<usize, String> {
        use std::io::Write;
        check_narrow_host_cap(cap_mask, cap_bit(CAP_ID_NET), "host_tcp_write")?;
        let stream = self
            .streams
            .get_mut(&conn_handle)
            .ok_or_else(|| format!("Invalid TCP connection handle {}", conn_handle))?;
        stream.write_all(data).map_err(|e| format!("TCP write failed: {}", e))?;
        let _ = stream.flush();
        Ok(data.len())
    }

    fn host_tcp_close(&mut self, cap_mask: CapMask, handle: u64) -> Result<(), String> {
        check_narrow_host_cap(cap_mask, cap_bit(CAP_ID_NET), "host_tcp_close")?;
        self.streams.remove(&handle);
        self.listeners.remove(&handle);
        Ok(())
    }

    fn host_file_open(&mut self, cap_mask: CapMask, path: &str, write: bool) -> Result<u64, String> {
        if write {
            if (cap_mask & (cap_bit(CAP_ID_DB_W) | cap_bit(CAP_ID_FS))) == 0 {
                return Err(String::from(
                    "error[MISSING_CAP] line 1, col 1: Narrow host syscall 'host_file_open(write)' requires CAP_ID_DB_W or CAP_ID_FS",
                ));
            }
        } else if (cap_mask & (cap_bit(CAP_ID_DB_R) | cap_bit(CAP_ID_FS))) == 0 {
            return Err(String::from(
                "error[MISSING_CAP] line 1, col 1: Narrow host syscall 'host_file_open(read)' requires CAP_ID_DB_R or CAP_ID_FS",
            ));
        }
        let p = resolve_sandboxed_host_path(&self.sandbox_root, path)?;
        let initial_bytes = if !write && p.exists() {
            std::fs::read(&p).map_err(|e| format!("Failed to read host file '{}': {}", p.display(), e))?
        } else {
            Vec::new()
        };
        let fh = self.alloc_handle();
        self.files.insert(fh, (p, write, initial_bytes));
        Ok(fh)
    }

    fn host_file_read(&mut self, cap_mask: CapMask, file_handle: u64) -> Result<Vec<u8>, String> {
        if (cap_mask & (cap_bit(CAP_ID_DB_R) | cap_bit(CAP_ID_FS))) == 0 {
            return Err(String::from(
                "error[MISSING_CAP] line 1, col 1: Narrow host syscall 'host_file_read' requires CAP_ID_DB_R or CAP_ID_FS",
            ));
        }
        let (_, _, bytes) = self
            .files
            .get(&file_handle)
            .ok_or_else(|| format!("Invalid file handle {}", file_handle))?;
        Ok(bytes.clone())
    }

    fn host_file_write(&mut self, cap_mask: CapMask, file_handle: u64, data: &[u8]) -> Result<usize, String> {
        if (cap_mask & (cap_bit(CAP_ID_DB_W) | cap_bit(CAP_ID_FS))) == 0 {
            return Err(String::from(
                "error[MISSING_CAP] line 1, col 1: Narrow host syscall 'host_file_write' requires CAP_ID_DB_W or CAP_ID_FS",
            ));
        }
        let (_, write, bytes) = self
            .files
            .get_mut(&file_handle)
            .ok_or_else(|| format!("Invalid file handle {}", file_handle))?;
        if !*write {
            return Err(format!("File handle {} was not opened for writing", file_handle));
        }
        *bytes = data.to_vec();
        Ok(data.len())
    }

    fn host_file_close(&mut self, cap_mask: CapMask, file_handle: u64) -> Result<(), String> {
        if (cap_mask & (cap_bit(CAP_ID_DB_R) | cap_bit(CAP_ID_DB_W) | cap_bit(CAP_ID_FS))) == 0 {
            return Err(String::from(
                "error[MISSING_CAP] line 1, col 1: Narrow host syscall 'host_file_close' requires DB/FS capability",
            ));
        }
        if let Some((path, write, bytes)) = self.files.remove(&file_handle) {
            if write {
                std::fs::write(&path, &bytes)
                    .map_err(|e| format!("Failed to flush host file '{}': {}", path.display(), e))?;
            }
        }
        Ok(())
    }

    fn host_time_ns(&mut self, cap_mask: CapMask) -> Result<u64, String> {
        check_narrow_host_cap(cap_mask, cap_bit(CAP_ID_SYS), "host_time_ns")?;
        Ok(self.start_instant.elapsed().as_nanos() as u64)
    }
}

/// Deterministic in-memory implementation of `OmniNarrowHost` for socket-free & disk-free verification of the host boundary.
pub struct SimulatedNarrowHostAdapter {
    next_handle: u64,
    pub bound_addr: Option<String>,
    pub pending_requests: Vec<Vec<u8>>,
    pub written_responses: Vec<Vec<u8>>,
    pub virtual_files: BTreeMap<String, Vec<u8>>,
    open_files: BTreeMap<u64, (String, bool, Vec<u8>)>,
    clock_ns: u64,
}

impl Default for SimulatedNarrowHostAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl SimulatedNarrowHostAdapter {
    pub fn new() -> Self {
        Self {
            next_handle: 100,
            bound_addr: None,
            pending_requests: Vec::new(),
            written_responses: Vec::new(),
            virtual_files: BTreeMap::new(),
            open_files: BTreeMap::new(),
            clock_ns: 1_000_000,
        }
    }

    pub fn enqueue_http_request(&mut self, raw_wire: &str) {
        self.pending_requests.push(raw_wire.as_bytes().to_vec());
    }

    pub(crate) fn alloc_handle(&mut self) -> u64 {
        let h = self.next_handle;
        self.next_handle += 1;
        h
    }
}

impl OmniNarrowHost for SimulatedNarrowHostAdapter {
    fn host_tcp_listen(&mut self, cap_mask: CapMask, bind_addr: &str) -> Result<(u64, String), String> {
        check_narrow_host_cap(cap_mask, cap_bit(CAP_ID_NET), "host_tcp_listen")?;
        let assigned = if bind_addr.ends_with(":0") {
            String::from("127.0.0.1:40080")
        } else {
            String::from(bind_addr)
        };
        self.bound_addr = Some(assigned.clone());
        Ok((self.alloc_handle(), assigned))
    }

    fn host_tcp_accept(&mut self, cap_mask: CapMask, _listener_handle: u64) -> Result<u64, String> {
        check_narrow_host_cap(cap_mask, cap_bit(CAP_ID_NET), "host_tcp_accept")?;
        Ok(self.alloc_handle())
    }

    fn host_tcp_read(&mut self, cap_mask: CapMask, _conn_handle: u64, _max_bytes: usize) -> Result<Vec<u8>, String> {
        check_narrow_host_cap(cap_mask, cap_bit(CAP_ID_NET), "host_tcp_read")?;
        if self.pending_requests.is_empty() {
            return Err(String::from("No pending simulated TCP request"));
        }
        Ok(self.pending_requests.remove(0))
    }

    fn host_tcp_write(&mut self, cap_mask: CapMask, _conn_handle: u64, data: &[u8]) -> Result<usize, String> {
        check_narrow_host_cap(cap_mask, cap_bit(CAP_ID_NET), "host_tcp_write")?;
        self.written_responses.push(data.to_vec());
        Ok(data.len())
    }

    fn host_tcp_close(&mut self, cap_mask: CapMask, _handle: u64) -> Result<(), String> {
        check_narrow_host_cap(cap_mask, cap_bit(CAP_ID_NET), "host_tcp_close")?;
        Ok(())
    }

    fn host_file_open(&mut self, cap_mask: CapMask, path: &str, write: bool) -> Result<u64, String> {
        if write {
            if (cap_mask & (cap_bit(CAP_ID_DB_W) | cap_bit(CAP_ID_FS))) == 0 {
                return Err(String::from(
                    "error[MISSING_CAP] line 1, col 1: Narrow host syscall 'host_file_open(write)' requires CAP_ID_DB_W or CAP_ID_FS",
                ));
            }
        } else if (cap_mask & (cap_bit(CAP_ID_DB_R) | cap_bit(CAP_ID_FS))) == 0 {
            return Err(String::from(
                "error[MISSING_CAP] line 1, col 1: Narrow host syscall 'host_file_open(read)' requires CAP_ID_DB_R or CAP_ID_FS",
            ));
        }
        reject_unsandboxed_virtual_path(path)?;
        let existing = self.virtual_files.get(path).cloned().unwrap_or_default();
        let fh = self.alloc_handle();
        self.open_files.insert(fh, (String::from(path), write, existing));
        Ok(fh)
    }

    fn host_file_read(&mut self, cap_mask: CapMask, file_handle: u64) -> Result<Vec<u8>, String> {
        if (cap_mask & (cap_bit(CAP_ID_DB_R) | cap_bit(CAP_ID_FS))) == 0 {
            return Err(String::from(
                "error[MISSING_CAP] line 1, col 1: Narrow host syscall 'host_file_read' requires CAP_ID_DB_R or CAP_ID_FS",
            ));
        }
        let (_, _, data) = self
            .open_files
            .get(&file_handle)
            .ok_or_else(|| format!("Invalid simulated file handle {}", file_handle))?;
        Ok(data.clone())
    }

    fn host_file_write(&mut self, cap_mask: CapMask, file_handle: u64, data: &[u8]) -> Result<usize, String> {
        if (cap_mask & (cap_bit(CAP_ID_DB_W) | cap_bit(CAP_ID_FS))) == 0 {
            return Err(String::from(
                "error[MISSING_CAP] line 1, col 1: Narrow host syscall 'host_file_write' requires CAP_ID_DB_W or CAP_ID_FS",
            ));
        }
        let (_, write, buf) = self
            .open_files
            .get_mut(&file_handle)
            .ok_or_else(|| format!("Invalid simulated file handle {}", file_handle))?;
        if !*write {
            return Err(format!("Simulated file handle {} not opened for write", file_handle));
        }
        *buf = data.to_vec();
        Ok(data.len())
    }

    fn host_file_close(&mut self, cap_mask: CapMask, file_handle: u64) -> Result<(), String> {
        if (cap_mask & (cap_bit(CAP_ID_DB_R) | cap_bit(CAP_ID_DB_W) | cap_bit(CAP_ID_FS))) == 0 {
            return Err(String::from(
                "error[MISSING_CAP] line 1, col 1: Narrow host syscall 'host_file_close' requires DB/FS capability",
            ));
        }
        if let Some((path, write, buf)) = self.open_files.remove(&file_handle) {
            if write {
                self.virtual_files.insert(path, buf);
            }
        }
        Ok(())
    }

    fn host_time_ns(&mut self, cap_mask: CapMask) -> Result<u64, String> {
        check_narrow_host_cap(cap_mask, cap_bit(CAP_ID_SYS), "host_time_ns")?;
        self.clock_ns += 250_000;
        Ok(self.clock_ns)
    }
}

/// Loads a `key=value` map through any `OmniNarrowHost` implementation (`CAP_ID_DB_R`).
pub fn load_omni_kv_via_narrow_host<H: OmniNarrowHost>(
    host: &mut H,
    cap_mask: CapMask,
    path: &str,
) -> Result<BTreeMap<i64, i64>, String> {
    let fh = host.host_file_open(cap_mask, path, false)?;
    let raw_bytes = host.host_file_read(cap_mask, fh)?;
    host.host_file_close(cap_mask, fh)?;
    let raw = String::from_utf8_lossy(&raw_bytes);
    let mut kv = BTreeMap::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((k_str, v_str)) = trimmed.split_once('=') {
            if let (Ok(k), Ok(v)) = (k_str.trim().parse::<i64>(), v_str.trim().parse::<i64>()) {
                kv.insert(k, v);
            }
        }
    }
    Ok(kv)
}

/// Saves a `key=value` map through any `OmniNarrowHost` implementation (`CAP_ID_DB_W`).
pub fn save_omni_kv_via_narrow_host<H: OmniNarrowHost>(
    host: &mut H,
    cap_mask: CapMask,
    path: &str,
    kv: &BTreeMap<i64, i64>,
) -> Result<(), String> {
    let mut buf = String::new();
    for (k, v) in kv {
        buf.push_str(&format!("{}={}\n", k, v));
    }
    let fh = host.host_file_open(cap_mask, path, true)?;
    let _ = host.host_file_write(cap_mask, fh, buf.as_bytes())?;
    host.host_file_close(cap_mask, fh)
}

/// Executes a full HTTP/1.1 + DB/KV batch cycle exclusively through the narrow `OmniNarrowHost` interface.
pub fn serve_omni_http_via_narrow_host<H: OmniNarrowHost>(
    host: &mut H,
    src: &str,
    bind_addr: &str,
    db_path: Option<&str>,
    num_requests: usize,
) -> Result<(String, Vec<OmniHttpResponse>, BTreeMap<i64, i64>), String> {
    let routes = extract_omni_routes(src)?;
    if routes.is_empty() {
        return Err(String::from("error[SYNTAX_ERROR] line 1, col 1: Surface program contains no 'route' handlers"));
    }
    let (u16_stream, sm) = compile_omni_surface_via_pure_omni_frontend_with_sourcemap(src, false)?;
    if let Err(kernel_err) = verify_module_via_phase7_pure_omni_kernel(&u16_stream) {
        for rep in verify_u16_stream_lockstep("serve", &u16_stream).map_err(|e| annotate_omni_error_with_sourcemap(&e, &sm))? {
            if !rep.passed {
                let first = rep.errors.first().cloned().unwrap_or_default();
                let annotated = annotate_omni_error_with_sourcemap(&first, &sm);
                return Err(format!("{} (fn #{}: {:?})", annotated, rep.fn_idx, rep.errors));
            }
        }
        return Err(annotate_omni_error_with_sourcemap(&kernel_err, &sm));
    }
    for rep in verify_u16_stream_lockstep("serve", &u16_stream)? {
        if !rep.passed {
            let first = rep.errors.first().cloned().unwrap_or_default();
            let annotated = annotate_omni_error_with_sourcemap(&first, &sm);
            return Err(format!("{} (fn #{}: {:?})", annotated, rep.fn_idx, rep.errors));
        }
    }
    let module = decode_omni_token_stream(&u16_stream)?;
    let combined_caps: CapMask = module.functions.iter().fold(0u16, |acc, f| acc | f.cap_mask);

    let mut kv_store = if let Some(p) = db_path {
        load_omni_kv_via_narrow_host(host, combined_caps | cap_bit(CAP_ID_DB_R), p)?
    } else {
        BTreeMap::new()
    };

    let (listener_h, local_addr) = host.host_tcp_listen(combined_caps, bind_addr)?;
    let mut responses = Vec::with_capacity(num_requests);
    for _ in 0..num_requests {
        let conn_h = host.host_tcp_accept(combined_caps, listener_h)?;
        let req_bytes = host.host_tcp_read(combined_caps, conn_h, 4096)?;
        let req_str = String::from_utf8_lossy(&req_bytes);
        let first_line = req_str.lines().next().unwrap_or("GET / HTTP/1.1");
        let mut parts = first_line.split_whitespace();
        let method = parts.next().unwrap_or("GET");
        let req_path = parts.next().unwrap_or("/");
        let resp = handle_omni_http_request(src, method, req_path, &mut kv_store)?;
        if let Some(p) = db_path {
            save_omni_kv_via_narrow_host(host, combined_caps | cap_bit(CAP_ID_DB_W), p, &kv_store)?;
        }
        let wire = resp.to_http11_wire_bytes();
        let _ = host.host_tcp_write(combined_caps, conn_h, &wire)?;
        let _ = host.host_tcp_close(combined_caps, conn_h)?;
        responses.push(resp);
    }
    let _ = host.host_tcp_close(combined_caps, listener_h)?;
    Ok((local_addr, responses, kv_store))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OmniRouteSpec {
    pub fn_idx: u16,
    pub method: String,
    pub path_pattern: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OmniHttpResponse {
    pub status: u16,
    pub content_type: String,
    pub body: String,
}

impl OmniHttpResponse {
    pub fn status_text(status: u16) -> &'static str {
        match status {
            200 => "OK",
            201 => "Created",
            204 => "No Content",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            500 => "Internal Server Error",
            _ => "OK",
        }
    }

    pub fn to_http11_wire_bytes(&self) -> Vec<u8> {
        format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            self.status,
            Self::status_text(self.status),
            self.content_type,
            self.body.len(),
            self.body
        )
        .into_bytes()
    }
}

/// Extracts all `route <METHOD> <path>` declarations from a surface `.omni` module alongside their assigned `fn_idx`.
pub fn extract_omni_routes(src: &str) -> Result<Vec<OmniRouteSpec>, String> {
    let effective_strict = src.contains("#[strict]");
    let tokens = tokenize_omni_surface_inner(src, effective_strict)?;
    let mut parser = SurfaceParser::new_with_strict(tokens, effective_strict);
    let (includes, fns) = parser.parse_module()?;
    if !includes.is_empty() {
        let expanded = resolve_omni_surface_includes(src, None, None)?;
        return extract_omni_routes(&expanded);
    }
    let multi_fn = fns.len() > 1;
    let mut routes = Vec::new();
    for (idx, f) in fns.iter().enumerate() {
        let fid = if let Some(id_attr) = f.fn_id_attr {
            id_attr
        } else if !multi_fn || idx + 1 == fns.len() {
            0
        } else {
            (idx + 1) as u16
        };
        if f.is_route {
            routes.push(OmniRouteSpec {
                fn_idx: fid,
                method: f.route_method.clone().unwrap_or_else(|| String::from("GET")),
                path_pattern: f.route_path.clone().unwrap_or_else(|| String::from("/")),
            });
        }
    }
    Ok(routes)
}

/// Matches a route pattern (e.g. `"/users/:id"`) against an incoming HTTP request path (e.g. `"/users/42?role=7"`),
/// returning `(primary_param_i64, req_params_by_fnv1a32)`.
pub fn match_omni_route_pattern(
    pattern: &str,
    request_path: &str,
) -> Option<(i64, BTreeMap<i64, i64>)> {
    let (clean_path, query_opt) = match request_path.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (request_path, None),
    };
    let pat_segs: Vec<&str> = pattern.trim_matches('/').split('/').filter(|s| !s.is_empty()).collect();
    let req_segs: Vec<&str> = clean_path.trim_matches('/').split('/').filter(|s| !s.is_empty()).collect();
    if pat_segs.len() != req_segs.len() {
        return None;
    }
    let mut params = BTreeMap::new();
    let mut primary_val: Option<i64> = None;
    for (ps, rs) in pat_segs.into_iter().zip(req_segs.into_iter()) {
        if let Some(param_name) = ps.strip_prefix(':') {
            let parsed = rs.parse::<i64>().unwrap_or_else(|_| fnv1a_32_positive(rs));
            params.insert(fnv1a_32_positive(param_name), parsed);
            if primary_val.is_none() {
                primary_val = Some(parsed);
            }
        } else if ps != rs {
            return None;
        }
    }
    if let Some(q) = query_opt {
        for pair in q.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                let parsed = v.parse::<i64>().unwrap_or_else(|_| fnv1a_32_positive(v));
                params.insert(fnv1a_32_positive(k), parsed);
                if primary_val.is_none() {
                    primary_val = Some(parsed);
                }
            }
        }
    }
    Some((primary_val.unwrap_or(0), params))
}

/// Compiles and statically verifies a `.omni` route program, then dispatches an HTTP request
/// (`method`, `request_path`) against the host's `kv_store` and returns a real `OmniHttpResponse`.
pub fn handle_omni_http_request(
    src: &str,
    method: &str,
    request_path: &str,
    kv_store: &mut BTreeMap<i64, i64>,
) -> Result<OmniHttpResponse, String> {
    let routes = extract_omni_routes(src)?;
    if routes.is_empty() {
        return Err(String::from("error[SYNTAX_ERROR] line 1, col 1: Surface program contains no 'route' handlers"));
    }
    let (u16_stream, sm) = compile_omni_surface_via_pure_omni_frontend_with_sourcemap(src, false)?;
    if let Err(kernel_err) = verify_module_via_phase7_pure_omni_kernel(&u16_stream) {
        for rep in verify_u16_stream_lockstep("serve", &u16_stream).map_err(|e| annotate_omni_error_with_sourcemap(&e, &sm))? {
            if !rep.passed {
                let first = rep.errors.first().cloned().unwrap_or_default();
                let annotated = annotate_omni_error_with_sourcemap(&first, &sm);
                return Err(format!("{} (fn #{}: {:?})", annotated, rep.fn_idx, rep.errors));
            }
        }
        return Err(annotate_omni_error_with_sourcemap(&kernel_err, &sm));
    }
    for rep in verify_u16_stream_lockstep("serve", &u16_stream)? {
        if !rep.passed {
            let first = rep.errors.first().cloned().unwrap_or_default();
            let annotated = annotate_omni_error_with_sourcemap(&first, &sm);
            return Err(format!("{} (fn #{}: {:?})", annotated, rep.fn_idx, rep.errors));
        }
    }
    let module = decode_omni_token_stream(&u16_stream)?;
    let norm_method = method.trim().to_ascii_uppercase();

    for rt in &routes {
        if rt.method == norm_method {
            if let Some((primary_arg, req_params)) = match_omni_route_pattern(&rt.path_pattern, request_path) {
                let (attr_steps, attr_mem) = extract_omni_surface_budgets(src);
                let mut vm = OmniBinaryVm::new();
                if let Some(sb) = attr_steps {
                    vm.step_limit = sb;
                }
                if let Some(mb) = attr_mem {
                    vm.memory_budget_bytes = mb;
                }
                vm.kv_store = kv_store.clone();
                vm.req_params = req_params;
                let val = vm
                    .execute_function(&module, rt.fn_idx, vec![OmniValue::Int(primary_arg)])
                    .map_err(|e| annotate_omni_error_with_sourcemap(&e, &sm))?;
                *kv_store = vm.kv_store;
                if let Some((status, content_type, body)) = vm.last_http_response {
                    return Ok(OmniHttpResponse {
                        status,
                        content_type,
                        body,
                    });
                }
                let body = format!(
                    "{{\"status\":200,\"data\":{}}}",
                    format_omni_value_compact(&val)
                );
                return Ok(OmniHttpResponse {
                    status: 200,
                    content_type: String::from("application/json"),
                    body,
                });
            }
        }
    }

    Ok(OmniHttpResponse {
        status: 404,
        content_type: String::from("application/json"),
        body: format!(
            "{{\"status\":404,\"error\":\"No route matched {} {}\"}}",
            norm_method, request_path
        ),
    })
}


fn sandbox_root_for_explicit_path(path: &std::path::Path) -> std::path::PathBuf {
    if path.is_absolute() {
        path.parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(path)
            .to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    }
}

/// Loads a persistent host KV/DB store from disk through the narrow capability-gated host adapter (`StdNarrowHostAdapter`).
pub fn load_omni_kv_file(path: &std::path::Path) -> Result<BTreeMap<i64, i64>, String> {
    let mut host = StdNarrowHostAdapter::with_sandbox_root(sandbox_root_for_explicit_path(path));
    load_omni_kv_via_narrow_host(&mut host, cap_bit(CAP_ID_DB_R), &path.to_string_lossy())
}

/// Atomically persists a host KV/DB store to disk through the narrow capability-gated host adapter (`StdNarrowHostAdapter`).
pub fn save_omni_kv_file(path: &std::path::Path, kv: &BTreeMap<i64, i64>) -> Result<(), String> {
    let mut host = StdNarrowHostAdapter::with_sandbox_root(sandbox_root_for_explicit_path(path));
    save_omni_kv_via_narrow_host(&mut host, cap_bit(CAP_ID_DB_W), &path.to_string_lossy(), kv)
}

/// Compiles and statically verifies `src` up front (failing immediately with `MISSING_CAP`, `TAINT_SINK`, etc.
/// before binding a socket if verification fails), then binds a real `std::net::TcpListener` on `bind_addr`
/// via `StdNarrowHostAdapter` and serves `num_requests` real HTTP/1.1 requests over TCP.
pub fn serve_omni_http_batch(
    src: &str,
    bind_addr: &str,
    initial_kv: BTreeMap<i64, i64>,
    num_requests: usize,
) -> Result<
    (
        std::net::SocketAddr,
        std::thread::JoinHandle<Result<(Vec<OmniHttpResponse>, BTreeMap<i64, i64>), String>>,
    ),
    String,
> {
    serve_omni_http_batch_inner(src, bind_addr, initial_kv, None, num_requests)
}

/// Serves `num_requests` real HTTP/1.1 requests over TCP backed by a persistent on-disk KV/DB file (`db_path`).
pub fn serve_omni_http_batch_with_db_file(
    src: &str,
    bind_addr: &str,
    db_path: &std::path::Path,
    num_requests: usize,
) -> Result<
    (
        std::net::SocketAddr,
        std::thread::JoinHandle<Result<(Vec<OmniHttpResponse>, BTreeMap<i64, i64>), String>>,
    ),
    String,
> {
    let initial_kv = load_omni_kv_file(db_path)?;
    serve_omni_http_batch_inner(
        src,
        bind_addr,
        initial_kv,
        Some(db_path.to_path_buf()),
        num_requests,
    )
}

pub(crate) fn serve_omni_http_batch_inner(
    src: &str,
    bind_addr: &str,
    initial_kv: BTreeMap<i64, i64>,
    db_file: Option<std::path::PathBuf>,
    num_requests: usize,
) -> Result<
    (
        std::net::SocketAddr,
        std::thread::JoinHandle<Result<(Vec<OmniHttpResponse>, BTreeMap<i64, i64>), String>>,
    ),
    String,
> {
    let routes = extract_omni_routes(src)?;
    if routes.is_empty() {
        return Err(String::from("error[SYNTAX_ERROR] line 1, col 1: Surface program contains no 'route' handlers"));
    }
    let (u16_stream, sm) = compile_omni_surface_via_pure_omni_frontend_with_sourcemap(src, false)?;
    if let Err(kernel_err) = verify_module_via_phase7_pure_omni_kernel(&u16_stream) {
        for rep in verify_u16_stream_lockstep("serve", &u16_stream).map_err(|e| annotate_omni_error_with_sourcemap(&e, &sm))? {
            if !rep.passed {
                let first = rep.errors.first().cloned().unwrap_or_default();
                let annotated = annotate_omni_error_with_sourcemap(&first, &sm);
                return Err(format!("{} (fn #{}: {:?})", annotated, rep.fn_idx, rep.errors));
            }
        }
        return Err(annotate_omni_error_with_sourcemap(&kernel_err, &sm));
    }
    for rep in verify_u16_stream_lockstep("serve", &u16_stream)? {
        if !rep.passed {
            let first = rep.errors.first().cloned().unwrap_or_default();
            let annotated = annotate_omni_error_with_sourcemap(&first, &sm);
            return Err(format!("{} (fn #{}: {:?})", annotated, rep.fn_idx, rep.errors));
        }
    }
    let module = decode_omni_token_stream(&u16_stream)?;
    let combined_caps: CapMask = module.functions.iter().fold(0u16, |acc, f| acc | f.cap_mask);

    let mut host = if let Some(ref p) = db_file {
        StdNarrowHostAdapter::with_sandbox_root(sandbox_root_for_explicit_path(p))
    } else {
        StdNarrowHostAdapter::new()
    };
    let (listener_h, addr_str) = host.host_tcp_listen(combined_caps, bind_addr)?;
    let local_addr: std::net::SocketAddr = addr_str
        .parse()
        .map_err(|e| format!("Failed to parse bound SocketAddr '{}': {}", addr_str, e))?;
    let src_owned = String::from(src);

    let handle = std::thread::spawn(move || {
        let mut kv_store = initial_kv;
        let mut responses = Vec::with_capacity(num_requests);
        for _ in 0..num_requests {
            let conn_h = host.host_tcp_accept(combined_caps, listener_h)?;
            let req_buf = host.host_tcp_read(combined_caps, conn_h, 4096)?;
            let req_str = String::from_utf8_lossy(&req_buf);
            let first_line = req_str.lines().next().unwrap_or("GET / HTTP/1.1");
            let mut parts = first_line.split_whitespace();
            let method = parts.next().unwrap_or("GET");
            let req_path = parts.next().unwrap_or("/");
            let resp = handle_omni_http_request(&src_owned, method, req_path, &mut kv_store)?;
            if let Some(ref p) = db_file {
                save_omni_kv_via_narrow_host(
                    &mut host,
                    combined_caps | cap_bit(CAP_ID_DB_W),
                    &p.to_string_lossy(),
                    &kv_store,
                )?;
            }
            let wire = resp.to_http11_wire_bytes();
            let _ = host.host_tcp_write(combined_caps, conn_h, &wire)?;
            let _ = host.host_tcp_close(combined_caps, conn_h)?;
            responses.push(resp);
        }
        let _ = host.host_tcp_close(combined_caps, listener_h)?;
        Ok((responses, kv_store))
    });

    Ok((local_addr, handle))
}

/// Serves a single live HTTP/1.1 request over TCP on `bind_addr` (e.g. `"127.0.0.1:0"`).
pub fn serve_omni_http_once(
    src: &str,
    bind_addr: &str,
    initial_kv: BTreeMap<i64, i64>,
) -> Result<
    (
        std::net::SocketAddr,
        std::thread::JoinHandle<Result<OmniHttpResponse, String>>,
    ),
    String,
> {
    let (addr, batch_handle) = serve_omni_http_batch(src, bind_addr, initial_kv, 1)?;
    let single_handle = std::thread::spawn(move || {
        let (mut resps, _) = batch_handle
            .join()
            .map_err(|_| String::from("HTTP host thread panicked"))??;
        resps
            .pop()
            .ok_or_else(|| String::from("No HTTP response produced"))
    });
    Ok((addr, single_handle))
}

/// Runs a live HTTP/1.1 server for `.omni` `route` handlers (`omni serve <file.omni> [bind_addr] [--max-requests N] [--db <path>]`).
pub fn run_omni_serve_cli(argv: &[String]) -> i32 {
    if argv.is_empty() {
        eprintln!("usage: omni serve <file.omni | \"source\"> [bind_addr] [--max-requests N] [--db <path>]");
        return 2;
    }
    let target = &argv[0];
    let src = if target.ends_with(".omni") && std::path::Path::new(target).is_file() {
        match std::fs::read_to_string(target) {
            Ok(raw) => {
                let parent_dir = std::path::Path::new(target).parent();
                match resolve_omni_surface_includes(&raw, parent_dir, None) {
                    Ok(exp) => exp,
                    Err(e) => {
                        eprintln!("{}", e);
                        return 1;
                    }
                }
            }
            Err(e) => {
                eprintln!("failed to read '{}': {}", target, e);
                return 2;
            }
        }
    } else {
        target.clone()
    };
    let mut bind_addr = String::from("127.0.0.1:8080");
    let mut max_requests = 1usize;
    let mut db_path: Option<std::path::PathBuf> = None;
    let mut i = 1usize;
    while i < argv.len() {
        if argv[i] == "--max-requests" && i + 1 < argv.len() {
            max_requests = argv[i + 1].parse::<usize>().unwrap_or(1).max(1);
            i += 2;
        } else if argv[i] == "--db" && i + 1 < argv.len() {
            db_path = Some(std::path::PathBuf::from(&argv[i + 1]));
            i += 2;
        } else {
            bind_addr = argv[i].clone();
            i += 1;
        }
    }
    let serve_res = match db_path {
        Some(ref p) => serve_omni_http_batch_with_db_file(&src, &bind_addr, p, max_requests),
        None => serve_omni_http_batch(&src, &bind_addr, BTreeMap::new(), max_requests),
    };
    match serve_res {
        Ok((addr, handle)) => {
            println!("Omni HTTP/1.1 + DB/KV host listening on http://{}", addr);
            match handle.join() {
                Ok(Ok((resps, _))) => {
                    println!("Served {} HTTP request(s) cleanly.", resps.len());
                    0
                }
                Ok(Err(e)) => {
                    eprintln!("HTTP host error: {}", e);
                    1
                }
                Err(_) => {
                    eprintln!("HTTP host thread panicked");
                    1
                }
            }
        }
        Err(e) => {
            eprintln!("{}", e);
            1
        }
    }
}

/// Negative and positive selftests for capability-bound FS/DB sandboxing.
/// Returns the number of checks passed (4: in-root write, `..` traversal, absolute escape, symlink escape).
pub fn run_host_sandbox_selftests() -> Result<usize, String> {
    let mut checks = 0usize;
    let tmp = std::env::temp_dir().join(format!("omni_sandbox_{}_{}", std::process::id(), {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)
    }));
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let root = tmp.join("root");
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let caps = cap_bit(CAP_ID_FS) | cap_bit(CAP_ID_DB_R) | cap_bit(CAP_ID_DB_W);
    let cleanup = |dir: &std::path::Path| {
        let _ = std::fs::remove_dir_all(dir);
    };

    let expect_path_escape = |res: Result<u64, String>, kind: &str| -> Result<(), String> {
        match res {
            Ok(_) => {
                cleanup(&tmp);
                Err(format!("sandbox {kind} was not rejected"))
            }
            Err(e) if e.contains("error[PATH_ESCAPE]") => Ok(()),
            Err(e) => {
                cleanup(&tmp);
                Err(format!("sandbox {kind} produced unexpected error: {e}"))
            }
        }
    };

    // 1. In-root access still works.
    {
        let mut host = StdNarrowHostAdapter::with_sandbox_root(&root);
        let fh = match host.host_file_open(caps, "ok.kv", true) {
            Ok(h) => h,
            Err(e) => {
                cleanup(&tmp);
                return Err(format!("in-root host_file_open failed: {e}"));
            }
        };
        if let Err(e) = host.host_file_write(caps, fh, b"hello") {
            cleanup(&tmp);
            return Err(format!("in-root host_file_write failed: {e}"));
        }
        if let Err(e) = host.host_file_close(caps, fh) {
            cleanup(&tmp);
            return Err(format!("in-root host_file_close failed: {e}"));
        }
        match std::fs::read(root.join("ok.kv")) {
            Ok(bytes) if bytes == b"hello" => {}
            Ok(bytes) => {
                cleanup(&tmp);
                return Err(format!("in-root write mismatch: {:?}", bytes));
            }
            Err(e) => {
                cleanup(&tmp);
                return Err(format!("in-root file missing after close: {e}"));
            }
        }
        checks += 1;
    }

    // 2. `..` traversal is rejected before any open.
    {
        let mut host = StdNarrowHostAdapter::with_sandbox_root(&root);
        expect_path_escape(host.host_file_open(caps, "../escape.kv", true), ".. traversal")?;
        expect_path_escape(host.host_file_open(caps, "foo/../../escape.kv", false), "nested .. traversal")?;
        checks += 1;
    }

    // 3. Absolute path outside the root is rejected.
    {
        let mut host = StdNarrowHostAdapter::with_sandbox_root(&root);
        expect_path_escape(host.host_file_open(caps, "/etc/passwd", false), "absolute path")?;
        checks += 1;
    }

    // 4. Symlink escape is rejected.
    {
        let mut host = StdNarrowHostAdapter::with_sandbox_root(&root);
        let outside = tmp.join("outside_secret");
        if let Err(e) = std::fs::write(&outside, b"secret") {
            cleanup(&tmp);
            return Err(format!("failed to write symlink target: {e}"));
        }
        let link = root.join("escape_link");
        #[cfg(unix)]
        {
            if let Err(e) = std::os::unix::fs::symlink(&outside, &link) {
                cleanup(&tmp);
                return Err(format!("failed to create escape symlink: {e}"));
            }
            expect_path_escape(host.host_file_open(caps, "escape_link", false), "symlink escape")?;
            let out_dir = tmp.join("out_dir");
            if let Err(e) = std::fs::create_dir(&out_dir) {
                cleanup(&tmp);
                return Err(format!("failed to create symlink dir: {e}"));
            }
            if let Err(e) = std::fs::write(out_dir.join("x"), b"x") {
                cleanup(&tmp);
                return Err(format!("failed to write symlink dir file: {e}"));
            }
            let dir_link = root.join("dir_link");
            if let Err(e) = std::os::unix::fs::symlink(&out_dir, &dir_link) {
                cleanup(&tmp);
                return Err(format!("failed to create dir symlink: {e}"));
            }
            expect_path_escape(host.host_file_open(caps, "dir_link/x", false), "dir symlink escape")?;
        }
        #[cfg(not(unix))]
        {
            let _ = (outside, link, host);
        }
        checks += 1;
    }

    cleanup(&tmp);
    Ok(checks)
}

