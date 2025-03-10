# GitHub Repository Rulesets for RustyGPT

This document outlines the recommended repository rulesets for the RustyGPT project. These rulesets help enforce code quality, ensure proper review processes, and maintain security standards.

## Setting Up Repository Rulesets

To implement these rulesets:

1. Go to your GitHub repository
2. Navigate to Settings > Rules > Rulesets
3. Click "New rule" to create each ruleset
4. Configure the target branches and rules as described below
5. Set the enforcement level (active or evaluate)
6. Save the ruleset

## Recommended Rulesets

### 1. Main Branch Protection Ruleset

**Target:** `main` branch

**Rules to include:**

- **Require pull requests:**

  - Require at least 1 approval
  - Dismiss stale approvals when new commits are pushed
  - Require approval from code owners
  - Require review from someone other than the author

- **Required status checks:**

  - `build` (from CI workflow)
  - `clippy` (from Lint workflow)
  - `rustfmt` (from Lint workflow)
  - `audit` (from Lint workflow)

- **Require signed commits:** Ensure commit authenticity

- **Require linear history:** Prevents merge commits for cleaner history

- **Block force pushes:** Protect commit history

- **Restrict deletions:** Prevent accidental branch deletion

- **Apply to admins:** Ensure everyone follows the same rules

### 2. Release Branch Ruleset

**Target:** `release/*` branches

**Rules:**

- **Require pull requests:**

  - Require at least 2 approvals
  - Dismiss stale approvals when new commits are pushed
  - Require approval from code owners
  - Require review from someone other than the author

- **Required status checks:**

  - Same as main branch

- **Require signed commits:** Ensure commit authenticity

- **Require linear history:** Prevents merge commits for cleaner history

- **Block force pushes:** Protect commit history

- **Restrict deletions:** Prevent accidental branch deletion

### 3. Development Ruleset

**Target:** All other branches

**Rules:**

- **Required status checks:**

  - `build` (from CI workflow)
  - `clippy` (from Lint workflow)
  - `rustfmt` (from Lint workflow)

- **Allow force pushes:** For development convenience

### 4. Tag Protection Ruleset

**Target:** `v*` (version tags)

**Rules:**

- **Restrict tag creation:** Limit to maintainers
- **Require signed tags:** Ensure tag authenticity
- **Block tag deletion:** Preserve release history

## Special Considerations

### Backend and Shared Code

As mentioned, the backend and shared projects are particularly sensitive. Consider:

1. Adding specific code owners in the CODEOWNERS file for these directories
2. Requiring additional approvals for changes to these directories
3. Setting up custom rulesets for branches that modify these directories

### Sensitive Files

Consider adding additional protection for sensitive files:

- `.github/workflows/*`
- `Cargo.toml`
- `docker-compose.yaml`
- `Dockerfile`

## Bypass Options

For emergency situations, consider setting up bypass lists:

1. Define specific users or teams who can bypass certain rules
2. Limit bypass capabilities to specific actions (e.g., allowing force pushes in emergencies)
3. Ensure bypasses are logged and reviewed

## Monitoring and Enforcement

1. Regularly review ruleset effectiveness
2. Adjust rules as the project evolves
3. Consider using evaluation mode for new rules before enforcing them

These rulesets will help ensure code quality, maintain security standards, and provide a structured contribution process for the RustyGPT project.
