; declarations

(type_alias_declaration
  name: (type_identifier) @type.definition)

(interface_declaration
  name: (type_identifier) @type.definition)

([
  (class_declaration)
  (abstract_class_declaration)
]
  name: (type_identifier) @type.definition)

(struct_declaration
  name: (type_identifier) @type.definition)

(enum_declaration
  name: (identifier) @type.definition)

(extension_declaration
  name: (type_identifier) @type.definition)

(associated_type_declaration
  name: (type_identifier) @type.definition)

(associated_const_declaration
  name: (type_identifier) @constant)

([
  (function_signature)
  (declare_function_signature)
]
  name: (identifier) @function)

([
  (method_signature)
  (abstract_method_signature)
]
  name: (_) @function.method)

([
  (enum_body)
  (enum_assignment)
  (enum_static_field)
]
  name: (_) @constant)

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

; types

(type_identifier) @type
(predefined_type) @type.builtin
(lifetime) @label

(type_parameter
  name: (type_identifier) @type.parameter)

((identifier) @type
  (#match? @type "^[A-Z]"))

(type_arguments
  "<" @punctuation.bracket
  ">" @punctuation.bracket)

(type_parameters
  "<" @punctuation.bracket
  ">" @punctuation.bracket)

; parameters

(required_parameter
  (identifier) @variable.parameter)

(optional_parameter
  (identifier) @variable.parameter)

; memory

([
  (memory_expression)
  (memory_pattern)
  (memory_type)
]
  ["&" "^" "*"] @operator)

; keywords

(type_alias_declaration
  "type" @keyword.declaration)

(type_value
  "type" @keyword)

[
  "abstract"
  "const"
  "declare"
  "enum"
  "exclusive"
  "extension"
  "final"
  "global"
  "implements"
  "interface"
  "local"
  "match"
  "module"
  "newtype"
  "override"
  "private"
  "protected"
  "public"
  "readonly"
  "satisfies"
  "shared"
  "struct"
  "using"
  "virtual"
  "where"
  "with"
] @keyword
