FROM rust:1.93 AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN cargo build --release --bin tesseras-daemon --bin tes --features "tesseras-daemon/bundled-sqlite tesseras-cli/bundled-sqlite"

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/tesseras-daemon /usr/local/bin/tesseras-daemon
COPY --from=builder /build/target/release/tes /usr/local/bin/tes
ENTRYPOINT ["tesseras-daemon"]
