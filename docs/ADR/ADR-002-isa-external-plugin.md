---
date: 2026-03-22
status: Accepted
project: Platon VM Kernel + AVAP ISA v2
---

# ADR-002: ISA as External Plugin via ISAProvider Trait

**Date:** 2026-03-22
**Status:** Accepted
**Authors:** Rafael Ruiz (101OBEX, Corp - CTO)

### Context

The original design had the ISA baked into the Platon kernel. The goal of language-agnosticism requires that different languages can provide different instruction sets without modifying the kernel.

### Decision

Define `ISAProvider` as a Rust trait in `platon-core`. The VM kernel dispatches all instruction execution through this trait. ISAs are registered at runtime via `vm.register_isa()`.

```rust
pub trait ISAProvider: Send + Sync {
    fn name(&self)            -> &str;
    fn version(&self)         -> (u8, u8, u8);
    fn instruction_set(&self) -> &InstructionSet;
}
```

Fat pointer transfer across the Python/Rust boundary uses `Arc<dyn ISAProvider>` encoded as `(u64, u64)` via `std::mem::transmute`.

### Alternatives Considered

**Enum-based dispatch**: All ISAs compiled into the kernel as enum variants. Rejected — requires recompiling the kernel for each new language.

**Function pointer table**: ISA registered as a raw C-style vtable. Rejected — not type-safe, complex lifetime management.

**gRPC/IPC**: ISA runs as a separate process. Rejected — too much overhead for hot execution path.

### Consequences

- The fat pointer transmute requires `unsafe` but is well-contained and documented
- Each ISA can be distributed as an independent Python wheel
- The kernel never needs to be recompiled to add new language support
- The `(u64, u64)` fat pointer encoding is specific to the current Rust ABI and must be revisited if upgraded to PyO3 0.21+

---
