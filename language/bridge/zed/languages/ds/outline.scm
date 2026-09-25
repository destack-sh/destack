(enum_declaration
  "enum" @context
  name: (_) @name) @item

(type_alias_declaration
  ["type" "newtype"] @context
  name: (_) @name) @item

([
  (function_declaration)
  (generator_function_declaration)
  (declare_function_signature)
]
  "function" @context
  name: (_) @name
  parameters: (formal_parameters
    "(" @context
    ")" @context)) @item

(interface_declaration
  ["interface" "newtype"]* @context
  name: (_) @name) @item

(class_declaration
  "class" @context
  name: (_) @name) @item

(abstract_class_declaration
  "abstract" @context
  "class" @context
  name: (_) @name) @item

(struct_declaration
  "struct" @context
  name: (_) @name) @item

(extension_declaration
  "extension" @context
  name: (_) @name
  "of" @context
  target: (_) @context) @item

(extension_declaration
  "extension" @context
  "of" @context
  target: (_) @name) @item

([
  (method_definition)
  (method_signature)
  (abstract_method_signature)
]
  name: (_) @name
  parameters: (formal_parameters
    "(" @context
    ")" @context)) @item

([
  (public_field_definition)
  (property_signature)
  (associated_type_declaration)
  (associated_const_declaration)
  (enum_assignment)
  (enum_static_field)
]
  name: (_) @name) @item

(enum_body
  name: (_) @name @item)
