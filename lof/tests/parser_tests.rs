use lof::parser::Parser;
use lof::lexer::Lexer;
use lof::ast::{Expression, Operator, Visibility};

fn parse_source(source: &str) -> Result<Vec<Expression>, String> {
    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);
    
    parser.parse_program()
        .map_err(|e| format!("Parse error: {:?}", e))
}

#[test]
fn test_parse_simple_proof() {
    let source = r#"
    proof SimpleProof {
        input x: Field;
        output y: Field;
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
            assert_eq!(signals[1].visibility, Visibility::Output);
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
        output result: Field;
        
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
            assert_eq!(signals[2].visibility, Visibility::Output);
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
        output result: Field;
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
fn test_parse_proof_with_dup() {
    let source = r#"
    proof DupTest {
        input x: Field;
        output result: Field;
        let x_copy = dup(x) in
        let a = x_copy * 2 in
        let b = x_copy * 3 in
        assert result === a + b;
    }"#;
    
    let result = parse_source(source).unwrap();
    assert_eq!(result.len(), 1);
    
    match &result[0] {
        Expression::Proof { name, .. } => {
            assert_eq!(name, "DupTest");
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
        output result: Field;
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
        output y: Field;
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
        output y: Field;
        assert y === x
    }
    
    proof SecondProof {
        input a: Field;
        output b: Field;
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
    output result: Field;
    assert result === double(value);
}
    "#;
    
    let result = parse_source(source).unwrap();
    assert_eq!(result.len(), 2);
    
    match (&result[0], &result[1]) {
        (Expression::FunctionDef { name, .. }, Expression::Proof { name: proof_name, .. }) => {
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
        output y: Field;
        assert y === 42;
    }
    "#;
    
    let result = parse_source(source).unwrap();
    assert_eq!(result.len(), 1);
    
    match &result[0] {
        Expression::Proof { signals, .. } => {
            assert_eq!(signals.len(), 1);
            assert_eq!(signals[0].visibility, Visibility::Output);
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
        output result: Field;
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
        output y: Field;
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
    // Test various invalid syntax cases that should fail
    assert!(parse_source("proof {").is_err()); // Missing name
    assert!(parse_source("42").is_err()); // Bare expression not allowed at top level
    assert!(parse_source("let x = 5").is_err()); // Incomplete let binding
    assert!(parse_source("assert x").is_err()); // Bare assert not allowed at top level
}

#[test]
fn test_minimal_proof() {
    let source = r#"
proof MinimalTest {
    input x: Field;
    output y: Field;
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
    // Test the supported let function syntax
    let source = r#"
    let square (x: Field): Field = x * x
    
    proof Test {
        input y: Field;
        output result: Field;
        assert result === square(y);
    }"#;
    
    let result = parse_source(source);
    
    // This should work
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