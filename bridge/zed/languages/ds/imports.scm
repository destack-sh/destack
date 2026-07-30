(import_statement
  (import_clause
    [
      (identifier) @name
      (namespace_import
        (identifier) @name)
      (named_imports
        (import_specifier
          name: (_) @name
          alias: (_)? @alias))
    ])
  source: (string
    (string_fragment) @source)) @import

(import_statement
  source: (string
    (string_fragment) @source @wildcard)) @import
