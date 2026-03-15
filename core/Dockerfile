FROM rust:1.88-slim-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY core/Cargo.toml core/Cargo.toml
COPY zkp/Cargo.toml zkp/Cargo.toml
COPY core core
COPY zkp zkp

RUN cargo build -p core --release

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/core /usr/local/bin/core

EXPOSE 3000
ENV RUST_LOG=info
CMD ["core"]
