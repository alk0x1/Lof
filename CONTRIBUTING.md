# Contributing to Lof

Thank you for your interest in contributing to Lof! This document provides guidelines and information for contributors.

## Code of Conduct

This project adheres to a Code of Conduct. By participating, you are expected to uphold this code. Please report unacceptable behavior to [conduct@lof-lang.org].

## How Can I Contribute?

### Reporting Bugs

Before creating a bug report, please check the existing issues to avoid duplicates.

**Good bug reports include:**
- Clear, descriptive title
- Steps to reproduce
- Expected vs actual behavior
- Lof version (`lof --version`)
- Minimal example code
- System information (OS, Rust version)

**Use this template:**

```markdown
## Description
Brief description of the bug

## Steps to Reproduce
1. Create file `example.lof` with content...
2. Run `lof compile example.lof`
3. Observe error...

## Expected Behavior
What should happen

## Actual Behavior
What actually happens

## Environment
- Lof version: 0.1.0
- OS: Ubuntu 22.04
- Rust version: 1.75.0

## Additional Context
Any other relevant information
```

### Suggesting Enhancements

Enhancement suggestions are tracked as GitHub issues.

**Good enhancement suggestions include:**
- Clear use case
- Proposed syntax (for language features)
- Expected behavior
- Why this improves Lof
- Potential implementation approach (optional)

### Pull Requests

1. **Fork the repository** and create a branch from `main`
2. **Make your changes** following our code style
3. **Add tests** for new functionality
4. **Update documentation** if needed
5. **Run the test suite** to ensure nothing breaks
6. **Submit a pull request**

## Development Workflow

### Setup Development Environment

```bash
# Clone your fork
git clone https://github.com/yourusername/lof.git
cd lof

# Add upstream remote
git remote add upstream https://github.com/lof-lang/lof.git

# Install development dependencies
rustup component add clippy rustfmt
cargo install cargo-deny

# Optional: for verification tests
npm install -g snarkjs
```

### Development Commands

```bash
# Quick development cycle (format + build + test)
make dev

# Before committing
make pre-commit    # format-check + lint + test-unit

# Before pushing
make pre-push      # All tests including integration

# Full validation
make test-all      # All tests
make verify-all    # Verification suite
```

### Code Style

We use rustfmt for consistent code formatting:

```bash
# Format all code
make format

# Check formatting
make format-check
```

**Style guidelines:**
- Max line length: 100 characters
- Use 4 spaces for indentation
- Group imports by std/external/internal
- Use field init shorthand
- Comment complex logic
- Write descriptive variable names

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks
- `perf`: Performance improvements

**Examples:**

```
feat(parser): add support for tuple destructuring

Implements pattern matching on tuples with nested patterns.
Includes exhaustiveness checking and type inference.

Closes #123
```

```
fix(typechecker): prevent witness leak to public outputs

The typechecker now correctly enforces visibility constraints,
preventing private witnesses from flowing to public outputs.

Fixes #456
```

### Testing Guidelines

All code changes should include tests:

**Unit tests** (in `src/*.rs` files):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Test implementation
    }
}
```

**Integration tests** (in `tests/` directory):
- Parser tests in `tests/parser_tests.rs`
- Typechecker tests in `tests/typechecker_tests.rs`
- R1CS tests in `tests/r1cs_tests.rs`

**Verification tests** (in `verification/`):
- Add paired `.lof` and `.circom` files
- Create test vectors in `test_cases/`
- Run `make verify-all` to validate

### Adding New Language Features

When adding language features, follow this workflow:

1. **Design**
   - Discuss in GitHub issue first
   - Document syntax and semantics
   - Consider type system implications

2. **Implementation Order**
   - Update lexer (`lof/src/lexer.rs`)
   - Update AST (`lof/src/ast.rs`)
   - Update parser (`lof/src/parser.rs`)
   - Update typechecker (`lof/src/typechecker.rs`)
   - Update R1CS compiler (`lof/src/r1cs.rs`)

3. **Testing**
   - Add unit tests in each module
   - Add integration tests
   - Add verification tests (if applicable)
   - Update documentation

4. **Documentation**
   - Update syntax documentation (`docs/syntax.md`)
   - Add examples
   - Update CHANGELOG.md

### Code Review Process

All submissions require review:

1. **Automated Checks**
   - CI must pass (tests, linting, formatting)
   - No new compiler warnings
   - Documentation builds successfully

2. **Human Review**
   - Code quality and style
   - Test coverage
   - Documentation completeness
   - Design appropriateness

3. **Merge Requirements**
   - At least one approving review
   - All CI checks passing
   - No unresolved conversations
   - Up-to-date with main branch

## Project Structure

```
lof/
├── lof/              # Core compiler library
│   ├── src/
│   │   ├── lexer.rs       # Tokenization
│   │   ├── parser.rs      # Parsing
│   │   ├── ast.rs         # Abstract syntax tree
│   │   ├── typechecker.rs # Type checking
│   │   ├── r1cs.rs        # R1CS compilation
│   │   └── lib.rs         # Public API
│   └── tests/        # Integration tests
│
├── cli/              # Command-line interface
├── lofit/            # ZK proving toolkit
├── lof-codegen/      # Code generation
├── verification/     # Differential testing
└── docs/             # Documentation
```

## Documentation

When updating documentation:

- Keep language simple and clear
- Include code examples
- Update table of contents if adding sections
- Test all code examples
- Check for broken links

## Performance

When optimizing:

- Benchmark before and after
- Profile to find bottlenecks
- Document why optimizations are needed
- Prefer clarity over micro-optimizations
- Consider constraint count impact

## Security

Security is critical for ZK circuits:

- Never compromise type safety for convenience
- Validate all inputs
- Document security assumptions
- Consider information flow implications
- Report security issues privately (see SECURITY.md)

## Community

- **GitHub Discussions**: General questions and ideas
- **GitHub Issues**: Bug reports and feature requests
- **Pull Requests**: Code contributions

## Recognition

Contributors are recognized in:
- CHANGELOG.md for their contributions
- GitHub contributors page
- Release notes for significant features

## Questions?

If you have questions:

1. Check existing documentation
2. Search closed issues
3. Ask in GitHub Discussions
4. Reach out to maintainers

## License

By contributing to Lof, you agree that your contributions will be licensed under the same terms as the project (MIT/Apache-2.0 dual license).

---

Thank you for contributing to Lof and helping make ZK circuit development safer!
