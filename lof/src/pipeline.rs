use crate::ast::Expression;
use crate::ir_generator::IRGenerator;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::r1cs::R1CSGenerator;
use crate::typechecker::{TypeChecker, TypeError};
use tracing::{debug, error, info, instrument, warn};

#[derive(Debug)]
pub enum CompilerError {
    LexerError(String),
    ParserError(String),
    TypeCheckerError(TypeError),
    R1CSError,
    IRError(String),
    NoProofs,
}

pub struct CompilerPipeline {
    pub source: String,
}

impl CompilerPipeline {
    pub fn new(source: String, _verbose: bool) -> Self {
        Self { source }
    }

    #[instrument(skip(self, _source_path))]
    pub fn type_check_only(&self, _source_path: &std::path::Path) -> Result<(), CompilerError> {
        info!("Starting type checking process");

        info!("Parsing source code");
        let lexer = Lexer::new(&self.source);
        let mut parser = Parser::new(lexer);

        let ast = parser.parse_program().map_err(|e| {
            error!("Parsing failed: {}", e);
            CompilerError::ParserError(format!("{:?}", e))
        })?;

        if ast.is_empty() {
            error!("No proofs found in source code");
            return Err(CompilerError::NoProofs);
        }

        info!("Parsing completed successfully");

        info!("Performing type checking...");
        let mut type_checker = TypeChecker::new();

        type_checker.check_program(&ast).map_err(|e| {
            error!("Type checking failed: {}", e);
            CompilerError::TypeCheckerError(e)
        })?;

        info!("Type checking completed successfully");

        let proof_count = ast
            .iter()
            .filter(|e| matches!(e, Expression::Proof { .. }))
            .count();
        info!(
            "Type checking successful! Summary: {} proof(s) parsed and type checked",
            proof_count
        );

        Ok(())
    }

    #[instrument(skip(self, source_path))]
    pub fn run(&self, source_path: &std::path::Path) -> Result<(), CompilerError> {
        info!("Starting compilation process");

        // Step 1: Lexical Analysis and Parsing
        info!("Parsing source code");
        let lexer = Lexer::new(&self.source);
        let mut parser = Parser::new(lexer);

        let ast = parser.parse_program().map_err(|e| {
            error!("Parsing failed: {}", e);
            CompilerError::ParserError(format!("{:?}", e))
        })?;

        if ast.is_empty() {
            error!("No proofs found in source code");
            return Err(CompilerError::NoProofs);
        }

        info!("Parsing completed successfully");

        // Step 2: Type Checking
        info!("Performing type checking...");
        let mut type_checker = TypeChecker::new();

        type_checker.check_program(&ast).map_err(|e| {
            error!("Type checking failed: {}", e);
            CompilerError::TypeCheckerError(e)
        })?;

        info!("Type checking completed successfully");

        // Step 3: R1CS and IR Generation
        info!("Generating R1CS constraints and IR...");
        let mut r1cs_generator = R1CSGenerator::new();
        let mut ir_generator = IRGenerator::new();

        let file_stem = source_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");

        // First pass: Register all function definitions
        for item in &ast {
            if let Expression::FunctionDef {
                name, params, body, ..
            } = item
            {
                debug!("Registering function '{}'", name);
                r1cs_generator.register_function(name.clone(), params.clone(), *body.clone());
                ir_generator.register_function(name.clone(), params.clone(), *body.clone());
            }
        }

        // Second pass: Convert proofs
        for proof in &ast {
            if let Expression::Proof { name, .. } = proof {
                debug!("Converting proof '{}' to R1CS and IR", name);

                // Generate R1CS
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
                            }
                            Err(e) => {
                                error!("Failed to write R1CS file: {}", e);
                                return Err(CompilerError::R1CSError);
                            }
                        }
                    }
                    Err(e) => {
                        error!("R1CS generation failed for proof '{}': {}", name, e);
                        return Err(CompilerError::R1CSError);
                    }
                }

                // Generate IR
                match ir_generator.convert_proof(proof) {
                    Ok(ir_circuit) => {
                        let ir_path = source_path.with_file_name(format!("{}.ir", file_stem));
                        info!("Writing IR file to: {}", ir_path.display());

                        match ir_circuit.write_to_file(&ir_path) {
                            Ok(_) => {
                                info!(
                                    "Successfully wrote IR to {} ({} instructions)",
                                    ir_path.display(),
                                    ir_circuit.instructions.len()
                                );

                                // Log IR metadata
                                info!(
                                    "IR metadata: pub_inputs={}, witnesses={}, outputs={}, instructions={}",
                                    ir_circuit.pub_inputs.len(),
                                    ir_circuit.witnesses.len(),
                                    ir_circuit.outputs.len(),
                                    ir_circuit.instructions.len()
                                );
                            }
                            Err(e) => {
                                error!("Failed to write IR file: {}", e);
                                return Err(CompilerError::IRError(format!("{}", e)));
                            }
                        }
                    }
                    Err(e) => {
                        error!("IR generation failed for proof '{}': {:?}", name, e);
                        return Err(CompilerError::IRError(format!("{:?}", e)));
                    }
                }
            }
        }

        // Final compilation summary
        let proof_count = ast
            .iter()
            .filter(|e| matches!(e, Expression::Proof { .. }))
            .count();
        info!(
            "Compilation successful! Summary: {} proof(s) parsed, {} R1CS constraint(s) generated",
            proof_count,
            r1cs_generator.constraints.len()
        );

        Ok(())
    }
}
