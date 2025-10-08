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

echo -e "${BLUE}Running R1CS compilation tests on all .lof files in $TESTS_DIR${NC}"
echo "=================================================="

# Check if tests directory exists
if [ ! -d "$TESTS_DIR" ]; then
    echo -e "${RED}Error: Tests directory not found: $TESTS_DIR${NC}"
    exit 1
fi

# Function to check if test should compile to R1CS successfully
should_compile_r1cs() {
    local file="$1"
    # Look for comment patterns in the first few lines for R1CS compilation expectations
    if head -5 "$file" | grep -q "// R1CS_PASS\|// COMPILE_PASS\|// SHOULD_COMPILE"; then
        return 0  # Should compile to R1CS successfully
    elif head -5 "$file" | grep -q "// R1CS_FAIL\|// COMPILE_FAIL\|// SHOULD_NOT_COMPILE"; then
        return 1  # Should fail R1CS compilation
    elif head -5 "$file" | grep -q "// SHOULD_FAIL\|// FAIL\|// EXPECT_FAIL\|// INVALID"; then
        return 1  # Type-level failures typically prevent R1CS generation
    elif head -5 "$file" | grep -q "// SHOULD_PASS\|// PASS\|// EXPECT_PASS\|// VALID"; then
        return 0  # Should compile successfully
    else
        # Default behavior - try to infer from filename
        if [[ "$file" == *"invalid"* ]] || [[ "$file" == *"fail"* ]] || [[ "$file" == *"error"* ]]; then
            return 1  # Should fail compilation
        else
            return 0  # Should compile successfully
        fi
    fi
}

# Create temporary directory for R1CS outputs
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Find all .lof files in both valid and invalid directories
for filepath in "$TESTS_DIR"/valid/*.lof "$TESTS_DIR"/invalid/*.lof; do
    # Check if any .lof files exist
    if [ ! -e "$filepath" ]; then
        continue
    fi
    
    # Extract just the filename
    filename=$(basename "$filepath")
    testname="${filename%.lof}"
    
    # Copy file to temp directory to avoid cluttering original directory
    temp_filepath="$TEMP_DIR/$filename"
    cp "$filepath" "$temp_filepath"
    
    # Determine expected outcome for R1CS compilation
    if should_compile_r1cs "$filepath"; then
        expected="COMPILE"
    else
        expected="FAIL"
    fi
    
    echo -n "Compiling $filename to R1CS (expect $expected): "
    
    # Run the R1CS compilation test
    if lof compile "$temp_filepath" >/dev/null 2>&1; then
        # Compilation succeeded
        if [ "$expected" = "COMPILE" ]; then
            echo -e "${GREEN}PASS ✓${NC}"
            ((PASSED++))
            
            # Optionally verify R1CS file was created
            r1cs_file="$TEMP_DIR/${testname}.r1cs"
            if [ -f "$r1cs_file" ]; then
                # Get R1CS file size for additional info
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
        # Compilation failed
        if [ "$expected" = "FAIL" ]; then
            echo -e "${GREEN}PASS (correctly failed to compile) ✓${NC}"
            ((PASSED++))
        else
            echo -e "${RED}FAIL (should have compiled) ✗${NC}"
            ((FAILED++))
            # Show error details for unexpected compilation failures
            echo -e "${RED}Compilation error details:${NC}"
            lof compile "$temp_filepath" 2>&1 | head -5
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

# Show summary of what was tested
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