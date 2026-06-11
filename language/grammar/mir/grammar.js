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
        $.declaration_keyword,
        $.function_keyword,
        $.terminator_keyword,
        $.type_keyword,
        $.memory_keyword,
        $.arrow,
        $.symbol_identifier,
        $.ssa_identifier,
        $.block_identifier,
        $.local_identifier,
        $.function_identifier,
        $.boolean_literal,
        $.number_literal,
        $.character_literal,
        $.string_literal,
        $.identifier,
        $.punctuation,
        $.operator,
        $.unknown
      ),

    line_comment: () => token(seq("//", /.*/)),

    block_comment: () => token(seq("/*", /[^*]*\*+([^/*][^*]*\*+)*/, "/")),

    function_keyword: () => "function",

    declaration_keyword: () => token(prec(2, choice("external", "export", "global", "type", "block", "local"))),

    terminator_keyword: () =>
      token(prec(2, choice(
        "return",
        "jump",
        "branch",
        "switch",
        "yield",
        "check",
        "panic",
        "unwind.resume",
        "trap.abort",
        "unreachable",
        "call",
        "tailCall",
        "call.indirect",
        "tailCall.indirect",
        "call.virtual",
        "tailCall.virtual",
        "call.dynamic",
        "tailCall.dynamic"
      ))),

    type_keyword: () =>
      token(prec(2, choice(
        "void",
        "boolean",
        "ref",
        "vector",
        "tensor",
        "tensorView",
        "space",
        "struct",
        "newtype",
        "atomic",
        "variant",
        "slice",
        "any",
        "isize",
        "usize",
        "typeDescriptor",
        "typeId",
        /(?:int|uint)(?:8|16|32|64|128|256)/,
        /float(?:32|64)/
      ))),

    memory_keyword: () =>
      token(prec(2, choice(
        "managed",
        "borrowed",
        "owned",
        "raw",
        "unique",
        "copy",
        "readonly",
        "const",
        "nullable",
        "undefined",
        "nullish",
        "exclusive"
      ))),

    arrow: () => "->",

    symbol_identifier: () => /@[A-Za-z_][A-Za-z0-9_.]*/,

    ssa_identifier: () => /v[0-9]+/,

    block_identifier: () => token(prec(2, /(?:block|entry|bb|b)[0-9]+/)),

    local_identifier: () => token(prec(2, /local[0-9]+/)),

    function_identifier: () => token(prec(2, /function[0-9]+/)),

    number_literal: () =>
      /[+-]?\d(?:[\d_]*\d)?(?:\.(?:\d(?:[\d_]*\d)?))?(?:[eE][+-]?\d(?:[\d_]*\d)?)?(?:(?:int|uint|float)(?:8|16|32|64|128|256)|[iuf](?:8|16|32|64|128))?/,

    boolean_literal: () => token(prec(2, choice("true", "false"))),

    character_literal: () => token(seq("'", repeat(choice(/[^'\\\n]+/, /\\./)), "'")),

    string_literal: () => token(seq('"', repeat(choice(/[^"\\\n]+/, /\\./)), '"')),

    identifier: () => /[A-Za-z_][A-Za-z0-9_.]*/,

    punctuation: () => /[(){}\[\],:;]/,

    operator: () => /[=<>+\-*/%!&|^~.]+/,

    unknown: () => /[^\s]/,
  },
});
