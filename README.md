# Lof

> A type-safe language for zero-knowledge proof circuits

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](.)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)

## Overview

Lof is a type-theoretic language designed to prevent ZK circuit vulnerabilities through strong static typing and formal guarantees. It compiles directly to R1CS (Rank-1 Constraint System) constraints for use in zero-knowledge proof systems.

### Key Features

- **Strong Static Types** - Catch errors at compile-time, not during proof generation
- **Visibility Tracking** - Track information flow with `input`, `witness`, and `output` modifiers
- **Explicit Constraints** - Predictable R1CS generation with the `===` constraint operator
- **Pattern Matching** - First-class pattern matching with exhaustiveness checking
- **Field Arithmetic** - Native finite field operations (BN254 curve)
- **Full zkSNARK Workflow** - Integrated Groth16 proving system via `lofit` toolkit

## Quick Start

### Installation

```bash
# Clone and build
git clone https://github.com/yourusername/lof.git
cd lof
make install

# Verify installation
lof --version
```

### Your First Circuit

Create `multiply.lof`:

```rust
proof Multiply {
    input a: field;
    input b: field;
    output c: field;

    c === a * b
}
```

Compile and prove:

```bash
# Compile to R1CS
lof compile multiply.lof

# Generate proving/verification keys
lofit setup -i multiply.r1cs

# Generate a proof
lofit prove -i multiply.r1cs

# Verify the proof
lofit verify -i multiply.r1cs
```

## Language Examples

### Range Proof

```rust
proof RangeProof {
    input value: field;
    witness bits: (field, field, field, field, field, field, field, field);
    output valid: bool;

    let (b7, b6, b5, b4, b3, b2, b1, b0) = bits in
    let reconstructed = b7 * 128 + b6 * 64 + b5 * 32 + b4 * 16 +
                        b3 * 8 + b2 * 4 + b1 * 2 + b0 in

    assert b0 * (b0 - 1) === 0;
    assert value === reconstructed;
    assert value < 256;

    valid === true
}
```

### Pattern Matching

```rust
proof Conditional {
    input x: field;
    output result: field;

    let value = match x with
        | 0 => 10
        | 1 => 20
        | _ => 30
    in

    result === value
}
```

## Documentation

- [Language Syntax](docs/syntax.md) - Complete syntax reference
- [Getting Started](docs/getting-started.md) - Detailed tutorial
- [Developer Guide](CLAUDE.md) - For contributors
- [Contributing](CONTRIBUTING.md) - How to contribute

## Architecture

- **lof** - Core compiler (lexer, parser, typechecker, R1CS compiler)
- **cli** - Command-line interface
- **lofit** - ZK toolkit for Groth16 proof generation/verification (arkworks)
- **lof-codegen** - Code generation utilities (WASM support)

## Development

```bash
# Quick development cycle
make dev

# Run tests
make test-all

# Run verification suite (requires circom/snarkjs)
make verify-all
```

### Testing

Three-tier testing strategy:

- **Tier 1** - Fast checks (~seconds): `make test-fast`
- **Tier 2** - Integration tests (~minutes): `make test-integration`
- **Tier 3** - Verification tests (~minutes): `make verify-all`

Current status: **25/25 circuits passing** (100% verification rate)

## Project Status

**Version:** 0.1.0 (MVP)

Lof is in active development. Core features are stable and well-tested, but the API may change before 1.0.

### Working Features

✅ Core language implementation (lexer, parser, typechecker)
✅ R1CS compilation
✅ Pattern matching with exhaustiveness checking
✅ Field arithmetic and constraints
✅ Full Groth16 proving workflow
✅ Comprehensive test suite (100% verification pass rate)

### Roadmap

See [ROADMAP.md](ROADMAP.md) for planned features.

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Comparison with Other Languages

| Feature | Lof | Circom | Noir | ZoKrates |
|---------|-----|--------|------|----------|
| Strong Static Types | ✅ | ❌ | ✅ | ✅ |
| Visibility Tracking | ✅ | ❌ | ⚠️ | ❌ |
| Pattern Matching | ✅ | ❌ | ✅ | ❌ |
| Explicit Constraints | ✅ | ⚠️ | ❌ | ❌ |
| Direct R1CS Output | ✅ | ✅ | ❌ | ❌ |

## License

Dual-licensed under MIT or Apache-2.0 (your choice).

## Acknowledgments

- Built with [arkworks](https://github.com/arkworks-rs) cryptography libraries
- Verified against [Circom](https://github.com/iden3/circom) reference implementation

---

**Status:** MVP - Extensively tested but use in production at your own risk.
