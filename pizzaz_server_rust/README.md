# Pizzaz Server Rust

A Rust-based MCP (Model Context Protocol) server that exposes pizza-themed widgets for integration with ChatGPT and other MCP clients.

## Overview

This is a Rust port of the [pizzaz_server_node](../pizzaz_server_node) implementation, built using:
- **rmcp** (≥0.8.1) - Official Rust SDK for MCP
- **HTTP Streamable transport** - For real-time bidirectional communication
- **Axum** - Modern web framework for Rust
- **Test-Driven Development** - Comprehensive test coverage

## Project Structure

```
pizzaz_server_rust/
├── src/
│   ├── lib.rs              # Public API exports
│   ├── main.rs             # Binary entry point
│   ├── handler.rs          # MCP ServerHandler implementation
│   ├── widgets.rs          # Widget definitions and registry
│   ├── types.rs            # Shared types
│   └── test_helpers.rs     # Test utilities
├── tests/
│   ├── integration_test.rs # HTTP integration tests
│   └── common/             # Shared test setup
└── benches/                # Performance benchmarks
```

## Development Status

**Phase 1 Complete**: Project structure initialized

**Next Steps**: Implement Phase 2 (Widget Module) following TDD methodology

## Getting Started

### Prerequisites

- Rust 1.75+ (for `LazyLock` support)
- Cargo

### Build

```bash
cargo build
```

### Run Tests

```bash
cargo test
```

### Run Server

```bash
cargo run
```

Server will start on `http://localhost:8000` (configurable via `PORT` environment variable).

## Documentation

See [docs/pizzaz-server-rust-implementation-plan.md](../docs/pizzaz-server-rust-implementation-plan.md) for the complete TDD implementation plan.

## License

Same as parent repository.
