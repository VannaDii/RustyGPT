# RustyGPT Copilot Instructions

This document defines strict, high-quality guidelines for GitHub Copilot to follow when generating or editing code for the RustyGPT project. All contributions must be idiomatic, efficient, safe, maintainable, and fully documented. These standards ensure correctness, performance, and long-term project sustainability.

> **Workspace Members**: See the [workspace Cargo.toml](../Cargo.toml) for crate membership and layout.

## Command Execution

You have many tools available to you for command execution. You should primarily use the `#execute_command` tool without providing a `timeout` attribute, and ensuring the command you want to run is prefixed by a `cd` command that specifies the correct directory for the command to execute successfully. This ensures that the command runs in the appropriate context and can access the necessary resources.

### Example Commands

```sh
cd ~/Source/tubarr && cargo check
```

```json
{
  "command": "cd ~/Source/tubarr && cargo check"
}
```

**Notice there is never a `timeout` attribute.** This is because the command will run until it completes, and you should not set a timeout for it.

### Test Commands

When running tests, you MUST ONLY use the `cargo test` command and it MUST be run in the workspace root! This is essential and not open to negotiation!

---

## 1. Rust Code Design & Best Practices

### Modularity & Separation of Concerns

- Organize code into cleanly separated crates and modules.
- Each function and module must have a single responsibility.
- Favor composition over inheritance; encapsulate logic.

### Idiomatic Rust

- Use ownership and borrowing effectively; avoid unnecessary `clone()`.
- Favor `match`, iterators, pattern matching, and expressive enums.
- Use `Option` and `Result` idiomatically.
- Prefer `?` for error propagation.

### Error Handling

- Return [`Result<T, E>`](https://doc.rust-lang.org/std/result/) from fallible functions.
- Use [`thiserror`](https://docs.rs/thiserror) for defining error enums.
- Use [`anyhow`](https://docs.rs/anyhow) for application-level error contexts.
- Never use `.unwrap()` or `.expect()` outside of tests or explicitly safe contexts.

---

## 2. Commenting & Documentation Standards

### Docstrings (`///`)

- Every public item (fn, struct, enum, trait, mod) must have a docstring.
- Format references like: [`String`](https://doc.rust-lang.org/std/string/struct.String.html) or [`MyType`](crate::path::to::MyType).
- Use `# Arguments`, `# Returns`, `# Errors`, and `# Examples` sections as needed.

**Example:**

```rust
/// Attempts to log in a user by validating their credentials.
///
/// # Arguments
/// * `username` - A [`String`](https://doc.rust-lang.org/std/string/struct.String.html)
/// * `password` - A raw user password
///
/// # Returns
/// A valid [`Session`](crate::auth::Session) if credentials are valid.
///
/// # Errors
/// Returns an error if credentials are invalid or the DB query fails.
```

### Module Docs (`//!`)

- All modules must begin with a `//!` comment explaining their purpose and main structs/functions.

### Inline Comments (`//`)

- Only for clarifying logic, not restating what the code does.
- Keep current with code changes.

---

## 3. Memory Efficiency & Performance

### Borrowing & Allocation

- Pass `&T` or `&mut T` unless ownership is required.
- Accept slices (`&[T]`) instead of `Vec<T>` when possible.
- Avoid `clone()` unless necessary; use `Cow` where applicable.

### Data Structures

- Use `Vec`, `HashMap`, `HashSet` from `std::collections` appropriately.
- For concurrency, prefer `DashMap` over `Mutex<HashMap>`.

### Concurrency & Async

- Use `tokio` for async work.
- Share data across threads with `Arc`.
- Lock only what's necessary with `Mutex`, `RwLock`.
- Avoid blocking code in async contexts.

---

## 4. Testing & Coverage

### Unit Testing

- Every function must be tested: happy path, all error paths.
- Use `#[cfg(test)]` modules co-located with implementation.

### Integration Testing

- Place in `/tests`; test end-to-end flows across crates.

### Code Coverage

- Use `cargo llvm-cov` to measure:

```sh
cargo llvm-cov --workspace --html --output-dir .coverage && open .coverage/index.html
```

- Maintain **100% coverage** of all logic.

### Linting

- Run formatting and Clippy checks in CI:

```sh
cargo fmt --all
cargo clippy --all-features -- -D warnings
```

- No warnings allowed in production.

---

## 5. Documentation & Commit Style

### Documentation Updates

- Update all comments, module docs, and examples with every code change.
- Never leave outdated or misleading documentation.

### Commit Messages (Conventional Commits)

Use the format:

```sh
<type>(<scope>): <short summary>

<detailed body if needed>

Refs: #issue
```

- `feat`: new feature
- `fix`: bug fix
- `docs`: documentation only
- `refactor`: code change not fixing a bug or adding a feature
- `test`: test changes only
- `chore`: infra or dependency update

**Example:**

```txt
feat(api): add session token refresh logic

Adds a new endpoint `/auth/refresh` that renews JWTs.
Includes full test coverage and updates OpenAPI docs.

Refs: #42
```

### Pull Request Descriptions

Use this template:

```
# <type(scope)>: <title>

## Summary
- What this PR does.

## Motivation & Context
- Why the change is needed.

## Changes
- Bullet list of changes made

## Testing
- Describe testing process and coverage level

## Related Issues
- Closes # or Refs #

## Additional Notes
- Anything else reviewers should know
```

---

## 6. Version Management

### Executable Crates

- Maintain synchronized versions across all executable crates.
- Use a shared `workspace.version` in the root Cargo.toml if possible.

### SemVer Compliance

- MAJOR: breaking changes
- MINOR: new, backward-compatible functionality
- PATCH: backward-compatible bug fixes

### Dependency Sync

- Align shared dependency versions across crates.
- Regularly run `cargo update` and verify compatibility.

---

## 7. UI/UX Standards (Yew + TailwindCSS + DaisyUI)

### CSS & Components

- Use [TailwindCSS](https://github.com/tailwindlabs/tailwindcss) for styling.
- Use [DaisyUI](https://github.com/saadeghi/daisyui) for UI components.
- Reference latest [DaisyUI v5 examples](https://github.com/saadeghi/daisyui/tree/v5/docs/src/routes/components).
- Keep component structure clean and responsive.
- Prefer functional, accessible HTML and ARIA patterns.

---

## 8. Copilot Enforcement Checklist

Copilot **must always**:

- Follow all idiomatic Rust practices.
- Avoid unnecessary clones or allocations.
- Maintain 100% unit test coverage.
- Write happy-path and error-path tests.
- Add or update all related doc comments.
- Format all commit messages per Conventional Commits.
- Write detailed PR descriptions using the defined template.
- Keep executable crate versions in sync using SemVer.
- Format and lint all code.
- Use Tailwind + DaisyUI for any frontend code.

---

## 9. Critical Rust Concepts to Reinforce

- Safety: use `unsafe` only with full justification and comments.
- Memory: prefer `&T`, avoid clones, use `Cow`, `Option`, `Result` idiomatically.
- Testing: fully test every logic path.
- Docs: every public item documented, all types reference-linked.
- Performance: iterate efficiently, async properly, structure data intentionally.

---

_End of RustyGPT Copilot Instructions._
