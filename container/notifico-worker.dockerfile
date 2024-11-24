FROM rust:1.82-bookworm AS builder

WORKDIR /app

COPY .. /app

RUN cargo build --release --package notifico-worker

FROM gcr.io/distroless/cc-debian12

LABEL org.opencontainers.image.authors="alex@shishenko.com"

COPY --from=builder /app/target/release/notifico-worker /

VOLUME /var/lib/notifico

ENTRYPOINT ["/notifico-worker"]
