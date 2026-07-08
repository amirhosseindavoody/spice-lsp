use tree_sitter::Node;

use crate::diagnostic::{Diagnostic, Span};

/// Collect `.subckt` / `.ends` directive spans for structural checks.
pub fn dot_directive_spans(source: &str, root: Node<'_>) -> Vec<(String, Span)> {
    let mut out = Vec::new();
    collect_dot_directives(source, root, &mut out);
    out
}

fn collect_dot_directives(source: &str, node: Node<'_>, out: &mut Vec<(String, Span)>) {
    if node.kind() == "dot_directive_line" {
        let text = node.utf8_text(source.as_bytes()).unwrap_or("");
        let name = text
            .trim_start()
            .trim_start_matches('.')
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if !name.is_empty() {
            out.push((
                name,
                Span {
                    start: node.start_byte(),
                    end: node.end_byte(),
                },
            ));
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_dot_directives(source, child, out);
    }
}

/// Flag unclosed `.subckt` blocks and unmatched `.ends`.
pub fn subckt_diagnostics(directives: &[(String, Span)]) -> Vec<Diagnostic> {
    let mut stack: Vec<(String, Span)> = Vec::new();
    let mut diagnostics = Vec::new();

    for (name, span) in directives {
        match name.as_str() {
            "subckt" => stack.push(("subckt".to_string(), *span)),
            "ends" if stack.pop().is_none() => {
                diagnostics.push(Diagnostic::error("unexpected .ends without .subckt", *span));
            }
            _ => {}
        }
    }

    for (_, span) in stack {
        diagnostics.push(Diagnostic::error("missing .ends for subcircuit", span));
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_unmatched_ends() {
        let span = Span { start: 0, end: 5 };
        let diags = subckt_diagnostics(&[("ends".into(), span)]);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("unexpected .ends"));
    }
}
