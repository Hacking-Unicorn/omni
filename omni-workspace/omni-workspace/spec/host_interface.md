# Omni Narrow Capability-Gated Host Interface (`omni-narrow-host-v1.0.0`)

**Contract Version:** `omni-narrow-host-v1.0.0`  
**Trait:** `OmniNarrowHost` (`omni_rust_bootstrap.rs` Section `7E`, `crates/omni-lang`)

## 1. Purpose & Trust Boundary

To resolve the tension between pure-Omni self-hosting and practical full-stack execution (`route`, HTTP/1.1, and persistent KV/DB storage), Omni defines a minimal, explicit 10-syscall boundary (`OmniNarrowHost`).

- **Trusted Boundary (Outside Pure Omni):** Only raw socket byte I/O, raw file byte I/O, and the monotonic nanosecond clock live in `OmniNarrowHost`.
- **Untrusted / Verified Pure-Omni Layer:** HTTP/1.1 request line parsing, `:id` route parameter extraction, FNV-1a key hashing, KV record serialization (`<key>=<val>\n`), capability bitmask verification, and taint tracking are executed above this narrow boundary.
- **Standalone AOT Separation (`SPEC.md` Section 6):** Standalone `x86-64` ELF64 and Wasm AOT backends continue to explicitly reject host opcodes (`0x7F64..=0x7F70`). Programs using host opcodes execute via the capability-gated `OmniBinaryVm` + `OmniNarrowHost` runtime.

## 2. The 10 Capability-Gated Syscalls

Every operation on `OmniNarrowHost` takes the caller's verified `CapMask` as its first argument and returns `error[MISSING_CAP]` immediately if the required capability bit is absent:

| # | Syscall | Signature | Required Capability | Purpose |
|---|---|---|---|---|
| 1 | `host_tcp_listen` | `(cap_mask, bind_addr: &str) -> Result<(u64, String), String>` | `CAP_ID_NET` (`bit 2`, `0x0004`) | Bind a listening TCP socket and return `(listener_handle, local_addr)` |
| 2 | `host_tcp_accept` | `(cap_mask, listener_handle: u64) -> Result<u64, String>` | `CAP_ID_NET` (`bit 2`, `0x0004`) | Accept one incoming TCP stream and return `conn_handle` |
| 3 | `host_tcp_read` | `(cap_mask, conn_handle: u64, max_bytes: usize) -> Result<Vec<u8>, String>` | `CAP_ID_NET` (`bit 2`, `0x0004`) | Read up to `max_bytes` from `conn_handle` |
| 4 | `host_tcp_write` | `(cap_mask, conn_handle: u64, data: &[u8]) -> Result<usize, String>` | `CAP_ID_NET` (`bit 2`, `0x0004`) | Write wire response bytes to `conn_handle` |
| 5 | `host_tcp_close` | `(cap_mask, handle: u64) -> Result<(), String>` | `CAP_ID_NET` (`bit 2`, `0x0004`) | Close a TCP listener or connection handle |
| 6 | `host_file_open` | `(cap_mask, path: &str, write: bool) -> Result<u64, String>` | `CAP_ID_DB_R` (`bit 0`, `0x0001`) or `CAP_ID_DB_W` (`bit 1`, `0x0002`) | Open a KV/DB backing store file for reading or writing |
| 7 | `host_file_read` | `(cap_mask, file_handle: u64) -> Result<Vec<u8>, String>` | `CAP_ID_DB_R` (`bit 0`, `0x0001`) | Read all bytes from an open file handle |
| 8 | `host_file_write` | `(cap_mask, file_handle: u64, data: &[u8]) -> Result<usize, String>` | `CAP_ID_DB_W` (`bit 1`, `0x0002`) | Write serialized KV/DB bytes to an open file handle |
| 9 | `host_file_close` | `(cap_mask, file_handle: u64) -> Result<(), String>` | `CAP_ID_DB_R` or `CAP_ID_DB_W` | Flush and close an open file handle |
| 10 | `host_time_ns` | `(cap_mask) -> Result<u64, String>` | `CAP_ID_SYS` (`bit 9`, `0x0200`) | Read monotonic nanosecond clock |

## 3. Mapping of Host Opcodes (`0x7F64..=0x7F70`)

| Opcode | Mnemonic | Surface Form | Implementation over `OmniNarrowHost` |
|---|---|---|---|
| `0x7F64` | `DB_GET` | `db_get(k)` / `kv_get(k)` | Reads in-memory KV state populated via `load_omni_kv_via_narrow_host` (`host_file_open` + `host_file_read` + `host_file_close`) |
| `0x7F65` | `DB_QRY` | `db_query(sql, p)` | Evaluates parameterized query over KV state loaded via `OmniNarrowHost` (requires `@SAFE` parameter) |
| `0x7F68` | `DB_CNT` | `db_count(k)` | Counts matching KV entries loaded via `OmniNarrowHost` |
| `0x7F6B` | `KV_PUT` | `kv_put(k, v)` | Updates KV state and persists via `save_omni_kv_via_narrow_host` (`host_file_open(..., true)` + `host_file_write` + `host_file_close`) |
| `0x7F6C` | `ROUTE` | `route METHOD /path` | Matched by `serve_omni_http_via_narrow_host` over `host_tcp_listen` / `host_tcp_accept` / `host_tcp_read` |
| `0x7F6D` | `REQ_PAR` | `req.param("id")` | Pure-Omni extraction of `:id` path parameter from the HTTP request line |
| `0x7F6E` | `RSP_JSON` | `respond(status, body)` | Serializes HTTP/1.1 JSON response and transmits via `host_tcp_write` + `host_tcp_close` |
| `0x7F6F` | `RSP_HTML` | `respond_html(status, html)` | Serializes HTTP/1.1 HTML response (requires `@SAFE` body) and transmits via `host_tcp_write` + `host_tcp_close` |
| `0x7F70` | `AUTH_VER` | `auth_verify(x)` | Pure verification sanitizer gated by `CAP_ID_AUTH` (`bit 8`, `0x0100`) |
