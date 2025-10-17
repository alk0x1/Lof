# Lof Verification Suite

Validates mathematical equivalence between Lof's R1CS compiler and Circom.

## Quick Start

### First-time Setup
```bash
cd verification

# Install circomlib (required for some test circuits)
npm install
```

### Using Python directly
```bash
cd verification

# List all circuits
python3 verify.py --list

# Verify a single circuit
python3 verify.py multiply

# Verify all circuits
python3 verify.py --all

# Clean outputs
python3 verify.py --clean
```

### Using Makefile (from project root)
```bash
make verify-quick    # Quick test (multiply)
make verify-add      # Verify addition
make verify-all      # All circuits
make verify-clean    # Clean outputs
```

## Architecture

Single Python file (`verify.py`) that:
1. Compiles circuits in both Lof and Circom
2. Generates witnesses using lofit and snarkjs
3. Compares results for mathematical equivalence

## Dependencies

### Required
- **circom** - Circom compiler for reference implementations
- **Node.js + npm** - For running witness generators and snarkjs
- **snarkjs** - For converting Circom witnesses to JSON (`npm install -g snarkjs`)
- **circomlib** - Standard library for Circom circuits (`npm install` in verification/)
- **Python 3** - For running the verification script
- **Lof compiler** - Install via `make install` from project root
- **Cargo + Rust** - For building lofit (the Lof ZK toolkit)

### Installation
```bash
# Install global tools
npm install -g snarkjs

# Install Lof compiler
cd /path/to/Lof
make install

# Install circomlib (from verification directory)
cd verification
npm install
```

## Directory Structure

```
verification/
├── verify.py          # Single-file verification system
├── circuits/          # Test circuits (.lof + .circom)
├── test_cases/        # Test vectors (JSON)
└── outputs/           # Generated files
```

## Adding New Circuits

1. Create `circuits/XX_category/my_circuit.lof` and `my_circuit.circom`
2. Create `test_cases/XX_category/my_circuit_tests.json`
3. Run `python3 verify.py my_circuit`

## Test Status

**Current: 8/9 circuits passing (88.9%)**

### Passing Tests
- `multiply` - Basic multiplication
- `add` - Addition
- `subtract` - Subtraction
- `equality` - Equality comparison
- `simple_let` - Simple let bindings
- `nested_let` - Nested let bindings
- `compound_ops` - Complex compound operations
- `multi_witness` - Multiple witness variables

### Skipped/Future Work
- `less_than` - Requires comparison operator implementation in Lof compiler

## Recent Improvements

### Witness Solver Enhancement
Enhanced lofit's automatic witness solver to handle three constraint patterns:
- **Case 1**: A and B known → solve for C (original)
- **Case 2**: B and C known → solve for A (NEW)
- **Case 3**: A and C known → solve for B (NEW)

This allows solving constraints where the unknown appears in any position (A, B, or C terms).

### Variable Resolution Fix
Fixed R1CS compiler to properly resolve variable substitutions in let bindings. The compiler now calls `resolve_symbol_map_variables()` before creating let binding constraints, ensuring substitutions like `y -> t_0` are properly resolved.
