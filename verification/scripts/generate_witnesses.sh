#!/bin/bash

# Witness generation script for Lof vs Circom verification
# Usage: ./generate_witnesses.sh multiply

CIRCUIT_NAME=${1:-multiply}
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
OUTPUTS_DIR="$PROJECT_ROOT/outputs"
TEST_CASES_FILE="$PROJECT_ROOT/test_cases/01_basic/${CIRCUIT_NAME}_tests.json"

echo "Generating witnesses for $CIRCUIT_NAME circuit..."

# Check if test cases file exists
if [ ! -f "$TEST_CASES_FILE" ]; then
    echo "FAILED: Test cases file not found: $TEST_CASES_FILE"
    exit 1
fi

# Create witness directories
mkdir -p "$OUTPUTS_DIR/circom/witnesses"
mkdir -p "$OUTPUTS_DIR/lof/witnesses"

# Extract test vectors from JSON (using python for JSON parsing)
python3 -c "
import json
import sys

with open('$TEST_CASES_FILE', 'r') as f:
    data = json.load(f)

for i, test_vector in enumerate(data['test_vectors']):
    with open('$OUTPUTS_DIR/test_input_{}.json'.format(i), 'w') as out:
        json.dump(test_vector['inputs'], out)
    print('test_input_{}.json'.format(i))
" > "$OUTPUTS_DIR/input_files.txt"

# Generate witnesses for each test case
while read -r input_file; do
    echo "  -> Processing $input_file..."
    
    # Generate Circom witness
    echo "    -> Generating Circom witness..."
    if [ -f "$OUTPUTS_DIR/circom/${CIRCUIT_NAME}_js/generate_witness.js" ]; then
        if node "$OUTPUTS_DIR/circom/${CIRCUIT_NAME}_js/generate_witness.js" \
             "$OUTPUTS_DIR/circom/${CIRCUIT_NAME}_js/${CIRCUIT_NAME}.wasm" \
             "$OUTPUTS_DIR/$input_file" \
             "$OUTPUTS_DIR/circom/witnesses/${input_file%.json}.wtns"; then
            echo "      SUCCESS: Circom witness generated"
            
            # Convert to JSON for easier comparison
            if command -v snarkjs >/dev/null 2>&1; then
                snarkjs wtns export json \
                    "$OUTPUTS_DIR/circom/witnesses/${input_file%.json}.wtns" \
                    "$OUTPUTS_DIR/circom/witnesses/${input_file%.json}.json"
                echo "      Generated: ${input_file%.json}.json"
            else
                echo "      WARNING: snarkjs not found, witness in binary format only"
            fi
        else
            echo "      FAILED: Circom witness generation"
        fi
    else
        echo "      FAILED: Circom witness generator not found"
    fi
    
    # Generate Lof witness using lofit
    echo "    -> Generating Lof witness using lofit..."
    
    # Create temporary directory for lofit operation
    temp_lofit_dir="$OUTPUTS_DIR/temp_lofit_${input_file%.json}"
    mkdir -p "$temp_lofit_dir"
    
    # Copy R1CS to temp directory
    cp "$OUTPUTS_DIR/lof/$CIRCUIT_NAME.r1cs" "$temp_lofit_dir/"
    
    # Copy input file as public inputs (lofit expects this format)
    cp "$OUTPUTS_DIR/$input_file" "$temp_lofit_dir/inputs.json"
    
    cd "$temp_lofit_dir"
    
    # Run lofit setup and prove to generate witness (using cargo run)
    if cargo run --bin lofit -- setup --input "$CIRCUIT_NAME.r1cs" --proving-key pk.bin --verification-key vk.bin > /dev/null 2>&1; then
        if cargo run --bin lofit -- prove --input "$CIRCUIT_NAME.r1cs" --proving-key pk.bin --public-inputs inputs.json --output proof.bin > /dev/null 2>&1; then
            # Extract witness from the generated full_witness.json
            if [ -f "full_witness.json" ]; then
                # Convert lofit witness format to simple array format for comparison
                python3 -c "
import json
import sys

# Read lofit witness format
with open('full_witness.json', 'r') as f:
    witness_data = json.load(f)

# Convert to simple array (extract just the values)
witness_array = list(witness_data.values())

# Save in format compatible with comparison script
with open('$OUTPUTS_DIR/lof/witnesses/${input_file%.json}.json', 'w') as f:
    json.dump(witness_array, f)
"
                echo "      SUCCESS: Lof witness generated via lofit"
            else
                echo "      FAILED: lofit did not generate full_witness.json"
            fi
        else
            echo "      FAILED: lofit prove command failed"
        fi
    else
        echo "      FAILED: lofit setup command failed"
    fi
    
    cd - > /dev/null
    
    # Clean up temp directory
    rm -rf "$temp_lofit_dir"
    
done < "$OUTPUTS_DIR/input_files.txt"

echo ""
echo "WITNESS GENERATION COMPLETE"
echo "Circom witnesses: $OUTPUTS_DIR/circom/witnesses/"
echo "Lof witnesses: $OUTPUTS_DIR/lof/witnesses/"
echo ""
echo "Next step: python3 compare_results.py $CIRCUIT_NAME"