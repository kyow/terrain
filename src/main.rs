use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Parser;
use rmcp::{ServerHandler, ServiceExt, model::ServerInfo, transport::stdio};
use traverze::Traverze;

/// terrain MCP server – Markdown full-text search
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Path to the directory containing Markdown files
    #[arg(long)]
    dir: PathBuf,
}

#[derive(Clone, Default)]
struct TerrainServer;

impl ServerHandler for TerrainServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("terrain MCP server".to_string()),
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let target_dir = resolve_dir(&cli.dir)?;
    let markdown_files = collect_markdown_files(&target_dir)?;

    let engine = Traverze::new().context("traverze index initialization failed")?;
    let indexed = engine
        .index_files(&markdown_files)
        .context("failed to index markdown files")?;

    eprintln!(
        "indexed {} markdown files from {}",
        indexed,
        target_dir.display()
    );

    let server: rmcp::service::RunningService<rmcp::RoleServer, TerrainServer> =
        TerrainServer.serve(stdio()).await?;
    server.waiting().await?;
    Ok(())
}

fn resolve_dir(dir: &Path) -> Result<PathBuf> {
    let canonical = fs::canonicalize(dir)
        .with_context(|| format!("directory not found: {}", dir.display()))?;

    if !canonical.is_dir() {
        bail!("not a directory: {}", canonical.display());
    }

    Ok(canonical)
}

fn collect_markdown_files(base_dir: &Path) -> Result<Vec<PathBuf>> {
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
