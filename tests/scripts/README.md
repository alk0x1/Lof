# Test Scripts

Shell scripts for running integration tests.

## Available Scripts

- `runparsertests.sh` - Tests parsing stage only
- `runtypecheckertests.sh` - Tests full typechecking pipeline
- `runcompiletests.sh` - Tests complete compilation to R1CS

## Test Data Location

Integration test files are located in:
- `tests/integration/valid/` - Tests that should pass
- `tests/integration/invalid/` - Tests that should fail

## Usage

```bash
cd tests/scripts
./runtypecheckertests.sh
```

All scripts will:
1. Discover test files automatically
2. Determine expected outcomes from comments/filenames
3. Run the appropriate `lof` command
4. Report pass/fail for each test
5. Exit with code 0 if all tests pass, 1 if any fail
