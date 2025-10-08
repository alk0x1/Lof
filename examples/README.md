# Lof Language Examples

This directory contains example programs demonstrating Lof language features.

## Directory Structure

- `01_functions/` - Function definitions and calls
- `02_let_binding/` - Let expressions and variable binding
- `03_pattern_matching/` - Pattern matching on types
- `04_recursion/` - Recursive function examples

## Running Examples

To run any example:

```bash
# Parse only
lof parse examples/01_functions/functions.lof

# Type check
lof check examples/01_functions/functions.lof

# Compile to R1CS
lof compile examples/01_functions/functions.lof
```

## Contributing Examples

When adding new examples:
1. Create a numbered directory (e.g., `05_new_feature/`)
2. Add a descriptive `.lof` file
3. Include a README explaining the feature
4. Add comments in the code explaining key concepts
