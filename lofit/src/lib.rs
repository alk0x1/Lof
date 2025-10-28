pub mod circuit;
pub mod field;
#[cfg(not(target_arch = "wasm32"))]
pub mod package_web;
pub mod proving;
pub mod r1cs;
pub mod setup;
pub mod verification;
pub mod witness;

pub use circuit::LofCircuit;
pub use field::{fr_from_str, FieldElementParseError};
#[cfg(not(target_arch = "wasm32"))]
pub use package_web::package_for_web;
pub use proving::Proof;
pub use r1cs::ConstraintSystem;
pub use setup::{ProverKey, VerifierKey};
pub use witness::{generate_full_witness, generate_full_witness_with_provided};

#[cfg(target_arch = "wasm32")]
pub mod wasm;
