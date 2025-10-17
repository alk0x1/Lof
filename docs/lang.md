# Lof Language Reference

## Overview

Lof is a type-safe language for zero-knowledge circuits that compiles to R1CS (Rank-1 Constraint Systems). It prevents common ZK vulnerabilities through static type checking.

## Core Concepts

### R1CS Constraints

**Fundamental Principle: Every Computation Must Be Proven**

In zero-knowledge circuits, **every computation generates constraints** because the prover must prove they computed everything correctly. Without constraints, a prover could lie about computations and the verifier would have no way to detect it.

ZK circuits are constraint systems where operations generate constraints of the form `A * B = C`. Understanding which operations generate constraints vs. linear combinations is crucial for circuit optimization.

#### Why Constraints Are Required

A ZK proof proves "I know secret values that satisfy these constraints" without revealing the values. The constraints ARE the proof system.

**Example:**
```lof
// Without constraints, prover could lie:
let result = a < b  // Prover: "I computed this, trust me!"

// With constraints, computation is proven:
let result = a < b  // Generates ~66 constraints proving the comparison was done correctly
```

If a computation didn't generate constraints, the verifier couldn't verify it was computed honestly!

#### Operations That Generate R1CS Constraints (Expensive)

- **Multiplication**: `a * b` (1 constraint)
- **Division**: `a / b` (multiple constraints)
- **Comparisons**: `a < b`, `a > b`, `a <= b`, `a >= b` (~66 constraints each - requires bit decomposition)
- **Equality checks**: `a == b` (1 constraint)
- **Boolean operations**: `a && b`, `a || b` (implemented via multiplication)
- **Assertions**: `a === b` (1 constraint enforcing equality)
- **Conditionals/Pattern matching**: `match x with ...` (generates constraints for each branch)

#### Operations That Are "Free" (Linear Combinations)

These don't generate new constraints, they just combine existing terms:
- **Addition**: `a + b`
- **Subtraction**: `a - b`
- **Scalar multiplication**: `2 * a`, `3 * b` (when one operand is a compile-time constant)
- **Variable access**: Reading a variable

**Why they're free:** Linear combinations can be folded into existing constraints without creating new ones.

#### Constraint Assignment vs Comparison

Lof has two distinct operators that generate constraints differently:

**`===` (Constraint Assertion)** - Forces two values to be equal:
```lof
result === (a < b)  // Creates constraint: result * 1 = (comparison_result)
// Returns: empty (unit-like type)
```

**`==` (Equality Comparison)** - Computes equality and returns the result:
```lof
let is_equal = (a == b)  // Creates constraint computing equality
// Returns: temporary variable holding 0 or 1
```

**Important:** Both generate constraints! The difference is:
- `===` enforces equality (assertion)
- `==` computes equality (returns a value)

#### Circuit Size Implications

More constraints = larger proof + slower proving:
- **10 constraints**: Milliseconds
- **1,000 constraints**: Seconds
- **1,000,000 constraints**: Minutes

**Optimization example:**
```lof
// ❌ Inefficient: Multiple multiplications
let x = a * b in
let y = x * c in
let z = y * d in
result === z
// Generates 3 constraints

// ✅ Better: Minimize multiplications where possible
let x = a * b in
result === x + c + d
// Generates 1 constraint (addition is free!)
```

#### Why R1CS is "Rank-1"

R1CS constraints have exactly **one multiplication** per constraint:
- ✅ Valid: `a * b = c` (one multiplication)
- ❌ Invalid: `(a * b) * c = d` (two multiplications - needs 2 constraints)
- ✅ Valid: `a + b + c = d` (no multiplication, just linear combination)

## Types

### Primitive Types

#### field
Element of finite field F_p (BN254 scalar field, approximately 2^254).

```rust
let x: field = 42;
let y: field = x * 2;
```

#### bool
Boolean value, represented as 0 or 1 in the field.

```rust
let is_positive: bool = x > 0;
assert is_positive;
```

#### unit
Type for expressions that don't return a value (e.g., assertions).

```rust
assert x > 0;  // Type: unit
```

### Composite Types

#### Tuples
Fixed-size heterogeneous collections.

```rust
let pair: (field, field) = (x, y);
let (a, b) = pair;
```

#### Arrays
Fixed-size homogeneous collections.

```rust
let arr: array<field, 3> = [1, 2, 3];
let first = arr[0];
```

#### Type Aliases

```rust
type Balance = field;
type Point = (field, field);
```

#### Enums

```rust
type Action =
    | Deposit(field)
    | Withdraw(field)
    | Transfer(field, field);
```

## Syntax

### Proof Blocks

The main entry point for a circuit.

```rust
proof Example {
    input x: field;      // Public input
    witness w: field;    // Private witness

    let result = x * w in
    result
}
```

### Let Bindings

```rust
let <name> = <expr> in <body>

// Example
let doubled = x * 2 in
let result = doubled + 1 in
result
```

### Pattern Matching

```rust
match <expr> with
    | <pattern> => <expr>
    | <pattern> => <expr>

// Example
match x with
    | 0 => y
    | n => n * 2
```

Patterns can be:
- Literal values: `0`, `1`, etc.
- Variable bindings: `n`
- Wildcards: `_`
- Constructor patterns: `Deposit(amount)`
- Tuple patterns: `(x, y)`

### Assertions

```rust
assert x > 0;
assert balance >= amount;
assert a * b == c;
```

Assertions generate constraints that must be satisfied for the proof to be valid.

### Functions

```rust
let multiply_and_add (a: field) (b: field) (c: field): field =
    let product = a * b in
    product + c

// Usage
let result = multiply_and_add(x, y, z) in
result
```

Functions are inlined during compilation - they don't exist at the R1CS level.

### Witnesses

Explicit witness declarations for non-deterministic values:

```rust
witness flag: field;
witness bits: array<field, 252>;
```

All witnesses must be properly constrained through assertions or operations that generate constraints.

## Operators

### Arithmetic

```rust
a + b   // Addition (linear combination)
a - b   // Subtraction (linear combination)
a * b   // Multiplication (R1CS constraint)
a / b   // Division (R1CS constraint, requires b != 0)
```

### Comparison

```rust
a == b  // Equality
a != b  // Inequality
a < b   // Less than
a > b   // Greater than
a <= b  // Less than or equal
a >= b  // Greater than or equal
```

All comparisons generate R1CS constraints.

### Logical

```rust
a && b  // Logical AND
a || b  // Logical OR
!a      // Logical NOT
```

### Field Operations

```rust
inv(x)        // Modular inverse (1/x)
pow(x, n)     // Exponentiation
```

## Examples

### Range Proof

```rust
proof RangeProof {
    input value: field;

    assert value >= 0;
    assert value < 100;
    value
}
```

### Merkle Tree Verification

```rust
let hash (left: field) (right: field): field =
    left * left + right * right + left + right + 7

let verify_merkle_proof (leaf: field) (root: field)
                        (path_indices: array<field, 2>)
                        (path_elements: array<field, 2>): () =
    let (path_idx_0, path_idx_1) = (path_indices[0], path_indices[1]) in
    let (path_el_0, path_el_1) = (path_elements[0], path_elements[1]) in

    // Level 1
    let left1  = (1 - path_idx_0) * leaf + path_idx_0 * path_el_0 in
    let right1 = path_idx_0 * leaf + (1 - path_idx_0) * path_el_0 in
    let hash1 = hash(left1, right1) in

    // Level 2
    let left2  = (1 - path_idx_1) * hash1 + path_idx_1 * path_el_1 in
    let right2 = path_idx_1 * hash1 + (1 - path_idx_1) * path_el_1 in
    let hash2 = hash(left2, right2) in

    assert hash2 == root

proof MerkleProof {
    input leaf: field;
    input root: field;
    input path_indices: array<field, 2>;
    witness path_elements: array<field, 2>;

    verify_merkle_proof(leaf, root, path_indices, path_elements)
}
```

### Conditional Logic with Pattern Matching

```rust
type Option =
    | Some(field)
    | None;

proof ProcessOption {
    input opt: Option;

    match opt with
        | Some(value) => assert value > 0; value
        | None => 0
}
```

## R1CS Compilation

### Variable Allocation

Public inputs and private witnesses become variables in the constraint system:

```rust
proof Example {
    input x: field;      // → Public variable
    witness w: field;    // → Private variable
    let y = x * w in     // → New witness variable for result
    y
}
```

### Constraint Generation

Each multiplication creates a constraint:

```rust
let product = a * b;
// Generates: (a) * (b) = (product)
```

Complex expressions require intermediate witnesses:

```rust
let result = (a + b) * c;
// Must be rewritten as:
let sum = a + b in      // Linear combination, no constraint
let result = sum * c in // Constraint: (sum) * (c) = (result)
result
```

### Function Inlining

Functions are completely inlined during compilation:

```rust
let square (x: field): field =
    x * x

proof Main {
    input x: field;
    let y = square(x) in
    y
}

// Compiles to:
// Variables: x (input), y (witness)
// Constraints: (x) * (x) = (y)
```

## Security Considerations

### Unconstrained Witnesses

The most common vulnerability in ZK circuits. All witnesses must participate in constraints:

**Vulnerable:**
```rust
proof Buggy {
    witness w: field;
    let y = w + 5 in  // Addition doesn't constrain w
    assert y > 0;     // Only constrains y, not w
}
// ERROR: w is unconstrained - prover can set it to any value
```

**Secure:**
```rust
proof Secure {
    witness w: field;
    let y = w * 2 in  // Multiplication constrains w
    assert y > 0;
}
```

### Division by Zero

Always ensure denominators are non-zero:

```rust
proof SafeDivision {
    input x: field;
    input y: field;

    assert y != 0;        // Required
    let result = x / y in
    result
}
```

### Field Overflow

Field elements wrap around at the modulus. Use range checks for bounded values:

```rust
proof BoundedValue {
    input value: field;

    assert value >= 0;
    assert value < 1000;  // Explicitly bound the value
    value
}
```

## Best Practices

1. **Minimize constraints**: Prefer linear combinations (addition/subtraction) over multiplication when possible
2. **Reuse calculations**: Store intermediate results in let bindings
3. **Explicit witnesses**: Only use explicit `witness` declarations when the value is truly non-deterministic
4. **Clear assertions**: Use assertions to document and enforce invariants
5. **Helper functions**: Break complex circuits into reusable functions
6. **Constant exponents**: Use compile-time constant exponents in `pow()` for efficiency

## Compiler Commands

```bash
# Parse only
lof parse file.lof

# Type check
lof typecheck file.lof

# Compile to R1CS
lof compile file.lof

# Full proof generation (future)
lof prove file.lof
lof verify file.lof
```
