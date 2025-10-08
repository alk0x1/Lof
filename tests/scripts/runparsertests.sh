#!/bin/bash

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"
TESTS_DIR="$PROJECT_ROOT/tests/integration"
PASSED=0
FAILED=0
TOTAL=0

echo -e "${BLUE}Running parser tests on all .lof files in $TESTS_DIR${NC}"
echo "=================================================="

# Check if tests directory exists
if [ ! -d "$TESTS_DIR" ]; then
    echo -e "${RED}Error: Tests directory not found: $TESTS_DIR${NC}"
    exit 1
fi

# Function to check if test should pass parsing based on comment pattern
should_parse() {
    local file="$1"
    # Look for comment patterns in the first few lines for parse-specific expectations
    if head -5 "$file" | grep -q "// PARSE_PASS\|// SHOULD_PARSE\|// VALID_SYNTAX"; then
        return 0  # Should parse successfully
    elif head -5 "$file" | grep -q "// PARSE_FAIL\|// SHOULD_NOT_PARSE\|// INVALID_SYNTAX"; then
        return 1  # Should fail to parse
    else
        # For parser tests, most files should parse successfully unless they have syntax errors
        # We assume most type-level errors (SHOULD_FAIL) will still parse fine
        return 0  # Default: should parse
    fi
}

# Find all .lof files in both valid and invalid directories
for filepath in "$TESTS_DIR"/valid/*.lof "$TESTS_DIR"/invalid/*.lof; do
    # Check if any .lof files exist
    if [ ! -e "$filepath" ]; then
        continue
    fi
    
    # Extract just the filename
    filename=$(basename "$filepath")
    testname="${filename%.lof}"
    
    # Determine expected outcome for parsing
    if should_parse "$filepath"; then
        expected="PARSE"
    else
        expected="FAIL"
    fi
    
    echo -n "Parsing $filename (expect $expected): "
    
    # Run the parser test
    if lof parse "$filepath" >/dev/null 2>&1; then
        # Parse succeeded
        if [ "$expected" = "PARSE" ]; then
            echo -e "${GREEN}PASS ✓${NC}"
            ((PASSED++))
        else
            echo -e "${RED}FAIL (should not have parsed) ✗${NC}"
            ((FAILED++))
        fi
    else
        # Parse failed
        if [ "$expected" = "FAIL" ]; then
            echo -e "${GREEN}PASS (correctly failed to parse) ✓${NC}"
            ((PASSED++))
        else
            echo -e "${RED}FAIL (should have parsed) ✗${NC}"
            ((FAILED++))
            # Show error details for unexpected parse failures
            echo -e "${RED}Parse error details:${NC}"
            lof parse "$filepath" 2>&1 | head -3
        fi
    fi
    
    ((TOTAL++))
done

echo "=================================================="

# Check if any tests were run
if [ $TOTAL -eq 0 ]; then
    echo -e "${YELLOW}No .lof test files found in $TESTS_DIR${NC}"
    exit 1
fi

echo -e "Results: ${GREEN}$PASSED passed${NC}, ${RED}$FAILED failed${NC} out of $TOTAL total"

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All parser tests passed! ${NC}"
    exit 0
else
    echo -e "${RED}$FAILED parser test(s) failed! ${NC}"
    exit 1
fi