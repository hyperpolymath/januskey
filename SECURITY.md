# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| latest  | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in this project, please report it responsibly.

### How to Report

1. **Do NOT** create a public GitHub issue for security vulnerabilities
2. Email: rhodium-standard@proton.me
3. Or use GitHub's private vulnerability reporting if enabled

### Include in Your Report

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (optional)

### Response Timeline

- **Acknowledgment**: Within 48 hours
- **Initial Assessment**: Within 7 days
- **Resolution Target**: Within 30 days for critical issues

### What to Expect

1. Acknowledgment of your report
2. Regular updates on progress
3. Credit in release notes (unless you prefer anonymity)
4. Coordinated disclosure timeline

## Security Measures

### Supply Chain Security

- All GitHub Actions pinned to SHA hashes
- SPDX license headers on source files
- Dependency auditing via Scorecard

### Code Security

- HTTPS only for all external URLs
- No hardcoded secrets
- Automated secret scanning via TruffleHog

## Scope

This security policy applies to:
- The januskey project
- Official releases
- Documentation

## Out of Scope

- Third-party forks or modifications
- Vulnerabilities in dependencies (report to respective maintainers)
