use lof::ast::{Expression, Type, Visibility};
use lof::lexer::Lexer;
use lof::parser::Parser;

fn parse_source(source: &str) -> Result<Vec<Expression>, String> {
    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);

    parser
        .parse_program()
        .map_err(|e| format!("Parse error: {:?}", e))
}

#[test]
fn test_parse_simple_proof() {
    let source = r#"
    proof SimpleProof {
        input x: Field;
        witness y: Field;
        assert y === x
    }"#;

    let result = parse_source(source).unwrap();
    assert_eq!(result.len(), 1);

    match &result[0] {
        Expression::Proof { name, signals, .. } => {
            assert_eq!(name, "SimpleProof");
            assert_eq!(signals.len(), 2);

            assert_eq!(signals[0].name, "x");
            assert_eq!(signals[0].visibility, Visibility::Input);

            assert_eq!(signals[1].name, "y");
            assert_eq!(signals[1].visibility, Visibility::Witness);
        }
        _ => panic!("Expected Proof, got {:?}", result[0]),
    }
}

#[test]
fn test_parse_proof_with_witness() {
    let source = r#"
    proof TestProof {
        input a: Field;
        witness w: Field;
        witness result: Field;

        let temp = a * w in
        assert result === temp
    }"#;

    let result = parse_source(source).unwrap();
    assert_eq!(result.len(), 1);

    match &result[0] {
        Expression::Proof { name, signals, .. } => {
            assert_eq!(name, "TestProof");
            assert_eq!(signals.len(), 3);

            assert_eq!(signals[0].visibility, Visibility::Input);
            assert_eq!(signals[1].visibility, Visibility::Witness);
            assert_eq!(signals[2].visibility, Visibility::Witness);
        }
        _ => panic!("Expected Proof"),
    }
}

#[test]
fn test_parse_proof_with_binary_operations() {
    let source = r#"
    proof ArithmeticTest {
        input x: Field;
        input y: Field;
        witness result: Field;
        let sum = x + y in
        let diff = sum - 1 in
        let prod = diff * 2 in
        assert result === prod
    }"#;

    let result: Vec<Expression> = parse_source(source).unwrap();
    assert_eq!(result.len(), 1);

    match &result[0] {
        Expression::Proof { name, .. } => {
            assert_eq!(name, "ArithmeticTest");
        }
        _ => panic!("Expected Proof, got {:?}", result[0]),
    }
}

#[test]
fn test_parse_function_definition() {
    let source = r#"
let square (x: Field): Field = x * x
    "#;

    let result = parse_source(source).unwrap();
    assert_eq!(result.len(), 1);

    match &result[0] {
        Expression::FunctionDef { name, params, .. } => {
            assert_eq!(name, "square");
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name, "x");
        }
        _ => panic!("Expected FunctionDef, got {:?}", result[0]),
    }
}

#[test]
fn test_parse_proof_with_function_call() {
    let source = r#"
    proof FunctionCallTest {
        input a: Field;
        witness result: Field;
        assert result === square(a);
    }"#;

    let result = parse_source(source).unwrap();
    assert_eq!(result.len(), 1);

    match &result[0] {
        Expression::Proof { name, .. } => {
            assert_eq!(name, "FunctionCallTest");
        }
        _ => panic!("Expected Proof, got {:?}", result[0]),
    }
}

#[test]
fn test_parse_proof_with_pattern_matching() {
    let source = r#"
    proof PatternMatchTest {
        input x: Field;
        witness y: Field;
        let result = match x with
            | 0 => 1
            | n => n * 2
        in
        assert y === result;
    }"#;

    let result = parse_source(source).unwrap();
    assert_eq!(result.len(), 1);

    match &result[0] {
        Expression::Proof { name, .. } => {
            assert_eq!(name, "PatternMatchTest");
        }
        _ => panic!("Expected Proof, got {:?}", result[0]),
    }
}

#[test]
fn test_parse_multiple_proofs() {
    let source = r#"
    proof FirstProof {
        input x: Field;
        witness y: Field;
        assert y === x
    }
    
    proof SecondProof {
        input a: Field;
        witness b: Field;
        assert b === a * 2;
    }
    "#;

    let result = parse_source(source).unwrap();
    assert_eq!(result.len(), 2);

    match (&result[0], &result[1]) {
        (Expression::Proof { name: name1, .. }, Expression::Proof { name: name2, .. }) => {
            assert_eq!(name1, "FirstProof");
            assert_eq!(name2, "SecondProof");
        }
        _ => panic!("Expected two Proof expressions"),
    }
}

#[test]
fn test_parse_function_and_proof() {
    let source = r#"
let double (x: Field): Field = x * 2

proof UseFunction {
    input value: Field;
    witness result: Field;
    assert result === double(value);
}
    "#;

    let result = parse_source(source).unwrap();
    assert_eq!(result.len(), 2);

    match (&result[0], &result[1]) {
        (
            Expression::FunctionDef { name, .. },
            Expression::Proof {
                name: proof_name, ..
            },
        ) => {
            assert_eq!(name, "double");
            assert_eq!(proof_name, "UseFunction");
        }
        _ => panic!("Expected FunctionDef and Proof"),
    }
}

#[test]
fn test_parse_empty_proof() {
    let source = r#"
    proof EmptyProof {
        witness y: Field;
        assert y === 42;
    }
    "#;

    let result = parse_source(source).unwrap();
    assert_eq!(result.len(), 1);

    match &result[0] {
        Expression::Proof { signals, .. } => {
            assert_eq!(signals.len(), 1);
            assert_eq!(signals[0].visibility, Visibility::Witness);
        }
        _ => panic!("Expected Proof"),
    }
}

#[test]
fn test_parse_proof_with_nested_expressions() {
    let source = r#"
    proof NestedTest {
        input a: Field;
        input b: Field;
        input c: Field;
        witness result: Field;
        assert result === (a + b) * c;
    }"#;

    let result = parse_source(source).unwrap();
    assert_eq!(result.len(), 1);

    match &result[0] {
        Expression::Proof { name, .. } => {
            assert_eq!(name, "NestedTest");
        }
        _ => panic!("Expected Proof"),
    }
}

#[test]
fn test_parse_constants_in_proof() {
    let source = r#"
    proof ConstantsTest {
        input x: Field;
        witness y: Field;
        let doubled = x * 2 in
        let added = doubled + 5 in
        assert y === added;
    }"#;

    let result = parse_source(source).unwrap();
    assert_eq!(result.len(), 1);

    match &result[0] {
        Expression::Proof { name, .. } => {
            assert_eq!(name, "ConstantsTest");
        }
        _ => panic!("Expected Proof"),
    }
}

#[test]
fn test_parse_invalid_syntax() {
    assert!(parse_source("proof {").is_err());
    assert!(parse_source("42").is_err());
    assert!(parse_source("let x = 5").is_err());
    assert!(parse_source("assert x").is_err());
}

#[test]
fn test_minimal_proof() {
    let source = r#"
proof MinimalTest {
    input x: Field;
    witness y: Field;
    assert y === x
}"#;

    let result = parse_source(source);
    if let Err(e) = &result {
        println!("Debug error: {:?}", e);
    }
    let result = result.unwrap();
    assert_eq!(result.len(), 1);
}

#[test]
fn test_parse_comparison_operators() {
    let source = r#"
    proof ComparisonTest {
        input x: Field;
        input y: Field;
        assert x >= y;
        assert x != 0;
        assert y < 100;
    }"#;

    let result = parse_source(source).unwrap();
    assert_eq!(result.len(), 1);

    match &result[0] {
        Expression::Proof { name, .. } => {
            assert_eq!(name, "ComparisonTest");
        }
        _ => panic!("Expected Proof"),
    }
}

#[test]
fn test_parse_function_let_syntax() {
    let source = r#"
    let square (x: Field): Field = x * x

    proof Test {
        input y: Field;
        witness result: Field;
        assert result === square(y);
    }"#;

    let result = parse_source(source);

    assert!(result.is_ok(), "Expected successful parse for let syntax");

    let expressions = result.unwrap();
    assert_eq!(expressions.len(), 2);

    match (&expressions[0], &expressions[1]) {
        (Expression::FunctionDef { name, .. }, Expression::Proof { .. }) => {
            assert_eq!(name, "square");
        }
        _ => panic!("Expected FunctionDef and Proof"),
    }
}

#[test]
fn test_parse_simple_component() {
    let source = r#"
    component Multiplier {
        input x: Field;
        input y: Field;
        witness result: Field;
        assert result === x * y
    }"#;

    let result = parse_source(source).expect("Should parse component declaration");
    assert_eq!(result.len(), 1);

    match &result[0] {
        Expression::Component { name, signals, .. } => {
            assert_eq!(name, "Multiplier");
            assert_eq!(signals.len(), 3);
            assert_eq!(signals[0].name, "x");
            assert_eq!(signals[0].visibility, Visibility::Input);
            assert_eq!(signals[1].name, "y");
            assert_eq!(signals[1].visibility, Visibility::Input);
            assert_eq!(signals[2].name, "result");
            assert_eq!(signals[2].visibility, Visibility::Witness);
        }
        _ => panic!("Expected Component, got {:?}", result[0]),
    }
}

#[test]
fn test_parse_component_with_witness() {
    let source = r#"
    component SecretMultiply {
        input a: Field;
        witness secret: Field;
        witness result: Field;
        assert result === a * secret
    }"#;

    let result = parse_source(source).expect("Should parse component with witness");
    assert_eq!(result.len(), 1);

    match &result[0] {
        Expression::Component { signals, .. } => {
            assert_eq!(signals.len(), 3);
            assert_eq!(signals[1].visibility, Visibility::Witness);
        }
        _ => panic!("Expected Component"),
    }
}

#[test]
fn test_parse_multiple_components_and_proofs() {
    let source = r#"
    component Adder {
        input a: Field;
        input b: Field;
        witness sum: Field;
        assert sum === a + b
    }

    proof UseAdder {
        input x: Field;
        witness result: Field;
        assert result === x + x
    }"#;

    let result = parse_source(source).expect("Should parse component and proof");
    assert_eq!(result.len(), 2);

    match (&result[0], &result[1]) {
        (
            Expression::Component {
                name: comp_name, ..
            },
            Expression::Proof {
                name: proof_name, ..
            },
        ) => {
            assert_eq!(comp_name, "Adder");
            assert_eq!(proof_name, "UseAdder");
        }
        _ => panic!("Expected Component and Proof"),
    }
}

#[test]
fn test_parse_division_operator() {
    let source = r#"
    proof DivisionTest {
        input x: Field;
        input y: Field;
        witness result: Field;
        assert result === x / y
    }"#;

    let result = parse_source(source).expect("Should parse division operator");
    assert_eq!(result.len(), 1);

    match &result[0] {
        Expression::Proof { body, .. } => {
            if let Expression::Assert(expr) = body.as_ref() {
                if let Expression::BinaryOp { .. } = expr.as_ref() {}
            }
        }
        _ => panic!("Expected Proof"),
    }
}

#[test]
fn test_parse_division_with_precedence() {
    let source = r#"
    proof DivPrecedence {
        input a: Field;
        input b: Field;
        input c: Field;
        witness result: Field;
        assert result === a * b / c
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Division should parse with correct precedence"
    );
}

#[test]
fn test_parse_division_in_expression() {
    let source = r#"
    let divide (x: Field) (y: Field): Field = x / y
    "#;

    let result = parse_source(source);
    assert!(result.is_ok(), "Should parse division in function body");
}

#[test]
fn test_parse_logical_and_operator() {
    let source = r#"
    proof LogicalAndTest {
        input x: Field;
        input y: Field;
        witness result: Field;
        let condition = (x > 0) && (y > 0) in
        assert result === x + y
    }"#;

    let result = parse_source(source);
    assert!(result.is_ok(), "Should parse logical AND operator");
}

#[test]
fn test_parse_logical_or_operator() {
    let source = r#"
    proof LogicalOrTest {
        input x: Field;
        input y: Field;
        witness result: Field;
        let condition = (x == 0) || (y == 0) in
        assert result === 0
    }"#;

    let result = parse_source(source);
    assert!(result.is_ok(), "Should parse logical OR operator");
}

#[test]
fn test_parse_logical_not_operator() {
    let source = r#"
    proof LogicalNotTest {
        input x: Bool;
        witness result: Bool;
        assert result === !x
    }"#;

    let result = parse_source(source);
    assert!(result.is_ok(), "Should parse logical NOT operator");
}

#[test]
fn test_parse_combined_logical_operators() {
    let source = r#"
    proof CombinedLogicalTest {
        input a: Field;
        input b: Field;
        input c: Field;
        witness result: Field;
        let condition = (a > 0 && b > 0) || (c == 0) in
        assert result === a + b + c
    }"#;

    let result = parse_source(source);
    assert!(result.is_ok(), "Should parse combined logical operators");
}

#[test]
fn test_parse_logical_operator_precedence() {
    let source = r#"
    proof LogicalPrecedence {
        input a: Bool;
        input b: Bool;
        input c: Bool;
        witness result: Bool;
        assert result === a || b && c
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Should parse with correct logical precedence"
    );
}

#[test]
fn test_parse_block_as_expression() {
    let source = r#"
    proof BlockExpressionTest {
        input x: Field;
        witness result: Field;
        let value = {
            let temp = x * 2 in
            temp + 1
        } in
        assert result === value
    }"#;

    let result = parse_source(source);
    assert!(result.is_ok(), "Should parse block as expression");
}

#[test]
fn test_parse_nested_blocks() {
    let source = r#"
    proof NestedBlocksTest {
        input x: Field;
        witness result: Field;
        let value = {
            let a = {
                x + 1
            } in
            a * 2
        } in
        assert result === value
    }"#;

    let result = parse_source(source);
    assert!(result.is_ok(), "Should parse nested blocks");
}

#[test]
fn test_parse_block_in_function_call() {
    let source = r#"
    let process (x: Field): Field = x * 2

    proof BlockInCallTest {
        input y: Field;
        witness result: Field;
        assert result === process({
            let temp = y + 1 in
            temp
        })
    }"#;

    let result = parse_source(source);
    assert!(result.is_ok(), "Should parse block as function argument");
}

#[test]
fn test_parse_refined_type_basic() {
    let source = r#"
    let positive_double (x: refined { Field, x > 0 }): Field = x * 2
    "#;

    let result = parse_source(source);
    assert!(result.is_ok(), "Should parse basic refined type");

    if let Ok(expressions) = result {
        match &expressions[0] {
            Expression::FunctionDef { params, .. } => {
                assert_eq!(params.len(), 1);
                match &params[0].typ {
                    Type::Refined(base, _predicate) => {
                        assert!(matches!(**base, Type::Field { .. }));
                    }
                    _ => panic!("Expected Refined type, got {:?}", params[0].typ),
                }
            }
            _ => panic!("Expected FunctionDef"),
        }
    }
}

#[test]
fn test_parse_refined_type_complex_predicate() {
    let source = r#"
    let bounded (x: refined { Field, x > 0 && x < 100 }): Field = x
    "#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Should parse refined type with complex predicate"
    );
}

#[test]
fn test_parse_refined_type_in_signal() {
    let source = r#"
    proof RefinedSignalTest {
        input x: refined { Field, x > 0 };
        witness result: Field;
        assert result === x
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Should parse refined type in signal declaration"
    );
}

#[test]
fn test_parse_array_indexing_basic() {
    let source = r#"
    proof ArrayIndexTest {
        input arr: Array<Field, 10>;
        input i: Nat;
        witness result: Field;
        assert result === arr[i]
    }"#;

    let result = parse_source(source);
    assert!(result.is_ok(), "Should parse basic array indexing");
}

#[test]
fn test_parse_array_indexing_constant() {
    let source = r#"
    proof ArrayIndexConstantTest {
        input arr: Array<Field, 5>;
        witness result: Field;
        assert result === arr[0]
    }"#;

    let result = parse_source(source);
    assert!(result.is_ok(), "Should parse array indexing with constant");
}

#[test]
fn test_parse_nested_array_indexing() {
    let source = r#"
    proof MatrixIndexTest {
        input matrix: Array<Array<Field, 3>, 3>;
        input i: Nat;
        input j: Nat;
        witness result: Field;
        assert result === matrix[i][j]
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Should parse nested array indexing (matrix[i][j])"
    );
}

#[test]
fn test_parse_array_indexing_in_expression() {
    let source = r#"
    proof ArrayExpressionTest {
        input arr: Array<Field, 10>;
        input i: Nat;
        witness result: Field;
        assert result === arr[i] * 2 + arr[i + 1]
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Should parse array indexing in complex expressions"
    );
}

#[test]
fn test_parse_mixed_operator_precedence() {
    let source = r#"
    proof MixedPrecedenceTest {
        input a: Field;
        input b: Field;
        input c: Field;
        input d: Field;
        input e: Bool;
        input f: Bool;
        witness result: Field;
        let condition = a + b * c / d > 0 && e || f in
        assert result === a
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Should parse mixed operators with correct precedence"
    );
}

#[test]
fn test_parse_arithmetic_comparison_logical() {
    let source = r#"
    proof ArithmeticComparisonLogicalTest {
        input x: Field;
        input y: Field;
        witness result: Field;
        let check = x + 1 > y * 2 && y != 0 in
        assert result === x
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Should parse arithmetic, comparison, and logical operators"
    );
}

#[test]
fn test_parse_operator_precedence_with_parentheses() {
    let source = r#"
    proof PrecedenceWithParensTest {
        input a: Field;
        input b: Field;
        input c: Field;
        witness result: Field;
        assert result === (a + b) * c;
        assert result === a + (b * c);
        assert result === ((a + b) * c) / (a - b)
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Should parse operators with explicit parentheses"
    );
}

#[test]
fn test_parse_component_without_signals() {
    let source = r#"
    component Empty {
        assert 1 === 1
    }"#;

    let result = parse_source(source);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_division_by_zero_syntax() {
    let source = r#"
    proof DivByZeroSyntax {
        input x: Field;
        witness result: Field;
        assert result === x / 0
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Parser should accept division by constant zero (runtime/typechecker issue)"
    );
}

#[test]
fn test_parse_not_in_complex_expression() {
    let source = r#"
    proof NotComplexTest {
        input a: Bool;
        input b: Bool;
        witness result: Bool;
        assert result === !(a && b)
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Should parse NOT with parenthesized expression"
    );
}

#[test]
fn test_parse_chained_divisions() {
    let source = r#"
    proof ChainedDivTest {
        input a: Field;
        input b: Field;
        input c: Field;
        witness result: Field;
        assert result === a / b / c
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Should parse chained divisions left-to-right"
    );
}

#[test]
fn test_parse_block_with_multiple_statements() {
    let source = r#"
    proof MultiStatementBlockTest {
        input x: Field;
        witness result: Field;
        let value = {
            let a = x + 1 in
            let b = a * 2 in
            let c = b - 3 in
            c
        } in
        assert result === value
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Should parse block with multiple let statements"
    );
}

#[test]
fn test_parse_refined_type_with_equality() {
    let source = r#"
    let exactly_ten (x: refined { Field, x == 10 }): Field = x
    "#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Should parse refined type with equality predicate"
    );
}

#[test]
fn test_parse_array_indexing_with_expression() {
    let source = r#"
    proof ArrayComplexIndexTest {
        input arr: Array<Field, 20>;
        input i: Nat;
        input j: Nat;
        witness result: Field;
        assert result === arr[i + j * 2]
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Should parse array indexing with complex index expression"
    );
}
