use crate::{Proof, VerifierKey};
use ark_bn254::{Bn254, Fr};
use ark_groth16::Groth16;
use ark_snark::SNARK;
use tracing::{debug, error, info, instrument};

#[derive(Debug, thiserror::Error)]
pub enum VerificationError {
    #[error("Verification failed: {0}")]
    Failed(String),
}

impl VerifierKey {
    #[instrument(skip(self, proof, public_inputs))]
    pub fn verify(&self, proof: &Proof, public_inputs: &[Fr]) -> Result<bool, VerificationError> {
        debug!("Number of public inputs: {}", public_inputs.len());
        debug!("Public inputs: {:?}", public_inputs);

        debug!("Calling SNARK::verify with {} inputs", public_inputs.len());
        let result = <Groth16<Bn254> as SNARK<Fr>>::verify(&self.vk, public_inputs, &proof.proof)
            .map_err(|e| {
            error!("SNARK::verify error: {:?}", e);
            VerificationError::Failed(e.to_string())
        })?;

        info!("Verification result: {}", result);
        Ok(result)
    }
}
