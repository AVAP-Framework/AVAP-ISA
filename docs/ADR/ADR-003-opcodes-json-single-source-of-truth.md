---
date: 2026-03-22
status: Accepted
project: Platon VM Kernel + AVAP ISA v2
---

# ADR-003: opcodes.json as Single Source of Truth

**Date:** 2026-03-22
**Status:** Accepted
**Authors:** Rafael Ruiz (101OBEX, Corp - CTO)

### Context

Opcode values were originally hardcoded as Rust constants in `avap-isa/src/lib.rs` and separately as Python constants in the compiler. This created two places that could diverge.

### Decision

Define all opcode values, argument counts, and descriptions in a single `opcodes.json` file in the `avap-isa` repo root. Both consumers read from this file:

- **Rust**: `build.rs` reads `opcodes.json` at compile time and generates `OUT_DIR/opcodes.rs` with `pub const` definitions. `lib.rs` includes this via `include!(concat!(env!("OUT_DIR"), "/opcodes.rs"))`.
- **Python compiler**: `ISA` class reads `opcodes.json` at runtime via `--isa` flag or `AVAP_ISA_PATH` env var.

### Alternatives Considered

**Protobuf/IDL**: Define ISA in a schema language. Rejected — unnecessary complexity for a simple integer mapping.

**Keep duplicated constants**: Simpler short-term. Rejected — guaranteed to diverge as the ISA evolves.

**Generate from Rust**: Use `serde` to export constants from Rust to JSON. Rejected — creates a circular dependency (JSON is the input, not the output).

### Consequences

- Any ISA change requires editing one file only
- Adding a new language compiler requires only pointing it at `opcodes.json`
- `build.rs` adds `serde_json` as a build-only dependency (not present in the distributed wheel)
- The opcode JSON schema is implicitly defined by `build.rs` — should be formally specified in a future version

---
