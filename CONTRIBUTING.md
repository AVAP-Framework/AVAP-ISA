# Contributing to avap-isa

Thank you for your interest in contributing. This document covers everything you need to get started.

---

## Code of Conduct

This project follows the [Contributor Covenant](https://www.contributor-covenant.org/) Code of Conduct. By participating, you agree to uphold it. Report unacceptable behavior to security@avapcloud.com.

---

## Ways to Contribute

- **Bug reports** — open a GitHub Issue with the `bug` label
- **Feature requests** — open a GitHub Issue with the `enhancement` label
- **New opcodes** — see [Adding a New Opcode](#adding-a-new-opcode)
- **New method support** — extend `call_method()` in `src/lib.rs`
- **Documentation** — fix typos, improve examples, clarify specs
- **Tests** — add test cases in `tests/`

---

## Development Setup

### Prerequisites

```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable

# Python 3.11+
python3 --version

# maturin
pip install maturin

# Clone repos (avap-isa depends on platon-core)
git clone https://github.com/avapcloud/platon ../PLATON
git clone https://github.com/avapcloud/avap-isa
cd avap-isa
```

### Build & test

```bash
# Compile the extension
maturin develop

# Verify basic functionality
python3 -c "from avap_isa import AvapISA; print(AvapISA())"
# <AvapISA v0.1.0 (56 opcodes)>

# Run tests
cargo test
```

---

## Project Structure

```
avap-isa/
├── opcodes.json          ← Opcode definitions (EDIT THIS to add opcodes)
├── build.rs              ← Generates op:: Rust constants from opcodes.json
├── Cargo.toml
├── pyproject.toml
├── src/
│   └── lib.rs            ← Instruction handlers + PyO3 bindings
├── python/
│   └── avap_isa/
│       └── __init__.py   ← Python package init
└── tests/                ← Integration tests
```

---

## Adding a New Opcode

Adding an opcode requires changes to exactly **two files**:

### 1. `opcodes.json`

Add your instruction to the `instructions` object. Choose an unused opcode byte:

```json
"MY_INSTR": {
  "opcode": 131,
  "args": 1,
  "description": "Does something useful"
}
```

The `args` field counts the number of `u32` arguments that follow the opcode byte in the instruction stream.

### 2. `src/lib.rs`

Add a handler function:

```rust
fn h_my_instr(s: &mut VMState, c: &[u8], ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let idx = read_u32(c, ip)? as usize;
    // ... implementation
    Ok(())
}
```

Register it in `AvapISA::new()`:

```rust
reg!(op::MY_INSTR, "MY_INSTR", 1, h_my_instr);
```

That is all. The `build.rs` script will automatically generate `op::MY_INSTR = 0x83` from `opcodes.json` on the next compile.

### Guidelines for new opcodes

- Opcode bytes `0x00–0xFF` are the complete space. Choose values that fit the existing categories (see README for ranges).
- Prefer composing existing opcodes over adding new ones for simple operations.
- Always add a test case demonstrating the new opcode.
- Document the stack effect clearly in `opcodes.json` `description`.

---

## Pull Request Process

1. **Fork** the repo and create a branch: `git checkout -b feat/my-feature`
2. Make your changes following the coding standards below
3. Add or update tests
4. Run `cargo test` and `cargo clippy -- -D warnings`
5. Update `CHANGELOG.md` under `[Unreleased]`
6. Open a PR with a clear description of what and why

### PR requirements

- All CI checks must pass
- At least one maintainer approval required
- No `unsafe` additions without justification in PR description
- New opcodes must include an `opcodes.json` entry, a handler, and a test

---

## Coding Standards

- **Rust**: follow `rustfmt` defaults (`cargo fmt`)
- **No panics** in instruction handlers — always return `Err(ISAError)` on failure
- **No allocations** in hot paths where avoidable
- **Stack discipline**: every handler that pops must document what it expects; every handler that pushes must document what it produces
- **Comments**: explain *why*, not *what*

---

## Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(isa): add MATCH_PATTERN opcode (0x83)
fix(call_method): handle isdigit on non-string values
docs(readme): clarify AVBC header format
chore(deps): update pyo3 to 0.21
```

---

## Questions?

Open a Discussion on GitHub or reach out at dev@avapcloud.com.
