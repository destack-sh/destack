---
title: Pointers
description: Pointers
---

# Pointers

- sometimes we need stuff that cannot be statically proven.. raw pointers, *T
- arrghh yes seriously pointers in TypeScript let's go

Safe code may convert a borrow into a raw pointer, while converting back requires an unsafe context:

```ds
class User {}

let user = new User();

let borrow: &User = &user;   // default: a checked borrow
let pointer: *User = &user;  // typed as raw: an inert, unchecked pointer value
let again: &User = pointer;  // ERROR: pointers only reborrow inside @unsafe
```

- `*T` and `Raw<T>` are the same unchecked, world-relative offset; it does not retain its target and follows the world mapping across CoW forks
- `Pointer<T>` stores an unchecked native machine address; conversion between the two is explicit
