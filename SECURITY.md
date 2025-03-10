# Security Policy

## Supported Versions

Use this section to tell people about which versions of your project are currently being supported with security updates.

| Version | Supported          |
| ------- | ------------------ |
| 1.0.x   | :white_check_mark: |
| < 1.0   | :x:                |

## Reporting a Vulnerability

We take the security of RustyGPT seriously. If you believe you've found a security vulnerability, please follow these steps:

### Where to Report

Please **DO NOT** report security vulnerabilities through public GitHub issues.

Instead, please report them via email to [INSERT SECURITY EMAIL]. If possible, encrypt your message with our PGP key (details below).

### What to Include

When reporting a vulnerability, please include as much information as possible:

- Type of issue (e.g., buffer overflow, SQL injection, cross-site scripting, etc.)
- Full paths of source file(s) related to the manifestation of the issue
- The location of the affected source code (tag/branch/commit or direct URL)
- Any special configuration required to reproduce the issue
- Step-by-step instructions to reproduce the issue
- Proof-of-concept or exploit code (if possible)
- Impact of the issue, including how an attacker might exploit the issue

### Response Process

After you have submitted a vulnerability report, you can expect:

1. **Acknowledgment**: We will acknowledge receipt of your vulnerability report within 48 hours.
2. **Verification**: Our security team will work to verify the vulnerability and its impact.
3. **Remediation**: We will develop a fix and test it.
4. **Disclosure**: Once the vulnerability has been fixed, we will publish a security advisory.

### Disclosure Policy

When we receive a security bug report, we will:

- Confirm the problem and determine the affected versions.
- Audit code to find any potential similar problems.
- Prepare fixes for all releases still under maintenance.
- Release new versions and update the security advisory.

## Security Best Practices for Contributors

If you're contributing to RustyGPT, please follow these security best practices:

1. **Keep dependencies up to date**: Use the latest stable versions of dependencies.
2. **Follow secure coding practices**: Validate all inputs, especially those from external sources.
3. **Avoid hardcoded secrets**: Never commit API keys, passwords, or other secrets to the repository.
4. **Use parameterized queries**: When working with databases, use parameterized queries to prevent SQL injection.
5. **Implement proper authentication and authorization**: Ensure that users can only access resources they are authorized to.

## Security-Related Configuration

RustyGPT requires certain security-related configuration to operate securely:

1. **Environment Variables**: Sensitive information should be stored in environment variables, not in code.
2. **HTTPS**: Always use HTTPS in production environments.
3. **Database Security**: Ensure your PostgreSQL database is properly secured.

## PGP Key

For encrypted communication, you can use our PGP key:

```
[INSERT PGP KEY HERE]
```

Thank you for helping keep RustyGPT and its users safe!
