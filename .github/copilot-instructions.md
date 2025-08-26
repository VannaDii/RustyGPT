# RustyGPT Copilot Instructions

This document defines strict, high-quality guidelines for GitHub Copilot to follow when generating or editing code for the RustyGPT project. All contributions must be idiomatic, efficient, safe, maintainable, and fully documented. These standards ensure correctness, performance, and long-term project sustainability.

> **Workspace Members**: See the [workspace Cargo.toml](../Cargo.toml) for crate membership and layout.

## Rule Discovery & Integration

This document focuses on **technical standards and implementation guidelines**. For comprehensive guidance, also reference:

**[Agent Rules](prompts/agent-rules.prompt.md)** - Essential behavioral and interactive guidance including:
- Communication optimization and token efficiency
- Simplicity principles and YAGNI methodology
- Conversation summarization strategies
- Impact awareness and rule adherence
- Meta-cognitive improvement practices

Both rule sets work together: **technical standards** (this document) + **behavioral guidance** (agent rules) = complete development framework.

## Architecture Overview

RustyGPT is a full-stack Rust application with clear separation between frontend (Yew + WASM), backend (Axum + PostgreSQL), and shared components:

- **rustygpt-server**: Axum-based API server with OAuth, SSE streaming, and PostgreSQL integration
- **rustygpt-web**: Yew frontend with Yewdux state management, i18n support, and DaisyUI components
- **rustygpt-shared**: Common models and configuration used by both frontend and backend
- **rustygpt-cli**: Command-line interface for project management
- **rustygpt-tools**: Development utilities (`confuse` for concurrent tasks, `i18n-agent` for translation management)

### Key Architectural Patterns

- **State Management**: Backend uses `Arc<AppState>` with optional PostgreSQL pool; frontend uses Yewdux for global state
- **API Communication**: Frontend uses `RustyGPTClient` wrapper around reqwest for type-safe API calls
- **Real-time Updates**: Server-Sent Events (SSE) for streaming message chunks from `/api/stream/{user_id}`
- **Authentication**: OAuth flow with GitHub/Apple, JWT tokens, protected routes via middleware
- **Error Handling**: Shared error types in `rustygpt-shared/src/models/errors.rs`, consistent JSON error responses

# Knowledge Base

The authors of this project often use AI tools for ideation and research. Conversations are stored in `.chats` folder in the project root and are JSON formatted. Use the `.chat` files for workspace context and planning activities. Draw on these conversations to inform suggestions and implementation decisions.

## Development Workflow & Commands

### Just Commands (Primary Development Interface)

This project uses [Just](https://just.systems/) as the primary task runner. Key commands:

```sh
# Development workflow
just dev          # Start concurrent frontend/backend with confuse tool
just check        # Run fmt, check, and clippy on workspace
just fix          # Auto-fix formatting and clippy issues
just test         # Run all workspace tests
just coverage     # Generate HTML coverage report with cargo llvm-cov

# Individual components
just run-server   # Start backend server on port 8080
just watch-server # Watch mode for backend development
just build        # Build all workspace crates + frontend
just build-release # Release builds
```

### Critical Development Patterns

- **Testing**: Always run `just test` from workspace root - never from individual crates
- **Development**: Use `just dev` which runs `confuse` tool to manage frontend/backend simultaneously
- **Database**: PostgreSQL with connection pooling; see `docker-compose.yaml` for local setup

### Project-Specific Command Execution

You have many tools available to you for command execution. You should primarily use the `mcp_shell-exec_execute_command` tool with a `timeout` attribute set to 300000 milliseconds (5 minutes), and ensuring the command you want to run is prefixed by a `cd` command that specifies the correct directory for the command to execute successfully. This ensures that the command runs in the appropriate context and can access the necessary resources.

### Example Commands

```sh
cd ~/Source/rusty_gpt && cargo check
```

```json
{
  "command": "cd ~/Source/rusty_gpt && cargo check",
  "timeout": 300000
}
```

**Always include a `timeout` attribute set to 300000 milliseconds (5 minutes).** This provides a reasonable timeout for most development commands while preventing indefinite hangs.

### Test Commands

When running tests, you MUST ONLY use the `just test` command and it MUST be run in the workspace root! This is essential and not open to negotiation!

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
- Mock external dependencies with `mockall` or similar.
- Keep all tests isolated in `_test` files; use the format `*_test.rs`.

### Integration Testing

- Place in `/tests`; test end-to-end flows across crates.

### Code Coverage

- Use `just coverage` to measure:

```sh
just coverage
```

- Maintain **90% minimum coverage, striving for 100% where practical and beneficial.**

### Linting

- Run formatting and Clippy checks in CI:

```sh
just check
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

### Frontend Architecture Specifics

- **Yew Framework**: Component-based with functional components (`#[function_component]`)
- **State Management**: Yewdux for global state, initialized in `main.rs` with `YewduxRoot`
- **Internationalization**: `i18nrs` with JSON translation files in `translations/` directory
- **API Client**: Centralized `RustyGPTClient` in `api.rs` with typed request/response models
- **Routing**: Uses Yew Router with routes defined in `routes.rs`

### CSS & Components

- Use [TailwindCSS](https://github.com/tailwindlabs/tailwindcss) for styling.
- Use [DaisyUI](https://github.com/saadeghi/daisyui) for UI components.
- Reference latest [DaisyUI v5 examples](https://github.com/saadeghi/daisyui/tree/v5/docs/src/routes/components).
- Keep component structure clean and responsive.
- Prefer functional, accessible HTML and ARIA patterns.

### Frontend Build Process

- **Trunk**: WASM bundler with config in `Trunk.toml`
- **Build Commands**: `trunk build` (dev) or `trunk build --release` (production)
- **Static Assets**: Served from backend with fallback routing to frontend SPA

---

## 8. Project-Specific Patterns

### Backend Route Organization

Routes are organized into logical modules in `rustygpt-server/src/routes/`:
- `setup.rs`: Initial configuration endpoints
- `auth.rs`: OAuth authentication flow (GitHub/Apple)
- `protected.rs`: Authenticated endpoints with middleware
- `copilot.rs`: AI-specific endpoints
- `openapi.rs`: OpenAPI/Swagger documentation

### Shared Models

All API types are defined in `rustygpt-shared/src/models/` for type safety between frontend/backend:
- `conversation.rs`: Chat conversation models
- `message.rs`: Message and streaming chunk types
- `oauth.rs`: OAuth request/response types
- `errors.rs`: Standardized error responses
- `streaming.rs`: SSE message chunk definitions

### Development Tools

- **confuse**: Custom tool for concurrent command execution (`rustygpt-tools/confuse/`)
- **i18n-agent**: Translation management with audit/clean/template generation (`rustygpt-tools/i18n-agent/`)
- Both tools have comprehensive CLI interfaces and are used in development workflows

## 9. Copilot Enforcement Checklist

Copilot **must always**:

- Follow all idiomatic Rust practices.
- Avoid unnecessary clones or allocations.
- Maintain a minimum of 90% unit test coverage, striving for 100% where practical and beneficial.
- Write happy-path and error-path tests.
- Add or update all related doc comments.
- Format all commit messages per Conventional Commits.
- Write detailed PR descriptions using the defined template.
- Keep executable crate versions in sync using SemVer.
- Format and lint all code.
- Use Tailwind + DaisyUI for any frontend code.

---

## 10. Critical Rust Concepts to Reinforce

- Safety: use `unsafe` only with full justification and comments.
- Memory: prefer `&T`, avoid clones, use `Cow`, `Option`, `Result` idiomatically.
- Testing: fully test every logic path.
- Docs: every public item documented, all types reference-linked.
- Performance: iterate efficiently, async properly, structure data intentionally.

---

_End of RustyGPT Copilot Instructions._
