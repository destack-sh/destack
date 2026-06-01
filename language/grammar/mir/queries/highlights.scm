; Comments

[
  (line_comment)
  (block_comment)
] @comment

; Keywords

[
  (declaration_keyword)
  (function_keyword)
  (terminator_keyword)
  (memory_keyword)
] @keyword

; Types

(type_keyword) @type.builtin

; Literals

[
  (boolean_literal)
  (number_literal)
  (character_literal)
  (string_literal)
] @constant

; Names

[
  (symbol_identifier)
  (ssa_identifier)
  (block_identifier)
  (local_identifier)
  (function_identifier)
] @variable

; Syntax

[
  (arrow)
  (punctuation)
  (operator)
] @punctuation
