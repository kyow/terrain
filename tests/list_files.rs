#![cfg(feature = "bundled-provider")]

//! Tests for `list_files`: provider-level paging semantics against a real
//! traverze index, plus an end-to-end pass through the MCP tool surface.

mod common;

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Value, json};
use terrain::{
    Config, IndexedPaths, KnowledgeProvider, ListOptions, TerrainServer, TraverzeProvider,
    build_engine,
};

/// Build a provider over `count` Markdown files named `doc-00.md`, `doc-01.md`, …
fn provider_with_files(tmp: &tempfile::TempDir, count: usize) -> TraverzeProvider {
    let docs = tmp.path().join("docs");
    fs::create_dir_all(&docs).unwrap();
    let mut files: Vec<PathBuf> = Vec::new();
    for i in 0..count {
        let path = docs.join(format!("doc-{i:02}.md"));
        fs::write(&path, format!("document number {i}")).unwrap();
        files.push(fs::canonicalize(&path).unwrap());
    }
    let (engine, indexed) = build_engine(&tmp.path().join("index"), &files).unwrap();
    assert_eq!(indexed, count);
    let indexed_paths = IndexedPaths::new();
    indexed_paths.extend(files);
    TraverzeProvider::new(engine, indexed_paths)
}

#[tokio::test]
async fn list_files_returns_sorted_paths_and_total() {
    let tmp = tempfile::tempdir().unwrap();
    let provider = provider_with_files(&tmp, 5);

    let list = provider.list_files(&ListOptions::default()).await.unwrap();
    assert_eq!(list.total, 5);
    assert_eq!(list.offset, 0);
    assert_eq!(list.paths.len(), 5);
    let mut sorted = list.paths.clone();
    sorted.sort();
    assert_eq!(list.paths, sorted, "paths must come back sorted");
    assert!(list.paths.iter().all(|p| p.ends_with(".md")));
}

#[tokio::test]
async fn list_files_pages_with_limit_and_offset() {
    let tmp = tempfile::tempdir().unwrap();
    let provider = provider_with_files(&tmp, 5);

    let all = provider.list_files(&ListOptions::default()).await.unwrap();

    let page = provider
        .list_files(&ListOptions { limit: 2, offset: 0 })
        .await
        .unwrap();
    assert_eq!(page.total, 5);
    assert_eq!(page.paths, all.paths[0..2]);

    let page = provider
        .list_files(&ListOptions { limit: 2, offset: 4 })
        .await
        .unwrap();
    assert_eq!(page.offset, 4);
    assert_eq!(page.paths, all.paths[4..5], "last page may be short");

    // Beyond the end: an empty page, but the total is still reported.
    let page = provider
        .list_files(&ListOptions {
            limit: 2,
            offset: 99,
        })
        .await
        .unwrap();
    assert_eq!(page.total, 5);
    assert!(page.paths.is_empty());

    // limit 0 is the "just give me the count" call.
    let page = provider
        .list_files(&ListOptions { limit: 0, offset: 0 })
        .await
        .unwrap();
    assert_eq!(page.total, 5);
    assert!(page.paths.is_empty());
}

/// Parse the JSON payload out of a `tools/call` response.
fn tool_payload(response: &Value) -> Value {
    assert!(
        response["error"].is_null(),
        "unexpected error: {response}"
    );
    assert_ne!(response["result"]["isError"], json!(true));
    serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}

#[tokio::test]
async fn list_files_tool_serves_paged_json() {
    let tmp = tempfile::tempdir().unwrap();
    let provider = provider_with_files(&tmp, 3);
    let server = TerrainServer::new(Arc::new(provider), &Config::default());
    let mut client = common::start(server).await;

    // No arguments: defaults apply (limit 100, offset 0).
    let payload = tool_payload(&client.call_tool(1, "list_files", json!({})).await);
    assert_eq!(payload["total"], json!(3));
    assert_eq!(payload["offset"], json!(0));
    assert_eq!(payload["paths"].as_array().unwrap().len(), 3);

    // Explicit paging.
    let payload = tool_payload(
        &client
            .call_tool(2, "list_files", json!({"limit": 1, "offset": 2}))
            .await,
    );
    assert_eq!(payload["total"], json!(3));
    assert_eq!(payload["offset"], json!(2));
    assert_eq!(payload["paths"].as_array().unwrap().len(), 1);

    // A listed path feeds straight into read_file.
    let path = payload["paths"][0].as_str().unwrap().to_string();
    let response = client.call_tool(3, "read_file", json!({"path": path})).await;
    assert!(response["error"].is_null(), "unexpected error: {response}");
    assert_ne!(response["result"]["isError"], json!(true));
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.starts_with("document number"), "got: {text}");
}
