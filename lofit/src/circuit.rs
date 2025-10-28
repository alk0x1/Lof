use crate::r1cs::{Constraint, LinearCombination};
use ark_ff::{Field, PrimeField};
use ark_relations::r1cs::{
    ConstraintSynthesizer, ConstraintSystemRef, LinearCombination as ArkLinearCombination,
    SynthesisError, Variable,
};
use ark_serialize::CanonicalDeserialize;
use num_bigint::BigInt;

pub struct LofCircuit<F: Field> {
    pub public_inputs: Vec<F>,
    pub witness: Vec<F>,
    pub constraints: Vec<Constraint>,
}

impl<F: PrimeField> ConstraintSynthesizer<F> for LofCircuit<F> {
    fn generate_constraints(self, cs: ConstraintSystemRef<F>) -> Result<(), SynthesisError> {
        println!("\nActual values being used:");
        println!("ONE = 1");
        for (i, input) in self.public_inputs.iter().enumerate() {
            println!("Public input {} = {:?}", i, input);
        }
        for (i, wit) in self.witness.iter().enumerate() {
            println!("Witness {} = {:?}", i, wit);
        }

        let mut public_vars = Vec::new();
        for input in self.public_inputs.iter() {
            let var = cs.new_input_variable(|| Ok(*input))?;
            public_vars.push(var);
        }

        let mut var_map = std::collections::HashMap::new();
        var_map.insert(0u32, Variable::One);
        for (i, var) in public_vars.iter().enumerate() {
            var_map.insert((i + 1) as u32, *var);
        }

        let max_var_idx = self
            .constraints
            .iter()
            .flat_map(|c| {
                c.a.terms
                    .iter()
                    .chain(c.b.terms.iter())
                    .chain(c.c.terms.iter())
                    .map(|(idx, _)| *idx)
            })
            .max()
            .unwrap_or(public_vars.len() as u32);

        let num_witness_needed = (max_var_idx as usize).saturating_sub(public_vars.len());

        let mut witness_vars = Vec::new();
        for i in 0..num_witness_needed {
            let witness_value = if i < self.witness.len() {
                self.witness[i]
            } else {
                println!("Using default value 0 for witness {}", i);
                F::from(0u64)
            };

            let var = cs.new_witness_variable(|| Ok(witness_value))?;
            witness_vars.push(var);
            var_map.insert((i + public_vars.len() + 1) as u32, var);
        }

        for (constraint_idx, constraint) in self.constraints.iter().enumerate() {
            println!("\nProcessing constraint {}", constraint_idx);

            let make_lc = |lc: &LinearCombination| {
                let mut ark_lc = ArkLinearCombination::zero();
                for (var_idx, coeff) in &lc.terms {
                    let variable = var_map.get(var_idx).ok_or_else(|| {
                        println!("Error: Variable {} not found in variable map", var_idx);
                        SynthesisError::AssignmentMissing
                    })?;

                    let field_coeff = bigint_to_field::<F>(coeff)?;
                    ark_lc += (field_coeff, *variable);
                }
                Ok(ark_lc)
            };

            let a_lc = make_lc(&constraint.a)?;
            let b_lc = make_lc(&constraint.b)?;
            let c_lc = make_lc(&constraint.c)?;

            cs.enforce_constraint(a_lc, b_lc, c_lc)?;
            println!("Enforced constraint {}", constraint_idx);
        }

        Ok(())
    }
}

fn bigint_to_field<F: PrimeField>(big: &BigInt) -> Result<F, SynthesisError> {
    use num_bigint::Sign;

    let (sign, bytes) = big.to_bytes_le();

    let mut limbs = [0u64; 4];
    for (i, chunk) in bytes.chunks(8).enumerate() {
        if i >= 4 {
            break;
        }
        let mut limb_bytes = [0u8; 8];
        limb_bytes[..chunk.len()].copy_from_slice(chunk);
        limbs[i] = u64::from_le_bytes(limb_bytes);
    }

    let mut all_bytes = [0u8; 32];
    for (i, &limb) in limbs.iter().enumerate() {
        let limb_bytes = limb.to_le_bytes();
        all_bytes[i * 8..(i + 1) * 8].copy_from_slice(&limb_bytes);
    }
    let ark_bigint = F::BigInt::deserialize_uncompressed(&all_bytes[..])
        .map_err(|_| SynthesisError::AssignmentMissing)?;

    let mut field_elem = F::from_bigint(ark_bigint).ok_or(SynthesisError::AssignmentMissing)?;

    if sign == Sign::Minus {
        field_elem = -field_elem;
    }

    Ok(field_elem)
}
