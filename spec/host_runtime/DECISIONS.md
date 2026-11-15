# Host-runtime port decisions (Ember Runtime, `ember/runtime-port`)

| ID | Decision | Source |
|---|---|---|
| HR-1 | Arena band `0x100000-0x17FFFF` is reserved for host-runtime state (`pure/vm_*`, `pure/host_*`). M2 stays below `0x80000`; M3 owns `0x80000-0xFFFFF`. | Boss Lady, 2026-11-14; HANDOFF.md §6.6 |
| HR-2 | `DB_QRY` (`0x7F65`) miss returns `-1` (`OMNI_DB_QRY_MISS`), never `key*10`. `DB_GET` (`0x7F64`) miss is unchanged (`key*10`) pending a separate decision. | Boss Lady, 2026-11-14; HANDOFF.md §6.7 |
| HR-3 | KV/DB in core Omni uses an arena-backed sorted array with iterative binary search (no hash maps). | Boss Lady, 2026-11-14; HANDOFF.md §6.7 |
| HR-4 | `pure/host_runtime.omni` `omni_pure_host_kv_apply` (op 3 = query, returns `key*10`) and its embedded stdlib literal are left untouched: they are in `PURE_OMNI_CORE_SUITE_MODULES`, so editing them moves `0x78462AE4C81DF408`. Revisit at the planned re-freeze. | Hash policy, HANDOFF.md §3 |
