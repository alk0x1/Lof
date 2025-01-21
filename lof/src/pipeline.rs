use std::path::PathBuf;
use crate::lexer::Lexer;
use crate::logger::CompilerLogger;
use crate::parser::Parser;
use crate::typechecker::{TypeChecker, TypeError};
use crate::ast::Expression;
use crate::r1cs::R1CSGenerator;

#[derive(Debug)]
pub enum CompilerError {
  LexerError(String),
  ParserError(String),
  TypeCheckerError(TypeError),
  R1CSError,
  NoProofs,
}


pub struct CompilerPipeline {
  pub source: String,
  logger: CompilerLogger,
}

impl CompilerPipeline {
  pub fn new(source: String, verbose: bool) -> Self {
    Self {
      source,
      logger: CompilerLogger::new(verbose),
    }
  }

  pub fn run(&self, source_path: &PathBuf) -> Result<(), CompilerError> {
    self.logger.start_compilation();

    // Step 1: Lexical Analysis and Parsing
    self.logger.start_parsing();
    let lexer = Lexer::new(&self.source);
    let mut parser = Parser::new(lexer);
    
    let ast = parser.parse_program()
      .map_err(|e| {
        self.logger.parsing_failed(&e.to_string());
        CompilerError::ParserError(format!("{:?}", e))
      })?;
        
    if ast.is_empty() {
      self.logger.no_proofs_found();
      return Err(CompilerError::NoProofs);
    }

    self.logger.parsing_completed();

    // Step 2: Type Checking
    self.logger.start_type_checking();
    let mut type_checker = TypeChecker::new();
    let mut typed_proofs = Vec::new();
    
    for expr in ast.iter() {
      if let Expression::Proof { name, .. } = expr {
        self.logger.checking_proof(name);
        
        match type_checker.check_proof(expr) {
          Ok(_) => {
            typed_proofs.push(expr);
            self.logger.proof_type_checked(name);
          },
          Err(e) => {
            self.logger.type_check_failed(name, &e.to_string());
            return Err(CompilerError::TypeCheckerError(e));
          }
        }
      }
    }

    self.logger.type_checking_completed();

    // Step 3: R1CS Generation
    self.logger.start_r1cs_generation();
    let mut r1cs_generator = R1CSGenerator::new(&self.logger);
    
    for proof in &typed_proofs {
      if let Expression::Proof { name, .. } = proof {
        self.logger.converting_proof_to_r1cs(name);
        
        match r1cs_generator.convert_proof(proof) {
          Ok(_) => {
            // Check for empty constraints
            if r1cs_generator.constraints.is_empty() {
              self.logger.no_constraints_warning(name);
            }

            // Log statistics
            self.logger.proof_statistics(
              name,
              &r1cs_generator.pub_inputs,
              &r1cs_generator.witnesses,
              r1cs_generator.constraints.len()
            );

            // Generate R1CS file
            let r1cs_path = source_path.with_file_name(format!("{}.r1cs", name));
            self.logger.writing_r1cs(&r1cs_path);
            
            match r1cs_generator.write_r1cs_file(&r1cs_path) {
              Ok(size) => {
                self.logger.r1cs_write_success(
                  &r1cs_path,
                  size,
                  r1cs_generator.constraints.len()
                );
                
                // Log metadata
                self.logger.r1cs_metadata(
                  r1cs_generator.pub_inputs.len(),
                  r1cs_generator.witnesses.len(),
                  r1cs_generator.constraints.len()
                );
              },
              Err(e) => {
                self.logger.r1cs_write_failed(&e);
                return Err(CompilerError::R1CSError);
              }
            }
          },
          Err(e) => {
            self.logger.r1cs_generation_failed(name, &e.to_string());
            return Err(CompilerError::R1CSError);
          }
        }
      }
    }

    // Final compilation summary
    self.logger.compilation_summary(
      typed_proofs.len(),
      r1cs_generator.constraints.len()
    );

    Ok(())
  }
}