; Variables

(identifier) @variable

(call_expression
  function: (member_expression
    object: (identifier) @type
    (#any-of?
      @type
      "Promise"
      "Array"
      "Object"
      "Map"
      "Set"
      "WeakMap"
      "WeakSet"
      "Date"
      "Error"
      "TypeError"
      "RangeError"
      "SyntaxError"
      "ReferenceError"
      "EvalError"
      "URIError"
      "RegExp"
      "Function"
      "Number"
      "String"
      "Boolean"
      "Symbol"
      "BigInt"
      "Proxy"
      "ArrayBuffer"
      "DataView"
    )
  )
)

; Properties

(property_identifier) @property
(shorthand_property_identifier) @property
(shorthand_property_identifier_pattern) @property
(private_property_identifier) @property

; Function and method calls

(call_expression
  function: (identifier) @function)

(call_expression
  function: (member_expression
    property: [(property_identifier) (private_property_identifier)] @function.method))

(new_expression
  constructor: (identifier) @type)

(nested_type_identifier
  module: (identifier) @type)

; Function and method definitions

(function_expression
  name: (identifier) @function)
(function_declaration
  name: (identifier) @function)
(method_definition
  name: [(property_identifier) (private_property_identifier)] @function.method)
(method_definition
    name: (property_identifier) @constructor
    (#eq? @constructor "constructor"))

(pair
  key: [(property_identifier) (private_property_identifier)] @function.method
  value: [(function_expression) (arrow_function)])

(assignment_expression
  left: (member_expression
    property: [(property_identifier) (private_property_identifier)] @function.method)
  right: [(function_expression) (arrow_function)])

(variable_declarator
  name: (identifier) @function
  value: [(function_expression) (arrow_function)])

(assignment_expression
  left: (identifier) @function
  right: [(function_expression) (arrow_function)])

; Parameters

(required_parameter
  (identifier) @variable.parameter)

(required_parameter
  (_
    ([
      (identifier)
      (shorthand_property_identifier_pattern)
    ]) @variable.parameter))

(optional_parameter
  (identifier) @variable.parameter)

(optional_parameter
  (_
    ([
      (identifier)
      (shorthand_property_identifier_pattern)
    ]) @variable.parameter))

(index_signature
  name: (identifier) @variable.parameter)

(arrow_function
  parameter: (identifier) @variable.parameter)

(type_predicate
  name: (identifier) @variable.parameter)

; Special identifiers

(type_annotation) @type
(type_identifier) @type
(predefined_type) @type.builtin

(type_alias_declaration
  (type_identifier) @type)

(type_alias_declaration
  value: (_
    (type_identifier) @type))

(interface_declaration
  (type_identifier) @type)

(class_declaration
  (type_identifier) @type.class)

(extends_clause
  value: (identifier) @type.class)

(extends_type_clause
  type: (type_identifier) @type)

(implements_clause
  (type_identifier) @type)

([
  (identifier)
  (shorthand_property_identifier)
  (shorthand_property_identifier_pattern)
 ] @constant
 (#match? @constant "^_*[A-Z_][A-Z\\d_]*$"))

; Literals

(this) @variable.special
(super) @variable.special

[
  (null)
  (undefined)
] @constant.builtin

[
  (true)
  (false)
] @boolean

(comment) @comment

[
  (string)
  (template_string)
  (template_literal_type)
] @string

(escape_sequence) @string.escape

(regex) @string.regex
(regex_flags) @keyword.operator.regex
(number) @number

; Tokens

[
  ";"
  "?."
  "."
  ","
  ":"
  "?"
] @punctuation.delimiter

[
  "-"
  "--"
  "-="
  "+"
  "++"
  "+="
  "*"
  "*="
  "**"
  "**="
  "/"
  "/="
  "%"
  "%="
  "<"
  "<="
  "<<"
  "<<="
  "="
  "=="
  "==="
  "!"
  "!="
  "!=="
  "=>"
  ">"
  ">="
  ">>"
  ">>="
  ">>>"
  ">>>="
  "~"
  "^"
  "&"
  "|"
  "^="
  "&="
  "|="
  "&&"
  "||"
  "??"
  "&&="
  "||="
  "??="
  "..."
] @operator

(regex "/" @string.regex)

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
]  @punctuation.bracket

(ternary_expression
  [
    "?"
    ":"
  ] @operator
)

; Keywords
[
  "abstract"
  "as"
  "async"
  "await"
  "debugger"
  "declare"
  "default"
  "delete"
  "extends"
  "get"
  "implements"
  "in"
  "infer"
  "instanceof"
  "is"
  "keyof"
  "module"
  "namespace"
  "new"
  "of"
  "override"
  "private"
  "protected"
  "public"
  "readonly"
  "satisfies"
  "set"
  "static"
  "target"
  "typeof"
  "where"
  "using"
  "match"
  "loop"
  "comptime"
  "with"
] @keyword

[
  "const"
  "let"
  "var"
  "function"
  "class"
  "enum"
  "interface"
  "type"
  "newtype"
  "struct"
  "extension"
] @keyword.declaration

[
  "export"
  "from"
  "import"
] @keyword.import

[
  "break"
  "case"
  "continue"
  "do"
  "else"
  "for"
  "if"
  "return"
  "switch"
  "while"
  "yield"
] @keyword.control

(switch_default "default" @keyword.control)

(template_substitution
  "${" @punctuation.special
  "}" @punctuation.special) @embedded

(template_type
  "${" @punctuation.special
  "}" @punctuation.special) @embedded

(type_arguments
  "<" @punctuation.bracket
  ">" @punctuation.bracket)

(type_parameters
  "<" @punctuation.bracket
  ">" @punctuation.bracket)

(decorator "@" @punctuation.special)

(union_type
  ("|") @punctuation.special)

(intersection_type
  ("&") @punctuation.special)

(type_annotation
  (":") @punctuation.special)

(index_signature
  (":") @punctuation.special)

(type_predicate_annotation
  (":") @punctuation.special)

(public_field_definition
  ("?") @punctuation.special)

(property_signature
  ("?") @punctuation.special)

(method_signature
  ("?") @punctuation.special)

(optional_parameter
  ([
    "?"
    ":"
  ]) @punctuation.special)



(jsx_opening_element
  [
    (identifier) @type
    (member_expression
      object: (identifier) @type
      property: (property_identifier) @type
    )
  ]
)
(jsx_closing_element
  [
    (identifier) @type
    (member_expression
      object: (identifier) @type
      property: (property_identifier) @type
    )
  ]
)
(jsx_self_closing_element
  [
    (identifier) @type
    (member_expression
      object: (identifier) @type
      property: (property_identifier) @type
    )
  ]
)

(jsx_opening_element (identifier) @tag.jsx (#match? @tag.jsx "^[a-z][^.]*$"))
(jsx_closing_element (identifier) @tag.jsx (#match? @tag.jsx "^[a-z][^.]*$"))
(jsx_self_closing_element (identifier) @tag.jsx (#match? @tag.jsx "^[a-z][^.]*$"))

(jsx_attribute (property_identifier) @attribute.jsx)
(jsx_opening_element (["<" ">"]) @punctuation.bracket.jsx)
(jsx_closing_element (["</" ">"]) @punctuation.bracket.jsx)
(jsx_self_closing_element (["<" "/>"]) @punctuation.bracket.jsx)
(jsx_attribute "=" @punctuation.delimiter.jsx)
(jsx_text) @text.jsx

; destack overlay

; fallback for keywords that parse as identifiers in the tsx baseline
((identifier) @keyword
 (#any-of? @keyword
  "public"
  "protected"
  "private"
  "readonly"
  "exclusive"
  "local"
  "shared"
  "static"
  "final"
  "virtual"
  "accessor"
  "default"
  "this"
  "super"
  "package"
  "import"
  "export"
  "from"
  "const"
  "let"
  "var"
  "namespace"
  "type"
  "newtype"
  "struct"
  "class"
  "enum"
  "union"
  "interface"
  "function"
  "extension"
  "declare"
  "new"
  "delete"
  "constructor"
  "asserts"
  "extends"
  "implements"
  "satisfies"
  "abstract"
  "override"
  "instanceof"
  "where"
  "typeof"
  "null"
  "keyof"
  "infer"
  "any"
  "never"
  "void"
  "undefined"
  "as"
  "is"
  "in"
  "of"
  "using"
  "provides"
  "comptime"
  "if"
  "else"
  "match"
  "switch"
  "case"
  "do"
  "while"
  "for"
  "loop"
  "assert"
  "break"
  "continue"
  "debugger"
  "return"
  "yield"
  "try"
  "catch"
  "throw"
  "finally"
  "goto"
  "async"
  "await"
  "get"
  "set"
  "move"
  "with"))

; destack builtin and tspp numeric aliases
([
  (identifier)
  (type_identifier)
] @type.builtin
 (#match?
  @type.builtin
  "^(?:unknown|boolean|bool|character|char|string|str|number|symbol|bigint|isize|usize|int(?:\\d+)?|uint(?:\\d+)?|float(?:\\d+)?|i\\d+|u\\d+|f\\d+)$"))
