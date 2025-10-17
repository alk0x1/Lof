use ark_bn254::Fr;
use lofit::r1cs::{Constraint, LinearCombination};
use lofit::{generate_full_witness, ConstraintSystem};
use num_bigint::BigInt;

/// Helper to create a ConstraintSystem manually for testing
fn create_test_r1cs(
    public_inputs: Vec<String>,
    witnesses: Vec<String>,
    constraints: Vec<Constraint>,
) -> ConstraintSystem {
    ConstraintSystem {
        public_inputs,
        witnesses,
        constraints,
    }
}

/// Helper to create a linear combination from variable indices and coefficients
fn lc(terms: Vec<(u32, i64)>) -> LinearCombination {
    LinearCombination {
        terms: terms
            .into_iter()
            .map(|(var, coeff)| (var, BigInt::from(coeff)))
            .collect(),
    }
}

/// Helper to convert u64 to Fr
fn fr(val: u64) -> Fr {
    Fr::from(val)
}

// ============================================================================
// BASIC WITNESS GENERATION TESTS
// ============================================================================

#[test]
fn test_simple_multiplication_witness() {
    // Circuit: a * b = c
    // Constraint: a * b = c
    // Variables: 0=ONE, 1=a, 2=b, 3=c (witness)

    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["c".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1)]), // a
            b: lc(vec![(2, 1)]), // b
            c: lc(vec![(3, 1)]), // c
        }],
    );

    // Test case: a=5, b=7 -> c should be 35
    let pub_inputs = vec![fr(5), fr(7)];
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    assert_eq!(witness.len(), 1);
    assert_eq!(witness[0], fr(35));
}

#[test]
fn test_addition_witness() {
    // Circuit: a + b = c
    // Constraint: (a + b) * 1 = c
    // Variables: 0=ONE, 1=a, 2=b, 3=c (witness)

    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["c".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1), (2, 1)]), // a + b
            b: lc(vec![(0, 1)]),         // ONE
            c: lc(vec![(3, 1)]),         // c
        }],
    );

    // Test case: a=3, b=4 -> c should be 7
    let pub_inputs = vec![fr(3), fr(4)];
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    assert_eq!(witness.len(), 1);
    assert_eq!(witness[0], fr(7));
}

#[test]
fn test_subtraction_witness() {
    // Circuit: a - b = c
    // For subtraction in field arithmetic: a + (-b) = c
    // But in R1CS: (a - b) * 1 = c
    // Variables: 0=ONE, 1=a, 2=b, 3=c (witness)

    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["c".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1), (2, -1)]), // a - b
            b: lc(vec![(0, 1)]),          // ONE
            c: lc(vec![(3, 1)]),          // c
        }],
    );

    // Test case: a=10, b=3 -> c should be 7
    let pub_inputs = vec![fr(10), fr(3)];
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    assert_eq!(witness.len(), 1);
    assert_eq!(witness[0], fr(7));
}

#[test]
fn test_multi_witness_values() {
    // Circuit: Multiple witnesses computed from inputs
    // x = a + b
    // y = a * b
    // z = a - b
    // Variables: 0=ONE, 1=a, 2=b, 3=x, 4=y, 5=z

    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["x".to_string(), "y".to_string(), "z".to_string()],
        vec![
            // x = a + b
            Constraint {
                a: lc(vec![(1, 1), (2, 1)]), // a + b
                b: lc(vec![(0, 1)]),         // ONE
                c: lc(vec![(3, 1)]),         // x
            },
            // y = a * b
            Constraint {
                a: lc(vec![(1, 1)]), // a
                b: lc(vec![(2, 1)]), // b
                c: lc(vec![(4, 1)]), // y
            },
            // z = a - b
            Constraint {
                a: lc(vec![(1, 1), (2, -1)]), // a - b
                b: lc(vec![(0, 1)]),          // ONE
                c: lc(vec![(5, 1)]),          // z
            },
        ],
    );

    // Test case: a=5, b=3 -> x=8, y=15, z=2
    let pub_inputs = vec![fr(5), fr(3)];
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    assert_eq!(witness.len(), 3);
    assert_eq!(witness[0], fr(8)); // x
    assert_eq!(witness[1], fr(15)); // y
    assert_eq!(witness[2], fr(2)); // z
}

// ============================================================================
// DEPENDENCY CHAIN TESTS
// ============================================================================

#[test]
fn test_dependent_witness_computation() {
    // Circuit: temp = a * b, result = temp * a
    // Variables: 0=ONE, 1=a, 2=b, 3=temp, 4=result

    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["temp".to_string(), "result".to_string()],
        vec![
            // temp = a * b
            Constraint {
                a: lc(vec![(1, 1)]), // a
                b: lc(vec![(2, 1)]), // b
                c: lc(vec![(3, 1)]), // temp
            },
            // result = temp * a
            Constraint {
                a: lc(vec![(3, 1)]), // temp
                b: lc(vec![(1, 1)]), // a
                c: lc(vec![(4, 1)]), // result
            },
        ],
    );

    // Test case: a=3, b=4 -> temp=12, result=36
    let pub_inputs = vec![fr(3), fr(4)];
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    assert_eq!(witness.len(), 2);
    assert_eq!(witness[0], fr(12)); // temp
    assert_eq!(witness[1], fr(36)); // result
}

#[test]
fn test_three_level_dependency() {
    // Circuit: step1 = a + b, step2 = step1 * a, step3 = step2 + step1
    // Variables: 0=ONE, 1=a, 2=b, 3=step1, 4=step2, 5=step3

    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec![
            "step1".to_string(),
            "step2".to_string(),
            "step3".to_string(),
        ],
        vec![
            // step1 = a + b
            Constraint {
                a: lc(vec![(1, 1), (2, 1)]), // a + b
                b: lc(vec![(0, 1)]),         // ONE
                c: lc(vec![(3, 1)]),         // step1
            },
            // step2 = step1 * a
            Constraint {
                a: lc(vec![(3, 1)]), // step1
                b: lc(vec![(1, 1)]), // a
                c: lc(vec![(4, 1)]), // step2
            },
            // step3 = step2 + step1
            Constraint {
                a: lc(vec![(4, 1), (3, 1)]), // step2 + step1
                b: lc(vec![(0, 1)]),         // ONE
                c: lc(vec![(5, 1)]),         // step3
            },
        ],
    );

    // Test case: a=2, b=3 -> step1=5, step2=10, step3=15
    let pub_inputs = vec![fr(2), fr(3)];
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    assert_eq!(witness.len(), 3);
    assert_eq!(witness[0], fr(5)); // step1 = 2 + 3
    assert_eq!(witness[1], fr(10)); // step2 = 5 * 2
    assert_eq!(witness[2], fr(15)); // step3 = 10 + 5
}

// ============================================================================
// CONSTRAINT SOLVING WITH MULTIPLE UNKNOWNS IN C
// ============================================================================

#[test]
fn test_solve_with_multiple_c_terms() {
    // Circuit: a * b = c + d, where c is known and d is witness
    // Variables: 0=ONE, 1=a, 2=b, 3=c (public), 4=d (witness)

    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
        vec!["d".to_string()],
        vec![
            // a * b = c + d, solving for d = (a*b) - c
            Constraint {
                a: lc(vec![(1, 1)]),         // a
                b: lc(vec![(2, 1)]),         // b
                c: lc(vec![(3, 1), (4, 1)]), // c + d
            },
        ],
    );

    // Test case: a=5, b=4, c=10 -> d should be 10 (since 5*4=20, 20-10=10)
    let pub_inputs = vec![fr(5), fr(4), fr(10)];
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    assert_eq!(witness.len(), 1);
    assert_eq!(witness[0], fr(10)); // d = 20 - 10
}

#[test]
fn test_solve_with_coefficients() {
    // Circuit: a * b = 2*c, solving for c
    // Variables: 0=ONE, 1=a, 2=b, 3=c (witness)

    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["c".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1)]), // a
            b: lc(vec![(2, 1)]), // b
            c: lc(vec![(3, 2)]), // 2*c
        }],
    );

    // Test case: a=6, b=4 -> 2*c=24 -> c=12
    let pub_inputs = vec![fr(6), fr(4)];
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    assert_eq!(witness.len(), 1);
    assert_eq!(witness[0], fr(12));
}

// ============================================================================
// EDGE CASES
// ============================================================================

#[test]
fn test_zero_multiplication() {
    // Circuit: a * b = c where one input is zero

    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["c".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1)]), // a
            b: lc(vec![(2, 1)]), // b
            c: lc(vec![(3, 1)]), // c
        }],
    );

    // Test case: a=0, b=999 -> c should be 0
    let pub_inputs = vec![fr(0), fr(999)];
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    assert_eq!(witness.len(), 1);
    assert_eq!(witness[0], fr(0));
}

#[test]
fn test_identity_operations() {
    // Circuit: a * 1 = c
    // Variables: 0=ONE, 1=a, 2=c (witness)

    let r1cs = create_test_r1cs(
        vec!["a".to_string()],
        vec!["c".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1)]), // a
            b: lc(vec![(0, 1)]), // ONE
            c: lc(vec![(2, 1)]), // c
        }],
    );

    // Test case: a=42 -> c should be 42
    let pub_inputs = vec![fr(42)];
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    assert_eq!(witness.len(), 1);
    assert_eq!(witness[0], fr(42));
}

#[test]
fn test_constant_constraint() {
    // Circuit: 1 * 1 = c (c must be 1)
    // Variables: 0=ONE, 1=c (witness)

    let r1cs = create_test_r1cs(
        vec![],
        vec!["c".to_string()],
        vec![Constraint {
            a: lc(vec![(0, 1)]), // ONE
            b: lc(vec![(0, 1)]), // ONE
            c: lc(vec![(1, 1)]), // c
        }],
    );

    let pub_inputs = vec![];
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    assert_eq!(witness.len(), 1);
    assert_eq!(witness[0], fr(1));
}

#[test]
fn test_no_witnesses_needed() {
    // Circuit with only assertions on public inputs, no witnesses to compute
    // Variables: 0=ONE, 1=a, 2=b
    // Constraint: a * 1 = b (assertion that a equals b)

    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec![], // No witnesses
        vec![Constraint {
            a: lc(vec![(1, 1)]), // a
            b: lc(vec![(0, 1)]), // ONE
            c: lc(vec![(2, 1)]), // b
        }],
    );

    let pub_inputs = vec![fr(5), fr(5)];
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    assert_eq!(witness.len(), 0);
}

// ============================================================================
// ERROR CASES
// ============================================================================

// NOTE: Circular dependencies and loops are actually IMPOSSIBLE in Lof
// because the language doesn't support loops or recursion in circuits.
// All witness dependencies form a DAG (Directed Acyclic Graph) by design.
//
// However, we still test error cases that could occur from:
// 1. Malformed R1CS files
// 2. Incorrect witness extraction logic
// 3. Edge cases in constraint solving

#[test]
fn test_unsolvable_constraint_missing_values() {
    // Circuit where witness can't be computed from available values
    // This could happen if R1CS is malformed or incomplete
    // x * y = z, but we don't have x or y values
    // Variables: 0=ONE, 1=x, 2=y, 3=z

    let r1cs = create_test_r1cs(
        vec![],
        vec!["x".to_string(), "y".to_string(), "z".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1)]), // x (unknown)
            b: lc(vec![(2, 1)]), // y (unknown)
            c: lc(vec![(3, 1)]), // z (unknown)
        }],
    );

    let pub_inputs = vec![];
    let result = generate_full_witness(&r1cs, &pub_inputs);

    // This should fail because we can't solve for 3 unknowns from one constraint
    assert!(
        result.is_err(),
        "Should error when witnesses cannot be computed"
    );
}

// ============================================================================
// REAL-WORLD CIRCUIT PATTERNS
// ============================================================================

#[test]
fn test_simple_let_binding_pattern() {
    // Matches the simple_let circuit pattern: let temp = a + b in temp * a
    // Variables: 0=ONE, 1=a, 2=b, 3=temp, 4=result

    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["temp".to_string(), "result".to_string()],
        vec![
            // temp = a + b
            Constraint {
                a: lc(vec![(1, 1), (2, 1)]), // a + b
                b: lc(vec![(0, 1)]),         // ONE
                c: lc(vec![(3, 1)]),         // temp
            },
            // result = temp * a
            Constraint {
                a: lc(vec![(3, 1)]), // temp
                b: lc(vec![(1, 1)]), // a
                c: lc(vec![(4, 1)]), // result
            },
        ],
    );

    // Test case from simple_let_tests.json: a=3, b=4 -> result=21
    let pub_inputs = vec![fr(3), fr(4)];
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    assert_eq!(witness.len(), 2);
    assert_eq!(witness[0], fr(7)); // temp = 3 + 4
    assert_eq!(witness[1], fr(21)); // result = 7 * 3
}

#[test]
fn test_large_values() {
    // Test with larger numbers to ensure field arithmetic works correctly

    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["c".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1)]), // a
            b: lc(vec![(2, 1)]), // b
            c: lc(vec![(3, 1)]), // c
        }],
    );

    // Test case: a=123, b=456 -> c=56088
    let pub_inputs = vec![fr(123), fr(456)];
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    assert_eq!(witness.len(), 1);
    assert_eq!(witness[0], fr(56088));
}

// ============================================================================
// LESS_THAN CIRCUIT PATTERN TEST (Bug Reproduction)
// ============================================================================

#[test]
fn test_witness_from_witness_assignment() {
    // This reproduces the less_than circuit pattern where:
    // 1. A comparison generates a temp witness variable
    // 2. result is constrained to equal that temp variable
    //
    // Simplified circuit: result = temp (where temp is another witness)
    // Constraint: result * 1 = temp
    // Variables: 0=ONE, 1=a, 2=b, 3=temp (computed from some operation), 4=result
    //
    // Full pattern:
    //   temp = a + b    // First constraint computes temp
    //   result = temp   // Second constraint assigns temp to result

    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["temp".to_string(), "result".to_string()],
        vec![
            // First compute temp = a + b
            Constraint {
                a: lc(vec![(1, 1), (2, 1)]), // a + b
                b: lc(vec![(0, 1)]),         // ONE
                c: lc(vec![(3, 1)]),         // temp
            },
            // Then assign result = temp via: result * 1 = temp
            Constraint {
                a: lc(vec![(4, 1)]), // result
                b: lc(vec![(0, 1)]), // ONE
                c: lc(vec![(3, 1)]), // temp
            },
        ],
    );

    // Test case: a=3, b=5 -> temp=8, result=8
    let pub_inputs = vec![fr(3), fr(5)];
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    assert_eq!(witness.len(), 2);
    assert_eq!(witness[0], fr(8)); // temp = 3 + 5
    assert_eq!(witness[1], fr(8)); // result = temp
}

#[test]
fn test_comparison_result_pattern() {
    // This closely mimics the actual less_than circuit pattern:
    // 1. Multiple constraints compute a comparison result (simplified to one here)
    // 2. result witness is constrained to equal the comparison output
    //
    // Simplified:
    //   comparison_temp = (a - b) * 1  // Simplified comparison logic
    //   result = comparison_temp       // Assignment via: result * 1 = comparison_temp
    //
    // Variables: 0=ONE, 1=a, 2=b, 3=comparison_temp, 4=result

    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["comparison_temp".to_string(), "result".to_string()],
        vec![
            // Simplified comparison: comparison_temp = a - b
            Constraint {
                a: lc(vec![(1, 1), (2, -1)]), // a - b
                b: lc(vec![(0, 1)]),          // ONE
                c: lc(vec![(3, 1)]),          // comparison_temp
            },
            // Assign result = comparison_temp
            Constraint {
                a: lc(vec![(4, 1)]), // result
                b: lc(vec![(0, 1)]), // ONE
                c: lc(vec![(3, 1)]), // comparison_temp
            },
        ],
    );

    // Test case: a=10, b=3 -> comparison_temp=7, result=7
    let pub_inputs = vec![fr(10), fr(3)];
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    assert_eq!(witness.len(), 2);
    assert_eq!(witness[0], fr(7)); // comparison_temp = 10 - 3
    assert_eq!(witness[1], fr(7)); // result = comparison_temp
}
