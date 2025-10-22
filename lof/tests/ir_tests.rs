use lof::ast::Expression;
use lof::ir_generator::IRGenerator;
use lof::lexer::Lexer;
use lof::parser::Parser;

#[test]
fn test_ir_generator_resets_state_between_proofs() {
    let source = r#"
    proof First {
        output out: field;
        out === 1;
    }

    proof Second {
        output out: field;
        out === 1;
    }
    "#;

    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);
    let ast = parser.parse_program().expect("parse program");

    let mut generator = IRGenerator::new();
    let mut circuits = Vec::new();

    for expr in &ast {
        if let Expression::Proof { .. } = expr {
            let circuit = generator.convert_proof(expr).expect("convert proof to IR");
            circuits.push(circuit);
        }
    }

    assert_eq!(circuits.len(), 2, "expected two circuits to be generated");
    let first = &circuits[0];
    let second = &circuits[1];

    assert_eq!(
        first.instructions.len(),
        second.instructions.len(),
        "IR generator should reset instructions between proofs"
    );
    assert_eq!(
        first.pub_inputs.len(),
        second.pub_inputs.len(),
        "IR generator should not accumulate public inputs between proofs"
    );
    assert_eq!(
        first.witnesses.len(),
        second.witnesses.len(),
        "IR generator should not accumulate witnesses between proofs"
    );
    assert_eq!(
        first.outputs.len(),
        second.outputs.len(),
        "IR generator should not accumulate outputs between proofs"
    );
}
