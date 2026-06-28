# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `KnowledgeProvider` trait and its contract types (`SearchHit`, `SearchOptions`, `FileContent`), owned by terrain so the tool surface is decoupled from the underlying search engine
- `TraverzeProvider`, a bundled reference provider backed by `traverze`, behind the new `bundled-provider` feature (enabled by default through `cli`)
- `serve_io` helper to serve the MCP server over any `AsyncRead + AsyncWrite` transport (stdio, named pipe, Unix domain socket)
- Re-export of `rmcp` (`pub use rmcp`) so embedding apps can construct transports without depending on `rmcp` directly
- Streamable HTTP transport: serve MCP over HTTP at `/mcp` with `--transport http`, plus `--port` and `--host` flags to control the bind address (`--host` with no value binds `0.0.0.0` for access from other machines)
- `streamable_http_service` helper and the `streamable-http` feature to build an `rmcp` Streamable HTTP tower `Service` for mounting into your own HTTP server (e.g. `axum`/`hyper`)

### Changed

- `TerrainServer` tools now delegate to a `KnowledgeProvider` instead of calling `traverze` directly, and `read_file` access control moved into the provider (internal refactor, no behavior change for the CLI)
- `TerrainServer::new` signature changed to `(provider, &config)` (was `(engine, indexed_paths, &config)`)
- `traverze` is now an optional dependency behind the `bundled-provider` feature, so embedding apps can depend on terrain without pulling in `traverze`; `build_engine` is gated behind the same feature

## [0.2.1] - 2026-06-26

### Changed

- Raised the minimum supported Rust version (MSRV) to 1.88
- Updated dependencies to their latest versions: `rmcp` 0.17 → 1.7, `notify` 7 → 8, `toml` 0.8 → 1, `clap` 4.5 → 4.6, `tokio` 1.49 → 1.52, `serde_json` 1.0.149 → 1.0.150

## [0.2.0] - 2026-06-20

### Added

- File watcher to monitor directory changes and automatically update the index
- Debounced event handling with batch processing for efficient indexing
- Rename event normalization in file watcher
- `IndexedPaths` type to share the registered-path set between the library and embedding apps

### Changed

- `read_file` now authorizes access by checking whether the path is registered in the index, instead of requiring it to live under a fixed base directory
- `TerrainServer::new` simplified to `(engine, indexed_paths, &config)`
- File watcher and directory scanning moved out of the library into the CLI binary; embedding apps drive registration themselves
- `notify` dependency moved behind the `cli` feature

### Removed

- `collect_markdown_files` and `start_watcher` from the library's public API

## [0.1.0] - 2026-03-15

### Added

- Markdown indexing server with full-text search capabilities
- stdio MCP server with `search` and `read_file` tools
- Command-line argument parsing with `clap`
- Configurable MCP server instructions via TOML file
- Example configuration file for MCP server
- Library crate for core logic, enabling use as a dependency
- Optional `cli` feature flag for `clap` dependency
- Apache 2.0 and MIT dual license

[unreleased]: https://github.com/kyow/terrain/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/kyow/terrain/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/kyow/terrain/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/kyow/terrain/releases/tag/v0.1.0
