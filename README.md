# Blog Project

Rust workspace with a blog server, shared client library, CLI, and a small WASM frontend. The server exposes both REST and gRPC APIs and stores data in PostgreSQL.

## Workspace layout

- `blog-server` - application server with REST and gRPC transports
- `blog-client` - reusable client crate with HTTP and gRPC implementations
- `blog-cli` - command line client built on top of `blog-client`
- `blog-wasm` - browser frontend compiled to WebAssembly
- `blog-grpc` - generated protobuf bindings shared by server and client

## Requirements

- Rust toolchain
- Docker with Compose support

## Configuration

Create a `.env` file in the repository root. You can start from `env.example`.

```env
DATABASE_URL=postgresql://blog:blog@127.0.0.1:5432/blog
JWT_SECRET=dev_super_secret_change_me_please
```

## Run locally

Start PostgreSQL:

```sh
docker compose up -d
```

Run the server:

```sh
cargo run -p blog-server
```

The default ports are:

- REST: `http://localhost:3000`
- gRPC: `http://localhost:50051`

## Use the CLI

Examples with the HTTP transport:

```sh
cargo run -p blog-cli -- register --username demo --email demo@example.com --password secret
cargo run -p blog-cli -- create --title "First post" --content "Hello from CLI"
cargo run -p blog-cli -- list --limit 10 --offset 0
```

Examples with the gRPC transport:

```sh
cargo run -p blog-cli -- --grpc register --username demo --email demo@example.com --password secret
cargo run -p blog-cli -- --grpc list --limit 10 --offset 0
```

The CLI stores the last received token in `.blog_token`.

## Run tests

Integration tests use Docker via `testcontainers`, so Docker must be running.

```sh
cargo test --tests -- --nocapture --test-threads=1
```

Package-specific examples:

```sh
cargo test -p blog-server --tests -- --nocapture --test-threads=1
cargo test -p blog-client --tests -- --nocapture --test-threads=1
```

## WASM frontend

The `blog-wasm` package contains a minimal browser client that talks to the REST API on `http://localhost:3000`.

## Useful commands

```sh
docker compose down
docker compose down -v
cargo fmt --all
cargo check --workspace
```
