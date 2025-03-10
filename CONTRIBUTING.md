# Contributing to RustyGPT

Thank you for considering contributing to RustyGPT! This document provides guidelines and instructions for contributing to the project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
  - [Development Environment](#development-environment)
  - [Project Structure](#project-structure)
- [Development Workflow](#development-workflow)
  - [Creating a Branch](#creating-a-branch)
  - [Making Changes](#making-changes)
  - [Testing](#testing)
  - [Submitting a Pull Request](#submitting-a-pull-request)
- [Coding Standards](#coding-standards)
  - [Rust Style Guidelines](#rust-style-guidelines)
  - [Documentation](#documentation)
  - [Commit Messages](#commit-messages)
- [Review Process](#review-process)
- [Community](#community)

## Code of Conduct

This project and everyone participating in it is governed by our [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## Getting Started

### Development Environment

1. **Prerequisites**:

   - Rust (latest stable version)
   - Cargo
   - Docker and Docker Compose (for local development)
   - PostgreSQL (if not using Docker)

2. **Setup**:

   ```bash
   # Clone the repository
   git clone https://github.com/VannaDii/RustyGPT.git
   cd RustyGPT

   # Copy the environment template
   cp .env.template .env
   # Edit .env with your configuration

   # Install dependencies and build
   cargo build
   ```

### Project Structure

The project follows a clean architecture with clear separation of concerns:

- **Backend**: Rust-based Axum server providing authentication and conversation APIs
  - **Handlers**: Handle HTTP requests and responses
  - **Routes**: Define API endpoints and group related routes
  - **Services**: Implement business logic
- **Frontend**: Rust-based web frontend using Yew
  - **Components**: Reusable UI elements
- **Shared**: Common models and utilities used by both frontend and backend
  - **Models**: Data structures for conversations, messages, users, and streaming functionality

## Development Workflow

### Creating a Branch

1. Create a branch from `main` with a descriptive name:

   ```bash
   git checkout -b feature/your-feature-name
   # or
   git checkout -b fix/issue-you-are-fixing
   ```

2. Keep your branch up to date with `main`:
   ```bash
   git fetch origin
   git rebase origin/main
   ```

### Making Changes

1. Make your changes in the appropriate files
2. Ensure your code follows the [Coding Standards](#coding-standards)
3. Add and commit your changes with a [descriptive commit message](#commit-messages)

### Testing

1. Write tests for your changes
2. Run the tests to ensure they pass:

   ```bash
   cargo test
   ```

3. Ensure your changes don't break existing functionality

### Submitting a Pull Request

1. Push your branch to your fork:

   ```bash
   git push origin feature/your-feature-name
   ```

2. Open a pull request against the `main` branch
3. Fill out the pull request template with all relevant information
4. Wait for review and address any feedback

## Coding Standards

### Rust Style Guidelines

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` to format your code
- Use `clippy` to catch common mistakes and improve your code
- Write idiomatic Rust code

### Documentation

- Document all public APIs using rustdoc comments
- Include examples where appropriate
- Keep documentation up to date with code changes
- Add comments for complex logic or non-obvious behavior

### Commit Messages

Follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

Types include:

- `feat`: A new feature
- `fix`: A bug fix
- `docs`: Documentation only changes
- `style`: Changes that do not affect the meaning of the code
- `refactor`: A code change that neither fixes a bug nor adds a feature
- `perf`: A code change that improves performance
- `test`: Adding missing tests or correcting existing tests
- `chore`: Changes to the build process or auxiliary tools

## Review Process

1. All pull requests require at least one review from a maintainer
2. CI checks must pass before merging
3. Reviewers may request changes or provide suggestions
4. Once approved, a maintainer will merge the pull request

## Community

- Join our community discussions
- Help answer questions from other contributors
- Share your ideas and feedback
- Respect everyone's time and contributions

Thank you for contributing to RustyGPT!
