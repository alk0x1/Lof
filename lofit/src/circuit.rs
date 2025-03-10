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
    
    let mut public_vars = Vec::new();
    for input in self.public_inputs.iter() {
      let var = cs.new_input_variable(|| Ok(*input))?;
      public_vars.push(var);
    }
    
    let mut var_map = std::collections::HashMap::new();
    var_map.insert(0u32, one);
    for (i, var) in public_vars.iter().enumerate() {
      var_map.insert((i + 1) as u32, *var);
    }
    
    let max_var_idx = self.constraints.iter()
      .flat_map(|c| {
        c.a.terms.iter()
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
    
    // Add constraints
    for (constraint_idx, constraint) in self.constraints.iter().enumerate() {
      println!("\nProcessing constraint {}", constraint_idx);
      
      let make_lc = |lc: &LinearCombination| {
        let mut ark_lc = ArkLinearCombination::zero();
        for (var_idx, coeff) in &lc.terms {
          let variable = var_map.get(var_idx).ok_or_else(|| {
            println!("Error: Variable {} not found in variable map", var_idx);
            SynthesisError::AssignmentMissing
          })?;
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