#![cfg(not(target_arch = "wasm32"))]

use ark_bn254::Fr;
use lofit::r1cs::{Constraint, LinearCombination};
use lofit::{generate_full_witness_with_provided, ConstraintSystem};
use num_bigint::BigInt;
use std::str::FromStr;

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
// WASM STATE ISOLATION TESTS
// ============================================================================

#[test]
fn test_witness_generation_is_stateless() {
    // a * b = c
    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["c".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1)]), // a
            b: lc(vec![(2, 1)]), // b
            c: lc(vec![(3, 1)]), // c
        }],
    );

    let public_inputs = vec![fr(3), fr(4)]; // a=3, b=4
    let provided_witness = vec![]; // c will be computed

    let results: Vec<_> = (0..10)
        .map(|_| {
            generate_full_witness_with_provided(&r1cs, &public_inputs, &provided_witness)
                .expect("Witness generation should succeed")
        })
        .collect();

    let first = &results[0];
    for (i, result) in results.iter().enumerate() {
        assert_eq!(
            result, first,
            "Witness generation {} produced different result than first",
            i
        );
        assert_eq!(result.len(), 1, "Should have computed 1 witness value");
        assert_eq!(result[0], fr(12), "c should equal 3 * 4 = 12");
    }
}

#[test]
fn test_witness_generation_with_different_inputs() {
    let r1cs = create_test_r1cs(
        vec!["x".to_string(), "y".to_string()],
        vec!["z".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1)]), // x
            b: lc(vec![(2, 1)]), // y
            c: lc(vec![(3, 1)]), // z
        }],
    );

    let test_cases = vec![
        (fr(2), fr(3), fr(6)),
        (fr(5), fr(7), fr(35)),
        (fr(10), fr(10), fr(100)),
        (fr(1), fr(1), fr(1)),
        (fr(0), fr(5), fr(0)),
    ];

    for (x, y, expected_z) in test_cases {
        let public_inputs = vec![x, y];
        let provided_witness = vec![];

        let witness = generate_full_witness_with_provided(&r1cs, &public_inputs, &provided_witness)
            .expect("Witness generation should succeed");

        assert_eq!(witness.len(), 1);
        assert_eq!(witness[0], expected_z, "z = x * y");
    }
}

#[test]
fn test_r1cs_clone_independence() {
    let original = create_test_r1cs(
        vec!["input".to_string()],
        vec!["output".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1)]),
            b: lc(vec![(0, 1)]), // ONE
            c: lc(vec![(2, 1)]),
        }],
    );

    let cloned = original.clone();

    assert_eq!(original.public_inputs, cloned.public_inputs);
    assert_eq!(original.witnesses, cloned.witnesses);
    assert_eq!(original.constraints.len(), cloned.constraints.len());

    let public_inputs = vec![fr(42)];
    let provided_witness = vec![];

    let witness1 =
        generate_full_witness_with_provided(&original, &public_inputs, &provided_witness)
            .expect("Original should work");

    let witness2 = generate_full_witness_with_provided(&cloned, &public_inputs, &provided_witness)
        .expect("Clone should work");

    assert_eq!(witness1, witness2, "Both should produce same witness");
}

#[test]
fn test_complex_witness_computation_stability() {
    //  0=ONE, 1=current_year, 2=min_age, 3=birth_year, 4=age, 5=is_adult
    // Constraints:
    //   1. age = current_year - birth_year
    //   2. is_adult = (age >= min_age) ? 1 : 0 (simplified as is_adult * 1 = 1)

    let r1cs = create_test_r1cs(
        vec!["current_year".to_string(), "min_age".to_string()],
        vec![
            "birth_year".to_string(),
            "age".to_string(),
            "is_adult".to_string(),
        ],
        vec![
            // age = current_year - birth_year
            // (current_year - birth_year) * 1 = age
            Constraint {
                a: lc(vec![(1, 1), (3, -1)]), // current_year - birth_year
                b: lc(vec![(0, 1)]),          // ONE
                c: lc(vec![(4, 1)]),          // age
            },
            // is_adult * 1 = 1 (assume adult for test)
            Constraint {
                a: lc(vec![(5, 1)]), // is_adult
                b: lc(vec![(0, 1)]), // ONE
                c: lc(vec![(0, 1)]), // ONE
            },
        ],
    );

    let scenarios = vec![
        (2024, 18, 1990, 34, 1), // Adult
        (2024, 18, 2006, 18, 1), // 18
        (2024, 21, 2000, 24, 1), // Over 21
    ];

    for (current_year, min_age, birth_year, expected_age, expected_is_adult) in scenarios {
        let public_inputs = vec![fr(current_year), fr(min_age)];
        let provided_witness = vec![fr(birth_year)];

        for iteration in 0..5 {
            let witness =
                generate_full_witness_with_provided(&r1cs, &public_inputs, &provided_witness)
                    .unwrap_or_else(|e| {
                        panic!(
                    "Witness generation failed at iteration {} for scenario ({}, {}, {}): {}",
                    iteration, current_year, min_age, birth_year, e
                )
                    });

            assert_eq!(witness.len(), 3, "Should have 3 witness values");
            assert_eq!(witness[0], fr(birth_year), "birth_year should match input");
            assert_eq!(
                witness[1],
                fr(expected_age),
                "age should be computed correctly"
            );
            assert_eq!(
                witness[2],
                fr(expected_is_adult),
                "is_adult should be correct"
            );
        }
    }
}

#[test]
fn test_witness_with_intermediate_variables() {
    // (a + b) * c = result
    // 0=ONE, 1=a, 2=b, 3=c, 4=t_0 (a+b), 5=result
    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
        vec!["t_0".to_string(), "result".to_string()],
        vec![
            // t_0 = a + b
            Constraint {
                a: lc(vec![(1, 1), (2, 1)]), // a + b
                b: lc(vec![(0, 1)]),         // ONE
                c: lc(vec![(4, 1)]),         // t_0
            },
            // result = t_0 * c
            Constraint {
                a: lc(vec![(4, 1)]), // t_0
                b: lc(vec![(3, 1)]), // c
                c: lc(vec![(5, 1)]), // result
            },
        ],
    );

    let test_cases = vec![
        (2, 3, 4, 5, 20),   // (2+3)*4 = 20
        (10, 5, 2, 15, 30), // (10+5)*2 = 30
        (1, 1, 1, 2, 2),    // (1+1)*1 = 2
    ];

    for (a, b, c, expected_t0, expected_result) in test_cases {
        let public_inputs = vec![fr(a), fr(b), fr(c)];
        let provided_witness = vec![];

        for _ in 0..3 {
            let witness =
                generate_full_witness_with_provided(&r1cs, &public_inputs, &provided_witness)
                    .expect("Should compute intermediate variables");

            assert_eq!(witness.len(), 2);
            assert_eq!(witness[0], fr(expected_t0), "t_0 = a + b");
            assert_eq!(witness[1], fr(expected_result), "result = t_0 * c");
        }
    }
}

#[test]
fn test_empty_provided_witness() {
    let r1cs = create_test_r1cs(
        vec!["x".to_string()],
        vec!["y".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 2)]), // 2 * x
            b: lc(vec![(0, 1)]), // ONE
            c: lc(vec![(2, 1)]), // y
        }],
    );

    let public_inputs = vec![fr(5)];
    let provided_witness = vec![];

    let witness = generate_full_witness_with_provided(&r1cs, &public_inputs, &provided_witness)
        .expect("Should compute all witnesses");

    assert_eq!(witness.len(), 1);
    assert_eq!(witness[0], fr(10), "y = 2 * x = 2 * 5 = 10");
}

#[test]
fn test_partial_provided_witness() {
    let r1cs = create_test_r1cs(
        vec!["input".to_string()],
        vec!["w1".to_string(), "w2".to_string(), "w3".to_string()],
        vec![
            // w1 * w2 = w3
            Constraint {
                a: lc(vec![(2, 1)]), // w1
                b: lc(vec![(3, 1)]), // w2
                c: lc(vec![(4, 1)]), // w3
            },
            // input * 1 = w1 (to make input relevant)
            Constraint {
                a: lc(vec![(1, 1)]), // input
                b: lc(vec![(0, 1)]), // ONE
                c: lc(vec![(2, 1)]), // w1
            },
        ],
    );

    let public_inputs = vec![fr(6)];
    let provided_witness = vec![fr(6), fr(7)]; // w1=6, w2=7, w3 to be computed

    let witness = generate_full_witness_with_provided(&r1cs, &public_inputs, &provided_witness)
        .expect("Should compute remaining witnesses");

    assert_eq!(witness.len(), 3);
    assert_eq!(witness[0], fr(6), "w1 provided");
    assert_eq!(witness[1], fr(7), "w2 provided");
    assert_eq!(witness[2], fr(42), "w3 = w1 * w2 = 6 * 7 = 42");
}

// ============================================================================
// EDGE CASES AND ERROR HANDLING
// ============================================================================

#[test]
fn test_zero_value_handling() {
    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["c".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1)]),
            b: lc(vec![(2, 1)]),
            c: lc(vec![(3, 1)]),
        }],
    );

    let test_cases = vec![
        (fr(0), fr(5), fr(0)),
        (fr(5), fr(0), fr(0)),
        (fr(0), fr(0), fr(0)),
    ];

    for (a, b, expected_c) in test_cases {
        let public_inputs = vec![a, b];
        let provided_witness = vec![];

        let witness = generate_full_witness_with_provided(&r1cs, &public_inputs, &provided_witness)
            .expect("Should handle zero values");

        assert_eq!(witness[0], expected_c);
    }
}

#[test]
fn test_large_field_elements() {
    let r1cs = create_test_r1cs(
        vec!["x".to_string()],
        vec!["y".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1)]),
            b: lc(vec![(0, 1)]), // ONE
            c: lc(vec![(2, 1)]),
        }],
    );

    let big_int = num_bigint::BigInt::from_str("123456789012345678901234567890").unwrap();
    let (sign, bytes) = big_int.to_bytes_le();

    let mut limbs = [0u64; 4];
    for (i, chunk) in bytes.chunks(8).enumerate() {
        if i >= 4 {
            break;
        }
        let mut limb_bytes = [0u8; 8];
        limb_bytes[..chunk.len()].copy_from_slice(chunk);
        limbs[i] = u64::from_le_bytes(limb_bytes);
    }

    let ark_bigint = ark_ff::BigInt(limbs);
    let mut large_val = Fr::from(ark_bigint);
    if sign == num_bigint::Sign::Minus {
        large_val = -large_val;
    }

    let public_inputs = vec![large_val];
    let provided_witness = vec![];

    let witness = generate_full_witness_with_provided(&r1cs, &public_inputs, &provided_witness)
        .expect("Should handle large field elements");

    assert_eq!(witness.len(), 1);
    assert_eq!(witness[0], large_val);
}

#[test]
fn test_concurrent_witness_generation_simulation() {
    let r1cs1 = create_test_r1cs(
        vec!["a".to_string()],
        vec!["b".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 2)]), // 2 * a
            b: lc(vec![(0, 1)]),
            c: lc(vec![(2, 1)]), // b
        }],
    );

    let r1cs2 = create_test_r1cs(
        vec!["x".to_string()],
        vec!["y".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 3)]), // 3 * x
            b: lc(vec![(0, 1)]),
            c: lc(vec![(2, 1)]), // y
        }],
    );

    for i in 0..10 {
        let w1 = generate_full_witness_with_provided(&r1cs1, &[fr(i)], &[])
            .expect("Circuit 1 should work");

        let w2 = generate_full_witness_with_provided(&r1cs2, &[fr(i)], &[])
            .expect("Circuit 2 should work");

        assert_eq!(w1[0], fr(2 * i), "Circuit 1: b = 2 * a");
        assert_eq!(w2[0], fr(3 * i), "Circuit 2: y = 3 * x");
    }
}

#[test]
fn test_insufficient_provided_witness_values() {
    // 0=ONE, 1=a (public), 2=b (witness), 3=c (witness computed)
    let r1cs = create_test_r1cs(
        vec!["a".to_string()],
        vec!["b".to_string(), "c".to_string()],
        vec![
            // b = a * 2 (so b can be computed from a)
            Constraint {
                a: lc(vec![(1, 2)]), // 2 * a
                b: lc(vec![(0, 1)]), // ONE
                c: lc(vec![(2, 1)]), // b
            },
            // c = a + b
            Constraint {
                a: lc(vec![(1, 1), (2, 1)]), // a + b
                b: lc(vec![(0, 1)]),         // ONE
                c: lc(vec![(3, 1)]),         // c
            },
        ],
    );

    let public_inputs = vec![fr(5)]; // a = 5
    let provided_witness = vec![];

    let witness = generate_full_witness_with_provided(&r1cs, &public_inputs, &provided_witness)
        .expect("Should compute all witnesses from constraints");

    assert_eq!(witness.len(), 2);
    assert_eq!(witness[0], fr(10), "b = 2 * a = 2 * 5 = 10");
    assert_eq!(witness[1], fr(15), "c = a + b = 5 + 10 = 15");
}

#[test]
fn test_witness_count_mismatch() {
    let r1cs = create_test_r1cs(
        vec!["x".to_string()],
        vec!["y".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1)]),
            b: lc(vec![(0, 1)]),
            c: lc(vec![(2, 1)]),
        }],
    );

    let public_inputs = vec![fr(42)];

    let provided_witness = vec![fr(42), fr(99), fr(123)];

    let witness = generate_full_witness_with_provided(&r1cs, &public_inputs, &provided_witness)
        .expect("Should ignore extra provided witness values");

    assert_eq!(witness.len(), 1);
    assert_eq!(witness[0], fr(42));
}
