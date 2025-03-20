# Builder stage
FROM rust:1.74-slim-bullseye as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# Copy the entire workspace
COPY . .

# Build the server
RUN cargo build --release -p code-exec-server

# Runtime stage
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    python3 \
    python3-pip \
    python3-venv \
    nodejs \
    npm \
    golang \
    rustc \
    cargo \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -m -u 1000 -s /bin/bash codeexec

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /usr/src/app/target/release/code-exec-server /app/

# Set ownership
RUN chown -R codeexec:codeexec /app

# Switch to non-root user
USER codeexec

# Expose the port
EXPOSE 3000

# Run the server
ENTRYPOINT ["/app/code-exec-server"] 