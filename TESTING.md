# Lof Testing Strategy

This document describes the comprehensive 3-tier testing strategy for the Lof language.

## Overview

The Lof project uses a layered testing approach to ensure correctness, language semantics, and mathematical soundness:

1. **Tier 1: Fast Checks** (2-3 min) - Unit tests, linting, build verification
2. **Tier 2: Integration Tests** (5-10 min) - Parser, typechecker, and compiler tests
3. **Tier 3: Mathematical Verification** (15-30 min) - Circom equivalence testing

## Quick Start

```bash
# Install dependencies
make install

# Run fast checks (recommended before every commit)
make test-fast

# Run integration tests
make test-integration

# Run verification (requires circom and snarkjs)
make verify-quick

# Run everything
make test-all
```

## Tier 1: Fast Checks

**Purpose:** Catch basic errors quickly during development

**What's tested:**
- Rust unit tests (lexer, parser, typechecker, R1CS generator)
- Code formatting (rustfmt)
- Linting (clippy)
- Build verification

**Location:** `lof/tests/*.rs`

**Run locally:**
```bash
make test-unit          # Unit tests only
make lint              # Clippy
make format-check      # Format check
make test-fast         # All of the above
```

**CI:** Runs on every push to any branch

## Tier 2: Integration Tests

**Purpose:** Verify end-to-end compiler behavior with real Lof programs

**What's tested:**
- Parser correctness (syntax validation)
- Type system (type checking and linearity analysis)
- R1CS compilation (constraint generation)

**Location:** `tests/integration/`
- `valid/` - Programs that should pass all stages
- `invalid/` - Programs that should fail at some stage

**Test conventions:**
- Files named `*_valid.lof` should pass
- Files named `*_invalid.lof` should fail
- Comment markers override filename conventions:
  - `// SHOULD_PASS` - Must pass all stages
  - `// SHOULD_FAIL` - Must fail at some stage
  - `// PARSE_FAIL` - Must fail parsing
  - `// TYPE_FAIL` - Must fail type checking

**Run locally:**
```bash
make test-parser        # Parser tests only
make test-typecheck     # Typechecker tests only
make test-compile       # Compiler tests only
make test-integration   # All integration tests
```

**CI:** Runs on every push to any branch

**Adding new tests:**
1. Create a `.lof` file in `tests/integration/valid/` or `tests/integration/invalid/`
2. Add appropriate comment markers
3. Run the test suite to verify behavior
4. Commit the test file

## Tier 3: Mathematical Verification

**Purpose:** Verify that Lof's R1CS output is mathematically equivalent to Circom

**What's tested:**
- R1CS constraint generation matches Circom
- Witness computation produces identical results
- Mathematical correctness across all language features

**Location:** `verification/`
- `circuits/` - Paired `.lof` and `.circom` implementations
- `test_cases/` - JSON test vectors with inputs/outputs
- `scripts/` - Verification pipeline automation

**Test categories:**
- `01_basic/` - Basic multiplication
- `02_arithmetic/` - Addition, subtraction
- `03_comparisons/` - Equality, less-than
- `04_let_bindings/` - Simple and nested let expressions
- `05_complex/` - Compound operations, multiple witnesses

**Verification process:**
1. Compile circuit in both Lof and Circom to R1CS
2. Generate witnesses for test inputs
3. Compare outputs for mathematical equivalence

**Run locally:**
```bash
# Prerequisites
sudo apt install circom  # or build from source
npm install -g snarkjs

# Run verification
make verify-quick       # Quick smoke test
make verify-add         # Verify addition circuit
make verify-all         # Full verification suite
```

**CI:**
- Smoke test runs on every PR to master/v1-stable-release
- Full suite runs nightly and on manual trigger

**Adding new verification tests:**
1. Create paired files: `verification/circuits/XX_category/name.lof` and `name.circom`
2. Add test cases: `verification/test_cases/XX_category/name_tests.json`
3. Run: `cd verification/scripts && ./compile_both.sh name`
4. Verify: `python3 compare_results.py name`

## GitHub Actions CI/CD

All three tiers are automated via GitHub Actions:

- **[tier1-fast-checks.yml](.github/workflows/tier1-fast-checks.yml)** - Unit tests, linting, build
- **[tier2-integration-tests.yml](.github/workflows/tier2-integration-tests.yml)** - Integration tests
- **[tier3-verification.yml](.github/workflows/tier3-verification.yml)** - Circom verification

**Workflow triggers:**
- Tier 1 & 2: Every push and PR
- Tier 3: PRs to main branches, nightly, manual

**Branch protection:**
- All checks must pass before merging to `master` or `v1-stable-release`

## Local Development Workflow

### Before committing:
```bash
make pre-commit
```

### Before pushing:
```bash
make pre-push
```

### Quick development cycle:
```bash
make dev  # format + build + unit tests
```

### Full local validation:
```bash
make ci  # Everything except verification
```

## Test Coverage

Current test coverage (as of migration):
- **Unit tests:** 7 test files in `lof/tests/`
- **Integration tests:** 26 test files (16 valid, 10 invalid)
- **Verification tests:** 9 circuits across 5 categories
- **Examples:** 4 example programs

## Troubleshooting

### Tests failing after language changes?

1. **Check unit tests first:** `make test-unit`
   - Fix any broken unit tests in `lof/tests/`

2. **Run integration tests:** `make test-integration`
   - If parser tests fail: syntax may have changed
   - If typecheck tests fail: type system may have changed
   - If compile tests fail: R1CS generation may be broken

3. **Check verification:** `make verify-quick`
   - If verification fails: R1CS output may be incorrect

### Adding a new language feature?

1. Write unit tests in `lof/tests/`
2. Add integration tests in `tests/integration/valid/`
3. Add negative tests in `tests/integration/invalid/`
4. Create verification test in `verification/circuits/`
5. Run full test suite: `make test-all && make verify-all`

### Test taking too long?

- Run only what you need: `make test-unit` during development
- Use `make verify-quick` instead of `make verify-all`
- Full verification is automated in CI, run locally only when needed

## Best Practices

1. **Write tests first** - TDD approach helps catch regressions
2. **Run fast checks frequently** - `make test-fast` before every commit
3. **Use descriptive test names** - Clear filenames and comments
4. **Test both success and failure** - Include valid and invalid cases
5. **Verify mathematical correctness** - Run verification for R1CS changes
6. **Keep tests fast** - Integration tests should complete in seconds
7. **Document expected behavior** - Use comments to explain tricky cases

## Future Improvements

- [ ] Add test coverage reporting
- [ ] Implement property-based testing (e.g., with QuickCheck)
- [ ] Add performance benchmarking suite
- [ ] Create fuzzing tests for parser/typechecker
- [ ] Add constraint count regression tests
- [ ] Generate HTML test reports in CI

## Resources

- [CLAUDE.md](CLAUDE.md) - Project overview and commands
- [Makefile](Makefile) - All available make targets
- [Tests README](tests/integration/README.md) - Integration test conventions
- [Verification README](verification/circuits/README.md) - Verification structure (TODO)

## Questions?

Open an issue or check the project documentation.
