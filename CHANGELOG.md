# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[unreleased]: https://github.com/kyow/terrain/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/kyow/terrain/releases/tag/v0.1.0
