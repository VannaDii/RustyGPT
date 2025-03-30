# Default recipe (runs when you just run "just" with no arguments)
default: build

# Recipe to start both frontend and backend watchers concurrently
dev:
    (cd frontend && trunk watch) &
    (cd backend && cargo watch -x run) &
    wait

# Standard checks for both frontend and backend
check:
    just frontend-check
    just frontend-test
    just backend-check
    just backend-test

# Build everything
build:
    just frontend-build
    just backend-build

# Test everything
test:
    just frontend-test
    just backend-test

# Build the frontend
frontend-build:
    cd frontend && trunk build

# Build the frontend
frontend-test:
    cd frontend && trunk build

# Run standard backend checks
frontend-check:
    cd frontend && cargo fmt
    cd frontend && cargo check
    cd frontend && cargo clippy --all-features -- -D warnings
    cd backend && cargo fmt --all -- --check

# Build the backend
backend-build:
    cd backend && cargo build

# Run backend tests
backend-test:
    cd backend && cargo test

# Run standard backend checks
backend-check:
    cd backend && cargo fmt
    cd backend && cargo check
    cd backend && cargo clippy --all-features -- -D warnings
    cd backend && cargo fmt --all -- --check
