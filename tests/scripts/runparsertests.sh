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

echo -e "${BLUE}Running parser tests on all .lof files in $TESTS_DIR${NC}"
echo "=================================================="

if [ ! -d "$TESTS_DIR" ]; then
    echo -e "${RED}Error: Tests directory not found: $TESTS_DIR${NC}"
    exit 1
fi

should_parse() {
    local file="$1"
    if head -5 "$file" | grep -q "// PARSE_PASS\|// SHOULD_PARSE\|// VALID_SYNTAX"; then
        return 0 
    elif head -5 "$file" | grep -q "// PARSE_FAIL\|// SHOULD_NOT_PARSE\|// INVALID_SYNTAX"; then
        return 1 
    else
        return 0 
    fi
}

for filepath in "$TESTS_DIR"/valid/*.lof "$TESTS_DIR"/invalid/*.lof; do
    if [ ! -e "$filepath" ]; then
        continue
    fi
    
    filename=$(basename "$filepath")
    testname="${filename%.lof}"
    
    if should_parse "$filepath"; then
        expected="PARSE"
    else
        expected="FAIL"
    fi
    
    echo -n "Parsing $filename (expect $expected): "
    
    if lof parse "$filepath" >/dev/null 2>&1; then
        if [ "$expected" = "PARSE" ]; then
            echo -e "${GREEN}PASS ✓${NC}"
            ((PASSED++))
        else
            echo -e "${RED}FAIL (should not have parsed) ✗${NC}"
            ((FAILED++))
        fi
    else
        if [ "$expected" = "FAIL" ]; then
            echo -e "${GREEN}PASS (correctly failed to parse) ✓${NC}"
            ((PASSED++))
        else
            echo -e "${RED}FAIL (should have parsed) ✗${NC}"
            ((FAILED++))
            echo -e "${RED}Parse error details:${NC}"
            lof parse "$filepath" 2>&1 | head -3
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
    echo -e "${GREEN}All parser tests passed! ${NC}"
    exit 0
else
    echo -e "${RED}$FAILED parser test(s) failed! ${NC}"
    exit 1
fi