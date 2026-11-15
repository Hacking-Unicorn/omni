# Host-runtime port decisions (Ember Runtime, `ember/runtime-port`)

| ID | Decision | Source |
|---|---|---|
| HR-1 | Arena band `0x100000-0x2FFFFF` (2 MiB) is reserved for everything the host runtime allocates: VM interpreter memory, decoded instruction table, call frames, KV/DB slabs, request/HTTP buffers, bump heap (`pure/vm_*`, `pure/host_*`). Supersedes the earlier `0x100000-0x17FFFF`. M2 stays below `0x80000`; M3 owns `0x80000-0xFFFFF`. | Boss Lady, 2026-11-14 (widened same day); HANDOFF.md §6.6 |
| HR-2 | `DB_QRY` (`0x7F65`) miss returns `-1` (`OMNI_DB_QRY_MISS`), never `key*10`. `DB_GET` (`0x7F64`) miss also returns `-1` (`OMNI_DB_GET_MISS`), never `key*10`. | Boss Lady, 2026-11-14; HANDOFF.md §6.7 |
| HR-3 | KV/DB in core Omni uses an arena-backed sorted array with iterative binary search (no hash maps). | Boss Lady, 2026-11-14; HANDOFF.md §6.7 |
| HR-4 | `pure/host_runtime.omni` `omni_pure_host_kv_apply` (op 3 = query, returns `key*10`) and its embedded stdlib literal are left untouched: they are in `PURE_OMNI_CORE_SUITE_MODULES`, so editing them moves `0x78462AE4C81DF408`. Known guest/host difference: the guest model returns `key*10` for op 3 while the Rust VM returns `-1` on a miss. Fixed at the planned re-freeze, not before. | Boss Lady, 2026-11-14; hash policy HANDOFF.md §3 |
