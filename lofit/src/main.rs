use ark_bn254::Fr;
use clap::{Parser, Subcommand};
use lofit::{ConstraintSystem, LofCircuit, Proof, ProverKey, VerifierKey};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use lofit::generate_full_witness;

#[derive(Parser)]
#[command(name = "lofit")]
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
    proving_key: PathBuf,
    #[arg(short = 'v', long)]
    verification_key: PathBuf,
  },
  Prove {
    #[arg(short, long)]
    input: PathBuf,
    #[arg(short = 'p', long)]
    proving_key: PathBuf,
    #[arg(short = 'u', long = "public-inputs")]
    public_inputs: PathBuf,
    #[arg(short, long)]
    witness: Option<PathBuf>,
    #[arg(short, long)]
    output: PathBuf,
  },
  Verify {
    #[arg(short, long)]
    verification_key: PathBuf,
    #[arg(short, long)]
    proof: PathBuf,
    #[arg(short = 'u', long = "public-inputs")]
    public_inputs: PathBuf,
  },
}
#[derive(Serialize, Deserialize)]
struct InputsJson {
  inputs: Vec<String>
}

fn fr_from_str(s: &str) -> Result<Fr, Box<dyn std::error::Error>> {
  let num = s.parse::<u64>()?;
  Ok(Fr::from(num))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let cli = Cli::parse();

  match cli.command {
    Commands::Setup { input, proving_key, verification_key } => {
      println!("Reading R1CS from {:?}", input);
      let r1cs_file = File::open(input)?;
      let r1cs = ConstraintSystem::from_file(r1cs_file)?;

      let circuit = LofCircuit {
        public_inputs: vec![Fr::from(0u64); r1cs.public_inputs.len()],
        witness: vec![Fr::from(0u64); 1],
        constraints: r1cs.constraints,
      };

      println!("Generating keys...");
      let (pk, vk) = ProverKey::setup(circuit)?;

      println!("Writing proving key to {:?}", proving_key);
      let pk_writer = BufWriter::new(File::create(proving_key)?);
      pk.write(pk_writer)?;

      println!("Writing verification key to {:?}", verification_key);
      let vk_writer = BufWriter::new(File::create(verification_key)?);
      vk.write(vk_writer)?;
    }
    Commands::Prove { input, proving_key, public_inputs, witness, output } => {
      println!("Reading R1CS...");
      let r1cs_file = File::open(input)?;
      let r1cs: ConstraintSystem = ConstraintSystem::from_file(r1cs_file)?;

      println!("Reading proving key...");
      let pk_reader = BufReader::new(File::open(proving_key)?);
      let pk = ProverKey::read(pk_reader)?;

      println!("Reading inputs...");
      let pub_inputs: InputsJson = serde_json::from_reader(File::open(public_inputs)?)?;

      println!("\nPublic inputs from JSON: {:?}", pub_inputs.inputs);

      let pub_values: Vec<Fr> = pub_inputs.inputs
        .iter()
        .map(|s| fr_from_str(s))
        .collect::<Result<Vec<_>, _>>()?;
        println!("Generating full witness...");
        let auto_witness = generate_full_witness(&r1cs, &pub_values)?;
        
        let wit_values = if let Some(witness_path) = witness {
          println!("Reading witness from {:?}", witness_path);
          let wit_inputs: InputsJson = serde_json::from_reader(File::open(witness_path)?)?;
          println!("Witness inputs from JSON: {:?}", wit_inputs.inputs);
            
          wit_inputs.inputs
            .iter()
            .map(|s| fr_from_str(s))
            .collect::<Result<Vec<_>, _>>()?
        } else {
          println!("Using auto-generated witness");
          auto_witness
        };

        let full_witness = InputsJson {
          inputs: wit_values.iter().map(|fr| fr.to_string()).collect()
        };
        let witness_path = output.with_file_name("full_witness.json");
        println!("Saving full witness to {:?}", witness_path);
        serde_json::to_writer_pretty(
          File::create(&witness_path)?,
          &full_witness
        )?;


      println!("\nConverted field elements:");
      for (i, val) in pub_values.iter().enumerate() {
        println!("Public input {}: {:?}", i, val);
      }
      for (i, val) in wit_values.iter().enumerate() {
        println!("Witness {}: {:?}", i, val);
      }

      println!("\nR1CS Constraints:");
      for (i, constraint) in r1cs.constraints.iter().enumerate() {
        println!("Constraint {}:", i);
        println!("  A terms: {:?}", constraint.a.terms);
        println!("  B terms: {:?}", constraint.b.terms);
        println!("  C terms: {:?}", constraint.c.terms);
      }

      let circuit = LofCircuit {
        public_inputs: pub_values,
        witness: wit_values,
        constraints: r1cs.constraints,
      };

      println!("\nGenerating proof...");
      let proof = pk.prove(circuit)?;

      println!("Writing proof to {:?}", output);
      let proof_writer = BufWriter::new(File::create(output)?);
      proof.write(proof_writer)?;
    },
    Commands::Verify { verification_key, proof, public_inputs } => {
      println!("Reading verification key from {:?}...", verification_key);
      let vk_contents = std::fs::read(&verification_key)?;
      println!("Read {} bytes from verification key file", vk_contents.len());
      
      let vk = VerifierKey::read(&vk_contents[..])?;
      
      println!("Reading proof...");
      let proof_contents = std::fs::read(proof)?;
      let proof = Proof::read(&proof_contents[..])?;

      println!("Reading public inputs...");
      let pub_inputs: InputsJson = serde_json::from_reader(File::open(public_inputs)?)?;
      let pub_values: Vec<Fr> = pub_inputs.inputs
        .iter()
        .map(|s| fr_from_str(s))
        .collect::<Result<Vec<_>, _>>()?;

      println!("Verifying proof with {} public inputs...", pub_values.len());
      match vk.verify(&proof, &pub_values) {
        Ok(true) => println!("Proof is valid!"),
        Ok(false) => println!("Proof is invalid!"),
        Err(e) => println!("Verification error: {:?}", e),
      }
    }
  }

  Ok(())
}
