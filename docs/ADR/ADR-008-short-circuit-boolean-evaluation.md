---
date: 2026-03-22
status: Accepted
project: Platon VM Kernel + AVAP ISA v2
**Authors:** Rafael Ruiz (101OBEX, Corp - CTO)
---

# ADR-008: Short-Circuit Boolean Evaluation in the Compiler

**Date:** 2026-03-22
**Status:** Accepted
**Authors:** Rafael Ruiz (101OBEX, Corp - CTO)

### Context

Python boolean operators (`and`, `or`) have short-circuit semantics: `a and b` does not evaluate `b` if `a` is falsy. The original compiler emitted `BOOL_AND` / `BOOL_OR` opcodes that evaluate both sides before combining, which caused errors like `None.isdigit()` when the left side of `isinstance(x, str) and x.isdigit()` was `False`.

### Decision

Compile `and` using `JMP_IF_NOT` (no pop, leaves TOS as result on false path):

```
# a and b
eval(a)
JMP_IF_NOT → end    # if falsy, jump to end leaving a on stack as result
POP                 # a was truthy, discard it
eval(b)             # b is now the result
end:
```

Compile `or` using `JMP_IF` (symmetric):

```
# a or b
eval(a)
JMP_IF → end        # if truthy, jump to end leaving a on stack as result
POP
eval(b)
end:
```

### Alternatives Considered

**`BOOL_AND` opcode**: Simple but eager — evaluates both sides. Rejected for any expression where the right side may fail when the left is false.

**`JMP_IF_NOT_POP` (pop on false)**: Leaves nothing on the stack on the false path, requiring a `LOAD_NONE` + `NOT` to push `False`. This was the first attempt — it pushed `True` instead of `False` due to `LOAD_NONE + NOT`. Rejected.

### Consequences

- `isinstance(x, str) and x.isdigit()` correctly short-circuits: `isdigit()` is never called when `x` is not a string
- The `BOOL_AND` and `BOOL_OR` opcodes remain in the ISA for use cases where both sides are side-effect-free
- Any existing bytecode compiled with the old `BOOL_AND` approach for complex boolean chains is now recompiled at Definition Server startup

---
