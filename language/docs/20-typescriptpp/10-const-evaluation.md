---
title: Const Evaluation
description: Runtime at compile time.
---

# Const Evaluation

- "static" vs static
- unfortunately static is very overloaded
- static vs const vs runtime
- runtime we already know about

## Static Evaluation

- actually "static" during comptime, trivial evaluation
- static litearls
- type algebra
- one world of static terms, simple stuff like arithemetic, boolean logic, ..
- only well known collections

## Type Values

Destack represents a type as a value with `Type<T>`.
Runtime-erased `Dynamic` values retain enough concrete type identity for `is` and `instanceof` checks without retaining the structure of every type at runtime.

```ds
struct User {
    name: string;
    age: uint;
}

const userType: Type<User> = User;
```

## Const Evaluation

- const evaluation
- "runtime at compile time"
- originally envisioned something closer to Zig's comptime (or even Jai's version of it)
- originally had a comptime keyword here but was kinda confusing
- `const <expr>` and `const { ... }` for comptime evaluation
- `const function` for comptime functions that can only be called at comptile time
- cardinality is measured by usage (sort of like how variance and )
- ordinary functions may run at compile time when called from a const expression; `const function` declares that no runtime callable form exists
- const evaluation is isolated to its expression and cannot mutate outer static or global state
- const results must be serializable into an artifact (runtime pointers and handles cannot escape compilation)

```ds:src/const.ds
const size = const { 4 * 1024 };
const buffer: [uint8; size];
```
