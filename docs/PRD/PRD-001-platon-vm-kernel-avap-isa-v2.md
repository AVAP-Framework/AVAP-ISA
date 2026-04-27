# PRD — Platon VM Kernel + AVAP ISA v2

**Product Requirements Document**
**Date:** 2026-03-22
**Status:** Implemented
**Authors:** Rafael Ruiz (101OBEX, Corp - CTO)

---

## 1. Overview

This document describes the requirements for the native VM execution kernel for the AVAP Language Server. The project replaces the Python `exec()` execution model with a Rust-based virtual machine that compiles AVAP command definitions to bytecode and executes them natively.

---

## 2. Problem Statement

### Current state (before this project)

The AVAP Language Server executes commands by:
1. Receiving Python source code from the Definition Server (packaged with HMAC signature)
2. Unpacking and verifying the signature
3. Calling Python `exec()` on the source code at runtime

This approach has several problems:

- **Performance**: `exec()` re-parses and interprets Python source on every call. No ahead-of-time optimisation.
- **Security surface**: `exec()` has broad access to Python builtins. Sandbox is shallow.
- **Language lock-in**: The execution model is tied to Python. Supporting other languages requires running separate interpreters.
- **No language independence**: The "AVAP language" is effectively just Python with a specific runtime context. There is no formal language boundary.
- **GIL contention**: Multiple concurrent `exec()` calls compete for the Python GIL.

### Desired state

- Commands are compiled to a defined bytecode format (AVBC) by the Definition Server
- Bytecode is executed by a Rust VM (Platon) — no Python at runtime
- The VM is language-agnostic: any language that compiles to AVBC can run on it
- The ISA is external and swappable: different languages provide different instruction sets
- Full backwards compatibility during transition: both execution paths coexist

---

## 3. Goals

| ID | Goal | Priority |
|---|---|---|
| G1 | Eliminate `exec()` from the hot execution path | P0 |
| G2 | Compile AVAP commands to AVBC bytecode at deploy time, not request time | P0 |
| G3 | Execute AVBC bytecode in Rust with no Python at runtime | P0 |
| G4 | Make the VM kernel language-agnostic (ISA is external) | P1 |
| G5 | Maintain full backwards compatibility via `exec()` fallback | P1 |
| G6 | All 27 existing AVAP commands must compile and execute correctly | P0 |
| G7 | The ISA opcode definitions must be a single source of truth | P1 |

---

## 4. Non-Goals

- Replacing the existing Python/Tornado HTTP layer
- Supporting WASM or other non-Python hosts in this version
- JIT compilation
- Multi-language support beyond AVAP in this release (architecture must support it)

---

## 5. Requirements

### 5.1 Platon VM Kernel (`platon` + `platon-core`)

**REQ-001** The VM kernel must be implemented in Rust and exposed to Python via PyO3.

**REQ-002** The kernel must be split into two crates:
- `platon-core`: pure Rust `rlib` with no PyO3 dependency (types, traits, state)
- `platon`: PyO3 `cdylib` with Python bindings

**REQ-003** `platon-core` must define the `ISAProvider` trait that any ISA implementation must satisfy.

**REQ-004** The VM must enforce execution limits: configurable timeout (seconds) and maximum instruction count.

**REQ-005** ISAs must be registered at runtime via `vm.register_isa(isa_object)` before calling `vm.execute()`.

**REQ-006** The VM must expose `vm.conector_vars` and `vm.results` after execution to read values written by the ISA to runtime namespaces.

**REQ-007** The AVBC bytecode format must have a 128-byte header containing: magic (`AVBC`), ISA version, flags, code size, constant pool count, entry point.

### 5.2 AVAP ISA (`avap-isa`)

**REQ-010** The ISA must implement 56 opcodes covering all Python constructs used in the 27 AVAP commands.

**REQ-011** Opcode values must never be hardcoded in Rust source. They must be generated from `opcodes.json` at compile time via `build.rs`.

**REQ-012** `opcodes.json` must be the single source of truth consumed by both the Rust ISA and the Python compiler.

**REQ-013** The ISA must handle the AVAP runtime context: `LOAD_CONECTOR`, `LOAD_TASK` opcodes must route to `VMState.conector_vars` and `VMState.results` directly (not to copies).

**REQ-014** `CALL_EXT` must support calling registered Python functions from Rust via the `NativeRegistry`.

**REQ-015** `IS_INSTANCE` must support both single types and tuples of types.

**REQ-016** String methods required: `strip`, `upper`, `lower`, `isdigit`, `startswith`, `endswith`, `replace`, `split`, `encode`, `decode`.

**REQ-017** Dict methods required: `get`, `keys`, `values`, `items`, `update`.

### 5.3 AVBC Compiler (`compiler.py`)

**REQ-020** The compiler must accept any ISA defined by an `opcodes.json` file, passed via `--isa` flag.

**REQ-021** The compiler must support all Python constructs present in the 27 AVAP commands: assignments, conditionals, loops, try/except, function definitions (inlined), imports, f-strings, list comprehensions.

**REQ-022** Boolean operators (`and`, `or`) must use short-circuit evaluation.

**REQ-023** `isinstance()` calls must compile to the `IS_INSTANCE` opcode, not to a Python function call.

**REQ-024** The compiler must exit with code 1 and a JSON error on unsupported constructs (not hard crash).

### 5.4 Definition Server

**REQ-030** The `avap_bytecode` table must store both legacy bytecode (`bytecode` column) and AVBC bytecode (`avbc_bytecode` column) simultaneously.

**REQ-031** At startup, the Definition Server must attempt to compile all commands to AVBC. Compilation failures must not prevent the server from starting (fallback to legacy path).

**REQ-032** The gRPC `CommandResponse` message must include `avbc_code` and `avbc_version` fields.

### 5.5 Language Server

**REQ-040** When `avbc_code` is available in the catalog, the Language Server must execute commands via the Rust VM (direct AVBC path).

**REQ-041** When `avbc_code` is not available, the Language Server must fall back to the legacy `CALL_EXT` path (exec() via native bridge).

**REQ-042** After AVBC execution, `conector.variables` and `conector.results` must be synchronised back to the execution context.

---

## 6. Architecture

```
Definition Server (Node.js)
  ├── compiler.py          ← Python→AVBC compiler (reads opcodes.json)
  ├── opcodes.json         ← ISA definition (shared with avap-isa)
  └── avap_bytecode table  ← stores both legacy + AVBC bytecode

            │ gRPC (avbc_code field)
            ▼

Language Server (Python/Tornado)
  ├── AVAPExecutor
  │   ├── avbc_cache       ← {cmd_name → AVBC bytes}
  │   └── _execute_command()
  │       ├── AVBC path    ← vm.load() → vm.execute() [Rust, no exec()]
  │       └── Legacy path  ← CALL_EXT → exec() [Python fallback]
  │
  └── Platon VM (Rust/PyO3)
      ├── platon-core      ← ISAProvider, VMState, Value
      └── avap-isa         ← 56 opcodes (from opcodes.json)
```

---

## 7. Success Metrics

| Metric | Target |
|---|---|
| Commands compiled to AVBC | 27/27 |
| AVBC test pass rate | 100% |
| Legacy fallback available | Yes |
| `exec()` calls on AVBC path | 0 |
| ISA opcode hardcoding | 0 occurrences |

---

## 8. Rollout Plan

**Phase 1 (Complete):** Platon kernel + AVAP ISA + compiler + dual-store in DB + dual-field in gRPC

**Phase 2 (In Progress):** Language Server uses AVBC path for all 27 commands, verified against legacy output

**Phase 3 (Future):** Remove legacy `exec()` path once AVBC path is validated in production

---

## 9. Risks

| Risk | Mitigation |
|---|---|
| AVBC compiler generates incorrect bytecode | Extensive test suite; legacy fallback always available |
| VMState mutation semantics differ from Python | Marker-based routing for conector namespaces |
| Docker cross-compilation issues (macOS .venv) | `maturin build` + `pip install wheel` pattern in containers |
| ISA version mismatch between compiler and runtime | Shared `opcodes.json` enforces consistency |
