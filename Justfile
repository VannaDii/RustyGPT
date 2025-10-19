# Default recipe (runs when you just run "just" with no arguments)
default: check

fetch:
    @echo "🔄 Fetching all dependencies…"
    cargo fetch --workspace

# Recipe to install all the necessary tools and dependencies
install:
    export CARGO_NET_JOBS="$(nproc)"
    cargo install --locked --jobs $(nproc) \
        sqlx-cli \
        trunk \
        cargo-audit \
        wasm-opt \
        wasm-bindgen-cli \
        cargo-llvm-cov \
        mdbook-mermaid
    mdbook-mermaid install .
    mv -f mermaid*.js scripts/
    scripts/install-hooks.sh

# Recipe to install only essential tools for CI/CD environments
install-ci:
    @echo "🔧 Installing CI-specific tools..."
    # Install frontend tools if not present
    @if ! command -v trunk >/dev/null 2>&1; then \
        echo "Installing trunk..."; \
        cargo install trunk; \
    else \
        echo "trunk already installed"; \
    fi
    @if ! command -v wasm-pack >/dev/null 2>&1; then \
        echo "Installing wasm-pack..."; \
        cargo install wasm-pack; \
    else \
        echo "wasm-pack already installed"; \
    fi
    # Install backend tools if not present
    @if ! command -v sqlx >/dev/null 2>&1; then \
        echo "Installing sqlx-cli..."; \
        cargo install --locked sqlx-cli; \
    else \
        echo "sqlx-cli already installed"; \
    fi
    @if ! command -v cargo-llvm-cov >/dev/null 2>&1; then \
        echo "Installing cargo-llvm-cov..."; \
        cargo install --locked cargo-llvm-cov; \
    else \
        echo "cargo-llvm-cov already installed"; \
    fi
    # Add WASM target
    rustup target add wasm32-unknown-unknown

# Recipe to install all the necessary tools and dependencies OFFLINE
install-offline:
    export CARGO_NET_JOBS="$(nproc)"
    cargo install --frozen --jobs $(nproc) \
        sqlx-cli \
        trunk \
        cargo-audit \
        wasm-opt \
        wasm-bindgen-cli \
        cargo-llvm-cov \
        mdbook-mermaid
    mdbook-mermaid install .
    mv -f mermaid*.js scripts/
    scripts/install-hooks.sh

# Recipe to start both frontend and backend watchers concurrently
dev:
    cargo run --manifest-path rustygpt-tools/confuse/Cargo.toml -- "server@./rustygpt-server:just watch-server" "client@./rustygpt-web:trunk watch"

# Standard checks for both frontend and backend
check:
    cargo fmt -- --check
    cargo check --workspace
    cargo clippy --workspace --all-targets --all-features -- -Dclippy::all -Dclippy::pedantic -Dclippy::cargo -Dclippy::nursery -Aclippy::multiple_crate_versions

# Auto-fix what can be
fix:
    cargo fmt --all
    cargo clippy --workspace --all --all-features --fix --allow-staged

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
    cargo test --workspace --lib

# Run all tests and generate coverage report
coverage:
    cargo llvm-cov --workspace --lib --html --output-dir .coverage
    @echo "📊 Coverage report generated at file://$PWD/.coverage/html/index.html"

# Docs local server
docs-serve:
    mdbook serve --open

# Strict build
docs-build:
    mdbook build

# Generate machine-readable manifests
docs-index:
    cargo run -p rustygpt-doc-indexer --release

# Optional external link check (nightly/manual)
docs-links:
    cargo install --locked lychee
    lychee --verbose --no-progress docs

# Deploy to gh-pages
docs-deploy:
    just docs-build
    just docs-index
    git add book/ docs/llm/
    git commit -m "docs: publish" || true
    git push origin gh-pages

# Default docs target
docs:
    just docs-build
    just docs-index

api-docs:
    # Generate documentation for all crates
    cargo doc --no-deps --workspace

    # Copy the generated docs to the docs directory
    mkdir -p docs/api
    cp -r target/doc/* docs/api/

# Run the backend server
run-server:
    cd rustygpt-server && cargo run -- serve --port 8080

# Watch the backend server
watch-server:
    cd rustygpt-server && cargo watch -x 'run -- serve --port 8080'

# Helper recipes for when you tinker too hard
nuke-port-zombies:
    sudo lsof -t -i :8080 | xargs kill -9
