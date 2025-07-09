use std::path::PathBuf;
use crate::lexer::Lexer;
use crate::parser::Parser;
// use crate::typechecker::{TypeChecker, TypeError};
use crate::ast::Expression;
use crate::r1cs::R1CSGenerator;
use tracing::{info, warn, error, debug, instrument};

#[derive(Debug)]
pub enum CompilerError {
  LexerError(String),
  ParserError(String),
  // TypeCheckerError(TypeError),
  R1CSError,
  NoProofs,
}

pub struct CompilerPipeline {
  pub source: String,
}

impl CompilerPipeline {
  pub fn new(source: String, _verbose: bool) -> Self {
    Self {
      source,
    }
  }

  #[instrument(skip(self, source_path))]
  pub fn run(&self, source_path: &PathBuf) -> Result<(), CompilerError> {
    info!("Starting compilation process");
    
    // Step 1: Lexical Analysis and Parsing
    info!("Parsing source code");
    let lexer = Lexer::new(&self.source);
    let mut parser = Parser::new(lexer);
    
    let ast = parser.parse_program()
      .map_err(|e| {
        error!("Parsing failed: {}", e);
        CompilerError::ParserError(format!("{:?}", e))
      })?;
        
    if ast.is_empty() {
      error!("No proofs found in source code");
      return Err(CompilerError::NoProofs);
    }

    info!("Parsing completed successfully");

    // Step 2: Type Checking
    // info!("Performing type checking...");
    // let mut type_checker = TypeChecker::new();
    let typed_proofs = Vec::new();
    
    // for expr in ast.iter() {
    //   if let Expression::Proof { name, .. } = expr {
    //     debug!("Checking proof '{}'", name);
        
    //     match type_checker.check_proof(expr) {
    //       Ok(_) => {
    //         typed_proofs.push(expr);
    //         debug!("Proof '{}' type-checked successfully", name);
    //       },
    //       Err(e) => {
    //         error!("Type checking failed for proof '{}': {}", name, e);
    //         // return Err(CompilerError::TypeCheckerError(e));
    //       }
    //     }
    //   }
    // }

    // info!("Type checking completed successfully");

    // Step 3: R1CS Generation
    info!("Generating R1CS constraints...");
    let mut r1cs_generator = R1CSGenerator::new();
    
    let file_stem = source_path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    
    for proof in &typed_proofs {
      if let Expression::Proof { name, .. } = proof {
        debug!("Converting proof '{}' to R1CS", name);
        
        match r1cs_generator.convert_proof(proof) {
          Ok(_) => {
            // Check for empty constraints
            if r1cs_generator.constraints.is_empty() {
              warn!("Proof '{}' generated no constraints", name);
            }

            // Log statistics
            debug!(
              "Statistics for proof '{}': pub_inputs={:?}, witnesses={:?}, constraints={}",
              name,
              r1cs_generator.pub_inputs,
              r1cs_generator.witnesses,
              r1cs_generator.constraints.len()
            );

            // Generate R1CS file
            let r1cs_path = source_path.with_file_name(format!("{}.r1cs", file_stem));
            info!("Writing R1CS file to: {}", r1cs_path.display());
            
            match r1cs_generator.write_r1cs_file(&r1cs_path) {
              Ok(size) => {
                info!(
                  "Successfully wrote {} bytes to {} ({} constraints)",
                  size,
                  r1cs_path.display(),
                  r1cs_generator.constraints.len()
                );
                
                // Log metadata
                info!(
                  "R1CS metadata: pub_inputs={}, witnesses={}, constraints={}",
                  r1cs_generator.pub_inputs.len(),
                  r1cs_generator.witnesses.len(),
                  r1cs_generator.constraints.len()
                );
              },
              Err(e) => {
                error!("Failed to write R1CS file: {}", e);
                return Err(CompilerError::R1CSError);
              }
            }
          },
          Err(e) => {
            error!("R1CS generation failed for proof '{}': {}", name, e);
            return Err(CompilerError::R1CSError);
          }
        }
      }
    }

    // Final compilation summary
    info!(
      "Compilation successful! Summary: {} proof(s) parsed, {} R1CS constraint(s) generated",
      typed_proofs.len(),
      r1cs_generator.constraints.len()
    );

    Ok(())
  }
}