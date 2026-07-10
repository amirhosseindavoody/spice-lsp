/**
 * Line-oriented SPICE netlist grammar for spice-lsp.
 * Recognizes comments, dot-directives, instance lines, `+` continuations,
 * and bare numeric rows used inside HSPICE `.DATA` … `.ENDDATA` blocks
 * (those rows do not require a leading `+`).
 */

module.exports = grammar({
  name: "spice",

  extras: ($) => [/\s/],

  rules: {
    source_file: ($) => repeat($._line),

    _line: ($) =>
      choice(
        $.comment_line,
        $.continuation_line,
        $.dot_directive_line,
        $.data_value_line,
        $.instance_line,
      ),

    comment_line: ($) =>
      token(
        choice(
          seq("*", /[^\n]*/),
          seq(";", /[^\n]*/),
          seq("$", /[^\n]*/),
        ),
      ),

    continuation_line: ($) => token(seq("+", /[^\n]*/)),

    dot_directive_line: ($) =>
      token(
        prec(
          2,
          seq(
            ".",
            /[a-zA-Z_][a-zA-Z0-9_]*/,
            repeat(/[^\n]/),
          ),
        ),
      ),

    /**
     * Numeric / engineering-value rows (e.g. inside `.DATA` blocks).
     * HSPICE allows these without a leading `+` continuation marker.
     */
    data_value_line: ($) =>
      token(
        prec(
          1,
          seq(
            // Optional sign, then digit or ".digit" so ".5" is a value, not a directive.
            /[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?(?:[TtGgMmKkUuNnPpFf]|meg|mil)?/,
            /[^\n]*/,
          ),
        ),
      ),

    instance_line: ($) =>
      token(
        prec(
          1,
          seq(
            /[A-Za-z]/,
            /[A-Za-z0-9._$:#\[\]<>-]*/,
            repeat(/[^\n]/),
          ),
        ),
      ),
  },
});
