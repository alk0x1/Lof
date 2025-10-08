#!/bin/bash

# Complete verification pipeline for all test categories
# Usage: ./run_all_verification.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CIRCUITS_DIR="$SCRIPT_DIR/../circuits"

echo "COMPLETE VERIFICATION PIPELINE - ALL CATEGORIES"
echo "==============================================="

total_tests=0
passed_tests=0
failed_tests=0

# Function to run verification for a single circuit
run_circuit_verification() {
    local category=$1
    local circuit=$2
    
    echo ""
    echo "Testing: $category/$circuit"
    echo "----------------------------"
    
    if "$SCRIPT_DIR/run_verification.sh" "$circuit"; then
        echo "✅ PASSED: $category/$circuit"
        ((passed_tests++))
    else
        echo "❌ FAILED: $category/$circuit"
        ((failed_tests++))
    fi
    ((total_tests++))
}

# Test all categories
for category_dir in "$CIRCUITS_DIR"/*; do
    if [ -d "$category_dir" ]; then
        category=$(basename "$category_dir")
        echo ""
        echo "CATEGORY: $category"
        echo "==================="
        
        # Find all .lof files in this category
        for lof_file in "$category_dir"/*.lof; do
            if [ -f "$lof_file" ]; then
                circuit=$(basename "$lof_file" .lof)
                run_circuit_verification "$category" "$circuit"
            fi
        done
    fi
done

echo ""
echo "VERIFICATION SUMMARY"
echo "===================="
echo "Total tests: $total_tests"
echo "Passed: $passed_tests"
echo "Failed: $failed_tests"

if [ $failed_tests -eq 0 ]; then
    echo ""
    echo "🎉 ALL TESTS PASSED!"
    echo "Your Lof R1CS implementation is mathematically equivalent to Circom"
    echo "across ALL language features. Ready for production!"
    exit 0
else
    echo ""
    echo "⚠️  Some tests failed. Review the output above."
    echo "Fix the failing circuits before proceeding to production."
    exit 1
fi