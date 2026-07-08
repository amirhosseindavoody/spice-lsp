use std::collections::HashMap;
use std::sync::Arc;

use spice_parser::analyze;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use url::Url;

use crate::convert::to_lsp_diagnostic;

#[derive(Debug, Clone)]
struct Document {
    text: String,
    version: i32,
}

pub struct Backend {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, Document>>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn publish_diagnostics(&self, uri: Url, text: &str) {
        let result = analyze(text);
        let diagnostics = result
            .diagnostics
            .into_iter()
            .map(|d| to_lsp_diagnostic(text, d))
            .collect::<Vec<_>>();

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    async fn apply_change(&self, uri: &Url, change: &TextDocumentContentChangeEvent) {
        let mut docs = self.documents.write().await;
        let Some(doc) = docs.get_mut(uri) else {
            return;
        };

        if let Some(range) = &change.range {
            let start = position_to_byte_offset(&doc.text, range.start);
            let end = position_to_byte_offset(&doc.text, range.end);
            if start <= end && end <= doc.text.len() {
                doc.text.replace_range(start..end, &change.text);
            }
        } else {
            doc.text = change.text.clone();
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::INCREMENTAL),
                        ..Default::default()
                    },
                )),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "spice-lsp".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {}

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        let version = params.text_document.version;

        {
            let mut docs = self.documents.write().await;
            docs.insert(
                uri.clone(),
                Document {
                    text: text.clone(),
                    version,
                },
            );
        }

        self.publish_diagnostics(uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;

        {
            let mut docs = self.documents.write().await;
            if let Some(doc) = docs.get_mut(&uri) {
                doc.version = version;
            }
        }

        for change in params.content_changes {
            self.apply_change(&uri, &change).await;
        }

        let text = {
            let docs = self.documents.read().await;
            docs.get(&uri).map(|d| d.text.clone())
        };

        if let Some(text) = text {
            self.publish_diagnostics(uri, &text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.write().await.remove(&uri);
    }
}

fn position_to_byte_offset(source: &str, position: Position) -> usize {
    let mut line = 0u32;
    let mut byte = 0usize;

    for ch in source.chars() {
        if line == position.line {
            break;
        }
        byte += ch.len_utf8();
        if ch == '\n' {
            line += 1;
        }
    }

    if line < position.line {
        return source.len();
    }

    let line_start = byte;
    let mut col = 0u32;
    for ch in source[line_start..].chars() {
        if ch == '\n' {
            break;
        }
        if col == position.character {
            break;
        }
        byte += ch.len_utf8();
        col += ch.len_utf16() as u32;
    }

    byte.min(source.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> String {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../test-data")
            .join(name);
        std::fs::read_to_string(root).expect("fixture exists")
    }

    #[test]
    fn analyze_invalid_fixture_produces_diagnostics() {
        let source = fixture("invalid/unclosed-subckt.cir");
        let result = analyze(&source);
        assert!(!result.diagnostics.is_empty());
    }
}
