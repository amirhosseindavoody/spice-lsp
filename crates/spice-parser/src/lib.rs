//! SPICE netlist parsing and syntax diagnostics.

mod analyze;
mod diagnostic;
mod subckt;

pub use analyze::analyze;
pub use diagnostic::{Diagnostic, Severity, Span};
