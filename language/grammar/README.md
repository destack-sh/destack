# Grammar

The canonical `tree-sitter-destack` hard-fork of TSX tree-sitter adjusted for and maintained by Destack.
Generated parser artifacts in `src/` are checked in for reproducible bindings and editor integrations.

## Corpus layout

`language/grammar/destack/test/corpus/*.txt` is the TypeScript and TSX compatibility corpus.
Those files use explicit `:language(typescript)` and `:language(tsx)` directives to avoid default-language ambiguity in the multi-grammar fork.
`language/grammar/destack/destack/test/corpus/*.txt` is the Destack-specific corpus.
`language/grammar/destack/destack/test/corpus/spec-features.txt` tracks core Destack extensions from `language/DESIGN.md` and `language/SPECIFICATION.md`, including flexible annotations, `where` clauses, `comptime` members, ranges, and `try`/`catch match`.

## Validation

Run `just -f language/justfile test-grammar-corpus-routing` to ensure corpus sections are routed to the intended grammar.
Run `just -f language/justfile test-grammar-typescript-tsx` for TS and TSX corpus compatibility.
Run `just -f language/justfile test-grammar-destack` for Destack corpus coverage.
Run `just -f language/justfile test-grammar-destack-node-coverage` to enforce coverage of all Destack-only named nodes in Destack corpus expected trees.
Run `just -f language/justfile test-grammar-specification-sweep` to parse specification markdown fixture code fences with the tree-sitter grammars and report parse errors.
Run `just -f language/justfile test-grammar-specification-sweep-positive` to fail when parser errors appear in positive specification cases.
Run `just -f language/justfile test-grammar-all` to run the full grammar validation stack.

## Profiling

```sh
node language/grammar/scripts/check-overlay.mjs
node language/grammar/scripts/profile-grammar-delta.mjs --target destack --baseline tsx --top 30
just -f language/justfile profile-grammar-delta --args="--target destack --baseline tsx --top 30"
```
