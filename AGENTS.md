# AGENTS.md

## What this is

Single-binary Rust web server that resolves Bilibili video/live URLs to raw CDN stream URLs and returns a 302 redirect. Five source files, flat module structure, no workspace.

## Key commands

```sh
cargo check              # fast compile check
cargo build --release    # production binary
cargo clippy             # lint (not configured in CI, but useful)
```

No CI. No linting config (`rustfmt.toml`, `clippy.toml`, `rust-toolchain.toml` — all absent).

Docker build is the real "release" process — multi-stage Alpine + musl static linking.

## Architecture

```
main.rs         → Axum router, two routes: GET /?url=... (302), GET /health; graceful shutdown
bilibili.rs     → URL resolution via Bilibili APIs (pub resolve_raw_url), shared reqwest::Client
cache.rs        → moka in-memory cache, 10-min TTL, global static
dedup.rs        → concurrent request deduplication (same URL, one API call)
mirror_cdn.rs   → CDN hostname rewriting + round-robin China mirror selection
```

Dependency flow: `main → bilibili → mirror_cdn`, `main → cache`, `main → dedup → bilibili + cache`.

## Config

- `PORT` env var — defaults to 3000
- `RUST_LOG` env var — defaults to `info` (tracing-subscriber env-filter)

## Gotchas

- **reqwest uses rustls-tls, not OpenSSL** — no system TLS dependency. Important for Alpine/musl builds.
- **Shared reqwest::Client** in `bilibili.rs` (`static CLIENT`) with 10s timeout and connection pooling. All HTTP calls go through it.
- **Cache key format:** `vid:<bvid>:<page>` for videos, `live:<room_id>` for live streams. Same video always maps to the same key regardless of input format (URL, bare BV, bare AV).
- **Cache lookup** happens inside `dedup_resolve` — cached results bypass dedup.
- **av2bv conversion** uses Bilibili's proprietary XOR + base58 algorithm — returns `Result`, propagates errors on invalid input.
- **SSRF protection:** `validate_bilibili_url()` in `bilibili.rs` checks hostname is `bilibili.com` or `*.bilibili.com` before making any outbound request.
- **API error checking:** `check_api_response()` inspects the `code` field from Bilibili API JSON responses (not just HTTP status).
- **Request deduplication:** `dedup.rs` uses a `DashMap` of `watch` channels. Concurrent requests for the same URL share one API call — the first request fetches, others wait (60s timeout). This prevents cache stampede (thundering herd).
- **Graceful shutdown:** handles SIGTERM (Unix) and Ctrl+C via `tokio::signal`. Docker stop triggers clean shutdown.
- **Live stream selection:** prefers `stream[1] > format[1] > codec[0]`, falls back to index 0.
- **mirror_cdn.rs** has nuanced hostname classification logic (proxy-tf passthrough, overseas replacement, MCDN IP detection) — read the full match arms before modifying.
- **No `.env` file committed** (gitignored). Only `PORT` and `RUST_LOG` matter.
- **Binary name** in Dockerfile is `bilibilianalysis-server` (from Cargo.toml `name` field).
