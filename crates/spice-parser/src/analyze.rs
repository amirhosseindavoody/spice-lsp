use tree_sitter::{Node, Parser, Tree};

use crate::diagnostic::{Diagnostic, Severity, Span};
use crate::symbols::{build_index, classify_line, Index, LineKind};

/// Result of analyzing a netlist buffer.
#[derive(Debug, Clone)]
pub struct ParseResult {
    pub tree: Tree,
    pub diagnostics: Vec<Diagnostic>,
    pub index: Index,
}

/// Parse `source` and return syntax / structural / semantic diagnostics.
pub fn analyze(source: &str) -> ParseResult {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_spice::language())
        .expect("tree-sitter-spice language");

    let tree = parser.parse(source, None).expect("parse succeeds");
    let root = tree.root_node();

    let lines = collect_lines(source, root);
    let (index, semantic_diagnostics) = build_index(source, &lines);

    let mut diagnostics = tree_diagnostics(source, root);
    diagnostics.extend(semantic_diagnostics);
    diagnostics.sort_by_key(|d| (d.span.start, d.span.end));
    diagnostics.dedup_by(|a, b| a.span == b.span && a.message == b.message);

    ParseResult {
        tree,
        diagnostics,
        index,
    }
}

fn collect_lines(source: &str, root: Node<'_>) -> Vec<(Span, LineKind)> {
    let mut out = Vec::new();
    collect_line_nodes(source, root, &mut out);
    out
}

fn collect_line_nodes(source: &str, node: Node<'_>, out: &mut Vec<(Span, LineKind)>) {
    match node.kind() {
        "dot_directive_line" | "instance_line" => {
            let span = Span {
                start: node.start_byte(),
                end: node.end_byte(),
            };
            out.push((span, classify_line(source, span)));
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                collect_line_nodes(source, child, out);
            }
        }
    }
}

fn tree_diagnostics(source: &str, root: Node<'_>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    collect_node_diagnostics(source, root, &mut out);
    out
}

fn collect_node_diagnostics(source: &str, node: Node<'_>, out: &mut Vec<Diagnostic>) {
    if node.is_error() {
        push_span_diagnostic(source, node, "syntax error", Severity::Error, out);
    } else if node.is_missing() {
        push_span_diagnostic(source, node, "missing token", Severity::Error, out);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_node_diagnostics(source, child, out);
    }
}

fn push_span_diagnostic(
    source: &str,
    node: Node<'_>,
    message: &str,
    severity: Severity,
    out: &mut Vec<Diagnostic>,
) {
    let start = node.start_byte();
    let end = node
        .end_byte()
        .max(start.saturating_add(1))
        .min(source.len());
    out.push(Diagnostic {
        message: message.to_string(),
        severity,
        span: Span { start, end },
        code: None,
    });
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
    fn valid_simple_rc_has_no_diagnostics() {
        let source = fixture("valid/simple-rc.cir");
        let result = analyze(&source);
        assert!(
            result.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn unclosed_subckt_reports_error() {
        let source = fixture("invalid/unclosed-subckt.cir");
        let result = analyze(&source);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.message.contains("missing .ends")),
            "expected missing .ends diagnostic, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn duplicate_instance_reports_warning() {
        let source = fixture("invalid/duplicate-instance.cir");
        let result = analyze(&source);
        assert!(
            result.diagnostics.iter().any(|d| {
                d.code.as_deref() == Some("spice/duplicate-name")
                    && d.message.contains("R1")
            }),
            "expected duplicate R1 warning, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn unknown_subckt_reports_warning() {
        let source = fixture("invalid/unknown-subckt.cir");
        let result = analyze(&source);
        assert!(
            result.diagnostics.iter().any(|d| {
                d.code.as_deref() == Some("spice/unknown-model")
                    && d.message.contains("missingbuf")
            }),
            "expected unknown subcircuit warning, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn subckt_fixture_builds_outline() {
        let source = fixture("valid/subckt.cir");
        let result = analyze(&source);
        assert!(
            result
                .index
                .document_symbols
                .iter()
                .any(|s| s.name == "buffer"),
            "expected buffer subcircuit in outline, got {:?}",
            result.index.document_symbols
        );
    }
}
