# Formatter

Design for the SPICE netlist formatter. Not shipped yet — documented so parser and CST design stay compatible.

## Goals

- Columnar alignment of instance name, nodes, model/value, and parameters
- Consistent handling of `+` continuation lines
- Normalized keyword and unit casing (configurable)
- Idempotent output: `format(format(x)) == format(x)`

## Input / output

| Input | Output |
|-------|--------|
| LSP `textDocument/formatting` request | `TextEdit[]` covering full document |
| LSP `textDocument/rangeFormatting` | `TextEdit[]` for selection (or expanded logical statement) |
| CLI `spice-lsp format --check file.cir` | Exit 1 if not formatted; `--write` in place |

## Formatting rules (draft)

### Instance lines

Align columns within a contiguous block of instance lines:

```spice
* before
R1 in out 1k
C1 out 0 1u
X1 a b mycell

* after (aligned)
R1  in  out  1k
C1  out  0    1u
X1  a   b    mycell
```

Column boundaries derive from the widest field in each column group.

### Continuation lines

Standardize `+` continuations with fixed indent:

```spice
* before
M1 d g s b nfet W=10u L=0.18u
+ $nfin=2

* after
M1  d  g  s  b  nfet  W=10u  L=0.18u
+   $nfin=2
```

### Directives

Keep dot-directives (`.tran`, `.model`, `.subckt`) on their own lines. Align parameters within a directive where sensible:

```spice
.model nfet nmos ( VTO=0.4  KP=200u )
```

### Comments

- Preserve comment text; normalize spacing before inline `;` / `$` comments
- Do not reorder or remove comment lines

## Architecture

The formatter lives in `crates/spice-parser` (or a sibling `crates/spice-format` if it grows large):

1. Parse buffer to CST (reuse parser)
2. Walk relevant nodes; compute column widths per alignment group
3. Render each line to a string buffer
4. Diff old vs new text → minimal `TextEdit` set (or full document replacement)

Formatting must **not** mutate the parse tree in place; generate text from a pretty-printer pass.

## Configuration

| Option | Values | Default |
|--------|--------|---------|
| `indentWidth` | 2, 4 | 2 |
| `keywordCase` | upper, lower, preserve | upper |
| `alignColumns` | true, false | true |
| `maxLineWidth` | number | 120 (soft wrap with `+`) |

Plumb through LSP `FormattingOptions` (`tabSize`, `insertSpaces`) where compatible.

## Testing

Golden-file tests in `crates/spice-parser/tests/format/`:

```
input.cir  →  format  →  compare to expected.cir
```

Include edge cases: empty file, comments only, deeply nested `.subckt`, long parameter lists.

## Related

- [Architecture](4_architecture.md) — FormatterEngine placement
- [LSP features](5_lsp-features.md) — when formatting registers as a capability
