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