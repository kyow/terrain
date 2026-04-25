# terrain

`terrain` is a lightweight, configurable, local-first full-text search engine for your Markdown knowledge base, with built-in support for Japanese text.

It runs as a command-line MCP (Model Context Protocol) server, indexing a specified directory of `.md` files and exposing search and retrieval tools.

## Features

- **Full-Text Search:** Powered by the `traverze` search engine, built on `tantivy`.
- **Japanese Support:** Utilizes `lindera` with an IPADIC dictionary for accurate morphological analysis and tokenization of Japanese text.
- **MCP Server:** Exposes a simple, machine-readable tool interface over standard I/O.
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
terrain = { version = "0.1.0", default-features = false }
```

Disabling default features drops the `clap` and `notify` dependencies that the CLI uses, leaving a lean library suitable for embedding.

The library exposes the following public API:

- `Config` — Load and parse a TOML configuration file.
- `TerrainServer` — The MCP server handler, ready to be plugged into an `rmcp` transport. Constructed with `TerrainServer::new(engine, indexed_paths, &config)`.
- `IndexedPaths` — A cloneable, shared set of paths currently registered in the index. `read_file` consults this set to authorize reads, so the embedding app controls access by registering paths.
- `resolve_dir` / `build_engine` — Utility functions for directory resolution and search engine initialization.

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

3.  **Interact via MCP:**
    Once indexed, the server listens on `stdin` for MCP JSON requests and sends responses to `stdout`. You can use this interface with any MCP-compatible client or controller.

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
