use crate::{Proof, VerifierKey};
use ark_bn254::{Bn254, Fr};
use ark_groth16::Groth16;
use ark_snark::SNARK;

#[derive(Debug, thiserror::Error)]
pub enum VerificationError {
  #[error("Verification failed: {0}")]
  Failed(String),
}

impl VerifierKey {
  pub fn verify(
    &self,
    proof: &Proof,
    public_inputs: &[Fr],
  ) -> Result<bool, VerificationError> {
    println!("\nVerification details:");
    println!("Number of public inputs: {}", public_inputs.len());
    println!("Public inputs: {:?}", public_inputs);
    
    // Create full input vector with just ONE and public inputs
    let mut full_inputs = vec![Fr::from(1u64)];  // Add ONE constant
    full_inputs.extend(public_inputs.iter().cloned());

    println!("Calling SNARK::verify with {} inputs", full_inputs.len());
    let result = <Groth16<Bn254> as SNARK<Fr>>::verify(
      &self.vk,
      &full_inputs,
      &proof.proof
    ).map_err(|e| {
      println!("SNARK::verify error: {:?}", e);
      VerificationError::Failed(e.to_string())
    })?;

    println!("Verification result: {}", result);
    Ok(result)
  }
}