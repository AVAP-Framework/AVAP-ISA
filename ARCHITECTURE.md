# Architecture

This document describes the internal design of `avap-isa` and how it integrates with the Platon kernel.

---

## Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        avap-language-server                      │
│  (Python/Tornado)                                               │
│                                                                  │
│  AVAPExecutor                                                    │
│    ├── vm = PlatonVM()                                           │
│    ├── vm.register_isa(AvapISA())   ◄── avap-isa               │
│    ├── vm.load(avbc_bytecode)                                    │
│    └── vm.execute()                                              │
└──────────────┬──────────────────────────────────────────────────┘
               │ PyO3
┌──────────────▼──────────────────────────────────────────────────┐
│                           platon                                 │
│  (Rust cdylib + PyO3)                                           │
│                                                                  │
│  VM::execute()                                                   │
│    └── for each opcode:                                          │
│          isa.instruction_set().get(opcode)?.handler(state, ...)  │
└──────────────┬──────────────────────────────────────────────────┘
               │ ISAProvider trait (platon-core)
┌──────────────▼──────────────────────────────────────────────────┐
│                         avap-isa                                 │
│  (Rust cdylib + PyO3)                                           │
│                                                                  │
│  AvapISA                                                         │
│    ├── InstructionSet (56 handlers registered)                   │
│    ├── h_push, h_load, h_store, h_add, ...                       │
│    ├── h_call_ext  ← calls Python via CALL_EXT                  │
│    └── h_load_conector, h_set_item ← direct VMState write       │
└──────────────┬──────────────────────────────────────────────────┘
               │ path dependency
┌──────────────▼──────────────────────────────────────────────────┐
│                        platon-core                               │
│  (Rust rlib — no PyO3)                                          │
│                                                                  │
│  Value, VMState, ISAProvider, InstructionSet, InstructionMeta   │
└─────────────────────────────────────────────────────────────────┘
```

---

## ISA Registration Protocol

The `AvapISA` Python object transfers an `Arc<dyn ISAProvider>` to the Platon VM using a fat pointer encoded as `(u64, u64)`:

```
Python:  vm.register_isa(AvapISA())
              │
              │ calls _get_arc_ptr() → (u64, u64)
              │
Rust:    Arc<dyn ISAProvider>
         └── Arc::into_raw() → *const dyn ISAProvider
         └── transmute → (data_ptr: u64, vtable_ptr: u64)

Platon:  transmute back → *const dyn ISAProvider
         Arc::from_raw() → Arc<dyn ISAProvider>
         vm.isa = Some(arc)
```

This is the only `unsafe` boundary in the ISA registration flow.

---

## Runtime Connector Routing

AVAP commands write to `self.conector.variables` and `self.conector.results` at runtime. Since Rust `Value::Dict` is a value type (not a reference), naive `GET_ATTR` / `SET_ITEM` chains would write to copies. `avap-isa` solves this with a marker-based routing scheme:

```
LOAD_CONECTOR     → push "__CONECTOR__" (string marker)
GET_ATTR "variables" → push "__CONECTOR_VARS__" (marker)
LOAD "x"          → push key
LOAD resolved     → push value
SET_ITEM          → detects "__CONECTOR_VARS__" marker
                    writes directly to VMState.conector_vars
```

`VMState.conector_vars` and `VMState.results` are `HashMap<String, Value>` fields added to `platon-core`. After `execute()`, the Language Server reads them back via `vm.conector_vars` and `vm.results`.

---

## Opcode Lifecycle

1. `opcodes.json` defines mnemonic, opcode byte, arg count, description
2. `build.rs` reads the JSON at cargo build time and emits `OUT_DIR/opcodes.rs`
3. `src/lib.rs` includes the generated file: `include!(concat!(env!("OUT_DIR"), "/opcodes.rs"))`
4. `AvapISA::new()` registers each opcode with `reg!(op::NAME, "NAME", n_args, handler_fn)`
5. Platon VM dispatches to handlers via `InstructionSet::get(opcode)`

---

## Bytecode Compilation

`avap-isa` does not include a compiler. The AVBC compiler lives in the Definition Server as `compiler.py`. It reads `opcodes.json` at runtime via the `ISA` class:

```python
isa = ISA('path/to/opcodes.json')
compiler = Compiler(isa=isa)
bytecode = compiler.compile(python_source)
```

The same `opcodes.json` drives both the Rust ISA implementation and the Python compiler, ensuring they are always in sync.
