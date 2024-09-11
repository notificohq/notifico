FROM rust as builder

COPY . .

RUN cargo build --release

FROM scratch

COPY --from=builder target/release/rule_engine ./

ENTRYPOINT /rule_engine
