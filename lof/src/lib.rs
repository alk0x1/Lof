pub mod ast;
pub mod cli;
pub mod ir;
pub mod ir_generator;
pub mod lexer;
pub mod parser;
pub mod pipeline;
pub mod r1cs;
pub mod typechecker;

pub use ast::Expression;
pub use ir::{IRCircuit, IRExpr, IRInstruction, IRType};
pub use ir_generator::{IRGenError, IRGenerator};
pub use pipeline::{CompilerError, CompilerPipeline};
pub use r1cs::{R1CSError, R1CSGenerator};

// Add this function for WASM integration
pub fn compile_dsl_to_r1cs(source: &str) -> Result<R1CSGenerator, CompilerError> {
    let lexer = lexer::Lexer::new(source);
    let mut parser = parser::Parser::new(lexer);

    let ast = parser
        .parse_program()
        .map_err(|e| CompilerError::ParserError(format!("{:?}", e)))?;

    if ast.is_empty() {
        return Err(CompilerError::NoProofs);
    }

    // Type check
    let mut type_checker = typechecker::TypeChecker::new();
    type_checker
        .check_program(&ast)
        .map_err(CompilerError::TypeCheckerError)?;

    // Generate R1CS
    let mut r1cs_generator = r1cs::R1CSGenerator::new();

    // First pass: Register function definitions
    for item in &ast {
        if let Expression::FunctionDef {
            name, params, body, ..
        } = item
        {
            r1cs_generator.register_function(name.clone(), params.clone(), *body.clone());
        }
    }

    // Second pass: Convert proofs
    for proof in &ast {
        if let Expression::Proof { .. } = proof {
            r1cs_generator
                .convert_proof(proof)
                .map_err(|_| CompilerError::R1CSError)?;
        }
    }

    Ok(r1cs_generator)
}

// Helper for just parsing (useful for WASM debugging)
pub fn parse_dsl(source: &str) -> Result<Vec<Expression>, CompilerError> {
    let lexer = lexer::Lexer::new(source);
    let mut parser = parser::Parser::new(lexer);

    parser
        .parse_program()
        .map_err(|e| CompilerError::ParserError(format!("{:?}", e)))
}
