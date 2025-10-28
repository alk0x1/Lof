use ark_bn254::Fr;
use lofit::r1cs::{Constraint, LinearCombination};
use lofit::{generate_full_witness, ConstraintSystem};
use num_bigint::BigInt;

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

fn lc(terms: Vec<(u32, i64)>) -> LinearCombination {
    LinearCombination {
        terms: terms
            .into_iter()
            .map(|(var, coeff)| (var, BigInt::from(coeff)))
            .collect(),
    }
}

fn fr(val: u64) -> Fr {
    Fr::from(val)
}

// ============================================================================
// BASIC WITNESS GENERATION TESTS
// ============================================================================

#[test]
fn test_simple_multiplication_witness() {
    // a * b = c
    // a * b = c
    // 0=ONE, 1=a, 2=b, 3=c (witness)

    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["c".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1)]), // a
            b: lc(vec![(2, 1)]), // b
            c: lc(vec![(3, 1)]), // c
        }],
    );

    let pub_inputs = vec![fr(5), fr(7)];
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    assert_eq!(witness.len(), 1);
    assert_eq!(witness[0], fr(35));
}

#[test]
fn test_addition_witness() {
    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["c".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1), (2, 1)]), // a + b
            b: lc(vec![(0, 1)]),         // ONE
            c: lc(vec![(3, 1)]),         // c
        }],
    );

    let pub_inputs = vec![fr(3), fr(4)];
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    assert_eq!(witness.len(), 1);
    assert_eq!(witness[0], fr(7));
}

#[test]
fn test_subtraction_witness() {
    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["c".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1), (2, -1)]), // a - b
            b: lc(vec![(0, 1)]),          // ONE
            c: lc(vec![(3, 1)]),          // c
        }],
    );

    let pub_inputs = vec![fr(10), fr(3)];
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    assert_eq!(witness.len(), 1);
    assert_eq!(witness[0], fr(7));
}

#[test]
fn test_multi_witness_values() {
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

    let pub_inputs = vec![fr(3), fr(4)];
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    assert_eq!(witness.len(), 2);
    assert_eq!(witness[0], fr(12)); // temp
    assert_eq!(witness[1], fr(36)); // result
}

#[test]
fn test_three_level_dependency() {
    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec![
            "step1".to_string(),
            "step2".to_string(),
            "step3".to_string(),
        ],
        vec![
            Constraint {
                a: lc(vec![(1, 1), (2, 1)]), // a + b
                b: lc(vec![(0, 1)]),         // ONE
                c: lc(vec![(3, 1)]),         // step1
            },
            Constraint {
                a: lc(vec![(3, 1)]), // step1
                b: lc(vec![(1, 1)]), // a
                c: lc(vec![(4, 1)]), // step2
            },
            Constraint {
                a: lc(vec![(4, 1), (3, 1)]), // step2 + step1
                b: lc(vec![(0, 1)]),         // ONE
                c: lc(vec![(5, 1)]),         // step3
            },
        ],
    );

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

    let pub_inputs = vec![fr(5), fr(4), fr(10)];
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    assert_eq!(witness.len(), 1);
    assert_eq!(witness[0], fr(10)); // d = 20 - 10
}

#[test]
fn test_solve_with_coefficients() {
    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["c".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1)]), // a
            b: lc(vec![(2, 1)]), // b
            c: lc(vec![(3, 2)]), // 2*c
        }],
    );

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
    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["c".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1)]), // a
            b: lc(vec![(2, 1)]), // b
            c: lc(vec![(3, 1)]), // c
        }],
    );

    let pub_inputs = vec![fr(0), fr(999)];
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    assert_eq!(witness.len(), 1);
    assert_eq!(witness[0], fr(0));
}

#[test]
fn test_identity_operations() {
    let r1cs = create_test_r1cs(
        vec!["a".to_string()],
        vec!["c".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1)]), // a
            b: lc(vec![(0, 1)]), // ONE
            c: lc(vec![(2, 1)]), // c
        }],
    );

    let pub_inputs = vec![fr(42)];
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    assert_eq!(witness.len(), 1);
    assert_eq!(witness[0], fr(42));
}

#[test]
fn test_constant_constraint() {
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

#[test]
fn test_unsolvable_constraint_missing_values() {
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

    let pub_inputs = vec![fr(3), fr(4)];
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    assert_eq!(witness.len(), 2);
    assert_eq!(witness[0], fr(7)); // temp = 3 + 4
    assert_eq!(witness[1], fr(21)); // result = 7 * 3
}

#[test]
fn test_large_values() {
    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["c".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1)]), // a
            b: lc(vec![(2, 1)]), // b
            c: lc(vec![(3, 1)]), // c
        }],
    );

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
    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["temp".to_string(), "result".to_string()],
        vec![
            Constraint {
                a: lc(vec![(1, 1), (2, 1)]), // a + b
                b: lc(vec![(0, 1)]),         // ONE
                c: lc(vec![(3, 1)]),         // temp
            },
            Constraint {
                a: lc(vec![(4, 1)]), // result
                b: lc(vec![(0, 1)]), // ONE
                c: lc(vec![(3, 1)]), // temp
            },
        ],
    );

    let pub_inputs = vec![fr(3), fr(5)];
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    assert_eq!(witness.len(), 2);
    assert_eq!(witness[0], fr(8)); // temp = 3 + 5
    assert_eq!(witness[1], fr(8)); // result = temp
}

#[test]
fn test_comparison_result_pattern() {
    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["comparison_temp".to_string(), "result".to_string()],
        vec![
            Constraint {
                a: lc(vec![(1, 1), (2, -1)]), // a - b
                b: lc(vec![(0, 1)]),          // ONE
                c: lc(vec![(3, 1)]),          // comparison_temp
            },
            Constraint {
                a: lc(vec![(4, 1)]), // result
                b: lc(vec![(0, 1)]), // ONE
                c: lc(vec![(3, 1)]), // comparison_temp
            },
        ],
    );

    let pub_inputs = vec![fr(10), fr(3)];
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    assert_eq!(witness.len(), 2);
    assert_eq!(witness[0], fr(7)); // comparison_temp = 10 - 3
    assert_eq!(witness[1], fr(7)); // result = comparison_temp
}
