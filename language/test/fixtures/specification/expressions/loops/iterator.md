# Iterators

Iteration over values and keys.

## for of

### for of loops over arrays

> For-of loops iterate over array values.

```ds
for (const value of [1, 2, 3]) {
    value;
}
```

## for in

### for in loops over object keys

> For-in loops iterate over object keys.

```ds
let target = { a: 1, b: 2 };
for (const key in target) {
    key;
}
```

### for of rejects non iterable values

> For-of loops reject values without iteration semantics.

```ds
for (const value of 1) {
    value;
}
```

- contains: not iterable

### for in keys are strings

> For-in loop keys are typed as strings.

```ds
let target = { a: 1, b: 2 };
for (const key in target) {
    key satisfies string;
}
```

### for of values preserve array element types

> For-of loop values use the iterated array element type.

```ds
for (const value of [1, 2, 3]) {
    value satisfies int32;
}
```

### for in rejects non-object values

> For-in loops reject primitives without enumerable key spaces.

```ds
for (const key in 1) {
    key;
}
```

- contains: not iterable

### for in keys stay string typed across union object sources

> For-in keys remain string typed for union object sources.

```ds
let target: { a: int32 } | { b: int32 } = { a: 1 };

for (const key in target) {
    key satisfies string;
}
```

### for of values preserve element types through renamed re-exports

> For-of loops preserve element types through renamed re-exported iterable producers.

```ds:source.ds
export function make_values(): int32[] {
    return [1, 2, 3];
}
```

```ds:index.ds
export { make_values as values } from "./source";
```

```ds:main.ds
import { values } from "./index";

for (const value of values()) {
    value satisfies int32;
}
```

### for of rejects unions containing non-iterable members

> For-of loops reject unions when any member is not iterable.

```ds
declare const value: int32[] | int32;

for (const item of value) {
    item;
}
```

- contains: not iterable
