((comment) @injection.content
  (#set! injection.language "comment"))

(((comment) @_documentation
  (#match? @_documentation "(?s)^/[*][*][^*].*[*]/$")) @injection.content
  (#set! injection.language "jsdoc"))

((regex) @injection.content
  (#set! injection.language "regex"))
