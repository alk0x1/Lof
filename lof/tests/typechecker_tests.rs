use lof::lexer::Lexer;
use lof::parser::Parser;
use lof::typechecker::{TypeChecker, TypeError};

fn parse_and_type_check(source: &str) -> Result<(), TypeError> {
    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);

    let ast = parser
        .parse_program()
        .map_err(|_| TypeError::InvalidExpression)?;

    let mut type_checker = TypeChecker::new();
    type_checker.check_program(&ast)
}

fn type_check_passes(source: &str) -> bool {
    parse_and_type_check(source).is_ok()
}

fn type_check_fails_with_undefined_error(source: &str) -> bool {
    matches!(
        parse_and_type_check(source),
        Err(TypeError::UndefinedVariable(_))
    )
}

fn type_check_fails_with_unconstrained_witness_error(source: &str) -> bool {
    matches!(
        parse_and_type_check(source),
        Err(TypeError::UnconstrainedWitness { .. })
    )
}

fn type_check_fails_with_nonzero_error(source: &str) -> bool {
    matches!(
        parse_and_type_check(source),
        Err(TypeError::NonZeroRequired { .. })
    )
}

fn type_check_fails_with_type_mismatch(source: &str) -> bool {
    matches!(
        parse_and_type_check(source),
        Err(TypeError::TypeMismatch { .. })
    )
}

#[test]
fn test_simple_variable_usage() {
    let source = r#"
    proof Test {
        input x: Field;
        witness y: Field;
        assert y === x;
    }"#;
    assert!(type_check_passes(source));
}

#[test]
fn test_linear_consumption_basic() {
    let source = r#"
    proof Test {
        input x: Field;
        witness y: Field;
        let temp = x * 2 in
        assert y === temp;
    }"#;
    assert!(type_check_passes(source));
}

#[test]
fn test_proof_block_allows_multi_use() {
    let source = r#"
    proof Test {
        input x: Field;
        witness y: Field;
        let a = x * 2 in
        let b = x * 3 in
        assert y === a + b;
    }"#;
    assert!(type_check_passes(source));
}

#[test]
fn test_undefined_variable() {
    let source = r#"
    proof Test {
        witness y: Field;
        assert y === undefined_var;
    }"#;
    assert!(type_check_fails_with_undefined_error(source));
}

#[test]
fn test_nested_let_bindings() {
    let source = r#"
    proof Test {
        input x: Field;
        witness y: Field;
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
        witness result: Field;
        let sum = a + b in
        let diff = sum - 1 in
        let prod = diff * 2 in
        assert result === prod;
    }"#;
    assert!(type_check_passes(source));
}

#[test]
fn test_assert_boolean_type() {
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
        witness y: Field;
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
        witness y: Field;
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
        witness result: Field;
        let temp = a * w in
        let sum = temp + b in
        assert result === sum;
    }"#;
    assert!(type_check_passes(source));
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
fn test_nested_let_in_proof() {
    let source = r#"
    proof Test {
        input x: Field;
        let doubled = x * 2 in
        let tripled = x * 3 in
        let sum = doubled + tripled in
        assert sum === x * 5
    }"#;
    assert!(type_check_passes(source));
}

#[test]
fn test_linear_type_enforcement() {
    let source = r#"
    proof LinearTest {
        input linear_var: Field;
        witness result: Field;

        let first_use = linear_var + 1 in
        let second_use = linear_var + 2 in
        assert result === first_use + second_use;
    }"#;

    assert!(type_check_passes(source));
}

#[test]
fn test_multiple_consumption_in_assertions() {
    let source = r#"
    proof TestProof {
        input threshold: Field;
        input hash: Field;
        witness value: Field;
        witness salt: Field;

        assert value >= threshold;

        let computed_hash = value + salt in
        assert hash === computed_hash;

        assert value >= 0;
    }"#;

    assert!(type_check_passes(source));
}

#[test]
fn test_assert_should_not_consume_variables() {
    let source = r#"
    proof TestProof {
        witness x: Field;

        assert x >= 0;
        assert x >= 0;
    }"#;

    assert!(type_check_passes(source));
}

#[test]
fn test_proof_block_multi_assertions() {
    let source = r#"
    proof RangeCheck {
        input value: Field;
        assert value >= 0;
        assert value < 100;
        assert value != 50;
    }"#;
    assert!(type_check_passes(source));
}

#[test]
fn test_witness_starts_unconstrained() {
    let source = r#"
    proof Test {
        input x: field;
        witness w: field;

        assert x > 0
    }
    "#;
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

#[test]
fn test_input_automatically_constrained() {
    let source = r#"
    proof Test {
        input x: field;

        assert x > 0
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_output_automatically_constrained() {
    let source = r#"
    proof Test {
        witness y: field;

        let temp = 42 in
        assert y === temp
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_multiple_witnesses_all_must_be_constrained() {
    let source = r#"
    proof Test {
        witness w1: field;
        witness w2: field;
        witness w3: field;

        let temp = w1 * 2 in
        assert temp === w2;

    }
    "#;
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

#[test]
fn test_witness_must_be_used_in_constraint() {
    let source = r#"
    proof Buggy {
        input x: field;
        witness forgotten: field;
        assert x > 0
    }
    "#;
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

#[test]
fn test_witness_used_only_in_addition_not_constrained() {
    let source = r#"
    proof Invalid {
        witness x: field;
        let y = x + 5 in
        assert y > 0
    }
    "#;
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

#[test]
fn test_witness_used_in_let_binding_not_constrained() {
    let source = r#"
    proof Invalid {
        witness x: field;
        let y = x in
        assert y > 0
    }
    "#;
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

#[test]
fn test_multiplication_constrains_witness() {
    let source = r#"
    proof Valid {
        witness x: field;
        let y = x * 2 in
        assert y > 0
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_multiplication_constrains_both_operands() {
    let source = r#"
    proof Valid {
        witness a: field;
        witness b: field;
        let product = a * b in
        assert product > 0
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_division_constrains_witness() {
    let source = r#"
    proof Valid {
        input divisor: field;
        witness x: field;
        assert divisor != 0;
        let result = x / divisor in
        assert result > 0
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_division_requires_nonzero_assertion() {
    let source = r#"
    proof Invalid {
        input divisor: field;
        witness x: field;
        let result = x / divisor in
        assert result > 0
    }
    "#;
    assert!(type_check_fails_with_nonzero_error(source));
}

#[test]
fn test_division_by_zero_literal_fails() {
    let source = r#"
    proof Invalid {
        witness x: field;
        let result = x / 0 in
        assert result > 0
    }
    "#;
    assert!(type_check_fails_with_nonzero_error(source));
}

#[test]
fn test_assertion_constrains_witness() {
    let source = r#"
    proof Valid {
        witness x: field;
        assert x > 0
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_assertion_constrains_all_variables() {
    let source = r#"
    proof Valid {
        witness a: field;
        witness b: field;
        witness c: field;
        assert a + b === c
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_equality_assertion_constrains_both_sides() {
    let source = r#"
    proof Valid {
        witness x: field;
        witness y: field;
        assert x === y
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_match_constrains_scrutinee() {
    let source = r#"
    proof Valid {
        witness x: field;
        let result = match x with
            | 0 => 1
            | n => n * 2
        in
        assert result > 0
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_nested_multiplication_constrains() {
    let source = r#"
    proof Valid {
        witness x: field;
        let temp = x * 2 in
        let final_val = temp + 100 in
        assert final_val > 0
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_addition_does_not_constrain() {
    let source = r#"
    proof Invalid {
        witness x: field;
        let y = x + 5 in
        assert y > 0
    }
    "#;
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

#[test]
fn test_subtraction_does_not_constrain() {
    let source = r#"
    proof Invalid {
        witness x: field;
        let y = x - 1 in
        assert y > 0
    }
    "#;
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

#[test]
fn test_comparison_does_not_constrain() {
    let source = r#"
    proof Invalid {
        witness x: field;
        witness y: field;
        input z: field;
        let is_greater = x > y in
        let result = z * z in
        assert result > 0
    }
    "#;

    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

#[test]
fn test_multiple_additions_still_unconstrained() {
    let source = r#"
    proof Invalid {
        witness x: field;
        let y = x + 1 in
        let z = y + 2 in
        let w = z + 3 in
        assert w > 0
    }
    "#;
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

#[test]
fn test_mimc_unconstrained_bug() {
    let source = r#"
    proof MiMCBroken {
        input x: field;
        witness intermediate: field;

        let hash = x + 1 in
        assert hash > 0
    }
    "#;
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

#[test]
fn test_mimc_fixed() {
    let source = r#"
    proof MiMCFixed {
        input x: field;
        witness intermediate: field;

        let hash = x + 1 in
        assert intermediate === hash
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_unconstrained_let_binding_in_proof_body() {
    let source = r#"
    proof Transfer {
        input balance: field;
        input amount: field;

        assert balance >= amount;

        let new_balance = balance - amount in
        new_balance
    }
    "#;
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

#[test]
fn test_constrained_let_binding_in_proof_body() {
    let source = r#"
    proof Transfer {
        input balance: field;
        input amount: field;

        assert balance >= amount;

        let new_balance = balance - amount in
        assert new_balance === balance - amount
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_let_binding_constrained_by_multiplication() {
    let source = r#"
    proof Square {
        input x: field;

        let squared = x * x in
        squared
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_nested_let_bindings_unconstrained() {
    let source = r#"
    proof Nested {
        input a: field;
        input b: field;

        let sum = a + b in
        let doubled = sum + sum in
        doubled
    }
    "#;
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

#[test]
fn test_range_proof_all_witnesses_constrained() {
    let source = r#"
    proof RangeProof {
        input value: field;
        witness bit_0: field;
        witness bit_1: field;
        witness bit_2: field;
        witness bit_3: field;

        assert bit_0 * (1 - bit_0) === 0;
        assert bit_1 * (1 - bit_1) === 0;
        assert bit_2 * (1 - bit_2) === 0;
        assert bit_3 * (1 - bit_3) === 0;

        assert value === bit_0 + 2 * bit_1 + 4 * bit_2 + 8 * bit_3
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_merkle_proof_all_witnesses_constrained() {
    let source = r#"
    proof MerkleProof {
        input leaf: field;
        input root: field;
        witness sibling_0: field;
        witness sibling_1: field;
        witness hash_0: field;
        witness hash_1: field;

        let sum_0 = leaf + sibling_0 in
        assert hash_0 === sum_0 * sum_0;

        let sum_1 = hash_0 + sibling_1 in
        assert hash_1 === sum_1 * sum_1;

        assert hash_1 === root
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_conditional_constraint() {
    let source = r#"
    proof Conditional {
        input x: field;
        input selector: field;
        witness result: field;

        assert selector * (1 - selector) === 0;

        let option_a = x * 2 in
        let option_b = x * 3 in
        let selected = selector * option_a + (1 - selector) * option_b in
        assert result === selected
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_bool_selector_arithmetic_passes() {
    let source = r#"
    proof Selector {
        input a: field;
        input b: field;
        witness choose_a: bool;
        witness result: field;

        let selected = choose_a * a + (1 - choose_a) * b in
        assert result === selected;
        assert choose_a == choose_a
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_assert_boolean_condition() {
    let source = r#"
    proof FlagCheck {
        witness flag: bool;
        assert flag;
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_unconstrained_bool_witness_errors() {
    let source = r#"
    proof Broken {
        witness flag: bool;
        assert 1 === 1;
    }
    "#;
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

#[test]
fn test_partial_constraint_chain() {
    let source = r#"
    proof PartiallyBroken {
        witness w1: field;
        witness w2: field;
        witness w3: field;

        let product = w1 * w2 in
        assert product > 0;

    }
    "#;
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

#[test]
fn test_witness_constrained_transitively() {
    let source = r#"
    proof Transitive {
        witness x: field;
        witness y: field;

        let temp = x * 2 in

        assert y === temp
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_addition_with_constrained_value_still_requires_constraint() {
    let source = r#"
    proof Tricky {
        input x: field;
        witness w: field;

        let sum = x + w in
        assert sum > 0
    }
    "#;

    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

#[test]
fn test_witness_only_in_witness_list() {
    let source = r#"
    proof EmptyWitness {
        witness unused: field;
        assert 1 > 0
    }
    "#;
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

#[test]
fn test_all_witnesses_must_be_constrained_individually() {
    let source = r#"
    proof MixedWitnesses {
        witness constrained_w: field;
        witness unconstrained_w: field;

        assert constrained_w > 0;

    }
    "#;
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

#[test]
fn test_witness_in_both_sides_of_multiplication() {
    let source = r#"
    proof SelfMultiply {
        witness x: field;
        let square = x * x in
        assert square > 0
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_complex_expression_constrains_all_witnesses() {
    let source = r#"
    proof ComplexExpression {
        witness a: field;
        witness b: field;
        witness c: field;
        witness d: field;

        assert (a * b) + (c * d) === 100
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_function_argument_type_mismatch_fails() {
    let source = r#"
    let double(x: field): field = x + x

    proof TypeMismatch {
        input flag: bool;
        witness out: field;
        let result = double(flag) in
        assert out === result;
    }
    "#;
    assert!(type_check_fails_with_type_mismatch(source));
}

#[test]
fn test_tuple_pattern_scope_does_not_leak() {
    let source = r#"
    proof TupleScope {
        input x: field;
        witness out: field;
        let result = (let (a, b) = (x, x) in b) in
        assert out === result;
        assert out === a;
    }
    "#;
    assert!(type_check_fails_with_undefined_error(source));
}

#[test]
fn test_proof_scope_isolated() {
    let source = r#"
    proof FirstProof {
        input x: field;
        witness secret: field;
        witness out: field;
        assert out === secret;
    }

    proof SecondProof {
        witness out: field;
        assert out === secret;
    }
    "#;
    match parse_and_type_check(source) {
        Err(TypeError::UndefinedVariable(_)) => {}
        Err(other) => panic!("expected undefined variable error, got {:?}", other),
        Ok(_) => panic!("expected type checker to fail"),
    }
}

#[test]
fn test_component_simple_call() {
    let source = r#"
    component Add {
        input a: field;
        input b: field;

        a + b
    }

    proof UseAdd {
        input x: field;
        input y: field;
        witness result: field;

        result === Add(x)(y)
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_component_with_witness() {
    let source = r#"
    component Square {
        input x: field;
        witness y: field;

        {
            assert y === x * x;
            y
        }
    }

    proof UseSquare {
        input a: field;
        witness result: field;

        result === Square(a)
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_component_single_input() {
    let source = r#"
    component Double {
        input x: field;

        x + x
    }

    proof UseDouble {
        input a: field;
        witness result: field;

        result === Double(a)
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_component_no_inputs() {
    let source = r#"
    component Constant {
        witness val: field;

        {
            assert val === 42;
            val
        }
    }

    proof UseConstant {
        witness result: field;

        result === Constant()
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_component_undefined_error() {
    let source = r#"
    proof UseNonexistent {
        input a: field;
        witness result: field;

        result === NonexistentComponent(a)
    }
    "#;
    match parse_and_type_check(source) {
        Err(TypeError::UndefinedFunction(_)) => {}
        Err(other) => panic!("expected undefined function error, got {:?}", other),
        Ok(_) => panic!("expected type checker to fail"),
    }
}

#[test]
fn test_component_chained_operations() {
    let source = r#"
    component Add {
        input a: field;
        input b: field;

        a + b
    }

    component Mul {
        input a: field;
        input b: field;

        a * b
    }

    proof UseComponents {
        input x: field;
        input y: field;
        input z: field;
        witness sum: field;
        witness product: field;

        assert sum === Add(x)(y);
        product === Mul(sum)(z)
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_refined_type_basic() {
    let source = r#"
    proof RefinedTest {
        witness x: refined { field, x > 0 && x < 100 };

        assert x > 0
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_refined_type_with_assertion() {
    let source = r#"
    proof RefinedRange {
        witness x: refined { field, x > 0 && x < 100 };
        witness y: field;

        {
            assert x > 0 && x < 100;
            y === x * 2
        }
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_refined_type_input() {
    let source = r#"
    proof RefinedInput {
        input x: refined { field, x != 0 };
        witness result: field;

        result === x * x
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_refined_type_multiple_witnesses() {
    let source = r#"
    proof MultipleRefined {
        witness a: refined { field, a > 0 };
        witness b: refined { field, b > 0 };
        witness sum: field;

        {
            assert a > 0;
            assert b > 0;
            sum === a + b
        }
    }
    "#;
    assert!(type_check_passes(source));
}
