FROM rust:1.82-bookworm AS builder

COPY .. .

RUN cargo build --release --package notifico-worker

FROM gcr.io/distroless/cc-debian12

LABEL org.opencontainers.image.authors="alex@shishenko.com"

WORKDIR /

COPY --from=builder target/release/notifico-worker /

ENTRYPOINT ["/notifico-worker"]
