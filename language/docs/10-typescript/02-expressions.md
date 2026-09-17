---
title: Expressions
description: Expressions as values.
---

# Expressions

TS++ keeps all TS expressions forms - okay, removing some legacy weirdness like `with`, `delete`, and sequence expressions - but supports new ones in a more modern expression-oriented style.

- remove some legacy weirdness
- "expressions as values"
- errors as values (Result)
- pattern matching
- (no weird switch fallthrough)

- almost every statement form is also an expression; the final expression without a trailing semicolon becomes the value of its enclosing block
- `do { ... }` makes a block expression explicit where a bare brace would be ambiguous with an object or statement block

```ds:src/values.ds
const service = "relay";
let deliveries = 0;

deliveries += 1;
```

## Using

- using / async using (like TC39 proposal)
- Dispose / AsyncDIspose
- `using` accepts `Dispose | null | undefined`
- `await using` accepts `AsyncDispose | Dispose | null | undefined` and falls back to synchronous disposal

- vs Drop
- resources are disposed in reverse declaration order on fallthrough, `return`, `break`, `continue`, and `?`
- loop-form `using` disposes the resource after every iteration
- `Drop` follows value lifetime and manages memory-shaped finalization; `using` follows lexical scope and manages files, locks, sockets, transactions, and similar resources

## Operators

TypeScript++ extends TypeScript operators with typed overloads and some additional precision.
Logical operators (`&&`, `||`, `??`), optional chaining, assignment, and strict identity (`===`, `!==`) are not (directly) overloadable, as usual, and same for increment (`++`) and decrement (`--`).
Compound assignment operators like `+=` are desugared into their component operations (`+` and `=`), and are thus indirectly overloadable.

## Block Expressions

TS++ supports "expressions as values" where (almost) all statements are expressions that produce values, and the last expression (no trailing `;`) becomes the value of the overall expression.

```ds
declare const condition: boolean;
declare function computeA(): int;
declare function computeB(): int;

const result = if (condition) {
    computeA()
} else if (condition) {
    computeB()
} else {
    computeB()
};

function add(a: int, b: int): int {
    a + b // implicit return
}
```
