use ark_bn254::{Bn254, Fr};
use ark_groth16::{
  Groth16,
  ProvingKey as ArkProvingKey,
  VerifyingKey as ArkVerifyingKey,
};
use ark_snark::SNARK;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use std::io::{Read, Write};
use crate::circuit::LofCircuit;
use tracing::{info, debug, error, instrument};

#[derive(Debug)]
pub struct ProverKey {
  pub(crate) params: ArkProvingKey<Bn254>
}

#[derive(Debug)]
pub struct VerifierKey {
  pub(crate) vk: ArkVerifyingKey<Bn254>
}

impl ProverKey {
  #[instrument(skip(circuit))]
  pub fn setup(circuit: LofCircuit<Fr>) -> Result<(Self, VerifierKey), Box<dyn std::error::Error>> {
    let rng = &mut rand::thread_rng();
    
    let (params, vk) = Groth16::<Bn254>::circuit_specific_setup(
      circuit,
      rng
    ).map_err(|e| Box::new(e))?;

    Ok((
      Self { params },
      VerifierKey { vk }
    ))
  }

  #[instrument(skip(self, writer))]
  pub fn write<W: Write>(&self, mut writer: W) -> std::io::Result<()> {
    self.params.serialize_compressed(&mut writer)
      .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
  }

  #[instrument(skip(reader))]
  pub fn read<R: Read>(mut reader: R) -> std::io::Result<Self> {
    let params = ArkProvingKey::deserialize_compressed(&mut reader)
      .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    Ok(Self { params })
  }
}

impl VerifierKey {
    #[instrument(skip(self, writer))]
    pub fn write<W: Write>(&self, mut writer: W) -> std::io::Result<()> {
      debug!("Writing verification key");
      self.vk.serialize_uncompressed(&mut writer)
        .map_err(|e| {
          error!("Error writing verification key: {:?}", e);
          std::io::Error::new(std::io::ErrorKind::Other, e)
        })
    }

    #[instrument(skip(reader))]
    pub fn read<R: Read>(mut reader: R) -> std::io::Result<Self> {
      let vk = ArkVerifyingKey::deserialize_uncompressed(&mut reader)
        .map_err(|e| {
          error!("Error reading verification key: {:?}", e);
          std::io::Error::new(std::io::ErrorKind::Other, e)
        })?;
      Ok(Self { vk })
    }
}