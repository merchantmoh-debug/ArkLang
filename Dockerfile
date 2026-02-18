# Stage 1: Rust builder
FROM rust:1.93-slim AS rust-builder
WORKDIR /build
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY core/ ./core/

# Build and test from workspace root (uses Cargo.toml workspace)
RUN cargo test --release -p ark-0-zheng
RUN cargo build --release --bin ark_loader

# Stage 2: Python runtime
FROM python:3.11-slim AS runtime
WORKDIR /app

# Install system deps
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy requirements and install dependencies
COPY requirements.txt ./
RUN pip install --no-cache-dir -r requirements.txt

# Install mcp if needed
RUN pip install --no-cache-dir mcp[cli]

# Copy Python code
COPY meta/ ./meta/
COPY lib/ ./lib/
COPY apps/ ./apps/
COPY src/ ./src/

# Copy Rust binary from workspace target dir
COPY --from=rust-builder /build/target/release/ark_loader /usr/local/bin/ark-core

# Security: non-root user
RUN useradd -m -s /bin/bash ark
USER ark

# Health check
HEALTHCHECK --interval=30s --timeout=10s --retries=3 \
    CMD python -c "print('ok')" || exit 1

# Default command
CMD ["python", "meta/ark.py", "repl"]
EXPOSE 8080
