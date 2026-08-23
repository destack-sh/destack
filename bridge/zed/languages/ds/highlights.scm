; variables

(identifier) @variable

; properties

(property_identifier) @property
(shorthand_property_identifier) @property
(shorthand_property_identifier_pattern) @property
(private_property_identifier) @property

; function and method calls

(call_expression
  function: (identifier) @function)

(call_expression
  function: (member_expression
    property: [(property_identifier) (private_property_identifier)] @function.method))

(new_expression
  constructor: (identifier) @type)

(nested_type_identifier
  module: (identifier) @type)

; function and method definitions

(function_expression
  name: (identifier) @function)

([
  (function_declaration)
  (generator_function_declaration)
  (declare_function_signature)
  (function_signature)
]
  name: (identifier) @function)

([
  (method_definition)
  (method_signature)
  (abstract_method_signature)
]
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

; decorators

(decorator
  "@" @punctuation.special)

(decorator
  (identifier) @attribute)

(decorator
  (call_expression
    function: (identifier) @attribute))

(decorator
  (call_expression
    function: (member_expression
      property: (property_identifier) @attribute)))

; parameters

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

; types

(lifetime) @label
(type_annotation) @type
(type_identifier) @type
(predefined_type) @type.builtin

(type_alias_declaration
  name: (type_identifier) @type)

(type_alias_declaration
  value: (_
    (type_identifier) @type))

(interface_declaration
  name: (type_identifier) @type)

(class_declaration
  name: (type_identifier) @type.class)

(abstract_class_declaration
  name: (type_identifier) @type.class)

(struct_declaration
  name: (type_identifier) @type)

(enum_declaration
  name: (identifier) @type)

(extension_declaration
  name: (type_identifier) @type)

(associated_type_declaration
  name: (type_identifier) @type)

(associated_const_declaration
  name: (type_identifier) @constant)

(type_parameter
  name: (type_identifier) @type.parameter)

(extends_clause
  value: (identifier) @type.class)

(extends_type_clause
  type: (type_identifier) @type)

(implements_clause
  (type_identifier) @type)

; enum members

(enum_body
  name: (_) @constant)

(enum_assignment
  name: (_) @constant)

(enum_static_field
  name: (_) @constant)

; literals

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

; tokens

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

; keywords
[
  "abstract"
  "as"
  "async"
  "await"
  "catch"
  "debugger"
  "declare"
  "default"
  "exclusive"
  "extends"
  "final"
  "finally"
  "get"
  "global"
  "implements"
  "in"
  "infer"
  "instanceof"
  "is"
  "keyof"
  "local"
  "loop"
  "match"
  "module"
  "new"
  "of"
  "override"
  "private"
  "protected"
  "public"
  "readonly"
  "satisfies"
  "set"
  "shared"
  "static"
  "try"
  "typeof"
  "virtual"
  "where"
  "using"
  "with"
] @keyword

[
  "const"
  "let"
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

(union_type
  ("|") @punctuation.special)

(intersection_type
  ("&") @punctuation.special)

(type_annotation
  (":") @punctuation.special)

(index_signature
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
