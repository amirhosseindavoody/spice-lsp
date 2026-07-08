use spice_parser::Diagnostic as SpiceDiagnostic;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

/// Map a byte span to an LSP range (UTF-16 code units).
pub fn span_to_range(source: &str, span: spice_parser::Span) -> Range {
    let start = byte_offset_to_position(source, span.start);
    let end = byte_offset_to_position(source, span.end);
    Range { start, end }
}

pub fn to_lsp_diagnostic(source: &str, diag: SpiceDiagnostic) -> Diagnostic {
    Diagnostic {
        range: span_to_range(source, diag.span),
        severity: Some(match diag.severity {
            spice_parser::Severity::Error => DiagnosticSeverity::ERROR,
            spice_parser::Severity::Warning => DiagnosticSeverity::WARNING,
            spice_parser::Severity::Info => DiagnosticSeverity::INFORMATION,
        }),
        message: diag.message,
        ..Default::default()
    }
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
        let range = span_to_range(source, spice_parser::Span { start: 0, end: 2 });
        assert_eq!(range.start, Position::new(0, 0));
        assert_eq!(range.end, Position::new(0, 2));
    }
}
