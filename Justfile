# Default recipe (runs when you just run "just" with no arguments)
default: check

install:
    @echo "Installing Rust components and targets..."
    rustup component add clippy rustfmt
    rustup target add wasm32-unknown-unknown

    @echo "Installing cargo tools..."
    cargo install sqlx-cli trunk cargo-audit wasm-opt wasm-bindgen-cli cargo-llvm-cov

    @echo "Installing git hooks..."
    ./scripts/install-hooks.sh

    @echo "Installation complete!"

# Recipe to start both frontend and backend watchers concurrently
dev:
    cargo run --manifest-path rustygpt-tools/confuse/Cargo.toml -- "server@./rustygpt-server:just watch-server" "client@./rustygpt-web:trunk watch"

# Standard checks for both frontend and backend
check:
    cargo fmt -- --check
    cargo check --workspace
    cargo clippy --workspace --all-features

# Auto-fix what can be
fix:
    cargo fmt --all
    cargo clippy --workspace --all-features --fix

# Build everything
build:
    cargo build --workspace
    cd rustygpt-web && trunk build

# Build everything
build-release:
    cargo build --workspace --release
    cd rustygpt-web && trunk build --release

# Test everything
test:
    cargo test --workspace

# Run all tests and generate coverage report
coverage:
    cargo llvm-cov --workspace --html --output-dir .coverage && open .coverage/html/index.html

docs:
    # Generate documentation for all crates
    cargo doc --no-deps --workspace

    # Copy the generated docs to the docs directory
    mkdir -p docs/api
    cp -r target/doc/* docs/api/

    # Ensure the index.html file exists
    touch docs/index.html

# Run the backend server
run-server:
    cd rustygpt-server && cargo run -- serve --port 8080

# Watch the backend server
watch-server:
    cd rustygpt-server && cargo watch -x 'run -- serve --port 8080'

# Helper recipes for when you tinker too hard
nuke-port-zombies:
    sudo lsof -t -i :8080 | xargs kill -9