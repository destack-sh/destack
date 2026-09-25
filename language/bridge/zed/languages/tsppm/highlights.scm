[
  (line_comment)
  (block_comment)
] @comment

[
  (declaration_keyword)
  (function_keyword)
  (memory_keyword)
] @keyword

(terminator_keyword) @keyword.control

(type_keyword) @type.builtin

(boolean_literal) @boolean

(number_literal) @number

[
  (character_literal)
  (string_literal)
] @string

(symbol_identifier) @variable

[
  (ssa_identifier)
  (local_identifier)
] @variable

(block_identifier) @label

(function_identifier) @function

[
  (arrow)
  (operator)
] @operator

(punctuation) @punctuation
