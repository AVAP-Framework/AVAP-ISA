---
date: 2026-03-22
status: Accepted
project: Platon VM Kernel + AVAP ISA v2
**Authors:** Rafael Ruiz (101OBEX, Corp - CTO)
---

# ADR-006: Marker-Based Routing for Conector Namespace Mutations

**Date:** 2026-03-22
**Status:** Accepted
**Authors:** Rafael Ruiz (101OBEX, Corp - CTO)

### Context

AVAP commands write to `self.conector.variables` and `self.conector.results` at runtime. In the Rust VM, `Value::Dict` is a value type (copy semantics). A naive approach of loading `__conector__` into the VM as a `Value::Dict`, mutating it via `SET_ITEM`, and reading it back would lose the mutations because `SET_ITEM` modifies a copy on the stack, not the original in `VMState.globals`.

### Decision

Use a marker-based routing scheme:

1. `LOAD_CONECTOR` pushes the string `"__CONECTOR__"` (a marker)
2. `GET_ATTR "variables"` on `"__CONECTOR__"` pushes `"__CONECTOR_VARS__"` (a marker)
3. `GET_ATTR "results"` on `"__CONECTOR__"` pushes `"__CONECTOR_RESULTS__"` (a marker)
4. `SET_ITEM` on `"__CONECTOR_VARS__"` writes directly to `VMState.conector_vars` (a `HashMap<String, Value>`)
5. `SET_ITEM` on `"__CONECTOR_RESULTS__"` writes directly to `VMState.results`
6. `CALL_METHOD "get"` on `"__CONECTOR_VARS__"` reads from `VMState.conector_vars`

After `execute()`, the Language Server reads `vm.conector_vars` and `vm.results` (exposed via PyO3 getters) and syncs them back to `conector.variables` and `conector.results`.

### Alternatives Considered

**Reference-counted `Value::Dict`**: Change `Value::Dict` to use `Rc<RefCell<...>>` to allow in-place mutation. Rejected — breaks `Send` + `Sync` requirements for the VM; significant architectural change to `platon-core`.

**Re-inject conector after execution**: Re-read `__conector__` from `VMState.globals` after execution. Rejected — `SET_ITEM` on a dict value type modifies the copy on the stack, not the value stored in globals. The copy in globals is stale.

**Add STORE_RESULT opcode**: Custom opcode that writes directly to `VMState.results`. Considered but more invasive — marker routing achieves the same with no new opcodes.

**Keep exec() for conector writes**: Hybrid approach. Rejected — defeats the purpose of eliminating exec().

### Consequences

- `VMState` now has two new fields: `conector_vars: HashMap<String, Value>` and `results: HashMap<String, Value>`
- Marker strings (`"__CONECTOR__"`, `"__CONECTOR_VARS__"`, `"__CONECTOR_RESULTS__"`) must not be used as actual variable names
- `h_call_method` needs to handle marker types, requiring access to `&mut VMState` — the existing `call_method()` pure function was refactored into `h_call_method` directly
- The scheme is transparent to the compiler — it generates standard `LOAD_CONECTOR` / `GET_ATTR` / `SET_ITEM` sequences

---
