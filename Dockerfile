# Stage 1: Rust builder
FROM rust:1.93-slim AS rust-builder
WORKDIR /build
COPY core/ ./core/
COPY Cargo.toml Cargo.lock ./
RUN cd core && cargo build --release

# Stage 2: Python runtime
FROM python:3.11-slim AS runtime
WORKDIR /app

# Install system deps (ca-certificates for HTTPS, curl for healthchecks if needed)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy requirements and install dependencies (Layer caching optimization)
COPY requirements.txt ./
RUN pip install --no-cache-dir -r requirements.txt

# Install 'mcp' if not in requirements (since it's used in src/)
RUN pip install --no-cache-dir mcp[cli]

# Copy Python code
COPY meta/ ./meta/
COPY lib/ ./lib/
COPY apps/ ./apps/
COPY src/ ./src/

# Copy Rust binary
# Note: Source binary is 'ark_loader' from core/src/bin/ark_loader.rs
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
