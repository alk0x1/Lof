use ark_bn254::Fr;
use lofit::r1cs::{Constraint, LinearCombination};
use lofit::{generate_full_witness, ConstraintSystem, LofCircuit, ProverKey, VerifierKey};
use num_bigint::BigInt;
use std::io::Cursor;

/// Helper to create a linear combination
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

/// Helper to create a test ConstraintSystem
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

// ============================================================================
// FULL WORKFLOW TESTS (SETUP -> PROVE -> VERIFY)
// ============================================================================

#[test]
fn test_simple_multiplication_full_workflow() {
    // Circuit: a * b = c
    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["c".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1)]), // a
            b: lc(vec![(2, 1)]), // b
            c: lc(vec![(3, 1)]), // c
        }],
    );

    // Test case: a=5, b=7 -> c=35
    let pub_inputs = vec![fr(5), fr(7)];

    // Step 1: Setup
    let setup_circuit = LofCircuit {
        public_inputs: vec![fr(0); pub_inputs.len()],
        witness: vec![fr(0); 1],
        constraints: r1cs.constraints.clone(),
    };

    let (pk, vk) = ProverKey::setup(setup_circuit).expect("Setup should succeed");

    // Step 2: Generate witness
    let witness =
        generate_full_witness(&r1cs, &pub_inputs).expect("Witness generation should succeed");
    assert_eq!(witness.len(), 1);
    assert_eq!(witness[0], fr(35));

    // Step 3: Prove
    let prove_circuit = LofCircuit {
        public_inputs: pub_inputs.clone(),
        witness: witness.clone(),
        constraints: r1cs.constraints.clone(),
    };

    let proof = pk.prove(prove_circuit).expect("Proving should succeed");

    // Step 4: Verify
    let is_valid = vk
        .verify(&proof, &pub_inputs)
        .expect("Verification should succeed");
    assert!(is_valid, "Proof should be valid");
}

#[test]
fn test_addition_full_workflow() {
    // Circuit: (a + b) * 1 = c
    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["c".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1), (2, 1)]), // a + b
            b: lc(vec![(0, 1)]),         // ONE
            c: lc(vec![(3, 1)]),         // c
        }],
    );

    let pub_inputs = vec![fr(10), fr(20)];

    // Setup
    let setup_circuit = LofCircuit {
        public_inputs: vec![fr(0); pub_inputs.len()],
        witness: vec![fr(0); 1],
        constraints: r1cs.constraints.clone(),
    };
    let (pk, vk) = ProverKey::setup(setup_circuit).unwrap();

    // Generate witness
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();
    assert_eq!(witness[0], fr(30));

    // Prove
    let prove_circuit = LofCircuit {
        public_inputs: pub_inputs.clone(),
        witness,
        constraints: r1cs.constraints.clone(),
    };
    let proof = pk.prove(prove_circuit).unwrap();

    // Verify
    let is_valid = vk.verify(&proof, &pub_inputs).unwrap();
    assert!(is_valid);
}

#[test]
fn test_multi_constraint_full_workflow() {
    // Circuit: temp = a * b, result = temp * a
    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["temp".to_string(), "result".to_string()],
        vec![
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
    );

    let pub_inputs = vec![fr(3), fr(4)];

    // Setup
    let setup_circuit = LofCircuit {
        public_inputs: vec![fr(0); pub_inputs.len()],
        witness: vec![fr(0); 2],
        constraints: r1cs.constraints.clone(),
    };
    let (pk, vk) = ProverKey::setup(setup_circuit).unwrap();

    // Generate witness
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();
    assert_eq!(witness[0], fr(12)); // temp
    assert_eq!(witness[1], fr(36)); // result

    // Prove
    let prove_circuit = LofCircuit {
        public_inputs: pub_inputs.clone(),
        witness,
        constraints: r1cs.constraints.clone(),
    };
    let proof = pk.prove(prove_circuit).unwrap();

    // Verify
    let is_valid = vk.verify(&proof, &pub_inputs).unwrap();
    assert!(is_valid);
}

// ============================================================================
// INVALID PROOF TESTS
// ============================================================================

#[test]
fn test_verification_fails_with_wrong_public_inputs() {
    // Circuit: a * b = c
    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["c".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1)]),
            b: lc(vec![(2, 1)]),
            c: lc(vec![(3, 1)]),
        }],
    );

    let pub_inputs = vec![fr(5), fr(7)];

    // Setup
    let setup_circuit = LofCircuit {
        public_inputs: vec![fr(0); pub_inputs.len()],
        witness: vec![fr(0); 1],
        constraints: r1cs.constraints.clone(),
    };
    let (pk, vk) = ProverKey::setup(setup_circuit).unwrap();

    // Generate witness
    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();

    // Prove
    let prove_circuit = LofCircuit {
        public_inputs: pub_inputs.clone(),
        witness,
        constraints: r1cs.constraints.clone(),
    };
    let proof = pk.prove(prove_circuit).unwrap();

    // Verify with WRONG public inputs
    let wrong_pub_inputs = vec![fr(99), fr(88)];
    let is_valid = vk.verify(&proof, &wrong_pub_inputs).unwrap();
    assert!(
        !is_valid,
        "Proof should be invalid with wrong public inputs"
    );
}

#[test]
#[should_panic(expected = "assertion failed")]
fn test_verification_fails_with_wrong_witness() {
    // Circuit: a * b = c
    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["c".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1)]),
            b: lc(vec![(2, 1)]),
            c: lc(vec![(3, 1)]),
        }],
    );

    let pub_inputs = vec![fr(5), fr(7)];

    // Setup
    let setup_circuit = LofCircuit {
        public_inputs: vec![fr(0); pub_inputs.len()],
        witness: vec![fr(0); 1],
        constraints: r1cs.constraints.clone(),
    };
    let (pk, _vk) = ProverKey::setup(setup_circuit).unwrap();

    // Use WRONG witness (should be 35, but we use 999)
    let wrong_witness = vec![fr(999)];

    // Prove with wrong witness - this will panic because the circuit is not satisfied
    let prove_circuit = LofCircuit {
        public_inputs: pub_inputs.clone(),
        witness: wrong_witness,
        constraints: r1cs.constraints.clone(),
    };

    // This should panic during proving because the circuit constraints aren't satisfied
    let _proof = pk.prove(prove_circuit).unwrap();
}

// ============================================================================
// KEY SERIALIZATION TESTS
// ============================================================================

#[test]
fn test_proving_key_serialization() {
    let r1cs = create_test_r1cs(
        vec!["a".to_string()],
        vec!["c".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1)]),
            b: lc(vec![(0, 1)]),
            c: lc(vec![(2, 1)]),
        }],
    );

    let setup_circuit = LofCircuit {
        public_inputs: vec![fr(0)],
        witness: vec![fr(0)],
        constraints: r1cs.constraints.clone(),
    };

    let (pk, _vk) = ProverKey::setup(setup_circuit).unwrap();

    // Serialize
    let mut buffer = Vec::new();
    pk.write(&mut buffer).expect("Should serialize proving key");

    // Deserialize
    let cursor = Cursor::new(buffer);
    let pk2 = ProverKey::read(cursor).expect("Should deserialize proving key");

    // Use the deserialized key to prove
    let pub_inputs = vec![fr(42)];
    let witness = vec![fr(42)];
    let prove_circuit = LofCircuit {
        public_inputs: pub_inputs.clone(),
        witness,
        constraints: r1cs.constraints.clone(),
    };

    let _proof = pk2
        .prove(prove_circuit)
        .expect("Should prove with deserialized key");

    // Proof creation succeeded if we got here
}

#[test]
fn test_verification_key_serialization() {
    let r1cs = create_test_r1cs(
        vec!["a".to_string()],
        vec!["c".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1)]),
            b: lc(vec![(0, 1)]),
            c: lc(vec![(2, 1)]),
        }],
    );

    let setup_circuit = LofCircuit {
        public_inputs: vec![fr(0)],
        witness: vec![fr(0)],
        constraints: r1cs.constraints.clone(),
    };

    let (pk, vk) = ProverKey::setup(setup_circuit).unwrap();

    // Serialize verification key
    let mut buffer = Vec::new();
    vk.write(&mut buffer)
        .expect("Should serialize verification key");

    // Deserialize
    let cursor = Cursor::new(buffer);
    let vk2 = VerifierKey::read(cursor).expect("Should deserialize verification key");

    // Create a proof and verify with deserialized key
    let pub_inputs = vec![fr(42)];
    let witness = vec![fr(42)];
    let prove_circuit = LofCircuit {
        public_inputs: pub_inputs.clone(),
        witness,
        constraints: r1cs.constraints.clone(),
    };

    let proof = pk.prove(prove_circuit).unwrap();
    let is_valid = vk2.verify(&proof, &pub_inputs).unwrap();
    assert!(is_valid);
}

#[test]
fn test_proof_serialization() {
    let r1cs = create_test_r1cs(
        vec!["a".to_string()],
        vec!["c".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1)]),
            b: lc(vec![(0, 1)]),
            c: lc(vec![(2, 1)]),
        }],
    );

    let setup_circuit = LofCircuit {
        public_inputs: vec![fr(0)],
        witness: vec![fr(0)],
        constraints: r1cs.constraints.clone(),
    };

    let (pk, vk) = ProverKey::setup(setup_circuit).unwrap();

    let pub_inputs = vec![fr(42)];
    let witness = vec![fr(42)];
    let prove_circuit = LofCircuit {
        public_inputs: pub_inputs.clone(),
        witness,
        constraints: r1cs.constraints.clone(),
    };

    let proof = pk.prove(prove_circuit).unwrap();

    // Serialize proof
    let mut buffer = Vec::new();
    proof.write(&mut buffer).expect("Should serialize proof");

    // Deserialize proof
    let cursor = Cursor::new(buffer);
    let proof2 = lofit::Proof::read(cursor).expect("Should deserialize proof");

    // Verify with deserialized proof
    let is_valid = vk.verify(&proof2, &pub_inputs).unwrap();
    assert!(is_valid);
}

// ============================================================================
// EDGE CASES
// ============================================================================

#[test]
fn test_circuit_with_zero_values() {
    // Circuit where inputs can be zero
    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["c".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1)]),
            b: lc(vec![(2, 1)]),
            c: lc(vec![(3, 1)]),
        }],
    );

    let pub_inputs = vec![fr(0), fr(999)];

    let setup_circuit = LofCircuit {
        public_inputs: vec![fr(0); pub_inputs.len()],
        witness: vec![fr(0); 1],
        constraints: r1cs.constraints.clone(),
    };
    let (pk, vk) = ProverKey::setup(setup_circuit).unwrap();

    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();
    assert_eq!(witness[0], fr(0));

    let prove_circuit = LofCircuit {
        public_inputs: pub_inputs.clone(),
        witness,
        constraints: r1cs.constraints.clone(),
    };
    let proof = pk.prove(prove_circuit).unwrap();

    let is_valid = vk.verify(&proof, &pub_inputs).unwrap();
    assert!(is_valid);
}

#[test]
fn test_circuit_with_no_witnesses() {
    // Circuit with only public inputs, no witnesses
    // Just an assertion: a * 1 = b
    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec![],
        vec![Constraint {
            a: lc(vec![(1, 1)]),
            b: lc(vec![(0, 1)]),
            c: lc(vec![(2, 1)]),
        }],
    );

    let pub_inputs = vec![fr(7), fr(7)];

    let setup_circuit = LofCircuit {
        public_inputs: vec![fr(0); pub_inputs.len()],
        witness: vec![],
        constraints: r1cs.constraints.clone(),
    };
    let (pk, vk) = ProverKey::setup(setup_circuit).unwrap();

    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();
    assert_eq!(witness.len(), 0);

    let prove_circuit = LofCircuit {
        public_inputs: pub_inputs.clone(),
        witness,
        constraints: r1cs.constraints.clone(),
    };
    let proof = pk.prove(prove_circuit).unwrap();

    let is_valid = vk.verify(&proof, &pub_inputs).unwrap();
    assert!(is_valid);
}

// ============================================================================
// REALISTIC CIRCUIT TESTS
// ============================================================================

#[test]
fn test_simple_let_binding_full_workflow() {
    // Realistic circuit pattern: let temp = a + b in temp * a
    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["temp".to_string(), "result".to_string()],
        vec![
            Constraint {
                a: lc(vec![(1, 1), (2, 1)]),
                b: lc(vec![(0, 1)]),
                c: lc(vec![(3, 1)]),
            },
            Constraint {
                a: lc(vec![(3, 1)]),
                b: lc(vec![(1, 1)]),
                c: lc(vec![(4, 1)]),
            },
        ],
    );

    let pub_inputs = vec![fr(3), fr(4)];

    let setup_circuit = LofCircuit {
        public_inputs: vec![fr(0); pub_inputs.len()],
        witness: vec![fr(0); 2],
        constraints: r1cs.constraints.clone(),
    };
    let (pk, vk) = ProverKey::setup(setup_circuit).unwrap();

    let witness = generate_full_witness(&r1cs, &pub_inputs).unwrap();
    assert_eq!(witness[0], fr(7)); // temp = 3 + 4
    assert_eq!(witness[1], fr(21)); // result = 7 * 3

    let prove_circuit = LofCircuit {
        public_inputs: pub_inputs.clone(),
        witness,
        constraints: r1cs.constraints.clone(),
    };
    let proof = pk.prove(prove_circuit).unwrap();

    let is_valid = vk.verify(&proof, &pub_inputs).unwrap();
    assert!(is_valid);
}

#[test]
fn test_multiple_independent_proofs_same_keys() {
    // Test that the same keys can be used for multiple proofs with different inputs
    let r1cs = create_test_r1cs(
        vec!["a".to_string(), "b".to_string()],
        vec!["c".to_string()],
        vec![Constraint {
            a: lc(vec![(1, 1)]),
            b: lc(vec![(2, 1)]),
            c: lc(vec![(3, 1)]),
        }],
    );

    // Setup once
    let setup_circuit = LofCircuit {
        public_inputs: vec![fr(0), fr(0)],
        witness: vec![fr(0)],
        constraints: r1cs.constraints.clone(),
    };
    let (pk, vk) = ProverKey::setup(setup_circuit).unwrap();

    // Proof 1: 5 * 7 = 35
    let pub_inputs1 = vec![fr(5), fr(7)];
    let witness1 = generate_full_witness(&r1cs, &pub_inputs1).unwrap();
    let prove_circuit1 = LofCircuit {
        public_inputs: pub_inputs1.clone(),
        witness: witness1,
        constraints: r1cs.constraints.clone(),
    };
    let proof1 = pk.prove(prove_circuit1).unwrap();
    assert!(vk.verify(&proof1, &pub_inputs1).unwrap());

    // Proof 2: 3 * 11 = 33
    let pub_inputs2 = vec![fr(3), fr(11)];
    let witness2 = generate_full_witness(&r1cs, &pub_inputs2).unwrap();
    let prove_circuit2 = LofCircuit {
        public_inputs: pub_inputs2.clone(),
        witness: witness2,
        constraints: r1cs.constraints.clone(),
    };
    let proof2 = pk.prove(prove_circuit2).unwrap();
    assert!(vk.verify(&proof2, &pub_inputs2).unwrap());

    // Verify that proof1 doesn't validate with pub_inputs2
    assert!(!vk.verify(&proof1, &pub_inputs2).unwrap());
}
