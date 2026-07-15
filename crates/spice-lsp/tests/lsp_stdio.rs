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
    handshake_with_dialect(server, "ngspice").await;
}

async fn handshake_with_dialect(server: &mut LspProcess, dialect: &str) {
    server
        .send(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": null,
                "rootUri": null,
                "capabilities": {},
                "initializationOptions": { "dialect": dialect }
            }
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
async fn initialize_advertises_navigation_capabilities() {
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
    let caps = &response["result"]["capabilities"];
    assert_eq!(caps["documentSymbolProvider"], true);
    assert_eq!(caps["definitionProvider"], true);
    assert_eq!(caps["referencesProvider"], true);
    assert_eq!(caps["hoverProvider"], true);
    server.shutdown().await;
}

#[tokio::test]
async fn hover_on_tran_uses_dialect_corpus() {
    let uri = "file:///test/tran.cir";
    let source = "* demo\n.tran 1n 100n\n.end\n";
    let offset = source.find("tran").expect("tran");

    let mut server = LspProcess::spawn().await;
    handshake_with_dialect(&mut server, "hspice").await;
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
    let _ = server
        .read_notification("textDocument/publishDiagnostics")
        .await;

    let (line, character) = byte_offset_to_line_col(&source, offset);
    server
        .send(json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }
        }))
        .await;
    let response = server.read_response(10).await;
    let value = response["result"]["contents"]["value"]
        .as_str()
        .expect("hover markdown");
    assert!(
        value.contains("HSPICE") && value.contains(".tran"),
        "unexpected hover: {value}"
    );
    server.shutdown().await;
}

#[tokio::test]
async fn hover_on_tran_ngspice_corpus() {
    let uri = "file:///test/tran-ng.cir";
    let source = "* demo\n.tran 1n 100n\n.end\n";
    let offset = source.find("tran").expect("tran");

    let mut server = LspProcess::spawn().await;
    handshake_with_dialect(&mut server, "ngspice").await;
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
    let _ = server
        .read_notification("textDocument/publishDiagnostics")
        .await;

    let (line, character) = byte_offset_to_line_col(&source, offset);
    server
        .send(json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }
        }))
        .await;
    let response = server.read_response(11).await;
    let value = response["result"]["contents"]["value"]
        .as_str()
        .expect("hover markdown");
    assert!(
        value.contains("Ngspice") && value.contains(".tran"),
        "unexpected hover: {value}"
    );
    server.shutdown().await;
}

#[tokio::test]
async fn hover_on_hspice_dc_and_data() {
    let uri = "file:///test/dc-data.cir";
    let source = "* demo\n.data load rload\n+ 1k\n.enddata\n.dc DATA=load\n.end\n";

    let mut server = LspProcess::spawn().await;
    handshake_with_dialect(&mut server, "hspice").await;
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
    let _ = server
        .read_notification("textDocument/publishDiagnostics")
        .await;

    for (idx, (needle, expect_id_fragment)) in [("data", "data"), ("dc", "dc")]
        .into_iter()
        .enumerate()
    {
        let offset = source.find(needle).expect(needle);
        let (line, character) = byte_offset_to_line_col(&source, offset);
        let id = 20 + idx as u64;
        server
            .send(json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "textDocument/hover",
                "params": {
                    "textDocument": { "uri": uri },
                    "position": { "line": line, "character": character }
                }
            }))
            .await;
        let response = server.read_response(id).await;
        let value = response["result"]["contents"]["value"]
            .as_str()
            .expect("hover markdown");
        assert!(
            value.contains("HSPICE") && value.to_ascii_lowercase().contains(expect_id_fragment),
            "unexpected hover for {needle}: {value}"
        );
    }
    server.shutdown().await;
}

#[tokio::test]
async fn document_symbol_returns_subckt_outline() {
    let uri = "file:///test/subckt.cir";
    let source = fixture("valid/subckt.cir");

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
    let _ = server
        .read_notification("textDocument/publishDiagnostics")
        .await;

    server
        .send(json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/documentSymbol",
            "params": { "textDocument": { "uri": uri } }
        }))
        .await;
    let response = server.read_response(2).await;
    let symbols = response["result"].as_array().expect("symbols");
    assert!(
        symbols.iter().any(|s| s["name"].as_str() == Some("buffer")),
        "expected buffer subcircuit in outline: {symbols:?}"
    );
    server.shutdown().await;
}

#[tokio::test]
async fn goto_definition_on_subckt_reference() {
    let uri = "file:///test/subckt.cir";
    let source = fixture("valid/subckt.cir");
    let use_line = source.lines().nth(4).expect("X1 line");
    let subckt_offset = use_line.find("buffer").expect("buffer in X1 line");
    let prefix = source
        .lines()
        .take(4)
        .chain(std::iter::once(use_line))
        .collect::<Vec<_>>()
        .join("\n");
    let offset = prefix.len() - use_line.len() + subckt_offset;

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
    let _ = server
        .read_notification("textDocument/publishDiagnostics")
        .await;

    let (line, character) = byte_offset_to_line_col(&source, offset);
    server
        .send(json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }
        }))
        .await;
    let response = server.read_response(3).await;
    let location = &response["result"];
    assert_eq!(location["uri"], uri);
    assert_eq!(location["range"]["start"]["line"], 1);
    server.shutdown().await;
}

#[tokio::test]
async fn goto_definition_follows_include_file() {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test-data/valid/with-include");
    let top_path = dir.join("top.cir");
    let uri = url::Url::from_file_path(&top_path).expect("top uri");
    let source = std::fs::read_to_string(&top_path).expect("top.cir");
    let offset = source.find("nch").expect("nch");

    let mut server = LspProcess::spawn().await;
    handshake_with_dialect(&mut server, "hspice").await;
    server
        .send(json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri.as_str(),
                    "languageId": "spice",
                    "version": 1,
                    "text": source
                }
            }
        }))
        .await;
    let diagnostics = server
        .read_notification("textDocument/publishDiagnostics")
        .await;
    let diags = diagnostics["params"]["diagnostics"]
        .as_array()
        .expect("diagnostics array");
    assert!(
        !diags.iter().any(|d| d["code"] == "spice/unknown-model"),
        "expected include to resolve nch/buffer: {diags:?}"
    );

    let (line, character) = byte_offset_to_line_col(&source, offset);
    server
        .send(json!({
            "jsonrpc": "2.0",
            "id": 31,
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": line, "character": character }
            }
        }))
        .await;
    let response = server.read_response(31).await;
    let location = &response["result"];
    let def_uri = location["uri"].as_str().expect("uri");
    assert!(
        def_uri.ends_with("models.inc"),
        "expected definition in models.inc, got {def_uri}"
    );
    server.shutdown().await;
}

#[tokio::test]
async fn references_on_subckt_definition() {
    let uri = "file:///test/subckt.cir";
    let source = fixture("valid/subckt.cir");
    let def_line = source.lines().nth(1).expect(".subckt line");
    let name_offset = def_line.find("buffer").expect("buffer in .subckt");
    let prefix = source.lines().take(1).collect::<Vec<_>>().join("\n");
    let offset = prefix.len() + 1 + name_offset;

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
    let _ = server
        .read_notification("textDocument/publishDiagnostics")
        .await;

    let (line, character) = byte_offset_to_line_col(&source, offset);
    server
        .send(json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "textDocument/references",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
                "context": { "includeDeclaration": true }
            }
        }))
        .await;
    let response = server.read_response(4).await;
    let locations = response["result"].as_array().expect("references");
    assert!(
        locations.len() >= 2,
        "expected definition + usage references, got {locations:?}"
    );

    server
        .send(json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "textDocument/references",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
                "context": { "includeDeclaration": false }
            }
        }))
        .await;
    let response = server.read_response(5).await;
    let without_decl = response["result"].as_array().expect("references");
    assert_eq!(
        without_decl.len(),
        locations.len() - 1,
        "includeDeclaration:false should omit the definition site"
    );
    assert!(
        without_decl
            .iter()
            .all(|loc| loc["range"]["start"]["line"] != 1),
        "definition line must be excluded when includeDeclaration is false: {without_decl:?}"
    );
    server.shutdown().await;
}

fn byte_offset_to_line_col(source: &str, offset: usize) -> (u32, u32) {
    let prefix = &source[..offset.min(source.len())];
    let line = prefix.matches('\n').count() as u32;
    let line_start = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let character = source[line_start..offset]
        .chars()
        .map(|c| c.len_utf16())
        .sum::<usize>() as u32;
    (line, character)
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
async fn hspice_data_block_without_plus_publishes_no_diagnostics() {
    // HSPICE allows bare multi-line .DATA rows (no leading '+'). The LSP must
    // not publish syntax errors for those lines.
    let uri = "file:///test/hspice-data-block.cir";
    let source = fixture("valid/hspice-data-block.cir");

    let mut server = LspProcess::spawn().await;
    handshake_with_dialect(&mut server, "hspice").await;
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
        .expect("diagnostics array");
    assert!(
        diagnostics.is_empty(),
        "expected no diagnostics for bare .DATA rows, got {diagnostics:?}"
    );
    server.shutdown().await;
}

#[tokio::test]
async fn sp_and_spf_uris_publish_diagnostics() {
    // Extension maps .sp / .spf → language id `spice`; the server is URI-agnostic.
    for (uri, fixture_name) in [
        ("file:///test/simple-rc.sp", "valid/simple-rc.sp"),
        ("file:///test/simple-rc.spf", "valid/simple-rc.spf"),
    ] {
        let source = fixture(fixture_name);
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
        assert_eq!(notification["params"]["uri"], uri);
        assert!(
            notification["params"]["diagnostics"]
                .as_array()
                .unwrap()
                .is_empty(),
            "expected no diagnostics for {uri}"
        );
        server.shutdown().await;
    }
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
