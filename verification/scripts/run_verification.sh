#!/bin/bash

# Complete verification pipeline for Lof vs Circom
# Usage: ./run_verification.sh [circuit_name]

CIRCUIT_NAME=${1:-multiply}
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "COMPLETE VERIFICATION PIPELINE"
echo "Circuit: $CIRCUIT_NAME"
echo "=============================="

# Step 1: Compile both implementations
echo ""
echo "STEP 1: Compiling circuits..."
if ! "$SCRIPT_DIR/compile_both.sh" "$CIRCUIT_NAME"; then
    echo "FAILED: Compilation step failed"
    exit 1
fi

# Step 2: Generate witnesses
echo ""
echo "STEP 2: Generating witnesses..."
if ! "$SCRIPT_DIR/generate_witnesses.sh" "$CIRCUIT_NAME"; then
    echo "FAILED: Witness generation step failed"
    exit 1
fi

# Step 3: Compare mathematical equivalence
echo ""
echo "STEP 3: Comparing mathematical equivalence..."
if python3 "$SCRIPT_DIR/compare_results.py" "$CIRCUIT_NAME"; then
    echo ""
    echo "=============================="
    echo "VERIFICATION COMPLETE: SUCCESS"
    echo "Your Lof R1CS implementation produces mathematically equivalent results to Circom"
    echo "This circuit is ready for production use"
    exit 0
else
    echo ""
    echo "=============================="
    echo "VERIFICATION COMPLETE: FAILED"
    echo "Your Lof R1CS implementation does NOT match Circom's mathematical behavior"
    echo "Review the comparison details above and fix your R1CS generation"
    exit 1
fi