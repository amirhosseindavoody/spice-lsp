//! Tree-sitter language binding for SPICE netlists.

use tree_sitter::Language;

extern "C" {
    fn tree_sitter_spice() -> Language;
}

/// Returns the Tree-sitter [`Language`] for SPICE netlists.
pub fn language() -> Language {
    unsafe { tree_sitter_spice() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_load_language() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&language())
            .expect("valid tree-sitter language");
    }
}
