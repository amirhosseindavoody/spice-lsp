use tree_sitter::{Node, Parser, Tree};

use crate::diagnostic::{Diagnostic, Severity, Span};
use crate::dialect::Dialect;
use crate::profile::{
    resolve_profile, AnalysisMode, AnalysisProfile, DEFAULT_EXTRACTED_BYTE_THRESHOLD,
};
use crate::symbols::{build_index_with_profile, classify_line, Index, LineKind};

/// Result of analyzing a netlist buffer.
#[derive(Debug, Clone)]
pub struct ParseResult {
    pub tree: Tree,
    pub diagnostics: Vec<Diagnostic>,
    pub index: Index,
    pub dialect: Dialect,
    pub profile: AnalysisProfile,
}

/// Parse `source` with the default dialect (HSPICE) and return diagnostics.
pub fn analyze(source: &str) -> ParseResult {
    analyze_with_dialect(source, Dialect::default())
}

/// Parse `source` under `dialect` with the default (full) analysis profile.
///
/// Phase A/B: the shared grammar is used for all dialects; `dialect` is stored
/// for hover / future profile-sensitive diagnostics.
pub fn analyze_with_dialect(source: &str, dialect: Dialect) -> ParseResult {
    analyze_with_profile(source, dialect, AnalysisProfile::Full)
}

/// Parse and analyze `source` under an explicit [`AnalysisProfile`].
pub fn analyze_with_profile(
    source: &str,
    dialect: Dialect,
    profile: AnalysisProfile,
) -> ParseResult {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_spice::language())
        .expect("tree-sitter-spice language");
    let tree = parser.parse(source, None).expect("parse succeeds");
    let root = tree.root_node();
    let lines = collect_lines(source, root);
    finish_analyze(source, dialect, profile, tree, &lines)
}

/// Classify every directive / instance line in `source` (Tree-sitter walk).
pub fn collect_classified_lines(source: &str) -> Vec<(Span, LineKind)> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_spice::language())
        .expect("tree-sitter-spice language");
    let tree = parser.parse(source, None).expect("parse succeeds");
    collect_lines(source, tree.root_node())
}

/// Analyze pre-classified lines (shared by plain analyze and include resolution).
///
/// Still performs one Tree-sitter parse for syntax diagnostics.
pub fn analyze_lines(
    source: &str,
    dialect: Dialect,
    lines: &[(Span, LineKind)],
) -> ParseResult {
    analyze_lines_with_profile(source, dialect, lines, AnalysisProfile::Full)
}

/// Like [`analyze_lines`] but with an explicit profile.
pub fn analyze_lines_with_profile(
    source: &str,
    dialect: Dialect,
    lines: &[(Span, LineKind)],
    profile: AnalysisProfile,
) -> ParseResult {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_spice::language())
        .expect("tree-sitter-spice language");

    let tree = parser.parse(source, None).expect("parse succeeds");
    finish_analyze(source, dialect, profile, tree, lines)
}

fn finish_analyze(
    source: &str,
    dialect: Dialect,
    profile: AnalysisProfile,
    tree: Tree,
    lines: &[(Span, LineKind)],
) -> ParseResult {
    let _profile = dialect.profile();
    let root = tree.root_node();

    let (index, semantic_diagnostics) = build_index_with_profile(source, lines, profile);

    let mut diagnostics = tree_diagnostics(source, root);
    diagnostics.extend(semantic_diagnostics);
    diagnostics.sort_by_key(|d| (d.span.start, d.span.end));
    diagnostics.dedup_by(|a, b| a.span == b.span && a.message == b.message);

    ParseResult {
        tree,
        diagnostics,
        index,
        dialect,
        profile,
    }
}

/// Resolve profile from mode + buffer size and analyze in one parse.
pub fn analyze_for_mode(
    source: &str,
    dialect: Dialect,
    mode: AnalysisMode,
    threshold: usize,
) -> ParseResult {
    let profile = resolve_profile(mode, source.len(), threshold);
    analyze_with_profile(source, dialect, profile)
}

/// Convenience: auto mode with the default byte threshold.
#[allow(dead_code)]
pub fn analyze_auto(source: &str, dialect: Dialect) -> ParseResult {
    analyze_for_mode(
        source,
        dialect,
        AnalysisMode::Auto,
        DEFAULT_EXTRACTED_BYTE_THRESHOLD,
    )
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
    use crate::symbols::SymbolKind;
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
                d.code.as_deref() == Some("spice/duplicate-name") && d.message.contains("R1")
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
                d.code.as_deref() == Some("spice/unknown-model") && d.message.contains("missingbuf")
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

    #[test]
    fn hspice_data_block_bare_rows_have_no_diagnostics() {
        let source = fixture("valid/hspice-data-block.cir");
        let result = analyze_with_dialect(&source, Dialect::Hspice);
        assert!(
            result.diagnostics.is_empty(),
            "bare .DATA rows should not be syntax errors: {:?}",
            result.diagnostics
        );
        assert!(!result.tree.root_node().has_error());
    }

    #[test]
    fn hspice_data_block_plus_rows_still_ok() {
        let source = fixture("valid/hspice-data-block-plus.cir");
        let result = analyze_with_dialect(&source, Dialect::Hspice);
        assert!(
            result.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn extracted_profile_skips_instances_keeps_defs() {
        let source = "\
.subckt buf in out
.param gain=1
X1 a b buf
X2 a b buf
.ends
.model nmos_tt nmos level=1
M1 d g s b nmos_tt
";
        let result = analyze_with_profile(source, Dialect::Hspice, AnalysisProfile::Extracted);
        assert_eq!(result.profile, AnalysisProfile::Extracted);
        assert!(result.index.has_definition(SymbolKind::Subckt, "buf"));
        assert!(result.index.has_definition(SymbolKind::Model, "nmos_tt"));
        assert!(result.index.has_definition(SymbolKind::Param, "gain"));
        assert!(
            !result
                .index
                .symbols
                .iter()
                .any(|s| s.kind == SymbolKind::Instance),
            "extracted mode must not store instance symbols"
        );
        let buf_outline = result
            .index
            .document_symbols
            .iter()
            .find(|s| s.name == "buf")
            .expect("buf outline");
        assert!(
            buf_outline
                .children
                .iter()
                .all(|c| c.kind != SymbolKind::Instance),
            "extracted outline should omit instance children: {:?}",
            buf_outline.children
        );
        assert!(
            buf_outline
                .children
                .iter()
                .any(|c| c.kind == SymbolKind::Param && c.name == "gain"),
            "extracted outline should still list params"
        );
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|d| d.code.as_deref() == Some("spice/duplicate-name")),
            "extracted mode skips duplicate-name"
        );
        // Known subckt/model refs should not warn.
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|d| d.code.as_deref() == Some("spice/unknown-model")),
            "unexpected unknown-model: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn extracted_profile_reports_unknown_model_sparsely() {
        let source = "X1 a b missingbuf\nX2 a b missingbuf\n";
        let result = analyze_with_profile(source, Dialect::Hspice, AnalysisProfile::Extracted);
        let unknowns: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.code.as_deref() == Some("spice/unknown-model"))
            .collect();
        assert_eq!(unknowns.len(), 1, "one diagnostic per unique name: {unknowns:?}");
        assert!(unknowns[0].message.contains("missingbuf"));
    }

    #[test]
    fn auto_mode_uses_threshold() {
        let source = "R1 a b 1k\n";
        let full = analyze_for_mode(source, Dialect::Hspice, AnalysisMode::Auto, 1024);
        assert_eq!(full.profile, AnalysisProfile::Full);

        let extracted =
            analyze_for_mode(source, Dialect::Hspice, AnalysisMode::Auto, 1);
        assert_eq!(extracted.profile, AnalysisProfile::Extracted);
    }
}
