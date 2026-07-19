//! Shared test client for driving a `TerrainServer` end-to-end over an
//! in-memory duplex stream with raw JSON-RPC frames (the newline-delimited
//! framing the stdio transport uses).

use std::time::Duration;

use serde_json::{Value, json};
use terrain::{TerrainServer, serve_io};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, DuplexStream, ReadHalf, WriteHalf};

/// Client half of the in-memory MCP session.
pub struct TestClient {
    reader: BufReader<ReadHalf<DuplexStream>>,
    writer: WriteHalf<DuplexStream>,
}

impl TestClient {
    pub async fn send(&mut self, frame: Value) {
        let mut line = frame.to_string();
        line.push('\n');
        self.writer.write_all(line.as_bytes()).await.unwrap();
    }

    pub async fn recv(&mut self) -> Value {
        let mut line = String::new();
        tokio::time::timeout(Duration::from_secs(10), self.reader.read_line(&mut line))
            .await
            .expect("timed out waiting for a server frame")
            .unwrap();
        serde_json::from_str(&line).unwrap()
    }

    pub async fn call_tool(&mut self, id: u64, name: &str, arguments: Value) -> Value {
        self.send(json!({
            "jsonrpc": "2.0", "id": id, "method": "tools/call",
            "params": {"name": name, "arguments": arguments}
        }))
        .await;
        self.recv().await
    }
}

/// Serve `server` on an in-memory stream and run the MCP initialization
/// handshake, returning the connected client half.
pub async fn start(server: TerrainServer) -> TestClient {
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
                "clientInfo": {"name": "terrain-test", "version": "0.0.0"}
            }
        }))
        .await;
    client.recv().await;
    client
        .send(json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
        .await;
    client
}
