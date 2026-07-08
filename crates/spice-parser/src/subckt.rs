#[cfg(test)]
mod tests {
    use crate::diagnostic::Span;
    use crate::symbols::{build_index, classify_line, LineKind};

    #[test]
    fn detects_unmatched_ends() {
        let source = ".ends\n";
        let lines = vec![(
            Span { start: 0, end: 5 },
            classify_line(source, Span { start: 0, end: 5 }),
        )];
        let (_, diags) = build_index(source, &lines);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("unexpected .ends"));
    }

    #[test]
    fn ends_line_kind_has_optional_name() {
        let source = ".ends buffer\n";
        let kind = classify_line(source, Span { start: 0, end: 12 });
        assert!(matches!(kind, LineKind::Ends { .. }));
    }
}
