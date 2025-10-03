#!/bin/bash

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

TESTS_DIR="/home/aces/Desktop/projects/Lof/tests/frontend"
PASSED=0
FAILED=0
TOTAL=0

echo -e "${BLUE}Running all .lof tests in $TESTS_DIR${NC}"
echo "=================================================="

# Check if tests directory exists
if [ ! -d "$TESTS_DIR" ]; then
    echo -e "${RED}Error: Tests directory not found: $TESTS_DIR${NC}"
    exit 1
fi

# Function to check if test should pass based on comment pattern
should_pass() {
    local file="$1"
    # Look for comment patterns in the first few lines
    if head -5 "$file" | grep -q "// SHOULD_PASS\|// PASS\|// EXPECT_PASS\|// VALID"; then
        return 0  # Should pass
    elif head -5 "$file" | grep -q "// SHOULD_FAIL\|// FAIL\|// EXPECT_FAIL\|// INVALID"; then
        return 1  # Should fail
    else
        # Default behavior - try to infer from filename
        if [[ "$file" == *"invalid"* ]] || [[ "$file" == *"fail"* ]] || [[ "$file" == *"error"* ]]; then
            return 1  # Should fail
        else
            return 0  # Should pass
        fi
    fi
}

# Find all .lof files and run tests
for filepath in "$TESTS_DIR"/*.lof; do
    # Check if any .lof files exist
    if [ ! -e "$filepath" ]; then
        echo -e "${YELLOW}No .lof files found in $TESTS_DIR${NC}"
        exit 0
    fi
    
    # Extract just the filename
    filename=$(basename "$filepath")
    testname="${filename%.lof}"
    
    # Determine expected outcome
    if should_pass "$filepath"; then
        expected="PASS"
    else
        expected="FAIL"
    fi
    
    echo -n "Testing $filename (expect $expected): "
    
    # Run the test
    if lof check "$filepath" >/dev/null 2>&1; then
        # Test passed
        if [ "$expected" = "PASS" ]; then
            echo -e "${GREEN}PASS ✓${NC}"
            ((PASSED++))
        else
            echo -e "${RED}FAIL (should have failed) ✗${NC}"
            ((FAILED++))
        fi
    else
        # Test failed
        if [ "$expected" = "FAIL" ]; then
            echo -e "${GREEN}PASS (correctly failed) ✓${NC}"
            ((PASSED++))
        else
            echo -e "${RED}FAIL (should have passed) ✗${NC}"
            ((FAILED++))
            # Optionally show error details for unexpected failures
            echo -e "${RED}Error details:${NC}"; lof check "$filepath"
        fi
    fi
    
    ((TOTAL++))
done

echo "=================================================="
echo -e "Results: ${GREEN}$PASSED passed${NC}, ${RED}$FAILED failed${NC} out of $TOTAL total"

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}$FAILED test(s) failed!${NC}"
    exit 1
fi