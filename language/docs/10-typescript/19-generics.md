---
title: Generics
description: Stay the same basically.
---

# Generics

- trivial syntactic cleanup: `T extends string` -> `T: string`
- stay the same basically
- in, out, in out, measured variance

- monomorph or not to monomorph
- how far? do we monomorph refs?
- JVM / CLR -> Go -> Rust / C++
- build vs release mode

- where clauses
- where clauses on members and extensions
- `where` clauses accept interface bounds, associated member bounds, and static equality constraints
-
any (trivially) statically decidable predicate
- `foo<const T: isize>() where T > 5`
- `const N: Domain` works like in Rust, *not* const like in TS (just use `as const`)

- mutable arrays are invariant, which is the only sound option (via the general variance measurement rules)
- readonly array views are covariant, and explicit copies may widen element values
- managed values follow derived variance under aliasing; owned and readonly storage may be covariant; mutable borrows and raw pointers are exact

```ds:src/generics.ds
function identity<T>(value: T): T {
    value
}
```

- obviously infer x and template inference and all that still works
- also partial generics / partial application
- explicit _ holes and inference
- like `Array<_>`
- or `^_`
- or ``

## Constraints

```ds:src/constraints.ds
function collect<T, C>(values: T[]): C where C: FromIterator<T> {
    values.intoIterator().collect()
}
```
