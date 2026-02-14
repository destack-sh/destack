(line_comment) @comment
(block_comment) @comment

(function_keyword) @keyword
(terminator_keyword) @keyword.control
(memory_keyword) @keyword

(arrow) @operator
(operator) @operator

(number_literal) @number
(string_literal) @string

(symbol_identifier) @function
(ssa_identifier) @variable
(block_identifier) @label

((identifier) @type.builtin
  (#match? @type.builtin "^(bool|void|isize|usize|i[0-9]+|u[0-9]+|f[0-9]+|fn|ref|ptr|type|vector|tensor)$"))

((identifier) @function.builtin
  (#match? @function.builtin "^(iconst|fconst|binary|unary|cast|select|load|store|struct|tuple|array|local\\.(get|set|addr)|global\\.(addr|const)|function\\.(addr|env)|managed\\.(alloc|alloc_array)|raw\\.(alloc|free|drop)|stack\\.(alloc|drop)|field\\.(get|set|addr)|element\\.(get|set|addr)|call(\\.(virtual|interface|indirect))?|intrinsic(\\.[A-Za-z_][A-Za-z0-9_.]*)?)$"))

(identifier) @variable

((punctuation) @punctuation.bracket
  (#match? @punctuation.bracket "^[(){}\\[\\]]$"))

((punctuation) @punctuation.delimiter
  (#match? @punctuation.delimiter "^[,:;]$"))
