; comments

(line_comment) @comment

; declarations

(function_keyword) @keyword

(function_name) @function

(label_identifier) @label

; instructions

(opcode) @function.builtin

((opcode) @keyword.control
  (#match? @keyword.control "^(await|branch|check\\.|invoke\\.|jump|panic|return|switch|trap|unreachable|yield)"))

; identities

(register_identifier) @variable

(function_identifier) @function

(type_identifier) @type

[
  (layout_identifier)
  (allocation_identifier)
  (dynamic_identifier)
  (global_identifier)
] @constant

; literals

(boolean_literal) @boolean

[
  (integer_literal)
  (float_literal)
] @number

((identifier) @constant.builtin
  (#match? @constant.builtin "^(Infinity|-Infinity|default)$"))

; punctuation

[
  (arrow)
  (fat_arrow)
  (pipe)
  (star)
  (equal)
] @operator

[
  (comma)
  (colon)
] @punctuation.delimiter
