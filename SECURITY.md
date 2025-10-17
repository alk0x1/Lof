# Security Policy

## Reporting a Vulnerability

Security is critical for zero-knowledge proof systems. If you discover a security vulnerability in Lof, please report it responsibly.

### How to Report

**DO NOT** create a public GitHub issue for security vulnerabilities.

Instead, please email:
- **Email:** security@lof-lang.org (or create a private security advisory on GitHub)

### What to Include

Please include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)
- Your contact information

### Response Timeline

- **Initial Response:** Within 48 hours
- **Status Update:** Within 7 days
- **Fix Timeline:** Depends on severity (critical issues within 2 weeks)

### Disclosure Policy

We follow coordinated disclosure:
1. You report the vulnerability privately
2. We confirm and develop a fix
3. We release the fix
4. We publicly disclose the vulnerability (with credit to you, if desired)

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Security Best Practices

When using Lof:

1. **Always audit generated circuits** before production use
2. **Verify R1CS constraints** match your expectations
3. **Use trusted setup ceremonies** for production deployments
4. **Keep dependencies updated** - run `cargo deny check`
5. **Test thoroughly** - use the verification suite

## Known Limitations

Lof is research software (v0.1.0). Known limitations:

- Type system implementation is evolving
- Some advanced features are experimental
- Performance optimizations ongoing

## Security Features

Lof provides:

- **Compile-time type safety** - Catch errors before proof generation
- **Visibility tracking** - Prevent witness leaks to public outputs
- **Explicit constraints** - No hidden constraint generation
- **Verification suite** - Differential testing against Circom

## Hall of Fame

Security researchers who responsibly disclosed vulnerabilities:

(None yet - you could be first!)

## Questions?

For security questions that don't involve a vulnerability, open a GitHub Discussion.
