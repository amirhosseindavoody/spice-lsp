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

    pub fn has_definition(&self, kind: SymbolKind, name: &str) -> bool {
        self.definition_span(kind, name).is_some()
    }

    /// Whether `name` is defined as a model or subcircuit (case-insensitive).
    pub fn has_model_or_subckt(&self, name: &str) -> bool {
        self.has_definition(SymbolKind::Model, name)
            || self.has_definition(SymbolKind::Subckt, name)
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
        self.symbols
            .iter()
            .find(|s| offset >= s.name_span.start && offset <= s.name_span.end)
    }

    /// All definition keys `(kind, lowercase name)` present in this index.
    pub fn definition_names(&self) -> impl Iterator<Item = (SymbolKind, &str)> + '_ {
        self.definitions
            .keys()
            .map(|(kind, name)| (*kind, name.as_str()))
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

    /// Record at most one reference span per `(kind, name)` (extracted-mode memory bound).
    pub(crate) fn add_reference_first(&mut self, kind: SymbolKind, name: String, span: Span) {
        let key = (kind, name.to_ascii_lowercase());
        self.references.entry(key).or_insert_with(|| vec![span]);
    }

    /// Unique model names referenced from instances, with a representative span.
    pub fn model_reference_sites(&self) -> impl Iterator<Item = (&str, Span)> + '_ {
        self.references.iter().filter_map(|((kind, name), spans)| {
            if *kind == SymbolKind::Model {
                spans.first().map(|span| (name.as_str(), *span))
            } else {
                None
            }
        })
    }
}

/// Build symbol index and semantic diagnostics (full profile).
pub fn build_index(source: &str, lines: &[(Span, LineKind)]) -> (Index, Vec<Diagnostic>) {
    build_index_with_profile(source, lines, crate::AnalysisProfile::Full)
}

/// Build symbol index under `profile`.
///
/// [`crate::AnalysisProfile::Extracted`] skips per-instance symbols, outline children,
/// and duplicate-name scans. Model/subckt names from instances are recorded as sparse
/// references (first span only) for unknown-model diagnostics.
pub fn build_index_with_profile(
    _source: &str,
    lines: &[(Span, LineKind)],
    profile: crate::AnalysisProfile,
) -> (Index, Vec<Diagnostic>) {
    let extracted = matches!(profile, crate::AnalysisProfile::Extracted);
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

                if extracted {
                    if let Some((model_name, model_span)) = model_ref {
                        // Sparse refs: one span per unique name keeps memory bounded.
                        index.add_reference_first(
                            SymbolKind::Subckt,
                            model_name.clone(),
                            *model_span,
                        );
                        index.add_reference_first(
                            SymbolKind::Model,
                            model_name.clone(),
                            *model_span,
                        );
                    }
                } else {
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
                    }
                }
            }
            LineKind::Include { .. }
            | LineKind::LibCall { .. }
            | LineKind::LibSection { .. }
            | LineKind::Endl { .. }
            | LineKind::Other => {}
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

    if !extracted {
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
    }

    diagnostics.extend(unknown_model_diagnostics(&index));

    (index, diagnostics)
}

/// Emit `spice/unknown-model` for model/subckt references with no local definition.
///
/// Prefer the references map (works for extracted-mode sparse refs and full mode).
/// Callers that merge include/lib definitions should filter these afterward.
pub fn unknown_model_diagnostics(index: &Index) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen = HashMap::<String, Span>::new();

    for (name_lc, span) in index.model_reference_sites() {
        seen.entry(name_lc.to_string()).or_insert(span);
    }

    // Full mode also stores model-ref symbols; cover any that lack a reference entry.
    for symbol in &index.symbols {
        if symbol.kind != SymbolKind::Model {
            continue;
        }
        if index
            .definition_span(SymbolKind::Model, &symbol.name)
            .is_some_and(|span| span == symbol.name_span)
        {
            continue;
        }
        let key = symbol.name.to_ascii_lowercase();
        seen.entry(key).or_insert(symbol.name_span);
    }

    for (name_lc, span) in seen {
        let display = index
            .symbols
            .iter()
            .find(|s| {
                (s.kind == SymbolKind::Model || s.kind == SymbolKind::Subckt)
                    && s.name.eq_ignore_ascii_case(&name_lc)
            })
            .map(|s| s.name.clone())
            .unwrap_or(name_lc.clone());

        if !index.has_model_or_subckt(&display) {
            diagnostics.push(Diagnostic {
                message: format!("'{display}' is not defined as a model or subcircuit"),
                severity: Severity::Warning,
                span,
                code: Some("spice/unknown-model".into()),
            });
        }
    }

    diagnostics
}

/// Byte span of the source line containing `offset` (including the trailing newline if present).
pub fn line_span_containing(source: &str, offset: usize) -> Option<Span> {
    if source.is_empty() {
        return None;
    }
    let offset = offset.min(source.len());
    let start = source[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let end = source[offset..]
        .find('\n')
        .map(|i| offset + i + 1)
        .unwrap_or(source.len());
    Some(Span { start, end })
}

/// Model/subckt token under `offset` on an instance line (extracted-mode goto fallback).
pub fn model_ref_at_offset(source: &str, offset: usize) -> Option<(String, Span)> {
    let line_span = line_span_containing(source, offset)?;
    match classify_line(source, line_span) {
        LineKind::Instance {
            model_ref: Some((name, span)),
            ..
        } if offset >= span.start && offset <= span.end => Some((name, span)),
        _ => None,
    }
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
    Subckt {
        name: String,
        name_span: Span,
    },
    Ends {
        name: Option<String>,
    },
    Model {
        name: String,
        name_span: Span,
    },
    Param {
        name: String,
        name_span: Span,
    },
    Instance {
        name: String,
        name_span: Span,
        model_ref: Option<(String, Span)>,
    },
    /// `.include` / `.inc` path
    Include {
        path: String,
        path_span: Span,
    },
    /// `.lib 'file' entry` — load a named section from a library file
    LibCall {
        path: String,
        path_span: Span,
        entry: String,
        entry_span: Span,
    },
    /// `.lib entry` section header inside a library file
    LibSection {
        name: String,
        name_span: Span,
    },
    /// `.endl` / `.endl entry`
    Endl {
        name: Option<String>,
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
            name_line_kind(source, line_span, text, &name, |name, name_span| {
                LineKind::Subckt { name, name_span }
            })
        }
        "ends" => {
            let name = parts.next().map(|s| s.to_string());
            LineKind::Ends { name }
        }
        "model" => {
            let name = parts.next().unwrap_or("").to_string();
            name_line_kind(source, line_span, text, &name, |name, name_span| {
                LineKind::Model { name, name_span }
            })
        }
        "param" => {
            let raw = parts.next().unwrap_or("");
            let name = raw.split('=').next().unwrap_or(raw).to_string();
            name_line_kind(source, line_span, text, &name, |name, name_span| {
                LineKind::Param { name, name_span }
            })
        }
        "include" | "inc" => classify_include(source, line_span, text, &mut parts),
        "lib" => classify_lib(source, line_span, text, &mut parts),
        "endl" => {
            let name = parts.next().map(|s| unquote(s).to_string());
            LineKind::Endl { name }
        }
        _ => LineKind::Other,
    }
}

fn classify_include<'a>(
    source: &str,
    line_span: Span,
    text: &str,
    parts: &mut impl Iterator<Item = &'a str>,
) -> LineKind {
    let Some(raw) = parts.next() else {
        return LineKind::Other;
    };
    let path = unquote(raw).to_string();
    if path.is_empty() {
        return LineKind::Other;
    }
    let path_span = path_token_span(source, line_span, text, raw);
    LineKind::Include { path, path_span }
}

fn classify_lib<'a>(
    source: &str,
    line_span: Span,
    text: &str,
    parts: &mut impl Iterator<Item = &'a str>,
) -> LineKind {
    let Some(first) = parts.next() else {
        return LineKind::Other;
    };
    let second = parts.next();

    match second {
        Some(entry_raw) if looks_like_path(first) || second_is_lib_entry(first, entry_raw) => {
            let path = unquote(first).to_string();
            let entry = unquote(entry_raw).to_string();
            if path.is_empty() || entry.is_empty() {
                return LineKind::Other;
            }
            LineKind::LibCall {
                path,
                path_span: path_token_span(source, line_span, text, first),
                entry: entry.clone(),
                entry_span: path_token_span(source, line_span, text, entry_raw),
            }
        }
        None => {
            let name = unquote(first).to_string();
            if name.is_empty() {
                return LineKind::Other;
            }
            // `.lib entry` section header (or bare `.lib file` treated as include of whole file)
            if looks_like_path(first) && unquote(first).contains(['/', '\\', '.']) {
                let path = unquote(first).to_string();
                LineKind::Include {
                    path,
                    path_span: path_token_span(source, line_span, text, first),
                }
            } else {
                LineKind::LibSection {
                    name: name.clone(),
                    name_span: path_token_span(source, line_span, text, first),
                }
            }
        }
        Some(_) => {
            // Ambiguous two-arg form without path-like first token: treat as section name only.
            let name = unquote(first).to_string();
            LineKind::LibSection {
                name: name.clone(),
                name_span: path_token_span(source, line_span, text, first),
            }
        }
    }
}

fn second_is_lib_entry(first: &str, _entry: &str) -> bool {
    // HSPICE calls always pass a filename then entry; quoted first token is a path.
    is_quoted(first) || looks_like_path(first)
}

fn is_quoted(token: &str) -> bool {
    (token.starts_with('\'') && token.ends_with('\''))
        || (token.starts_with('"') && token.ends_with('"'))
}

fn looks_like_path(token: &str) -> bool {
    let bare = unquote(token);
    is_quoted(token)
        || bare.contains('/')
        || bare.contains('\\')
        || bare.contains('.')
        || std::path::Path::new(bare).is_absolute()
}

fn unquote(token: &str) -> &str {
    if token.len() >= 2
        && ((token.starts_with('\'') && token.ends_with('\''))
            || (token.starts_with('"') && token.ends_with('"')))
    {
        &token[1..token.len() - 1]
    } else {
        token
    }
}

fn path_token_span(source: &str, line_span: Span, line_text: &str, token: &str) -> Span {
    let needle = unquote(token);
    if needle.is_empty() {
        return subspan(source, line_span, line_text, token);
    }
    // Prefer the bare path inside quotes when present.
    if let Some(pos) = line_text.find(needle) {
        let start = line_span.start + pos;
        return Span {
            start,
            end: start + needle.len(),
        };
    }
    subspan(source, line_span, line_text, token)
}

fn classify_instance(source: &str, line_span: Span, text: &str) -> LineKind {
    let Some(first) = text.chars().next() else {
        return LineKind::Other;
    };
    if !first.is_ascii_alphabetic() {
        return LineKind::Other;
    }

    let name_end = first.len_utf8()
        + text[first.len_utf8()..]
            .chars()
            .take_while(|c| {
                c.is_ascii_alphanumeric()
                    || matches!(c, '_' | '.' | '$' | ':' | '#' | '[' | ']' | '<' | '>' | '-')
            })
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
                Some((
                    model_name.to_string(),
                    subspan(source, line_span, text, model_name),
                ))
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
        parser.set_language(&tree_sitter_spice::language()).unwrap();
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

    #[test]
    fn model_ref_at_offset_finds_x_instance_model() {
        let source = "X1 a b mybuf\n";
        let offset = source.find("mybuf").unwrap();
        let (name, span) = model_ref_at_offset(source, offset).expect("model ref");
        assert_eq!(name, "mybuf");
        assert_eq!(&source[span.start..span.end], "mybuf");
    }

    #[test]
    fn extracted_index_keeps_defs_only() {
        let source = ".subckt buf in out\nX1 a b buf\nX2 a b buf\n.ends\n";
        let parsed = lines(source);
        let (index, diags) =
            build_index_with_profile(source, &parsed, crate::AnalysisProfile::Extracted);
        assert!(index.has_definition(SymbolKind::Subckt, "buf"));
        assert_eq!(
            index.symbols.iter().filter(|s| s.kind == SymbolKind::Instance).count(),
            0
        );
        assert!(diags
            .iter()
            .all(|d| d.code.as_deref() != Some("spice/duplicate-name")));
    }
}
