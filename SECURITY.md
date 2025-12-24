# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in the Havklo SDK, please report it responsibly.

### How to Report

1. **Do NOT open a public issue** for security vulnerabilities
2. Email security concerns to: hitakshiarora@gmail.com
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

### What to Expect

- **Acknowledgment**: Within 48 hours
- **Initial Assessment**: Within 7 days
- **Resolution Timeline**: Depends on severity
  - Critical: 24-48 hours
  - High: 7 days
  - Medium: 30 days
  - Low: Next release

### Scope

Security issues we care about:

- **Authentication bypass** in private channel handling
- **Secret leakage** in logs, errors, or debug output
- **Memory safety** issues (buffer overflows, use-after-free)
- **Denial of service** via malformed messages
- **Timing attacks** on authentication signatures

### Out of Scope

- Vulnerabilities in dependencies (report upstream)
- Kraken API security issues (report to Kraken)
- Social engineering attacks
- Physical security

## Security Design

### Secrets Handling

- API secrets are never logged
- Secrets use `secrecy` crate for zeroization
- Authentication is behind `auth` feature flag
- No secrets in error messages

### Network Security

- TLS-only WebSocket connections (`wss://`)
- Certificate validation enabled
- No downgrade to unencrypted connections

### Input Validation

- All Kraken messages are validated before processing
- Malformed JSON is rejected with clear errors
- Checksum validation on orderbook data

### Memory Safety

- Pure Rust implementation (no unsafe blocks in core logic)
- Bounded queues prevent memory exhaustion
- No unbounded allocations from network input

## Threat Model

### Trusted

- Kraken WebSocket servers
- Local system time
- Rust standard library

### Untrusted

- Network data (validated before use)
- User-provided configuration (validated)

### Assumptions

- TLS provides confidentiality and integrity
- Kraken servers are not malicious
- System random number generator is secure
