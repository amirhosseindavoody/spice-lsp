use tree_sitter::{Node, Parser, Tree};

use crate::diagnostic::{Diagnostic, Severity, Span};
use crate::subckt::{dot_directive_spans, subckt_diagnostics};

/// Result of analyzing a netlist buffer.
#[derive(Debug, Clone)]
pub struct ParseResult {
    pub tree: Tree,
    pub diagnostics: Vec<Diagnostic>,
}

/// Parse `source` and return syntax / structural diagnostics.
pub fn analyze(source: &str) -> ParseResult {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_spice::language())
        .expect("tree-sitter-spice language");

    let tree = parser.parse(source, None).expect("parse succeeds");
    let root = tree.root_node();

    let mut diagnostics = tree_diagnostics(source, root);
    diagnostics.extend(subckt_diagnostics(&dot_directive_spans(source, root)));
    diagnostics.sort_by_key(|d| (d.span.start, d.span.end));
    diagnostics.dedup_by(|a, b| a.span == b.span && a.message == b.message);

    ParseResult { tree, diagnostics }
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
}
