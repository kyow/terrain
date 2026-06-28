# terrain

English | [日本語](README.ja.md)

`terrain` is a lightweight, configurable, local-first full-text search engine for your Markdown knowledge base, with built-in support for Japanese text.

It runs as a command-line MCP (Model Context Protocol) server, indexing a specified directory of `.md` files and exposing search and retrieval tools.

## Features

- **Full-Text Search:** Powered by the `traverze` search engine, built on `tantivy`.
- **Japanese Support:** Utilizes `lindera` with an IPADIC dictionary for accurate morphological analysis and tokenization of Japanese text.
- **MCP Server:** Exposes a simple, machine-readable tool interface over stdio or Streamable HTTP.
- **Auto-Indexing:** Watches the target directory and updates the index automatically when files are added, modified, removed, or renamed — no restart required. Events are debounced and processed in batches for efficiency.
- **Secure:** `read_file` only serves paths that have been registered in the index, so registration is the permission grant.
- **Configurable:** Customize tool descriptions via a TOML configuration file to tailor AI model behavior.
- **Cross-Platform:** Built with Rust, runs on Windows, macOS, and Linux.

## Installation

You need to have [Rust](https://www.rust-lang.org/tools/install) installed.

### As a CLI tool

```bash
cargo install terrain
```

### As a library

Add the following to your `Cargo.toml`:

```toml
[dependencies]
terrain = { version = "0.2", default-features = false }
```

Disabling default features drops the CLI dependencies (`clap`, `notify`, `axum`) and the bundled `traverze` provider, leaving a lean library where you bring your own search engine. Opt back in per feature as needed:

- `bundled-provider` — the reference `TraverzeProvider` plus `resolve_dir` / `build_engine`.
- `streamable-http` — the `streamable_http_service` helper (Streamable HTTP transport).

The library exposes the following public API:

- `Config` — Load and parse a TOML configuration file.
- `KnowledgeProvider` — The trait backing the `search` / `read_file` tools, with the `SearchHit`, `SearchOptions`, and `FileContent` types. Implement it to plug in your own search engine and access-control policy.
- `TerrainServer` — The MCP server handler, ready to be plugged into an `rmcp` transport. Constructed with `TerrainServer::new(provider, &config)`, where `provider` is an `Arc<dyn KnowledgeProvider>`.
- `IndexedPaths` — A cloneable, shared set of paths currently registered in the index. The bundled provider consults this set to authorize `read_file` reads, so the embedding app controls access by registering paths.
- `serve_io` — Serve the server over any `rmcp` I/O transport (stdio, a pipe, or a socket).
- `streamable_http_service` *(feature `streamable-http`)* — Build an `rmcp` Streamable HTTP tower `Service` to mount into your own HTTP server (e.g. `axum`/`hyper`).
- `TraverzeProvider` / `resolve_dir` / `build_engine` *(feature `bundled-provider`)* — The reference provider backed by `traverze`, plus directory-resolution and engine-initialization helpers.

The library does not scan directories or watch the filesystem on its own — embedding apps decide which files to register and when to re-index. See [src/main.rs](src/main.rs) for a reference integration that walks a directory of `.md` files and keeps the index in sync via [`notify`](https://crates.io/crates/notify).

## MCP Client Setup

To use `terrain` with an MCP-compatible client such as Claude Desktop, add the following to your client's configuration file (e.g., `claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "terrain": {
      "command": "terrain",
      "args": ["--dir", "/path/to/your/notes"]
    }
  }
}
```

If you built from source without `cargo install`, use the full path to the executable instead (e.g., `"/path/to/terrain"`).

For clients that connect over the network, start the server with the HTTP transport (see [Transports](#transports)) and point the client at the endpoint URL instead of a command:

```json
{
  "mcpServers": {
    "terrain": {
      "url": "http://127.0.0.1:8000/mcp"
    }
  }
}
```

## Usage

1.  **Start the server:**
    Run the program from your terminal, pointing it to the directory containing your Markdown files.

    ```bash
    terrain --dir /path/to/your/notes
    ```

2.  **Indexing:**
    The server will first index all Markdown files in the specified directory. You will see a message indicating how many files have been indexed.

    ```
    indexed 1234 markdown files from /path/to/your/notes
    ```

3.  **Watching for changes:**
    After the initial index, the server watches the directory and keeps the index in sync automatically — there is no need to restart when you add, edit, remove, or rename Markdown files. File-system events are debounced and processed in batches, and you will see log lines as the index is updated.

    ```
    watching /path/to/your/notes for changes
    watcher: re-indexed 1 file(s)
    watcher: removed 1 file(s) from index
    ```

4.  **Interact via MCP:**
    Once indexed, the server listens on `stdin` for MCP JSON requests and sends responses to `stdout`. You can use this interface with any MCP-compatible client or controller.

## Transports

`terrain` speaks MCP over two transports, selected with `--transport`:

- `stdio` (default) — communicates over standard input/output. Used by most MCP clients (e.g. Claude Desktop), which launch the server as a subprocess.
- `http` — serves the [Streamable HTTP transport](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports#streamable-http) at `/mcp`, so clients connect over the network.

### Streamable HTTP

```bash
# Listen on 127.0.0.1:8000 (this machine only)
terrain --dir /path/to/your/notes --transport http

# Change the port
terrain --dir /path/to/your/notes --transport http --port 9000

# Make it reachable from other machines (binds 0.0.0.0)
terrain --dir /path/to/your/notes --transport http --host
```

The endpoint is `http://<host>:<port>/mcp`.

| Flag | Default | Description |
|------|---------|-------------|
| `--transport <stdio\|http>` | `stdio` | Transport to serve over. |
| `--port <PORT>` | `8000` | Port for the `http` transport. |
| `--host [ADDR]` | `127.0.0.1` | Bind address for `http`. Omit for local only; pass the flag with no value to bind `0.0.0.0` (reachable from other machines); pass an address to bind a specific interface. |

> **Security:** `terrain` has no built-in authentication — reachability is governed entirely by the bind address. The default (`127.0.0.1`) keeps the server private to your machine. Only use `--host` on a trusted network, and put `terrain` behind a reverse proxy, SSH tunnel, or VPN if you need authenticated or public access.

## Configuration

In MCP, tool descriptions directly influence how the AI model decides when and how to use each tool. You can customize these descriptions to better suit your use case by providing a TOML configuration file.

```bash
terrain --dir /path/to/your/notes --config terrain.config.toml
```

See [terrain.config.example.toml](terrain.config.example.toml) for all available options.

## MCP Tools

The server provides the following tools:

### `search`

Search indexed Markdown files and return matching file paths, scores, and snippets.

- **Description:** This tool is highly optimized for Japanese text. Use it to find relevant context to answer a user's question. It returns a list of matching absolute file paths, relevance scores, and surrounding text snippets.
- **Parameters:**
    - `query` (string, required): The search query. You can specify multiple keywords separated by spaces.
    - `limit` (integer, optional): The maximum number of search results to return (default: 20).
- **Example Return Value:**
    ```json
    [
      {
        "path": "/path/to/your/notes/example.md",
        "score": 18.72,
        "snippet": "This is a snippet of text surrounding the matched keyword."
      }
    ]
    ```

### `read_file`

Read the full contents of a specific Markdown file.

- **Description:** Use this when you find a promising snippet from the `search` tool and need more detailed context. Provide the exact absolute file path retrieved from the search results.
- **Parameters:**
    - `path` (string, required): The absolute path of the Markdown file to read. You must use the exact path returned by the `search` tool.
- **Example Return Value:**
    The full, raw content of the specified Markdown file.

## License

Licensed under either of

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

at your option.
