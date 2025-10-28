use crate::r1cs::{Constraint, ConstraintSystem};
use ark_bn254::Fr;
use ark_ff::{Field, PrimeField};
use num_bigint::{BigInt, Sign};
use std::collections::HashMap;

fn coeff_to_fr(coeff: &BigInt) -> Fr {
    let (sign, bytes) = coeff.to_bytes_le();

    let mut limbs = [0u64; 4]; // BN254 uses 4 u64 limbs
    for (i, chunk) in bytes.chunks(8).enumerate() {
        if i >= 4 {
            break;
        }
        let mut limb_bytes = [0u8; 8];
        limb_bytes[..chunk.len()].copy_from_slice(chunk);
        limbs[i] = u64::from_le_bytes(limb_bytes);
    }

    let ark_bigint = ark_ff::BigInt(limbs);
    let mut field_elem: Fr = ark_bigint.into();

    if sign == Sign::Minus {
        field_elem = -field_elem;
    }

    field_elem
}

pub fn generate_full_witness(
    r1cs: &ConstraintSystem,
    pub_inputs: &[Fr],
) -> Result<Vec<Fr>, Box<dyn std::error::Error>> {
    generate_full_witness_with_provided(r1cs, pub_inputs, &[])
}

pub fn generate_full_witness_with_provided(
    r1cs: &ConstraintSystem,
    pub_inputs: &[Fr],
    provided_witnesses: &[Fr],
) -> Result<Vec<Fr>, Box<dyn std::error::Error>> {
    let mut values = seed_value_table(pub_inputs, provided_witnesses);

    eprintln!(
        "Starting witness computation with {} public inputs and {} provided witnesses",
        pub_inputs.len(),
        provided_witnesses.len()
    );

    solve_constraints(
        r1cs,
        pub_inputs.len(),
        provided_witnesses.len(),
        &mut values,
    )?;
    extract_witness_values(r1cs, pub_inputs, &values)
}

fn seed_value_table(pub_inputs: &[Fr], provided_witnesses: &[Fr]) -> HashMap<u32, Fr> {
    let mut values = HashMap::new();
    values.insert(0u32, Fr::from(1u64));

    for (i, val) in pub_inputs.iter().enumerate() {
        values.insert((i + 1) as u32, *val);
    }

    let witness_start_index = (pub_inputs.len() + 1) as u32;
    for (i, val) in provided_witnesses.iter().enumerate() {
        values.insert(witness_start_index + i as u32, *val);
    }

    values
}

fn solve_constraints(
    r1cs: &ConstraintSystem,
    num_pub_inputs: usize,
    num_provided_witnesses: usize,
    values: &mut HashMap<u32, Fr>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut changed = true;
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 1000;

    while changed && iterations < MAX_ITERATIONS {
        changed = false;
        iterations += 1;

        let values_before = values.len();

        for constraint in r1cs.constraints.iter() {
            if process_constraint(constraint, values, r1cs) {
                changed = true;
            }
        }

        let solved_this_iteration = values.len() - values_before;
        if iterations <= 5 || solved_this_iteration > 0 {
            eprintln!(
                "Iteration {}: solved {} new variables, {} total variables known",
                iterations,
                solved_this_iteration,
                values.len()
            );
        }
    }

    if iterations == MAX_ITERATIONS {
        return Err("Failed to calculate all witness values: maximum iterations exceeded".into());
    }

    eprintln!(
        "Witness computation finished after {} iterations",
        iterations
    );
    eprintln!(
        "Computed {} variable values (including ONE and public inputs)",
        values.len()
    );
    eprintln!(
        "Summary: {} pub inputs, {} provided witnesses, {} solved variables",
        num_pub_inputs,
        num_provided_witnesses,
        values.len()
    );

    Ok(())
}

fn process_constraint(
    constraint: &Constraint,
    values: &mut HashMap<u32, Fr>,
    r1cs: &ConstraintSystem,
) -> bool {
    if try_solve_bit_decomposition(constraint, values, r1cs) {
        return true;
    }

    if try_solve_is_zero_inverse(constraint, values) {
        return true;
    }

    let a_known = can_evaluate_terms(&constraint.a.terms, values);
    let b_known = can_evaluate_terms(&constraint.b.terms, values);
    let c_known = can_evaluate_terms(&constraint.c.terms, values);

    if a_known && b_known {
        return solve_for_c_terms(constraint, values);
    }

    if b_known && c_known && constraint.a.terms.len() == 1 {
        let (a_var, ref a_coeff) = constraint.a.terms[0];
        return solve_single_unknown(
            (a_var, a_coeff),
            evaluate_terms(&constraint.b.terms, values),
            evaluate_terms(&constraint.c.terms, values),
            values,
        );
    }

    if a_known && c_known && constraint.b.terms.len() == 1 {
        let (b_var, ref b_coeff) = constraint.b.terms[0];
        return solve_single_unknown(
            (b_var, b_coeff),
            evaluate_terms(&constraint.a.terms, values),
            evaluate_terms(&constraint.c.terms, values),
            values,
        );
    }

    false
}

fn solve_for_c_terms(constraint: &Constraint, values: &mut HashMap<u32, Fr>) -> bool {
    let a_val = evaluate_terms(&constraint.a.terms, values);
    let b_val = evaluate_terms(&constraint.b.terms, values);
    let ab_product = a_val * b_val;

    if constraint.c.terms.len() == 1 {
        let (c_var, c_coeff) = &constraint.c.terms[0];
        if values.contains_key(c_var) {
            return false;
        }
        let mut c_val = ab_product;
        if c_coeff != &BigInt::from(1) {
            if let Some(inv_coeff) = coeff_to_fr(c_coeff).inverse() {
                c_val *= inv_coeff;
            } else {
                return false;
            }
        }
        values.insert(*c_var, c_val);
        return true;
    }

    solve_single_unknown_in_c(constraint, values, ab_product)
}

fn solve_single_unknown_in_c(
    constraint: &Constraint,
    values: &mut HashMap<u32, Fr>,
    ab_product: Fr,
) -> bool {
    let unknowns: Vec<_> = constraint
        .c
        .terms
        .iter()
        .filter(|(var, _)| !values.contains_key(var))
        .collect();

    if unknowns.len() != 1 {
        return false;
    }

    let (unknown_var, unknown_coeff) = unknowns[0];

    let mut known_sum = Fr::from(0u64);
    for (c_var, c_coeff) in &constraint.c.terms {
        if let Some(val) = values.get(c_var) {
            known_sum += *val * coeff_to_fr(c_coeff);
        }
    }

    let mut unknown_val = ab_product - known_sum;
    if *unknown_coeff != BigInt::from(1) {
        if let Some(inv_coeff) = coeff_to_fr(unknown_coeff).inverse() {
            unknown_val *= inv_coeff;
        } else {
            return false;
        }
    }

    values.insert(*unknown_var, unknown_val);
    true
}

fn solve_single_unknown(
    (unknown_var, unknown_coeff): (u32, &BigInt),
    known_side: Fr,
    result_side: Fr,
    values: &mut HashMap<u32, Fr>,
) -> bool {
    if values.contains_key(&unknown_var) || known_side == Fr::from(0u64) {
        return false;
    }

    if let Some(inv) = known_side.inverse() {
        let mut solved_val = result_side * inv;
        if unknown_coeff != &BigInt::from(1) {
            if let Some(coeff_inv) = coeff_to_fr(unknown_coeff).inverse() {
                solved_val *= coeff_inv;
            } else {
                return false;
            }
        }
        values.insert(unknown_var, solved_val);
        return true;
    }

    false
}

fn evaluate_terms(terms: &[(u32, BigInt)], values: &HashMap<u32, Fr>) -> Fr {
    let mut total = Fr::from(0u64);
    for (var, coeff) in terms {
        if let Some(val) = values.get(var) {
            total += *val * coeff_to_fr(coeff);
        }
    }
    total
}

fn extract_witness_values(
    r1cs: &ConstraintSystem,
    pub_inputs: &[Fr],
    values: &HashMap<u32, Fr>,
) -> Result<Vec<Fr>, Box<dyn std::error::Error>> {
    let witness_start_idx = (pub_inputs.len() + 1) as u32;
    let expected_witness_count = r1cs.witnesses.len();
    let mut witness = Vec::new();

    for i in 0..expected_witness_count {
        let var_idx = witness_start_idx + i as u32;
        if let Some(val) = values.get(&var_idx) {
            witness.push(*val);
        } else {
            eprintln!(
                "Failed at witness index {}, variable index {}, name '{}'",
                i,
                var_idx,
                r1cs.witnesses
                    .get(i)
                    .map(|s| s.as_str())
                    .unwrap_or("unknown")
            );
            eprintln!(
                "Computed variables: {:?}",
                values.keys().collect::<Vec<_>>()
            );
            return Err(format!(
                "Failed to compute witness variable '{}' (index {})",
                r1cs.witnesses
                    .get(i)
                    .map(|s| s.as_str())
                    .unwrap_or("unknown"),
                var_idx
            )
            .into());
        }
    }

    Ok(witness)
}

fn can_evaluate_terms(terms: &[(u32, BigInt)], values: &HashMap<u32, Fr>) -> bool {
    terms.iter().all(|(var, _)| values.contains_key(var))
}

fn try_solve_bit_decomposition(
    constraint: &Constraint,
    values: &mut HashMap<u32, Fr>,
    _r1cs: &ConstraintSystem,
) -> bool {
    if constraint.b.terms.len() != 1 || constraint.c.terms.len() != 1 {
        return false;
    }

    if constraint.b.terms[0].0 != 0 || constraint.b.terms[0].1 != BigInt::from(1) {
        return false;
    }

    let (c_var, c_coeff) = &constraint.c.terms[0];
    if !values.contains_key(c_var) {
        return false;
    }
    if c_coeff != &BigInt::from(1) {
        return false;
    }

    if constraint.a.terms.len() < 2 {
        return false;
    }

    if constraint.a.terms.len() > 10 && values.contains_key(c_var) {
        eprintln!(
            "Checking potential bit decomposition: {} terms in A, C var {} known",
            constraint.a.terms.len(),
            c_var
        );
    }

    let mut expected_coeff = BigInt::from(1);
    let mut bit_vars = Vec::new();

    for (var, coeff) in &constraint.a.terms {
        if coeff != &expected_coeff {
            if constraint.a.terms.len() > 10 {
                eprintln!(
                    "  Coeff mismatch: expected {}, got {} at bit {}",
                    expected_coeff,
                    coeff,
                    bit_vars.len()
                );
            }
            return false;
        }

        if values.contains_key(var) {
            if constraint.a.terms.len() > 10 {
                eprintln!("  Bit {} already computed, skipping", bit_vars.len());
            }
            return false;
        }

        bit_vars.push(*var);
        expected_coeff *= 2;
    }

    let value_fr = values[c_var];

    let value_bigint = value_fr.into_bigint();

    let mut changed = false;
    for (i, &bit_var) in bit_vars.iter().enumerate() {
        let limb_index = i / 64;
        let bit_in_limb = i % 64;

        let bit_value = if limb_index < value_bigint.0.len() {
            (value_bigint.0[limb_index] >> bit_in_limb) & 1
        } else {
            0
        };

        values.insert(bit_var, Fr::from(bit_value));
        changed = true;
    }

    if changed {
        eprintln!(
            "Solved bit decomposition: {} bits from variable {}",
            bit_vars.len(),
            c_var
        );
        eprintln!(
            "  Limbs: [{}, {}, {}, {}]",
            value_bigint.0[0], value_bigint.0[1], value_bigint.0[2], value_bigint.0[3]
        );
        if bit_vars.len() == 64 {
            eprintln!("  Bit 63 of limb 0 = {}", (value_bigint.0[0] >> 63) & 1);
        }
    }

    changed
}

fn try_solve_is_zero_inverse(constraint: &Constraint, values: &mut HashMap<u32, Fr>) -> bool {
    let a_known = can_evaluate_terms(&constraint.a.terms, values);
    let b_single_unknown =
        constraint.b.terms.len() == 1 && !values.contains_key(&constraint.b.terms[0].0);
    let c_single_unknown =
        constraint.c.terms.len() == 1 && !values.contains_key(&constraint.c.terms[0].0);

    if !a_known || !b_single_unknown || !c_single_unknown {
        return false;
    }

    let mut a_val = Fr::from(0u64);
    for (a_var, a_coeff) in &constraint.a.terms {
        a_val += values[a_var] * coeff_to_fr(a_coeff);
    }

    let (inv_var, inv_coeff) = &constraint.b.terms[0];
    let (product_var, product_coeff) = &constraint.c.terms[0];

    let inv_val = if a_val != Fr::from(0u64) {
        match a_val.inverse() {
            Some(inv) => inv,
            None => return false,
        }
    } else {
        Fr::from(0u64)
    };

    let final_inv_val = if *inv_coeff != BigInt::from(1) {
        if let Some(coeff_inv) = coeff_to_fr(inv_coeff).inverse() {
            inv_val * coeff_inv
        } else {
            return false;
        }
    } else {
        inv_val
    };

    let product_val = a_val * final_inv_val;
    let final_product_val = if *product_coeff != BigInt::from(1) {
        if let Some(coeff_inv) = coeff_to_fr(product_coeff).inverse() {
            product_val * coeff_inv
        } else {
            return false;
        }
    } else {
        product_val
    };

    values.insert(*inv_var, final_inv_val);
    values.insert(*product_var, final_product_val);

    true
}
