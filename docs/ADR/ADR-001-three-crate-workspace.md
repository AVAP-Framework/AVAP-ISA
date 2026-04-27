---
date: 2026-03-22
status: Accepted
project: Platon VM Kernel + AVAP ISA v2
---

# ADR-001: Three-Crate Rust Workspace Architecture

**Date:** 2026-03-22
**Status:** Accepted
**Authors:** Rafael Ruiz (101OBEX, Corp - CTO)

### Context

The project requires a Rust VM kernel exposed to Python, plus a separate ISA crate. Both need to share common types. The initial approach was a single crate, but this caused linker errors on macOS when both crates tried to link PyO3.

### Decision

Split into three crates:

```
platon/              (Cargo workspace root)
├── platon-core/     rlib — no PyO3 (Value, VMState, ISAProvider)
├── ./ (platon)      cdylib — PyO3 bindings
avap-isa/            cdylib — separate repo, depends on platon-core
```

### Alternatives Considered

**Single crate**: All code in one repo. Rejected — `avap-isa` as a separate open-source repo requires independence from the PyO3 bindings.

**Two crates (platon + avap-isa)**: `avap-isa` depends directly on `platon`. Rejected — causes double-linking of Python symbols on macOS, resulting in a dyld linker error at runtime.

**Dynamic loading**: ISA loaded as a `.so` at runtime. Rejected — too complex, no type safety across the boundary.

### Consequences

- `platon-core` is a pure Rust library with no Python dependency — any language runtime can depend on it
- The `ISAProvider` trait is the clean contract between kernel and ISA
- `avap-isa` can be open-sourced independently without any Python build toolchain concern
- Adding a new ISA for another language requires only a new crate depending on `platon-core`

---
