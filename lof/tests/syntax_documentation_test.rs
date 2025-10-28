use lof::ast::Expression;
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
fn test_simple_proof_from_docs() {
    let source = r#"
    proof ProofName {
        input x: Field;
        witness secret: Field;
        witness result: Field;

        assert x > 0;
        result === x + secret
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse simple proof from docs: {:?}",
        result.err()
    );
}

#[test]
fn test_component_definition() {
    let source = r#"
    component Square {
        input x: Field;
        witness y: Field;

        y === x * x
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse component: {:?}",
        result.err()
    );
}

#[test]
fn test_component_instantiation_in_proof() {
    let source = r#"
    component Multiplier {
        input a: Field;
        input b: Field;
        witness c: Field;

        c === a * b
    }

    proof UseMultiplier {
        witness x: Field;
        witness y: Field;
        witness result: Field;

        result === Multiplier(x, y)
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse component instantiation: {:?}",
        result.err()
    );
}

#[test]
fn test_all_signal_types() {
    let source = r#"
    proof SignalTypes {
        input publicValue: Field;
        witness privateData: Field;
        witness result: Bool;

        result === publicValue > privateData
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse signal types: {:?}",
        result.err()
    );
}

#[test]
fn test_primitive_types() {
    let source = r#"
    proof PrimitiveTypes {
        input f: Field;
        input b: Bool;
        input n: Nat;
        witness result: Field;

        result === f
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse primitive types: {:?}",
        result.err()
    );
}

#[test]
fn test_array_type() {
    let source = r#"
    proof ArrayTest {
        input arr: Array<Field, 10>;
        witness result: Field;

        result === arr[0]
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse array type: {:?}",
        result.err()
    );
}

#[test]
fn test_tuple_type() {
    let source = r#"
    proof TupleTest {
        input pair: (Field, Bool);
        input triple: (Field, Field, Bool);
        witness result: Field;

        result === 0
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse tuple types: {:?}",
        result.err()
    );
}

#[test]
fn test_refined_type() {
    let source = r#"
    let positive (x: refined { Field, x > 0 && x < 100 }): Field = x
    "#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse refined type: {:?}",
        result.err()
    );
}

#[test]
fn test_curried_functions() {
    let source = r#"
    let add (x: Field) (y: Field): Field = x + y

    proof CurriedTest {
        input a: Field;
        witness result: Field;

        let increment = add(1) in
        let five = increment(4) in
        result === five
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse curried functions: {:?}",
        result.err()
    );
}

#[test]
fn test_let_binding_with_pattern() {
    let source = r#"
    proof LetBindingTest {
        input x: Field;
        witness result: Field;

        let y = 42 in
        let z = y + 10 in
        result === x + z
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse let bindings: {:?}",
        result.err()
    );
}

#[test]
fn test_tuple_destructuring() {
    let source = r#"
    proof TupleDestructure {
        input pair: (Field, Field);
        witness result: Field;

        let (a, b) = pair in
        result === a + b
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse tuple destructuring: {:?}",
        result.err()
    );
}

#[test]
fn test_all_pattern_types() {
    let source = r#"
    proof PatternTest {
        input x: Field;
        witness result: Field;

        let value = match x with
            | 0 => 1
            | 1 => 2
            | n => n * 2
        in
        result === value
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse patterns: {:?}",
        result.err()
    );
}

#[test]
fn test_wildcard_pattern() {
    let source = r#"
    proof WildcardTest {
        input x: Field;
        witness result: Field;

        let value = match x with
            | 42 => 100
            | _ => 0
        in
        result === value
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse wildcard pattern: {:?}",
        result.err()
    );
}

#[test]
fn test_all_arithmetic_operators() {
    let source = r#"
    proof ArithmeticTest {
        input a: Field;
        input b: Field;
        witness result: Field;

        let sum = a + b in
        let diff = a - b in
        let prod = a * b in
        let quot = a / b in
        result === sum + diff + prod + quot
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse arithmetic operators: {:?}",
        result.err()
    );
}

#[test]
fn test_all_comparison_operators() {
    let source = r#"
    proof ComparisonTest {
        input x: Field;
        input y: Field;
        witness result: Bool;

        assert x == y;
        assert x != y;
        assert x < y;
        assert x > y;
        assert x <= y;
        assert x >= y;
        result === x == y
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse comparison operators: {:?}",
        result.err()
    );
}

#[test]
fn test_all_logical_operators() {
    let source = r#"
    proof LogicalTest {
        input a: Bool;
        input b: Bool;
        witness result: Bool;

        let and_result = a && b in
        let or_result = a || b in
        let not_result = !a in
        result === and_result || or_result || not_result
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse logical operators: {:?}",
        result.err()
    );
}

#[test]
fn test_constraint_equality() {
    let source = r#"
    proof ConstraintTest {
        input x: Field;
        witness y: Field;

        y === x * 2
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse constraint equality: {:?}",
        result.err()
    );
}

#[test]
fn test_operator_precedence() {
    let source = r#"
    proof PrecedenceTest {
        input a: Field;
        input b: Field;
        input c: Field;
        witness result: Field;

        result === a + b * c
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse operator precedence: {:?}",
        result.err()
    );
}

#[test]
fn test_parentheses_override_precedence() {
    let source = r#"
    proof ParenTest {
        input a: Field;
        input b: Field;
        input c: Field;
        witness result: Field;

        result === (a + b) * c
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse parentheses: {:?}",
        result.err()
    );
}

#[test]
fn test_assertions() {
    let source = r#"
    proof AssertionTest {
        input balance: Field;
        input amount: Field;
        witness result: Bool;

        assert balance >= amount;
        assert amount > 0;
        assert amount < 100;
        result === balance > amount
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse assertions: {:?}",
        result.err()
    );
}

#[test]
fn test_block_expressions() {
    let source = r#"
    proof BlockTest {
        input x: Field;
        witness result: Field;

        let value = {
            let temp1 = x + 1 in
            let temp2 = temp1 * 2 in
            temp2
        } in
        result === value
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse block expressions: {:?}",
        result.err()
    );
}

#[test]
fn test_array_indexing() {
    let source = r#"
    proof ArrayIndexTest {
        input arr: Array<Field, 3>;
        witness result: Field;

        let first = arr[0] in
        result === first
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse array indexing: {:?}",
        result.err()
    );
}

#[test]
fn test_complete_example_from_docs() {
    let source = r#"
    let square (x: Field): Field = x * x

    proof RangeProof {
        input value: Field;
        witness valid: Bool;

        assert value < 256;
        valid === value > 0
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse complete example: {:?}",
        result.err()
    );
}

#[test]
fn test_multiple_assertions() {
    let source = r#"
    proof MultipleAssertions {
        input x: Field;
        witness result: Field;

        assert x > 0;
        assert x < 100;
        result === x
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse multiple assertions: {:?}",
        result.err()
    );
}

#[test]
fn test_complex_pattern_matching() {
    let source = r#"
    proof ComplexMatch {
        input x: Field;
        witness result: Field;

        let value = match x with
            | 0 => 1
            | 1 => 2
            | 2 => 3
            | _ => x * 2
        in
        result === value
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse complex pattern matching: {:?}",
        result.err()
    );
}

#[test]
fn test_nested_expressions() {
    let source = r#"
    proof NestedExpressions {
        input a: Field;
        input b: Field;
        input c: Field;
        witness result: Field;

        let x = a + b * c in
        let y = (a + b) * c in
        let z = a * b + c in
        result === x + y + z
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse nested expressions: {:?}",
        result.err()
    );
}

#[test]
fn test_tuple_construction() {
    let source = r#"
    proof TupleConstruction {
        input x: Field;
        input y: Field;
        witness result: Field;

        let pair = (x, y) in
        let triple = (x, y, x) in
        result === x
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse tuple construction: {:?}",
        result.err()
    );
}

#[test]
fn test_empty_tuple() {
    let source = r#"
    proof EmptyTuple {
        input x: Field;
        witness result: Field;

        let unit = () in
        result === x
    }"#;

    let result = parse_source(source);
    assert!(
        result.is_ok(),
        "Failed to parse empty tuple: {:?}",
        result.err()
    );
}
