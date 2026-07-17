//! SPICE netlist pretty-printer.
//!
//! Formats source text without mutating the CST. Column alignment, continuation
//! normalization, and directive keyword casing follow [`FormatOptions`].

/// How to rewrite leading `.directive` keywords.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeywordCase {
    #[default]
    Upper,
    Lower,
    Preserve,
}

/// Configuration for [`format_source`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatOptions {
    /// Spaces after `+` on continuation lines.
    pub indent_width: usize,
    /// Case for the directive keyword (the token after `.`).
    pub keyword_case: KeywordCase,
    /// Align fields within contiguous instance blocks.
    pub align_columns: bool,
    /// Soft wrap width; long statements continue with `+` lines.
    pub max_line_width: usize,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            indent_width: 2,
            keyword_case: KeywordCase::Upper,
            align_columns: true,
            max_line_width: 120,
        }
    }
}

/// Format an entire netlist buffer. Output is idempotent under the same options.
pub fn format_source(source: &str, options: &FormatOptions) -> String {
    if source.is_empty() {
        return String::new();
    }

    let newline = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let had_trailing_newline = source.ends_with('\n');

    let physical = split_physical_lines(source);
    let statements = group_statements(&physical);
    let rendered = render_statements(&statements, options);

    let mut out = rendered.join(newline);
    if had_trailing_newline {
        out.push_str(newline);
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineKind {
    Blank,
    Comment,
    Continuation,
    Directive,
    Instance,
    DataValue,
    Other,
}

#[derive(Debug, Clone)]
struct PhysicalLine<'a> {
    kind: LineKind,
    /// Full line without newline characters.
    text: &'a str,
}

#[derive(Debug, Clone)]
enum Statement<'a> {
    Blank,
    Comment(&'a str),
    /// Instance / directive / data / other, with optional continuation payloads.
    Code {
        kind: LineKind,
        head: &'a str,
        continuations: Vec<&'a str>,
    },
}

fn split_physical_lines(source: &str) -> Vec<PhysicalLine<'_>> {
    let mut lines = Vec::new();
    for raw in source.split('\n') {
        let text = raw.strip_suffix('\r').unwrap_or(raw);
        lines.push(PhysicalLine {
            kind: classify_physical(text),
            text,
        });
    }
    // `split` yields a trailing empty string when source ends with `\n`.
    // Keep it as a blank only if it represents a real empty line mid-file;
    // a final empty from trailing newline is dropped so join can re-add EOL.
    if source.ends_with('\n') {
        if let Some(last) = lines.last() {
            if last.text.is_empty() {
                lines.pop();
            }
        }
    }
    lines
}

fn classify_physical(text: &str) -> LineKind {
    let trimmed = text.trim_start();
    if trimmed.is_empty() {
        return LineKind::Blank;
    }
    let first = trimmed.as_bytes()[0];
    match first {
        b'*' | b';' => LineKind::Comment,
        b'$' => {
            // Line-leading `$` is a comment in Ngspice/LTspice; HSPICE params
            // on their own line are rare. Treat as comment to match the grammar.
            LineKind::Comment
        }
        b'+' => LineKind::Continuation,
        b'.' => LineKind::Directive,
        _ if looks_like_data_value(trimmed) => LineKind::DataValue,
        _ if trimmed
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic()) =>
        {
            LineKind::Instance
        }
        _ => LineKind::Other,
    }
}

fn looks_like_data_value(trimmed: &str) -> bool {
    let mut chars = trimmed.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if first == '+' || first == '-' {
        return chars.next().is_some_and(|c| c.is_ascii_digit() || c == '.');
    }
    first.is_ascii_digit() || (first == '.' && chars.next().is_some_and(|c| c.is_ascii_digit()))
}

fn group_statements<'a>(lines: &'a [PhysicalLine<'a>]) -> Vec<Statement<'a>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i];
        match line.kind {
            LineKind::Blank => {
                out.push(Statement::Blank);
                i += 1;
            }
            LineKind::Comment => {
                out.push(Statement::Comment(line.text));
                i += 1;
            }
            LineKind::Continuation => {
                // Orphan continuation: keep as code with a synthetic `+` head.
                let mut conts = vec![continuation_payload(line.text)];
                i += 1;
                while i < lines.len() && lines[i].kind == LineKind::Continuation {
                    conts.push(continuation_payload(lines[i].text));
                    i += 1;
                }
                out.push(Statement::Code {
                    kind: LineKind::Other,
                    head: "+",
                    continuations: conts,
                });
            }
            kind @ (LineKind::Directive
            | LineKind::Instance
            | LineKind::DataValue
            | LineKind::Other) => {
                let head = line.text;
                i += 1;
                let mut conts = Vec::new();
                while i < lines.len() && lines[i].kind == LineKind::Continuation {
                    conts.push(continuation_payload(lines[i].text));
                    i += 1;
                }
                out.push(Statement::Code {
                    kind,
                    head,
                    continuations: conts,
                });
            }
        }
    }
    out
}

fn continuation_payload(text: &str) -> &str {
    let trimmed = text.trim_start();
    trimmed.strip_prefix('+').unwrap_or(trimmed).trim_start()
}

fn render_statements(statements: &[Statement<'_>], options: &FormatOptions) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < statements.len() {
        match &statements[i] {
            Statement::Blank => {
                out.push(String::new());
                i += 1;
            }
            Statement::Comment(text) => {
                out.push((*text).trim_end().to_string());
                i += 1;
            }
            Statement::Code { kind, .. }
                if *kind == LineKind::Instance && options.align_columns =>
            {
                let start = i;
                i += 1;
                while i < statements.len() {
                    match &statements[i] {
                        Statement::Code {
                            kind: LineKind::Instance,
                            ..
                        } => i += 1,
                        _ => break,
                    }
                }
                let block = &statements[start..i];
                out.extend(render_instance_block(block, options));
            }
            Statement::Code {
                kind,
                head,
                continuations,
            } => {
                out.extend(render_code_statement(*kind, head, continuations, options));
                i += 1;
            }
        }
    }
    out
}

fn render_instance_block(block: &[Statement<'_>], options: &FormatOptions) -> Vec<String> {
    let mut parsed: Vec<(Vec<String>, Option<String>)> = Vec::new();
    for stmt in block {
        let Statement::Code {
            head,
            continuations,
            ..
        } = stmt
        else {
            continue;
        };
        let (code, comment) = split_inline_comment(head);
        let mut tokens = tokenize(code);
        for cont in continuations {
            let (cont_code, cont_comment) = split_inline_comment(cont);
            tokens.extend(tokenize(cont_code));
            // Only keep a comment from the last fragment that has one.
            let _ = cont_comment;
        }
        // Prefer comment on the head line; if none, scan continuations.
        let comment = comment.or_else(|| {
            continuations
                .iter()
                .rev()
                .find_map(|c| split_inline_comment(c).1)
        });
        parsed.push((tokens, comment));
    }

    let col_count = parsed.iter().map(|(t, _)| t.len()).max().unwrap_or(0);
    let mut widths = vec![0usize; col_count];
    if options.align_columns {
        for (tokens, _) in &parsed {
            for (idx, tok) in tokens.iter().enumerate() {
                widths[idx] = widths[idx].max(tok.len());
            }
        }
    }

    let mut lines = Vec::new();
    for (tokens, comment) in parsed {
        lines.extend(emit_wrapped_tokens(
            &tokens,
            comment.as_deref(),
            Some(&widths),
            LineKind::Instance,
            options,
        ));
    }
    lines
}

fn render_code_statement(
    kind: LineKind,
    head: &str,
    continuations: &[&str],
    options: &FormatOptions,
) -> Vec<String> {
    let (code, comment) = split_inline_comment(head);
    let mut tokens = match kind {
        LineKind::Directive => tokenize_directive(code, options),
        _ => tokenize(code),
    };
    for cont in continuations {
        let (cont_code, _) = split_inline_comment(cont);
        // Continuations of directives are parameter tokens, not new directives.
        tokens.extend(tokenize(cont_code));
    }
    let comment = comment.or_else(|| {
        continuations
            .iter()
            .rev()
            .find_map(|c| split_inline_comment(c).1)
    });
    emit_wrapped_tokens(&tokens, comment.as_deref(), None, kind, options)
}

fn tokenize_directive(code: &str, options: &FormatOptions) -> Vec<String> {
    let trimmed = code.trim();
    if !trimmed.starts_with('.') {
        return tokenize(trimmed);
    }
    let rest = &trimmed[1..];
    let mut parts = split_ws_preserving_quotes(rest);
    if parts.is_empty() {
        return vec![".".to_string()];
    }
    let keyword = match options.keyword_case {
        KeywordCase::Upper => parts[0].to_ascii_uppercase(),
        KeywordCase::Lower => parts[0].to_ascii_lowercase(),
        KeywordCase::Preserve => parts[0].clone(),
    };
    parts[0] = keyword;
    let mut tokens = vec![format!(".{}", parts[0])];
    tokens.extend(parts.into_iter().skip(1));
    tokens
}

fn tokenize(code: &str) -> Vec<String> {
    split_ws_preserving_quotes(code.trim())
}

fn split_ws_preserving_quotes(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut buf = String::new();
    let mut in_quote: Option<char> = None;

    for ch in text.chars() {
        if let Some(q) = in_quote {
            buf.push(ch);
            if ch == q {
                in_quote = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' => {
                in_quote = Some(ch);
                buf.push(ch);
            }
            c if c.is_whitespace() => {
                if !buf.is_empty() {
                    tokens.push(std::mem::take(&mut buf));
                }
            }
            _ => buf.push(ch),
        }
    }
    if !buf.is_empty() {
        tokens.push(buf);
    }
    tokens
}

/// Split trailing `; …` inline comment. `$` is not treated as a comment
/// delimiter here so HSPICE `$param` tokens stay intact.
fn split_inline_comment(line: &str) -> (&str, Option<String>) {
    let mut in_quote: Option<char> = None;
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if let Some(q) = in_quote {
            if ch == q {
                in_quote = None;
            }
            i += 1;
            continue;
        }
        match ch {
            '\'' | '"' => {
                in_quote = Some(ch);
                i += 1;
            }
            ';' => {
                let code = line[..i].trim_end();
                let comment = line[i..].trim_end().to_string();
                return (code, Some(comment));
            }
            _ => i += 1,
        }
    }
    (line.trim_end(), None)
}

fn emit_wrapped_tokens(
    tokens: &[String],
    comment: Option<&str>,
    widths: Option<&[usize]>,
    kind: LineKind,
    options: &FormatOptions,
) -> Vec<String> {
    if tokens.is_empty() {
        if let Some(c) = comment {
            return vec![c.to_string()];
        }
        return Vec::new();
    }

    let indent = " ".repeat(options.indent_width);
    let max_width = options.max_line_width.max(20);

    // Column stride includes a one-space gap after each non-final field.
    let strides: Option<Vec<usize>> = widths.map(|w| {
        w.iter()
            .enumerate()
            .map(|(i, &len)| if i + 1 == w.len() { len } else { len + 1 })
            .collect()
    });

    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut col_index = 0usize;
    let mut on_continuation = false;

    for token in tokens {
        let piece = if !on_continuation {
            if let Some(ref strides) = strides {
                let stride = strides.get(col_index).copied().unwrap_or(token.len());
                col_index += 1;
                format!("{token:<stride$}")
            } else {
                col_index += 1;
                token.clone()
            }
        } else {
            token.clone()
        };

        if current.is_empty() {
            current = if on_continuation {
                format!("+{indent}{}", piece.trim_end())
            } else {
                piece
            };
            continue;
        }

        let candidate = if on_continuation || strides.is_none() {
            format!("{} {}", current.trim_end(), piece.trim_end())
        } else {
            // Alignment path: `piece` already includes the trailing gap.
            format!("{}{}", current, piece)
        };

        if display_width(&candidate) > max_width {
            lines.push(current.trim_end().to_string());
            on_continuation = true;
            current = format!("+{indent}{}", token.trim_end());
        } else {
            current = candidate;
        }
    }

    if !current.is_empty() {
        let line = if let Some(c) = comment {
            format!("{}  {}", current.trim_end(), c)
        } else {
            current.trim_end().to_string()
        };
        lines.push(line);
    } else if let Some(c) = comment {
        lines.push(c.to_string());
    }

    let _ = kind;
    lines
}

fn display_width(text: &str) -> usize {
    text.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aligns_instance_block() {
        let src = "* before\nR1 in out 1k\nC1 out 0 1u\nX1 a b mycell\n";
        let out = format_source(src, &FormatOptions::default());
        assert_eq!(
            out,
            "* before\nR1 in  out 1k\nC1 out 0   1u\nX1 a   b   mycell\n"
        );
    }

    #[test]
    fn formats_continuation_joined_when_short() {
        let src = "M1 d g s b nfet W=10u L=0.18u\n+ $nfin=2\n";
        let out = format_source(src, &FormatOptions::default());
        // Continuations are folded into the statement, then re-wrapped if needed.
        assert_eq!(out, "M1 d g s b nfet W=10u L=0.18u $nfin=2\n");
    }

    #[test]
    fn formats_continuation_wrap_keeps_plus() {
        let src = "M1 d g s b nfet W=10u L=0.18u\n+ $nfin=2\n";
        let opts = FormatOptions {
            max_line_width: 28,
            indent_width: 2,
            ..FormatOptions::default()
        };
        let out = format_source(src, &opts);
        assert!(
            out.lines().any(|l| l.starts_with("+  ")),
            "expected + continuation, got:\n{out}"
        );
        assert!(out.contains("$nfin=2"), "got:\n{out}");
    }

    #[test]
    fn wraps_long_lines_with_plus() {
        let src = "M1 d g s b nfet W=10u L=0.18u AS=1e-12 AD=1e-12 PS=1u PD=1u NRD=0 NRS=0\n";
        let opts = FormatOptions {
            max_line_width: 40,
            ..FormatOptions::default()
        };
        let out = format_source(src, &opts);
        assert!(out.contains("\n+  "), "expected continuation, got:\n{out}");
        let again = format_source(&out, &opts);
        assert_eq!(out, again, "format should be idempotent");
    }

    #[test]
    fn uppercases_directive_keywords() {
        let src = ".tran 1u 1m\n.subckt buf in out\n.ends\n";
        let out = format_source(src, &FormatOptions::default());
        assert!(out.starts_with(".TRAN 1u 1m\n"));
        assert!(out.contains(".SUBCKT buf in out\n"));
        assert!(out.contains(".ENDS\n"));
    }

    #[test]
    fn preserves_comments_and_blank_lines() {
        let src = "* header\n\nR1 a b 1k ; note\n";
        let out = format_source(src, &FormatOptions::default());
        assert_eq!(out, "* header\n\nR1 a b 1k  ; note\n");
    }

    #[test]
    fn idempotent_on_simple_rc() {
        let src = include_str!("../../../test-data/valid/simple-rc.cir");
        let once = format_source(src, &FormatOptions::default());
        let twice = format_source(&once, &FormatOptions::default());
        assert_eq!(once, twice);
    }

    #[test]
    fn empty_input() {
        assert_eq!(format_source("", &FormatOptions::default()), "");
    }

    #[test]
    fn align_columns_off() {
        let src = "R1 in out 1k\nC1 out 0 1u\n";
        let opts = FormatOptions {
            align_columns: false,
            ..FormatOptions::default()
        };
        let out = format_source(src, &opts);
        assert_eq!(out, "R1 in out 1k\nC1 out 0 1u\n");
    }
}
