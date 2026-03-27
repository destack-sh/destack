# Inline

## Inline Expression

### Inlines an immutable binding with a complex initializer

Inline should replace all references and remove the declaration.

```ds:main.ds
const base = 1 + 2;
//    ^^^^ target
const total = base + base;
```

```query inline target
```

```expected:main.ds
const total = (1 + 2) + (1 + 2);
```

## Inline Literal

### Inlines a simple literal without extra parentheses

Inline should drop the declaration and keep the literal formatting.

```ds:main.ds
function main(): int32 {
    const offset = 10;
    //    ^^^^^ target
    return offset + 1;
}
```

```query inline target
```

```expected:main.ds
function main(): int32 {
    return 10 + 1;
}
```

## Inline With Annotation

### Inlines bindings with type annotations

Inline should ignore type annotations and replace references with the initializer.

```ds:main.ds
const offset: int32 = 10;
//    ^^^^^ target
const total = offset + 1;
```

```query inline target
```

```expected:main.ds
const total = 10 + 1;
```

## Inline Object Literal

### Wraps object literals with parentheses

Inline should parenthesize object literals when replacing references.

```ds:main.ds
const config = { enabled: true };
//    ^^^^^^ target
const flag = config.enabled;
```

```query inline target
```

```expected:main.ds
const flag = ({ enabled: true }).enabled;
```

## Inline Shorthand Property

### Expands object literal shorthand properties

Inline should expand shorthand properties to preserve the property name.

```ds:main.ds
const value = 1;
//    ^^^^^ target
const obj = { value };
```

```query inline target
```

```expected:main.ds
const obj = { value: 1 };
```

## Single Reference Side Effects

### Inlines when side effects are used once

Inline should allow side effects when there is only one reference.

```ds:main.ds
function next(): int32 {
    return 1;
}

const value = next();
//    ^^^^^ target
const total = value;
```

```query inline target
```

```expected:main.ds
function next(): int32 {
    return 1;
}

const total = next();
```

## Side Effects

### Skips inlining when the initializer has side effects

Inline should avoid duplicating side effects when there are multiple references.

```ds:main.ds
function next(): int32 {
    return 1;
}

const value = next();
//    ^^^^^ target
const sum = value + value;
```

```query inline target
<none>
```

## Damaged Syntax

### Inlines after malformed call statements

Inline should still rewrite later valid bindings after malformed call statements.

```ds:main.ds
broken(,

const value = 1;
//    ^^^^^ target
const total = value + 1;
```

```query inline target
```

```expected:main.ds
broken(,

const total = 1 + 1;
```

### Inlines after bare new recovery statements

Inline should still rewrite later valid bindings after bare `new` recovery statements.

```ds:main.ds
new

const value = 1;
//    ^^^^^ target
const total = value + 1;
```

```query inline target
```

```expected:main.ds
new

const total = 1 + 1;
```

### Inlines after throw recovery statements

Inline should still rewrite later valid bindings after recovered `throw` statements.

```ds:main.ds
throw

const value = 1;
//    ^^^^^ target
const total = value + 1;
```

```query inline target
```

```expected:main.ds
throw

const total = 1 + 1;
```

## Reassignment

### Skips inlining when the symbol is reassigned

Inline should avoid inlining when the symbol is reassigned after declaration.

```ds:main.ds
const value = 1;
//    ^^^^^ target
value = 2;
const total = value + 1;
```

```query inline target
<none>
```

## Mutable Binding

### Inlines mutable bindings without reassignment

Inline should allow `let` bindings when they are never reassigned.

```ds:main.ds
let value = 1;
//  ^^^^^ target
const total = value + 1;
```

```query inline target
```

```expected:main.ds
const total = 1 + 1;
```

## Shadowing

### Skips inlining when captured symbols are shadowed

Inline should refuse to inline when the initializer would resolve to different symbols.

```ds:main.ds
const base_value = 1;
const total = base_value + 1;
//    ^^^^^ target

function main(): int32 {
    const base_value = 10;
    return total + base_value;
}
```

```query inline target
<none>
```

## Multi Declarator

### Removes only the targeted declarator

Inline should remove only the matching declarator in multi-declarator statements.

```ds:main.ds
const base = 1, total = base + 2;
//    ^^^^ target
const result = base + total;
```

```query inline target
```

```expected:main.ds
const total = 1 + 2;
const result = 1 + total;
```

## Object Destructuring

### Inlines object destructuring bindings

Inline should resolve simple object destructuring paths.

```ds:main.ds
const { name } = user;
//      ^^^^ target
const label = name + "!";
```

```query inline target
```

```expected:main.ds
const label = user.name + "!";
```

## Array Destructuring

### Inlines array destructuring bindings

Inline should resolve simple array destructuring paths.

```ds:main.ds
const [first] = items;
//      ^^^^^ target
const total = first + 1;
```

```query inline target
```

```expected:main.ds
const total = items[0] + 1;
```

## Inline In Condition

### Rewrites references inside conditionals

Inline should replace uses inside conditionals.

```ds:main.ds
const limit = 10;
//    ^^^^^ target

if (limit > 5) {
    console.log(limit);
}
```

```query inline target
```

```expected:main.ds
if (10 > 5) {
    console.log(10);
}
```

## Template Literals

### Inlines bindings inside template literals

Inline should replace references inside template literal interpolations.

```ds:main.ds
const name = user.name;
//    ^^^^ target
const label = `Hello ${name}!`;
```

```query inline target
```

```expected:main.ds
const label = `Hello ${user.name}!`;
```

## Class Field Initializers

### Inlines bindings in class field initializers

Inline should update references inside class field initializers.

```ds:main.ds
const increment = 2;
//    ^^^^^^^^^ target

class Counter {
    value: int32 = 1 + increment;
}
```

```query inline target
```

```expected:main.ds
class Counter {
    value: int32 = 1 + 2;
}
```

## Cross-Module Safety

### Skips inline when symbol is exported

Inline should refuse to inline exported bindings referenced across modules.

```ds:lib.ds
export const greeting = "hello";
//           ^^^^^^^^ target
```

```ds:main.ds
import { greeting } from "./lib.ds";

const message = greeting + "!";
```

```query inline target
<none>
```

### Skips inline for imported aliases

Inline should refuse imported aliases because the owning definition lives in another file.

```ds:lib.ds
export const value = 1;
```

```ds:main.ds
import { value as amount } from "./lib.ds";
//                ^^^^^^ target

const total = amount + 1;
```

```query inline target
<none>
```

### Skips inline for namespace imports

Inline should also refuse namespace imports because they are owned by another module.

```ds:lib.ds
export const value = 1;
```

```ds:main.ds
import * as api from "./lib.ds";
//          ^^^ target

const total = api.value + 1;
```

```query inline target
<none>
```

## Call Arguments

### Inlines references in call arguments

Inline should replace references used in positional call arguments.

```ds:main.ds
const width = 10;
//    ^^^^^ target

render(width);
```

```query inline target
```

```expected:main.ds
render(10);
```

## Destructuring Aliases

### Inlines aliased object destructuring bindings

Inline should resolve aliased object destructuring bindings to property access.

```ds:main.ds
const { value: amount } = entry;
//             ^^^^^^ target
const total = amount + 1;
```

```query inline target
```

```expected:main.ds
const total = entry.value + 1;
```

## Multi Binding Destructuring

### Skips inline for multi-binding destructuring patterns

Inline should return no edits for destructuring declarations with multiple bindings.

```ds:main.ds
const { left, right } = point;
//      ^^^^ target
const total = left + right;
```

```query inline target
<none>
```
