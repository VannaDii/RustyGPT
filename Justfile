# Default recipe (runs when you just run "just" with no arguments)
default: fmt lint check

# Tool lists - SINGLE SOURCE OF TRUTH
_core_tools := "sqlx-cli trunk cargo-llvm-cov"
_dev_tools := "cargo-audit wasm-opt wasm-bindgen-cli mdbook-mermaid"
_docs_tools := "mdbook mdbook-mermaid"
_all_tools := _core_tools + " " + _dev_tools

fetch:
    @echo "🔄 Fetching all dependencies…"
    cargo fetch --workspace

# Internal recipe to install tools with specified mode
_install_tools mode="locked" tools=_all_tools:
    #!/usr/bin/env bash
    set -euo pipefail
    export CARGO_NET_JOBS="$(nproc)"

    # Parse tools into array
    tools_array=({{tools}})

    # Install each tool
    for tool in "${tools_array[@]}"; do
        if [[ "{{mode}}" == "conditional" ]]; then
            # Check if tool exists before installing (for CI)
            case "$tool" in
                "sqlx-cli")
                    cmd_check="sqlx"
                    ;;
                "cargo-llvm-cov")
                    cmd_check="cargo-llvm-cov"
                    ;;
                *)
                    cmd_check="$tool"
                    ;;
            esac

            if ! command -v "$cmd_check" >/dev/null 2>&1; then
                echo "Installing $tool..."
                cargo install --locked "$tool"
            else
                echo "$tool already installed"
            fi
        else
            # Direct install mode
            install_flag="--{{mode}}"
            cargo install $install_flag --jobs $(nproc) "$tool"
        fi
    done

# Internal recipe for post-install setup
_post_install mode="full":
    # Add wasm-pack if not present (CI needs it)
    @if ! command -v wasm-pack >/dev/null 2>&1; then \
        echo "Installing wasm-pack..."; \
        cargo install wasm-pack; \
    else \
        echo "wasm-pack already installed"; \
    fi
    # Add WASM target
    rustup target add wasm32-unknown-unknown

    # Setup mdbook-mermaid if it was installed (full and offline mode only)
    @if [[ "{{mode}}" != "ci" ]] && command -v mdbook-mermaid >/dev/null 2>&1; then \
        echo "Setting up mdbook-mermaid..."; \
        mdbook-mermaid install .; \
        mv -f mermaid*.js scripts/ 2>/dev/null || true; \
    fi

    # Install git hooks (full and offline mode only)
    @if [[ "{{mode}}" != "ci" && -f "scripts/install-hooks.sh" ]]; then \
        echo "Installing Git hooks..."; \
        scripts/install-hooks.sh; \
    fi

# Recipe to install all development tools and dependencies
install:
    @echo "🔧 Installing all development tools..."
    just _install_tools locked "{{_all_tools}}"
    just _post_install full

# Recipe to install only essential tools for CI/CD environments
install-ci:
    @echo "🔧 Installing CI-specific tools..."
    just _install_tools conditional "{{_core_tools}}"
    just _post_install ci

# Recipe to install all tools in offline mode
install-offline:
    @echo "🔧 Installing all tools (offline mode)..."
    just _install_tools frozen "{{_all_tools}}"
    just _post_install offline

# Recipe to install only documentation tools for docs workflow
install-docs:
    @echo "🔧 Installing documentation tools..."
    just _install_tools locked "{{_docs_tools}}"
    # Setup mdbook-mermaid for docs
    @if command -v mdbook-mermaid >/dev/null 2>&1; then \
        echo "Setting up mdbook-mermaid..."; \
        mdbook-mermaid install .; \
        mv -f mermaid*.js scripts/ 2>/dev/null || true; \
    fi

# Recipe to start both frontend and backend watchers concurrently
dev:
    cargo run --manifest-path rustygpt-tools/confuse/Cargo.toml -- "server@./rustygpt-server:just watch-server" "client@./rustygpt-web:trunk watch"

# Standard checks for both frontend and backend
fmt:
    cargo fmt --all --check

fmt-fix:
    cargo fmt --all

lint:
    cargo clippy --workspace --all-targets --all-features -- -Dwarnings -Dclippy::all -Dclippy::pedantic -Dclippy::cargo -Dclippy::nursery

lint-fix:
    cargo clippy --workspace --all-targets --all-features --fix --allow-staged -- -Dwarnings -Dclippy::all -Dclippy::pedantic -Dclippy::cargo -Dclippy::nursery

check:
    cargo check --workspace --all-features

# Auto-fix what can be
fix:
    just fmt-fix
    just lint-fix

# Build everything
build:
    cargo build --workspace
    cd rustygpt-web && trunk build

# Build everything
build-release:
    cargo build --workspace --release
    cd rustygpt-web && trunk build --release

# Run CLI commands
cli *args:
    cargo run -p rustygpt-cli -- {{args}}

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
