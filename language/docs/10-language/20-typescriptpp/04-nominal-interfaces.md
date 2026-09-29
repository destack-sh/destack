---
title: Nominal Interfaces
description: Nominal interfaces (traits).
---

# Nominal Interfaces

- nominal interfaces (traits): explicit conformance, like a Rust trait
- receiver qualifiers state access requirements; associated constants can select an implementation's required access
- borrow weakening permits calls to weaker receivers; it does not establish another trait conformance

- implementations always need exact receivers, so `clone(&immutable this)` is implemented as `clone(&immutable this)`
- generated methods copy Copy union values before payload calls and require non-aliasing "protection" for non-Copy inline payload borrows
- generated methods obey ordinary borrowing rules; a handle admits `&immutable` and `&exclusive` only by the object's type (see borrowing)

```tspp
newtype interface Add<T> {
    add(a: T, b: T): T;
}
```
