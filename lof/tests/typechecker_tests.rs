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
fn test_proof_block_allows_multi_use() {
    // Inside proof: multi-use is OK (scoped linearity)
    let source = r#"
    proof Test {
        input x: Field;
        output y: Field;
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
        output y: Field;
        assert y === undefined_var;
    }"#;
    assert!(type_check_fails_with_undefined_error(source));
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
        output result: Field;

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

// ============================================================================
// PHASE 2: CONSTRAINT TRACKING TESTS
// ============================================================================
// These tests implement TDD for constraint tracking feature (Week 2-3)
// Following the roadmap in ROADMAP.md

// ----------------------------------------------------------------------------
// Day 1-2: Basic Constraint Status Tracking
// ----------------------------------------------------------------------------

#[test]
fn test_witness_starts_unconstrained() {
    // Witnesses should start with Unconstrained status
    // This test will fail until ConstraintStatus is added to AST
    let source = r#"
    proof Test {
        input x: field;
        witness w: field;
        // w is never used in a constraint - should ERROR
        assert x > 0
    }
    "#;
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

#[test]
fn test_input_automatically_constrained() {
    // Inputs are public - automatically constrained, no witnesses to validate
    let source = r#"
    proof Test {
        input x: field;
        // No explicit constraint needed - inputs are inherently constrained
        assert x > 0
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_output_automatically_constrained() {
    // Outputs are public - automatically constrained
    let source = r#"
    proof Test {
        output y: field;
        // No explicit constraint needed - outputs are inherently constrained
        let temp = 42 in
        assert y === temp
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_multiple_witnesses_all_must_be_constrained() {
    // All witnesses must be constrained
    let source = r#"
    proof Test {
        witness w1: field;
        witness w2: field;
        witness w3: field;

        // Only w1 and w2 are constrained
        let temp = w1 * 2 in
        assert temp === w2;

        // w3 is forgotten - ERROR
    }
    "#;
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

// ----------------------------------------------------------------------------
// Day 3: Signal Declaration Rules
// ----------------------------------------------------------------------------

#[test]
fn test_witness_must_be_used_in_constraint() {
    // Most basic test - witness declared but never constrained
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
    // Addition doesn't create constraints - witness still unconstrained
    let source = r#"
    proof Invalid {
        witness x: field;
        let y = x + 5 in  // Addition is linear combination - no constraint!
        assert y > 0
    }
    "#;
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

#[test]
fn test_witness_used_in_let_binding_not_constrained() {
    // Simple let binding (aliasing) doesn't constrain
    let source = r#"
    proof Invalid {
        witness x: field;
        let y = x in  // Just aliasing - no constraint
        assert y > 0
    }
    "#;
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

// ----------------------------------------------------------------------------
// Day 4: Constraint Promotion Rules - Operations That CONSTRAIN
// ----------------------------------------------------------------------------

#[test]
fn test_multiplication_constrains_witness() {
    // Multiplication creates R1CS constraint
    let source = r#"
    proof Valid {
        witness x: field;
        let y = x * 2 in  // x is now constrained
        assert y > 0
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_multiplication_constrains_both_operands() {
    // Both operands in multiplication are constrained
    let source = r#"
    proof Valid {
        witness a: field;
        witness b: field;
        let product = a * b in  // Both a and b are constrained
        assert product > 0
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_division_constrains_witness() {
    // Division requires multiplicative inverse - creates constraint
    let source = r#"
    proof Valid {
        input divisor: field;
        witness x: field;
        let result = x / divisor in  // x is constrained
        assert result > 0
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_assertion_constrains_witness() {
    // Assert creates a constraint - all variables in expression constrained
    let source = r#"
    proof Valid {
        witness x: field;
        assert x > 0  // x is now constrained
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_assertion_constrains_all_variables() {
    // All variables in assertion expression are constrained
    let source = r#"
    proof Valid {
        witness a: field;
        witness b: field;
        witness c: field;
        assert a + b === c  // All three constrained
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_equality_assertion_constrains_both_sides() {
    // === operator in assertion constrains both sides
    let source = r#"
    proof Valid {
        witness x: field;
        witness y: field;
        assert x === y  // Both x and y constrained
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_match_constrains_scrutinee() {
    // Match expressions create selector constraints
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
    // Nested operations - witness used in multiplication anywhere
    let source = r#"
    proof Valid {
        witness x: field;
        let temp = x * 2 in
        let final_val = temp + 100 in  // Addition doesn't constrain temp, but x already constrained
        assert final_val > 0
    }
    "#;
    assert!(type_check_passes(source));
}

// ----------------------------------------------------------------------------
// Day 4: Constraint Promotion Rules - Operations That DON'T CONSTRAIN
// ----------------------------------------------------------------------------

#[test]
fn test_addition_does_not_constrain() {
    // Addition is linear combination - doesn't create new constraints
    let source = r#"
    proof Invalid {
        witness x: field;
        let y = x + 5 in  // x still unconstrained
        assert y > 0
    }
    "#;
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

#[test]
fn test_subtraction_does_not_constrain() {
    // Subtraction is linear combination - doesn't create new constraints
    let source = r#"
    proof Invalid {
        witness x: field;
        let y = x - 1 in  // x still unconstrained
        assert y > 0
    }
    "#;
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

#[test]
fn test_comparison_does_not_constrain() {
    // Comparison operators don't create constraints by themselves
    // The witnesses x and y are only used in a comparison, which creates a bool
    // But the bool isn't used in a constraining operation (like mult)
    let source = r#"
    proof Invalid {
        witness x: field;
        witness y: field;
        input z: field;
        let is_greater = x > y in  // Comparison doesn't constrain x or y
        let result = z * z in       // This constrains z (input, auto-constrained)
        assert result > 0
    }
    "#;
    // x and y should still be unconstrained - they're only used in comparison
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

#[test]
fn test_multiple_additions_still_unconstrained() {
    // Chain of additions - witness never constrained
    let source = r#"
    proof Invalid {
        witness x: field;
        let y = x + 1 in
        let z = y + 2 in
        let w = z + 3 in
        assert w > 0  // w is constrained, but x is not
    }
    "#;
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

// ----------------------------------------------------------------------------
// Real-World Vulnerability Tests (from roadmap)
// ----------------------------------------------------------------------------

#[test]
fn test_mimc_unconstrained_bug() {
    // Recreates real Circomlib MiMC bug - intermediate witness unconstrained
    let source = r#"
    proof MiMCBroken {
        input x: field;
        witness intermediate: field;  // BUG: Never constrained

        let hash = x + 1 in
        assert hash > 0
    }
    "#;
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

#[test]
fn test_mimc_fixed() {
    // Fixed version - intermediate is constrained
    let source = r#"
    proof MiMCFixed {
        input x: field;
        witness intermediate: field;

        let hash = x + 1 in
        assert intermediate === hash  // intermediate now constrained
    }
    "#;
    assert!(type_check_passes(source));
}

// ----------------------------------------------------------------------------
// Complex Scenarios
// ----------------------------------------------------------------------------

#[test]
fn test_range_proof_all_witnesses_constrained() {
    // Range proof with bit decomposition - all bits must be constrained
    let source = r#"
    proof RangeProof {
        input value: field;
        witness bit_0: field;
        witness bit_1: field;
        witness bit_2: field;
        witness bit_3: field;

        // Constrain bits to be binary
        assert bit_0 * (1 - bit_0) === 0;
        assert bit_1 * (1 - bit_1) === 0;
        assert bit_2 * (1 - bit_2) === 0;
        assert bit_3 * (1 - bit_3) === 0;

        // Constrain value decomposition
        assert value === bit_0 + 2 * bit_1 + 4 * bit_2 + 8 * bit_3
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_merkle_proof_all_witnesses_constrained() {
    // Merkle proof - all intermediate hashes must be constrained
    let source = r#"
    proof MerkleProof {
        input leaf: field;
        input root: field;
        witness sibling_0: field;
        witness sibling_1: field;
        witness hash_0: field;
        witness hash_1: field;

        // Hash level 0
        let sum_0 = leaf + sibling_0 in
        assert hash_0 === sum_0 * sum_0;  // hash_0 and sibling_0 constrained

        // Hash level 1
        let sum_1 = hash_0 + sibling_1 in
        assert hash_1 === sum_1 * sum_1;  // hash_1 and sibling_1 constrained

        // Verify root
        assert hash_1 === root
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_conditional_constraint() {
    // Conditional logic with selector
    let source = r#"
    proof Conditional {
        input x: field;
        input selector: field;
        witness result: field;

        // Selector must be binary
        assert selector * (1 - selector) === 0;

        // Conditional logic
        let option_a = x * 2 in
        let option_b = x * 3 in
        let selected = selector * option_a + (1 - selector) * option_b in
        assert result === selected
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_partial_constraint_chain() {
    // Some witnesses constrained, some not
    let source = r#"
    proof PartiallyBroken {
        witness w1: field;
        witness w2: field;
        witness w3: field;

        // w1 and w2 are constrained
        let product = w1 * w2 in
        assert product > 0;

        // w3 is forgotten - ERROR
    }
    "#;
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

#[test]
fn test_witness_constrained_transitively() {
    // Witness constrained through chain of operations
    let source = r#"
    proof Transitive {
        witness x: field;
        witness y: field;

        // x constrained by multiplication
        let temp = x * 2 in

        // y constrained by assertion involving temp
        assert y === temp
    }
    "#;
    assert!(type_check_passes(source));
}

#[test]
fn test_addition_with_constrained_value_still_requires_constraint() {
    // Adding constrained value to unconstrained witness doesn't constrain witness
    let source = r#"
    proof Tricky {
        input x: field;  // x is constrained
        witness w: field;  // w starts unconstrained

        let sum = x + w in  // Addition doesn't constrain w
        assert sum > 0  // This constrains sum but not necessarily w
    }
    "#;
    // This is a tricky case - need to verify correct behavior
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

// ----------------------------------------------------------------------------
// Edge Cases
// ----------------------------------------------------------------------------

#[test]
fn test_witness_only_in_witness_list() {
    // Witness declared but literally never used anywhere
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
    // Can't have one constrained and one not
    let source = r#"
    proof MixedWitnesses {
        witness constrained_w: field;
        witness unconstrained_w: field;

        assert constrained_w > 0;  // This one is OK
        // unconstrained_w is never used - ERROR
    }
    "#;
    assert!(type_check_fails_with_unconstrained_witness_error(source));
}

#[test]
fn test_witness_in_both_sides_of_multiplication() {
    // Witness multiplied by itself
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
    // Complex expression with multiple witnesses
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
