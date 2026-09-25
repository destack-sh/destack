(function_signature
  name: (identifier) @name) @definition.function

(declare_function_signature
  name: (identifier) @name) @definition.function

([
  (method_signature)
  (abstract_method_signature)
]
  name: (_) @name) @definition.method

([
  (class_declaration)
  (abstract_class_declaration)
]
  name: (type_identifier) @name) @definition.class

(interface_declaration
  name: (type_identifier) @name) @definition.interface

([
  (struct_declaration)
  (type_alias_declaration)
  (extension_declaration)
]
  name: (type_identifier) @name) @definition.type

(enum_declaration
  name: (identifier) @name) @definition.type

(associated_type_declaration
  name: (type_identifier) @name) @definition.type

(associated_const_declaration
  name: (type_identifier) @name) @definition.constant

(type_annotation
  (type_identifier) @name) @reference.type
