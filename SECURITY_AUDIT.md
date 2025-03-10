# Security Audit Findings

During the setup of the GitHub Actions workflows, a security audit was performed using `cargo audit`. This document outlines the security vulnerabilities and maintenance issues found in the project dependencies.

## Vulnerabilities

### 1. RSA Timing Side-Channel (RUSTSEC-2023-0071)

**Package**: `rsa` v0.9.7

**Severity**: High

**Description**: Due to a non-constant-time implementation, information about the private key is leaked through timing information which is observable over the network. An attacker may be able to use that information to recover the key.

**Impact**: This vulnerability could potentially allow an attacker to recover private keys through timing side-channel attacks.

**Recommendation**:

- Monitor the `rsa` crate for updates that address this vulnerability
- Consider using alternative RSA implementations that are constant-time
- If possible, limit the use of RSA operations to contexts where timing attacks are not feasible

## Unmaintained Dependencies

### 1. paste (RUSTSEC-2024-0436)

**Package**: `paste` v1.0.15

**Status**: Unmaintained

**Description**: The creator of the crate `paste` has stated in the `README.md` that this project is no longer maintained and has archived the repository.

**Recommendation**:

- Consider finding an alternative to the `paste` crate
- Monitor dependencies that rely on `paste` for updates that might replace it

### 2. proc-macro-error (RUSTSEC-2024-0370)

**Package**: `proc-macro-error` v1.0.4

**Status**: Unmaintained

**Description**: The maintainer of `proc-macro-error` seems to be unreachable, with no commits for 2 years, no releases pushed for 4 years, and no activity on the GitLab repo or response to email.

**Recommendation**:

- Consider replacing with one of these alternatives:
  - [manyhow](https://crates.io/crates/manyhow)
  - [proc-macro-error2](https://crates.io/crates/proc-macro-error2)
  - [proc-macro2-diagnostics](https://github.com/SergioBenitez/proc-macro2-diagnostics)

## Action Plan

1. **Short-term**:

   - Add these vulnerabilities to the project's issue tracker
   - Document the risks in the security policy
   - Implement any available workarounds

2. **Medium-term**:

   - Investigate replacing unmaintained dependencies
   - Monitor for updates to the `rsa` crate that address the vulnerability
   - Consider implementing additional security measures to mitigate the RSA timing attack risk

3. **Long-term**:
   - Establish a regular security audit process
   - Consider contributing to or forking unmaintained dependencies if they are critical to the project
   - Implement automated dependency updates with security checks

## Regular Monitoring

The GitHub Actions workflow has been configured to run security audits on every pull request and push to the main branch. This will help catch new vulnerabilities as they are discovered.

To run a security audit manually:

```bash
cargo install cargo-audit
cargo audit
```

This document should be updated as vulnerabilities are addressed or new ones are discovered.
