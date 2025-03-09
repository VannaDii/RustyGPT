# ===========================
# Stage 1: Build Rust Backend
# ===========================
FROM nvidia/cuda:12.3.1-devel-ubuntu22.04 AS builder

# Install dependencies
RUN apt-get update && apt-get install -y \
  curl \
  build-essential \
  cmake \
  pkg-config \
  libssl-dev
RUN rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:$PATH"

# Install trunk
RUN cargo install trunk

# Install wasm32 target
RUN rustup target add wasm32-unknown-unknown

# Acquire sources
WORKDIR /source
COPY . /source/

# ===========================
# Stage 1: Build Backend
# ===========================

WORKDIR /source/backend

# Ensure all dependencies are downloaded
RUN cargo fetch

# Compile the Rust binary in release mode
RUN cargo build --release

# ===========================
# Stage 2: Build Frontend
# ===========================

WORKDIR /source

# Build frontend
RUN trunk build --release

# ===========================
# Stage 3: Minimal Runtime
# ===========================
FROM nvidia/cuda:12.3.1-runtime-ubuntu22.04 AS runtime

# Set working directory
WORKDIR /rusty_gpt

# Copy compiled backend binary
COPY --from=builder /source/target/release/backend /rusty_gpt/backend

# Copy frontend static files
COPY --from=builder /source/frontend/dist /rusty_gpt/frontend/

# Ensure we run as a non-root user for security
USER 1000

# Expose ports
EXPOSE 8080

# Start Nginx and backend
CMD ["/rusty_gpt/backend"]
