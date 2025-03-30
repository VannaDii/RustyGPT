# Default recipe (runs when you just run "just" with no arguments)
default: build

# Recipe to start both frontend and backend watchers concurrently
dev:
    just confuse-build
    (cd frontend && trunk watch) &
    (cd backend && cargo watch -x run) &
    wait

# Standard checks for both frontend and backend
check:
    just i18n-check
    just i18n-test
    just confuse-check
    just confuse-test
    just frontend-check
    just frontend-test
    just backend-check
    just backend-test

# Build everything
build:
    just i18n-build
    just confuse-build
    just frontend-build
    just backend-build

# Test everything
test:
    just i18n-test
    just confuse-test
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

# Build the tools/i18n-agent
i18n-build:
    cd tools/i18n-agent && cargo build

# Run tools/i18n-agent tests
i18n-test:
    cd tools/i18n-agent && cargo test

# Run standard tools/i18n-agent checks
i18n-check:
    cd tools/i18n-agent && cargo fmt
    cd tools/i18n-agent && cargo check
    cd tools/i18n-agent && cargo clippy --all-features -- -D warnings
    cd tools/i18n-agent && cargo fmt --all -- --check

# Build the tools/confuse
confuse-build:
    cd tools/confuse && cargo build

# Run tools/confuse tests
confuse-test:
    cd tools/confuse && cargo test

# Run standard tools/confuse checks
confuse-check:
    cd tools/confuse && cargo fmt
    cd tools/confuse && cargo check
    cd tools/confuse && cargo clippy --all-features -- -D warnings
    cd tools/confuse && cargo fmt --all -- --check
