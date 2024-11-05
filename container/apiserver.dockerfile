FROM rust:1.82-bookworm as builder

COPY .. .

RUN cargo build --release --package notifico-apiserver

FROM gcr.io/distroless/cc-debian12

LABEL org.opencontainers.image.authors="alex@shishenko.com"

COPY --from=builder target/release/notifico-apiserver /

# Service API
EXPOSE 8000
ENV NOTIFICO_SERVICE_API_BIND=[::]:8000
# Client API
EXPOSE 9000
ENV NOTIFICO_CLIENT_API_BIND=[::]:9000

ENTRYPOINT ["/notifico-apiserver"]
