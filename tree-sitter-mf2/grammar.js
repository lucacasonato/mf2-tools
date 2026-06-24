module.exports = grammar({
  name: "mf2",

  extras: ($) => [/[\s\u3000]+/],

  rules: {
    source_file: ($) => $.message,

    message: ($) => choice($.complex_message, $.simple_message),

    simple_message: ($) => repeat1($._simple_part),

    _simple_part: ($) => choice($.text, $.escape, $.placeholder),

    complex_message: ($) => seq(repeat($.declaration), choice($.quoted_pattern, $.matcher_statement)),

    declaration: ($) => choice($.input_declaration, $.local_declaration),

    input_declaration: ($) => seq(
      field("keyword", ".input"),
      field("expression", $.expression)
    ),

    local_declaration: ($) => seq(
      field("keyword", ".local"),
      field("name", $.variable),
      "=",
      field("expression", $.expression)
    ),

    matcher_statement: ($) => seq(
      field("keyword", ".match"),
      repeat1(field("selector", $.variable)),
      repeat1(field("variant", $.variant))
    ),

    variant: ($) => seq(
      repeat1(field("key", choice($.wildcard, $.literal, $.identifier))),
      field("value", $.quoted_pattern)
    ),

    wildcard: () => "*",

    quoted_pattern: ($) => seq("{{", repeat($._quoted_part), "}}"),

    _quoted_part: ($) => choice($.quoted_text, $.escape, $.placeholder),

    placeholder: ($) => choice($.markup, $.expression),

    expression: ($) => seq(
      token(/\{(?![\s\u3000]*[#/])/),
      optional(field("value", choice($.variable, $.literal))),
      optional(field("annotation", $.annotation)),
      repeat(field("option", $.option)),
      repeat(field("attribute", $.attribute)),
      "}"
    ),

    annotation: ($) => choice($.function_annotation, $.private_use_annotation),

    function_annotation: ($) => seq(":", $.identifier),

    private_use_annotation: () => token(/[&^]/),

    option: ($) => seq($.identifier, "=", choice($.variable, $.literal)),

    attribute: ($) => seq("@", $.identifier, optional(seq("=", choice($.variable, $.literal)))),

    markup: ($) => choice($.markup_open, $.markup_close),

    markup_open: ($) => seq(
      token(/\{[\s\u3000]*#/),
      $.identifier,
      repeat(choice($.option, $.attribute)),
      optional("/"),
      "}"
    ),

    markup_close: ($) => seq(
      token(/\{[\s\u3000]*\//),
      $.identifier,
      repeat(choice($.option, $.attribute)),
      "}"
    ),

    literal: ($) => choice($.quoted_literal, $.number, $.identifier),

    quoted_literal: ($) => seq("|", repeat(choice($.quoted_literal_text, $.escape)), "|"),

    number: () => /-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?/,

    variable: ($) => seq("$", $.identifier),

    identifier: () => /(?:[A-Za-z_][A-Za-z0-9_.-]*:)?[A-Za-z_][A-Za-z0-9_.-]*/,

    text: () => token(prec(1, /[^\\{]+/)),
    quoted_text: () => token(prec(1, /[^\\{}]+/)),
    quoted_literal_text: () => token(prec(1, /[^\\|]+/)),

    escape: () => /\\[\\{}|]/,
  },
});
