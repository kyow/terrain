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
- Config `[server]` table to override the MCP `serverInfo` name and version, so embedding hosts can identify themselves by their own name/version
- `ToolCallObserver` trait, `ToolCallEvent`, and `TerrainServer::with_observer` to observe each tool call (input arguments and outcome) at the handler layer, so embedding hosts can display MCP traffic in their own UI; the hook fires regardless of transport (stdio, in-process stream, or Streamable HTTP)
- `list_files` MCP tool to list the absolute paths of all indexed files, sorted and paged via `limit` / `offset` (`limit: 0` returns just the total count), so MCP hosts can discover what documents exist without guessing search keywords; every returned path can be passed directly to `read_file`. The `KnowledgeProvider` trait gains a required `list_files` method with the new contract types `ListOptions` and `FileList` (breaking change for external provider implementations)

### Changed

- `TerrainServer` tools now delegate to a `KnowledgeProvider` instead of calling `traverze` directly, and `read_file` access control moved into the provider (internal refactor, no behavior change for the CLI)
- `TerrainServer::new` signature changed to `(provider, &config)` (was `(engine, indexed_paths, &config)`)
- `traverze` is now an optional dependency behind the `bundled-provider` feature, so embedding apps can depend on terrain without pulling in `traverze`; `build_engine` is gated behind the same feature
- Config tool descriptions moved from the flat `search_description` / `read_file_description` keys to a per-tool `[tools.<name>]` table (e.g. `[tools.search] description = "…"`); this generalizes tool customization as more tools are added
- Default `serverInfo` now reports terrain's own name and version instead of `rmcp`'s
- Updated `rmcp` 1.7 → 2.2 (no source changes required; the JSON wire format is unchanged). Inherited behavior change: rmcp 2.x silently ignores unparseable JSON-RPC lines on stdio, where 1.x replied with a `Parse error` response
- Relaxed the `tokio` version requirement from `1.47.1` to `1` to match the granularity of the other dependency specs (version requirements are lower bounds; the resolved version is unchanged)
- Updated `traverze` 0.2 → 0.3, following its renamed API (builder-based construction, `index_files`/`remove_files` → `index`/`remove`, `search_with_options` merged into `search`). Query preprocessing is explicitly pinned to `Plain`, keeping the 0.2 search semantics — space-separated keywords are OR-combined and ranked by BM25 — instead of 0.3's new `Auto` mode, which ANDs all tokens together and would zero-hit the speculative multi-keyword queries MCP hosts typically send. Inherited improvement: query parsing is now lenient, so queries containing Tantivy syntax characters no longer hard-fail
- The `search` tool description (built-in default, example config, and README) now documents the OR + BM25 semantics, so MCP hosts know that listing several candidate keywords or synonyms is a good strategy

### Fixed

- `tests/build_engine.rs` is now gated behind the `bundled-provider` feature, so `cargo check --tests --no-default-features` compiles again

## [0.2.2] - 2026-07-08

### Fixed

- `search` no longer returns stale results from directories served by previous runs: `build_engine` now resets the index directory before indexing, and the CLI namespaces the index directory per canonicalized `--dir` so concurrent servers do not clobber each other's index

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

[unreleased]: https://github.com/kyow/terrain/compare/v0.2.2...HEAD
[0.2.2]: https://github.com/kyow/terrain/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/kyow/terrain/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/kyow/terrain/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/kyow/terrain/releases/tag/v0.1.0
