use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use ark_bn254::{Bn254, Fr};
use ark_groth16::{Groth16, ProvingKey, VerifyingKey};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_snark::SNARK;
use ark_std::rand::thread_rng;
use ark_ff::PrimeField;
use num_bigint::BigInt;
use std::io::Cursor;

use crate::circuit::LofCircuit;
use crate::r1cs::ConstraintSystem;

/// Convert string to field element
fn fr_from_str(s: &str) -> Result<Fr, String> {
    let bigint = s.parse::<BigInt>()
        .map_err(|e| format!("Failed to parse as BigInt: {}", e))?;

    let bytes = bigint.to_string().into_bytes();
    Fr::from_le_bytes_mod_order(&bytes);

    // Better approach: use from_str if available
    let num = s.parse::<u64>()
        .map_err(|e| format!("Failed to parse as u64: {}", e))?;
    Ok(Fr::from(num))
}

#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

#[derive(Serialize, Deserialize)]
pub struct WasmProof {
    pub proof_bytes: Vec<u8>,
}

#[wasm_bindgen]
pub struct WasmProver {
    r1cs: ConstraintSystem,
    proving_key: ProvingKey<Bn254>,
}

#[wasm_bindgen]
impl WasmProver {
    #[wasm_bindgen(constructor)]
    pub fn new(r1cs_bytes: &[u8], proving_key_bytes: &[u8]) -> Result<WasmProver, JsValue> {
        init_panic_hook();

        let r1cs = ConstraintSystem::from_file(Cursor::new(r1cs_bytes))
            .map_err(|e| JsValue::from_str(&format!("Failed to deserialize R1CS: {}", e)))?;

        let proving_key = ProvingKey::<Bn254>::deserialize_compressed(proving_key_bytes)
            .map_err(|e| JsValue::from_str(&format!("Failed to deserialize proving key: {}", e)))?;

        Ok(WasmProver { r1cs, proving_key })
    }

    #[wasm_bindgen]
    pub fn prove(&self, witness_json: JsValue) -> Result<Vec<u8>, JsValue> {
        let witness: Vec<String> = serde_wasm_bindgen::from_value(witness_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse witness: {}", e)))?;

        let witness_values: Result<Vec<Fr>, _> = witness
            .iter()
            .map(|s| fr_from_str(s))
            .collect();

        let witness_values = witness_values
            .map_err(|e| JsValue::from_str(&format!("Failed to parse field element: {}", e)))?;

        // Split into public inputs and witness based on R1CS structure
        let num_public = self.r1cs.public_inputs.len();
        let public_inputs = witness_values[..num_public].to_vec();
        let witness_vals = witness_values[num_public..].to_vec();

        let circuit = LofCircuit {
            public_inputs,
            witness: witness_vals,
            constraints: self.r1cs.constraints.clone(),
        };

        let mut rng = thread_rng();
        let proof = Groth16::<Bn254>::prove(&self.proving_key, circuit, &mut rng)
            .map_err(|e| JsValue::from_str(&format!("Failed to generate proof: {}", e)))?;

        let mut proof_bytes = Vec::new();
        proof.serialize_compressed(&mut proof_bytes)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize proof: {}", e)))?;

        Ok(proof_bytes)
    }
}

#[wasm_bindgen]
pub struct WasmVerifier {
    verifying_key: VerifyingKey<Bn254>,
}

#[wasm_bindgen]
impl WasmVerifier {
    #[wasm_bindgen(constructor)]
    pub fn new(verifying_key_bytes: &[u8]) -> Result<WasmVerifier, JsValue> {
        init_panic_hook();

        let verifying_key = VerifyingKey::<Bn254>::deserialize_compressed(verifying_key_bytes)
            .map_err(|e| JsValue::from_str(&format!("Failed to deserialize verifying key: {}", e)))?;

        Ok(WasmVerifier { verifying_key })
    }

    #[wasm_bindgen]
    pub fn verify(&self, proof_bytes: &[u8], public_inputs_json: JsValue) -> Result<bool, JsValue> {
        let public_inputs: Vec<String> = serde_wasm_bindgen::from_value(public_inputs_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse public inputs: {}", e)))?;

        let public_values: Result<Vec<Fr>, _> = public_inputs
            .iter()
            .map(|s| fr_from_str(s))
            .collect();

        let public_values = public_values
            .map_err(|e| JsValue::from_str(&format!("Failed to parse field element: {}", e)))?;

        let proof = ark_groth16::Proof::<Bn254>::deserialize_compressed(proof_bytes)
            .map_err(|e| JsValue::from_str(&format!("Failed to deserialize proof: {}", e)))?;

        let result = Groth16::<Bn254>::verify(&self.verifying_key, &public_values, &proof)
            .map_err(|e| JsValue::from_str(&format!("Verification failed: {}", e)))?;

        Ok(result)
    }
}
