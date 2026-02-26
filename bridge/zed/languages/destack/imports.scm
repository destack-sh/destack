(import_statement
    (import_clause
        [
            (identifier) @name
            (named_imports
                (import_specifier
                    name: (_) @name
                    alias: (_)? @alias))
        ])
    source: (string (string_fragment) @source)) @import

(import_statement
    (import_require_clause
        (identifier) @name
        source: (string (string_fragment) @source)) @import)

((import_statement
    source: (string (string_fragment) @source @wildcard)) @import
 (#match? @import "^\\s*import\\s*[\"']"))
