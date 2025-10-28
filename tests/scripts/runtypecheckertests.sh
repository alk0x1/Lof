#!/bin/bash

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"
TESTS_DIR="$PROJECT_ROOT/tests/integration"
PASSED=0
FAILED=0
TOTAL=0

echo -e "${BLUE}Running all .lof tests in $TESTS_DIR${NC}"
echo "=================================================="

if [ ! -d "$TESTS_DIR" ]; then
    echo -e "${RED}Error: Tests directory not found: $TESTS_DIR${NC}"
    exit 1
fi

should_pass() {
    local file="$1"
    if head -5 "$file" | grep -q "// SHOULD_PASS\|// PASS\|// EXPECT_PASS\|// VALID"; then
        return 0
    elif head -5 "$file" | grep -q "// SHOULD_FAIL\|// FAIL\|// EXPECT_FAIL\|// INVALID"; then
        return 1
    else
        if [[ "$file" == *"invalid"* ]] || [[ "$file" == *"fail"* ]] || [[ "$file" == *"error"* ]]; then
            return 1
        else
            return 0
        fi
    fi
}

for filepath in "$TESTS_DIR"/valid/*.lof "$TESTS_DIR"/invalid/*.lof; do
    if [ ! -e "$filepath" ]; then
        continue
    fi
    
    filename=$(basename "$filepath")
    testname="${filename%.lof}"
    
    if should_pass "$filepath"; then
        expected="PASS"
    else
        expected="FAIL"
    fi
    
    echo -n "Testing $filename (expect $expected): "
    
    if lof check "$filepath" >/dev/null 2>&1; then
        if [ "$expected" = "PASS" ]; then
            echo -e "${GREEN}PASS ✓${NC}"
            ((PASSED++))
        else
            echo -e "${RED}FAIL (should have failed) ✗${NC}"
            ((FAILED++))
        fi
    else
        if [ "$expected" = "FAIL" ]; then
            echo -e "${GREEN}PASS (correctly failed) ✓${NC}"
            ((PASSED++))
        else
            echo -e "${RED}FAIL (should have passed) ✗${NC}"
            ((FAILED++))
            echo -e "${RED}Error details:${NC}"; lof check "$filepath"
        fi
    fi
    
    ((TOTAL++))
done

echo "=================================================="

if [ $TOTAL -eq 0 ]; then
    echo -e "${YELLOW}No .lof test files found in $TESTS_DIR${NC}"
    exit 1
fi

echo -e "Results: ${GREEN}$PASSED passed${NC}, ${RED}$FAILED failed${NC} out of $TOTAL total"

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}$FAILED test(s) failed!${NC}"
    exit 1
fi