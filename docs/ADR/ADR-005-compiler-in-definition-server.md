---
date: 2026-03-22
status: Accepted
project: Platon VM Kernel + AVAP ISA v2
**Authors:** Rafael Ruiz (101OBEX, Corp - CTO)
---

# ADR-005: Python→AVBC Compiler Lives in the Definition Server

**Date:** 2026-03-22
**Status:** Accepted
**Authors:** Rafael Ruiz (101OBEX, Corp - CTO)

### Context

The AVBC compiler (`compiler.py`) could live in several places: in `avap-isa`, in a standalone `avap-compiler` package, or in the Definition Server.

### Decision

Place `compiler.py` in the Definition Server repo. It is invoked by `compiler.js` (a Node.js bridge) as a child process during `loadDefinitions()` at server startup.

### Alternatives Considered

**Compiler in `avap-isa`**: Natural home since it uses the ISA. Rejected — `avap-isa` is a Rust/Python crate; adding a compiler would require Python packaging complexity and would blur the crate's focus.

**Standalone `avap-compiler` package**: Clean separation. Rejected — adds a third dependency for what is currently a single Python file; premature for the current scale.

**Compile at Language Server startup**: Rejected — Language Server should be a pure execution node with no compile-time dependencies.

**Compile on first request**: Rejected — cold start latency on first use of each command.

### Consequences

- The Definition Server requires Python in its Docker container (`apk add python3`)
- `compiler.py` reads `opcodes.json` from the filesystem — the path must be correct inside the container
- File permissions on `opcodes.json` must be set to 644 before switching to `USER node` in the Dockerfile
- Compilation is cached in the database — subsequent starts skip recompilation unless source changes

---
