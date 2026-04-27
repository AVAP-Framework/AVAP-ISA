# Architecture Decision Records

| ID | Title | Status | Date |
|---|---|---|---|
| [ADR-001](ADR-001-three-crate-workspace.md) | Three-Crate Rust Workspace Architecture | Accepted | 2026-03-22 |
| [ADR-002](ADR-002-isa-external-plugin.md) | ISA as External Plugin via ISAProvider Trait | Accepted | 2026-03-22 |
| [ADR-003](ADR-003-opcodes-json-single-source-of-truth.md) | opcodes.json as Single Source of Truth | Accepted | 2026-03-22 |
| [ADR-004](ADR-004-dual-bytecode-storage.md) | Dual Bytecode Storage for Backwards Compatibility | Accepted | 2026-03-22 |
| [ADR-005](ADR-005-compiler-in-definition-server.md) | Python→AVBC Compiler in Definition Server | Accepted | 2026-03-22 |
| [ADR-006](ADR-006-marker-based-conector-routing.md) | Marker-Based Routing for Conector Namespace Mutations | Accepted | 2026-03-22 |
| [ADR-007](ADR-007-maturin-build-docker-pattern.md) | maturin build + pip install wheel Pattern for Docker | Accepted | 2026-03-22 |
| [ADR-008](ADR-008-short-circuit-boolean-evaluation.md) | Short-Circuit Boolean Evaluation in the Compiler | Accepted | 2026-03-22 |
| [ADR-009](ADR-009-isinstance-to-is-instance-opcode.md) | isinstance() Compiles to IS_INSTANCE Opcode | Accepted | 2026-03-22 |

## Format

Each ADR follows the format:
- **Context**: What situation led to this decision
- **Decision**: What was decided
- **Alternatives Considered**: What was evaluated and rejected, and why
- **Consequences**: What changes as a result

## Adding a new ADR

1. Create a new file: `ADR-NNN-short-title.md`
2. Add it to this index
3. Open a PR — ADRs are immutable once merged; supersede with a new ADR if needed
