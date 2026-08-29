# Production Multi-Stage Dockerfile for Pure Rust Manga OCR Server
FROM rust:1.88-slim AS builder

WORKDIR /usr/src/app

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock* ./
COPY crates/ crates/

# Build release binaries
RUN cargo build --release -p comic-ocr-runtime

# Production Runtime Stage
FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy compiled Rust server binary from builder stage
COPY --from=builder /usr/src/app/target/release/comic-ocr-runtime /usr/local/bin/comic-ocr-runtime

EXPOSE 8000

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:8000/v1/runtime/health || exit 1

CMD ["comic-ocr-runtime"]
