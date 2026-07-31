
## Expression

### [ignored] Inline an immutable expression

Inlining replaces every reference and removes the declaration.

```ds main.ds
const base = 1 + 2;
      ^^^^ target
const total = base + base;
```

```query inline main.ds#target
```

```ds main.ds after
const total = 1 + 2 + (1 + 2);
```

## Literal

### Inline a literal

A literal needs no extra parentheses in the surrounding expression.

```ds main.ds
function total(): int32 {
    const offset = 10;
          ^^^^^^ target
    return offset + 1;
}
```

```query inline main.ds#target
```

```ds main.ds after
function total(): int32 {
    return 10 + 1;
}
```

### Inline from a reference

The declaration and any of its references identify the same inline operation.

```ds main.ds
const offset = 10;
const total = offset + 1;
              ^^^^^^ target
```

```query inline main.ds#target
```

```ds main.ds after
const total = 10 + 1;
```

## Precedence

### Preserve expression precedence

An initializer receives parentheses only when its new parent requires them.

```ds main.ds
function scale(left: int32, right: int32): int32 {
    const value = left + right;
          ^^^^^ target
    return value * 2;
}
```

```query inline main.ds#target
```

```ds main.ds after
function scale(left: int32, right: int32): int32 {
    return (left + right) * 2;
}
```

## Shorthand Property

### Expand a shorthand property

A shorthand property expands to preserve its property name.

```ds main.ds
const value = 1;
      ^^^^^ target
const record = { value };
```

```query inline main.ds#target
```

```ds main.ds after
const record = { value: 1 };
```

## No Edit

### Reject duplicated side effects

A call cannot be duplicated across multiple references.

```ds main.ds
function next(): int32 {
    return 1;
}

const value = next();
      ^^^^^ target
const total = value + value;
```

```query inline main.ds#target
@inline.none
```

### Reject moving an effect past another effect

Inlining must preserve the relative order of observable operations.

```ds main.ds
function next(): int32 {
    return 1;
}

function observe(): void {}

const value = next();
      ^^^^^ target
observe();
const result = value;
```

```query inline main.ds#target
@inline.none
```

### Reject moving an effect into a branch

Inlining must not make an unconditional initializer conditional.

```ds main.ds
function next(): int32 {
    return 1;
}

declare const condition: boolean;

const value = next();
      ^^^^^ target

if (condition) {
    const result = value;
}
```

```query inline main.ds#target
@inline.none
```

### Reject a reassigned binding

A binding with a later write cannot be replaced by its initializer.

```ds main.ds
let value = 1;
    ^^^^^ target
value = 2;
const result = value;
```

```query inline main.ds#target
@inline.none
```

### Reject a captured name under shadowing

Inlining must not change which declaration a captured name resolves to.

```ds main.ds
const offset = 1;
const value = offset + 1;
      ^^^^^ target

function read(): int32 {
    const offset = 2;
    return value;
}
```

```query inline main.ds#target
@inline.none
```

## Type Annotations

### Inline a typed binding

Inlining removes the declaration annotation with its binding.

```ds main.ds
const offset: int32 = 10;
      ^^^^^^ target
const total = offset + 1;
```

```query inline main.ds#target
```

```ds main.ds after
const total = 10 + 1;
```

## Object Literals

### Parenthesize an inlined object

An object initializer remains grouped before member access.

```ds main.ds
const configuration = { enabled: true };
      ^^^^^^^^^^^^^ target
const enabled = configuration.enabled;
```

```query inline main.ds#target
```

```ds main.ds after
const enabled = ({ enabled: true }).enabled;
```

## Side Effects

### Inline one use of an effectful initializer

One reference preserves the initializer's single evaluation.

```ds main.ds
function next(): int32 {
    return 1;
}

const value = next();
      ^^^^^ target
const total = value;
```

```query inline main.ds#target
```

```ds main.ds after
function next(): int32 {
    return 1;
}

const total = next();
```

## Mutable Bindings

### Inline an unwritten mutable binding

A mutable binding without any writes can still be inlined.

```ds main.ds
let offset = 10;
    ^^^^^^ target
const total = offset + 1;
```

```query inline main.ds#target
```

```ds main.ds after
const total = 10 + 1;
```

## Declarators

### Remove only one declarator

Inlining one declarator preserves its siblings.

```ds main.ds
const base = 1, total = base + 2;
      ^^^^ target
const result = base + total;
```

```query inline main.ds#target
```

```ds main.ds after
const total = 1 + 2;
const result = 1 + total;
```

## Call Arguments

### Inline a call argument

Call arguments receive the initializer expression directly.

```ds main.ds
function render(width: int32): void {}

const width = 10;
      ^^^^^ target
render(width);
```

```query inline main.ds#target
```

```ds main.ds after
function render(width: int32): void {}

render(10);
```

## Conditions

### Inline inside a condition

References inside control-flow conditions are ordinary inline sites.

```ds main.ds
const enabled = true;
      ^^^^^^^ target

if (enabled) {
    const value = 1;
}
```

```query inline main.ds#target
```

```ds main.ds after
if (true) {
    const value = 1;
}
```

## Template Literals

### Inline inside a template interpolation

Inlining preserves the interpolation around the replacement.

```ds main.ds
const name = "World";
      ^^^^ target
const message = `Hello, ${name}`;
```

```query inline main.ds#target
```

```ds main.ds after
const message = `Hello, ${"World"}`;
```

## Class Fields

### Inline inside a field initializer

A class field reference receives the initializer value.

```ds main.ds
const initial = 1;
      ^^^^^^^ target

class Counter {
    value: int32 = initial;
}
```

```query inline main.ds#target
```

```ds main.ds after
class Counter {
    value: int32 = 1;
}
```

## Destructuring

### Inline an object binding

A destructured binding becomes access through its source object.

```ds main.ds
struct Point {
    x: int32;
    y: int32;
}

const point = Point { x: 1, y: 2 };
const { x } = point;
        ^ target
const value = x + 1;
```

```query inline main.ds#target
```

```ds main.ds after
struct Point {
    x: int32;
    y: int32;
}

const point = Point { x: 1, y: 2 };
const value = point.x + 1;
```

### Inline an aliased object binding

An aliased binding uses the source property name in its replacement.

```ds main.ds
struct Point {
    x: int32;
}

const point = Point { x: 1 };
const { x: horizontal } = point;
           ^^^^^^^^^^ target
const value = horizontal + 1;
```

```query inline main.ds#target
```

```ds main.ds after
struct Point {
    x: int32;
}

const point = Point { x: 1 };
const value = point.x + 1;
```

### Preserve sibling bindings

Inlining one name removes only that binding from the shared pattern.

```ds main.ds
struct Point {
    x: int32;
    y: int32;
}

const point = Point { x: 1, y: 2 };
const { x, y } = point;
        ^ target
const value = x + y;
```

```query inline main.ds#target
```

```ds main.ds after
struct Point {
    x: int32;
    y: int32;
}

const point = Point { x: 1, y: 2 };
const { y } = point;
const value = point.x + y;
```

## External Bindings

### Reject an exported binding

Inlining cannot remove a public declaration.

```ds main.ds
export const value = 1;
             ^^^^^ target
const total = value + 1;
```

```query inline main.ds#target
@inline.none
```

### Reject an imported binding

Inlining cannot remove a declaration owned by another module.

```ds library.ds
export const value = 1;
```

```ds main.ds
import { value } from "./library.ds";
         ^^^^^ target

const total = value + 1;
```

```query inline main.ds#target
@inline.none
```

### Reject a namespace import

A namespace binding has no local initializer to inline.

```ds library.ds
export const value = 1;
```

```ds main.ds
import * as library from "./library.ds";
            ^^^^^^^ target

const value = library.value;
```

```query inline main.ds#target
@inline.none
```

## Successive Inlining

### Inline bindings across applied revisions

Each inline operation uses the source produced by the preceding edit.

```ds main.ds
const first = 1;
      ^^^^^ target:first
const firstResult = first;
const second = 2;
      ^^^^^^ target:second
const secondResult = second;
```

```query inline main.ds#target:first apply
```

```ds main.ds after
const firstResult = 1;
const second = 2;
      ^^^^^^ target:second
const secondResult = second;
```

```query inline main.ds#target:second
```

```ds main.ds after
const firstResult = 1;
const secondResult = 2;
```
