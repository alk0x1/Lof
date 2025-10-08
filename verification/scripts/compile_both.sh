#!/bin/bash

# Compilation script for Lof vs Circom verification
# Usage: ./compile_both.sh multiply

CIRCUIT_NAME=${1:-multiply}
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
OUTPUTS_DIR="$PROJECT_ROOT/outputs"

# Find the circuit in any category directory
CIRCUIT_FILE=""
for category_dir in "$PROJECT_ROOT/circuits"/*; do
    if [ -d "$category_dir" ]; then
        if [ -f "$category_dir/$CIRCUIT_NAME.lof" ]; then
            CIRCUITS_DIR="$category_dir"
            CIRCUIT_FILE="$category_dir/$CIRCUIT_NAME.lof"
            break
        fi
    fi
done

if [ -z "$CIRCUIT_FILE" ]; then
    echo "Error: Circuit '$CIRCUIT_NAME' not found in any category"
    exit 1
fi

echo "Found circuit in: $CIRCUITS_DIR"

echo "Compiling $CIRCUIT_NAME circuit in both languages..."

# Create output directories
mkdir -p "$OUTPUTS_DIR/circom"
mkdir -p "$OUTPUTS_DIR/lof"

cd "$CIRCUITS_DIR"

# Compile Circom circuit
echo "  -> Compiling Circom circuit..."
if circom "$CIRCUIT_NAME.circom" --r1cs --wasm --sym --output "$OUTPUTS_DIR/circom/"; then
    echo "    SUCCESS: Circom compilation"
    
    # Check if R1CS file was created
    if [ -f "$OUTPUTS_DIR/circom/$CIRCUIT_NAME.r1cs" ]; then
        echo "    Generated: $CIRCUIT_NAME.r1cs"
    else
        echo "    WARNING: R1CS file not found"
    fi
    
    # Check if WASM was created
    if [ -d "$OUTPUTS_DIR/circom/${CIRCUIT_NAME}_js" ]; then
        echo "    Generated: ${CIRCUIT_NAME}_js/ (witness calculator)"
    else
        echo "    WARNING: WASM directory not found"
    fi
else
    echo "    FAILED: Circom compilation"
    exit 1
fi

# Compile Lof circuit
echo "  -> Compiling Lof circuit..."
if lof compile "$CIRCUIT_NAME.lof"; then
    echo "    SUCCESS: Lof compilation"
    
    # Lof generates R1CS in current directory, move it to outputs
    if [ -f "$CIRCUIT_NAME.r1cs" ]; then
        echo "    Generated: $CIRCUIT_NAME.r1cs"
        mv "$CIRCUIT_NAME.r1cs" "$OUTPUTS_DIR/lof/$CIRCUIT_NAME.r1cs"
        echo "    Moved to: $OUTPUTS_DIR/lof/$CIRCUIT_NAME.r1cs"
        
        # Check file size to ensure it's not empty
        file_size=$(stat -c%s "$OUTPUTS_DIR/lof/$CIRCUIT_NAME.r1cs" 2>/dev/null || echo "0")
        if [ "$file_size" -eq 0 ]; then
            echo "    WARNING: R1CS file is empty (0 bytes) - R1CS generation may have failed"
        else
            echo "    R1CS file size: $file_size bytes"
        fi
    else
        echo "    WARNING: Lof R1CS file not found in current directory"
        # Check if it was created in build directory anyway
        if [ -f "build/$CIRCUIT_NAME.r1cs" ]; then
            echo "    Found in build/ directory, moving..."
            mv "build/$CIRCUIT_NAME.r1cs" "$OUTPUTS_DIR/lof/$CIRCUIT_NAME.r1cs"
        fi
    fi
    
    # Clean up any created directories
    rm -rf build/ keys/ inputs/ proofs/
    
else
    echo "    FAILED: Lof compilation"
    exit 1
fi

echo ""
echo "COMPILATION COMPLETE for both languages"
echo "Outputs stored in: $OUTPUTS_DIR/"
echo ""
echo "Next steps:"
echo "  1. Run: ./generate_witnesses.sh $CIRCUIT_NAME"
echo "  2. Run: python3 compare_results.py $CIRCUIT_NAME"