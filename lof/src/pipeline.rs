use crate::ast::{Expression, Parameter, Visibility};
use crate::ir::IRCircuit;
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

        let ast = self.parse_program()?;
        if ast.is_empty() {
            error!("No proofs found in source code");
            return Err(CompilerError::NoProofs);
        }
        self.type_check_ast(&ast)?;
        self.log_typecheck_summary(&ast);

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
        let ast = self.parse_program()?;
        self.ensure_proofs_present(&ast)?;
        self.type_check_ast(&ast)?;

        info!("Generating R1CS constraints and IR...");
        let mut r1cs_generator = R1CSGenerator::new();
        let mut ir_generator = IRGenerator::new();
        self.register_items(&ast, &mut r1cs_generator, &mut ir_generator);

        let file_stem = source_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");

        self.convert_proofs(source_path, file_stem, &ast, r1cs_generator, ir_generator)
    }

    fn parse_program(&self) -> Result<Vec<Expression>, CompilerError> {
        info!("Parsing source code");
        let lexer = Lexer::new(&self.source);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_program().map_err(|e| {
            error!("Parsing failed: {}", e);
            CompilerError::ParserError(format!("{:?}", e))
        })?;
        info!("Parsing completed successfully");
        Ok(ast)
    }

    fn type_check_ast(&self, ast: &[Expression]) -> Result<(), CompilerError> {
        info!("Performing type checking...");
        let mut type_checker = TypeChecker::new();
        type_checker.check_program(ast).map_err(|e| {
            error!("Type checking failed: {}", e);
            CompilerError::TypeCheckerError(e)
        })?;
        info!("Type checking completed successfully");
        Ok(())
    }

    fn ensure_proofs_present(&self, ast: &[Expression]) -> Result<(), CompilerError> {
        if ast.iter().any(|e| matches!(e, Expression::Proof { .. })) {
            Ok(())
        } else {
            error!("No proofs found in source code");
            Err(CompilerError::NoProofs)
        }
    }

    fn log_typecheck_summary(&self, ast: &[Expression]) {
        let proof_count = ast
            .iter()
            .filter(|e| matches!(e, Expression::Proof { .. }))
            .count();
        info!(
            "Type checking successful! Summary: {} proof(s) parsed and type checked",
            proof_count
        );
    }

    fn register_items(
        &self,
        ast: &[Expression],
        r1cs_generator: &mut R1CSGenerator,
        ir_generator: &mut IRGenerator,
    ) {
        for item in ast {
            match item {
                Expression::FunctionDef {
                    name, params, body, ..
                } => {
                    debug!("Registering function '{}'", name);
                    r1cs_generator.register_function(name.clone(), params.clone(), *body.clone());
                    ir_generator.register_function(name.clone(), params.clone(), *body.clone());
                }
                Expression::Component {
                    name,
                    signals,
                    body,
                    ..
                } => {
                    debug!("Registering component '{}'", name);
                    let params: Vec<Parameter> = signals
                        .iter()
                        .filter(|s| s.visibility == Visibility::Input)
                        .map(|s| Parameter {
                            name: s.name.clone(),
                            typ: s.typ.clone(),
                        })
                        .collect();
                    r1cs_generator.register_function(name.clone(), params.clone(), *body.clone());
                    ir_generator.register_component(name.clone(), params.clone(), *body.clone());
                }
                _ => {}
            }
        }
    }

    fn convert_proofs(
        &self,
        source_path: &std::path::Path,
        file_stem: &str,
        ast: &[Expression],
        mut r1cs_generator: R1CSGenerator,
        mut ir_generator: IRGenerator,
    ) -> Result<(), CompilerError> {
        let mut total_constraints = 0;
        for proof in ast.iter().filter(|e| matches!(e, Expression::Proof { .. })) {
            if let Expression::Proof { name, .. } = proof {
                debug!("Converting proof '{}' to R1CS and IR", name);
                total_constraints +=
                    self.generate_r1cs(source_path, file_stem, name, proof, &mut r1cs_generator)?;
                self.generate_ir(source_path, file_stem, name, proof, &mut ir_generator)?;
            }
        }

        self.log_compilation_summary(ast, total_constraints);

        Ok(())
    }

    fn generate_r1cs(
        &self,
        source_path: &std::path::Path,
        file_stem: &str,
        proof_name: &str,
        proof: &Expression,
        r1cs_generator: &mut R1CSGenerator,
    ) -> Result<usize, CompilerError> {
        r1cs_generator.convert_proof(proof).map_err(|e| {
            error!("R1CS generation failed for proof '{}': {}", proof_name, e);
            CompilerError::R1CSError
        })?;

        let constraint_count = r1cs_generator.constraints.len();
        let r1cs_path = source_path.with_file_name(format!("{}.r1cs", file_stem));
        self.write_r1cs_artifact(r1cs_generator, &r1cs_path, proof_name)?;

        Ok(constraint_count)
    }

    fn generate_ir(
        &self,
        source_path: &std::path::Path,
        file_stem: &str,
        proof_name: &str,
        proof: &Expression,
        ir_generator: &mut IRGenerator,
    ) -> Result<(), CompilerError> {
        let ir_circuit = ir_generator.convert_proof(proof).map_err(|e| {
            error!("IR generation failed for proof '{}': {:?}", proof_name, e);
            CompilerError::IRError(format!("{:?}", e))
        })?;

        let ir_path = source_path.with_file_name(format!("{}.ir", file_stem));
        self.write_ir_artifact(&ir_circuit, &ir_path)?;

        Ok(())
    }

    fn write_r1cs_artifact(
        &self,
        r1cs_generator: &R1CSGenerator,
        r1cs_path: &std::path::Path,
        proof_name: &str,
    ) -> Result<(), CompilerError> {
        let constraint_count = r1cs_generator.constraints.len();

        if constraint_count == 0 {
            warn!("Proof '{}' generated no constraints", proof_name);
        }

        debug!(
            "Statistics for proof '{}': pub_inputs={:?}, witnesses={:?}, constraints={}",
            proof_name, r1cs_generator.pub_inputs, r1cs_generator.witnesses, constraint_count
        );

        info!("Writing R1CS file to: {}", r1cs_path.display());
        r1cs_generator.write_r1cs_file(r1cs_path).map_err(|e| {
            error!("Failed to write R1CS file: {}", e);
            CompilerError::R1CSError
        })?;

        info!(
            "R1CS metadata: pub_inputs={}, witnesses={}, constraints={}",
            r1cs_generator.pub_inputs.len(),
            r1cs_generator.witnesses.len(),
            constraint_count
        );

        Ok(())
    }

    fn write_ir_artifact(
        &self,
        circuit: &IRCircuit,
        ir_path: &std::path::Path,
    ) -> Result<(), CompilerError> {
        info!("Writing IR file to: {}", ir_path.display());
        circuit.write_to_file(ir_path).map_err(|e| {
            error!("Failed to write IR file: {}", e);
            CompilerError::IRError(e.to_string())
        })?;

        info!(
            "IR metadata: pub_inputs={}, witnesses={}, outputs={}, instructions={}",
            circuit.pub_inputs.len(),
            circuit.witnesses.len(),
            circuit.outputs.len(),
            circuit.instructions.len()
        );
        Ok(())
    }

    fn log_compilation_summary(&self, ast: &[Expression], total_constraints: usize) {
        let proof_count = ast
            .iter()
            .filter(|e| matches!(e, Expression::Proof { .. }))
            .count();
        info!(
            "Compilation successful! Summary: {} proof(s) parsed, {} total R1CS constraint(s) generated",
            proof_count, total_constraints
        );
    }
}
