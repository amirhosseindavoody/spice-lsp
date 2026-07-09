//! SPICE netlist parsing and syntax diagnostics.

mod analyze;
mod dialect;
mod diagnostic;
mod hover_token;
mod subckt;
mod symbols;

pub use analyze::{analyze, analyze_with_dialect, ParseResult};
pub use dialect::{Dialect, DialectProfile};
pub use diagnostic::{Diagnostic, Severity, Span};
pub use hover_token::{hover_token_at, HoverKind, HoverToken};
pub use symbols::{
    build_index, classify_line, DocumentSymbolEntry, Index, LineKind, Symbol, SymbolKind,
};
