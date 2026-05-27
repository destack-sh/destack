# Elaborate

Elaborate turns DIR into a "canonical" form by transforming and reifying ("elaborating") implicit or higher level concepts from the base DIR into "canonical DIR".

The exact responsibilities of Elaborate are unfortunately a bit fuzzy because we need to support both high-level targets like JS/TS *and* low-level AOT targets.
We try to keep most reifications / transforms the same across targets to reduce the combinatorial explosion, but in some cases it's inevitable, for example to retain some nullish coalescing behavior without complicating the JS/TS codegen backend.

## Transform

Transform rewrites (and "simplifies") structure without changing meaning.
Evaluation order stays the same and the output remains target-independent DIR.

| Module | Responsibility |
|--------|----------------|
| `block` | Normalize block expression value flow |
| `coalesce` | Normalize coalesce expression structure |
| `declarator` | Split and normalize declarations |
| `expression` | Route expression-level normalization |
| `let` | Normalize let patterns and value flow |
| `match` | Normalize match and pattern decision structure |
| `return` | Normalize implicit and explicit returns |
| `statement` | Route statement-level normalization |
| `ternary` | Normalize if/else value expressions |

## Reify

Reify makes certain abstractions explicit ("realized").
It is profile-aware and uses Analyze resolutions and profile libraries.
The output remains DIR and feeds Lower.

| Module | Responsibility |
|--------|----------------|
| `cast` | Insert and classify casts |
| `expression` | Route expression reification |
| `operator` | Reify operator resolution |
| `resolution` | Replace dynamic resolutions with explicit static branches |
| `tagged` | Reify nominal constructor and tagged expression forms |
| `tree` | Reify tree literal construction |
| `type` | Reify type-level canonical forms |

## Boundary

Elaborate consumes checked DIR and emits DIR.
It should not lower into MIR concepts.
It should not own target packaging, linking, or backend printing.

## Testing

Run these from the repository root.

```sh
cargo test -p destack_compiler elaborate
cargo test -p destack_test --test specification
just language/check-quick
just language/check-full
```
