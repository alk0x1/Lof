use lof::ast::Expression;
use lof::lexer::Lexer;
use lof::parser::Parser;
use lof::r1cs::{LinearCombination, R1CSConstraint, R1CSGenerator};
use lof::typechecker::TypeChecker;
use num_bigint::BigInt;

fn compile_to_r1cs(source: &str) -> Result<R1CSGenerator, String> {
    // Parse
    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);
    let ast = parser
        .parse_program()
        .map_err(|e| format!("Parse error: {:?}", e))?;

    // Type check
    let mut type_checker = TypeChecker::new();
    type_checker
        .check_program(&ast)
        .map_err(|e| format!("Type error: {:?}", e))?;

    // Generate R1CS
    let mut r1cs_generator = R1CSGenerator::new();

    // Find proof and convert it
    for expr in &ast {
        if let Expression::Proof { .. } = expr {
            r1cs_generator
                .convert_proof(expr)
                .map_err(|e| format!("R1CS error: {:?}", e))?;
            break;
        }
    }

    Ok(r1cs_generator)
}

// ============================================================================
// BASIC FUNCTIONALITY TESTS
// ============================================================================

#[test]
fn test_simple_assertion() {
    let source = r#"
    proof SimpleAssertion {
        input x: Field;
        output y: Field;
        assert y === x;
    }"#;

    let r1cs = compile_to_r1cs(source).unwrap();

    // Should have at least one constraint for the assertion
    assert!(!r1cs.constraints.is_empty());
    assert!(r1cs.pub_inputs.contains(&"x".to_string()));
    assert!(r1cs.pub_inputs.contains(&"y".to_string()));
}

#[test]
fn test_constant_assertion() {
    let source = r#"
    proof ConstantAssertion {
        output y: Field;
        assert y === 42;
    }"#;

    let r1cs = compile_to_r1cs(source).unwrap();

    // Should create constraint for the assertion
    assert!(!r1cs.constraints.is_empty());
    assert!(r1cs.pub_inputs.contains(&"y".to_string()));
}

#[test]
fn test_basic_multiplication() {
    let source = r#"
    proof BasicMult {
        input x: Field;
        input y: Field;
        output z: Field;
        let product = x * y in
        assert z === product;
    }"#;

    let r1cs = compile_to_r1cs(source).unwrap();

    // Should have constraints for multiplication and assertion
    assert!(r1cs.constraints.len() >= 2);
    assert!(r1cs.pub_inputs.contains(&"x".to_string()));
    assert!(r1cs.pub_inputs.contains(&"y".to_string()));
    assert!(r1cs.pub_inputs.contains(&"z".to_string()));
}

#[test]
fn test_addition_handling() {
    let source = r#"
    proof Addition {
        input x: Field;
        input y: Field;
        output z: Field;
        let sum = x + y in
        assert z === sum;
    }"#;

    let r1cs = compile_to_r1cs(source).unwrap();

    // Addition creates linear combination, then constraint for assertion
    assert!(!r1cs.constraints.is_empty());
    assert!(r1cs.pub_inputs.contains(&"x".to_string()));
    assert!(r1cs.pub_inputs.contains(&"y".to_string()));
    assert!(r1cs.pub_inputs.contains(&"z".to_string()));
}

#[test]
fn test_witness_vs_public_inputs() {
    let source = r#"
    proof WitnessTest {
        input x: Field;
        witness w: Field;
        output y: Field;
        assert y === w;
    }"#;

    let r1cs = compile_to_r1cs(source).unwrap();

    // Check variable classification
    assert!(r1cs.pub_inputs.contains(&"x".to_string()));
    assert!(r1cs.pub_inputs.contains(&"y".to_string()));
    assert!(r1cs.witnesses.contains(&"w".to_string()));
    assert!(!r1cs.witnesses.contains(&"x".to_string()));
    assert!(!r1cs.pub_inputs.contains(&"w".to_string()));
}

#[test]
fn test_multiple_assertions() {
    let source = r#"
    proof MultipleAssertions {
        input w: Field;
        input x: Field;
        output y: Field;
        output z: Field;
        assert y === w;
        assert z === x;
    }"#;

    let r1cs = compile_to_r1cs(source).unwrap();

    // Should have separate constraints for each assertion (may have additional constraints)
    assert!(r1cs.constraints.len() >= 2);
}

#[test]
fn test_sequential_operations() {
    let source = r#"
    proof SequentialOps {
        input a: Field;
        input b: Field;
        output result: Field;
        let step1 = a + b in
        let step2 = step1 + 10 in
        assert result === step2;
    }"#;

    let r1cs = compile_to_r1cs(source).unwrap();

    // Addition operations should be combined into linear combinations
    assert!(!r1cs.constraints.is_empty()); // At least the final assertion
}

// ============================================================================
// LINEARITY CONSTRAINT TESTS
// ============================================================================

#[test]
fn test_linearity_violation_detection() {
    // Variables are consumed when used in let bindings due to linearity
    let source = r#"
    proof LinearityTest {
        input x: Field;
        output y: Field;
        output z: Field;
        let temp1 = x + 1 in
        let temp2 = x + 2 in  // Should fail - x already consumed
        assert y === temp1;
        assert z === temp2;
    }"#;

    let result = compile_to_r1cs(source);

    // Should fail due to variable consumption
    assert!(result.is_err());
}

#[test]
fn test_proper_variable_usage() {
    let source = r#"
    proof ProperUsage {
        input x: Field;
        input y: Field;
        output result: Field;
        let sum = x + y in  // Both x and y consumed here
        assert result === sum;
    }"#;

    let r1cs = compile_to_r1cs(source).unwrap();

    // Should succeed with proper usage
    assert!(!r1cs.constraints.is_empty());
}

// ============================================================================
// R1CS STRUCTURE TESTS
// ============================================================================

#[test]
fn test_constraint_structure() {
    let source = r#"
    proof ConstraintStructure {
        input a: Field;
        input b: Field;
        output c: Field;
        assert c === a * b;
    }"#;

    let r1cs = compile_to_r1cs(source).unwrap();

    // Verify constraint structure
    for constraint in &r1cs.constraints {
        let has_content = !constraint.a.terms.is_empty()
            || !constraint.b.terms.is_empty()
            || !constraint.c.terms.is_empty();
        assert!(has_content);
    }
}

#[test]
fn test_matrix_generation() {
    let source = r#"
    proof MatrixGeneration {
        input x: Field;
        input y: Field;
        output z: Field;
        assert z === x + y;
    }"#;

    let r1cs = compile_to_r1cs(source).unwrap();
    let (a_matrix, b_matrix, c_matrix) = r1cs.get_matrices();

    // All matrices should have consistent dimensions
    assert_eq!(a_matrix.len(), r1cs.constraints.len());
    assert_eq!(b_matrix.len(), r1cs.constraints.len());
    assert_eq!(c_matrix.len(), r1cs.constraints.len());

    // Check row consistency
    if !a_matrix.is_empty() {
        let width = a_matrix[0].len();
        assert!(a_matrix.iter().all(|row| row.len() == width));
        assert!(b_matrix.iter().all(|row| row.len() == width));
        assert!(c_matrix.iter().all(|row| row.len() == width));
    }
}

#[test]
fn test_variable_organization() {
    let source = r#"
    proof VariableOrg {
        input pub1: Field;
        input pub2: Field;
        witness wit1: Field;
        witness wit2: Field;
        output out1: Field;

        let temp1 = wit1 * 2 in
        let temp2 = wit2 * 3 in
        assert out1 === pub1
    }"#;

    let r1cs = compile_to_r1cs(source).unwrap();

    // Check variable organization
    assert!(r1cs.pub_inputs.contains(&"pub1".to_string()));
    assert!(r1cs.pub_inputs.contains(&"pub2".to_string()));
    assert!(r1cs.pub_inputs.contains(&"out1".to_string()));
    assert!(r1cs.witnesses.contains(&"wit1".to_string()));
    assert!(r1cs.witnesses.contains(&"wit2".to_string()));
}

// ============================================================================
// EDGE CASES AND UTILITY TESTS
// ============================================================================

#[test]
fn test_linear_combination_structure() {
    let lc1 = LinearCombination {
        terms: vec![
            ("x".to_string(), BigInt::from(2)),
            ("y".to_string(), BigInt::from(3)),
        ],
    };
    let lc2 = LinearCombination {
        terms: vec![("z".to_string(), BigInt::from(1))],
    };

    // Test basic structure and access
    assert_eq!(lc1.terms.len(), 2);
    assert_eq!(lc1.terms[0], ("x".to_string(), BigInt::from(2)));
    assert_eq!(lc1.terms[1], ("y".to_string(), BigInt::from(3)));

    assert_eq!(lc2.terms.len(), 1);
    assert_eq!(lc2.terms[0], ("z".to_string(), BigInt::from(1)));
}

#[test]
fn test_r1cs_constraint_creation() {
    let a = LinearCombination {
        terms: vec![("x".to_string(), BigInt::from(1))],
    };
    let b = LinearCombination {
        terms: vec![("y".to_string(), BigInt::from(1))],
    };
    let c = LinearCombination {
        terms: vec![("z".to_string(), BigInt::from(1))],
    };

    let constraint = R1CSConstraint { a, b, c };

    assert_eq!(constraint.a.terms[0], ("x".to_string(), BigInt::from(1)));
    assert_eq!(constraint.b.terms[0], ("y".to_string(), BigInt::from(1)));
    assert_eq!(constraint.c.terms[0], ("z".to_string(), BigInt::from(1)));
}

#[test]
fn test_constraint_display_formatting() {
    let constraint = R1CSConstraint {
        a: LinearCombination {
            terms: vec![("x".to_string(), BigInt::from(1))],
        },
        b: LinearCombination {
            terms: vec![("ONE".to_string(), BigInt::from(1))],
        },
        c: LinearCombination {
            terms: vec![("y".to_string(), BigInt::from(1))],
        },
    };

    let display_str = format!("{}", constraint);
    assert!(display_str.contains("x"));
    assert!(display_str.contains("ONE"));
    assert!(display_str.contains("y"));
}

#[test]
fn test_large_constants() {
    let source = r#"
    proof LargeConstants {
        input x: Field;
        output y: Field;
        assert y === x + 1000000;
    }"#;

    let r1cs = compile_to_r1cs(source).unwrap();

    // Should handle large constants without issue
    assert!(!r1cs.constraints.is_empty());
}

#[test]
fn test_zero_handling() {
    let source = r#"
    proof ZeroHandling {
        input x: Field;
        output y: Field;
        assert y === x + 0;
    }"#;

    let r1cs = compile_to_r1cs(source).unwrap();

    // Should handle zero constants
    assert!(!r1cs.constraints.is_empty());
}

// ============================================================================
// COMPREHENSIVE INTEGRATION TESTS
// ============================================================================

#[test]
fn test_complex_valid_proof() {
    let source = r#"
    proof ComplexValid {
        input a: Field;
        input b: Field;
        witness intermediate: Field;
        output result: Field;
        
        let sum = a + b in
        assert intermediate === sum;
        assert result === intermediate;
    }"#;

    let r1cs = compile_to_r1cs(source).unwrap();

    // Complex proofs might generate additional constraints
    assert!(r1cs.constraints.len() >= 2);

    // Verify variable categorization
    assert!(r1cs.pub_inputs.contains(&"a".to_string()));
    assert!(r1cs.pub_inputs.contains(&"b".to_string()));
    assert!(r1cs.pub_inputs.contains(&"result".to_string()));
    assert!(r1cs.witnesses.contains(&"intermediate".to_string()));
}

#[test]
fn test_r1cs_generator_state() {
    let source = r#"
    proof StateTest {
        input x: Field;
        output y: Field;
        assert y === x;
    }"#;

    let r1cs = compile_to_r1cs(source).unwrap();

    // Test basic generator state
    assert!(!r1cs.constraints.is_empty());
    assert!(!r1cs.pub_inputs.is_empty());
    assert_eq!(r1cs.temp_var_counter, 0); // No temp vars created for simple assertion
}

#[test]
fn test_constraint_count_accuracy() {
    let source = r#"
    proof ConstraintCount {
        input a: Field;
        input b: Field;
        input c: Field;
        output x: Field;
        output y: Field;
        output z: Field;
        
        assert x === a;
        assert y === b;
        assert z === c;
    }"#;

    let r1cs = compile_to_r1cs(source).unwrap();

    // Should have at least 3 constraints for 3 assertions
    assert!(r1cs.constraints.len() >= 3);
    assert_eq!(r1cs.pub_inputs.len(), 6); // 3 inputs + 3 outputs
}
