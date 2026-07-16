//! SPICE netlist parsing and syntax diagnostics.

mod analyze;
mod dialect;
mod diagnostic;
mod hover_token;
pub mod includes;
mod subckt;
mod symbols;

pub use analyze::{
    analyze, analyze_lines, analyze_with_dialect, collect_classified_lines, ParseResult,
};
pub use dialect::{Dialect, DialectProfile};
pub use diagnostic::{Diagnostic, Severity, Span};
pub use hover_token::{hover_token_at, HoverKind, HoverToken};
pub use includes::{
    analyze_with_includes, collect_include_refs, disk_loader_with_overrides, filter_unknown_models,
    find_lib_section_span, resolve_include_path, resolve_includes, IncludeRef, IncludeResolution,
    IncludedFile, ResolvedInclude, ResolveOptions, DEFAULT_MAX_INCLUDE_DEPTH,
};
pub use symbols::{
    build_index, classify_line, unknown_model_diagnostics, DocumentSymbolEntry, Index, LineKind,
    Symbol, SymbolKind,
};
