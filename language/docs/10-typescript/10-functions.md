---
title: Functions
description: Functions and lambdas.
---

# Functions

```ds:src/functions.ds
function add(left: int32, right: int32): int32 {
    left + right
}

const double = (value: int32) => value * 2;
```

- `FunctionPointer` for raw / think function pointers without environment

- declarations do not nest in function bodies, except `function`, `type`, and `newtype` — a nested `class`, `struct`, `enum`, `interface`, or `extension` is a compiler error
- nested `type` and `newtype` declarations may reference enclosing generics; both erase, so no instantiation identity is created

## Overloading

- dispatch and coherence
- need some .. coherent model
- first-match wins, receiver access and exclusivity requirements included
- function overloading *only* within same declaration scope (single struct, class, extension, ..)

- an elided `this` uses the declaring type's default form: class receivers are managed, value-type receivers are `&readonly this`
- dual-use methods declare their receiver: `this` takes its selected form, `&this` borrows it
- borrow qualifiers default to mutable and aliasable; a borrow can only weaken its permissions
