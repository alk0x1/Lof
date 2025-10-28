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

LOF_BIN="$PROJECT_ROOT/target/debug/lof"
if [ ! -x "$LOF_BIN" ]; then
    LOF_BIN="$(command -v lof || true)"
fi

if [ -z "$LOF_BIN" ] || [ ! -x "$LOF_BIN" ]; then
    echo -e "${RED}Error: unable to find the 'lof' binary. Build the project first.${NC}"
    exit 1
fi

echo -e "${BLUE}Running R1CS compilation tests on all .lof files in $TESTS_DIR${NC}"
echo "=================================================="

if [ ! -d "$TESTS_DIR" ]; then
    echo -e "${RED}Error: Tests directory not found: $TESTS_DIR${NC}"
    exit 1
fi

should_compile_r1cs() {
    local file="$1"
    if head -5 "$file" | grep -q "// R1CS_PASS\|// COMPILE_PASS\|// SHOULD_COMPILE"; then
        return 0
    elif head -5 "$file" | grep -q "// R1CS_FAIL\|// COMPILE_FAIL\|// SHOULD_NOT_COMPILE"; then
        return 1
    elif head -5 "$file" | grep -q "// SHOULD_FAIL\|// FAIL\|// EXPECT_FAIL\|// INVALID"; then
        return 1
    elif head -5 "$file" | grep -q "// SHOULD_PASS\|// PASS\|// EXPECT_PASS\|// VALID"; then
        return 0
    else
        if [[ "$file" == *"invalid"* ]] || [[ "$file" == *"fail"* ]] || [[ "$file" == *"error"* ]]; then
            return 1
        else
            return 0
        fi
    fi
}

TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

for filepath in "$TESTS_DIR"/valid/*.lof "$TESTS_DIR"/invalid/*.lof; do
    if [ ! -e "$filepath" ]; then
        continue
    fi
    
    filename=$(basename "$filepath")
    testname="${filename%.lof}"
    
    temp_filepath="$TEMP_DIR/$filename"
    cp "$filepath" "$temp_filepath"
    
    if should_compile_r1cs "$filepath"; then
        expected="COMPILE"
    else
        expected="FAIL"
    fi
    
    echo -n "Compiling $filename to R1CS (expect $expected): "
    
    if "$LOF_BIN" compile "$temp_filepath" >/dev/null 2>&1; then
        if [ "$expected" = "COMPILE" ]; then
            echo -e "${GREEN}PASS ✓${NC}"
            ((PASSED++))
            
            r1cs_file="$TEMP_DIR/${testname}.r1cs"
            if [ -f "$r1cs_file" ]; then
                file_size=$(stat -f%z "$r1cs_file" 2>/dev/null || stat -c%s "$r1cs_file" 2>/dev/null || echo "unknown")
                echo -e "  ${BLUE}→ Generated R1CS file (${file_size} bytes)${NC}"
            else
                echo -e "  ${YELLOW}→ Warning: R1CS file not found${NC}"
            fi
        else
            echo -e "${RED}FAIL (should not have compiled) ✗${NC}"
            ((FAILED++))
        fi
    else
        if [ "$expected" = "FAIL" ]; then
            echo -e "${GREEN}PASS (correctly failed to compile) ✓${NC}"
            ((PASSED++))
        else
            echo -e "${RED}FAIL (should have compiled) ✗${NC}"
            ((FAILED++))
            echo -e "${RED}Compilation error details:${NC}"
            "$LOF_BIN" compile "$temp_filepath" 2>&1 | head -5
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

echo -e "${BLUE}Test Summary:${NC}"
echo -e "  • Lexical analysis and parsing"
echo -e "  • Type checking and linearity validation"
echo -e "  • R1CS constraint generation"
echo -e "  • Binary R1CS file output"

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All R1CS compilation tests passed!${NC}"
    exit 0
else
    echo -e "${RED}$FAILED R1CS compilation test(s) failed!${NC}"
    exit 1
fi
