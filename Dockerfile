# --- Build stage ---------------------------------------------------------
FROM rust:1.96-slim AS builder
WORKDIR /build

# Cache dependencies separately from source changes.
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs \
    && cargo build --release && rm -rf src

COPY src ./src
RUN touch src/main.rs src/lib.rs && cargo build --release

# --- Runtime stage --------------------------------------------------------
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Ownership verification for the official MCP Registry.
LABEL io.modelcontextprotocol.server.name="io.github.Liohtml/matomo-mcp"

COPY --from=builder /build/target/release/matomo-mcp /usr/local/bin/matomo-mcp

# MCP stdio server: keep stdin/stdout clean, logs go to stderr.
ENTRYPOINT ["matomo-mcp"]
