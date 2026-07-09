//! Resolve the SPICE construct under the cursor for hover / completion.

use crate::diagnostic::Span;

/// Kind of hoverable construct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoverKind {
    Directive,
    Element,
}

/// Token under the cursor that can be looked up in the reference corpus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoverToken {
    pub kind: HoverKind,
    /// Normalized lookup name: `.tran` or `R` (element letter, uppercase).
    pub name: String,
    pub span: Span,
}

/// Find a hoverable token at byte `offset` in `source`.
pub fn hover_token_at(source: &str, offset: usize) -> Option<HoverToken> {
    let offset = offset.min(source.len());
    let line_start = source[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_end = source[offset..]
        .find('\n')
        .map(|i| offset + i)
        .unwrap_or(source.len());
    let line = &source[line_start..line_end];
    let trimmed = line.trim_start();
    let trim_prefix = line.len() - trimmed.len();
    let content_start = line_start + trim_prefix;

    if trimmed.is_empty() {
        return None;
    }

    if trimmed.starts_with('.') {
        return directive_token(source, content_start, trimmed);
    }

    element_token(source, content_start, trimmed)
}

fn directive_token(source: &str, content_start: usize, trimmed: &str) -> Option<HoverToken> {
    let rest = &trimmed[1..];
    let name_len = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .map(|c| c.len_utf8())
        .sum::<usize>();
    if name_len == 0 {
        return None;
    }
    let name = format!(".{}", rest[..name_len].to_ascii_lowercase());
    let span = Span {
        start: content_start,
        end: content_start + 1 + name_len,
    };
    // Ensure the span is within the source (defensive).
    let _ = &source[span.start..span.end.min(source.len())];
    Some(HoverToken {
        kind: HoverKind::Directive,
        name,
        span,
    })
}

fn element_token(source: &str, content_start: usize, trimmed: &str) -> Option<HoverToken> {
    let first = trimmed.chars().next()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }
    let letter = first.to_ascii_uppercase().to_string();
    let span = Span {
        start: content_start,
        end: content_start + first.len_utf8(),
    };
    let _ = &source[span.start..span.end.min(source.len())];
    Some(HoverToken {
        kind: HoverKind::Element,
        name: letter,
        span,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_directive_name() {
        let source = "* c\n.tran 1n 100n\n";
        let offset = source.find("tran").unwrap();
        let token = hover_token_at(source, offset).unwrap();
        assert_eq!(token.kind, HoverKind::Directive);
        assert_eq!(token.name, ".tran");
    }

    #[test]
    fn finds_element_letter() {
        let source = "R1 a b 1k\n";
        let token = hover_token_at(source, 0).unwrap();
        assert_eq!(token.kind, HoverKind::Element);
        assert_eq!(token.name, "R");
    }
}
