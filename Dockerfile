FROM rust:1.87-bookworm AS builder

WORKDIR /build

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY notifico-core/Cargo.toml notifico-core/Cargo.toml
COPY notifico-db/Cargo.toml notifico-db/Cargo.toml
COPY notifico-template/Cargo.toml notifico-template/Cargo.toml
COPY notifico-queue/Cargo.toml notifico-queue/Cargo.toml
COPY notifico-server/Cargo.toml notifico-server/Cargo.toml

# Create stub files so cargo can resolve the workspace
RUN mkdir -p notifico-core/src && echo "pub fn _stub() {}" > notifico-core/src/lib.rs && \
    mkdir -p notifico-db/src && echo "pub fn _stub() {}" > notifico-db/src/lib.rs && \
    mkdir -p notifico-template/src && echo "pub fn _stub() {}" > notifico-template/src/lib.rs && \
    mkdir -p notifico-queue/src && echo "pub fn _stub() {}" > notifico-queue/src/lib.rs && \
    mkdir -p notifico-server/src && echo "fn main() {}" > notifico-server/src/main.rs

# Build dependencies (cached layer)
RUN cargo build --release --bin notifico 2>/dev/null || true

# Copy actual source
COPY . .

# Touch files to invalidate stubs
RUN find . -name "*.rs" -exec touch {} +

# Build frontend
RUN curl -fsSL https://bun.sh/install | bash
ENV PATH="/root/.bun/bin:$PATH"
RUN cd notifico-frontend && bun install --frozen-lockfile && bun run build

# Build the real binary
RUN cargo build --release --bin notifico

# ── Runtime ──────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/notifico /usr/local/bin/notifico

RUN useradd -r -s /bin/false notifico
USER notifico

EXPOSE 8000

ENV RUST_LOG=info
ENV NOTIFICO_SERVER_HOST=0.0.0.0
ENV NOTIFICO_SERVER_PORT=8000
# OpenTelemetry OTLP endpoint (e.g. http://jaeger:4317). Leave empty to disable.
ENV NOTIFICO_OTEL_ENDPOINT=""

ENTRYPOINT ["notifico"]
