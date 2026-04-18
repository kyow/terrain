# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- File watcher to monitor directory changes and automatically update the index
- Debounced event handling with batch processing for efficient indexing
- Rename event normalization in file watcher
- Logging of indexed document count when indexing is complete

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
