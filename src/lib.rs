use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{ServerHandler, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use traverze::{SearchOptions, SnippetOptions, TokenizerMode, Traverze};

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub instructions: Option<String>,
    pub search_description: Option<String>,
    pub read_file_description: Option<String>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("failed to parse config file: {}", path.display()))
    }
}

// ---------------------------------------------------------------------------
// MCP Tool parameters
// ---------------------------------------------------------------------------

#[derive(Deserialize, JsonSchema)]
struct SearchParams {
    /// The search query string. You can specify multiple keywords separated by spaces.
    /// Japanese text is fully supported and accurately tokenized using morphological analysis.
    query: String,
    /// The maximum number of search results to return (default: 20).
    /// Keep this reasonable to avoid overflowing your context window.
    limit: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
struct ReadFileParams {
    /// The absolute path of the Markdown file to read.
    /// You must use the exact path returned by the 'search' tool.
    path: String,
}

// ---------------------------------------------------------------------------
// TerrainServer
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct TerrainServer {
    engine: Traverze,
    base_dir: PathBuf,
    tool_router: ToolRouter<Self>,
    instructions: String,
}

#[tool_router]
impl TerrainServer {
    /// Search indexed Markdown files and return matching file paths, scores,
    /// and snippets.
    #[tool(
        name = "search",
        description = "Search local Markdown files (knowledge base) using full-text search. This engine is highly optimized for Japanese text using morphological analysis, so you can confidently pass natural Japanese keywords, phrases, or technical terms. Use this as your first action to find relevant context to answer the user's question. It returns a list of matching absolute file paths, relevance scores, and surrounding text snippets."
    )]
    async fn search(&self, Parameters(params): Parameters<SearchParams>) -> Result<String, String> {
        let options = SearchOptions {
            limit: params.limit.unwrap_or(20),
            snippet: Some(SnippetOptions::default()),
        };
        let hits = self
            .engine
            .search_with_options(&params.query, options)
            .map_err(|e| format!("search failed: {e}"))?;

        let results: Vec<serde_json::Value> = hits
            .iter()
            .map(|h| {
                serde_json::json!({
                    "path": h.path,
                    "score": h.score,
                    "snippet": h.snippet,
                })
            })
            .collect();

        serde_json::to_string_pretty(&results).map_err(|e| format!("serialization failed: {e}"))
    }

    /// Read the contents of a file within the indexed directory.
    #[tool(
        name = "read_file",
        description = "Read the full contents of a specific Markdown file. Use this when you find a promising snippet from the 'search' tool and need more detailed context, full sections, or complete code blocks. Provide the exact absolute file path retrieved from the search results."
    )]
    async fn read_file(
        &self,
        Parameters(params): Parameters<ReadFileParams>,
    ) -> Result<String, String> {
        let canonical = fs::canonicalize(&params.path)
            .map_err(|e| format!("file not found: {}: {e}", params.path))?;

        if !canonical.starts_with(&self.base_dir) {
            return Err("access denied: path is outside the indexed directory".to_string());
        }

        fs::read_to_string(&canonical).map_err(|e| format!("failed to read file: {e}"))
    }
}

#[tool_handler]
impl ServerHandler for TerrainServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(self.instructions.clone()),
            ..Default::default()
        }
    }
}

impl TerrainServer {
    pub fn new(engine: Traverze, base_dir: PathBuf, config: &Config) -> Self {
        let mut router = Self::tool_router();

        if let Some(desc) = &config.search_description {
            if let Some(route) = router.map.get_mut("search") {
                route.attr.description = Some(desc.clone().into());
            }
        }
        if let Some(desc) = &config.read_file_description {
            if let Some(route) = router.map.get_mut("read_file") {
                route.attr.description = Some(desc.clone().into());
            }
        }

        let instructions = config
            .instructions
            .clone()
            .unwrap_or_else(|| {
                "terrain MCP server – search and read indexed Markdown files".to_string()
            });

        Self {
            engine,
            base_dir,
            tool_router: router,
            instructions,
        }
    }
}

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

pub fn resolve_dir(dir: &Path) -> Result<PathBuf> {
    let canonical =
        fs::canonicalize(dir).with_context(|| format!("directory not found: {}", dir.display()))?;

    if !canonical.is_dir() {
        bail!("not a directory: {}", canonical.display());
    }

    Ok(canonical)
}

pub fn collect_markdown_files(base_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut stack = vec![base_dir.to_path_buf()];
    let mut files = Vec::new();

    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)
            .with_context(|| format!("failed to read directory: {}", dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;

            if file_type.is_dir() {
                stack.push(path);
                continue;
            }

            if file_type.is_file()
                && path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("md"))
                    .unwrap_or(false)
            {
                files.push(path);
            }
        }
    }

    Ok(files)
}

/// Create a `Traverze` engine and index the given Markdown files.
pub fn build_engine(index_dir: &Path, files: &[PathBuf]) -> Result<(Traverze, usize)> {
    let engine =
        Traverze::new_in_dir_for_indexing(index_dir, TokenizerMode::LinderaIpadic, true)
            .context("traverze index initialization failed")?;
    let indexed = engine
        .index_files(files)
        .context("failed to index markdown files")?;
    Ok((engine, indexed))
}
