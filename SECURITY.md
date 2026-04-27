# Security Policy

## Supported Versions

| Version | Supported |
|---|---|
| 0.1.x | ✅ Yes |

---

## Reporting a Vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

Report security issues to: **security@avapcloud.com**

Please include:
- A description of the vulnerability
- Steps to reproduce (minimal PoC preferred)
- Potential impact assessment
- Your name/handle for acknowledgment (optional)

We will respond within **72 hours** and aim to release a patch within **14 days** for critical issues.

---

## Security Model

### What avap-isa guarantees

- **Sandboxed execution**: instruction handlers operate exclusively on `VMState` — they cannot access the host filesystem, network, or processes directly.
- **No arbitrary code execution**: `CALL_EXT` dispatches only to functions explicitly registered via `NativeRegistry`. It cannot invoke arbitrary callables.
- **Bytecode integrity**: the AVBC format supports HMAC-SHA256 signatures (enforced at the Language Server layer). The ISA itself does not validate signatures — that is the responsibility of the bytecode loader.
- **Timeout enforcement**: the Platon VM enforces per-execution timeouts and instruction count limits before dispatching to ISA handlers.

### What avap-isa does NOT guarantee

- **Bytecode authenticity**: the ISA executes whatever bytecode is loaded. Callers are responsible for verifying bytecode integrity before loading.
- **Memory isolation between executions**: `VMState` is reused across calls in the same VM instance. Callers must reset state between untrusted executions.
- **Protection against malicious `CALL_EXT` targets**: if a caller registers a malicious Python function via `NativeRegistry`, `CALL_EXT` will invoke it. Only register trusted callables.

### `unsafe` usage

`avap-isa` contains minimal `unsafe` code, strictly limited to:

1. **Fat pointer transfer** (`_get_arc_ptr` / `register_isa`): transferring `Arc<dyn ISAProvider>` across the Python/Rust boundary as `(u64, u64)`. The invariant is maintained by the `platon` crate which reconstructs the `Arc` immediately.

2. **GIL assumption** (`py_from_ctx`): the `py_ctx` pointer is valid for the duration of `CALL_EXT` execution. The Platon VM guarantees this by holding the GIL during `execute()`.

All `unsafe` blocks have accompanying `// SAFETY:` comments explaining the invariants.

---

## Known Limitations

- **Integer overflow**: arithmetic operations use wrapping semantics for `i64`. No overflow traps.
- **Division by zero**: `DIV` and `MOD` return an `ISAError` on zero divisor; they do not panic.
- **Stack depth**: no hard stack depth limit beyond available memory. Production deployments should enforce VM-level limits.

---

## Acknowledgments

We thank the following researchers for responsible disclosures: *(none yet)*
