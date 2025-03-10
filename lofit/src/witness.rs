use crate::r1cs::{ConstraintSystem, LinearCombination};
use ark_bn254::Fr;
use ark_ff::Field;
use std::collections::HashMap;

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
	const MAX_ITERATIONS: usize = 100; // Prevent infinite loops
	
	while changed && iterations < MAX_ITERATIONS {
		changed = false;
		iterations += 1;
		
		for constraint in &r1cs.constraints {
			// skip constraints we can't fully evaluate yet
			if !can_evaluate_terms(&constraint.a.terms, &values) || 
				!can_evaluate_terms(&constraint.b.terms, &values) {
				continue;
			}
				
			// only consider constraints where C is a single term we don't know yet
			if constraint.c.terms.len() == 1 {
				let (c_var, c_coeff) = constraint.c.terms[0];
				if !values.contains_key(&c_var) {
					//calculate A * B
					let mut a_val = Fr::from(0u64);
					for (a_var, a_coeff) in &constraint.a.terms {
						a_val += values[a_var] * Fr::from(*a_coeff as u64);
					}
					
					let mut b_val = Fr::from(0u64);
					for (b_var, b_coeff) in &constraint.b.terms {
						b_val += values[b_var] * Fr::from(*b_coeff as u64);
					}
					
					let c_val = a_val * b_val;
					if c_coeff != 1 {
						// adjust for coefficient
						let inv_coeff = Fr::from(c_coeff as u64).inverse().unwrap();
						values.insert(c_var, c_val * inv_coeff);
					} else {
						values.insert(c_var, c_val);
					}
					
					changed = true;
				}
			}
		}
	}
	
	if iterations == MAX_ITERATIONS {
		return Err("Failed to calculate all witness values".into());
	}
	
	// find the maximum variable index
	let max_var = *values.keys().max().unwrap_or(&0);
	
	let mut witness = Vec::new();
	// skip the public inputs (including ONE), start with witnesses
	for i in (pub_inputs.len() + 1) as u32..=max_var {
		if let Some(val) = values.get(&i) {
			witness.push(*val);
		} else {
			return Err(format!("Missing value for variable {}", i).into());
		}
	}
	
	Ok(witness)
}

fn can_evaluate_terms(terms: &[(u32, i64)], values: &HashMap<u32, Fr>) -> bool {
	terms.iter().all(|(var, _)| values.contains_key(var))
}