FROM rust:1.83-bookworm AS builder

WORKDIR /app

COPY .. /app

RUN cargo build --release --package notifico-app

FROM gcr.io/distroless/cc-debian12

LABEL org.opencontainers.image.authors="alex@shishenko.com"

COPY --from=builder /app/target/release/notifico-app /

VOLUME /var/lib/notifico

ENTRYPOINT ["/notifico-app"]
CMD ["run"]
