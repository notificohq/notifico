FROM rust:1.82-bookworm AS builder

WORKDIR /app

COPY .. /app

RUN cargo build --release --package notifico-ingest

FROM gcr.io/distroless/cc-debian12

LABEL org.opencontainers.image.authors="alex@shishenko.com"

COPY --from=builder /app/target/release/notifico-ingest /

# Client API
EXPOSE 8000
ENV NOTIFICO_HTTP_INGEST_BIND=[::]:8000

VOLUME /var/lib/notifico

ENTRYPOINT ["/notifico-ingest"]
