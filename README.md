# Blog Project

Rust workspace: REST + gRPC server, CLI client, WASM frontend, PostgreSQL.

## Requirements

- Rust 1.85+ — [rustup.rs](https://rustup.rs)
- Docker + Compose
- `wasm-pack` — `cargo install wasm-pack`

> `protoc` не требуется — `blog-grpc` crate использует `protoc-bin-vendored`.

## Build everything

```sh
# 1. Configure
cp env.example .env

# 2. Start PostgreSQL
docker compose up -d

# 3. Build server, client, CLI
cargo build --release --workspace --exclude blog-wasm

# 4. Build WASM
wasm-pack build blog-wasm --target web --release
```

Binaries → `target/release/`.  
WASM package → `blog-wasm/pkg/`.

## Run
Postgres должен работать до старта сервера, например через docker - `docker compose up -d`

```sh
cargo run -p blog-server
```

REST: `http://localhost:3000` · gRPC: `localhost:50051`

## Run WASM frontend

Сначала собери WASM-пакет (если ещё не собран):

```sh
wasm-pack build blog-wasm --target web --release
```

Затем подними HTTP-сервер из корня `blog-project/`

```sh
# Python (обычно уже есть в системе)
python3 -m http.server 8080 --directory blog-wasm


Открой `http://localhost:8080` — сервер должен быть запущен (`cargo run -p blog-server`).

## CLI

```sh
cargo run -p blog-cli -- register --username demo --email demo@example.com --password secret
cargo run -p blog-cli -- create --title "First post" --content "Hello"
cargo run -p blog-cli -- list --limit 10 --offset 0

# gRPC transport
cargo run -p blog-cli -- --grpc list --limit 10 --offset 0
```

## Tests

Docker должен быть запущен (tests использует `testcontainers`).

```sh
cargo test --tests -- --nocapture --test-threads=1
```
