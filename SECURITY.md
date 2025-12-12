# Security Policy

## Supported Versions

We provide security updates for the following versions of Control Path:

| Version | Supported          |
| ------- | ------------------ |
| Latest  | :white_check_mark: |
| < Latest | :x:                |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please report security vulnerabilities by emailing [INSERT SECURITY EMAIL] with the subject line "Security Vulnerability: [Brief Description]".

### What to Include

When reporting a vulnerability, please include:

- A description of the vulnerability
- Steps to reproduce the issue
- Potential impact of the vulnerability
- Suggested fix (if you have one)

### Response Timeline

- **Initial Response**: We will acknowledge receipt of your report within 48 hours
- **Status Update**: We will provide a status update within 7 days
- **Resolution**: We will work to resolve critical vulnerabilities as quickly as possible

### Disclosure Policy

- We will work with you to understand and resolve the issue quickly
- We will credit you for the discovery (unless you prefer to remain anonymous)
- We will not disclose the vulnerability publicly until a fix is available
- We will coordinate with you on the disclosure timeline

## Security Best Practices

When using Control Path:

- **Keep dependencies updated**: Regularly update your dependencies to receive security patches
- **Validate inputs**: Always validate user inputs and AST artifacts
- **Use signature verification**: Enable signature verification when loading AST artifacts from untrusted sources
- **Secure key management**: Store private keys securely and never commit them to version control
- **Review configurations**: Review flag definitions and deployment files before deploying to production

## Known Security Considerations

### AST Artifact Signing

Control Path supports optional Ed25519 signing of AST artifacts to prevent tampering and MITM attacks. When loading AST artifacts from URLs (CDN, object storage, etc.), we strongly recommend enabling signature verification.

### Expression Evaluation

The expression engine evaluates user-provided expressions. While expressions are compiled and validated, ensure that:

- Expression sources are trusted
- AST artifacts are loaded from trusted sources
- Signature verification is enabled for untrusted sources

### Default Fallbacks

Control Path follows a "Never Throws" policy, always returning default values on errors. Ensure that default values are appropriate for your use case and that errors are properly logged.

## Security Updates

Security updates will be:

- Released as patch versions (e.g., 1.0.1)
- Documented in release notes
- Prioritized over feature development

## Contact

For security-related questions or concerns, please contact [INSERT SECURITY EMAIL].

---

**Thank you for helping keep Control Path and its users safe!**

