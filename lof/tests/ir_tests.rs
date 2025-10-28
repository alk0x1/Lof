use std::collections::HashMap;

use lof::ast::Expression;
use lof::ir_generator::IRGenerator;
use lof::lexer::Lexer;
use lof::parser::Parser;
use lof::{IRCircuit, IRExpr, IRInstruction, IRType};

#[test]
fn test_ir_generator_resets_state_between_proofs() {
    let source = r#"
    proof First {
        witness out: field;
        out === 1
    }

    proof Second {
        witness out: field;
        out === 1
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

#[test]
fn test_ir_serialization() {
    let circuit = IRCircuit {
        name: "test".to_string(),
        pub_inputs: vec![("x".to_string(), IRType::Field)],
        witnesses: vec![("y".to_string(), IRType::Field)],
        outputs: vec![("z".to_string(), IRType::Field)],
        instructions: vec![
            IRInstruction::Assign {
                target: "z".to_string(),
                expr: IRExpr::Add(
                    Box::new(IRExpr::Variable("x".to_string())),
                    Box::new(IRExpr::Variable("y".to_string())),
                ),
            },
            IRInstruction::Constrain {
                left: IRExpr::Variable("z".to_string()),
                right: IRExpr::Constant("42".to_string()),
            },
        ],
        functions: HashMap::new(),
    };

    let json = serde_json::to_string_pretty(&circuit).unwrap();
    let deserialized: IRCircuit = serde_json::from_str(&json).unwrap();

    assert_eq!(circuit.name, deserialized.name);
    assert_eq!(circuit.pub_inputs.len(), deserialized.pub_inputs.len());
    assert_eq!(circuit.instructions.len(), deserialized.instructions.len());
}
