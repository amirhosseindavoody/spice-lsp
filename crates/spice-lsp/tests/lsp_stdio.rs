use std::process::Stdio;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::time::timeout;

const TIMEOUT: Duration = Duration::from_secs(5);

struct LspProcess {
    child: Child,
    writer: tokio::process::ChildStdin,
    reader: BufReader<tokio::process::ChildStdout>,
}

impl LspProcess {
    async fn spawn() -> Self {
        let bin = env!("CARGO_BIN_EXE_spice-lsp");
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .expect("spawn spice-lsp");

        let writer = child.stdin.take().expect("stdin");
        let reader = BufReader::new(child.stdout.take().expect("stdout"));

        Self {
            child,
            writer,
            reader,
        }
    }

    async fn send(&mut self, message: Value) {
        let body = serde_json::to_vec(&message).expect("serialize");
        self.writer
            .write_all(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes())
            .await
            .expect("header");
        self.writer.write_all(&body).await.expect("body");
        self.writer.flush().await.expect("flush");
    }

    async fn read_timed(&mut self) -> Value {
        timeout(TIMEOUT, read_message(&mut self.reader))
            .await
            .expect("timed out waiting for LSP message")
            .expect("read LSP message")
    }

    async fn read_response(&mut self, id: u64) -> Value {
        loop {
            let message = self.read_timed().await;
            if message.get("id") == Some(&json!(id)) {
                return message;
            }
        }
    }

    async fn read_notification(&mut self, method: &str) -> Value {
        loop {
            let message = self.read_timed().await;
            if message.get("method").and_then(Value::as_str) == Some(method) {
                return message;
            }
        }
    }

    async fn shutdown(mut self) {
        self.send(json!({"jsonrpc":"2.0","id":99,"method":"shutdown","params":null}))
            .await;
        let _ = self.read_response(99).await;
        self.send(json!({"jsonrpc":"2.0","method":"exit","params":null}))
            .await;
        let _ = self.child.kill().await;
    }
}

async fn read_message(reader: &mut BufReader<tokio::process::ChildStdout>) -> std::io::Result<Value> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).await? == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "server closed stdout",
            ));
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length: ") {
            content_length = Some(value.trim().parse().expect("length"));
        }
    }
    let len = content_length.expect("Content-Length");
    let mut body = vec![0_u8; len];
    reader.read_exact(&mut body).await?;
    Ok(serde_json::from_slice(&body)?)
}

fn fixture(name: &str) -> String {
    std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../test-data")
            .join(name),
    )
    .expect("fixture")
}

async fn handshake(server: &mut LspProcess) {
    server
        .send(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": { "processId": null, "rootUri": null, "capabilities": {} }
        }))
        .await;
    let _ = server.read_response(1).await;
    server
        .send(json!({"jsonrpc":"2.0","method":"initialized","params":{}}))
        .await;
}

#[tokio::test]
async fn initialize_advertises_incremental_sync() {
    let mut server = LspProcess::spawn().await;
    server
        .send(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": { "processId": null, "rootUri": null, "capabilities": {} }
        }))
        .await;
    let response = server.read_response(1).await;
    assert_eq!(response["result"]["capabilities"]["textDocumentSync"]["change"], 2);
    server.shutdown().await;
}

#[tokio::test]
async fn unclosed_subckt_publishes_diagnostics() {
    let uri = "file:///test/unclosed-subckt.cir";
    let source = fixture("invalid/unclosed-subckt.cir");

    let mut server = LspProcess::spawn().await;
    handshake(&mut server).await;
    server
        .send(json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "spice",
                    "version": 1,
                    "text": source
                }
            }
        }))
        .await;

    let notification = server
        .read_notification("textDocument/publishDiagnostics")
        .await;
    let diagnostics = notification["params"]["diagnostics"]
        .as_array()
        .expect("diagnostics");
    assert!(!diagnostics.is_empty());
    assert!(
        diagnostics
            .iter()
            .any(|d| d["message"].as_str().unwrap().contains("missing .ends"))
    );
    server.shutdown().await;
}

#[tokio::test]
async fn valid_netlist_publishes_no_diagnostics() {
    let uri = "file:///test/simple-rc.cir";
    let source = fixture("valid/simple-rc.cir");

    let mut server = LspProcess::spawn().await;
    handshake(&mut server).await;
    server
        .send(json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "spice",
                    "version": 1,
                    "text": source
                }
            }
        }))
        .await;

    let notification = server
        .read_notification("textDocument/publishDiagnostics")
        .await;
    assert!(
        notification["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    server.shutdown().await;
}

#[tokio::test]
async fn did_change_updates_diagnostics() {
    let uri = "file:///test/live.cir";
    let invalid = fixture("invalid/unclosed-subckt.cir");
    let fixed = fixture("valid/subckt.cir");

    let mut server = LspProcess::spawn().await;
    handshake(&mut server).await;
    server
        .send(json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "spice",
                    "version": 1,
                    "text": invalid
                }
            }
        }))
        .await;

    let first = server
        .read_notification("textDocument/publishDiagnostics")
        .await;
    assert!(!first["params"]["diagnostics"].as_array().unwrap().is_empty());

    server
        .send(json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [{ "text": fixed }]
            }
        }))
        .await;

    let second = server
        .read_notification("textDocument/publishDiagnostics")
        .await;
    assert!(
        second["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    server.shutdown().await;
}
