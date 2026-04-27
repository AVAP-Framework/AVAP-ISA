# Changelog

All notable changes to `avap-isa` are documented here.

This project follows [Semantic Versioning](https://semver.org/) and [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [Unreleased]

### Planned
- AVBC disassembler (`avap-isa disasm <file>`)
- Benchmarking suite for instruction throughput
- WASM target support

---

## [0.1.0] — 2026-03-22

### Added

**Core ISA — 56 opcodes across 12 categories:**
- Stack & Variables: `NOP`, `PUSH`, `POP`, `DUP`, `LOAD_NONE`, `LOAD`, `STORE`
- Arithmetic: `ADD`, `SUB`, `MUL`, `DIV`, `MOD`, `NEG`, `NOT`
- Comparison: `EQ`, `NEQ`, `LT`, `GT`, `LTE`, `GTE`, `IN`, `NOT_IN`, `IS`, `IS_NOT`
- Boolean: `BOOL_AND`, `BOOL_OR` (short-circuit evaluation)
- Control Flow: `JMP`, `JMP_IF`, `JMP_IF_NOT`, `JMP_IF_POP`, `JMP_IF_NOT_POP`, `PUSH_TRY`, `POP_TRY`, `RAISE`, `RETURN`
- Iteration: `GET_ITER`, `FOR_ITER`
- Object Access: `GET_ATTR`, `SET_ATTR`, `GET_ITEM`, `SET_ITEM`, `DELETE_ITEM`
- Collections: `BUILD_LIST`, `BUILD_DICT`, `BUILD_TUPLE`
- Calls: `CALL_EXT`, `CALL_FUNC`, `CALL_METHOD`
- Runtime: `LOAD_CONECTOR`, `LOAD_TASK`, `LOAD_BUILTIN`, `IMPORT_MOD`
- Type System: `IS_INSTANCE`, `IS_NONE`, `TYPE_OF`
- System: `HALT`

**String method support in `CALL_METHOD`:**
`strip`, `upper`, `lower`, `isdigit`, `startswith`, `endswith`, `replace`, `split`, `encode`, `decode`

**Dict method support in `CALL_METHOD`:**
`get`, `keys`, `values`, `items`, `update`

**List method support in `CALL_METHOD`:**
`append`

**Runtime connector routing:**
- `LOAD_CONECTOR` / `GET_ATTR "variables"` → direct VMState write via `SET_ITEM`
- `LOAD_CONECTOR` / `GET_ATTR "results"` → direct VMState results write
- Eliminates copy-on-write semantics for mutable runtime state

**`IS_INSTANCE` with tuple of types:**
`isinstance(x, (int, float))` compiles and executes correctly

**`opcodes.json` as single source of truth:**
- Opcode values never hardcoded in Rust source
- `build.rs` generates `op::` constants at compile time from JSON
- Same JSON consumed by `compiler.py` (Python → AVBC compiler)

**PyO3 Python bindings:**
- `AvapISA` class exposed to Python
- `_get_arc_ptr()` returns fat pointer `(u64, u64)` for `ISAProvider` trait object transfer to Platon VM
- `vm.register_isa(AvapISA())` protocol

---

## Format

Each entry uses: `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`.
