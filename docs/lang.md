## Let Binding
```rust
// Syntax
let <name> = <expr> in <body>

// Example:
let doubled = x * 2 in doubled

// Type Transformation
// Source:
input x: field;
let doubled = x * 2 in doubled

// After Type Checking:
input x: field^{linear};
let doubled: field^{linear, witness, constraint: doubled = x * 2} = 
    x^{consume} * 2 in 
doubled^{consume}

// Type Checking Steps
x has type field^{linear}

x * 2 consumes x, returns field^{needs_witness}

let doubled = creates witness with constraint obligation

doubled gets type field^{linear, has_constraint}

Using x again would ERROR: "x already consumed"

// R1CS Generation
// Variables:
- ONE (constant 1)
- x (public input)
- doubled (witness)

// Constraints:
- doubled = x * 2
  => (x) * (2) = (doubled)
  => A=[x], B=[2*ONE], C=[doubled]

```

## Dup Function
```rust
// syntax
let <name> = dup(<linear_value>) in <body>

//   Non-linear Value Usage
Explicit duplication: let (x1, x2) = dup(x) in let squared = x1 * x2
let x_copy = dup(x) in
let a = x_copy * 2 in
let b = x_copy * 3 in    // x_copy still available
let c = x_copy * x_copy  // still ok


// Intermediate Computations
let y = (a + b) * c needs witness for a + b
let sum = a + b in let y = sum * c

// circuit equality
let y = x * 2 in y // Compiler generates constraint: y === x * 2

// Input Types
input x: field^{linear};
dup(x): field^{copyable} 
let x_c = dup(x) in
let squared = x_c * x_c

// dup copyables aren't allowed
let x_c = dup(x) in      // x_c: field^{copyable}
let x_cc = dup(x_c) in    // It will reurn an error

// R1CS for dup:
let x_copy = dup(x) in ...

// Generates:
// Variables: x_copy (witness)
// Constraint: x_copy = x
```

## Constants and Literals
```rust
// Constants are implicitly copyable
2: field^{copyable}
let four = 2 * 2  // OK: 2 is copyable
```

## Arithmetic Operators
```rust
// Syntax
<expr> + <expr>
<expr> - <expr>  
<expr> * <expr>

// Type Rules
// Addition
a: field^{linear}, b: field^{linear}
a + b: field^{linear}
// consumes both a and b

// Subtraction  
a: field^{linear}, b: field^{linear}
a - b: field^{linear}
// consumes both a and b

// Multiplication
a: field^{linear}, b: field^{linear}
a * b: field^{linear}
// consumes both a and b

// Examples
let sum = a + b in sum
let diff = a - b in diff
let prod = a * b in prod

// Complex Expression
let result = (a + b) * c
// ERROR: needs intermediate binding

let sum = a + b in
let result = sum * c in result
// OK: explicit witness for sum

// R1CS Generation
// Addition: a + b
// No new witness needed (linear combination)

// Subtraction: a - b  
// No new witness needed (linear combination)

// Multiplication: a * b
// When in let binding:
let prod = a * b
// Variables: prod (witness)
// Constraint: (a) * (b) = (prod)

// Nested multiplication:
let x = a * b in
let y = x * c
// Variables: x (witness), y (witness)
// Constraints: 
// - (a) * (b) = (x)
// - (x) * (c) = (y)
```

## Basic Proof Structure
```rust
/// Syntax
proof <Name> {
    <signal_declarations>
    <body_expression>
}

// Signal Declaration Syntax
input <name>: <type>;
witness <name>: <type>;

// Example
proof Example {
    input x: field;
    witness w: field;
    
    let result = x * w in
    result
}

// Type Rules
input x: field    => x: field^{linear, public}
witness w: field  => w: field^{linear, private}

// Final expression becomes the output
// Must be fully constrained

// R1CS Generation
// Variables:
- x (public input)
- w (private witness)
- result (witness, becomes output)

// Constraints:
- (x) * (w) = (result)

// Error Case
proof Invalid {
    input x: field;
    witness w: field;
    
    w  // ERROR: w not constrained to inputs
}
```

## Pattern Matching
```rust
// Syntax
match <expr> with
    | <pattern> => <expr>
    | <pattern> => <expr>

// Pattern Syntax
<constructor>(<pattern>, ...)  // Constructor pattern
<variable>                      // Variable binding
<literal>                       // Literal value
_                              // Wildcard

// Example
match x with
    | 0 => y
    | n => n * 2


// Type Rules
// Match expression consumes the scrutinee
x: field^{linear}
match x with ...: field^{linear}
// x is consumed

// All branches must return same type
match x with
    | 0 => 1        // field^{linear}
    | n => n * 2    // field^{linear}

// Pattern variables are linear
match x with
    | 0 => y
    | n => let doubled = n * 2 in doubled  // n consumed

// R1CS Generation
match x with
    | 0 => a
    | _ => b

// Generates:
// Variables: 
// - x_is_zero (witness, boolean)
// - result (witness)

// Constraints:
// - x * x_is_zero = 0  // x is 0 when x_is_zero = 1
// - result = x_is_zero * a + (1 - x_is_zero) * b

// Error Cases
match x with
    | 0 => x  // ERROR: x already consumed by match
    | n => n

match x with
    | 0 => 1     // returns field
    | n => true  // ERROR: returns bool, type mismatch

```

## Type aliases
```rust
// Syntax
type <name> = <type>

// Examples
type Balance = field
type Point = (field, field)
type Address = field

// Usage
proof Transfer {
    input from_balance: Balance;
    input amount: Balance;
    
    let new_balance = from_balance - amount in
    new_balance
}

// R1CS Generation
// Type aliases don't affect R1CS
// They are resolved during type checking
```

## Enum Types
```rust
type <name> =
    | <constructor>
    | <constructor>(<type>, ...)

// Example
type Action =
    | Deposit(field)
    | Withdraw(field)
    | Transfer(field, field)

// In R1CS, enum becomes multiple witnesses:
// - action_tag: field (0, 1, or 2)
// - action_data_0: field
// - action_data_1: field (unused for Deposit/Withdraw)

// Usage
match action with
    | Deposit(amount) => balance + amount
    | Withdraw(amount) => balance - amount
    | Transfer(from, to) => from - to

// R1CS Translation
// Variables:
- action_tag (witness)
- action_data_0 (witness) 
- action_data_1 (witness)
- is_deposit (witness, boolean)
- is_withdraw (witness, boolean)
- is_transfer (witness, boolean)
- result (witness)

// Constraints:
// Tag validity
- is_deposit * (action_tag - 0) = 0
- is_withdraw * (action_tag - 1) = 0  
- is_transfer * (action_tag - 2) = 0
- is_deposit + is_withdraw + is_transfer = 1

// Boolean constraints
- is_deposit * (1 - is_deposit) = 0
- is_withdraw * (1 - is_withdraw) = 0
- is_transfer * (1 - is_transfer) = 0

// Result computation
- result = is_deposit * (balance + action_data_0) +
          is_withdraw * (balance - action_data_0) +
          is_transfer * (action_data_0 - action_data_1)

// Memory Layout Example
Deposit(100):
- action_tag = 0
- action_data_0 = 100
- action_data_1 = 0 (unused)

Transfer(500, 200):
- action_tag = 2
- action_data_0 = 500
- action_data_1 = 200

// Enums need witnesses for the tag AND the maximum data size across all variants, plus boolean flags for each branch.
```

## Arrays
```rust
[<expr>, <expr>, ...]              // Array literal
<array>[<index>]                    // Array access
array<<type>, <size>>               // Array type

// Examples
let arr = [1, 2, 3] in
let first = arr[0] in
first

// Type Rules
[1, 2, 3]: array<field, 3>^{linear}
arr[i]: field^{linear}
// Array access consumes both array and index

// Fixed Size Only
input values: array<field, 10>;    // Size must be constant
let arr = [x, y, z] in             // Size inferred: 3
arr

// Linearity
let arr = [1, 2, 3] in
let x = arr[0] in
let y = arr[1] in    // ERROR: arr already consumed

// Must dup for multiple accesses
let arr = [1, 2, 3] in
let arr_copy = dup(arr) in
let x = arr_copy[0] in
let y = arr_copy[1] in    // OK: arr_copy is copyable
x + y

// R1CS Generation
// Array literal
let arr = [a, b, c]
// Variables:
- arr_0 = a (witness)
- arr_1 = b (witness)  
- arr_2 = c (witness)

// Array access
let x = arr[i]

// R1CS
// Variables:
- i (must be witness)
- x (witness)
- is_0, is_1, is_2 (witness, boolean)

// Constraints:
- is_0 * (i - 0) = 0
- is_1 * (i - 1) = 0
- is_2 * (i - 2) = 0
- is_0 + is_1 + is_2 = 1
- x = is_0 * arr_0 + is_1 * arr_1 + is_2 * arr_2

// Error Cases
let arr = [1, 2, 3] in
arr[3]    // ERROR: index out of bounds (compile-time)

let arr = [1, 2, 3] in
arr[i]    // OK if i is witness, adds bounds check constraints
```

## Assertions
```rust 
// Syntax
assert <expr>

// Examples
assert x > 0
assert balance >= amount
assert a * b == c

// Type Rules
assert <expr>: ()
// Expression must be boolean type
// Returns unit
// Consumes variables in expression

// Usage Patterns
proof Transfer {
    input balance: field;
    input amount: field;
    
    assert balance >= amount;
    let new_balance = balance - amount in
    new_balance
}

// In Let Binding
let _ = assert x > 0 in
let y = x * 2 in y

// Chained Assertions
assert x > 0;
assert x < 100;
let y = x * 2 in y

// R1CS Generation
assert x > 0

// Variables:
- x_positive (witness, boolean)
- x_bits[252] (witness array, bit decomposition of x)

// Constraints:
// 1. Decompose x into bits
- x = sum(x_bits[i] * 2^i)
- x_bits[i] * (1 - x_bits[i]) = 0 for all i  // Boolean check

// 2. Check if positive (highest bit = 0 for positive)
- x_positive = (1 - x_bits[251])  

// 3. Force assertion to be true
- (1 - x_positive) = 0  // This fails if x <= 0

// Complex Assertion
assert balance >= amount

// R1CS Generation:
// Variables:
- diff (witness) = balance - amount
- diff_bits[252] (witness array)
- is_non_negative (witness, boolean)


// Simple Equality
assert a == b
// Constraint: a - b = 0


// Complex Assertion
assert balance >= amount

// R1CS Generation:
// Variables:
- diff (witness) = balance - amount
- diff_bits[252] (witness array)
- is_non_negative (witness, boolean)

// Constraints:
// 1. Compute difference
- diff = balance - amount

// 2. Bit decomposition
- diff = sum(diff_bits[i] * 2^i)
- diff_bits[i] * (1 - diff_bits[i]) = 0

// 3. Check sign (in field, negative = very large number)
- is_non_negative = (1 - diff_bits[251])

// 4. Force to be true
- (1 - is_non_negative) = 0



// assertions add a constraint that forces something to equal 1 or 0
// If the assertion is false, the constraint is unsatisfiable

```

## Functions
```rust
// Syntax
let <name> (<arg1>: <type1>) (<arg2>: <type2>) ... : <return_type> =
    <body>

// Example: Definition
let multiply_and_add (a: field) (b: field) (c: field): field =
    let product = a * b in
    product + c

// Example: Usage
proof MainProof {
    input x: field;
    input y: field;
    input z: field;

    let result = multiply_and_add (x) (y) (z) in
    result
}

// Type Rules
// - Function arguments are treated as linear inputs, consumed within the function body.
// - The function body is a single expression, and its type must match the declared return type.
// - Calling a function consumes the arguments passed to it.
// - The value returned by the function is linear.

// R1CS Compiler Behavior
// The compiler handles functions through a process of inlining. Function definitions themselves do not generate any R1C S constraints; they are templates. The constraints are generated only when a function is called.

// 1. Argument Mapping: The compiler maps the caller's arguments (x, y, z) to the function's parameters (a, b, c). From this point, within the context of this call, a refers to x, b to y, and c to z.

// 2. Body Inlining: The compiler processes the function's body expression (let product = a * b in product + c) as if it were written directly at the call site.

// 3. Witness and Constraint Generation:
// - The compiler encounters the local binding let product = a * b. Since a is x and b is y, this is x * y.
//- Because this is a multiplication, a new witness variable must be created to hold the result. Let's call this witness w_product.
// - A new R1CS constraint is added to the MainProof's constraint system: (x) * (y) = (w_product).
// - The function's body then evaluates to product + c, which is now w_product + z.

// Return Value Binding:
// - The value w_product + z is the return value of the function call.
// - The caller binds this to the name result.
// - The compiler constrains result to be equal to this linear combination: result = w_product + z. This does not require a new multiplication gate. result itself might be a new witness or an alias depending on the context.

// Final R1CS for MainProof:
// - x (public input)
// - y (public input)
// - z (public input)
// - w_product (private witness)
// - result (witness, becomes the proof output)
// Constraints:
// - x * y = w_product
// - w_product + z = result
```

## Field Operations
Modular Inverse

```rust
 inv(<expr>)

// Example: Compute x / y
proof DivisionExample {
    input x: field;
    input y: field;

    // We assert y != 0 to ensure an inverse exists.
    assert y != 0;

    let y_inv = inv(y) in
    let result = x * y_inv in
    result

    // Type Rules
    // inv consumes its argument and returns a new linear value.
    inv(x: field^{linear}): field^{linear}

    // R1CS Generation
    // `inv(y)` introduces a new witness and a constraint to enforce the inverse relationship.
    let y_inv = inv(y)

    // Variables:
    - y_inv (witness)

    // Constraints:
    - (y) * (y_inv) = (1) // Where 1 is the constant ONE variable
}
 ```

## Exponentiation
```rust
 // Syntax
pow(<base>, <exponent>)

// Example
proof PowExample {
    input base: field;
    input exp: field;
    let result = pow(base, exp) in
    result
}

// Type Rules
// pow consumes both the base and the exponent.
pow(base: field^{linear}, exp: field^{linear}): field^{linear}

// R1CS Generation
// The generation strategy depends on whether the exponent is a constant or a variable.

// Case 1: Exponent is a known constant literal (e.g., `pow(base, 5)`)
// The compiler unrolls this into a chain of multiplications.
// `let result = pow(base, 5)` becomes:
let base_2 = base * base in
let base_4 = base_2 * base_2 in
let result = base_4 * base in

// This is efficient and generates a minimal number of constraints.

// Case 2: Exponent is a variable (witness)
// This requires a more complex circuit using binary decomposition of the exponent.
// It is significantly more expensive in terms of constraints.
let result = pow(base, exp)

// R1CS (Simplified conceptual model):
// Variables:
- exp_bits[252] (witness array, bit decomposition of exp)
- intermediate_powers[252] (witness array)
- result (witness)

// Constraints:
// 1. Decompose exponent:
- exp = sum(exp_bits[i] * 2^i)
- exp_bits[i] * (1 - exp_bits[i]) = 0 // each bit is boolean

// 2. "Square-and-multiply" chain:
- intermediate_powers[0] = exp_bits[0] * (base - 1) + 1
- intermediate_powers[i] = intermediate_powers[i-1] * (exp_bits[i] * (base^(2^i) - 1) + 1)
- result = intermediate_powers[251]

// Note: This is a high-level representation. An actual implementation would
// create witnesses for each step of the square-and-multiply algorithm.
// The high cost should encourage developers to use constant exponents where possible.
```

## Explict Witness Declaration
```rust
// You must declare a witness when a value is non-deterministic from the compiler's view. 
// This is for values that depend on a property of an input, not just a calculation.

// Use witness for:
// Decompositions: The bits of a number.
// Conditional Flags: A boolean that is 1 if a condition is true (e.g., is_zero).
// Computational Hints: The modular inverse of a number, which the prover finds and the circuit verifies.

// Syntax
witness <name>: <type>;

// R1CS Generation
- witness <name>: <type>; // allocates a new, unconstrained private witness variable.
// - Crucially, it is the programmer's responsibility to add assert statements to fully constrain the witness. An unconstrained witness is a security vulnerability.

// Example: is_zero
// It is impossible to write is_zero with only arithmetic. The prover must provide advice, which the circuit then verifies.

let is_zero (x: field): field =
    // The prover supplies two pieces of advice:
    // 1. A boolean flag `is_z` which they claim is 1 if x is zero.
    // 2. A value `x_inv` which they claim is the inverse of x if x is not zero.
    witness is_z: field;
    witness x_inv: field;

    // Constraint 1: The flag must be a boolean.
    let _ = assert is_z * (1 - is_z) == 0 in

    // Constraint 2: If x is not zero, the flag must be 0.
    let _ = assert x * is_z == 0 in

    // Constraint 3: Verifies the prover's claims are consistent.
    // If x != 0, then is_z=0, and this becomes `x * x_inv = 1`.
    // If x == 0, then is_z=1, and this becomes `0 * x_inv = 0`.
    let _ = assert x * x_inv == 1 - is_z in

    // Return the verified boolean flag.
    is_z

```