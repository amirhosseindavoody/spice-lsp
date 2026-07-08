use std::collections::HashMap;

use crate::diagnostic::{Diagnostic, Severity, Span};

/// Kind of indexed SPICE symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Subckt,
    Model,
    Param,
    Instance,
}

/// A definition or reference occurrence in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub kind: SymbolKind,
    pub name: String,
    /// Span covering the symbol name (for navigation/hover).
    pub name_span: Span,
    /// Full line span.
    pub line_span: Span,
    /// `.subckt` name containing this symbol, empty at top level.
    pub scope: String,
}

/// Hierarchical entry for document symbols.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSymbolEntry {
    pub name: String,
    pub kind: SymbolKind,
    pub name_span: Span,
    pub line_span: Span,
    pub children: Vec<DocumentSymbolEntry>,
}

/// Index built from a parsed netlist.
#[derive(Debug, Clone, Default)]
pub struct Index {
    pub symbols: Vec<Symbol>,
    pub document_symbols: Vec<DocumentSymbolEntry>,
    definitions: HashMap<(SymbolKind, String), Vec<Span>>,
    references: HashMap<(SymbolKind, String), Vec<Span>>,
}

impl Index {
    pub fn definition_span(&self, kind: SymbolKind, name: &str) -> Option<Span> {
        let key = (kind, name.to_ascii_lowercase());
        self.definitions.get(&key).and_then(|v| v.first().copied())
    }

    pub fn reference_spans(&self, kind: SymbolKind, name: &str) -> &[Span] {
        static EMPTY: [Span; 0] = [];
        let key = (kind, name.to_ascii_lowercase());
        self.references
            .get(&key)
            .map(|v| v.as_slice())
            .unwrap_or(&EMPTY)
    }

    pub fn symbol_at_offset(&self, offset: usize) -> Option<&Symbol> {
        self.symbols.iter().find(|s| {
            offset >= s.name_span.start && offset <= s.name_span.end
        })
    }

    pub(crate) fn add_definition(&mut self, kind: SymbolKind, name: String, span: Span) {
        self.definitions
            .entry((kind, name.to_ascii_lowercase()))
            .or_default()
            .push(span);
    }

    pub(crate) fn add_reference(&mut self, kind: SymbolKind, name: String, span: Span) {
        self.references
            .entry((kind, name.to_ascii_lowercase()))
            .or_default()
            .push(span);
    }
}

/// Build symbol index and semantic diagnostics from line-oriented CST nodes.
pub fn build_index(_source: &str, lines: &[(Span, LineKind)]) -> (Index, Vec<Diagnostic>) {
    let mut index = Index::default();
    let mut diagnostics = Vec::new();
    let mut scope_stack: Vec<(String, Span)> = Vec::new();
    let mut instance_counts: HashMap<(String, String), (String, Vec<Span>)> = HashMap::new();

    let mut current_subckt: Option<DocumentSymbolEntry> = None;

    for (line_span, line) in lines {
        match line {
            LineKind::Subckt { name, name_span } => {
                let scope = current_scope(&scope_stack);
                let sym = Symbol {
                    kind: SymbolKind::Subckt,
                    name: name.clone(),
                    name_span: *name_span,
                    line_span: *line_span,
                    scope: scope.clone(),
                };
                index.add_definition(SymbolKind::Subckt, name.clone(), *name_span);
                index.symbols.push(sym);

                scope_stack.push((name.clone(), *line_span));
                current_subckt = Some(DocumentSymbolEntry {
                    name: name.clone(),
                    kind: SymbolKind::Subckt,
                    name_span: *name_span,
                    line_span: *line_span,
                    children: Vec::new(),
                });
            }
            LineKind::Ends { name: _ } => {
                if scope_stack.pop().is_none() {
                    diagnostics.push(Diagnostic {
                        message: "unexpected .ends without .subckt".into(),
                        severity: Severity::Error,
                        span: *line_span,
                        code: None,
                    });
                } else if let Some(subckt) = current_subckt.take() {
                    index.document_symbols.push(subckt);
                }
            }
            LineKind::Model { name, name_span } => {
                let scope = current_scope(&scope_stack);
                index.add_definition(SymbolKind::Model, name.clone(), *name_span);
                index.symbols.push(Symbol {
                    kind: SymbolKind::Model,
                    name: name.clone(),
                    name_span: *name_span,
                    line_span: *line_span,
                    scope,
                });
                push_outline_child(
                    &mut current_subckt,
                    &mut index.document_symbols,
                    DocumentSymbolEntry {
                        name: name.clone(),
                        kind: SymbolKind::Model,
                        name_span: *name_span,
                        line_span: *line_span,
                        children: Vec::new(),
                    },
                );
            }
            LineKind::Param { name, name_span } => {
                let scope = current_scope(&scope_stack);
                index.add_definition(SymbolKind::Param, name.clone(), *name_span);
                index.symbols.push(Symbol {
                    kind: SymbolKind::Param,
                    name: name.clone(),
                    name_span: *name_span,
                    line_span: *line_span,
                    scope,
                });
                push_outline_child(
                    &mut current_subckt,
                    &mut index.document_symbols,
                    DocumentSymbolEntry {
                        name: name.clone(),
                        kind: SymbolKind::Param,
                        name_span: *name_span,
                        line_span: *line_span,
                        children: Vec::new(),
                    },
                );
            }
            LineKind::Instance {
                name,
                name_span,
                model_ref,
            } => {
                let scope = current_scope(&scope_stack);
                let key = (scope.clone(), name.to_ascii_lowercase());
                instance_counts
                    .entry(key)
                    .or_insert_with(|| (name.clone(), Vec::new()))
                    .1
                    .push(*name_span);

                index.symbols.push(Symbol {
                    kind: SymbolKind::Instance,
                    name: name.clone(),
                    name_span: *name_span,
                    line_span: *line_span,
                    scope: scope.clone(),
                });

                push_outline_child(
                    &mut current_subckt,
                    &mut index.document_symbols,
                    DocumentSymbolEntry {
                        name: name.clone(),
                        kind: SymbolKind::Instance,
                        name_span: *name_span,
                        line_span: *line_span,
                        children: Vec::new(),
                    },
                );

                if let Some((model_name, model_span)) = model_ref {
                    index.add_reference(SymbolKind::Subckt, model_name.clone(), *model_span);
                    index.add_reference(SymbolKind::Model, model_name.clone(), *model_span);
                    index.symbols.push(Symbol {
                        kind: SymbolKind::Model,
                        name: model_name.clone(),
                        name_span: *model_span,
                        line_span: *line_span,
                        scope: scope.clone(),
                    });

                    if index.definition_span(SymbolKind::Model, model_name).is_none()
                        && index.definition_span(SymbolKind::Subckt, model_name).is_none()
                    {
                        diagnostics.push(Diagnostic {
                            message: format!("'{model_name}' is not defined as a model or subcircuit"),
                            severity: Severity::Warning,
                            span: *model_span,
                            code: Some("spice/unknown-model".into()),
                        });
                    }
                }
            }
            LineKind::Other => {}
        }
    }

    if let Some(subckt) = current_subckt.take() {
        index.document_symbols.push(subckt);
    }

    if !scope_stack.is_empty() {
        for (_, span) in scope_stack {
            diagnostics.push(Diagnostic {
                message: "missing .ends for subcircuit".into(),
                severity: Severity::Error,
                span,
                code: None,
            });
        }
    }

    for ((scope, _name), (display_name, spans)) in instance_counts {
        if spans.len() > 1 {
            for span in spans {
                diagnostics.push(Diagnostic {
                    message: format!("duplicate component name '{display_name}'"),
                    severity: Severity::Warning,
                    span,
                    code: Some("spice/duplicate-name".into()),
                });
            }
            let _ = scope;
        }
    }

    (index, diagnostics)
}

fn current_scope(scope_stack: &[(String, Span)]) -> String {
    scope_stack
        .last()
        .map(|(name, _)| name.clone())
        .unwrap_or_default()
}

fn push_outline_child(
    current_subckt: &mut Option<DocumentSymbolEntry>,
    top_level: &mut Vec<DocumentSymbolEntry>,
    child: DocumentSymbolEntry,
) {
    if let Some(subckt) = current_subckt {
        subckt.children.push(child);
    } else {
        top_level.push(child);
    }
}

/// Parsed classification of a source line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineKind {
    Subckt { name: String, name_span: Span },
    Ends { name: Option<String> },
    Model { name: String, name_span: Span },
    Param { name: String, name_span: Span },
    Instance {
        name: String,
        name_span: Span,
        model_ref: Option<(String, Span)>,
    },
    Other,
}

/// Classify a line node and extract symbol spans relative to `source`.
pub fn classify_line(source: &str, line_span: Span) -> LineKind {
    let text = &source[line_span.start..line_span.end.min(source.len())];
    let trimmed = text.trim_start();

    if trimmed.starts_with('.') {
        return classify_directive(source, line_span, trimmed);
    }

    classify_instance(source, line_span, trimmed)
}

fn classify_directive(source: &str, line_span: Span, text: &str) -> LineKind {
    let rest = text.trim_start_matches('.');
    let mut parts = rest.split_whitespace();
    let directive = parts.next().unwrap_or("").to_ascii_lowercase();

    match directive.as_str() {
        "subckt" => {
            let name = parts.next().unwrap_or("").to_string();
            name_line_kind(source, line_span, text, &name, |name, name_span| LineKind::Subckt {
                name,
                name_span,
            })
        }
        "ends" => {
            let name = parts.next().map(|s| s.to_string());
            LineKind::Ends { name }
        }
        "model" => {
            let name = parts.next().unwrap_or("").to_string();
            name_line_kind(source, line_span, text, &name, |name, name_span| LineKind::Model {
                name,
                name_span,
            })
        }
        "param" => {
            let raw = parts.next().unwrap_or("");
            let name = raw.split('=').next().unwrap_or(raw).to_string();
            name_line_kind(source, line_span, text, &name, |name, name_span| LineKind::Param {
                name,
                name_span,
            })
        }
        _ => LineKind::Other,
    }
}

fn classify_instance(source: &str, line_span: Span, text: &str) -> LineKind {
    let Some(first) = text.chars().next() else {
        return LineKind::Other;
    };
    if !first.is_ascii_alphabetic() {
        return LineKind::Other;
    }

    let name_end = first
        .len_utf8()
        + text[first.len_utf8()..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '$' | ':' | '#' | '[' | ']' | '<' | '>' | '-'))
            .map(|c| c.len_utf8())
            .sum::<usize>();

    let name = text[..name_end.min(text.len())].to_string();
    if name.len() <= 1 {
        return LineKind::Other;
    }

    let name_span = subspan(source, line_span, text, &name);

    let element = first.to_ascii_uppercase();
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let model_ref = match element {
        'X' if tokens.len() >= 2 => {
            let model_name = tokens.last().copied().unwrap_or("");
            if model_name.contains('=') {
                None
            } else {
                Some((model_name.to_string(), subspan(source, line_span, text, model_name)))
            }
        }
        'M' if tokens.len() >= 6 => Some((
            tokens[5].to_string(),
            subspan(source, line_span, text, tokens[5]),
        )),
        _ => None,
    };

    LineKind::Instance {
        name,
        name_span,
        model_ref,
    }
}

fn name_line_kind<F>(source: &str, line_span: Span, line_text: &str, name: &str, f: F) -> LineKind
where
    F: FnOnce(String, Span) -> LineKind,
{
    if name.is_empty() {
        return LineKind::Other;
    }
    f(
        name.to_string(),
        subspan(source, line_span, line_text, name),
    )
}

fn subspan(source: &str, line_span: Span, line_text: &str, needle: &str) -> Span {
    let line_bytes = &source[line_span.start..line_span.end.min(source.len())];
    let offset_in_line = line_text
        .find(needle)
        .or_else(|| line_bytes.find(needle))
        .unwrap_or(0);
    let start = line_span.start + offset_in_line;
    let end = start + needle.len();
    Span { start, end }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    fn lines(source: &str) -> Vec<(Span, LineKind)> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_spice::language())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let mut out = Vec::new();
        collect_lines(source, tree.root_node(), &mut out);
        out
    }

    fn collect_lines(source: &str, node: tree_sitter::Node<'_>, out: &mut Vec<(Span, LineKind)>) {
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
                    collect_lines(source, child, out);
                }
            }
        }
    }

    #[test]
    fn classifies_subckt_and_instance() {
        let source = ".subckt buf in out\nX1 a b buf\n.ends\n";
        let parsed = lines(source);
        assert!(matches!(
            parsed[0].1,
            LineKind::Subckt { ref name, .. } if name == "buf"
        ));
        assert!(matches!(
            parsed[1].1,
            LineKind::Instance { ref name, .. } if name == "X1"
        ));
    }
}
