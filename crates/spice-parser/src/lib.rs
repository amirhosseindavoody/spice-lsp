//! SPICE netlist parsing and syntax diagnostics.

mod analyze;
mod diagnostic;
mod subckt;
mod symbols;

pub use analyze::{analyze, ParseResult};
pub use diagnostic::{Diagnostic, Severity, Span};
pub use symbols::{
    build_index, classify_line, DocumentSymbolEntry, Index, LineKind, Symbol, SymbolKind,
};
