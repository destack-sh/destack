(comment)+ @comment.around

([
  (function_declaration)
  (generator_function_declaration)
  (function_expression)
  (generator_function)
  (method_definition)
]
  body: (statement_block
    "{"
    (_)* @function.inside
    "}")) @function.around

(arrow_function
  body: (statement_block
    "{"
    (_)* @function.inside
    "}")) @function.around

(arrow_function
  body: (_) @function.inside) @function.around

([
  (function_signature)
  (declare_function_signature)
] @function.around)

([
  (class_declaration)
  (abstract_class_declaration)
  (struct_declaration)
  (extension_declaration)
]
  body: (class_body
    "{"
    (_)* @class.inside
    "}")) @class.around

(interface_declaration
  body: (interface_body
    "{"
    (_)* @class.inside
    "}")) @class.around

(enum_declaration
  body: (enum_body
    "{"
    (_)* @class.inside
    "}")) @class.around

(type_alias_declaration) @class.around
