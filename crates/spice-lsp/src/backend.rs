use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use spice_parser::{
    analyze_with_includes, disk_loader_with_overrides, hover_token_at, Dialect, IncludeResolution,
    Index, ResolveOptions, DEFAULT_MAX_INCLUDE_DEPTH,
};
use spice_reference::ReferenceIndex;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use url::Url;

use crate::convert::{
    definition_location_with_includes, position_to_byte_offset, reference_locations, span_to_range,
    to_document_symbols, to_lsp_diagnostic,
};

/// Coalesce rapid `didChange` events before re-analyzing and publishing diagnostics.
const DIAGNOSTIC_DEBOUNCE: Duration = Duration::from_millis(150);

#[derive(Debug, Clone)]
struct Document {
    text: String,
    version: i32,
    index: Index,
    includes: IncludeResolution,
}

#[derive(Debug, Clone)]
struct WorkspaceConfig {
    library_paths: Vec<PathBuf>,
    max_include_depth: usize,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            library_paths: Vec::new(),
            max_include_depth: DEFAULT_MAX_INCLUDE_DEPTH,
        }
    }
}

pub struct Backend {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, Document>>>,
    debounce_tasks: Arc<RwLock<HashMap<Url, JoinHandle<()>>>>,
    dialect: Arc<RwLock<Dialect>>,
    config: Arc<RwLock<WorkspaceConfig>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
            debounce_tasks: Arc::new(RwLock::new(HashMap::new())),
            dialect: Arc::new(RwLock::new(Dialect::default())),
            config: Arc::new(RwLock::new(WorkspaceConfig::default())),
        }
    }

    async fn current_dialect(&self) -> Dialect {
        *self.dialect.read().await
    }

    async fn current_config(&self) -> WorkspaceConfig {
        self.config.read().await.clone()
    }

    fn dialect_from_init(params: &InitializeParams) -> Dialect {
        if let Some(Value::Object(map)) = &params.initialization_options {
            if let Some(Value::String(d)) = map.get("dialect") {
                let (dialect, _) = Dialect::parse_or_default(d);
                return dialect;
            }
        }
        Dialect::default()
    }

    fn config_from_settings(settings: &Value) -> Option<(Option<Dialect>, WorkspaceConfig)> {
        let spice = settings.get("spiceLsp")?;
        let dialect = spice
            .get("dialect")
            .and_then(Value::as_str)
            .map(|raw| Dialect::parse_or_default(raw).0);

        let mut config = WorkspaceConfig::default();
        if let Some(paths) = spice.get("libraryPaths").and_then(Value::as_array) {
            config.library_paths = paths
                .iter()
                .filter_map(|v| v.as_str())
                .map(PathBuf::from)
                .collect();
        }
        if let Some(depth) = spice
            .get("include")
            .and_then(|i| i.get("maxDepth"))
            .and_then(Value::as_u64)
        {
            config.max_include_depth = depth as usize;
        }

        Some((dialect, config))
    }

    async fn set_dialect(&self, dialect: Dialect) {
        let mut slot = self.dialect.write().await;
        if *slot != dialect {
            *slot = dialect;
        }
    }

    async fn set_config(&self, config: WorkspaceConfig) {
        *self.config.write().await = config;
    }

    fn uri_to_path(uri: &Url) -> Option<PathBuf> {
        uri.to_file_path().ok()
    }

    fn base_dir_for(uri: &Url) -> PathBuf {
        Self::uri_to_path(uri)
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."))
    }

    async fn open_buffer_overrides(&self) -> HashMap<PathBuf, String> {
        let docs = self.documents.read().await;
        let mut map = HashMap::new();
        for (uri, doc) in docs.iter() {
            if let Some(path) = Self::uri_to_path(uri) {
                map.insert(path.clone(), doc.text.clone());
                if let Ok(canon) = path.canonicalize() {
                    map.insert(canon, doc.text.clone());
                }
            }
        }
        map
    }

    async fn analyze_document(
        &self,
        uri: &Url,
        text: &str,
    ) -> (Vec<Diagnostic>, Index, IncludeResolution) {
        let dialect = self.current_dialect().await;
        let config = self.current_config().await;
        let overrides = self.open_buffer_overrides().await;
        let options = ResolveOptions {
            base_dir: Self::base_dir_for(uri),
            library_paths: config.library_paths,
            max_depth: config.max_include_depth,
            dialect,
        };
        let loader = disk_loader_with_overrides(overrides);
        let (result, resolution) = analyze_with_includes(text, &options, &loader);
        let diagnostics = result
            .diagnostics
            .into_iter()
            .map(|d| to_lsp_diagnostic(text, d))
            .collect::<Vec<_>>();
        (diagnostics, result.index, resolution)
    }

    async fn analyze_and_store(&self, uri: &Url, text: &str) -> (Vec<Diagnostic>, Index) {
        let (diagnostics, index, resolution) = self.analyze_document(uri, text).await;

        if let Some(doc) = self.documents.write().await.get_mut(uri) {
            doc.index = index.clone();
            doc.includes = resolution;
        }

        (diagnostics, index)
    }

    async fn publish_diagnostics(&self, uri: Url, text: &str, version: Option<i32>) {
        let (diagnostics, _) = self.analyze_and_store(&uri, text).await;
        self.client
            .publish_diagnostics(uri, diagnostics, version)
            .await;
    }

    async fn reanalyze_all(&self) {
        let snapshot: Vec<(Url, String, i32)> = {
            let docs = self.documents.read().await;
            docs.iter()
                .map(|(uri, doc)| (uri.clone(), doc.text.clone(), doc.version))
                .collect()
        };
        for (uri, text, version) in snapshot {
            self.publish_diagnostics(uri, &text, Some(version)).await;
        }
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
        let dialect_slot = Arc::clone(&self.dialect);
        let config_slot = Arc::clone(&self.config);
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

            let dialect = *dialect_slot.read().await;
            let config = config_slot.read().await.clone();

            let overrides = {
                let docs = documents.read().await;
                let mut map = HashMap::new();
                for (u, doc) in docs.iter() {
                    if let Ok(path) = u.to_file_path() {
                        map.insert(path.clone(), doc.text.clone());
                        if let Ok(canon) = path.canonicalize() {
                            map.insert(canon, doc.text.clone());
                        }
                    }
                }
                map
            };

            let options = ResolveOptions {
                base_dir: uri_for_task
                    .to_file_path()
                    .ok()
                    .and_then(|p| p.parent().map(Path::to_path_buf))
                    .unwrap_or_else(|| PathBuf::from(".")),
                library_paths: config.library_paths,
                max_depth: config.max_include_depth,
                dialect,
            };
            let loader = disk_loader_with_overrides(overrides);
            let (result, resolution) = analyze_with_includes(&text, &options, &loader);
            let diagnostics = result
                .diagnostics
                .into_iter()
                .map(|d| to_lsp_diagnostic(&text, d))
                .collect::<Vec<_>>();
            let index = result.index;

            {
                let mut docs = documents.write().await;
                if let Some(doc) = docs.get_mut(&uri_for_task) {
                    if doc.version != version || doc.text != text {
                        return;
                    }
                    doc.index = index;
                    doc.includes = resolution;
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

    async fn ensure_index_current(&self, uri: &Url) {
        let snapshot = {
            let docs = self.documents.read().await;
            docs.get(uri)
                .map(|doc| (doc.text.clone(), doc.version))
        };
        let Some((text, version)) = snapshot else {
            return;
        };

        let (diagnostics, index, resolution) = self.analyze_document(uri, &text).await;
        let _ = diagnostics;
        let mut docs = self.documents.write().await;
        if let Some(doc) = docs.get_mut(uri) {
            if doc.version == version && doc.text == text {
                doc.index = index;
                doc.includes = resolution;
            }
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let dialect = Self::dialect_from_init(&params);
        self.set_dialect(dialect).await;

        if let Some(Value::Object(map)) = &params.initialization_options {
            let mut config = WorkspaceConfig::default();
            if let Some(Value::Array(paths)) = map.get("libraryPaths") {
                config.library_paths = paths
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(PathBuf::from)
                    .collect();
            }
            if let Some(depth) = map
                .get("include")
                .and_then(|i| i.get("maxDepth"))
                .and_then(Value::as_u64)
            {
                config.max_include_depth = depth as usize;
            }
            self.set_config(config).await;
        }

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
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "spice-lsp".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        let dialect = self.current_dialect().await;
        self.client
            .log_message(
                MessageType::INFO,
                format!("spice-lsp ready (dialect={})", dialect.id()),
            )
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        let mut tasks = self.debounce_tasks.write().await;
        for (_, handle) in tasks.drain() {
            handle.abort();
        }
        Ok(())
    }

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        let Some((dialect_opt, config)) = Self::config_from_settings(&params.settings) else {
            return;
        };

        let mut reanalyze = false;
        if let Some(dialect) = dialect_opt {
            let previous = self.current_dialect().await;
            if dialect != previous {
                self.set_dialect(dialect).await;
                self.client
                    .log_message(
                        MessageType::INFO,
                        format!("dialect changed: {} → {}", previous.id(), dialect.id()),
                    )
                    .await;
                reanalyze = true;
            }
        }

        {
            let previous = self.current_config().await;
            if previous.library_paths != config.library_paths
                || previous.max_include_depth != config.max_include_depth
            {
                self.set_config(config).await;
                reanalyze = true;
            }
        }

        if reanalyze {
            self.reanalyze_all().await;
        }
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
                    includes: IncludeResolution::default(),
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

    async fn did_change_watched_files(&self, _params: DidChangeWatchedFilesParams) {
        // Included / library files may have changed on disk — refresh all open buffers.
        self.reanalyze_all().await;
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
        let location = definition_location_with_includes(
            uri,
            &doc.text,
            &doc.index,
            offset,
            Some(&doc.includes),
        );

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

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let docs = self.documents.read().await;
        let Some(doc) = docs.get(uri) else {
            return Ok(None);
        };

        let offset =
            position_to_byte_offset(&doc.text, params.text_document_position_params.position);
        let Some(token) = hover_token_at(&doc.text, offset) else {
            return Ok(None);
        };

        let dialect = self.current_dialect().await;
        // 1) dialect reference corpus
        if let Some(entry) = ReferenceIndex::global().lookup_token(dialect, &token) {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: entry.render_markdown(dialect),
                }),
                range: Some(span_to_range(&doc.text, token.span)),
            }));
        }

        // 2) file-local symbol detail
        if let Some(symbol) = doc.index.symbol_at_offset(offset) {
            if let Some(local) = file_local_hover(&doc.text, symbol) {
                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: local,
                    }),
                    range: Some(span_to_range(&doc.text, token.span)),
                }));
            }
        }

        // 3) included / library definition detail
        if let Some(symbol) = doc.index.symbol_at_offset(offset) {
            if let Some((file, kind, span)) = doc.includes.find_model_or_subckt(&symbol.name) {
                if let Some(external) = included_hover(&file.text, kind, &symbol.name, span) {
                    return Ok(Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: external,
                        }),
                        range: Some(span_to_range(&doc.text, token.span)),
                    }));
                }
            }
        }

        Ok(None)
    }
}

fn file_local_hover(source: &str, symbol: &spice_parser::Symbol) -> Option<String> {
    use spice_parser::SymbolKind;
    match symbol.kind {
        SymbolKind::Subckt => {
            let line = line_containing(source, symbol.line_span.start)?;
            let ports = subckt_ports(line);
            Some(format!(
                "**`.subckt {}`** (file-local)\n\nPorts: `{}`",
                symbol.name,
                ports.join(", ")
            ))
        }
        SymbolKind::Model | SymbolKind::Param => {
            let line = line_containing(source, symbol.line_span.start)?;
            Some(format!(
                "**{} `{}`** (file-local)\n\n```\n{}\n```",
                match symbol.kind {
                    SymbolKind::Model => ".model",
                    SymbolKind::Param => ".param",
                    _ => "symbol",
                },
                symbol.name,
                line.trim()
            ))
        }
        SymbolKind::Instance => None,
    }
}

fn included_hover(
    source: &str,
    kind: spice_parser::SymbolKind,
    name: &str,
    span: spice_parser::Span,
) -> Option<String> {
    use spice_parser::SymbolKind;
    let line = line_containing(source, span.start)?;
    match kind {
        SymbolKind::Subckt => {
            let ports = subckt_ports(line);
            Some(format!(
                "**`.subckt {}`** (included)\n\nPorts: `{}`",
                name,
                ports.join(", ")
            ))
        }
        SymbolKind::Model => Some(format!(
            "**`.model {}`** (included)\n\n```\n{}\n```",
            name,
            line.trim()
        )),
        _ => None,
    }
}

fn line_containing(source: &str, offset: usize) -> Option<&str> {
    let offset = offset.min(source.len());
    let start = source[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let end = source[offset..]
        .find('\n')
        .map(|i| offset + i)
        .unwrap_or(source.len());
    Some(&source[start..end])
}

fn subckt_ports(line: &str) -> Vec<String> {
    let trimmed = line.trim_start().trim_start_matches('.');
    let mut parts = trimmed.split_whitespace();
    let _ = parts.next(); // subckt
    let _ = parts.next(); // name
    parts
        .take_while(|p| !p.contains('='))
        .map(|p| p.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use spice_parser::analyze_with_dialect;
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
        let result = analyze_with_dialect(&source, Dialect::Ngspice);
        assert!(!result.diagnostics.is_empty());
    }

    #[test]
    fn ngspice_and_hspice_share_diagnostics_for_unclosed() {
        let source = fixture("invalid/unclosed-subckt.cir");
        let ng = analyze_with_dialect(&source, Dialect::Ngspice);
        let hs = analyze_with_dialect(&source, Dialect::Hspice);
        assert_eq!(ng.diagnostics.len(), hs.diagnostics.len());
    }

    #[test]
    fn include_fixture_has_no_unknown_model() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../test-data/valid/with-include");
        let source = std::fs::read_to_string(dir.join("top.cir")).unwrap();
        let options = ResolveOptions {
            base_dir: dir,
            library_paths: Vec::new(),
            max_depth: 8,
            dialect: Dialect::Hspice,
        };
        let loader = disk_loader_with_overrides(HashMap::new());
        let (result, _) = analyze_with_includes(&source, &options, &loader);
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|d| d.code.as_deref() == Some("spice/unknown-model")),
            "{:?}",
            result.diagnostics
        );
    }
}
