module.exports = grammar({
  name: "tspp_bytecode",

  word: ($) => $.identifier,

  extras: () => [/[ \t\f\uFEFF\u2060\u200B\u00A0]/],

  rules: {
    source_file: ($) =>
      repeat(choice($.function_declaration, $.function_definition, $.line_comment, $._newline)),

    function_declaration: ($) =>
      prec.right(seq(
        field("keyword", $.function_keyword),
        field("name", $.function_name),
        choice(seq($.line_comment, $._newline), optional($._newline))
      )),

    function_definition: ($) =>
      prec.right(seq(
        field("keyword", $.function_keyword),
        field("name", $.function_name),
        "{",
        repeat(choice($.label, $.instruction, seq($.line_comment, $._newline), $._newline)),
        "}",
        optional($._newline)
      )),

    label: ($) =>
      prec(
        2,
        seq(
          field("name", $.label_identifier),
          $.colon,
          choice(
            seq(optional($.line_comment), $._newline),
            field("instruction", $.instruction)
          )
        )
      ),

    instruction: ($) =>
      prec(
        1,
        seq(
          field("opcode", $.opcode),
          optional(field("operands", $.operands)),
          optional($.line_comment),
          $._newline
        )
      ),

    operands: ($) =>
      repeat1(
        choice(
          $.register_span,
          $.register_identifier,
          $.function_identifier,
          $.label_identifier,
          $.type_identifier,
          $.layout_identifier,
          $.allocation_identifier,
          $.dynamic_identifier,
          $.global_identifier,
          $.float_literal,
          $.integer_literal,
          $.boolean_literal,
          $.list,
          $.tuple,
          $.type_arguments,
          $.block,
          $.identifier,
          $.arrow,
          $.fat_arrow,
          $.comma,
          $.pipe,
          $.star,
          $.equal,
          $.colon
        )
      ),

    list: ($) => seq("[", optional($.operands), "]"),

    tuple: ($) => seq("(", optional($.operands), ")"),

    type_arguments: ($) => seq("<", optional($.operands), ">"),

    block: ($) => seq("{", optional($.operands), "}"),

    register_span: ($) =>
      prec(
        3,
        seq(
          field("start", $.register_identifier),
          $.colon,
          field("end", $.register_identifier)
        )
      ),

    function_keyword: () => "function",

    boolean_literal: () => token(prec(3, choice("true", "false"))),

    register_identifier: () => token(prec(2, /r[0-9]+/)),

    function_identifier: () => token(prec(2, /f[0-9]+/)),

    label_identifier: () => token(prec(2, /b[0-9]+/)),

    type_identifier: () => token(prec(2, /t[0-9]+/)),

    layout_identifier: () => token(prec(2, /l[0-9]+/)),

    allocation_identifier: () => token(prec(2, /a[0-9]+/)),

    dynamic_identifier: () => token(prec(2, /d[0-9]+/)),

    global_identifier: () => token(prec(2, /g[0-9]+/)),

    function_name: () => /[A-Za-z_][A-Za-z0-9_.]*/,

    opcode: () => /[A-Za-z_](?:[A-Za-z_.][A-Za-z0-9_.]*)?/,

    identifier: () => /-?[A-Za-z_][A-Za-z0-9_.]*/,

    float_literal: () =>
      token(
        prec(
          2,
          /-?(?:[0-9][0-9_]*\.[0-9_]+(?:[eE][+-]?[0-9][0-9_]*)?|[0-9][0-9_]*[eE][+-]?[0-9][0-9_]*)/
        )
      ),

    integer_literal: () => token(prec(1, /-?[0-9][0-9A-Za-z_]*/)),

    line_comment: () => token(seq("//", /[^\r\n]*/)),

    arrow: () => "->",

    fat_arrow: () => "=>",

    comma: () => ",",

    pipe: () => "|",

    star: () => "*",

    equal: () => "=",

    colon: () => ":",

    _newline: () => /\r?\n/,
  },
});
