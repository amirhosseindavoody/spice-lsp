use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use spice_parser::{analyze, Index};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use url::Url;

use crate::convert::{
    definition_location, position_to_byte_offset, reference_locations, to_document_symbols,
    to_lsp_diagnostic,
};

/// Coalesce rapid `didChange` events before re-analyzing and publishing diagnostics.
const DIAGNOSTIC_DEBOUNCE: Duration = Duration::from_millis(150);

#[derive(Debug, Clone)]
struct Document {
    text: String,
    version: i32,
    index: Index,
}

pub struct Backend {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, Document>>>,
    debounce_tasks: Arc<RwLock<HashMap<Url, JoinHandle<()>>>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
            debounce_tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn analyze_and_store(&self, uri: &Url, text: &str) -> (Vec<Diagnostic>, Index) {
        let result = analyze(text);
        let diagnostics = result
            .diagnostics
            .into_iter()
            .map(|d| to_lsp_diagnostic(text, d))
            .collect::<Vec<_>>();
        let index = result.index;

        if let Some(doc) = self.documents.write().await.get_mut(uri) {
            doc.index = index.clone();
        }

        (diagnostics, index)
    }

    async fn publish_diagnostics(&self, uri: Url, text: &str, version: Option<i32>) {
        let (diagnostics, _) = self.analyze_and_store(&uri, text).await;
        self.client
            .publish_diagnostics(uri, diagnostics, version)
            .await;
    }

    async fn cancel_debounce(&self, uri: &Url) {
        if let Some(handle) = self.debounce_tasks.write().await.remove(uri) {
            handle.abort();
        }
    }

    async fn schedule_diagnostics(&self, uri: Url) {
        self.cancel_debounce(&uri).await;

        let documents = Arc::clone(&self.documents);
        let client = self.client.clone();
        let debounce_tasks = Arc::clone(&self.debounce_tasks);
        let uri_for_task = uri.clone();

        let handle = tokio::spawn(async move {
            tokio::time::sleep(DIAGNOSTIC_DEBOUNCE).await;

            let snapshot = {
                let docs = documents.read().await;
                docs.get(&uri_for_task)
                    .map(|doc| (doc.text.clone(), doc.version))
            };

            let Some((text, version)) = snapshot else {
                return;
            };

            let result = analyze(&text);
            let diagnostics = result
                .diagnostics
                .into_iter()
                .map(|d| to_lsp_diagnostic(&text, d))
                .collect::<Vec<_>>();
            let index = result.index;

            {
                let mut docs = documents.write().await;
                if let Some(doc) = docs.get_mut(&uri_for_task) {
                    // Drop stale results if the buffer changed again while we analyzed.
                    if doc.version != version || doc.text != text {
                        return;
                    }
                    doc.index = index;
                } else {
                    return;
                }
            }

            client
                .publish_diagnostics(uri_for_task.clone(), diagnostics, Some(version))
                .await;

            let mut tasks = debounce_tasks.write().await;
            if let Some(current) = tasks.get(&uri_for_task) {
                if current.is_finished() {
                    tasks.remove(&uri_for_task);
                }
            }
        });

        self.debounce_tasks.write().await.insert(uri, handle);
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

    /// Ensure the symbol index matches the current buffer before navigation requests.
    /// Debounced diagnostics may lag behind the latest edit.
    async fn ensure_index_current(&self, uri: &Url) {
        let snapshot = {
            let docs = self.documents.read().await;
            docs.get(uri)
                .map(|doc| (doc.text.clone(), doc.version))
        };
        let Some((text, version)) = snapshot else {
            return;
        };

        let result = analyze(&text);
        let mut docs = self.documents.write().await;
        if let Some(doc) = docs.get_mut(uri) {
            if doc.version == version && doc.text == text {
                doc.index = result.index;
            }
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
                document_symbol_provider: Some(OneOf::Left(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
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
        let mut tasks = self.debounce_tasks.write().await;
        for (_, handle) in tasks.drain() {
            handle.abort();
        }
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
                    index: Index::default(),
                },
            );
        }

        self.publish_diagnostics(uri, &text, Some(version)).await;
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

        self.schedule_diagnostics(uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.cancel_debounce(&uri).await;
        self.documents.write().await.remove(&uri);
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        self.ensure_index_current(&params.text_document.uri).await;
        let docs = self.documents.read().await;
        let Some(doc) = docs.get(&params.text_document.uri) else {
            return Ok(None);
        };

        let symbols = to_document_symbols(&doc.text, &doc.index.document_symbols);
        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        self.ensure_index_current(uri).await;
        let docs = self.documents.read().await;
        let Some(doc) = docs.get(uri) else {
            return Ok(None);
        };

        let offset = position_to_byte_offset(
            &doc.text,
            params.text_document_position_params.position,
        );
        let location = definition_location(uri, &doc.text, &doc.index, offset);

        Ok(location.map(GotoDefinitionResponse::Scalar))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = &params.text_document_position.text_document.uri;
        self.ensure_index_current(uri).await;
        let docs = self.documents.read().await;
        let Some(doc) = docs.get(uri) else {
            return Ok(None);
        };

        let offset = position_to_byte_offset(&doc.text, params.text_document_position.position);
        let locations = reference_locations(
            uri,
            &doc.text,
            &doc.index,
            offset,
            params.context.include_declaration,
        );

        Ok(Some(locations))
    }
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
