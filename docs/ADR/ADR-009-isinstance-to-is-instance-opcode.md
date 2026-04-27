---
date: 2026-03-22
status: Accepted
project: Platon VM Kernel + AVAP ISA v2
**Authors:** Rafael Ruiz (101OBEX, Corp - CTO)
---

# ADR-009: isinstance() Compiles to IS_INSTANCE Opcode

**Date:** 2026-03-22
**Status:** Accepted
**Authors:** Rafael Ruiz (101OBEX, Corp - CTO)

### Context

`isinstance(x, str)` was being compiled as a call to the Python builtin `isinstance` via `LOAD_BUILTIN("isinstance") + LOAD(x) + LOAD_BUILTIN("str") + CALL_FUNC(2)`. The `CALL_FUNC` handler in `avap-isa` did not know how to execute `isinstance` — it treated `"isinstance"` as a function name and tried to call it as a method.

### Decision

Detect `isinstance()` calls in the compiler and emit `IS_INSTANCE` directly:

```python
# isinstance(x, str)    → LOAD(x), LOAD_BUILTIN("str"), IS_INSTANCE
# isinstance(x, (int, float)) → LOAD(x), LOAD_BUILTIN("int"), LOAD_BUILTIN("float"), BUILD_TUPLE(2), IS_INSTANCE
```

The `IS_INSTANCE` handler in `avap-isa` supports both single type names and `Value::List` (tuple of types). The `type_matches()` helper handles Python type name to `Value` type mappings, including the `float`/`int` coercion (`float` also matches `Value::Int`).

### Alternatives Considered

**Implement isinstance in CALL_FUNC**: Handle `isinstance` as a special case in the generic function call handler. Rejected — conflates type-checking semantics with function call mechanics; requires the ISA to know about Python's type system.

**Evaluate via Python callback (CALL_EXT)**: Route isinstance to a Python function. Rejected — defeats the purpose of native execution.

### Consequences

- `isinstance` cannot be shadowed by a user variable named `isinstance` in AVAP commands
- The `IS_INSTANCE` opcode is specific to AVAP's type system — a different language ISA would implement type-checking differently
- Type coercion rule: `float` matches both `Value::Float` and `Value::Int` (Python's `isinstance(42, float)` returns `False`, but AVAP's is more permissive for practical reasons)
