FROM rust:1.82-bookworm as builder

COPY .. .

RUN cargo build --release --package notifico-worker

FROM gcr.io/distroless/cc-debian12

LABEL org.opencontainers.image.authors="alex@shishenko.com"

# Create templates directory
WORKDIR /var/notifico/templates

WORKDIR /

COPY --from=builder target/release/notifico-worker /

ENTRYPOINT ["/notifico-worker"]
