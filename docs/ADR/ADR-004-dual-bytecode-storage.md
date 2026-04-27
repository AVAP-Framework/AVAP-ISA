---
date: 2026-03-22
status: Accepted
project: Platon VM Kernel + AVAP ISA v2
**Authors:** Rafael Ruiz (101OBEX, Corp - CTO)
---

# ADR-004: Dual Bytecode Storage for Backwards Compatibility

**Date:** 2026-03-22
**Status:** Accepted
**Authors:** Rafael Ruiz (101OBEX, Corp - CTO)

### Context

The Definition Server currently stores Python source as HMAC-signed binary (`AVAP` format). The Language Server uses this to run `exec()`. We need to introduce AVBC bytecode without breaking existing Language Server deployments.

### Decision

Add three columns to the `avap_bytecode` table:
- `avbc_bytecode BYTEA` — compiled AVBC instructions
- `avbc_version SMALLINT` — ISA version used to compile
- `avbc_compiled_at TIMESTAMP` — compilation timestamp

The `bytecode` column (legacy) is preserved unchanged. Both columns are populated at startup. The gRPC `CommandResponse` message includes both `code` (legacy) and `avbc_code` (new).

The Language Server selects the execution path at runtime:
```python
if cmd_name in self.avbc_cache:
    # Direct Rust VM path — no exec()
else:
    # Legacy CALL_EXT path — exec() via native bridge
```

### Alternatives Considered

**Replace legacy bytecode**: Simpler schema. Rejected — breaks any Language Server not yet updated to use AVBC.

**Separate table for AVBC**: Cleaner schema. Rejected — requires JOIN on every catalog sync; single table is simpler.

**Feature flag**: Gate AVBC on an env var. Considered but not adopted as primary mechanism — the command-level fallback is more granular and automatic.

### Consequences

- Zero-downtime migration: old Language Server instances continue using legacy path
- New Language Server instances automatically use AVBC when available
- Definition Server must run both compilers on startup — adds ~2-5s to boot time for 27 commands
- If AVBC compilation fails for a command, that command falls back gracefully to `exec()`

---
