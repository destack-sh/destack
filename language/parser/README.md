# parser

Destack lexer and parser.
Transforms source text into tokens (lexer) and tokens into an AST (parser).

## Layout

| Path | Purpose | Description |
| --- | --- | --- |
| `lex/` | Lexer | Tokenizer that converts source text into tokens. |
| `parse/` | Parser | Recursive descent parser that builds AST from tokens. |
| `tests/` | Tests | Parser tests. |
