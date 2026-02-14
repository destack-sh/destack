module.exports = grammar({
  name: "mir",

  word: ($) => $.identifier,

  extras: () => [/[\s\uFEFF\u2060\u200B\u00A0]/],

  rules: {
    source_file: ($) => repeat($._token),

    _token: ($) =>
      choice(
        $.line_comment,
        $.block_comment,
        $.function_keyword,
        $.terminator_keyword,
        $.memory_keyword,
        $.arrow,
        $.symbol_identifier,
        $.ssa_identifier,
        $.block_identifier,
        $.number_literal,
        $.string_literal,
        $.identifier,
        $.punctuation,
        $.operator,
        $.unknown
      ),

    line_comment: () => token(seq("//", /.*/)),

    block_comment: () => token(seq("/*", /[^*]*\*+([^/*][^*]*\*+)*/, "/")),

    function_keyword: () => "function",

    terminator_keyword: () =>
      choice("return", "jump", "branch", "switch", "yield", "check", "unreachable"),

    memory_keyword: () => choice("managed", "borrowed", "owned", "raw"),

    arrow: () => "->",

    symbol_identifier: () => /@[A-Za-z_][A-Za-z0-9_.]*/,

    ssa_identifier: () => /v[0-9]+/,

    block_identifier: () => /block[0-9]+/,

    number_literal: () =>
      /[+-]?\d(?:[\d_]*\d)?(?:\.(?:\d(?:[\d_]*\d)?))?(?:[eE][+-]?\d(?:[\d_]*\d)?)?(?:[iu](?:8|16|32|64|128))?/,

    string_literal: () => token(choice(seq('"', repeat(choice(/[^"\\\n]+/, /\\./)), '"'), seq("'", repeat(choice(/[^'\\\n]+/, /\\./)), "'"))),

    identifier: () => /[A-Za-z_][A-Za-z0-9_.]*/,

    punctuation: () => /[(){}\[\],:;]/,

    operator: () => /[=<>+\-*/%!&|^~.]+/,

    unknown: () => /[^\s]/,
  },
});
