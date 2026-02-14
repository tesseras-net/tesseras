FROM rust:1.85 AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN cargo build --release --bin tesseras-daemon --features tesseras-daemon/bundled-sqlite

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/tesseras-daemon /usr/local/bin/tesseras-daemon
ENTRYPOINT ["tesseras-daemon"]
