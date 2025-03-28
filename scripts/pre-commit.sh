#!/bin/sh

# Pre-push hook to run the same commands as CI
# This ensures that your code will pass CI checks before being pushed to GitHub

echo "Running pre-commit checks..."

# Save current directory
CURRENT_DIR=$(pwd)
export SQLX_OFFLINE=true

# Check if we're in the project root, if not navigate to it
if [ ! -f "Cargo.toml" ]; then
    # Try to find the project root
    PROJECT_ROOT=$(git rev-parse --show-toplevel)
    if [ -n "$PROJECT_ROOT" ]; then
        cd "$PROJECT_ROOT"
    else
        echo "Error: Could not find project root. Make sure you're in a git repository."
        exit 1
    fi
fi

# Function to run a command and exit if it fails
run_check() {
    echo "Running: $1"
    eval "$1"
    if [ $? -ne 0 ]; then
        echo "Error: $1 failed. Push aborted."
        cd "$CURRENT_DIR"
        exit 1
    fi
}

# Check if .env file exists, if not create it from template
if [ ! -f ".env" ] && [ -f ".env.template" ]; then
    echo "Creating .env file from template..."
    cp .env.template .env
    echo "GITHUB_CLIENT_ID=test_client_id" >>.env
    echo "GITHUB_CLIENT_SECRET=test_client_secret" >>.env
    echo "APPLE_CLIENT_ID=test_client_id" >>.env
    echo "DATABASE_URL=postgres://postgres:postgres@localhost/rusty_gpt" >>.env
fi

# Check formatting
run_check "cargo fmt --all -- --check"

# Run clippy
run_check "cargo clippy --all-features -- -D warnings"

# Build the project
run_check "cargo build"

# Run tests
run_check "cargo test"

# Check if trunk is installed
if command -v trunk >/dev/null 2>&1; then
    # Build frontend
    echo "Building frontend with trunk..."
    cd frontend
    run_check "trunk build"
    cd ..
else
    echo "Warning: trunk is not installed. Skipping frontend build."
    echo "To install trunk: cargo install trunk"
fi

# Check if cargo-audit is installed and run security audit
if command -v cargo-audit >/dev/null 2>&1; then
    echo "Running security audit..."
    cargo audit || {
        echo "Warning: Security audit found issues."
        echo "Review the output above for details."
        echo "You can still push, but consider fixing these issues."
    }
else
    echo "Warning: cargo-audit is not installed. Skipping security audit."
    echo "To install cargo-audit: cargo install cargo-audit"
fi

echo "All checks passed!"
# Return to the original directory
cd "$CURRENT_DIR"
exit 0
