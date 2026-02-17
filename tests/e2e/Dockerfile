FROM rust:1.85-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY tesseras/ tesseras/
COPY tes/ tes/
COPY crates/ crates/

RUN cargo build --release -p tes

FROM alpine:3.21

COPY --from=builder /build/target/release/tes /usr/local/bin/tes
COPY tests/e2e/entrypoint.sh /usr/local/bin/entrypoint.sh

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
CMD ["tes"]
