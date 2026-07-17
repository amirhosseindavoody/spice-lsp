use spice_parser::{DocumentSymbolEntry, IncludeResolution, Index, Span, SymbolKind};
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticSeverity, DocumentSymbol, Location, NumberOrString, Position, Range,
    SymbolKind as LspSymbolKind, Url,
};

/// Map a byte span to an LSP range (UTF-16 code units).
pub fn span_to_range(source: &str, span: Span) -> Range {
    let start = byte_offset_to_position(source, span.start);
    let end = byte_offset_to_position(source, span.end);
    Range { start, end }
}

pub fn to_lsp_diagnostic(source: &str, diag: spice_parser::Diagnostic) -> Diagnostic {
    Diagnostic {
        range: span_to_range(source, diag.span),
        severity: Some(match diag.severity {
            spice_parser::Severity::Error => DiagnosticSeverity::ERROR,
            spice_parser::Severity::Warning => DiagnosticSeverity::WARNING,
            spice_parser::Severity::Info => DiagnosticSeverity::INFORMATION,
        }),
        code: diag.code.map(NumberOrString::String),
        message: diag.message,
        ..Default::default()
    }
}

pub fn to_document_symbols(source: &str, entries: &[DocumentSymbolEntry]) -> Vec<DocumentSymbol> {
    entries
        .iter()
        .map(|entry| to_document_symbol(source, entry))
        .collect()
}

fn to_document_symbol(source: &str, entry: &DocumentSymbolEntry) -> DocumentSymbol {
    let children = if entry.children.is_empty() {
        None
    } else {
        Some(
            entry
                .children
                .iter()
                .map(|child| to_document_symbol(source, child))
                .collect(),
        )
    };

    #[allow(deprecated)]
    DocumentSymbol {
        name: entry.name.clone(),
        detail: None,
        kind: to_lsp_symbol_kind(entry.kind),
        tags: None,
        deprecated: None,
        range: span_to_range(source, entry.line_span),
        selection_range: span_to_range(source, entry.name_span),
        children,
    }
}

fn to_lsp_symbol_kind(kind: SymbolKind) -> LspSymbolKind {
    match kind {
        SymbolKind::Subckt => LspSymbolKind::NAMESPACE,
        SymbolKind::Model => LspSymbolKind::CLASS,
        SymbolKind::Param => LspSymbolKind::VARIABLE,
        SymbolKind::Instance => LspSymbolKind::FIELD,
    }
}

pub fn location(uri: &Url, source: &str, span: Span) -> Location {
    Location {
        uri: uri.clone(),
        range: span_to_range(source, span),
    }
}

pub fn definition_location_with_includes(
    uri: &Url,
    source: &str,
    index: &Index,
    offset: usize,
    resolution: Option<&IncludeResolution>,
) -> Option<Location> {
    // Prefer include/lib path or entry under the cursor (not indexed as symbols).
    if let Some(resolution) = resolution {
        if let Some((file, span)) = resolution.definition_at_include_offset(offset) {
            let file_uri = path_to_url(&file.path)?;
            return Some(location(&file_uri, &file.text, span));
        }
    }

    let symbol = index.symbol_at_offset(offset)?;

    if let Some(span) = local_definition_span(index, symbol.kind, &symbol.name) {
        return Some(location(uri, source, span));
    }

    if let Some(resolution) = resolution {
        if matches!(symbol.kind, SymbolKind::Model | SymbolKind::Subckt) {
            if let Some((file, _, span)) = resolution.find_model_or_subckt(&symbol.name) {
                let file_uri = path_to_url(&file.path)?;
                return Some(location(&file_uri, &file.text, span));
            }
        }
    }

    // Fallback: previous same-file behavior (reference span).
    let span = resolve_definition_span(index, symbol.kind, &symbol.name)?;
    Some(location(uri, source, span))
}

fn path_to_url(path: &std::path::Path) -> Option<Url> {
    Url::from_file_path(path).ok()
}

pub fn reference_locations(
    uri: &Url,
    source: &str,
    index: &Index,
    offset: usize,
    include_declaration: bool,
) -> Vec<Location> {
    let Some(symbol) = index.symbol_at_offset(offset) else {
        return Vec::new();
    };

    let kinds = reference_kinds(symbol.kind);
    let mut spans = Vec::new();
    for kind in kinds {
        if include_declaration {
            if let Some(def) = index.definition_span(*kind, &symbol.name) {
                spans.push(def);
            }
        }
        spans.extend_from_slice(index.reference_spans(*kind, &symbol.name));
    }

    spans.sort_by_key(|s| (s.start, s.end));
    spans.dedup();

    spans
        .into_iter()
        .map(|span| location(uri, source, span))
        .collect()
}

fn reference_kinds(kind: SymbolKind) -> &'static [SymbolKind] {
    match kind {
        SymbolKind::Subckt => &[SymbolKind::Subckt],
        SymbolKind::Model => &[SymbolKind::Model, SymbolKind::Subckt],
        SymbolKind::Param => &[SymbolKind::Param],
        SymbolKind::Instance => &[SymbolKind::Instance],
    }
}

fn local_definition_span(index: &Index, kind: SymbolKind, name: &str) -> Option<Span> {
    index.definition_span(kind, name).or_else(|| {
        if kind == SymbolKind::Model {
            index.definition_span(SymbolKind::Subckt, name)
        } else {
            None
        }
    })
}

fn resolve_definition_span(index: &Index, kind: SymbolKind, name: &str) -> Option<Span> {
    local_definition_span(index, kind, name)
        .or_else(|| index.reference_spans(kind, name).first().copied())
}

pub fn position_to_byte_offset(source: &str, position: Position) -> usize {
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

fn byte_offset_to_position(source: &str, offset: usize) -> Position {
    let offset = offset.min(source.len());
    let prefix = &source[..offset];
    let line = prefix.matches('\n').count() as u32;
    let line_start = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let character = utf16_len(&source[line_start..offset]);
    Position { line, character }
}

fn utf16_len(text: &str) -> u32 {
    text.chars().map(|c| c.len_utf16()).sum::<usize>() as u32
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
    fn ascii_byte_offset_maps_to_position() {
        let source = "R1 a b 1k\nC1 c 0 1u";
        let range = span_to_range(source, Span { start: 0, end: 2 });
        assert_eq!(range.start, Position::new(0, 0));
        assert_eq!(range.end, Position::new(0, 2));
    }

    #[test]
    fn references_include_declaration_when_requested() {
        let source = fixture("valid/subckt.cir");
        let result = spice_parser::analyze(&source);
        let def = result
            .index
            .definition_span(SymbolKind::Subckt, "buffer")
            .expect("buffer definition");
        let uri = Url::parse("file:///test/subckt.cir").unwrap();

        let with_decl = reference_locations(&uri, &source, &result.index, def.start, true);
        let without_decl = reference_locations(&uri, &source, &result.index, def.start, false);

        assert!(with_decl.len() >= 2);
        assert_eq!(without_decl.len(), with_decl.len() - 1);
        assert!(without_decl.iter().all(|loc| loc.range.start.line != 1));
    }

    #[test]
    fn definition_jumps_into_include_file() {
        let dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-data/valid/with-include");
        let source = std::fs::read_to_string(dir.join("top.cir")).unwrap();
        let options = spice_parser::ResolveOptions {
            base_dir: dir.clone(),
            library_paths: Vec::new(),
            max_depth: 8,
            dialect: spice_parser::Dialect::Hspice,
        };
        let loader = spice_parser::disk_loader_with_overrides(Default::default());
        let (result, resolution) = spice_parser::analyze_with_includes(&source, &options, &loader);

        let uri = Url::from_file_path(dir.join("top.cir")).unwrap();
        // Cursor on `nch` in `M1 d g s b nch`
        let offset = source.find("nch").expect("nch ref");
        let loc = definition_location_with_includes(
            &uri,
            &source,
            &result.index,
            offset,
            Some(&resolution),
        )
        .expect("definition");

        assert!(loc.uri.path().ends_with("models.inc"));
    }

    #[test]
    fn definition_jumps_to_lib_path_and_entry() {
        let dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-data/valid/with-include");
        let source = std::fs::read_to_string(dir.join("top-lib.cir")).unwrap();
        let options = spice_parser::ResolveOptions {
            base_dir: dir.clone(),
            library_paths: Vec::new(),
            max_depth: 8,
            dialect: spice_parser::Dialect::Hspice,
        };
        let loader = spice_parser::disk_loader_with_overrides(Default::default());
        let (result, resolution) = spice_parser::analyze_with_includes(&source, &options, &loader);

        let uri = Url::from_file_path(dir.join("top-lib.cir")).unwrap();

        let path_offset = source.find("corners.lib").expect("lib path");
        let path_loc = definition_location_with_includes(
            &uri,
            &source,
            &result.index,
            path_offset,
            Some(&resolution),
        )
        .expect("path definition");
        assert!(
            path_loc.uri.path().ends_with("corners.lib"),
            "got {}",
            path_loc.uri
        );
        assert_eq!(path_loc.range.start.line, 0);
        assert_eq!(path_loc.range.start.character, 0);

        let entry_offset = source.find(" TT").expect("entry") + 1; // on 'T'
        let entry_loc = definition_location_with_includes(
            &uri,
            &source,
            &result.index,
            entry_offset,
            Some(&resolution),
        )
        .expect("entry definition");
        assert!(
            entry_loc.uri.path().ends_with("corners.lib"),
            "got {}",
            entry_loc.uri
        );
        // `.lib TT` is the second line of corners.lib (0-based line 1)
        assert_eq!(entry_loc.range.start.line, 1);
    }

    #[test]
    fn definition_jumps_to_include_path() {
        let dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-data/valid/with-include");
        let source = std::fs::read_to_string(dir.join("top.cir")).unwrap();
        let options = spice_parser::ResolveOptions {
            base_dir: dir.clone(),
            library_paths: Vec::new(),
            max_depth: 8,
            dialect: spice_parser::Dialect::Hspice,
        };
        let loader = spice_parser::disk_loader_with_overrides(Default::default());
        let (result, resolution) = spice_parser::analyze_with_includes(&source, &options, &loader);

        let uri = Url::from_file_path(dir.join("top.cir")).unwrap();
        let offset = source.find("models.inc").expect("include path");
        let loc = definition_location_with_includes(
            &uri,
            &source,
            &result.index,
            offset,
            Some(&resolution),
        )
        .expect("include path definition");
        assert!(loc.uri.path().ends_with("models.inc"));
        assert_eq!(loc.range.start.line, 0);
    }
}
