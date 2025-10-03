#!/usr/bin/env python3
"""
Mathematical equivalence verification between Lof and Circom
Compares witness outputs to verify identical mathematical behavior
"""

import json
import sys
import os
from pathlib import Path

def load_test_cases(circuit_name):
    """Load test cases for a given circuit"""
    script_dir = Path(__file__).parent
    test_file = script_dir.parent / "test_cases" / "01_basic" / f"{circuit_name}_tests.json"
    
    if not test_file.exists():
        raise FileNotFoundError(f"Test cases file not found: {test_file}")
    
    with open(test_file, 'r') as f:
        return json.load(f)

def load_circom_witness(witness_file):
    """Load Circom witness from JSON file"""
    if not os.path.exists(witness_file):
        return None
    
    with open(witness_file, 'r') as f:
        return json.load(f)

def load_lof_witness(witness_file):
    """Load Lof witness from JSON file"""
    if not os.path.exists(witness_file):
        return None
    
    with open(witness_file, 'r') as f:
        return json.load(f)

def extract_outputs(witness, expected_outputs):
    """Extract output values from witness"""
    
    if isinstance(witness, list):
        # Handle different witness formats
        if len(witness) == 4:
            # Circom format: [ONE, output, input_a, input_b]
            return {"c": witness[1]} if witness else {}
        elif len(witness) == 2:
            # Lof format: [witness_0, witness_1] - use first element as output
            return {"c": witness[0]} if witness else {}
        else:
            # Fallback: assume last element is the output
            return {"c": witness[-1]} if witness else {}
    elif isinstance(witness, dict):
        # If witness is a dict, look for output fields
        outputs = {}
        for key in expected_outputs.keys():
            if key in witness:
                outputs[key] = witness[key]
        return outputs
    else:
        return {}

def compare_mathematical_equivalence(circuit_name, test_vector_index, test_vector):
    """Compare mathematical equivalence for one test case"""
    
    script_dir = Path(__file__).parent
    outputs_dir = script_dir.parent / "outputs"
    
    result = {
        "test_name": test_vector.get("name", f"test_{test_vector_index}"),
        "inputs": test_vector["inputs"],
        "expected": test_vector["expected_outputs"],
        "success": False,
        "details": {}
    }
    
    try:
        # Load witnesses
        circom_witness_file = outputs_dir / "circom" / "witnesses" / f"test_input_{test_vector_index}.json"
        lof_witness_file = outputs_dir / "lof" / "witnesses" / f"test_input_{test_vector_index}.json"
        
        circom_witness = load_circom_witness(circom_witness_file)
        lof_witness = load_lof_witness(lof_witness_file)
        
        if circom_witness is None:
            result["details"]["error"] = f"Circom witness not found: {circom_witness_file}"
            return result
        
        if lof_witness is None:
            result["details"]["error"] = f"Lof witness not found: {lof_witness_file}"
            return result
        
        # Extract outputs
        expected_outputs = test_vector["expected_outputs"]
        circom_outputs = extract_outputs(circom_witness, expected_outputs)
        lof_outputs = extract_outputs(lof_witness, expected_outputs)
        
        # Compare each expected output
        outputs_match = True
        output_comparison = {}
        
        for output_name, expected_value in expected_outputs.items():
            circom_value = circom_outputs.get(output_name)
            lof_value = lof_outputs.get(output_name)
            
            # Convert to strings for comparison (handles different number formats)
            circom_str = str(circom_value) if circom_value is not None else "MISSING"
            lof_str = str(lof_value) if lof_value is not None else "MISSING"
            expected_str = str(expected_value)
            
            circom_correct = circom_str == expected_str
            lof_correct = lof_str == expected_str
            values_match = circom_str == lof_str
            
            output_comparison[output_name] = {
                "expected": expected_str,
                "circom": circom_str,
                "lof": lof_str,
                "circom_correct": circom_correct,
                "lof_correct": lof_correct,
                "values_match": values_match
            }
            
            if not (circom_correct and lof_correct and values_match):
                outputs_match = False
        
        result["details"] = {
            "output_comparison": output_comparison,
            "all_outputs_match": outputs_match
        }
        
        result["success"] = outputs_match
        
    except Exception as e:
        result["details"]["error"] = str(e)
    
    return result

def validate_circuit(circuit_name):
    """Validate all test cases for a circuit"""
    
    print(f"MATHEMATICAL EQUIVALENCE VERIFICATION: {circuit_name}")
    print("=" * 60)
    
    try:
        test_data = load_test_cases(circuit_name)
    except Exception as e:
        print(f"FAILED: Could not load test cases: {e}")
        return False
    
    results = {
        "circuit_name": circuit_name,
        "total_tests": len(test_data["test_vectors"]),
        "passed": 0,
        "failed": 0,
        "test_results": []
    }
    
    # Process each test vector
    for i, test_vector in enumerate(test_data["test_vectors"]):
        print(f"\nTest {i+1}: {test_vector.get('name', f'test_{i}')}")
        print(f"Inputs: {test_vector['inputs']}")
        
        test_result = compare_mathematical_equivalence(circuit_name, i, test_vector)
        results["test_results"].append(test_result)
        
        if test_result["success"]:
            results["passed"] += 1
            print("  RESULT: PASS - Mathematical equivalence verified")
            
            # Show output comparison
            if "output_comparison" in test_result["details"]:
                for output_name, comparison in test_result["details"]["output_comparison"].items():
                    print(f"    {output_name}: Expected={comparison['expected']}, "
                          f"Circom={comparison['circom']}, Lof={comparison['lof']}")
        else:
            results["failed"] += 1
            print("  RESULT: FAIL - Mathematical equivalence NOT verified")
            
            if "error" in test_result["details"]:
                print(f"    ERROR: {test_result['details']['error']}")
            elif "output_comparison" in test_result["details"]:
                for output_name, comparison in test_result["details"]["output_comparison"].items():
                    print(f"    {output_name}:")
                    print(f"      Expected: {comparison['expected']}")
                    print(f"      Circom:   {comparison['circom']} ({'CORRECT' if comparison['circom_correct'] else 'WRONG'})")
                    print(f"      Lof:      {comparison['lof']} ({'CORRECT' if comparison['lof_correct'] else 'WRONG'})")
                    print(f"      Match:    {comparison['values_match']}")
    
    # Summary
    print("\n" + "=" * 60)
    print(f"SUMMARY: {circuit_name}")
    print(f"Total tests: {results['total_tests']}")
    print(f"Passed: {results['passed']}")
    print(f"Failed: {results['failed']}")
    
    success_rate = results['passed'] / results['total_tests'] * 100 if results['total_tests'] > 0 else 0
    print(f"Success rate: {success_rate:.1f}%")
    
    overall_success = results['failed'] == 0
    print(f"OVERALL RESULT: {'PASS' if overall_success else 'FAIL'}")
    
    if overall_success:
        print("Mathematical equivalence with Circom: VERIFIED")
    else:
        print("Mathematical equivalence with Circom: NOT VERIFIED")
        print("Your R1CS implementation needs fixes before production use")
    
    return overall_success

def main():
    if len(sys.argv) != 2:
        print("Usage: python3 compare_results.py <circuit_name>")
        print("Example: python3 compare_results.py multiply")
        sys.exit(1)
    
    circuit_name = sys.argv[1]
    success = validate_circuit(circuit_name)
    
    sys.exit(0 if success else 1)

if __name__ == "__main__":
    main()