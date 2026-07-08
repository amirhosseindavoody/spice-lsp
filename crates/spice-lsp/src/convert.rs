use spice_parser::{DocumentSymbolEntry, Index, Span, SymbolKind};
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

pub fn definition_location(uri: &Url, source: &str, index: &Index, offset: usize) -> Option<Location> {
    let symbol = index.symbol_at_offset(offset)?;
    let span = resolve_definition_span(index, symbol.kind, &symbol.name)?;
    Some(location(uri, source, span))
}

pub fn reference_locations(uri: &Url, source: &str, index: &Index, offset: usize) -> Vec<Location> {
    let Some(symbol) = index.symbol_at_offset(offset) else {
        return Vec::new();
    };

    let kinds = reference_kinds(symbol.kind);
    let mut spans = Vec::new();
    for kind in kinds {
        if let Some(def) = index.definition_span(*kind, &symbol.name) {
            spans.push(def);
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

fn resolve_definition_span(index: &Index, kind: SymbolKind, name: &str) -> Option<Span> {
    index
        .definition_span(kind, name)
        .or_else(|| {
            if kind == SymbolKind::Model {
                index.definition_span(SymbolKind::Subckt, name)
            } else {
                None
            }
        })
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

    #[test]
    fn ascii_byte_offset_maps_to_position() {
        let source = "R1 a b 1k\nC1 c 0 1u";
        let range = span_to_range(source, Span { start: 0, end: 2 });
        assert_eq!(range.start, Position::new(0, 0));
        assert_eq!(range.end, Position::new(0, 2));
    }
}
