use lof::typechecker::{TypeChecker, TypeError};
use lof::parser::Parser;
use lof::lexer::Lexer;
use lof::ast::{Expression, Type, LinearityKind};

fn parse_and_type_check(source: &str) -> Result<(), TypeError> {
    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);
    
    let ast = parser.parse_program()
        .map_err(|_| TypeError::InvalidExpression)?;
    
    let mut type_checker = TypeChecker::new();
    type_checker.check_program(&ast)
}

fn type_check_passes(source: &str) -> bool {
    parse_and_type_check(source).is_ok()
}

fn type_check_fails_with_consumed_error(source: &str) -> bool {
    match parse_and_type_check(source) {
        Err(TypeError::VariableAlreadyConsumed(_)) => true,
        Err(TypeError::VariableAlreadyConsumedAt { .. }) => true,
        _ => false,
    }
}

fn type_check_fails_with_undefined_error(source: &str) -> bool {
    match parse_and_type_check(source) {
        Err(TypeError::UndefinedVariable(_)) => true,
        _ => false,
    }
}

#[test]
fn test_simple_variable_usage() {
    // Basic variable usage should pass
    let source = r#"
    proof Test {
        input x: Field;
        output y: Field;
        assert y === x;
    }"#;
    assert!(type_check_passes(source));
}

#[test]
fn test_linear_consumption_basic() {
    // Using a linear variable once should pass
    let source = r#"
    proof Test {
        input x: Field;
        output y: Field;
        let temp = x * 2 in
        assert y === temp;
    }"#;
    assert!(type_check_passes(source));
}

#[test] 
fn test_linear_consumption_violation() {
    // Using a linear variable twice should fail
    let source = r#"
    proof Test {
        input x: Field;
        output y: Field;
        let a = x * 2 in
        let b = x * 3 in
        assert y === a + b;
    }"#;
    assert!(type_check_fails_with_consumed_error(source));
}

#[test]
fn test_dup_makes_copyable() {
    // Using dup should allow multiple uses
    let source = r#"
    proof Test {
        input x: Field;
        output y: Field;
        let x_copy = dup(x) in
        let a = x_copy * 2 in
        let b = x_copy * 3 in
        assert y === a + b;
    }"#;
    assert!(type_check_passes(source));
}

#[test]
fn test_constants_are_copyable() {
    // Constants should be usable multiple times
    let source = r#"
    proof Test {
        output y: Field;
        let a = 5 * 2 in
        let b = 5 * 3 in
        assert y === a + b;
    }"#;
    assert!(type_check_passes(source));
}

#[test]
fn test_undefined_variable() {
    let source = r#"
    proof Test {
        output y: Field;
        assert y === undefined_var;
    }"#;
    assert!(type_check_fails_with_undefined_error(source));
}

#[test]
fn test_function_parameter_consumption() {
    let source = r#"
    let square (x: Field): Field = x * x
    
    proof Test {
        input y: Field;
        output result: Field;
        assert result === square(y);
    }"#;
    assert!(type_check_fails_with_consumed_error(source));
}

#[test]
fn test_nested_let_bindings() {
    let source = r#"
    proof Test {
        input x: Field;
        output y: Field;
        let a = x * 2 in
        let b = a * 3 in
        assert y === b;
    }"#;
    assert!(type_check_passes(source));
}

#[test]
fn test_arithmetic_operations() {
    let source = r#"
    proof Test {
        input a: Field;
        input b: Field;
        output result: Field;
        let sum = a + b in
        let diff = sum - 1 in
        let prod = diff * 2 in
        assert result === prod;
    }"#;
    assert!(type_check_passes(source));
}

#[test]
fn test_assert_boolean_type() {
    // Assert should require boolean expressions
    let source = r#"
    proof Test {
        input x: Field;
        input y: Field;
        assert x == y;
    }"#;
    assert!(type_check_passes(source));
}

#[test]
fn test_variable_scoping_in_let() {
    let source = r#"
    proof Test {
        input x: Field;
        output y: Field;
        let x = 42 in
        assert y === x;
    }"#;
    assert!(type_check_passes(source));
}

#[test]
fn test_pattern_matching_basic() {
    let source = r#"
    proof Test {
        input x: Field;
        output y: Field;
        let result = match x with
            | 0 => 1
            | n => n * 2
        in
        assert y === result;
    }"#;
    assert!(type_check_passes(source));
}

#[test]
fn test_multiple_inputs_and_witnesses() {
    let source = r#"
    proof Test {
        input a: Field;
        input b: Field;
        witness w: Field;
        output result: Field;
        let temp = a * w in
        let sum = temp + b in
        assert result === sum;
    }"#;
    assert!(type_check_passes(source));
}

#[test]
fn test_invalid_dup_on_consumed() {
    let source = r#"
    proof Test {
        input x: Field;
        output y: Field;
        let temp = x * 2 in
        let x_copy = dup(x) in
        assert y === temp + x_copy;
    }"#;
    assert!(type_check_fails_with_consumed_error(source));
}

#[test]
fn test_comparison_operators() {
    let source = r#"
    proof Test {
        input x: Field;
        input y: Field;
        assert x >= y;
    }"#;
    assert!(type_check_passes(source));
}

#[test]
fn test_error_messages_contain_variable_name() {
    match parse_and_type_check(r#"
    proof Test {
        input x: Field;
        let a = x * 2 in
        let b = x * 3 in
        a + b
    }"#) {
        Err(TypeError::VariableAlreadyConsumed(name)) => {
            assert_eq!(name, "x");
        }
        Err(TypeError::VariableAlreadyConsumedAt { name, .. }) => {
            assert_eq!(name, "x");
        }
        result => panic!("Expected consumed error with variable name, got: {:?}", result),
    }
}

#[test]
fn test_linear_type_enforcement() {
    // Test that linear types are properly enforced
    let source = r#"
    proof LinearTest {
        input linear_var: Field;
        output result: Field;
        
        let first_use = linear_var + 1 in
        let second_use = linear_var + 2 in  // Should fail here
        assert result === first_use + second_use;
    }"#;
    
    assert!(type_check_fails_with_consumed_error(source));
}

#[test]
fn test_dup_correctness() {
    // Test that dup properly converts linear to copyable
    let source = r#"
    proof DupTest {
        input x: Field;
        output result: Field;
        
        let x_dup = dup(x) in
        let use1 = x_dup * 2 in
        let use2 = x_dup * 3 in
        let use3 = x_dup * 4 in  // Should work since x_dup is copyable
        assert result === use1 + use2 + use3;
    }"#;
    
    assert!(type_check_passes(source));
}

#[test]
fn test_function_linearity() {
    // Test that function parameters follow linearity rules
    let source = r#"
    let double_use (x: Field): Field = x + x  // Should fail - using x twice
    
    proof FunctionLinearityTest {
        input y: Field;
        output result: Field;
        assert result === double_use(y);
    }"#;
    
    assert!(type_check_fails_with_consumed_error(source));
}

#[test]
fn test_multiple_consumption_in_assertions() {
    // Test replicating the bug where a witness variable is consumed multiple times
    let source = r#"
    proof TestProof {
        input threshold: Field;
        input hash: Field;
        witness value: Field;
        witness salt: Field;
        
        // First consumption in comparison
        assert value >= threshold;
        
        // Second consumption in arithmetic operation
        let computed_hash = value + salt in
        assert hash === computed_hash;
        
        // Third consumption in another comparison
        assert value >= 0;
    }"#;
    
    // Test should pass - let's see what error we get
    match parse_and_type_check(source) {
        Ok(()) => println!("SUCCESS: Test passed!"),
        Err(e) => {
            println!("ERROR: {:?}", e);
            panic!("Test failed with error: {:?}", e);
        }
    }
}

#[test]
fn test_assert_should_not_consume_variables() {
    // Variables used in assert statements should NOT be consumed
    // This test shows the bug where assert incorrectly consumes variables
    let source = r#"
    proof TestProof {
        witness x: Field;
        
        // These should all be non-consuming reads, not consumption
        assert x >= 0;
        assert x >= 0;  // Should be allowed - same variable, multiple asserts
    }"#;
    
    // This should PASS because assert should not consume variables
    // But currently fails due to the bug
    assert!(type_check_passes(source));
}