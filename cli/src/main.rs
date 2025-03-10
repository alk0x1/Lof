use clap::{Parser, Subcommand};
use colored::*;
use lof::pipeline::{CompilerPipeline, CompilerError};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use serde_json::{json, to_string_pretty};

#[derive(Parser)]
#[command(name = "lof")]
#[command(about = "Lof language compiler for ZK circuit verification", long_about = None)]
struct Cli {
  #[command(subcommand)]
  command: Commands,
}

#[derive(Subcommand)]
enum Commands {
  /// Type check a Lof source file and generate R1CS
  Check {
    #[arg(value_name = "FILE")]
    file: PathBuf,

    #[arg(short, long)]
    verbose: bool,
    
    /// Generate JSON template files for inputs
    #[arg(short = 'g', long)]
    generate_templates: bool,
  },
}

fn main() {
  if let Err(err) = run() {
    eprintln!("{} {}", "Error:".red(), err);
    std::process::exit(1);
  }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
  let cli = Cli::parse();

  match cli.command {
    Commands::Check { file, verbose, generate_templates } => {
        if file.extension().and_then(|ext| ext.to_str()) != Some("lof") {
          return Err("File must have .lof extension".into());
        }

        println!("{} {}", "Processing".blue(), file.display());
        let source = fs::read_to_string(&file)?;

        let pipeline = CompilerPipeline::new(source, verbose);
        
        if verbose {
          println!("{}", "Starting compilation pipeline...".yellow());
        }

        match pipeline.run(&file) {
          Ok(_) => {
            println!("{}", "Compilation successful".green());
            
            if generate_templates {
              let base_name = file.file_stem().unwrap().to_str().unwrap();
              let r1cs_file = file.with_file_name(format!("{}.r1cs", base_name));
              
              if r1cs_file.exists() {
                generate_json_templates(base_name, &vec!["x".to_string()], &vec!["y".to_string()])?;
              } else {
                println!("{} {}", "Warning:".yellow(), "R1CS file not found, skipping template generation");
              }
            }
            
            Ok(())
          }
          Err(err) => {
            match err {
              CompilerError::LexerError(e) => {
                Err(format!("Lexer error: {}", e).into())
              }
              CompilerError::ParserError(e) => {
                Err(format!("Parser error: {}", e).into())
              }
              CompilerError::TypeCheckerError(e) => {
                Err(format!("Type error: {:?}", e).into())
              }
              CompilerError::NoProofs => {
                Err("No proofs found in the source file".into())
              }
              CompilerError::R1CSError => {
                Err("R1CS error".into())
              }
            }
          }
        }
    }
  }
}

fn generate_json_templates(
  proof_name: &str,
  public_inputs: &[String],
  witnesses: &[String]
) -> Result<(), Box<dyn std::error::Error>> {
  let public_values = public_inputs.iter()
    .map(|_| "0".to_string())
    .collect::<Vec<_>>();
  
  let public_json = json!({
    "inputs": public_values
  });
  
  let public_file = format!("{}_public.json", proof_name);
  let mut file = fs::File::create(&public_file)?;
  file.write_all(to_string_pretty(&public_json)?.as_bytes())?;
  println!("{} {}", "Generated public inputs template:".green(), public_file);

  let witness_values = witnesses.iter()
    .map(|_| "0".to_string())
    .collect::<Vec<_>>();
  
  let witness_json = json!({
    "inputs": witness_values
  });
  
  let witness_file = format!("{}_witness.json", proof_name);
  let mut file = fs::File::create(&witness_file)?;
  file.write_all(to_string_pretty(&witness_json)?.as_bytes())?;
  println!("{} {}", "Generated witness template:".green(), witness_file);

  println!("\nTo use these templates:");
  println!("1. Edit {} with your public input values", public_file);
  println!("2. Edit {} with your witness values", witness_file);
  println!("3. Run the following commands:");
  println!("   lofit setup --input {}.r1cs --proving-key pk.bin --verification-key vk.bin", proof_name);
  println!("   lofit prove --input {}.r1cs --proving-key pk.bin --public-inputs {} --witness {} --output proof.bin", 
           proof_name, public_file, witness_file);
  println!("   lofit verify --verification-key vk.bin --proof proof.bin --public-inputs {}", public_file);

  Ok(())
}