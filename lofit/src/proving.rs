use ark_bn254::{Bn254, Fr};
use ark_groth16::Proof as ArkProof;
use ark_snark::SNARK;
use ark_groth16::Groth16;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use std::io::{Read, Write};
use crate::circuit::LofCircuit;
use crate::setup::ProverKey;

pub struct Proof {
  pub(crate) proof: ArkProof<Bn254>
}

impl ProverKey {
  pub fn prove(
    &self,
    circuit: LofCircuit<Fr>,
  ) -> Result<Proof, ProverError> {
    let rng = &mut rand::thread_rng();
    
    let proof = <Groth16<Bn254> as SNARK<Fr>>::prove(
      &self.params,
      circuit,
      rng
    ).map_err(|e| ProverError::ProvingFailed(e.to_string()))?;

    Ok(Proof { proof })
  }
}

#[derive(Debug, thiserror::Error)]
pub enum ProverError {
  #[error("Proving failed: {0}")]
  ProvingFailed(String),
}

impl Proof {
  pub fn write<W: Write>(&self, mut writer: W) -> std::io::Result<()> {
    self.proof.serialize_compressed(&mut writer)
      .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
  }

  pub fn read<R: Read>(mut reader: R) -> std::io::Result<Self> {
    let proof = ArkProof::deserialize_compressed(&mut reader)
      .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    Ok(Self { proof })
  }
}