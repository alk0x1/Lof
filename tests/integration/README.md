# Integration Tests

This directory contains integration tests for the Lof compiler pipeline.

## Directory Structure

- `valid/` - Test files that should successfully parse, typecheck, and compile
- `invalid/` - Test files that should fail at some stage (parse, typecheck, or compile)

## Test Conventions

Each test file should include a comment header indicating expected behavior:

```lof
// SHOULD_PASS - File should parse, typecheck, and compile successfully
// SHOULD_FAIL - File should fail at some stage
// PARSE_FAIL - File should fail parsing
// TYPE_FAIL - File should pass parsing but fail typechecking
```

Alternatively, test expectations can be inferred from filenames:
- `*_valid.lof` - Should pass all stages
- `*_invalid.lof` - Should fail at some stage

## Running Tests

```bash
# Run all integration tests
./tests/scripts/runtypecheckertests.sh

# Run parser tests only
./tests/scripts/runparsertests.sh

# Run compiler tests only
./tests/scripts/runcompiletests.sh
```

## Adding New Tests

1. Create a `.lof` file in `valid/` or `invalid/`
2. Add appropriate comment markers
3. Run the test suite to verify behavior
4. Commit the test file
