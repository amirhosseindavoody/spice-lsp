/**
 * Minimal Ngspice-oriented SPICE netlist grammar for spice-lsp MVP.
 * Line-oriented: comments, dot-directives, instance lines, continuations.
 */

module.exports = grammar({
  name: "spice",

  extras: ($) => [/\s/],

  rules: {
    source_file: ($) => repeat($._line),

    _line: ($) => choice($.comment_line, $.continuation_line, $.dot_directive_line, $.instance_line),

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
