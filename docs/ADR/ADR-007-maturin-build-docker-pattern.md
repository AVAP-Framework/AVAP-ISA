---
date: 2026-03-22
status: Accepted
project: Platon VM Kernel + AVAP ISA v2
---

# ADR-007: maturin build + pip install wheel Pattern for Docker

**Date:** 2026-03-22
**Status:** Accepted
**Authors:** Rafael Ruiz (101OBEX, Corp - CTO)

### Context

During Docker container startup, `platon` and `avap-isa` must be compiled from source (mounted volumes). The typical `maturin develop` command requires a virtual environment. However, the macOS development `.venv` contains macOS ARM binaries and a `pyvenv.cfg` that confuses maturin when the volume is mounted in a Linux container, causing a cross-compilation error.

### Decision

Use `maturin build --release` to compile a Linux wheel, then `pip install` the wheel (selecting the most recently built one with `ls -t | head -1`). This bypasses the need for a virtual environment entirely.

For `avap-isa`, copy the source to `/tmp` before building to ensure no macOS artifacts (`.venv`, `target/`) are visible to maturin.

### Alternatives Considered

**`maturin develop` with `VIRTUAL_ENV=`**: Forces maturin to use system Python. Rejected — maturin still finds the `pyvenv.cfg` in the mounted PLATON directory and interprets the macOS `bin/python` symlink as a cross-compilation target.

**`PYO3_PYTHON` env var**: Explicitly set Python interpreter. Tested extensively — maturin 1.5 ignores this in the presence of a `pyvenv.cfg`.

**Pre-build in Dockerfile**: Build `avap-isa` during `docker build` using `COPY`. Requires changing the build context to the parent directory — works but makes the Dockerfile less portable and increases build time on every source change.

**Symlink `/build` on macOS**: Create a symlink so paths match inside the container. Rejected — macOS root filesystem is read-only since Catalina.

**Delete `.venv` from PLATON repo**: The simplest solution. Adopted for development — developers delete their local `.venv`, and the container build works cleanly.

### Consequences

- The Language Server Dockerfile CMD builds wheels at every container start (~2-3 minutes for `avap-isa`, ~1 minute for `platon`)
- Old wheels accumulate in `target/wheels/` — the `ls -t | head -1` pattern ensures only the newest is installed
- Production deployments should pre-build wheels in CI and include them in the Docker image rather than building at startup

---
