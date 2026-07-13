use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use rmcp::handler::server::tool::{ToolCallContext, ToolRouter};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Implementation, JsonObject, ServerCapabilities,
    ServerInfo,
};
use rmcp::service::RequestContext;
use rmcp::transport::IntoTransport;
#[cfg(feature = "streamable-http")]
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
};
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt, tool, tool_handler, tool_router};

// Re-exported so callers can construct/customize the HTTP server config without
// naming an rmcp path of their own (keeps the rmcp version aligned with terrain).
#[cfg(feature = "streamable-http")]
pub use rmcp::transport::streamable_http_server::StreamableHttpServerConfig;

// Re-export rmcp so that consumers (e.g. an embedding host) can build transports
// and other rmcp values without taking their own dependency on rmcp, keeping the
// version aligned with the one terrain serves with.
pub use rmcp;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[cfg(feature = "bundled-provider")]
use traverze::{
    SearchOptions as TraverzeSearchOptions, SnippetFormat, SnippetOptions, TokenizerMode, Traverze,
};

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub instructions: Option<String>,
    /// Overrides for the MCP `serverInfo` (name / version) reported at
    /// initialization. Lets an embedding host identify itself by its own name
    /// instead of terrain's defaults.
    pub server: ServerInfoConfig,
    /// Per-tool customization, keyed by tool name (e.g. `search`, `read_file`).
    /// Unknown tool names are ignored.
    pub tools: HashMap<String, ToolConfig>,
}

/// Overrides for the values reported in the MCP `serverInfo` object.
#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ServerInfoConfig {
    /// Server name; defaults to terrain's crate name when unset.
    pub name: Option<String>,
    /// Server version; defaults to terrain's crate version when unset.
    pub version: Option<String>,
}

/// Per-tool configuration overrides.
#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ToolConfig {
    /// Overrides the tool's description advertised to the MCP client.
    pub description: Option<String>,
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
// IndexedPaths
// ---------------------------------------------------------------------------

/// Shared set of paths currently registered in the search index.
///
/// `read_file` consults this set to decide whether a path may be read,
/// replacing the previous "must live under base_dir" check. Registration
/// into the index is therefore the permission grant.
///
/// Cheap to clone (internally `Arc`).
#[derive(Clone, Default)]
pub struct IndexedPaths(Arc<RwLock<HashSet<PathBuf>>>);

impl IndexedPaths {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, path: PathBuf) {
        self.0
            .write()
            .expect("indexed-paths lock poisoned")
            .insert(path);
    }

    pub fn extend<I: IntoIterator<Item = PathBuf>>(&self, paths: I) {
        self.0
            .write()
            .expect("indexed-paths lock poisoned")
            .extend(paths);
    }

    pub fn remove(&self, path: &Path) -> bool {
        self.0
            .write()
            .expect("indexed-paths lock poisoned")
            .remove(path)
    }

    pub fn contains(&self, path: &Path) -> bool {
        self.0
            .read()
            .expect("indexed-paths lock poisoned")
            .contains(path)
    }

    pub fn len(&self) -> usize {
        self.0.read().expect("indexed-paths lock poisoned").len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.read().expect("indexed-paths lock poisoned").is_empty()
    }
}

// ---------------------------------------------------------------------------
// Knowledge provider contract
// ---------------------------------------------------------------------------
//
// terrain owns these types so that the search / read_file tool surface is
// defined independently of any particular search engine. A `KnowledgeProvider`
// supplies the behaviour; `Traverze` (or any other backend) stays hidden behind
// the implementation. These types are part of terrain's public contract and
// therefore carry a stability commitment.

/// A single search result returned by a [`KnowledgeProvider`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    /// Absolute path of the matching file.
    pub path: String,
    /// Relevance score; higher is more relevant.
    pub score: f32,
    /// Surrounding text snippet, if the provider produced one.
    pub snippet: Option<String>,
}

/// Options controlling a [`KnowledgeProvider::search`] call.
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Maximum number of results to return.
    pub limit: usize,
    /// Maximum snippet length in characters. `None` disables snippets.
    pub snippet_max_chars: Option<usize>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: 20,
            snippet_max_chars: Some(150),
        }
    }
}

/// The full contents of a file returned by [`KnowledgeProvider::read_file`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContent {
    /// Absolute path of the file that was read.
    pub path: String,
    /// The file's contents.
    pub content: String,
}

/// A source of searchable knowledge backing terrain's MCP tools.
///
/// Implementations own the search engine and the access-control policy for
/// `read_file`; the engine type never appears in this interface, which is what
/// makes the backend transparent to MCP clients.
#[async_trait]
pub trait KnowledgeProvider: Send + Sync {
    /// Search the knowledge base and return matching hits.
    async fn search(&self, query: &str, opts: &SearchOptions) -> Result<Vec<SearchHit>>;

    /// Read the full contents of `path`.
    ///
    /// Implementations are responsible for enforcing which paths may be read.
    async fn read_file(&self, path: &Path) -> Result<FileContent>;
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
// Tool-call observation
// ---------------------------------------------------------------------------

/// A completed tool call, observed at the handler layer.
///
/// One event is emitted per `tools/call` round-trip and carries both the
/// input arguments and the outcome, so a subscriber can render each call as
/// a single self-contained entry. Because the hook sits in the handler
/// rather than the transport, the same events fire however the server is
/// served — stdio, an in-process stream, or Streamable HTTP.
#[non_exhaustive]
pub struct ToolCallEvent<'a> {
    /// Name of the tool that was called (e.g. `search`).
    pub tool_name: &'a str,
    /// Arguments the client passed, if any.
    pub arguments: Option<&'a JsonObject>,
    /// The call's outcome. `Ok` is the result returned to the client; tool
    /// execution failures are reported there via [`CallToolResult::is_error`],
    /// not as `Err`. `Err` is a protocol-level error (unknown tool name,
    /// invalid parameters).
    pub outcome: Result<&'a CallToolResult, &'a ErrorData>,
    /// Wall-clock time the call took.
    pub duration: Duration,
}

/// Receives a [`ToolCallEvent`] for every tool call handled by a
/// [`TerrainServer`], letting an embedding host display MCP traffic in its
/// own UI.
///
/// Called synchronously on the request path: implementations must not block.
/// Hand the event off (e.g. into a channel or a UI event emitter) and return.
pub trait ToolCallObserver: Send + Sync {
    fn on_tool_call(&self, event: &ToolCallEvent<'_>);
}

// ---------------------------------------------------------------------------
// TerrainServer
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct TerrainServer {
    provider: Arc<dyn KnowledgeProvider>,
    tool_router: ToolRouter<Self>,
    instructions: String,
    server_info: Implementation,
    observer: Option<Arc<dyn ToolCallObserver>>,
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
            limit: params.limit.unwrap_or_else(|| SearchOptions::default().limit),
            ..SearchOptions::default()
        };
        let hits = self
            .provider
            .search(&params.query, &options)
            .await
            .map_err(|e| format!("search failed: {e:#}"))?;

        serde_json::to_string_pretty(&hits).map_err(|e| format!("serialization failed: {e}"))
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
        let content = self
            .provider
            .read_file(Path::new(&params.path))
            .await
            .map_err(|e| format!("{e:#}"))?;

        Ok(content.content)
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for TerrainServer {
    // Hand-written so the observer sees every tool call; #[tool_handler]
    // skips generating `call_tool` when the impl block already has one.
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let Some(observer) = &self.observer else {
            let tcc = ToolCallContext::new(self, request, context);
            return self.tool_router.call(tcc).await;
        };

        let tool_name = request.name.clone();
        let arguments = request.arguments.clone();
        let started = Instant::now();
        let tcc = ToolCallContext::new(self, request, context);
        let result = self.tool_router.call(tcc).await;
        observer.on_tool_call(&ToolCallEvent {
            tool_name: &tool_name,
            arguments: arguments.as_ref(),
            outcome: result.as_ref(),
            duration: started.elapsed(),
        });
        result
    }

    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.instructions = Some(self.instructions.clone());
        info.server_info = self.server_info.clone();
        info
    }
}

impl TerrainServer {
    pub fn new(provider: Arc<dyn KnowledgeProvider>, config: &Config) -> Self {
        let mut router = Self::tool_router();

        for (name, tool_config) in &config.tools {
            if let Some(desc) = &tool_config.description
                && let Some(route) = router.map.get_mut(name.as_str()) {
                    route.attr.description = Some(desc.clone().into());
                }
        }

        let instructions = config.instructions.clone().unwrap_or_else(|| {
            "terrain MCP server – search and read indexed Markdown files".to_string()
        });

        let server_info = Implementation::new(
            config
                .server
                .name
                .clone()
                .unwrap_or_else(|| env!("CARGO_PKG_NAME").to_string()),
            config
                .server
                .version
                .clone()
                .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string()),
        );

        Self {
            provider,
            tool_router: router,
            instructions,
            server_info,
            observer: None,
        }
    }

    /// Register an observer that receives one [`ToolCallEvent`] per handled
    /// tool call. Replaces any previously registered observer.
    pub fn with_observer(mut self, observer: Arc<dyn ToolCallObserver>) -> Self {
        self.observer = Some(observer);
        self
    }
}

// ---------------------------------------------------------------------------
// Serving
// ---------------------------------------------------------------------------

/// Serve `server` over `transport` until the connection closes.
///
/// The transport is anything rmcp can serve over, including a `(reader, writer)`
/// pair or a single bidirectional `AsyncRead + AsyncWrite` stream — stdio, a
/// named pipe, or a Unix domain socket. This keeps the rmcp dependency inside
/// terrain: callers hand over a stream and never have to name an rmcp type
/// (use the re-exported [`rmcp`] if they need transport constructors).
pub async fn serve_io<T, E, A>(server: TerrainServer, transport: T) -> Result<()>
where
    T: IntoTransport<RoleServer, E, A>,
    E: std::error::Error + Send + Sync + 'static,
{
    let running = server
        .serve(transport)
        .await
        .context("failed to start MCP server")?;
    running
        .waiting()
        .await
        .context("MCP server terminated unexpectedly")?;
    Ok(())
}

/// Build the rmcp Streamable HTTP [`tower::Service`] for this server.
///
/// terrain stops at the tower service: the caller mounts it into an HTTP
/// runtime (e.g. `axum::Router::nest_service`) and binds the socket. This
/// mirrors how [`serve_io`] leaves the stream to the caller and keeps `axum`
/// out of terrain's dependency tree — embedding hosts can graft the MCP
/// endpoint onto their own HTTP server.
///
/// Sessions are kept in-memory via [`LocalSessionManager`]. `config` controls
/// session/SSE behaviour and inbound `Host`/`Origin` validation; see
/// [`StreamableHttpServerConfig`].
#[cfg(feature = "streamable-http")]
pub fn streamable_http_service(
    server: TerrainServer,
    config: StreamableHttpServerConfig,
) -> StreamableHttpService<TerrainServer, LocalSessionManager> {
    StreamableHttpService::new(
        move || Ok(server.clone()),
        Arc::new(LocalSessionManager::default()),
        config,
    )
}

// ---------------------------------------------------------------------------
// TraverzeProvider (bundled reference provider)
// ---------------------------------------------------------------------------

/// The default [`KnowledgeProvider`], backed by a `traverze` search engine.
///
/// This is the reference implementation used by the terrain CLI binary. Hosts
/// that embed terrain (and bring their own engine) implement
/// [`KnowledgeProvider`] themselves and can compile terrain without the
/// `bundled-provider` feature.
///
/// Access control for `read_file` is enforced here: only paths registered in
/// [`IndexedPaths`] may be read.
#[cfg(feature = "bundled-provider")]
#[derive(Clone)]
pub struct TraverzeProvider {
    engine: Traverze,
    indexed_paths: IndexedPaths,
}

#[cfg(feature = "bundled-provider")]
impl TraverzeProvider {
    pub fn new(engine: Traverze, indexed_paths: IndexedPaths) -> Self {
        Self {
            engine,
            indexed_paths,
        }
    }
}

#[cfg(feature = "bundled-provider")]
#[async_trait]
impl KnowledgeProvider for TraverzeProvider {
    async fn search(&self, query: &str, opts: &SearchOptions) -> Result<Vec<SearchHit>> {
        let snippet = opts.snippet_max_chars.map(|max_num_chars| SnippetOptions {
            max_num_chars,
            format: SnippetFormat::Text,
        });
        let options = TraverzeSearchOptions {
            limit: opts.limit,
            snippet,
        };
        let hits = self.engine.search_with_options(query, options)?;
        Ok(hits
            .into_iter()
            .map(|h| SearchHit {
                path: h.path,
                score: h.score,
                snippet: h.snippet,
            })
            .collect())
    }

    async fn read_file(&self, path: &Path) -> Result<FileContent> {
        let canonical = fs::canonicalize(path)
            .with_context(|| format!("file not found: {}", path.display()))?;

        if !self.indexed_paths.contains(&canonical) {
            bail!("access denied: path is not in the index");
        }

        let content =
            fs::read_to_string(&canonical).context("failed to read file")?;

        Ok(FileContent {
            path: canonical.to_string_lossy().into_owned(),
            content,
        })
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

/// Create a `Traverze` engine and index the given files.
///
/// Any existing index in `index_dir` is **deleted** first, so the resulting
/// index reflects only `files`. Without this reset, documents indexed by
/// previous runs would persist and leak into search results.
#[cfg(feature = "bundled-provider")]
pub fn build_engine(index_dir: &Path, files: &[PathBuf]) -> Result<(Traverze, usize)> {
    if index_dir.exists() {
        fs::remove_dir_all(index_dir)
            .with_context(|| format!("failed to reset index dir: {}", index_dir.display()))?;
    }
    let engine = Traverze::new_in_dir_for_indexing(index_dir, TokenizerMode::LinderaIpadic, true)
        .context("traverze index initialization failed")?;
    let indexed = engine
        .index_files(files)
        .context("failed to index files")?;
    Ok((engine, indexed))
}
