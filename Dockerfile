FROM rust:1-alpine AS builder

RUN apk add --no-cache musl-dev
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release && \
    cp target/release/bili-analysis-server-rs /app/server

FROM alpine:latest

RUN apk add --no-cache ca-certificates

WORKDIR /app

COPY --from=builder /app/server .

EXPOSE 3000

CMD ["./server"]
