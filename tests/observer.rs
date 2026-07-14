//! Integration tests for the tool-call observation hook.
//!
//! Drives a `TerrainServer` end-to-end over an in-memory duplex stream with
//! raw JSON-RPC frames (the newline-delimited framing the stdio transport
//! uses), asserting that a registered observer sees each tool call.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Result, bail};
use async_trait::async_trait;
use serde_json::{Value, json};
use terrain::{
    Config, FileContent, KnowledgeProvider, SearchHit, SearchOptions, TerrainServer, ToolCallEvent,
    ToolCallObserver, serve_io,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, DuplexStream, ReadHalf, WriteHalf};

struct StubProvider;

#[async_trait]
impl KnowledgeProvider for StubProvider {
    async fn search(&self, query: &str, _opts: &SearchOptions) -> Result<Vec<SearchHit>> {
        if query == "boom" {
            bail!("engine exploded");
        }
        Ok(vec![SearchHit {
            path: "/kb/hit.md".to_string(),
            score: 1.0,
            snippet: None,
        }])
    }

    async fn read_file(&self, path: &Path) -> Result<FileContent> {
        Ok(FileContent {
            path: path.display().to_string(),
            content: "hello".to_string(),
        })
    }
}

/// One observed call with the borrowed event data made owned.
struct RecordedCall {
    tool_name: String,
    arguments: Option<Value>,
    /// `Ok`: the `CallToolResult` serialized to JSON. `Err`: protocol error message.
    outcome: Result<Value, String>,
    duration: Duration,
}

#[derive(Default)]
struct RecordingObserver(Mutex<Vec<RecordedCall>>);

impl ToolCallObserver for RecordingObserver {
    fn on_tool_call(&self, event: &ToolCallEvent<'_>) {
        self.0.lock().unwrap().push(RecordedCall {
            tool_name: event.tool_name.to_string(),
            arguments: event.arguments.map(|args| Value::Object(args.clone())),
            outcome: match event.outcome {
                Ok(result) => Ok(serde_json::to_value(result).unwrap()),
                Err(e) => Err(e.message.to_string()),
            },
            duration: event.duration,
        });
    }
}

/// Client half of the in-memory MCP session.
struct TestClient {
    reader: BufReader<ReadHalf<DuplexStream>>,
    writer: WriteHalf<DuplexStream>,
}

impl TestClient {
    async fn send(&mut self, frame: Value) {
        let mut line = frame.to_string();
        line.push('\n');
        self.writer.write_all(line.as_bytes()).await.unwrap();
    }

    async fn recv(&mut self) -> Value {
        let mut line = String::new();
        tokio::time::timeout(Duration::from_secs(10), self.reader.read_line(&mut line))
            .await
            .expect("timed out waiting for a server frame")
            .unwrap();
        serde_json::from_str(&line).unwrap()
    }

    async fn call_tool(&mut self, id: u64, name: &str, arguments: Value) -> Value {
        self.send(json!({
            "jsonrpc": "2.0", "id": id, "method": "tools/call",
            "params": {"name": name, "arguments": arguments}
        }))
        .await;
        self.recv().await
    }
}

/// Start a `TerrainServer` with `observer` on an in-memory stream and run
/// the MCP initialization handshake.
async fn start(observer: Arc<RecordingObserver>) -> TestClient {
    let server =
        TerrainServer::new(Arc::new(StubProvider), &Config::default()).with_observer(observer);
    let (client_end, server_end) = tokio::io::duplex(64 * 1024);
    tokio::spawn(async move {
        let _ = serve_io(server, server_end).await;
    });

    let (reader, writer) = tokio::io::split(client_end);
    let mut client = TestClient {
        reader: BufReader::new(reader),
        writer,
    };
    client
        .send(json!({
            "jsonrpc": "2.0", "id": 0, "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "observer-test", "version": "0.0.0"}
            }
        }))
        .await;
    client.recv().await;
    client
        .send(json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
        .await;
    client
}

fn recorded_text(result: &Value) -> String {
    result["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

#[tokio::test]
async fn observer_receives_input_and_output_of_a_successful_call() {
    let observer = Arc::new(RecordingObserver::default());
    let mut client = start(observer.clone()).await;

    let response = client
        .call_tool(1, "search", json!({"query": "terrain"}))
        .await;
    assert!(response["error"].is_null(), "unexpected error: {response}");

    let calls = observer.0.lock().unwrap();
    assert_eq!(calls.len(), 1, "expected exactly one event per tool call");
    let call = &calls[0];
    assert_eq!(call.tool_name, "search");
    assert_eq!(call.arguments, Some(json!({"query": "terrain"})));
    let result = call.outcome.as_ref().expect("expected a tool result");
    assert_ne!(result["isError"], json!(true));
    assert!(recorded_text(result).contains("/kb/hit.md"));
    assert!(call.duration < Duration::from_secs(60));
}

#[tokio::test]
async fn observer_sees_tool_level_errors_in_the_result() {
    let observer = Arc::new(RecordingObserver::default());
    let mut client = start(observer.clone()).await;

    let response = client
        .call_tool(1, "search", json!({"query": "boom"}))
        .await;
    assert_eq!(response["result"]["isError"], json!(true));

    let calls = observer.0.lock().unwrap();
    assert_eq!(calls.len(), 1);
    let result = calls[0].outcome.as_ref().expect("expected a tool result");
    assert_eq!(result["isError"], json!(true));
    assert!(recorded_text(result).contains("engine exploded"));
}

#[tokio::test]
async fn observer_sees_protocol_errors_for_unknown_tools() {
    let observer = Arc::new(RecordingObserver::default());
    let mut client = start(observer.clone()).await;

    let response = client.call_tool(1, "no_such_tool", json!({})).await;
    assert!(
        !response["error"].is_null(),
        "expected a JSON-RPC error: {response}"
    );

    let calls = observer.0.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].tool_name, "no_such_tool");
    let message = calls[0]
        .outcome
        .as_ref()
        .expect_err("expected a protocol error");
    assert!(message.contains("tool not found"), "got: {message}");
}
