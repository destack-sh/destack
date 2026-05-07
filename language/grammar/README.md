# grammar

Decidedly non-normative tree-sitter grammars for Destack (and our MIR).
These grammars are optimized for editor interaction and parser throughput, not full language correctness (!).

We maintain two grammars:

| Grammar | Purpose |
| --- | --- |
| `tree-sitter-destack` | Hard-fork of the TSX grammar, adjusted for Destack source code |
| `tree-sitter-mir` | Grammar for MIR editor highlighting |

## Validation

Run these from the repository root.

```sh
# corpus routing
just -f language/justfile test-grammar-corpus-routing

# compatibility and language coverage
just -f language/justfile test-grammar-typescript-tsx
just -f language/justfile test-grammar-destack
just -f language/justfile test-grammar-mir

# coverage
just -f language/justfile test-grammar-destack-node-coverage

# full stack
just -f language/justfile test-grammar
```

## Profiling

Use these when comparing the Destack overlay against the TSX baseline.

```sh
node language/grammar/scripts/check-overlay.mjs
node language/grammar/scripts/profile-grammar-delta.mjs --target destack --baseline tsx --top 30
just -f language/justfile profile-grammar-delta
```
