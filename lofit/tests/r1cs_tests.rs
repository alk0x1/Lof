use lofit::r1cs::{Constraint, ConstraintSystem, LinearCombination};
use num_bigint::BigInt;
use std::io::Cursor;

type ConstraintTuple = (Vec<(u32, i64)>, Vec<(u32, i64)>, Vec<(u32, i64)>);

fn create_r1cs_bytes(
    public_inputs: &[&str],
    witnesses: &[&str],
    constraints: &[ConstraintTuple],
) -> Vec<u8> {
    let mut bytes = Vec::new();

    bytes.extend_from_slice(b"lof-r1cs");

    bytes.extend_from_slice(&1u32.to_le_bytes());

    bytes.extend_from_slice(&(public_inputs.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(witnesses.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(constraints.len() as u32).to_le_bytes());

    for name in public_inputs {
        bytes.extend_from_slice(&(name.len() as u32).to_le_bytes());
        bytes.extend_from_slice(name.as_bytes());
    }

    for name in witnesses {
        bytes.extend_from_slice(&(name.len() as u32).to_le_bytes());
        bytes.extend_from_slice(name.as_bytes());
    }

    for (a_terms, b_terms, c_terms) in constraints {
        bytes.extend_from_slice(&(a_terms.len() as u32).to_le_bytes());
        for (var_idx, coeff) in a_terms {
            bytes.extend_from_slice(&var_idx.to_le_bytes());
            let big = BigInt::from(*coeff);
            let coeff_bytes = big.to_signed_bytes_le();
            bytes.extend_from_slice(&(coeff_bytes.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&coeff_bytes);
        }

        bytes.extend_from_slice(&(b_terms.len() as u32).to_le_bytes());
        for (var_idx, coeff) in b_terms {
            bytes.extend_from_slice(&var_idx.to_le_bytes());
            let big = BigInt::from(*coeff);
            let coeff_bytes = big.to_signed_bytes_le();
            bytes.extend_from_slice(&(coeff_bytes.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&coeff_bytes);
        }

        bytes.extend_from_slice(&(c_terms.len() as u32).to_le_bytes());
        for (var_idx, coeff) in c_terms {
            bytes.extend_from_slice(&var_idx.to_le_bytes());
            let big = BigInt::from(*coeff);
            let coeff_bytes = big.to_signed_bytes_le();
            bytes.extend_from_slice(&(coeff_bytes.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&coeff_bytes);
        }
    }

    bytes
}

// ============================================================================
// BASIC DESERIALIZATION TESTS
// ============================================================================

#[test]
fn test_empty_r1cs() {
    let bytes = create_r1cs_bytes(&[], &[], &[]);
    let cursor = Cursor::new(bytes);
    let r1cs = ConstraintSystem::from_file(cursor).unwrap();

    assert_eq!(r1cs.public_inputs.len(), 0);
    assert_eq!(r1cs.witnesses.len(), 0);
    assert_eq!(r1cs.constraints.len(), 0);
}

#[test]
fn test_simple_r1cs() {
    // a * b = c
    let bytes = create_r1cs_bytes(
        &["a", "b"],
        &["c"],
        &[(vec![(1, 1)], vec![(2, 1)], vec![(3, 1)])],
    );

    let cursor = Cursor::new(bytes);
    let r1cs = ConstraintSystem::from_file(cursor).unwrap();

    assert_eq!(r1cs.public_inputs.len(), 2);
    assert_eq!(r1cs.public_inputs[0], "a");
    assert_eq!(r1cs.public_inputs[1], "b");

    assert_eq!(r1cs.witnesses.len(), 1);
    assert_eq!(r1cs.witnesses[0], "c");

    assert_eq!(r1cs.constraints.len(), 1);
    assert_eq!(r1cs.constraints[0].a.terms.len(), 1);
    assert_eq!(r1cs.constraints[0].a.terms[0].0, 1);
    assert_eq!(r1cs.constraints[0].a.terms[0].1, BigInt::from(1));
}

#[test]
fn test_r1cs_with_multiple_constraints() {
    let bytes = create_r1cs_bytes(
        &["x", "y"],
        &["temp", "result"],
        &[
            // temp = x + y: (x + y) * 1 = temp
            (vec![(1, 1), (2, 1)], vec![(0, 1)], vec![(3, 1)]),
            // result = temp * x
            (vec![(3, 1)], vec![(1, 1)], vec![(4, 1)]),
        ],
    );

    let cursor = Cursor::new(bytes);
    let r1cs = ConstraintSystem::from_file(cursor).unwrap();

    assert_eq!(r1cs.public_inputs.len(), 2);
    assert_eq!(r1cs.witnesses.len(), 2);
    assert_eq!(r1cs.constraints.len(), 2);

    // (x + y) * 1 = temp
    assert_eq!(r1cs.constraints[0].a.terms.len(), 2);
    assert_eq!(r1cs.constraints[0].b.terms.len(), 1);
    assert_eq!(r1cs.constraints[0].c.terms.len(), 1);

    // temp * x = result
    assert_eq!(r1cs.constraints[1].a.terms[0].0, 3); // temp
    assert_eq!(r1cs.constraints[1].a.terms[0].1, BigInt::from(1));
    assert_eq!(r1cs.constraints[1].b.terms[0].0, 1); // x
    assert_eq!(r1cs.constraints[1].b.terms[0].1, BigInt::from(1));
    assert_eq!(r1cs.constraints[1].c.terms[0].0, 4); // result
    assert_eq!(r1cs.constraints[1].c.terms[0].1, BigInt::from(1));
}

#[test]
fn test_r1cs_with_negative_coefficients() {
    // a - b = c: (a - b) * 1 = c
    let bytes = create_r1cs_bytes(
        &["a", "b"],
        &["c"],
        &[(vec![(1, 1), (2, -1)], vec![(0, 1)], vec![(3, 1)])],
    );

    let cursor = Cursor::new(bytes);
    let r1cs = ConstraintSystem::from_file(cursor).unwrap();

    assert_eq!(r1cs.constraints[0].a.terms.len(), 2);
    assert_eq!(r1cs.constraints[0].a.terms[0].0, 1);
    assert_eq!(r1cs.constraints[0].a.terms[0].1, BigInt::from(1));
    assert_eq!(r1cs.constraints[0].a.terms[1].0, 2);
    assert_eq!(r1cs.constraints[0].a.terms[1].1, BigInt::from(-1));
}

#[test]
fn test_r1cs_with_larger_coefficients() {
    // 5*a * 3*b = c
    let bytes = create_r1cs_bytes(
        &["a", "b"],
        &["c"],
        &[(vec![(1, 5)], vec![(2, 3)], vec![(3, 1)])],
    );

    let cursor = Cursor::new(bytes);
    let r1cs = ConstraintSystem::from_file(cursor).unwrap();

    assert_eq!(r1cs.constraints[0].a.terms[0].0, 1);
    assert_eq!(r1cs.constraints[0].a.terms[0].1, BigInt::from(5));
    assert_eq!(r1cs.constraints[0].b.terms[0].0, 2);
    assert_eq!(r1cs.constraints[0].b.terms[0].1, BigInt::from(3));
}

// ============================================================================
// VARIABLE NAME TESTS
// ============================================================================

#[test]
fn test_r1cs_with_long_variable_names() {
    let long_name = "very_long_variable_name_that_exceeds_normal_length";
    let bytes = create_r1cs_bytes(
        &[long_name],
        &["result"],
        &[(vec![(1, 1)], vec![(0, 1)], vec![(2, 1)])],
    );

    let cursor = Cursor::new(bytes);
    let r1cs = ConstraintSystem::from_file(cursor).unwrap();

    assert_eq!(r1cs.public_inputs[0], long_name);
}

#[test]
fn test_r1cs_with_unicode_variable_names() {
    let bytes = create_r1cs_bytes(
        &["alpha", "beta"],
        &["gamma"],
        &[(vec![(1, 1)], vec![(2, 1)], vec![(3, 1)])],
    );

    let cursor = Cursor::new(bytes);
    let r1cs = ConstraintSystem::from_file(cursor).unwrap();

    assert_eq!(r1cs.public_inputs[0], "alpha");
    assert_eq!(r1cs.public_inputs[1], "beta");
    assert_eq!(r1cs.witnesses[0], "gamma");
}

// ============================================================================
// ERROR CASE TESTS
// ============================================================================

#[test]
fn test_invalid_magic_bytes() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"invalid!");
    bytes.extend_from_slice(&1u32.to_le_bytes()); // version
    bytes.extend_from_slice(&0u32.to_le_bytes()); // pub inputs
    bytes.extend_from_slice(&0u32.to_le_bytes()); // witnesses
    bytes.extend_from_slice(&0u32.to_le_bytes()); // constraints

    let cursor = Cursor::new(bytes);
    let result = ConstraintSystem::from_file(cursor);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn test_unsupported_version() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"lof-r1cs");
    bytes.extend_from_slice(&999u32.to_le_bytes()); // unsupported version
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());

    let cursor = Cursor::new(bytes);
    let result = ConstraintSystem::from_file(cursor);

    assert!(result.is_err());
}

#[test]
fn test_truncated_file() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"lof-r1cs");
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());

    let cursor = Cursor::new(bytes);
    let result = ConstraintSystem::from_file(cursor);

    assert!(result.is_err());
}

#[test]
fn test_empty_file() {
    let bytes = Vec::new();
    let cursor = Cursor::new(bytes);
    let result = ConstraintSystem::from_file(cursor);

    assert!(result.is_err());
}

// ============================================================================
// COMPLEX CONSTRAINT TESTS
// ============================================================================

#[test]
fn test_r1cs_with_complex_linear_combinations() {
    // a*2 + b*3 + c*(-1) in the A term
    let bytes = create_r1cs_bytes(
        &["a", "b", "c"],
        &["result"],
        &[(vec![(1, 2), (2, 3), (3, -1)], vec![(0, 1)], vec![(4, 1)])],
    );

    let cursor = Cursor::new(bytes);
    let r1cs = ConstraintSystem::from_file(cursor).unwrap();

    assert_eq!(r1cs.constraints[0].a.terms.len(), 3);
    assert_eq!(r1cs.constraints[0].a.terms[0].0, 1);
    assert_eq!(r1cs.constraints[0].a.terms[0].1, BigInt::from(2));
    assert_eq!(r1cs.constraints[0].a.terms[1].0, 2);
    assert_eq!(r1cs.constraints[0].a.terms[1].1, BigInt::from(3));
    assert_eq!(r1cs.constraints[0].a.terms[2].0, 3);
    assert_eq!(r1cs.constraints[0].a.terms[2].1, BigInt::from(-1));
}

#[test]
fn test_r1cs_with_many_constraints() {
    let mut constraints = Vec::new();
    for i in 0..100 {
        constraints.push((vec![(1, 1)], vec![(2, 1)], vec![(3 + i as u32, 1)]));
    }

    let witness_names: Vec<String> = (0..100).map(|i| format!("w{}", i)).collect();
    let witness_refs: Vec<&str> = witness_names.iter().map(|s| s.as_str()).collect();

    let bytes = create_r1cs_bytes(&["a", "b"], &witness_refs, &constraints);

    let cursor = Cursor::new(bytes);
    let r1cs = ConstraintSystem::from_file(cursor).unwrap();

    assert_eq!(r1cs.constraints.len(), 100);
    assert_eq!(r1cs.witnesses.len(), 100);
}

#[test]
fn test_r1cs_only_public_inputs() {
    let bytes = create_r1cs_bytes(
        &["a", "b"],
        &[],
        &[
            // a * 1 = b
            (vec![(1, 1)], vec![(0, 1)], vec![(2, 1)]),
        ],
    );

    let cursor = Cursor::new(bytes);
    let r1cs = ConstraintSystem::from_file(cursor).unwrap();

    assert_eq!(r1cs.public_inputs.len(), 2);
    assert_eq!(r1cs.witnesses.len(), 0);
    assert_eq!(r1cs.constraints.len(), 1);
}

#[test]
fn test_r1cs_with_zero_coefficients() {
    let bytes = create_r1cs_bytes(
        &["a"],
        &["c"],
        &[(vec![(1, 0)], vec![(0, 1)], vec![(2, 1)])],
    );

    let cursor = Cursor::new(bytes);
    let r1cs = ConstraintSystem::from_file(cursor).unwrap();

    assert_eq!(r1cs.constraints[0].a.terms[0].0, 1);
    assert_eq!(r1cs.constraints[0].a.terms[0].1, BigInt::from(0));
}

// ============================================================================
// CLONE AND DEBUG TESTS
// ============================================================================

#[test]
fn test_constraint_system_clone() {
    let bytes = create_r1cs_bytes(
        &["a", "b"],
        &["c"],
        &[(vec![(1, 1)], vec![(2, 1)], vec![(3, 1)])],
    );

    let cursor = Cursor::new(bytes);
    let r1cs = ConstraintSystem::from_file(cursor).unwrap();
    let cloned = r1cs.clone();

    assert_eq!(cloned.public_inputs, r1cs.public_inputs);
    assert_eq!(cloned.witnesses, r1cs.witnesses);
    assert_eq!(cloned.constraints.len(), r1cs.constraints.len());
}

#[test]
fn test_linear_combination_clone() {
    let lc = LinearCombination {
        terms: vec![(1, BigInt::from(2)), (3, BigInt::from(4))],
    };

    let cloned = lc.clone();
    assert_eq!(cloned.terms.len(), lc.terms.len());
    assert_eq!(cloned.terms[0].0, 1);
    assert_eq!(cloned.terms[0].1, BigInt::from(2));
    assert_eq!(cloned.terms[1].0, 3);
    assert_eq!(cloned.terms[1].1, BigInt::from(4));
}

#[test]
fn test_constraint_debug() {
    let constraint = Constraint {
        a: LinearCombination {
            terms: vec![(1, BigInt::from(1))],
        },
        b: LinearCombination {
            terms: vec![(2, BigInt::from(1))],
        },
        c: LinearCombination {
            terms: vec![(3, BigInt::from(1))],
        },
    };

    let debug_str = format!("{:?}", constraint);
    assert!(debug_str.contains("Constraint"));
}
