// ==============================================================================
// host_runtime/pure_default.rs  (Audit §6.2) — Production Default Pure-Omni Host
// ==============================================================================
// Extracted from `m2_host/decode_diff.rs` so the production default compile/run/
// verify/include path lives in its own honestly counted host module rather than
// inside the differential oracle test file.
// ==============================================================================

/// Seed-Shrink Phase A3: Pre-scan raw source for quarantined `macro` directives BEFORE include
/// expansion so the reported 1-based `(line, col)` span always points at the user's source file.
pub fn pure_default_reject_macros(src: &str) -> Result<(), String> {
    for (idx, raw_line) in src.lines().enumerate() {
        let trimmed = raw_line.trim_start();
        if trimmed.starts_with("macro ") || trimmed.starts_with("macro\t") {
            let col = raw_line.len().saturating_sub(trimmed.len()) + 1;
            return Err(format!(
                "error[ORACLE_ONLY_MACRO] line {}, col {}: 'macro' is quarantined behind --rust/--oracle; rewrite as a pure fn for the default pure path",
                idx + 1, col
            ));
        }
    }
    Ok(())
}

// ==============================================================================
// Audit §3.1: Hash-pinned native ELF toolchain (`m2.elf`, `m3.elf`, `m4.elf`)
// + Audit §3.3: Per-process decoded seed module caches (`OnceLock`).
// ==============================================================================

pub const NATIVE_M2_ELF_BYTES: usize = 109_076;
pub const NATIVE_M2_ELF_FNV: u64 = 0xE2BF_E6BF_AD8E_0D3E;
pub const NATIVE_M3_ELF_BYTES: usize = 165_900;
pub const NATIVE_M3_ELF_FNV: u64 = 0x54B2_48B6_9704_ED59;
pub const NATIVE_M4_ELF_BYTES: usize = 81_763;
pub const NATIVE_M4_ELF_FNV: u64 = 0xC935_88C4_A4D4_D962;
pub const NATIVE_VERIFIER_ELF_BYTES: usize = 37_960;
pub const NATIVE_VERIFIER_ELF_FNV: u64 = 0x0191_7D97_B6D3_5247;

/// Audit Round 2 L5: Default & maximum fuel for `omni run --fuel N`, plus documented
/// 65,536-word (128 KiB) native M2/M3/M4/verifier input-window threshold (larger inputs
/// fall back cleanly to the interpreted seed VM).
pub const PURE_DEFAULT_FUEL: i64 = 500_000;
pub const PURE_MAX_FUEL: i64 = 50_000_000;
pub const NATIVE_WINDOW_MAX_WORDS: usize = 65_536;

/// Audit Round 2 L6: Derives the standalone `array`-input verifier source deterministically
/// from `pure/verifier_engine.omni` (single source of truth; zero duplicated rules).
pub fn verifier_standalone_source() -> String {
    let mut s = OMNI_VERIFIER_ENGINE_SURFACE.replace("stream: bytes", "stream: array");
    s = s.replace(
        "fn load_u16(stream: array, idx: i64) -> i64 {\n    let off: i64 = idx * 2;\n    let lo: i64 = iget(stream, off);\n    let hi: i64 = iget(stream, off + 1);\n    let v: i64 = lo | (hi << 8);\n    return v;\n}",
        "fn load_u16(stream: array, idx: i64) -> i64 {\n    return iget(stream, idx);\n}",
    );
    s = s.replace(
        "fn load_u16(stream: array, idx: i64) -> i64 {\r\n    let off: i64 = idx * 2;\r\n    let lo: i64 = iget(stream, off);\r\n    let hi: i64 = iget(stream, off + 1);\r\n    let v: i64 = lo | (hi << 8);\r\n    return v;\r\n}",
        "fn load_u16(stream: array, idx: i64) -> i64 {\n    return iget(stream, idx);\n}",
    );
    s = s.replace(
        "    let nbytes: i64 = slen(stream);\n    let nwords: i64 = nbytes / 2;",
        "    let nwords: i64 = slen(stream);",
    );
    s = s.replace(
        "    let nbytes: i64 = slen(stream);\r\n    let nwords: i64 = nbytes / 2;",
        "    let nwords: i64 = slen(stream);",
    );
    s.push_str(
        "\n#[strict]\nfn verifier_standalone_entry(stream: array, api_mode: i64, _a1: i64, _a2: i64) -> bytes {\n    let rep: bytes = omni_verify_module_with_mode(stream, api_mode);\n    let n: i64 = ld_u64(rep, 168) * 8;\n    return ret_slice(rep, 8192, n);\n}\n",
    );
    s
}

fn cached_m2_seed_module() -> Result<&'static BinModule, String> {
    static M2_MOD: std::sync::OnceLock<Result<BinModule, String>> = std::sync::OnceLock::new();
    M2_MOD
        .get_or_init(|| {
            let m2_raw = std::fs::read("pure/m2/m2.omni_bc")
                .map_err(|e| format!("Pure-M2 seed read failed (pure/m2/m2.omni_bc): {}", e))?;
            let m2_u16 = decode_bytes_to_u16_stream(&m2_raw);
            decode_omni_token_stream(&m2_u16)
        })
        .as_ref()
        .map_err(|e| e.clone())
}

fn cached_m4_seed_module() -> Result<&'static BinModule, String> {
    static M4_MOD: std::sync::OnceLock<Result<BinModule, String>> = std::sync::OnceLock::new();
    M4_MOD
        .get_or_init(|| {
            let m4_raw = std::fs::read("pure/m4/m4.omni_bc")
                .map_err(|e| format!("Pure-M4 seed read failed (pure/m4/m4.omni_bc): {}", e))?;
            let m4_u16 = decode_bytes_to_u16_stream(&m4_raw);
            decode_omni_token_stream(&m4_u16)
        })
        .as_ref()
        .map_err(|e| e.clone())
}

fn cached_verifier_engine_module() -> Result<&'static BinModule, String> {
    static VE_MOD: std::sync::OnceLock<Result<BinModule, String>> = std::sync::OnceLock::new();
    VE_MOD
        .get_or_init(|| {
            let ve_u16 = with_lockstep_suspended(|| {
                compile_omni_surface_strict_to_u16(OMNI_VERIFIER_ENGINE_SURFACE)
            })?;
            decode_omni_token_stream(&ve_u16)
        })
        .as_ref()
        .map_err(|e| e.clone())
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn current_euid() -> u32 {
    extern "C" {
        fn geteuid() -> u32;
    }
    unsafe { geteuid() }
}

/// Audit Round-3 L-C: one stable per-user fallback cache directory
/// (`$TMPDIR/omni-native-cache-<euid>`, default `/tmp`) when `XDG_CACHE_HOME`/`HOME` are
/// unset or empty, so consecutive runs reuse the cached native ELFs instead of leaking a fresh
/// `mkdtemp` directory per process. The caller creates it 0700 and runs it through
/// `verify_private_cache_dir` (real dir, owned by euid, mode & 077 == 0), so a directory
/// pre-created by another user is rejected (`CACHE_UNSAFE_DIR`), never used.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn fallback_user_cache_dir() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("omni-native-cache-{}", current_euid()))
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn verify_private_cache_dir(dir: &std::path::Path) -> Result<(), String> {
    use std::os::unix::fs::MetadataExt;
    let meta = std::fs::symlink_metadata(dir).map_err(|e| {
        format!("error[CACHE_UNSAFE_DIR]: lstat {}: {}", dir.display(), e)
    })?;
    if !meta.file_type().is_dir() {
        return Err(format!(
            "error[CACHE_UNSAFE_DIR]: {} is not a real directory (symlink/non-dir rejected)",
            dir.display()
        ));
    }
    let euid = current_euid();
    if meta.uid() != euid {
        return Err(format!(
            "error[CACHE_UNSAFE_DIR]: {} owned by uid {} != euid {}",
            dir.display(),
            meta.uid(),
            euid
        ));
    }
    if (meta.mode() & 0o077) != 0 {
        return Err(format!(
            "error[CACHE_UNSAFE_DIR]: {} has loose permissions {:o} (require 0700 / mode & 077 == 0)",
            dir.display(),
            meta.mode() & 0o777
        ));
    }
    Ok(())
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn native_toolchain_cache_dir() -> Result<std::path::PathBuf, String> {
    use std::os::unix::fs::DirBuilderExt;
    let base = if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        if !xdg.is_empty() {
            std::path::PathBuf::from(xdg).join("omni").join("native-v1")
        } else {
            fallback_user_cache_dir()
        }
    } else if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            std::path::PathBuf::from(home).join(".cache").join("omni").join("native-v1")
        } else {
            fallback_user_cache_dir()
        }
    } else {
        fallback_user_cache_dir()
    };
    if !base.exists() {
        let mut b = std::fs::DirBuilder::new();
        b.recursive(true).mode(0o700);
        if let Err(e) = b.create(&base) {
            // A concurrent process may have created it first; verify_private_cache_dir decides.
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(format!(
                    "error[CACHE_CREATE_DIR]: create native cache dir {}: {}",
                    base.display(),
                    e
                ));
            }
        }
    }
    verify_private_cache_dir(&base)?;
    Ok(base)
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn spawn_verified_native_elf(
    path: &std::path::Path,
    expected_len: usize,
    expected_fnv: u64,
    args: &[&std::ffi::OsStr],
) -> Result<std::process::Output, String> {
    use std::io::Read;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    use std::os::unix::io::AsRawFd;
    const O_NOFOLLOW: i32 = 0x20000;
    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(O_NOFOLLOW)
        .open(path)
        .map_err(|e| format!("error[CACHE_IO]: open verified native elf {}: {}", path.display(), e))?;
    let meta = f
        .metadata()
        .map_err(|e| format!("error[CACHE_IO]: fstat native elf {}: {}", path.display(), e))?;
    let euid = current_euid();
    if !meta.is_file() || meta.uid() != euid || (meta.mode() & 0o022) != 0 {
        return Err(format!(
            "error[CACHE_UNSAFE_ELF]: {} failed fstat check (uid={}, mode={:o})",
            path.display(),
            meta.uid(),
            meta.mode() & 0o777
        ));
    }
    let mut bytes = Vec::with_capacity(expected_len);
    f.read_to_end(&mut bytes)
        .map_err(|e| format!("error[CACHE_IO]: read verified native elf {}: {}", path.display(), e))?;
    let got_fnv = fnv1a_64_hash(&bytes);
    if bytes.len() != expected_len || got_fnv != expected_fnv {
        return Err(format!(
            "error[NATIVE_ELF_PIN_MISMATCH]: {} pin mismatch before spawn: got {} B (0x{:016X}), expected {} B (0x{:016X})",
            path.display(),
            bytes.len(),
            got_fnv,
            expected_len,
            expected_fnv
        ));
    }
    let fd_path = format!("/proc/self/fd/{}", f.as_raw_fd());
    let out = std::process::Command::new(&fd_path)
        .args(args)
        .output()
        .map_err(|e| format!("spawn verified native elf {} ({}): {}", path.display(), fd_path, e))?;
    drop(f);
    Ok(out)
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
struct PrivateTempDataFile {
    path: std::path::PathBuf,
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl PrivateTempDataFile {
    fn create(prefix: &str, bytes: &[u8]) -> Result<Self, String> {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = native_toolchain_cache_dir()?;
        let pid = std::process::id();
        let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let path = dir.join(format!("{}_{}_{}_{}.tmp", prefix, pid, seq, nanos));
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .map_err(|e| format!("error[CACHE_IO]: create temp input {}: {}", path.display(), e))?;
        f.write_all(bytes)
            .map_err(|e| format!("error[CACHE_IO]: write temp input {}: {}", path.display(), e))?;
        Ok(Self { path })
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl Drop for PrivateTempDataFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn ensure_cached_native_elf<F: FnOnce() -> Result<Vec<u8>, String>>(
    name: &str,
    expected_len: usize,
    expected_fnv: u64,
    build_fn: F,
) -> Result<std::path::PathBuf, String> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let dir = native_toolchain_cache_dir()?;
    let final_path = dir.join(format!("{}_{:016x}.elf", name, expected_fnv));
    if let Ok(existing) = std::fs::read(&final_path) {
        if existing.len() == expected_len && fnv1a_64_hash(&existing) == expected_fnv {
            return Ok(final_path);
        }
        let _ = std::fs::remove_file(&final_path);
    }
    let elf = build_fn()?;
    let got_fnv = fnv1a_64_hash(&elf);
    if elf.len() != expected_len || got_fnv != expected_fnv {
        return Err(format!(
            "native {}.elf pin mismatch: got {} B (0x{:016X}), expected {} B (0x{:016X})",
            name,
            elf.len(),
            got_fnv,
            expected_len,
            expected_fnv
        ));
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp_path = dir.join(format!(".{}_{}_{}.tmp", name, std::process::id(), nanos));
    {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o700)
            .open(&tmp_path)
            .map_err(|e| format!("error[CACHE_IO]: create native elf tmp {}: {}", tmp_path.display(), e))?;
        f.write_all(&elf)
            .map_err(|e| format!("error[CACHE_IO]: write native elf tmp {}: {}", tmp_path.display(), e))?;
    }
    std::fs::rename(&tmp_path, &final_path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp_path);
        format!("error[CACHE_IO]: rename native elf {}: {}", final_path.display(), e)
    })?;
    Ok(final_path)
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn ensure_native_m2_elf() -> Result<&'static std::path::PathBuf, String> {
    static M2_ELF: std::sync::OnceLock<Result<std::path::PathBuf, String>> = std::sync::OnceLock::new();
    M2_ELF
        .get_or_init(|| {
            ensure_cached_native_elf("m2", NATIVE_M2_ELF_BYTES, NATIVE_M2_ELF_FNV, || {
                let m2_raw = std::fs::read("pure/m2/m2.omni_bc")
                    .map_err(|e| format!("read pure/m2/m2.omni_bc: {}", e))?;
                let m2_u16 = decode_bytes_to_u16_stream(&m2_raw);
                pure_default_compile_u16_to_elf_interpreted(&m2_u16, 0, 0, 0)
            })
        })
        .as_ref()
        .map_err(|e| e.clone())
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn ensure_native_m3_elf() -> Result<&'static std::path::PathBuf, String> {
    static M3_ELF: std::sync::OnceLock<Result<std::path::PathBuf, String>> = std::sync::OnceLock::new();
    M3_ELF
        .get_or_init(|| {
            ensure_cached_native_elf("m3", NATIVE_M3_ELF_BYTES, NATIVE_M3_ELF_FNV, || {
                let m2_elf = ensure_native_m2_elf()?;
                let out = spawn_verified_native_elf(
                    m2_elf,
                    NATIVE_M2_ELF_BYTES,
                    NATIVE_M2_ELF_FNV,
                    &[
                        std::ffi::OsStr::new("pure/m3/m3_standalone.omni_bc"),
                        std::ffi::OsStr::new("0"),
                        std::ffi::OsStr::new("0"),
                        std::ffi::OsStr::new("0"),
                    ],
                )?;
                if !out.status.success() {
                    return Err(format!(
                        "m2.elf failed building m3.elf with status {:?}",
                        out.status.code()
                    ));
                }
                Ok(out.stdout)
            })
        })
        .as_ref()
        .map_err(|e| e.clone())
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn ensure_native_m4_elf() -> Result<&'static std::path::PathBuf, String> {
    static M4_ELF: std::sync::OnceLock<Result<std::path::PathBuf, String>> = std::sync::OnceLock::new();
    M4_ELF
        .get_or_init(|| {
            ensure_cached_native_elf("m4", NATIVE_M4_ELF_BYTES, NATIVE_M4_ELF_FNV, || {
                let m2_elf = ensure_native_m2_elf()?;
                let out = spawn_verified_native_elf(
                    m2_elf,
                    NATIVE_M2_ELF_BYTES,
                    NATIVE_M2_ELF_FNV,
                    &[
                        std::ffi::OsStr::new("pure/m4/m4_standalone.omni_bc"),
                        std::ffi::OsStr::new("0"),
                        std::ffi::OsStr::new("0"),
                        std::ffi::OsStr::new("0"),
                    ],
                )?;
                if !out.status.success() {
                    return Err(format!(
                        "m2.elf failed building m4.elf with status {:?}",
                        out.status.code()
                    ));
                }
                Ok(out.stdout)
            })
        })
        .as_ref()
        .map_err(|e| e.clone())
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn ensure_native_verifier_elf() -> Result<&'static std::path::PathBuf, String> {
    static VERIFIER_ELF: std::sync::OnceLock<Result<std::path::PathBuf, String>> =
        std::sync::OnceLock::new();
    VERIFIER_ELF
        .get_or_init(|| {
            ensure_cached_native_elf(
                "verifier",
                NATIVE_VERIFIER_ELF_BYTES,
                NATIVE_VERIFIER_ELF_FNV,
                || {
                    let m2_elf = ensure_native_m2_elf()?;
                    let out = spawn_verified_native_elf(
                        m2_elf,
                        NATIVE_M2_ELF_BYTES,
                        NATIVE_M2_ELF_FNV,
                        &[
                            std::ffi::OsStr::new("pure/verifier_standalone.omni_bc"),
                            std::ffi::OsStr::new("0"),
                            std::ffi::OsStr::new("0"),
                            std::ffi::OsStr::new("0"),
                        ],
                    )?;
                    if !out.status.success() {
                        return Err(format!(
                            "m2.elf failed building verifier.elf with status {:?}",
                            out.status.code()
                        ));
                    }
                    Ok(out.stdout)
                },
            )
        })
        .as_ref()
        .map_err(|e| e.clone())
}

/// Seed-Shrink Phase A1/A3 + Audit §3.1: Compile `.omni` surface source using ONLY the committed
/// Pure-Omni M3 seed (hash-pinned native `m3.elf` on Linux x86-64 when `src.len() <= 65_536` and
/// not forced interpreted; otherwise interpreted `pure/m3/m3.omni_bc`) — zero Rust frontend calls,
/// and hard-rejects Rust-only `macro` directives with `error[ORACLE_ONLY_MACRO]`.
pub fn pure_default_compile_surface_to_u16(src: &str, strict: bool) -> Result<Vec<u16>, String> {
    pure_default_reject_macros(src)?;
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    if !pure_default_force_interpreted()
        && src.len() <= 65_536
        && std::path::Path::new("pure/m3/m3_standalone.omni_bc").exists()
    {
        let m3_elf = ensure_native_m3_elf()?;
        let mut u16_bytes = Vec::with_capacity(src.len() * 2);
        for &b in src.as_bytes() {
            u16_bytes.extend_from_slice(&(b as u16).to_le_bytes());
        }
        let tmp = PrivateTempDataFile::create("m3_in", &u16_bytes)?;
        let out = spawn_verified_native_elf(
            m3_elf,
            NATIVE_M3_ELF_BYTES,
            NATIVE_M3_ELF_FNV,
            &[
                tmp.path.as_os_str(),
                std::ffi::OsStr::new(if strict { "1" } else { "0" }),
            ],
        )?;
        if !out.status.success() {
            return Err(format!(
                "error[M3_NATIVE_TRAP]: native m3.elf exited with {:?}",
                out.status.code()
            ));
        }
        let out_bytes = out.stdout;
        if out_bytes.len() == 16 && u16::from_le_bytes([out_bytes[0], out_bytes[1]]) == 0xFFFF {
            let code = u16::from_le_bytes([out_bytes[2], out_bytes[3]]);
            let line = u16::from_le_bytes([out_bytes[4], out_bytes[5]]);
            let col = u16::from_le_bytes([out_bytes[6], out_bytes[7]]);
            return Err(format!(
                "error[M3_CODE_{}] line {}, col {}: Pure-Omni M3 frontend rejected input",
                code, line, col
            ));
        }
        if out_bytes.len() % 2 != 0 {
            return Err(format!("native m3.elf returned odd byte length {}", out_bytes.len()));
        }
        let words: Vec<u16> = out_bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        return Ok(words);
    }
    match m3_lockstep_compile_via_omni(src, strict)? {
        Ok(u16_stream) => Ok(u16_stream),
        Err((code, line, col)) => Err(format!(
            "error[M3_CODE_{}] line {}, col {}: Pure-Omni M3 frontend rejected input",
            code, line, col
        )),
    }
}

/// Interpreted Pure-M2 path (`pure/m2/m2.omni_bc` on `OmniBinaryVm`), used on non-Linux hosts,
/// for `--interpreted` / `OMNI_FORCE_INTERPRETED=1` differential tests, and to bootstrap `m2.elf` once.
pub fn pure_default_compile_u16_to_elf_interpreted(
    u16_stream: &[u16],
    a0: i64,
    a1: i64,
    a2: i64,
) -> Result<Vec<u8>, String> {
    let m2_mod = cached_m2_seed_module()?;
    let m2_entry = m2_entry_of(m2_mod);
    let mut vm_m2 = OmniBinaryVm::with_budgets(500_000_000, 128 * 1024 * 1024);
    match vm_m2.execute_function(
        m2_mod,
        m2_entry,
        vec![
            OmniValue::U16Array(u16_stream.to_vec()),
            OmniValue::Int(a0),
            OmniValue::Int(a1),
            OmniValue::Int(a2),
        ],
    )? {
        OmniValue::Bytes(elf) => Ok(elf),
        other => Err(format!("Pure-M2 seed returned unexpected value: {:?}", other)),
    }
}

/// Seed-Shrink Phase A1 + Audit §3.1: Lower a `.omni_bc` u16 stream to native Linux x86-64 ELF64
/// using ONLY the committed Pure-Omni M2 seed (hash-pinned native `m2.elf` on Linux x86-64 when
/// not forced interpreted; otherwise interpreted `pure/m2/m2.omni_bc`) — zero Rust emitter calls.
pub fn pure_default_compile_u16_to_elf(u16_stream: &[u16], a0: i64, a1: i64, a2: i64) -> Result<Vec<u8>, String> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    if !pure_default_force_interpreted()
        && u16_stream.len() <= 65_536
        && std::path::Path::new("pure/m2/m2.omni_bc").exists()
    {
        let m2_elf = ensure_native_m2_elf()?;
        let bc_bytes = encode_u16_stream_to_bytes(u16_stream);
        let tmp = PrivateTempDataFile::create("m2_in", &bc_bytes)?;
        let s0 = a0.to_string();
        let s1 = a1.to_string();
        let s2 = a2.to_string();
        let out = spawn_verified_native_elf(
            m2_elf,
            NATIVE_M2_ELF_BYTES,
            NATIVE_M2_ELF_FNV,
            &[
                tmp.path.as_os_str(),
                std::ffi::OsStr::new(&s0),
                std::ffi::OsStr::new(&s1),
                std::ffi::OsStr::new(&s2),
            ],
        )?;
        if !out.status.success() {
            let code = out.status.code();
            let (tag, reason) = match code {
                Some(1) => ("error[VERIFY_FAILED]", "input stream failed M2 verification or was empty"),
                Some(100) => ("error[DECODE_ERROR]", "input stream truncated or malformed"),
                Some(101) => ("error[VERIFY_OR_OOB]", "M2 verifier rejected stream (bad op/arity/call) or bounds trap"),
                Some(102) => ("error[BUDGET_EXCEEDED]", "M2 arena output capacity exceeded"),
                _ => ("error[M2_NATIVE_TRAP]", "native m2.elf exited abnormally"),
            };
            return Err(format!("{}: native m2.elf rejected input (exit {:?}: {})", tag, code, reason));
        }
        return Ok(out.stdout);
    }
    pure_default_compile_u16_to_elf_interpreted(u16_stream, a0, a1, a2)
}

fn decode_m4_vm_state_64(st_bytes: &[u8]) -> Result<(i64, i64), String> {
    if st_bytes.len() < 64 {
        return Err(format!(
            "error[VM_TRAP]: Pure-M4 VM state truncated ({} B < 64 B)",
            st_bytes.len()
        ));
    }
    let status = u32::from_le_bytes(st_bytes[28..32].try_into().unwrap()) as i64;
    let ret = i64::from_le_bytes(st_bytes[32..40].try_into().unwrap());
    let steps = u32::from_le_bytes(st_bytes[44..48].try_into().unwrap()) as i64;
    if status != 0 {
        let prefix = match status {
            2 => OMNI_OOB_ERROR,
            3 | 4 => "error[BUDGET_EXCEEDED]",
            1 => "error[DECODE_ERROR]",
            _ => "error[VM_TRAP]",
        };
        return Err(format!("{}: Pure-M4 VM trapped with status={} after {} steps", prefix, status, steps));
    }
    Ok((ret, steps))
}

/// Seed-Shrink Phase A1 + Audit §3.1 + Audit Round 2 L5: Execute a `.omni_bc` u16 stream using ONLY
/// the committed Pure-Omni M4 VM seed with configurable `fuel` (1..=PURE_MAX_FUEL). Uses hash-pinned
/// native `m4.elf` on Linux x86-64 when `fuel == PURE_DEFAULT_FUEL`, `!trace`, `u16_stream.len() <= NATIVE_WINDOW_MAX_WORDS`,
/// and not forced interpreted; otherwise executes `pure/m4/m4.omni_bc` with the requested `fuel`.
pub fn pure_default_run_u16_with_fuel(
    u16_stream: &[u16],
    a0: i64,
    a1: i64,
    a2: i64,
    fuel: i64,
    trace: bool,
) -> Result<(i64, i64), String> {
    if fuel <= 0 || fuel > PURE_MAX_FUEL {
        return Err(format!(
            "error[INVALID_ARG]: --fuel must be in 1..={} (got {})",
            PURE_MAX_FUEL, fuel
        ));
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    if !pure_default_force_interpreted()
        && !trace
        && fuel == PURE_DEFAULT_FUEL
        && u16_stream.len() <= NATIVE_WINDOW_MAX_WORDS
        && std::path::Path::new("pure/m4/m4_standalone.omni_bc").exists()
    {
        let m4_elf = ensure_native_m4_elf()?;
        let bc_bytes = encode_u16_stream_to_bytes(u16_stream);
        let tmp = PrivateTempDataFile::create("m4_in", &bc_bytes)?;
        let s0 = a0.to_string();
        let s1 = a1.to_string();
        let s2 = a2.to_string();
        let out = spawn_verified_native_elf(
            m4_elf,
            NATIVE_M4_ELF_BYTES,
            NATIVE_M4_ELF_FNV,
            &[
                tmp.path.as_os_str(),
                std::ffi::OsStr::new(&s0),
                std::ffi::OsStr::new(&s1),
                std::ffi::OsStr::new(&s2),
            ],
        )?;
        if !out.status.success() {
            let code = out.status.code();
            let (tag, reason) = match code {
                Some(100) => ("error[DECODE_ERROR]", "input stream truncated or malformed"),
                Some(101) => (OMNI_OOB_ERROR, "bounds/memory trap"),
                Some(102) => ("error[BUDGET_EXCEEDED]", "arena capacity exceeded"),
                _ => ("error[VM_TRAP]", "native m4.elf exited abnormally"),
            };
            return Err(format!("{}: native m4.elf trapped (exit {:?}: {})", tag, code, reason));
        }
        return decode_m4_vm_state_64(&out.stdout);
    }
    let m4_mod = cached_m4_seed_module()?;
    let m4_run_fn = m4_mod.functions[m4_mod.functions.len() - 2].fn_idx;
    let vm_budget = ((fuel as u64).saturating_mul(2000)).max(500_000_000);
    let mut vm = OmniBinaryVm::with_budgets(vm_budget, 128 * 1024 * 1024);
    let arena = match vm.execute_function(
        m4_mod,
        m4_run_fn,
        vec![
            OmniValue::Bytes(vec![0u8; 524_288]),
            OmniValue::U16Array(u16_stream.to_vec()),
            OmniValue::Int(a0),
            OmniValue::Int(a1),
            OmniValue::Int(a2),
            OmniValue::Int(fuel),
            OmniValue::Int(if trace { 1 } else { 0 }),
        ],
    )? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("Pure-M4 VM seed returned unexpected value: {:?}", other)),
    };
    let st = 167_936usize;
    decode_m4_vm_state_64(&arena[st..st + 64])
}

pub fn pure_default_run_u16(
    u16_stream: &[u16],
    a0: i64,
    a1: i64,
    a2: i64,
    trace: bool,
) -> Result<(i64, i64), String> {
    pure_default_run_u16_with_fuel(u16_stream, a0, a1, a2, PURE_DEFAULT_FUEL, trace)
}

/// Seed-Shrink Phase A2 (O-5): Pure-Omni include expander driven by `pure/m3/include.omni`.
/// Zero Rust lexer/parser calls (`tokenize_omni_surface` / `SurfaceParser` never invoked);
/// uses Pure-Omni `m3i_scan_includes`, `m3i_strip_include_lines`, `m3i_hash_path`, and `m3i_check_cycle`.
pub fn pure_default_expand_includes(
    src: &str,
    base_dir: Option<&std::path::Path>,
    vfs: Option<&std::collections::BTreeMap<String, String>>,
) -> Result<String, String> {
    // Reject quarantined `macro` directives on the user's raw source BEFORE include expansion
    // so reported `(line, col)` spans refer to the user's file, not post-expansion line numbers.
    pure_default_reject_macros(src)?;
    // Audit Round-3 L-F: only invoke the Pure-Omni include scanner when a line actually starts
    // with an `include ` / `include"` / `mod ` / `mod"` directive (matching `m3i_scan_includes`),
    // avoiding false positives on identifiers like `empty_mod ` or comments mentioning `include`.
    let has_inc_or_mod = src.lines().any(|line| {
        let t = line.trim_start();
        t.starts_with("include ")
            || t.starts_with("include\"")
            || t.starts_with("mod ")
            || t.starts_with("mod\"")
    });
    if !has_inc_or_mod {
        return Ok(String::from(src));
    }
    let inc_u16 = pure_default_compile_surface_to_u16(M3_INCLUDE_OMNI_SRC, true)?;
    let inc_mod = decode_omni_token_stream(&inc_u16)?;
    let n_fns = inc_mod.functions.len();
    // Order at end of spec/m3/include_harness.omni:
    //   m3i_scan_entry (-5), m3i_strip_entry (-4), m3i_hash_entry (-3), m3i_cycle_entry (-2), main (-1)
    let fn_scan = inc_mod.functions[n_fns - 5].fn_idx;
    let fn_strip = inc_mod.functions[n_fns - 4].fn_idx;
    let fn_hash = inc_mod.functions[n_fns - 3].fn_idx;
    let fn_cycle = inc_mod.functions[n_fns - 2].fn_idx;
    let mut vm = OmniBinaryVm::with_budgets(500_000_000, 64 * 1024 * 1024);

    fn expand_rec(
        vm: &mut OmniBinaryVm,
        inc_mod: &BinModule,
        fn_scan: u16,
        fn_strip: u16,
        fn_hash: u16,
        fn_cycle: u16,
        cur_src: &str,
        cur_dir: Option<&std::path::Path>,
        vfs: Option<&std::collections::BTreeMap<String, String>>,
        stack_hashes: &mut Vec<u8>,
    ) -> Result<String, String> {
        let src_bytes = cur_src.as_bytes().to_vec();
        let tbl = match vm.execute_function(inc_mod, fn_scan, vec![OmniValue::Bytes(src_bytes.clone())])? {
            OmniValue::Bytes(b) => b,
            other => return Err(format!("A2 m3i_scan_includes returned {:?}", other)),
        };
        if tbl.len() < 4 {
            return Ok(String::from(cur_src));
        }
        let cnt = u32::from_le_bytes(tbl[0..4].try_into().unwrap()) as usize;
        if cnt == 0 {
            return Ok(String::from(cur_src));
        }
        let mut combined = String::new();
        for i in 0..cnt {
            let off = 4 + i * 16;
            let poff = u32::from_le_bytes(tbl[off..off + 4].try_into().unwrap()) as usize;
            let plen = u32::from_le_bytes(tbl[off + 4..off + 8].try_into().unwrap()) as usize;
            let line = u32::from_le_bytes(tbl[off + 8..off + 12].try_into().unwrap());
            let col = u32::from_le_bytes(tbl[off + 12..off + 16].try_into().unwrap());
            let inc_path = std::str::from_utf8(&src_bytes[poff..poff + plen])
                .map_err(|_| format!("error[SYNTAX_ERROR] line {}, col {}: Invalid UTF-8 in include path", line, col))?
                .to_string();
            let phash = match vm.execute_function(
                inc_mod,
                fn_hash,
                vec![
                    OmniValue::Bytes(src_bytes.clone()),
                    OmniValue::Int(poff as i64),
                    OmniValue::Int(plen as i64),
                ],
            )? {
                OmniValue::Int(v) => v,
                other => return Err(format!("A2 m3i_hash_path returned {:?}", other)),
            };
            let depth = (stack_hashes.len() / 4) as i64;
            let is_cyc = match vm.execute_function(
                inc_mod,
                fn_cycle,
                vec![
                    OmniValue::Bytes(stack_hashes.clone()),
                    OmniValue::Int(depth),
                    OmniValue::Int(phash),
                ],
            )? {
                OmniValue::Int(v) => v,
                other => return Err(format!("A2 m3i_check_cycle returned {:?}", other)),
            };
            if is_cyc != 0 {
                return Err(format!(
                    "error[CIRCULAR_INCLUDE] line {}, col {}: Circular include detected for '{}'",
                    line, col, inc_path
                ));
            }
            let inc_content = if let Some(s) = vfs.and_then(|m| m.get(&inc_path)) {
                s.clone()
            } else if let Some(std_src) = lookup_omni_stdlib_module(&inc_path) {
                String::from(std_src)
            } else if let Some(dir) = cur_dir {
                let full_p = dir.join(&inc_path);
                std::fs::read_to_string(&full_p).map_err(|e| {
                    format!(
                        "error[SYNTAX_ERROR] line {}, col {}: Failed to read included file '{}': {}",
                        line, col, full_p.display(), e
                    )
                })?
            } else {
                std::fs::read_to_string(&inc_path).map_err(|e| {
                    format!(
                        "error[SYNTAX_ERROR] line {}, col {}: Failed to read included file '{}': {}",
                        line, col, inc_path, e
                    )
                })?
            };
            stack_hashes.extend_from_slice(&(phash as u32).to_le_bytes());
            let expanded = expand_rec(
                vm, inc_mod, fn_scan, fn_strip, fn_hash, fn_cycle,
                &inc_content, cur_dir, vfs, stack_hashes,
            )?;
            let new_len = stack_hashes.len().saturating_sub(4);
            stack_hashes.truncate(new_len);
            combined.push_str(&expanded);
            combined.push('\n');
        }
        let body_bytes = match vm.execute_function(inc_mod, fn_strip, vec![OmniValue::Bytes(src_bytes)])? {
            OmniValue::Bytes(b) => b,
            other => return Err(format!("A2 m3i_strip_include_lines returned {:?}", other)),
        };
        let body_str = String::from_utf8(body_bytes)
            .map_err(|e| format!("A2 stripped body UTF-8 error: {}", e))?;
        combined.push_str(&body_str);
        Ok(combined)
    }

    let mut stack_hashes: Vec<u8> = Vec::new();
    expand_rec(
        &mut vm, &inc_mod, fn_scan, fn_strip, fn_hash, fn_cycle,
        src, base_dir, vfs, &mut stack_hashes,
    )
}

/// Seed-Shrink Phase B2: Verify a `.omni_bc` u16 token stream using ONLY the Pure-Omni verifier
/// (`pure/verifier_engine.omni`: `omni_verify_module_api` when `api_mode=true`, `omni_verify_module`
/// otherwise) â€” ZERO calls to the frozen Rust verifier on the default hot path.
pub fn pure_default_verify_u16(u16_stream: &[u16], api_mode: bool) -> Result<(), String> {
    let rep: Vec<u8> = {
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        let native_rep = if !pure_default_force_interpreted()
            && u16_stream.len() <= NATIVE_WINDOW_MAX_WORDS
            && std::path::Path::new("pure/verifier_standalone.omni_bc").exists()
        {
            let v_elf = ensure_native_verifier_elf()?;
            let bc_bytes = encode_u16_stream_to_bytes(u16_stream);
            let tmp = PrivateTempDataFile::create("ve_in", &bc_bytes)?;
            let mode_s = if api_mode { "1" } else { "0" };
            let out = spawn_verified_native_elf(
                v_elf,
                NATIVE_VERIFIER_ELF_BYTES,
                NATIVE_VERIFIER_ELF_FNV,
                &[
                    tmp.path.as_os_str(),
                    std::ffi::OsStr::new(mode_s),
                    std::ffi::OsStr::new("0"),
                    std::ffi::OsStr::new("0"),
                ],
            )?;
            if !out.status.success() {
                return Err(format!(
                    "error[PURE_VERIFY_NATIVE_TRAP]: native verifier.elf exited with {:?}",
                    out.status.code()
                ));
            }
            Some(out.stdout)
        } else {
            None
        };
        #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
        let native_rep: Option<Vec<u8>> = None;

        if let Some(b) = native_rep {
            b
        } else {
            let ve_mod = cached_verifier_engine_module()?;
            let fn_verify = if api_mode {
                ve_mod.functions[ve_mod.functions.len() - 2].fn_idx
            } else {
                ve_mod.functions[ve_mod.functions.len() - 1].fn_idx
            };
            let mut vm = OmniBinaryVm::with_budgets(500_000_000, 64 * 1024 * 1024);
            match vm.execute_function(
                ve_mod,
                fn_verify,
                vec![OmniValue::Bytes(encode_u16_stream_to_bytes(u16_stream))],
            )? {
                OmniValue::Bytes(b) => b,
                other => return Err(format!("Pure verifier returned unexpected value: {:?}", other)),
            }
        }
    };
    if rep.len() < 24 {
        return Err(String::from("error[PURE_VERIFY_MALFORMED]: Pure verifier report truncated"));
    }
    let r_i64 = |idx: usize| -> i64 {
        i64::from_le_bytes(rep[idx * 8..idx * 8 + 8].try_into().unwrap())
    };
    let status = r_i64(0);
    let dec_off = r_i64(1);
    if status == 3007 {
        return Err(String::from(
            "error[PURE_VERIFY_EMPTY_MODULE] @word 0: Pure-Omni verifier rejected zero-function module",
        ));
    }
    if status == 3008 {
        return Err(format!(
            "error[PURE_VERIFY_REPORT_OVERFLOW] @word {}: Pure-Omni verifier report buffer overflowed",
            dec_off
        ));
    }
    if status != 0 {
        return Err(format!(
            "error[PURE_VERIFY_DECODE_{}] @word {}: Pure-Omni verifier rejected malformed stream",
            status, dec_off
        ));
    }
    let nfns = r_i64(2) as usize;
    let total_words = rep.len() / 8;
    let mut cur = 3usize;
    let mut read_fns = 0usize;
    for _ in 0..nfns {
        if cur + 3 > total_words {
            return Err(format!(
                "error[PURE_VERIFY_TRUNCATED]: Pure-Omni verifier report truncated at fn {}/{} (cur={}, words={})",
                read_fns, nfns, cur, total_words
            ));
        }
        let fn_idx = r_i64(cur);
        let passed = r_i64(cur + 1);
        let nerr = r_i64(cur + 2) as usize;
        if cur + 3 + nerr.saturating_mul(2) > total_words {
            return Err(format!(
                "error[PURE_VERIFY_TRUNCATED]: Pure-Omni verifier error list truncated at fn#{} (nerr={}, cur={}, words={})",
                fn_idx, nerr, cur, total_words
            ));
        }
        if passed == 0 || nerr > 0 {
            let code = if nerr > 0 { r_i64(cur + 3) } else { 0 };
            let off = if nerr > 0 { r_i64(cur + 4) } else { 0 };
            return Err(format!(
                "error[PURE_VERIFY_{}] fn#{} @word {}: Pure-Omni verifier rejected function ({} error(s))",
                code, fn_idx, off, nerr
            ));
        }
        cur += 3 + nerr * 2;
        read_fns += 1;
    }
    if read_fns != nfns {
        return Err(format!(
            "error[PURE_VERIFY_TRUNCATED]: Pure-Omni verifier read {} of {} function entries",
            read_fns, nfns
        ));
    }
    Ok(())
}

thread_local! {
    static PURE_DEFAULT_FORCE_INTERPRETED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Returns true when the caller explicitly requested the interpreted Pure-Omni VM path
/// via `--interpreted` or `OMNI_FORCE_INTERPRETED=1` (for differential testing).
pub fn pure_default_force_interpreted() -> bool {
    if PURE_DEFAULT_FORCE_INTERPRETED.with(|c| c.get()) {
        return true;
    }
    matches!(
        std::env::var("OMNI_FORCE_INTERPRETED").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes")
    )
}

/// Runs `f` with the interpreted Pure-Omni VM path forced on the current thread.
pub fn with_pure_default_interpreted<T, F: FnOnce() -> T>(f: F) -> T {
    let prev = PURE_DEFAULT_FORCE_INTERPRETED.with(|c| {
        let p = c.get();
        c.set(true);
        p
    });
    let out = f();
    PURE_DEFAULT_FORCE_INTERPRETED.with(|c| c.set(prev));
    out
}

