---
title: Extensions
description: Like impl in Rust but a little broader.
---

# Extensions

- Rust has `impl` blocks for as the _sole_ mechanism for attaching members to nominal targets
- TS++ has its as an additional mechanism
- like `impl` in Rust but a little broader

## Visibility

- inherent, anonymous, named extensions
- E / T, T may be local or imported
- `extension of T`
- `extension E of T`
- `export extension of T`
- `export extension E of T`
- an inhernt extension beside its target is visible wherever the target is visible
- an anonymous extension on a foreign target is visible only in its declaring file
- a named foreign extension must be imported explicitly

## Conformance

- `extension<T> of T`: blanket extension, basically like in Rust
- a blanket target binds the object "beneath" the receiver's forms, so one `extension<S: Display> of S` serves `S`, `^S`, and every borrow of `S` (again, like in Rust)
- structural types, unions, and intersections cannot receive extensions
- coherence is program-wide: one implementation of an interface per type
- overlapping implementations of one interface for one type, including blanket overlap, are errors
- no orphan rule?
- extension members are lexical, but `implements` contributes a program-wide relation whenever its module is in the program
- global extensions considered for impls (not import order)
- member overloading only within a single declaration block (extension or itme declaration)
- also for @unsafe impls

```ds:src/extensions.ds
newtype UserId = string;

extension of UserId {
    value(this): string {
        string(this)
    }
}
```

- extension members are lexical and import-scoped
- interface implementations participate in the whole program (even if not imported/exported)
