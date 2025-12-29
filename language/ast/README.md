# ast

Destack AST (Abstract Syntax Tree) definitions.
The AST is the first intermediate representation in the pipeline, produced by the parser from source text.

## Overview

The Destack AST is (mostly) a superset of TypeScript's AST, extended with Destack-specific constructs (`struct`, `newtype`, `match`, ownership modifiers, etc.). 
Valid modern TypeScript parses to a valid subset of the Destack AST; the parser handles `.ts`, `.tsx`, `.js`, `.jsx`, and `.ds` files uniformly.
(We measure conformance to this in our various [conformance tests](../test/fixtures/conformance/)).

Unlike traditional ASTs, Destack's AST preserves whitespace, comments, and formatting information (closer to a CST).
This enables the formatter and other tools to reproduce source text faithfully.
It also plays nicely into our reflection / "types as values" design (we can just forward relevant doc comments).

### Unified Expression

Destack doesn't distinguish "statements" and "expressions" at the AST level: everything is an `Expression`.
This reflects the language design where `if`, `match`, and blocks are expressions that return values. 
The `Statement` variant wraps expressions that are explicitly terminated with `;`.

```
const result = if (cond) { a } else { b };  // if is an expression
```

## Architecture

Like every IR in Destack, the AST uses a `NodeTree` arena to store nodes.
Every module / source file gets its own `NodeTree`, and each node gets a typed ID (`LocalNodeId<T>`) for type-safe access. 

```ds
const exprId: LocalNodeId<Expression> = tree.insert(expr, span)
const expr: Expression = tree.get(exprId)
const span: Span = tree.getSpan(exprId)
```

### Nodes

The highlight reel of AST nodes:

| Node Type | Examples | Purpose |
|----------|----------|---------|
| `Expression` | if, match, call, binary, let, import | All executable constructs |
| `Declaration` | class, struct, function, interface, enum | Named type/value definitions |
| `Block` | `{ ... }` | Scoped expression sequences |
| `Pattern` | destructuring, match arms | Pattern matching |
| `Parameter`, `Argument` | function params, call args | Function signatures and calls |
| `Annotation` | comments, docs, decorators, blanks | Metadata and formatting |

### Traversal

The `NodeVisitor` trait provides a visitor pattern for AST traversal. 
Override specific `visit_*` methods and call the corresponding `walk_*` function to recurse:

```ds
class MyVisitor implements NodeVisitor {
    visitExpression(tree: NodeTree, id: LocalNodeId<Expression>, expr: Expression): void {
        // custom logic here
        walkExpression(this, tree, id, expr)  // recurse into children
    }
}
```

---

## Added

- Add expression nodes for TS type forms (mirroring value counterparts where possible):
- `Expression::TypeConditional { left, right, then_type, else_type }` for `T extends U ? X : Y` (aligned with `left/right` naming)
- `Expression::TypeMapped { parameter, modifiers, value }` where `parameter` carries `name`, `constraint`, and optional `key_remap`
- `Expression::TypeIndex { left, index }` for `T[K]` (aligned with `Expression::Index`, but type-only)
- `Expression::TypeTemplateLiteral { strings, spans }` with `spans: Vec<LocalNodeId<Expression>>` for type interpolations
- `Expression::TypeImport { target, qualifier }` for `import("mod").Type`
- `Expression::TypeInfer { name, constraint }` for `infer T` bindings
- `Expression::TypePredicate { asserts, subject, target }` for `x is T` and `asserts x is T`
- `Expression::This` for `this` in both value and type contexts (avoid a separate `TypeThis`)
- Keep `TypeUnary`/`TypeBinary` for operator-like constructs:
- `TypeUnary`: `readonly`, `typeof`, `keyof`, `type`, `newtype`, `as const`, prefix `!`, postfix `!`
  - `TypeBinary`: `as`, `is`, `instanceof`, `satisfies`, `extends`, `implements`
  - `infer` and `asserts` move to `TypeInfer`/`TypePredicate`
- Add supporting structs/enums:
- `TypeMappedModifiers { readonly: TypeModifier, optional: TypeModifier }`
- `TypeModifier::Add | TypeModifier::Remove | TypeModifier::None` for `readonly/-readonly` and `?/-?`
- `TypePredicateSubject::Identifier(StringId) | This`
- Add tuple element modifiers for type tuples: extend `Argument` (or introduce `TupleElement`) to carry `readonly`, `?`, and `...` on `[readonly x?: T, ...U[]]` (likely via `BindingModifier`)
- Add `this` parameter support in signatures: extend `FunctionSignature` with an optional `this_parameter`
- Support call/construct signatures in type literals: allow `Property::Method` with `FunctionMode::Call/New` and `key: None` in type contexts
- Add `intrinsic` as a type literal for TS builtin utility types (current intrinsic aliases: `Uppercase`, `Lowercase`, `Capitalize`, `Uncapitalize`, `NoInfer`, `BuiltinIteratorReturn`)
- Update walkers/dumpers/visitors to traverse the new expression nodes and preserve spans/annotations
