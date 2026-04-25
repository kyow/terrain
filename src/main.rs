use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use notify::event::{CreateKind, RemoveKind, RenameMode};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use rmcp::{ServiceExt, transport::stdio};
use terrain::{Config, IndexedPaths, TerrainServer, build_engine, resolve_dir};
use traverze::Traverze;

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

    let indexed_paths = IndexedPaths::new();
    indexed_paths.extend(markdown_files);

    eprintln!(
        "indexed {} markdown files from {}",
        indexed,
        target_dir.display()
    );

    let _watcher = start_watcher(engine.clone(), indexed_paths.clone(), target_dir.clone())
        .context("failed to start file watcher")?;
    eprintln!("watching {} for changes", target_dir.display());

    let server = TerrainServer::new(engine, indexed_paths, &config)
        .serve(stdio())
        .await?;
    server.waiting().await?;
    Ok(())
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

            if file_type.is_file() && is_markdown(&path) {
                // Canonicalize so paths line up with `fs::canonicalize` lookups
                // performed by `read_file`, which on Windows yields `\\?\` paths.
                match fs::canonicalize(&path) {
                    Ok(canonical) => files.push(canonical),
                    Err(e) => eprintln!(
                        "warning: skipping {} (canonicalize failed: {e})",
                        path.display()
                    ),
                }
            }
        }
    }

    Ok(files)
}

/// Start watching the directory for file changes and update the index accordingly.
///
/// Returns the [`RecommendedWatcher`] handle. The watcher stops when this
/// handle is dropped, so keep it alive for the lifetime of the server.
fn start_watcher(
    engine: Traverze,
    indexed_paths: IndexedPaths,
    base_dir: PathBuf,
) -> Result<RecommendedWatcher> {
    let (tx, rx) = std::sync::mpsc::channel();

    let mut watcher = RecommendedWatcher::new(
        move |res: std::result::Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        },
        notify::Config::default().with_poll_interval(Duration::from_secs(2)),
    )
    .context("failed to create file watcher")?;

    watcher
        .watch(base_dir.as_ref(), RecursiveMode::Recursive)
        .with_context(|| format!("failed to watch directory: {}", base_dir.display()))?;

    tokio::spawn(async move {
        tokio::task::spawn_blocking(move || {
            let debounce = Duration::from_millis(500);
            let mut pending: HashMap<PathBuf, EventKind> = HashMap::new();

            loop {
                let event = match rx.recv() {
                    Ok(e) => e,
                    Err(_) => break,
                };
                accumulate(&mut pending, &event);

                loop {
                    match rx.recv_timeout(debounce) {
                        Ok(e) => accumulate(&mut pending, &e),
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => break,
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }

                // Adds: canonicalize so the stored form matches read_file lookups.
                let to_index: Vec<PathBuf> = pending
                    .iter()
                    .filter(|(_, kind)| {
                        matches!(kind, EventKind::Create(_) | EventKind::Modify(_))
                    })
                    .filter_map(|(path, _)| fs::canonicalize(path).ok())
                    .collect();

                // Removes: file is already gone, so canonicalize will fail.
                // Use the literal path; IndexedPaths::remove may miss the
                // canonical form but that only leaves a stale entry — read_file
                // would still fail at the I/O step.
                let to_remove: Vec<PathBuf> = pending
                    .iter()
                    .filter(|(_, kind)| matches!(kind, EventKind::Remove(_)))
                    .map(|(path, _)| path.clone())
                    .collect();

                pending.clear();

                if !to_remove.is_empty() {
                    for path in &to_remove {
                        indexed_paths.remove(path);
                    }
                    match engine.remove_files(&to_remove) {
                        Ok(n) => eprintln!("watcher: removed {} file(s) from index", n),
                        Err(e) => eprintln!("watcher: remove error: {e}"),
                    }
                }

                if !to_index.is_empty() {
                    let _ = engine.remove_files(&to_index);
                    match engine.index_files(&to_index) {
                        Ok(n) => {
                            indexed_paths.extend(to_index.iter().cloned());
                            eprintln!("watcher: re-indexed {} file(s)", n);
                        }
                        Err(e) => eprintln!("watcher: re-index error: {e}"),
                    }
                }
            }
        });
    });

    Ok(watcher)
}

/// Accumulate file-system events into `pending`, keeping only the latest
/// [`EventKind`] per path and filtering to Markdown files.
///
/// Rename events are normalised into Remove / Create so that the processing
/// loop does not need to know about renames.
fn accumulate(pending: &mut HashMap<PathBuf, EventKind>, event: &notify::Event) {
    match &event.kind {
        EventKind::Modify(notify::event::ModifyKind::Name(mode)) => match mode {
            // "From" carries the old path → treat as removal.
            RenameMode::From => {
                if let Some(old) = event.paths.first() {
                    if is_markdown(old) {
                        pending.insert(old.clone(), EventKind::Remove(RemoveKind::File));
                    }
                }
            }
            // "To" carries the new path → treat as creation.
            RenameMode::To => {
                if let Some(new) = event.paths.first() {
                    if is_markdown(new) {
                        pending.insert(new.clone(), EventKind::Create(CreateKind::File));
                    }
                }
            }
            // "Both" carries [old, new] in a single event.
            RenameMode::Both => {
                if let Some(old) = event.paths.first() {
                    if is_markdown(old) {
                        pending.insert(old.clone(), EventKind::Remove(RemoveKind::File));
                    }
                }
                if let Some(new) = event.paths.get(1) {
                    if is_markdown(new) {
                        pending.insert(new.clone(), EventKind::Create(CreateKind::File));
                    }
                }
            }
            // "Any" / "Other" – direction unknown. If the file exists now
            // treat it as a creation; otherwise as a removal.
            _ => {
                for path in &event.paths {
                    if is_markdown(path) {
                        let kind = if path.exists() {
                            EventKind::Create(CreateKind::File)
                        } else {
                            EventKind::Remove(RemoveKind::File)
                        };
                        pending.insert(path.clone(), kind);
                    }
                }
            }
        },
        // Non-rename events: pass through as-is.
        _ => {
            for path in &event.paths {
                if is_markdown(path) {
                    pending.insert(path.clone(), event.kind.clone());
                }
            }
        }
    }
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("md"))
        .unwrap_or(false)
}
