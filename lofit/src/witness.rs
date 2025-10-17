use crate::r1cs::{Constraint, ConstraintSystem};
use ark_bn254::Fr;
use ark_ff::{Field, PrimeField};
use num_bigint::{BigInt, Sign};
use std::collections::HashMap;

/// Convert a BigInt coefficient to a field element
/// Handles negative coefficients by converting to field arithmetic
fn coeff_to_fr(coeff: &BigInt) -> Fr {
    let (sign, bytes) = coeff.to_bytes_le();

    // Convert bytes to arkworks BigInteger (for BN254, this is 256 bits = 32 bytes)
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

    // Handle negative values
    if sign == Sign::Minus {
        field_elem = -field_elem;
    }

    field_elem
}

pub fn generate_full_witness(
    r1cs: &ConstraintSystem,
    pub_inputs: &[Fr],
) -> Result<Vec<Fr>, Box<dyn std::error::Error>> {
    let mut values = HashMap::new();

    // add ONE
    values.insert(0u32, Fr::from(1u64));

    for (i, val) in pub_inputs.iter().enumerate() {
        values.insert((i + 1) as u32, *val);
    }

    // calculate all witness values by evaluating constraints
    let mut changed = true;
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 1000; // Prevent infinite loops (increased for complex circuits)

    while changed && iterations < MAX_ITERATIONS {
        changed = false;
        iterations += 1;

        let values_before = values.len();

        for constraint in r1cs.constraints.iter() {
            // Special case: Try bit decomposition first
            if try_solve_bit_decomposition(constraint, &mut values, r1cs) {
                changed = true;
                continue;
            }

            // Special case: Try IsZero pattern (for equality checks)
            if try_solve_is_zero_inverse(constraint, &mut values) {
                changed = true;
                continue;
            }

            // Try to solve for unknowns in A, B, or C
            let a_known = can_evaluate_terms(&constraint.a.terms, &values);
            let b_known = can_evaluate_terms(&constraint.b.terms, &values);
            let c_known = can_evaluate_terms(&constraint.c.terms, &values);

            // Case 1: A and B are known, solve for C
            if a_known && b_known {
                // calculate A * B
                let mut a_val = Fr::from(0u64);
                for (a_var, a_coeff) in &constraint.a.terms {
                    a_val += values[a_var] * coeff_to_fr(a_coeff);
                }

                let mut b_val = Fr::from(0u64);
                for (b_var, b_coeff) in &constraint.b.terms {
                    b_val += values[b_var] * coeff_to_fr(b_coeff);
                }

                let ab_product = a_val * b_val;

                // Try to solve for a single unknown variable in C
                if constraint.c.terms.len() == 1 {
                    let (c_var, c_coeff) = &constraint.c.terms[0];
                    if !values.contains_key(c_var) {
                        let c_val = ab_product;
                        if c_coeff != &BigInt::from(1) {
                            // adjust for coefficient
                            let inv_coeff = coeff_to_fr(c_coeff).inverse().unwrap();
                            values.insert(*c_var, c_val * inv_coeff);
                        } else {
                            values.insert(*c_var, c_val);
                        }

                        changed = true;
                    }
                } else if constraint.c.terms.len() > 1 {
                    // Check if there's exactly one unknown variable in C
                    let unknowns: Vec<_> = constraint
                        .c
                        .terms
                        .iter()
                        .filter(|(var, _)| !values.contains_key(var))
                        .collect();

                    if unknowns.len() == 1 {
                        let (unknown_var, unknown_coeff) = unknowns[0];

                        // Calculate sum of known terms
                        let mut known_sum = Fr::from(0u64);
                        for (c_var, c_coeff) in &constraint.c.terms {
                            if values.contains_key(c_var) {
                                known_sum += values[c_var] * coeff_to_fr(c_coeff);
                            }
                        }

                        // Solve: ab_product = known_sum + unknown_var * unknown_coeff
                        // unknown_var = (ab_product - known_sum) / unknown_coeff
                        let unknown_val = ab_product - known_sum;
                        if *unknown_coeff != BigInt::from(1) {
                            let inv_coeff = coeff_to_fr(unknown_coeff).inverse().unwrap();
                            values.insert(*unknown_var, unknown_val * inv_coeff);
                        } else {
                            values.insert(*unknown_var, unknown_val);
                        }

                        changed = true;
                    }
                }
            }
            // Case 2: B and C are known, solve for A (A = C / B)
            else if b_known && c_known && constraint.a.terms.len() == 1 {
                let (a_var, a_coeff) = &constraint.a.terms[0];
                if !values.contains_key(a_var) {
                    let mut b_val = Fr::from(0u64);
                    for (b_var, b_coeff) in &constraint.b.terms {
                        b_val += values[b_var] * coeff_to_fr(b_coeff);
                    }

                    let mut c_val = Fr::from(0u64);
                    for (c_var, c_coeff) in &constraint.c.terms {
                        c_val += values[c_var] * coeff_to_fr(c_coeff);
                    }

                    // Solve: A * B = C  =>  A = C / B
                    if b_val != Fr::from(0u64) {
                        let a_val = c_val * b_val.inverse().unwrap();
                        if a_coeff != &BigInt::from(1) {
                            let inv_coeff = coeff_to_fr(a_coeff).inverse().unwrap();
                            values.insert(*a_var, a_val * inv_coeff);
                        } else {
                            values.insert(*a_var, a_val);
                        }
                        changed = true;
                    }
                }
            }
            // Case 3: A and C are known, solve for B (B = C / A)
            else if a_known && c_known && constraint.b.terms.len() == 1 {
                let (b_var, b_coeff) = &constraint.b.terms[0];
                if !values.contains_key(b_var) {
                    let mut a_val = Fr::from(0u64);
                    for (a_var, a_coeff) in &constraint.a.terms {
                        a_val += values[a_var] * coeff_to_fr(a_coeff);
                    }

                    let mut c_val = Fr::from(0u64);
                    for (c_var, c_coeff) in &constraint.c.terms {
                        c_val += values[c_var] * coeff_to_fr(c_coeff);
                    }

                    // Solve: A * B = C  =>  B = C / A
                    if a_val != Fr::from(0u64) {
                        let b_val = c_val * a_val.inverse().unwrap();
                        if b_coeff != &BigInt::from(1) {
                            let inv_coeff = coeff_to_fr(b_coeff).inverse().unwrap();
                            values.insert(*b_var, b_val * inv_coeff);
                        } else {
                            values.insert(*b_var, b_val);
                        }
                        changed = true;
                    }
                }
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

    // Extract witness values
    // Witnesses start after ONE and public inputs
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

/// Detect and solve bit decomposition constraints
/// Pattern: (bit0*1 + bit1*2 + bit2*4 + ... + bitN*2^N) * 1 = value
/// Where bits are consecutive witness variables with power-of-2 coefficients
fn try_solve_bit_decomposition(
    constraint: &Constraint,
    values: &mut HashMap<u32, Fr>,
    _r1cs: &ConstraintSystem,
) -> bool {
    // Check if this looks like a bit decomposition constraint:
    // A has multiple terms with power-of-2 coefficients
    // B is ONE
    // C is a single variable that's known

    if constraint.b.terms.len() != 1 || constraint.c.terms.len() != 1 {
        return false;
    }

    // Check B is ONE
    if constraint.b.terms[0].0 != 0 || constraint.b.terms[0].1 != BigInt::from(1) {
        return false;
    }

    // Check C is known
    let (c_var, c_coeff) = &constraint.c.terms[0];
    if !values.contains_key(c_var) {
        // C variable not yet known, skip
        return false;
    }
    if c_coeff != &BigInt::from(1) {
        return false;
    }

    // Check A has multiple terms with power-of-2 coefficients
    if constraint.a.terms.len() < 2 {
        return false;
    }

    // Debug: Print when we find a potential bit decomposition
    if constraint.a.terms.len() > 10 && values.contains_key(c_var) {
        eprintln!(
            "Checking potential bit decomposition: {} terms in A, C var {} known",
            constraint.a.terms.len(),
            c_var
        );
    }

    // Verify coefficients are powers of 2 (1, 2, 4, 8, ...)
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
            return false; // Not a power-of-2 sequence
        }

        // Check if this bit is unknown
        if values.contains_key(var) {
            // Bit already computed, skip this constraint
            if constraint.a.terms.len() > 10 {
                eprintln!("  Bit {} already computed, skipping", bit_vars.len());
            }
            return false;
        }

        bit_vars.push(*var);
        expected_coeff *= 2;
    }

    // Get the value to decompose
    let value_fr = values[c_var];

    // Convert to bits for decomposition
    // BigInt is stored as an array of u64 limbs (4 limbs for BN254 = 256 bits)
    let value_bigint = value_fr.into_bigint();

    // Decompose into bits and insert
    let mut changed = false;
    for (i, &bit_var) in bit_vars.iter().enumerate() {
        // Determine which limb and bit position
        let limb_index = i / 64;
        let bit_in_limb = i % 64;

        // Extract bit value
        let bit_value = if limb_index < value_bigint.0.len() {
            (value_bigint.0[limb_index] >> bit_in_limb) & 1
        } else {
            0 // Bit is beyond the stored limbs
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

/// Detect and solve IsZero inverse pattern used in equality checks
/// Pattern: diff * inv = product, where diff is known
/// This implements the hint: inv = (diff != 0) ? 1/diff : 0
fn try_solve_is_zero_inverse(constraint: &Constraint, values: &mut HashMap<u32, Fr>) -> bool {
    // Pattern: A * B = C
    // Where A is a known difference value (or linear combination)
    // And B is unknown (the inverse)
    // And C is unknown (the product)

    // Check if A is known and B is a single unknown variable
    let a_known = can_evaluate_terms(&constraint.a.terms, values);
    let b_single_unknown =
        constraint.b.terms.len() == 1 && !values.contains_key(&constraint.b.terms[0].0);
    let c_single_unknown =
        constraint.c.terms.len() == 1 && !values.contains_key(&constraint.c.terms[0].0);

    if !a_known || !b_single_unknown || !c_single_unknown {
        return false;
    }

    // Evaluate A
    let mut a_val = Fr::from(0u64);
    for (a_var, a_coeff) in &constraint.a.terms {
        a_val += values[a_var] * coeff_to_fr(a_coeff);
    }

    // Get the inverse variable
    let (inv_var, inv_coeff) = &constraint.b.terms[0];
    let (product_var, product_coeff) = &constraint.c.terms[0];

    // Compute hint: inv = (a != 0) ? 1/a : 0
    let inv_val = if a_val != Fr::from(0u64) {
        a_val.inverse().unwrap()
    } else {
        Fr::from(0u64)
    };

    // Apply coefficient adjustment
    let final_inv_val = if *inv_coeff != BigInt::from(1) {
        inv_val * coeff_to_fr(inv_coeff).inverse().unwrap()
    } else {
        inv_val
    };

    // Compute product = a * inv
    let product_val = a_val * final_inv_val;
    let final_product_val = if *product_coeff != BigInt::from(1) {
        product_val * coeff_to_fr(product_coeff).inverse().unwrap()
    } else {
        product_val
    };

    // Insert values
    values.insert(*inv_var, final_inv_val);
    values.insert(*product_var, final_product_val);

    true
}
