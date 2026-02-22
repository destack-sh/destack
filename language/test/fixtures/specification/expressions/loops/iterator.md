# Iterators Basics

Basic iteration constructs.

## for of

### for of loops over arrays

> for-of loops iterate over array values.

```ds
for (const value of [1, 2, 3]) {
    value;
}
```

## for in

### for in loops over object keys

> for-in loops iterate over object keys.

```ds
let target = { a: 1, b: 2 };
for (const key in target) {
    key;
}
```

### for of rejects non iterable values

> for-of loops reject values without iteration semantics.

```ds
for (const value of 1) {
    value;
}
```

- contains: not iterable

### for in keys are strings

> for-in loop keys are typed as strings.

```ds
let target = { a: 1, b: 2 };
for (const key in target) {
    key satisfies string;
}
```

### for of values preserve array element types

> for-of loop values use the iterated array element type.

```ds
for (const value of [1, 2, 3]) {
    value satisfies int32;
}
```

### for in rejects non-object values

> for-in loops reject primitives without enumerable key spaces.

```ds
for (const key in 1) {
    key;
}
```

- contains: not iterable
