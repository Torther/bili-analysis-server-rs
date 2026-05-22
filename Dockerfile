FROM rust:1-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

RUN cargo build --release && \
    cp target/release/bilibilianalysis-server /app/server

FROM alpine:latest

RUN apk add --no-cache ca-certificates

WORKDIR /app

COPY --from=builder /app/server .

EXPOSE 3000

CMD ["./server"]
