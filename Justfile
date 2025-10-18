# Default recipe (runs when you just run "just" with no arguments)
default: check

fetch:
    @echo "🔄 Fetching all dependencies…"
    cargo fetch --manifest-path ./Cargo.toml
    cargo fetch --manifest-path ./rustygpt-cli/Cargo.toml
    cargo fetch --manifest-path ./rustygpt-server/Cargo.toml
    cargo fetch --manifest-path ./rustygpt-shared/Cargo.toml
    cargo fetch --manifest-path ./rustygpt-tools/confuse/Cargo.toml
    cargo fetch --manifest-path ./rustygpt-tools/i18n-agent/Cargo.toml
    cargo fetch --manifest-path ./rustygpt-web/Cargo.toml

# Recipe to install all the necessary tools and dependencies
install:
    export CARGO_NET_JOBS="$(nproc)"
    cargo install --locked --jobs $(nproc) \
        sqlx-cli \
        trunk \
        cargo-audit \
        wasm-opt \
        wasm-bindgen-cli \
        cargo-llvm-cov
    scripts/install-hooks.sh

# Recipe to install all the necessary tools and dependencies OFFLINE
install-offline:
    export CARGO_NET_JOBS="$(nproc)"
    cargo install --frozen --jobs $(nproc) \
        sqlx-cli \
        trunk \
        cargo-audit \
        wasm-opt \
        wasm-bindgen-cli \
        cargo-llvm-cov
    scripts/install-hooks.sh

# Recipe to start both frontend and backend watchers concurrently
dev:
    cargo run --manifest-path rustygpt-tools/confuse/Cargo.toml -- "server@./rustygpt-server:just watch-server" "client@./rustygpt-web:trunk watch"

# Standard checks for both frontend and backend
check:
    cargo fmt -- --check
    cargo check --workspace
    cargo clippy --workspace --all --all-targets --all-features -- -D warnings -D clippy::pedantic

# Auto-fix what can be
fix:
    cargo fmt --all
    cargo clippy --workspace --all --all-features --fix

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
    cargo test --workspace --lib -- --test-threads=1

# Run all tests and generate coverage report
coverage:
    cargo llvm-cov --workspace --lib --html --output-dir .coverage -- --test-threads=1
    @echo "📊 Coverage report generated at file://$PWD/.coverage/html/index.html"

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