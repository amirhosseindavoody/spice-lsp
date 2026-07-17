# Formatter

SPICE netlist formatter: columnar instance alignment, `+` continuation wrapping, and directive keyword casing.

## Goals

- Columnar alignment of instance name, nodes, model/value, and parameters
- Consistent handling of `+` continuation lines
- Normalized directive keyword casing (configurable)
- Idempotent output: `format(format(x)) == format(x)`

## Input / output

| Input | Output |
|-------|--------|
| LSP `textDocument/formatting` | `TextEdit[]` (full-document replacement when changed) |
| LSP `textDocument/rangeFormatting` | Same as full-document formatting (alignment stays consistent) |
| CLI `spice-lsp format [--check\|--write] file.cir…` | stdout / in-place write / exit 1 when `--check` would change a file |

## Formatting rules

### Instance lines

Align columns within a contiguous block of instance lines:

```spice
* before
R1 in out 1k
C1 out 0 1u
X1 a b mycell

* after (aligned)
R1 in  out 1k
C1 out 0   1u
X1 a   b   mycell
```

Column widths come from the widest field in each column. Blank lines, comments, and directives break an alignment block.

### Continuation lines

Logical statements fold `+` continuations into one token stream, then soft-wrap at `maxLineWidth` with a fixed indent after `+`:

```spice
* before
M1 d g s b nfet W=10u L=0.18u AS=1e-12 AD=1e-12 PS=1u PD=1u

* after (maxLineWidth=40)
M1 d g s b nfet W=10u L=0.18u AS=1e-12
+  AD=1e-12 PS=1u PD=1u
```

### Directives

Dot-directives stay on their own lines. The leading keyword is cased per `keywordCase` (default **upper**); other tokens keep their spelling:

```spice
.TRAN 1u 1m
.SUBCKT buffer in out
.MODEL nfet nmos ( LEVEL=1 )
```

### Comments

- Preserve `*`, `;`, and `$` full-line comments (trim trailing whitespace only)
- Normalize spacing before inline `;` comments (`  ; …`)
- Do not treat `$` as an inline comment delimiter (HSPICE `$param` tokens stay intact)
- Do not reorder or remove comment lines

## Architecture

The formatter lives in `crates/spice-parser` (`format_source`):

1. Split the buffer into physical lines and group `+` continuations into statements
2. For contiguous instance blocks, compute per-column widths
3. Pretty-print tokens (wrap at `maxLineWidth`)
4. LSP maps old vs new text to a full-document `TextEdit`

Formatting does **not** mutate the parse tree; it is a pure text pretty-printer. Tree-sitter line kinds inform classification but field columns are tokenized from the line text.

## Configuration

| Option | Values | Default |
|--------|--------|---------|
| `indentWidth` | positive integer | 2 (LSP `tabSize` when &gt; 0) |
| `keywordCase` | upper, lower, preserve | upper |
| `alignColumns` | true, false | true |
| `maxLineWidth` | number | 120 (soft wrap with `+`) |

CLI and LSP use these defaults today. Dialect-specific formatter profiles are not shipped yet.

## CLI

```bash
pixi run format-spice -- file.cir          # print formatted text
pixi run format-spice -- --write file.cir  # rewrite in place
pixi run format-spice -- --check file.cir  # exit 1 if not formatted
```

Equivalent direct invocation: `cargo run -p spice-lsp -- format …`.

## Testing

Golden-file tests in `crates/spice-parser/tests/format/`:

```
input.cir  →  format  →  compare to expected.cir
```

Cases cover instance alignment, directives, comments, and wrap. Each case also asserts idempotence. LSP stdio tests advertise `documentFormattingProvider` and check a formatting round-trip.

## Related

- [Architecture](4_architecture.md) — FormatterEngine placement
- [LSP features](5_lsp-features.md) — `textDocument/formatting` capability
