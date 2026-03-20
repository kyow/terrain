use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use rmcp::{ServiceExt, transport::stdio};
use terrain::{Config, TerrainServer, build_engine, collect_markdown_files, resolve_dir, start_watcher};

/// terrain MCP server – Markdown full-text search
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Path to the directory containing Markdown files
    #[arg(long)]
    dir: PathBuf,
    /// Path to a TOML config file for customizing tool description
    #[arg(long)]
    config: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let target_dir = resolve_dir(&cli.dir)?;
    let config = match &cli.config {
        Some(path) => Config::load(path)?,
        None => Config::default(),
    };
    let markdown_files = collect_markdown_files(&target_dir)?;

    let index_dir = env::temp_dir().join("terrain-index");
    let (engine, indexed) =
        build_engine(&index_dir, &markdown_files).context("failed to build search engine")?;

    eprintln!(
        "indexed {} markdown files from {}",
        indexed,
        target_dir.display()
    );

    let _watcher = start_watcher(engine.clone(), target_dir.clone())
        .context("failed to start file watcher")?;
    eprintln!("watching {} for changes", target_dir.display());

    let server = TerrainServer::new(engine, target_dir, &config, indexed)
        .serve(stdio())
        .await?;
    server.waiting().await?;
    Ok(())
}
