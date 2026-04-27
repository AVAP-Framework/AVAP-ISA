# avap-isa

**AVAP Instruction Set Architecture for the Platon kernel**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![PyO3](https://img.shields.io/badge/PyO3-0.20-blue.svg)](https://pyo3.rs)
[![Python](https://img.shields.io/badge/python-3.11%2B-blue.svg)](https://www.python.org)

`avap-isa` is the reference implementation of the AVAP language instruction set for the [Platon](https://github.com/avapcloud/platon) language-agnostic virtual machine kernel. It provides 56 opcodes that cover all Python constructs used in AVAP command definitions — compiled to AVBC bytecode and executed natively by the Rust VM with no `exec()` at runtime.

---

## Architecture

```
opcodes.json          ← Single source of truth for all opcode definitions
    │
    ├── build.rs      ← Generates Rust op:: constants at compile time
    │       └──▶ OUT_DIR/opcodes.rs
    │
    └── src/lib.rs    ← 56 instruction handlers + ISAProvider impl
            └──▶ avap_isa.so  (Python extension via PyO3)
```

`avap-isa` implements the `ISAProvider` trait from `platon-core`, making it a first-class, swappable ISA for any language targeting the Platon kernel. Opcode values are never hardcoded in source — they are always derived from `opcodes.json`.

---

## Requirements

| Dependency | Version |
|---|---|
| Rust | 1.75+ |
| Python | 3.11+ |
| maturin | 1.5+ |
| platon-core | 0.3+ |

---

## Installation

### From source (development)

```bash
git clone https://github.com/avapcloud/avap-isa
cd avap-isa
maturin develop --release
```

### In a Docker container

```dockerfile
# Mount alongside platon and build at startup
volumes:
  - ../PLATON:/build/PLATON
  - ../avap-isa:/build/avap-isa
```

---

## Quick Start

```python
from platon import VM
from avap_isa import AvapISA

vm = VM(timeout=5.0)
vm.register_isa(AvapISA())
vm.load(avbc_bytecode)
result = vm.execute()
```

### ISA introspection

```python
isa = AvapISA()
print(isa)            # <AvapISA v0.1.0 (56 opcodes)>
print(isa.name)       # "AVAP-ISA"
print(isa.version)    # (0, 1, 0)
```

---

## Instruction Set Reference

All opcodes are defined in [`opcodes.json`](./opcodes.json). Each instruction argument is a `u32` little-endian value immediately following the opcode byte (4 bytes per argument).

### Stack & Variables

| Mnemonic | Opcode | Args | Description |
|---|---|---|---|
| `NOP` | `0x00` | 0 | No operation |
| `PUSH` | `0x01` | 1 | Push constant[const_idx] onto stack |
| `POP` | `0x02` | 0 | Discard top of stack |
| `DUP` | `0x03` | 0 | Duplicate top of stack |
| `LOAD_NONE` | `0x04` | 0 | Push Null onto stack |
| `LOAD` | `0x40` | 1 | Push globals[name_idx] onto stack |
| `STORE` | `0x41` | 1 | Pop and store into globals[name_idx] |

### Arithmetic

| Mnemonic | Opcode | Args | Description |
|---|---|---|---|
| `ADD` | `0x10` | 0 | pop b, pop a, push a+b |
| `SUB` | `0x11` | 0 | pop b, pop a, push a-b |
| `MUL` | `0x12` | 0 | pop b, pop a, push a*b |
| `DIV` | `0x13` | 0 | pop b, pop a, push a/b |
| `MOD` | `0x14` | 0 | pop b, pop a, push a%b |
| `NEG` | `0x15` | 0 | pop a, push -a |
| `NOT` | `0x16` | 0 | pop a, push !a |

### Comparison

| Mnemonic | Opcode | Args | Description |
|---|---|---|---|
| `EQ` | `0x20` | 0 | pop b, pop a, push a==b |
| `LT` | `0x21` | 0 | pop b, pop a, push a<b |
| `GT` | `0x22` | 0 | pop b, pop a, push a>b |
| `LTE` | `0x23` | 0 | pop b, pop a, push a<=b |
| `GTE` | `0x24` | 0 | pop b, pop a, push a>=b |
| `NEQ` | `0x25` | 0 | pop b, pop a, push a!=b |
| `IN` | `0x26` | 0 | pop container, pop item, push item in container |
| `NOT_IN` | `0x27` | 0 | pop container, pop item, push item not in container |
| `IS` | `0x28` | 0 | pop b, pop a, push a is b |
| `IS_NOT` | `0x29` | 0 | pop b, pop a, push a is not b |

### Boolean

| Mnemonic | Opcode | Args | Description |
|---|---|---|---|
| `BOOL_AND` | `0x2A` | 0 | Short-circuit AND — leaves result on stack |
| `BOOL_OR` | `0x2B` | 0 | Short-circuit OR — leaves result on stack |

### Control Flow

| Mnemonic | Opcode | Args | Description |
|---|---|---|---|
| `JMP` | `0x30` | 1 | Unconditional jump to target_ip |
| `JMP_IF` | `0x31` | 1 | Jump if TOS is truthy (no pop) |
| `JMP_IF_NOT` | `0x32` | 1 | Jump if TOS is falsy (no pop) |
| `JMP_IF_POP` | `0x33` | 1 | Jump if TOS is truthy and pop |
| `JMP_IF_NOT_POP` | `0x34` | 1 | Jump if TOS is falsy and pop |
| `PUSH_TRY` | `0x35` | 1 | Push exception handler at handler_ip |
| `POP_TRY` | `0x36` | 0 | Pop exception handler (normal exit from try) |
| `RAISE` | `0x37` | 0 | Raise TOS as exception |
| `RETURN` | `0x38` | 0 | Return TOS from current function |

### Iteration

| Mnemonic | Opcode | Args | Description |
|---|---|---|---|
| `GET_ITER` | `0x3A` | 0 | Pop iterable, push iterator |
| `FOR_ITER` | `0x3B` | 1 | Advance iterator; push next value or jump to exit_ip |

### Object Access

| Mnemonic | Opcode | Args | Description |
|---|---|---|---|
| `GET_ATTR` | `0x50` | 1 | Pop obj, push obj.attr_name |
| `SET_ATTR` | `0x51` | 1 | Pop value, set obj.attr_name = value |
| `GET_ITEM` | `0x52` | 0 | Pop key, pop obj, push obj[key] |
| `SET_ITEM` | `0x53` | 0 | Pop value, pop key, set obj[key] = value |
| `DELETE_ITEM` | `0x54` | 0 | Pop key, pop obj, del obj[key] |

### Collections

| Mnemonic | Opcode | Args | Description |
|---|---|---|---|
| `BUILD_LIST` | `0x55` | 1 | Pop n items, push [item0..itemN] |
| `BUILD_DICT` | `0x56` | 1 | Pop 2n items (k,v pairs), push {k:v...} |
| `BUILD_TUPLE` | `0x57` | 1 | Pop n items, push (item0..itemN) |

### Calls

| Mnemonic | Opcode | Args | Description |
|---|---|---|---|
| `CALL_EXT` | `0x60` | 1 | Call registered native function by func_id |
| `CALL_FUNC` | `0x61` | 1 | Pop n_args + callable, call callable(*args) |
| `CALL_METHOD` | `0x62` | 2 | Pop n_args + obj, call obj.method(*args) |

### Runtime

| Mnemonic | Opcode | Args | Description |
|---|---|---|---|
| `LOAD_CONECTOR` | `0x70` | 0 | Push runtime connector object onto stack |
| `LOAD_TASK` | `0x71` | 0 | Push current task definition onto stack |
| `LOAD_BUILTIN` | `0x72` | 1 | Push builtin function by name_idx |
| `IMPORT_MOD` | `0x90` | 1 | Push module proxy by name_idx |

### Type System

| Mnemonic | Opcode | Args | Description |
|---|---|---|---|
| `IS_INSTANCE` | `0x80` | 0 | Pop type (or tuple of types), pop obj, push isinstance result |
| `IS_NONE` | `0x81` | 0 | Pop obj, push obj is None |
| `TYPE_OF` | `0x82` | 0 | Pop obj, push type name string |

### System

| Mnemonic | Opcode | Args | Description |
|---|---|---|---|
| `HALT` | `0xFF` | 0 | Stop execution |

---

## AVBC Bytecode Format

```
Offset  Size  Field
──────  ────  ─────────────────────────────────────────
0       4     Magic: b'AVBC'
4       2     ISA version major.minor (u16 LE)
6       2     Flags (u16 LE, reserved = 0)
8       4     Code size in bytes (u32 LE)
12      4     Constant pool entry count (u32 LE)
16      4     Entry point IP (u32 LE, usually 0)
20      108   Reserved (zero-padded to 128 bytes)
128     var   Constant pool
var     var   Instruction stream
```

### Constant Pool Tags

| Tag | Type | Payload |
|---|---|---|
| `0x01` | Null | (none) |
| `0x02` | Int | 8 bytes, i64 LE |
| `0x03` | Float | 8 bytes, f64 LE |
| `0x04` | String | 4-byte length (u32 LE) + UTF-8 bytes |
| `0x05` | Bool true | (none) |
| `0x06` | Bool false | (none) |

---

## Extending the ISA

`avap-isa` is the **reference implementation** for Platon ISAs. To create an ISA for another language:

1. Create your own `opcodes.json`
2. Depend on `platon-core` for `ISAProvider`, `VMState`, `InstructionSet`
3. Implement instruction handlers as `InstructionFn` functions
4. Register them in `ISAProvider::instruction_set()`

```rust
use platon_core::{ISAProvider, InstructionSet, VMState, ISAError};

pub struct MyISA { isa: InstructionSet }

impl ISAProvider for MyISA {
    fn name(&self)            -> &str         { "my-isa" }
    fn version(&self)         -> (u8, u8, u8) { (1, 0, 0) }
    fn instruction_set(&self) -> &InstructionSet { &self.isa }
}
```

---

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md).

## Security

See [SECURITY.md](./SECURITY.md).

## Changelog

See [CHANGELOG.md](./CHANGELOG.md).

## License

MIT — see [LICENSE](./LICENSE).

Copyright © 2026 101OBEX, Corp
