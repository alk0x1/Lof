use clap::{Parser, Subcommand};
use colored::*;
use lof::pipeline::{CompilerPipeline, CompilerError};
use lof::lexer::Lexer;
use lof::parser::Parser as LofParser;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use serde_json::{json, to_string_pretty};
use tracing::{info, warn, error, debug};
use tracing_subscriber;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "lof")]
#[command(version = VERSION)]
#[command(about = "Lof language compiler for ZK circuit verification", long_about = None)]
struct Cli {
  #[command(subcommand)]
  command: Commands,
}

#[derive(Subcommand)]
enum Commands {
  Check {
    #[arg(value_name = "FILE")]
    file: PathBuf,

    #[arg(short, long)]
    verbose: bool,
  },
  /// Compile a Lof source file and generate R1CS with organized output
  Compile {
    #[arg(value_name = "FILE")]
    file: PathBuf,

    #[arg(short, long)]
    verbose: bool,
    
    /// Generate JSON template files for inputs
    #[arg(short = 'g', long)]
    generate_templates: bool,
  },
  /// Parse a Lof source file and display the AST (parser testing)
  Parse {
    #[arg(value_name = "FILE")]
    file: PathBuf,

    #[arg(short, long)]
    verbose: bool,

    /// Pretty print the AST output
    #[arg(short, long)]
    pretty: bool,
  },
  Version,
}

fn main() {
  // Initialize tracing
  if std::env::var("RUST_LOG").is_err() {
    std::env::set_var("RUST_LOG", "info");
  }
  tracing_subscriber::fmt::init();

  if let Err(err) = run() {
    error!("Application error: {}", err);
    eprintln!("{} {}", "Error:".red(), err);
    std::process::exit(1);
  }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
  let cli = Cli::parse();

  match cli.command {
    Commands::Version => {
      println!("{}", VERSION);
      Ok(())
    }
    Commands::Check { file, verbose } => {
        if file.extension().and_then(|ext| ext.to_str()) != Some("lof") {
          let err_msg = "File must have .lof extension";
          error!("{}", err_msg);
          return Err(err_msg.into());
        }

        info!("Type checking file: {}", file.display());
        println!("{} {}", "Type checking".blue(), file.display());
        let source = fs::read_to_string(&file)?;

        let pipeline = CompilerPipeline::new(source, verbose);
        
        if verbose {
          debug!("Starting type checking in verbose mode");
          println!("{}", "Starting type checking...".yellow());
        }

        match pipeline.type_check_only(&file) {
          Ok(_) => {
            info!("Type checking completed successfully");
            println!("{}", "Type checking successful".green());
            Ok(())
          }
          Err(err) => {
            match err {
              CompilerError::LexerError(e) => {
                error!("Lexer error: {}", e);
                Err(format!("Lexer error: {}", e).into())
              }
              CompilerError::ParserError(e) => {
                error!("Parser error: {}", e);
                Err(format!("Parser error: {}", e).into())
              }
              CompilerError::TypeCheckerError(e) => {
                error!("Type error: {:?}", e);
                Err(format!("Type error: {:?}", e).into())
              }
              CompilerError::NoProofs => {
                error!("No proofs found in the source file");
                Err("No proofs found in the source file".into())
              }
              CompilerError::R1CSError => {
                error!("R1CS generation failed");
                Err("R1CS error".into())
              }
            }
          }
        }
    }
    Commands::Compile { file, verbose, generate_templates } => {
        if file.extension().and_then(|ext| ext.to_str()) != Some("lof") {
          let err_msg = "File must have .lof extension";
          error!("{}", err_msg);
          return Err(err_msg.into());
        }

        let base_name = file.file_stem().unwrap().to_str().unwrap();
        let file_dir = file.parent().unwrap_or_else(|| std::path::Path::new("."));

        // Create directories relative to the .lof file location
        let build_dir = file_dir.join("build");
        let keys_dir = file_dir.join("keys");
        let inputs_dir = file_dir.join("inputs");
        let proofs_dir = file_dir.join("proofs");
        
        fs::create_dir_all(&build_dir)?;
        fs::create_dir_all(&keys_dir)?;
        fs::create_dir_all(&inputs_dir)?;
        fs::create_dir_all(&proofs_dir)?;
        println!("{}", "Created project directories: build/, keys/, inputs/, proofs/".cyan());

        info!("Processing file: {}", file.display());
        println!("{} {}", "Processing".blue(), file.display());
        let source = fs::read_to_string(&file)?;

        let pipeline = CompilerPipeline::new(source, verbose);
        
        if verbose {
          debug!("Starting compilation pipeline in verbose mode");
          println!("{}", "Starting compilation pipeline...".yellow());
        }

        match pipeline.run(&file) {
          Ok(_) => {
            info!("Compilation completed successfully");
            println!("{}", "Compilation successful".green());
            
            // Check that R1CS was generated (should be next to .lof file)
            let r1cs_file = file.with_extension("r1cs");
            
            if r1cs_file.exists() {
              // Move R1CS to build directory
              let build_r1cs = build_dir.join(format!("{}.r1cs", base_name));
              fs::rename(&r1cs_file, &build_r1cs)?;
              println!("{} {}", "Generated R1CS:".green(), build_r1cs.display());
              
              if generate_templates {
                info!("Generating JSON templates for proof: {}", base_name);
                
                // Generate templates directly in inputs/ directory
                generate_json_templates(&file_dir, base_name, &vec!["x".to_string()], &vec!["y".to_string()])?;
                
                // Print next steps
                print_next_steps(&file_dir, base_name)?;
              }
            } else {
              warn!("R1CS file not found, compilation may have failed");
              println!("{} {}", "Warning:".yellow(), "R1CS file not generated");
            }
            
            Ok(())
          }
          Err(err) => {
            match err {
              CompilerError::LexerError(e) => {
                error!("Lexer error: {}", e);
                Err(format!("Lexer error: {}", e).into())
              }
              CompilerError::ParserError(e) => {
                error!("Parser error: {}", e);
                Err(format!("Parser error: {}", e).into())
              }
              CompilerError::TypeCheckerError(e) => {
                error!("Type error: {:?}", e);
                Err(format!("Type error: {:?}", e).into())
              }
              CompilerError::NoProofs => {
                error!("No proofs found in the source file");
                Err("No proofs found in the source file".into())
              }
              CompilerError::R1CSError => {
                error!("R1CS generation failed");
                Err("R1CS error".into())
              }
            }
          }
        }
    }
    Commands::Parse { file, verbose, pretty } => {
        if file.extension().and_then(|ext| ext.to_str()) != Some("lof") {
          let err_msg = "File must have .lof extension";
          error!("{}", err_msg);
          return Err(err_msg.into());
        }

        info!("Parsing file: {}", file.display());
        println!("{} {}", "Parsing".blue(), file.display());
        
        let source = fs::read_to_string(&file)?;
        
        if verbose {
          println!("\n{}", "Source code:".yellow());
          println!("{}", source);
          println!("\n{}", "--- Lexing & Parsing ---".yellow());
        }

        let lexer = Lexer::new(&source);
        let mut parser = LofParser::new(lexer);
        
        match parser.parse_program() {
          Ok(ast) => {
            info!("Parsing completed successfully");
            println!("{}", "✅ Parsing successful!".green());
            
            println!("\n{}", "AST:".cyan());
            if pretty {
              println!("{:#?}", ast);
            } else {
              println!("{:?}", ast);
            }
            
            Ok(())
          }
          Err(e) => {
            error!("Parser error: {}", e);
            println!("{} {}", "❌ Parse error:".red(), e);
            Err(format!("Parser error: {}", e).into())
          }
        }
    }
  }
}

fn generate_json_templates(
  base_dir: &std::path::Path,
  proof_name: &str,
  public_inputs: &[String],
  witnesses: &[String]
) -> Result<(), Box<dyn std::error::Error>> {
  debug!("Generating templates for proof '{}' with {} public inputs and {} witnesses", 
         proof_name, public_inputs.len(), witnesses.len());

  let public_values = public_inputs.iter()
    .map(|_| "0".to_string())
    .collect::<Vec<_>>();
  
  let public_json = json!({
    "inputs": public_values
  });
  
  // Generate directly in inputs directory (no duplication)
  let public_file = base_dir.join("inputs").join(format!("{}_public.json", proof_name));
  let mut file = fs::File::create(&public_file)?;
  file.write_all(to_string_pretty(&public_json)?.as_bytes())?;
  info!("Generated public inputs template: {}", public_file.display());
  println!("{} {}", "Generated public inputs template:".green(), public_file.display());

  let witness_values = witnesses.iter()
    .map(|_| "0".to_string())
    .collect::<Vec<_>>();
  
  let witness_json = json!({
    "inputs": witness_values
  });
  
  let witness_file = base_dir.join("inputs").join(format!("{}_witness.json", proof_name));
  let mut file = fs::File::create(&witness_file)?;
  file.write_all(to_string_pretty(&witness_json)?.as_bytes())?;
  info!("Generated witness template: {}", witness_file.display());
  println!("{} {}", "Generated witness template:".green(), witness_file.display());

  Ok(())
}

fn print_next_steps(base_dir: &std::path::Path, proof_name: &str) -> Result<(), Box<dyn std::error::Error>> {
  println!("\n{}", "🚀 Next steps:".bold().green());
  
  let build_r1cs = base_dir.join("build").join(format!("{}.r1cs", proof_name));
  let keys_pk = base_dir.join("keys").join(format!("{}_pk.bin", proof_name));
  let keys_vk = base_dir.join("keys").join(format!("{}_vk.bin", proof_name));
  let inputs_public = base_dir.join("inputs").join(format!("{}_public.json", proof_name));
  let inputs_witness = base_dir.join("inputs").join(format!("{}_witness.json", proof_name));
  let proofs_proof = base_dir.join("proofs").join(format!("{}_proof.bin", proof_name));
  
  println!("1. Generate keys:");
  println!("   {}", format!("lofit setup --input {} --proving-key {} --verification-key {}", 
           build_r1cs.display(), keys_pk.display(), keys_vk.display()).cyan());
  
  println!("\n2. Edit your input files:");
  println!("   {} - Edit with your public input values", inputs_public.display().to_string().yellow());
  println!("   {} - Edit with your witness values", inputs_witness.display().to_string().yellow());
  
  println!("\n3. Generate proof:");
  println!("   {}", format!("lofit prove --input {} --proving-key {} --public-inputs {} --witness {} --output {}", 
           build_r1cs.display(), keys_pk.display(), inputs_public.display(), inputs_witness.display(), proofs_proof.display()).cyan());
  
  println!("\n4. Verify proof:");
  println!("   {}", format!("lofit verify --verification-key {} --proof {} --public-inputs {}", 
           keys_vk.display(), proofs_proof.display(), inputs_public.display()).cyan());

  println!("\n{}", "💡 Tip: Copy and paste these commands directly!".bright_blue());
  Ok(())
}