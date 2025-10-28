use ark_bn254::{Bn254, Fr};
use ark_ff::PrimeField;
use ark_groth16::{Groth16, ProvingKey, VerifyingKey};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_snark::SNARK;
use ark_std::rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use wasm_bindgen::prelude::*;

use crate::circuit::LofCircuit;
use crate::field::fr_from_str;
use crate::r1cs::ConstraintSystem;
use crate::witness::generate_full_witness_with_provided;

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
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&"[WASM Prover] Starting new prove() call".into());

        let witness: Vec<String> = serde_wasm_bindgen::from_value(witness_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse witness: {}", e)))?;

        let witness_values = parse_field_elements(&witness, "witness")?;

        let num_public = self.r1cs.public_inputs.len();
        let num_user_witnesses = self.r1cs.witnesses.len();
        let expected_total = num_public + num_user_witnesses;

        if witness_values.len() < num_public {
            return Err(JsValue::from_str(&format!(
                "Insufficient witness values: got {}, need at least {} public inputs (R1CS has {} public inputs, {} witnesses)",
                witness_values.len(),
                num_public,
                num_public,
                num_user_witnesses
            )));
        }

        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(
            &format!(
                "[WASM Prover] Input validation: received {} values, R1CS expects {} public + {} user witnesses = {} total (+ intermediate variables computed from constraints)",
                witness_values.len(),
                num_public,
                num_user_witnesses,
                expected_total
            )
            .into(),
        );

        let public_inputs = witness_values[..num_public].to_vec();
        let provided_witness = if witness_values.len() > num_public {
            witness_values[num_public..].to_vec()
        } else {
            vec![]
        };

        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(
            &format!(
                "[WASM Prover] Public inputs: {}, Provided witnesses: {}",
                public_inputs.len(),
                provided_witness.len()
            )
            .into(),
        );

        let r1cs_clone = self.r1cs.clone();

        let full_witness =
            generate_full_witness_with_provided(&r1cs_clone, &public_inputs, &provided_witness)
                .map_err(|e| {
                    #[cfg(target_arch = "wasm32")]
                    web_sys::console::error_1(
                        &format!("[WASM Prover] Witness generation failed: {}", e).into(),
                    );
                    JsValue::from_str(&format!("Failed to build full witness: {}", e))
                })?;

        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(
            &format!(
                "[WASM Prover] Full witness computed: {} values",
                full_witness.len()
            )
            .into(),
        );

        let circuit = LofCircuit {
            public_inputs,
            witness: full_witness,
            constraints: r1cs_clone.constraints,
        };

        let mut rng = thread_rng();
        let proof = Groth16::<Bn254>::prove(&self.proving_key, circuit, &mut rng)
            .map_err(|e| JsValue::from_str(&format!("Failed to generate proof: {}", e)))?;

        let mut proof_bytes = Vec::new();
        proof
            .serialize_compressed(&mut proof_bytes)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize proof: {}", e)))?;

        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(
            &format!("[WASM Prover] Proof generated: {} bytes", proof_bytes.len()).into(),
        );

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

        let verifying_key = VerifyingKey::<Bn254>::deserialize_uncompressed(verifying_key_bytes)
            .or_else(|_| VerifyingKey::<Bn254>::deserialize_compressed(verifying_key_bytes))
            .map_err(|e| {
                JsValue::from_str(&format!("Failed to deserialize verifying key: {}", e))
            })?;

        Ok(WasmVerifier { verifying_key })
    }

    #[wasm_bindgen]
    pub fn verify(&self, proof_bytes: &[u8], public_inputs_json: JsValue) -> Result<bool, JsValue> {
        let public_inputs: Vec<String> = serde_wasm_bindgen::from_value(public_inputs_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse public inputs: {}", e)))?;

        let public_values = parse_field_elements(&public_inputs, "public input")?;

        let proof = ark_groth16::Proof::<Bn254>::deserialize_compressed(proof_bytes)
            .map_err(|e| JsValue::from_str(&format!("Failed to deserialize proof: {}", e)))?;

        let result = Groth16::<Bn254>::verify(&self.verifying_key, &public_values, &proof)
            .map_err(|e| JsValue::from_str(&format!("Verification failed: {}", e)))?;

        Ok(result)
    }
}

fn parse_field_elements(values: &[String], label: &str) -> Result<Vec<Fr>, JsValue> {
    values
        .iter()
        .enumerate()
        .map(|(idx, value)| {
            fr_from_str(value).map_err(|err| {
                JsValue::from_str(&format!("Invalid {label}[{idx}] value '{value}': {err}"))
            })
        })
        .collect()
}
