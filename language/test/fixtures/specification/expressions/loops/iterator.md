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
