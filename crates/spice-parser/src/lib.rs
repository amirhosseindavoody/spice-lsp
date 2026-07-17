//! SPICE netlist parsing, diagnostics, and formatting.

mod analyze;
mod diagnostic;
mod dialect;
mod format;
mod hover_token;
pub mod includes;
mod profile;
mod subckt;
mod symbols;

pub use analyze::{
    analyze, analyze_for_mode, analyze_lines, analyze_lines_with_profile, analyze_with_dialect,
    analyze_with_profile, collect_classified_lines, ParseResult,
};
pub use diagnostic::{Diagnostic, Severity, Span};
pub use dialect::{Dialect, DialectProfile};
pub use format::{format_source, FormatOptions, KeywordCase};
pub use hover_token::{hover_token_at, HoverKind, HoverToken};
pub use includes::{
    analyze_with_includes, collect_include_refs, disk_loader_with_overrides, filter_unknown_models,
    find_lib_section_span, resolve_include_path, resolve_includes, IncludeRef, IncludeResolution,
    IncludedFile, ResolveOptions, ResolvedInclude, DEFAULT_MAX_INCLUDE_DEPTH,
};
pub use profile::{
    resolve_profile, AnalysisMode, AnalysisProfile, DEFAULT_EXTRACTED_BYTE_THRESHOLD,
};
pub use symbols::{
    build_index, build_index_with_profile, classify_line, line_span_containing,
    model_ref_at_offset, unknown_model_diagnostics, DocumentSymbolEntry, Index, LineKind, Symbol,
    SymbolKind,
};
