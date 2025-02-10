use ark_relations::r1cs::{
  ConstraintSynthesizer, 
  ConstraintSystemRef, 
  SynthesisError,
  LinearCombination as ArkLinearCombination,
};
use ark_ff::Field;
use crate::r1cs::{Constraint, LinearCombination};

pub struct LofCircuit<F: Field> {
  pub public_inputs: Vec<F>,
  pub witness: Vec<F>,
  pub constraints: Vec<Constraint>,
}

impl<F: Field> ConstraintSynthesizer<F> for LofCircuit<F> {
  fn generate_constraints(self, cs: ConstraintSystemRef<F>) -> Result<(), SynthesisError> {
    println!("\nActual values being used:");
    println!("ONE = 1");
    for (i, input) in self.public_inputs.iter().enumerate() {
      println!("Public input {} = {:?}", i, input);
    }
    for (i, wit) in self.witness.iter().enumerate() {
      println!("Witness {} = {:?}", i, wit);
    }
    
    // Allocate variables in the correct order for verification
    let one = cs.new_input_variable(|| Ok(F::from(1u64)))?;
    
    // Allocate public inputs (x, y, z)
    let mut public_vars = Vec::new();
    for input in self.public_inputs.iter() {
      let var = cs.new_input_variable(|| Ok(*input))?;
      public_vars.push(var);
    }
    
    // Allocate witness variables (t)
    let mut witness_vars = Vec::new();
    for wit in self.witness.iter() {
      let var = cs.new_witness_variable(|| Ok(*wit))?;
      witness_vars.push(var);
    }

    // Create variable mapping
    let mut var_map = std::collections::HashMap::new();
    var_map.insert(0u32, one);
    for (i, var) in public_vars.iter().enumerate() {
      var_map.insert((i + 1) as u32, *var);
    }
    // Map witness variables after public inputs
    for (i, var) in witness_vars.iter().enumerate() {
      var_map.insert((i + public_vars.len() + 1) as u32, *var);
    }

    // Add constraints
    for (constraint_idx, constraint) in self.constraints.iter().enumerate() {
      println!("\nProcessing constraint {}", constraint_idx);
      
      let make_lc = |lc: &LinearCombination| {
        let mut ark_lc = ArkLinearCombination::zero();
        for (var_idx, coeff) in &lc.terms {
          let variable = var_map.get(var_idx).ok_or(SynthesisError::Unsatisfiable)?;
          ark_lc = ark_lc + (F::from(*coeff as u64), *variable);
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