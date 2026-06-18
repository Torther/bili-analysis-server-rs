FROM rust:1-alpine AS builder

RUN apk add --no-cache musl-dev

ARG CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
ARG TARGETARCH

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

RUN --mount=type=cache,target=/usr/local/cargo/registry,id=cargo-registry-${TARGETARCH} \
    --mount=type=cache,target=/app/target,id=cargo-target-${TARGETARCH} \
    CARGO_REGISTRIES_CRATES_IO_PROTOCOL=${CARGO_REGISTRIES_CRATES_IO_PROTOCOL} \
    cargo build --release && \
    cp target/release/bili-analysis-server-rs /app/server

FROM alpine:latest

RUN apk add --no-cache ca-certificates

WORKDIR /app

COPY --from=builder /app/server .

EXPOSE 3000

CMD ["./server"]
