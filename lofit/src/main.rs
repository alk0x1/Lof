use ark_bn254::Fr;
use clap::{ArgAction, Parser, Subcommand};
use indexmap::IndexMap;
use lofit::{
    field::{fr_from_str, FieldElementParseError},
    generate_full_witness, generate_full_witness_with_provided, package_for_web, ConstraintSystem,
    LofCircuit, Proof, ProverKey, VerifierKey,
};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{debug, error, info, instrument, warn};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "lofit")]
#[command(version = VERSION)]
#[command(about = "Lof ZK Toolkit - Handles proving and verification for Lof circuits")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Setup {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short = 'p', long)]
        proving_key: Option<PathBuf>,
        #[arg(short = 'v', long)]
        verification_key: Option<PathBuf>,
    },
    Prove {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short = 'p', long)]
        proving_key: Option<PathBuf>,
        #[arg(short = 'u', long = "public-inputs")]
        public_inputs: Option<PathBuf>,
        #[arg(short, long)]
        witness: Option<PathBuf>,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    Verify {
        #[arg(short = 'v', long)]
        verification_key: Option<PathBuf>,
        #[arg(short, long)]
        proof: Option<PathBuf>,
        #[arg(short = 'u', long = "public-inputs")]
        public_inputs: Option<PathBuf>,
        #[arg(short, long, help = "R1CS input file (used to determine base name)")]
        input: Option<PathBuf>,
    },
    PackageWeb {
        #[arg(short, long, help = "R1CS input file")]
        input: PathBuf,
        #[arg(short, long, help = "Output directory for web package")]
        output: Option<PathBuf>,
        #[arg(long, action = ArgAction::SetTrue, help = "Skip building WASM artifacts (generate sources only)")]
        skip_wasm: bool,
    },
    Version,
}

type InputsJson = HashMap<String, String>;

#[derive(Debug, Error)]
enum InputReadError {
    #[error("missing variable '{name}' in JSON file")]
    MissingVariable { name: String },
    #[error("failed to parse value for '{name}': {source}")]
    InvalidField {
        name: String,
        #[source]
        source: FieldElementParseError,
    },
}

#[instrument(level = "debug", skip(json_map))]
fn parse_inputs_in_order(
    json_map: &InputsJson,
    variable_names: &[String],
) -> Result<Vec<Fr>, InputReadError> {
    let mut values = Vec::new();
    for name in variable_names {
        let value_str = json_map
            .get(name)
            .ok_or_else(|| InputReadError::MissingVariable { name: name.clone() })?;

        let value = fr_from_str(value_str).map_err(|source| InputReadError::InvalidField {
            name: name.clone(),
            source,
        })?;

        values.push(value);
    }
    debug!("Parsed {} input values", values.len());
    Ok(values)
}

fn parse_partial_witness(json_map: &InputsJson, variable_names: &[String]) -> Vec<Fr> {
    let mut values = Vec::new();
    for name in variable_names {
        if let Some(value_str) = json_map.get(name) {
            match fr_from_str(value_str) {
                Ok(val) => values.push(val),
                Err(err) => {
                    warn!("Failed to parse value for '{name}': {err}", name = name);
                    break;
                }
            }
        } else {
            break;
        }
    }
    debug!("Parsed {} partial witness values from JSON", values.len());
    values
}

fn handle_setup(
    input: PathBuf,
    proving_key: Option<PathBuf>,
    verification_key: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Reading R1CS from {}", input.display());
    let r1cs_file = File::open(&input)?;
    let r1cs = ConstraintSystem::from_file(r1cs_file)?;

    let base_name = infer_base_name(&input);
    let keys_dir = Path::new("keys");

    let proving_key_path =
        proving_key.unwrap_or_else(|| keys_dir.join(format!("{}_pk.bin", base_name)));
    let verification_key_path =
        verification_key.unwrap_or_else(|| keys_dir.join(format!("{}_vk.bin", base_name)));

    let circuit = LofCircuit {
        public_inputs: vec![Fr::from(0u64); r1cs.public_inputs.len()],
        witness: vec![Fr::from(0u64); 1],
        constraints: r1cs.constraints,
    };

    info!("Generating cryptographic keys...");
    let (pk, vk) = ProverKey::setup(circuit)?;

    if let Some(parent) = proving_key_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if let Some(parent) = verification_key_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    info!("Writing proving key to {}", proving_key_path.display());
    let pk_writer = BufWriter::new(File::create(&proving_key_path)?);
    pk.write(pk_writer)?;

    info!(
        "Writing verification key to {}",
        verification_key_path.display()
    );
    let vk_writer = BufWriter::new(File::create(&verification_key_path)?);
    vk.write(vk_writer)?;

    info!("Setup completed successfully!");
    info!("Next: Edit your input files, then generate a proof!");

    Ok(())
}

fn handle_prove(
    input: PathBuf,
    proving_key: Option<PathBuf>,
    public_inputs: Option<PathBuf>,
    witness: Option<PathBuf>,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Reading R1CS from {}", input.display());
    let r1cs_file = File::open(&input)?;
    let r1cs: ConstraintSystem = ConstraintSystem::from_file(r1cs_file)?;

    let base_name = infer_base_name(&input);
    let proving_key_path = proving_key
        .unwrap_or_else(|| Path::new("keys").join(format!("{}_pk.bin", base_name.as_str())));
    let public_inputs_path = public_inputs
        .unwrap_or_else(|| Path::new("inputs").join(format!("{}_public.json", base_name.as_str())));
    let output_path = output
        .unwrap_or_else(|| Path::new("proofs").join(format!("{}_proof.bin", base_name.as_str())));

    info!("Reading proving key from {}", proving_key_path.display());
    let pk_reader = BufReader::new(File::open(&proving_key_path)?);
    let pk = ProverKey::read(pk_reader)?;

    info!(
        "Reading public inputs from {}",
        public_inputs_path.display()
    );
    let pub_inputs_json: InputsJson = serde_json::from_reader(File::open(&public_inputs_path)?)?;
    debug!("Public inputs: {:?}", pub_inputs_json);

    let pub_values = parse_inputs_in_order(&pub_inputs_json, &r1cs.public_inputs)?;

    let witness_path = witness.unwrap_or_else(|| {
        Path::new("inputs").join(format!("{}_witness.json", base_name.as_str()))
    });

    let wit_values = if witness_path.exists() {
        info!("Reading provided witness from {}", witness_path.display());
        let wit_inputs_json: InputsJson = serde_json::from_reader(File::open(&witness_path)?)?;
        debug!("Witness inputs: {:?}", wit_inputs_json);
        let provided_witnesses = parse_partial_witness(&wit_inputs_json, &r1cs.witnesses);

        info!(
            "Generating full witness with {} provided witness values...",
            provided_witnesses.len()
        );
        generate_full_witness_with_provided(&r1cs, &pub_values, &provided_witnesses)?
    } else {
        info!("No witness file found, generating witness from constraints only...");
        generate_full_witness(&r1cs, &pub_values)?
    };

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let full_witness_map: IndexMap<String, String> = if r1cs.witnesses.len() == wit_values.len() {
        r1cs.witnesses
            .iter()
            .zip(wit_values.iter())
            .map(|(name, fr)| (name.clone(), fr.to_string()))
            .collect()
    } else {
        wit_values
            .iter()
            .enumerate()
            .map(|(i, fr)| (format!("witness_{}", i), fr.to_string()))
            .collect()
    };

    let witness_output_path = output_path
        .parent()
        .map(|parent| parent.join("full_witness.json"))
        .unwrap_or_else(|| PathBuf::from("full_witness.json"));

    info!("Saving full witness to {}", witness_output_path.display());
    serde_json::to_writer_pretty(File::create(&witness_output_path)?, &full_witness_map)?;

    if std::env::var("LOFIT_VERBOSE").is_ok() {
        debug!("Converted field elements:");
        for (i, val) in pub_values.iter().enumerate() {
            debug!("  Public input {}: {:?}", i, val);
        }
        for (i, val) in wit_values.iter().enumerate() {
            debug!("  Witness {}: {:?}", i, val);
        }

        debug!("R1CS Constraints:");
        for (i, constraint) in r1cs.constraints.iter().enumerate() {
            debug!("  Constraint {}:", i);
            debug!("    A terms: {:?}", constraint.a.terms);
            debug!("    B terms: {:?}", constraint.b.terms);
            debug!("    C terms: {:?}", constraint.c.terms);
        }
    }

    let circuit = LofCircuit {
        public_inputs: pub_values,
        witness: wit_values,
        constraints: r1cs.constraints,
    };

    info!("Generating proof...");
    let proof = pk.prove(circuit)?;

    info!("Writing proof to {}", output_path.display());
    let proof_writer = BufWriter::new(File::create(&output_path)?);
    proof.write(proof_writer)?;

    info!("Proof generated successfully!");
    info!("Next: Verify your proof!");

    Ok(())
}

fn handle_verify(
    verification_key: Option<PathBuf>,
    proof: Option<PathBuf>,
    public_inputs: Option<PathBuf>,
    input: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let base_name = detect_base_name_for_verify(input.as_ref());

    let verification_key_path = verification_key
        .unwrap_or_else(|| Path::new("keys").join(format!("{}_vk.bin", base_name.as_str())));
    let proof_path = proof
        .unwrap_or_else(|| Path::new("proofs").join(format!("{}_proof.bin", base_name.as_str())));
    let public_inputs_path = public_inputs
        .unwrap_or_else(|| Path::new("inputs").join(format!("{}_public.json", base_name.as_str())));

    info!(
        "Reading verification key from {}",
        verification_key_path.display()
    );
    let vk_contents = std::fs::read(&verification_key_path)?;
    debug!(
        "Read {} bytes from verification key file",
        vk_contents.len()
    );
    let vk = VerifierKey::read(&vk_contents[..])?;

    info!("Reading proof from {}", proof_path.display());
    let proof_contents = std::fs::read(&proof_path)?;
    let proof_obj = Proof::read(&proof_contents[..])?;

    info!(
        "Reading public inputs from {}",
        public_inputs_path.display()
    );
    let pub_inputs_json: InputsJson = serde_json::from_reader(File::open(&public_inputs_path)?)?;

    let pub_values: Vec<Fr> = pub_inputs_json
        .values()
        .map(|s| fr_from_str(s))
        .collect::<Result<Vec<_>, _>>()?;

    info!("Verifying proof with {} public inputs...", pub_values.len());
    match vk.verify(&proof_obj, &pub_values) {
        Ok(true) => {
            info!("Proof is valid!");
            info!("Zero-knowledge proof verification successful!");
        }
        Ok(false) => {
            error!("Proof is invalid!");
            error!("The proof does not satisfy the circuit constraints");
        }
        Err(e) => {
            error!("Verification error: {:?}", e);
            warn!("Check your input files and try again");
        }
    }

    Ok(())
}

fn infer_base_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("circuit")
        .to_string()
}

fn detect_base_name_for_verify(input: Option<&PathBuf>) -> String {
    if let Some(input_path) = input {
        let filename = input_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("circuit");
        if let Some(stripped) = filename.strip_suffix("_proof") {
            stripped.to_string()
        } else {
            filename.to_string()
        }
    } else {
        infer_base_from_keys_dir().unwrap_or_else(|| "circuit".to_string())
    }
}

fn infer_base_from_keys_dir() -> Option<String> {
    let entries = std::fs::read_dir("keys").ok()?;
    for entry in entries.filter_map(|e| e.ok()) {
        if let Some(filename) = entry.path().file_name().and_then(|s| s.to_str()) {
            if let Some(stripped) = filename.strip_suffix("_vk.bin") {
                return Some(stripped.to_string());
            }
        }
    }
    None
}

#[instrument]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Version => {
            println!("{}", VERSION);
            Ok(())
        }
        Commands::PackageWeb {
            input,
            output,
            skip_wasm,
        } => {
            let package_dir = package_for_web(&input, output.as_deref(), skip_wasm)?;
            println!("Web package ready at {}", package_dir.display());
            Ok(())
        }
        Commands::Setup {
            input,
            proving_key,
            verification_key,
        } => handle_setup(input, proving_key, verification_key),
        Commands::Prove {
            input,
            proving_key,
            public_inputs,
            witness,
            output,
        } => handle_prove(input, proving_key, public_inputs, witness, output),
        Commands::Verify {
            verification_key,
            proof,
            public_inputs,
            input,
        } => handle_verify(verification_key, proof, public_inputs, input),
    }
}
