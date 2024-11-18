FROM rust:1.82-bookworm AS builder

WORKDIR /project

COPY .. .

RUN --mount=type=cache,target=/project/target cargo build --release --package notifico-worker

FROM gcr.io/distroless/cc-debian12

LABEL org.opencontainers.image.authors="alex@shishenko.com"

WORKDIR /

COPY --from=builder /project/target/release/notifico-worker /

ENTRYPOINT ["/notifico-worker"]
