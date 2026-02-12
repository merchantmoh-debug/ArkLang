# Ark Compiler - Sovereign Sandbox
# Base: Python 3.12 (Slim) + Rust (for Core)
FROM python:3.12-slim

# Install system dependencies (Rust, Build Tools)
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Install Rust (stable)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Set Working Directory
WORKDIR /app

# Copy Project Files
COPY . /app

# Install Python Dependencies
RUN pip install --no-cache-dir -r requirements.txt || echo "No requirements.txt found, skipping."
# Install 'mcp' if not in requirements (since it's used in src/)
RUN pip install mcp[cli] pydantic requests

# Build Core (Rust) - Optional but recommended for full experience
RUN cd core && cargo build --release || echo "Core build skipped or failed."

# Set Environment Variables
ENV ARK_SANDBOX="true"
ENV ALLOW_DANGEROUS_LOCAL_EXECUTION="false"

# Default Command: Ark REPL (Python Meta-Layer)
CMD ["python", "meta/repl.py"]
