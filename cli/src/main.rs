use clap::{Parser, Subcommand};
use colored::*;
use lof::pipeline::{CompilerPipeline, CompilerError};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "lof")]
#[command(about = "Lof language compiler for ZK circuit verification", long_about = None)]
struct Cli {
  #[command(subcommand)]
  command: Commands,
}

#[derive(Subcommand)]
enum Commands {
  /// Type check a Lof source file
  Check {
    #[arg(value_name = "FILE")]
    file: PathBuf,

    #[arg(short, long)]
    verbose: bool,
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
    Commands::Check { file, verbose } => {
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
            println!("{}", "✓ Compilation successful".green());
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