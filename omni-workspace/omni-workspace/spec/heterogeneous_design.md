# Omni Heterogeneous Execution Design (`spec/heterogeneous_design.md`)

**Status:** Design Specification Only — Deferred Implementation (Section 7 of `OMNI_IMPROVEMENT_INSTRUCTIONS.md`)  
**Core Principle:** Preserve Omni's static capability, taint, linear-memory, and budget guarantees across CPU, GPU, and NPU execution units without introducing dynamic kernel compilation or raw pointer sharing.

---

## 1. Explicit Non-Goals (First Version)

1. **No dynamic runtime kernel generation (no JIT string compilation on device).** All device kernels must be statically verified from `.omni` surface functions into `Vec<u16>` token streams at compile time.
2. **No shared mutable pointers across host CPU and device memory.** All cross-device data movement uses explicit linear arena ownership transfer.
3. **No host/network/filesystem syscalls (`0x7F64..=0x7F70`) on GPU or NPU devices.**

---

## 2. Surface Placement Annotations & Supported Subset Fences

Placement is declared at the function level using existing attribute syntax:

```omni
#[strict]
#[device(npu)]
#[cap(AI)]
#[step_budget(100000)]
#[mem_budget(65536)]
fn ternary_attention_step(weights: bytes, act: bytes) -> trit {
    let dot: i64 = pack5_tdot(weights, act);
    let gate: trit = egate(dot);
    return gate;
}
```

### Supported Opcode Subset Fences per Device Target

| Opcode Family | `#[device(cpu)]` (Default) | `#[device(gpu)]` | `#[device(npu)]` |
|---|---|---|---|
| Integer ALU (`0x7F10..=0x7F1B`) | Supported | Supported | Supported |
| Balanced Ternary ALU (`0x7F20..=0x7F25`, `0x7F45`) | Supported | Supported | Supported (Hardware Native) |
| `@PACK5` Tensor Ops (`PACK5`, `UNPACK5`, `TDOT`, `EGATE`, `TLOGIC`) | Supported | Supported | Supported (Primary Fast Path) |
| Linear Arenas (`0x7F01..=0x7F05`) | Supported | Supported (Device Arena) | Supported (On-Chip SRAM Arena) |
| String / Sanitizer Ops (`0x7F50..=0x7F5D`) | Supported | **Rejected at Compile Time** | **Rejected at Compile Time** |
| Host / Network / DB Ops (`0x7F64..=0x7F70`) | VM + `OmniNarrowHost` Only | **Rejected at Compile Time** | **Rejected at Compile Time** |

---

## 3. Linear Memory Ownership & Cross-Device Transfer Model

To prevent data races and use-after-free between CPU and accelerator memory spaces:

1. **Disjoint Address Spaces:** An `arena(N)` handle carries a static residency tag $\in \{\text{CPU}, \text{GPU}, \text{NPU}\}$.
2. **Linear Ownership Transfer:** Moving a buffer to a device consumes the host arena handle (`MOVE` `0x7F04`) until the device kernel completes and returns ownership. Attempting to read or drop a host arena while borrowed by a device kernel triggers `error[BORROW_CONFLICT]` with 1-based `(line, col)`.
3. **Zero Post-Kernel Leak:** Every device kernel must release its local scratch arenas before `RET`, preserving Compositional Invariant Lemma 2 (`allocated_bytes == 0`).

---

## 4. Capability, Taint, and Budget Enforcement Across Devices

- **Taint Non-Interference:** `@UNTRUSTED` data cannot be transferred into a `#[device(gpu)]` or `#[device(npu)]` kernel unless sanitized on the CPU first, or the device kernel parameter is explicitly marked `@untrusted` and never reaches a decision sink (`egate`) or `RET`.
- **Capability Enforcement:** Invoking a `#[device(npu)]` kernel requires `#[cap(AI)]` (`CAP_ID_AI = 4`).
- **Unified Step & Memory Budgets:** Device launches deduct a deterministic cost proportional to tensor element count from the caller's `#[step_budget(N)]` and `#[mem_budget(N)]` BEFORE dispatch, guaranteeing bounded execution time even on synchronous hardware queues.
