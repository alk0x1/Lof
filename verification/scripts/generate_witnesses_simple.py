#!/usr/bin/env python3

import json
import sys
import os
import subprocess

def main():
    if len(sys.argv) != 2:
        print("Usage: python3 generate_witnesses_simple.py <circuit_name>")
        sys.exit(1)
    
    circuit_name = sys.argv[1]
    script_dir = os.path.dirname(os.path.abspath(__file__))
    project_root = os.path.dirname(script_dir)
    outputs_dir = os.path.join(project_root, "outputs")
    
    # Find test cases file
    test_cases_file = None
    test_cases_dir = os.path.join(project_root, "test_cases")
    
    for category in os.listdir(test_cases_dir):
        category_path = os.path.join(test_cases_dir, category)
        if os.path.isdir(category_path):
            test_file = os.path.join(category_path, f"{circuit_name}_tests.json")
            if os.path.exists(test_file):
                test_cases_file = test_file
                break
    
    if not test_cases_file:
        print(f"FAILED: Test cases file not found for circuit: {circuit_name}")
        return 1
    
    print(f"Using test cases: {test_cases_file}")
    
    # Create output directories
    os.makedirs(os.path.join(outputs_dir, "circom", "witnesses"), exist_ok=True)
    os.makedirs(os.path.join(outputs_dir, "lof", "witnesses"), exist_ok=True)
    
    # Load test cases
    with open(test_cases_file, 'r') as f:
        data = json.load(f)
    
    print(f"Generating witnesses for {circuit_name} circuit...")
    print(f"Found {len(data['test_vectors'])} test cases")
    
    # Generate witnesses for each test case
    for i, test_vector in enumerate(data['test_vectors']):
        input_file = os.path.join(outputs_dir, f"test_input_{i}.json")
        with open(input_file, 'w') as f:
            json.dump(test_vector['inputs'], f)
        
        print(f"  -> Processing test case {i}: {test_vector['name']}")
        
        # Generate Circom witness
        circom_cmd = [
            "node", 
            os.path.join(outputs_dir, "circom", f"{circuit_name}_js", "generate_witness.js"),
            os.path.join(outputs_dir, "circom", f"{circuit_name}_js", f"{circuit_name}.wasm"),
            input_file,
            os.path.join(outputs_dir, "circom", "witnesses", f"test_input_{i}.wtns")
        ]
        
        try:
            subprocess.run(circom_cmd, check=True, capture_output=True, text=True)
            print(f"    ✅ Circom witness generated")
        except subprocess.CalledProcessError as e:
            print(f"    ❌ Circom witness failed: {e.stderr}")
            continue
        
        # Generate Lof witness
        lof_cmd = [
            "cargo", "run", "--bin", "lofit", "--", "witness", 
            "--r1cs", os.path.join(outputs_dir, "lof", f"{circuit_name}.r1cs"),
            "--inputs", input_file,
            "--output", os.path.join(outputs_dir, "lof", "witnesses", f"test_input_{i}.json")
        ]
        
        try:
            # Run from the project root directory (where Cargo.toml is)
            project_cargo_root = os.path.join(project_root, "..")
            result = subprocess.run(lof_cmd, check=True, capture_output=True, text=True, cwd=project_cargo_root)
            print(f"    ✅ Lof witness generated")
        except subprocess.CalledProcessError as e:
            print(f"    ❌ Lof witness failed: {e.stderr}")
            continue
    
    print("SUCCESS: Witness generation completed")
    return 0

if __name__ == "__main__":
    sys.exit(main())