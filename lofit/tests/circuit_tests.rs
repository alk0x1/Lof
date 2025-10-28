use ark_bn254::Fr;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem as ArkConstraintSystem};
use lofit::r1cs::{Constraint, LinearCombination};
use lofit::LofCircuit;
use num_bigint::BigInt;

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
// BASIC CIRCUIT SYNTHESIS TESTS
// ============================================================================

#[test]
fn test_empty_circuit() {
    let circuit = LofCircuit {
        public_inputs: vec![],
        witness: vec![],
        constraints: vec![],
    };

    let cs = ArkConstraintSystem::<Fr>::new_ref();
    let result = circuit.generate_constraints(cs.clone());

    assert!(result.is_ok());
    assert!(cs.num_instance_variables() >= 1);
}

#[test]
fn test_simple_multiplication_circuit() {
    // a * b = c
    // 0=ONE, 1=a, 2=b, 3=c
    let circuit = LofCircuit {
        public_inputs: vec![fr(5), fr(7)],
        witness: vec![fr(35)],
        constraints: vec![Constraint {
            a: lc(vec![(1, 1)]), // a
            b: lc(vec![(2, 1)]), // b
            c: lc(vec![(3, 1)]), // c
        }],
    };

    let cs = ArkConstraintSystem::<Fr>::new_ref();
    let result = circuit.generate_constraints(cs.clone());

    assert!(result.is_ok());

    assert!(cs.is_satisfied().unwrap());

    assert_eq!(cs.num_instance_variables(), 3);
    assert_eq!(cs.num_witness_variables(), 1);
    assert_eq!(cs.num_constraints(), 1);
}

#[test]
fn test_addition_circuit() {
    // (a + b) * 1 = c
    let circuit = LofCircuit {
        public_inputs: vec![fr(3), fr(4)],
        witness: vec![fr(7)],
        constraints: vec![Constraint {
            a: lc(vec![(1, 1), (2, 1)]), // a + b
            b: lc(vec![(0, 1)]),         // ONE
            c: lc(vec![(3, 1)]),         // c
        }],
    };

    let cs = ArkConstraintSystem::<Fr>::new_ref();
    let result = circuit.generate_constraints(cs.clone());

    assert!(result.is_ok());
    assert!(cs.is_satisfied().unwrap());
}

#[test]
fn test_subtraction_circuit() {
    // (a - b) * 1 = c
    let circuit = LofCircuit {
        public_inputs: vec![fr(10), fr(3)],
        witness: vec![fr(7)],
        constraints: vec![Constraint {
            a: lc(vec![(1, 1), (2, -1)]), // a - b
            b: lc(vec![(0, 1)]),          // ONE
            c: lc(vec![(3, 1)]),          // c
        }],
    };

    let cs = ArkConstraintSystem::<Fr>::new_ref();
    let result = circuit.generate_constraints(cs.clone());

    assert!(result.is_ok());
}

// ============================================================================
// MULTI-CONSTRAINT TESTS
// ============================================================================

#[test]
fn test_multi_constraint_circuit() {
    // temp = a * b, result = temp * a
    // 0=ONE, 1=a, 2=b, 3=temp, 4=result
    let circuit = LofCircuit {
        public_inputs: vec![fr(3), fr(4)],
        witness: vec![fr(12), fr(36)],
        constraints: vec![
            Constraint {
                a: lc(vec![(1, 1)]), // a
                b: lc(vec![(2, 1)]), // b
                c: lc(vec![(3, 1)]), // temp
            },
            Constraint {
                a: lc(vec![(3, 1)]), // temp
                b: lc(vec![(1, 1)]), // a
                c: lc(vec![(4, 1)]), // result
            },
        ],
    };

    let cs = ArkConstraintSystem::<Fr>::new_ref();
    let result = circuit.generate_constraints(cs.clone());

    assert!(result.is_ok());
    assert!(cs.is_satisfied().unwrap());
    assert_eq!(cs.num_constraints(), 2);
}

#[test]
fn test_complex_linear_combinations() {
    // (2*a + 3*b) * (4*c) = result
    let circuit = LofCircuit {
        public_inputs: vec![fr(1), fr(2), fr(3)],
        witness: vec![fr(96)], // (2*1 + 3*2) * (4*3) = 8 * 12 = 96
        constraints: vec![Constraint {
            a: lc(vec![(1, 2), (2, 3)]), // 2*a + 3*b
            b: lc(vec![(3, 4)]),         // 4*c
            c: lc(vec![(4, 1)]),         // result
        }],
    };

    let cs = ArkConstraintSystem::<Fr>::new_ref();
    let result = circuit.generate_constraints(cs.clone());

    assert!(result.is_ok());
    assert!(cs.is_satisfied().unwrap());
}

// ============================================================================
// CONSTRAINT SATISFACTION TESTS (NEGATIVE CASES)
// ============================================================================

#[test]
fn test_unsatisfied_constraint() {
    // a * b should equal c, but we provide wrong c
    let circuit = LofCircuit {
        public_inputs: vec![fr(5), fr(7)],
        witness: vec![fr(999)], // Wrong! Should be 35
        constraints: vec![Constraint {
            a: lc(vec![(1, 1)]), // a
            b: lc(vec![(2, 1)]), // b
            c: lc(vec![(3, 1)]), // c
        }],
    };

    let cs = ArkConstraintSystem::<Fr>::new_ref();
    let result = circuit.generate_constraints(cs.clone());

    assert!(result.is_ok());
    assert!(!cs.is_satisfied().unwrap());
}

#[test]
fn test_unsatisfied_addition() {
    // (a + b) * 1 = c, but with wrong c
    let circuit = LofCircuit {
        public_inputs: vec![fr(3), fr(4)],
        witness: vec![fr(10)], // Wrong! Should be 7
        constraints: vec![Constraint {
            a: lc(vec![(1, 1), (2, 1)]), // a + b
            b: lc(vec![(0, 1)]),         // ONE
            c: lc(vec![(3, 1)]),         // c
        }],
    };

    let cs = ArkConstraintSystem::<Fr>::new_ref();
    let result = circuit.generate_constraints(cs.clone());

    assert!(result.is_ok());
    assert!(!cs.is_satisfied().unwrap());
}

// ============================================================================
// VARIABLE ALLOCATION TESTS
// ============================================================================

#[test]
fn test_variable_ordering() {
    let circuit = LofCircuit {
        public_inputs: vec![fr(1), fr(2), fr(3)],
        witness: vec![fr(4), fr(5)],
        constraints: vec![
            Constraint {
                a: lc(vec![(1, 1)]), // first public input
                b: lc(vec![(0, 1)]), // ONE
                c: lc(vec![(4, 1)]), // first witness (index 4)
            },
            Constraint {
                a: lc(vec![(2, 1)]), // second public input
                b: lc(vec![(0, 1)]), // ONE
                c: lc(vec![(5, 1)]), // second witness (index 5)
            },
        ],
    };

    let cs = ArkConstraintSystem::<Fr>::new_ref();
    let result = circuit.generate_constraints(cs.clone());

    assert!(result.is_ok());

    // ONE + 3 public inputs + 1 arkworks internal = 5 instance variables
    assert_eq!(cs.num_instance_variables(), 4);
    // 2 witnesses are allocated because they're referenced in constraints
    assert_eq!(cs.num_witness_variables(), 2);
}

#[test]
fn test_only_public_inputs_no_witnesses() {
    let circuit = LofCircuit {
        public_inputs: vec![fr(5), fr(5)],
        witness: vec![],
        constraints: vec![
            // a * 1 = b (both public)
            Constraint {
                a: lc(vec![(1, 1)]), // first public input
                b: lc(vec![(0, 1)]), // ONE
                c: lc(vec![(2, 1)]), // second public input
            },
        ],
    };

    let cs = ArkConstraintSystem::<Fr>::new_ref();
    let result = circuit.generate_constraints(cs.clone());

    assert!(result.is_ok());
    assert!(cs.is_satisfied().unwrap());
    assert_eq!(cs.num_witness_variables(), 0);
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[test]
fn test_zero_value_handling() {
    let circuit = LofCircuit {
        public_inputs: vec![fr(0), fr(999)],
        witness: vec![fr(0)],
        constraints: vec![Constraint {
            a: lc(vec![(1, 1)]), // 0
            b: lc(vec![(2, 1)]), // 999
            c: lc(vec![(3, 1)]), // 0
        }],
    };

    let cs = ArkConstraintSystem::<Fr>::new_ref();
    let result = circuit.generate_constraints(cs.clone());

    assert!(result.is_ok());
    assert!(cs.is_satisfied().unwrap());
}

#[test]
fn test_identity_constraint() {
    // a * 1 = a (identity)
    let circuit = LofCircuit {
        public_inputs: vec![fr(42)],
        witness: vec![],
        constraints: vec![Constraint {
            a: lc(vec![(1, 1)]), // a
            b: lc(vec![(0, 1)]), // ONE
            c: lc(vec![(1, 1)]), // a (same variable)
        }],
    };

    let cs = ArkConstraintSystem::<Fr>::new_ref();
    let result = circuit.generate_constraints(cs.clone());

    assert!(result.is_ok());
    assert!(cs.is_satisfied().unwrap());
}

#[test]
fn test_constant_constraint() {
    // 1 * 1 = c, c must be 1
    let circuit = LofCircuit {
        public_inputs: vec![],
        witness: vec![fr(1)],
        constraints: vec![Constraint {
            a: lc(vec![(0, 1)]), // ONE
            b: lc(vec![(0, 1)]), // ONE
            c: lc(vec![(1, 1)]), // c
        }],
    };

    let cs = ArkConstraintSystem::<Fr>::new_ref();
    let result = circuit.generate_constraints(cs.clone());

    assert!(result.is_ok());
    assert!(cs.is_satisfied().unwrap());
}

// ============================================================================
// WITNESS PADDING TESTS
// ============================================================================

#[test]
fn test_insufficient_witness_values() {
    let circuit = LofCircuit {
        public_inputs: vec![fr(1)],
        witness: vec![fr(2)],
        constraints: vec![Constraint {
            a: lc(vec![(1, 1)]), // public input
            b: lc(vec![(2, 1)]), // first witness
            c: lc(vec![(3, 1)]), // second witness (will be padded with 0)
        }],
    };

    let cs = ArkConstraintSystem::<Fr>::new_ref();
    let result = circuit.generate_constraints(cs.clone());

    assert!(result.is_ok());
}

#[test]
fn test_more_witnesses_than_needed() {
    let circuit = LofCircuit {
        public_inputs: vec![fr(2), fr(3)],
        witness: vec![fr(6), fr(999), fr(888)],
        constraints: vec![Constraint {
            a: lc(vec![(1, 1)]), // a
            b: lc(vec![(2, 1)]), // b
            c: lc(vec![(3, 1)]), // first witness only
        }],
    };

    let cs = ArkConstraintSystem::<Fr>::new_ref();
    let result = circuit.generate_constraints(cs.clone());

    assert!(result.is_ok());
    assert!(cs.is_satisfied().unwrap());
}

// ============================================================================
// LARGE CIRCUIT TESTS
// ============================================================================

#[test]
fn test_many_constraints() {
    let mut constraints = Vec::new();
    let mut witnesses = Vec::new();

    for i in 0..50 {
        constraints.push(Constraint {
            a: lc(vec![(1, 1)]),            // a (public input)
            b: lc(vec![(2, 1)]),            // b (public input)
            c: lc(vec![(3 + i as u32, 1)]), // witness i
        });
        witnesses.push(fr(20)); // 4 * 5 = 20
    }

    let circuit = LofCircuit {
        public_inputs: vec![fr(4), fr(5)],
        witness: witnesses,
        constraints,
    };

    let cs = ArkConstraintSystem::<Fr>::new_ref();
    let result = circuit.generate_constraints(cs.clone());

    assert!(result.is_ok());
    assert!(cs.is_satisfied().unwrap());
    assert_eq!(cs.num_constraints(), 50);
}

#[test]
fn test_many_public_inputs() {
    let mut public_inputs = Vec::new();
    for i in 1..=20 {
        public_inputs.push(fr(i));
    }

    let circuit = LofCircuit {
        public_inputs,
        witness: vec![],
        constraints: vec![],
    };

    let cs = ArkConstraintSystem::<Fr>::new_ref();
    let result = circuit.generate_constraints(cs.clone());

    assert!(result.is_ok());
    assert_eq!(cs.num_instance_variables(), 21);
}

// ============================================================================
// REAL-WORLD PATTERN TESTS
// ============================================================================

#[test]
fn test_simple_let_binding_pattern() {
    // let temp = a + b in temp * a
    let circuit = LofCircuit {
        public_inputs: vec![fr(3), fr(4)],
        witness: vec![fr(7), fr(21)],
        constraints: vec![
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
    };

    let cs = ArkConstraintSystem::<Fr>::new_ref();
    let result = circuit.generate_constraints(cs.clone());

    assert!(result.is_ok());
    assert!(cs.is_satisfied().unwrap());
}

#[test]
fn test_square_pattern() {
    // a * a = a_squared
    let circuit = LofCircuit {
        public_inputs: vec![fr(7)],
        witness: vec![fr(49)],
        constraints: vec![Constraint {
            a: lc(vec![(1, 1)]), // a
            b: lc(vec![(1, 1)]), // a
            c: lc(vec![(2, 1)]), // a_squared
        }],
    };

    let cs = ArkConstraintSystem::<Fr>::new_ref();
    let result = circuit.generate_constraints(cs.clone());

    assert!(result.is_ok());
    assert!(cs.is_satisfied().unwrap());
}
