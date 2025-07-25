pub mod ast;
pub mod lexer;
pub mod parser;
pub mod typechecker;
pub mod r1cs;
pub mod pipeline;

pub use pipeline::{CompilerPipeline, CompilerError};
pub use ast::Expression;
pub use r1cs::{R1CSGenerator, R1CSError};

// Add this function for WASM integration
pub fn compile_dsl_to_r1cs(source: &str) -> Result<R1CSGenerator, CompilerError> {
    let lexer = lexer::Lexer::new(source);
    let mut parser = parser::Parser::new(lexer);
    
    let ast = parser.parse_program()
        .map_err(|e| CompilerError::ParserError(format!("{:?}", e)))?;
    
    if ast.is_empty() {
        return Err(CompilerError::NoProofs);
    }

    // Type check
    let mut type_checker = typechecker::TypeChecker::new();
    type_checker.check_program(&ast)
        .map_err(CompilerError::TypeCheckerError)?;

    // Generate R1CS
    let mut r1cs_generator = r1cs::R1CSGenerator::new();
    
    for proof in &ast {
        if let Expression::Proof { .. } = proof {
            r1cs_generator.convert_proof(proof)
                .map_err(|_| CompilerError::R1CSError)?;
        }
    }
    
    Ok(r1cs_generator)
}

// Helper for just parsing (useful for WASM debugging)
pub fn parse_dsl(source: &str) -> Result<Vec<Expression>, CompilerError> {
    let lexer = lexer::Lexer::new(source);
    let mut parser = parser::Parser::new(lexer);
    
    parser.parse_program()
        .map_err(|e| CompilerError::ParserError(format!("{:?}", e)))
}