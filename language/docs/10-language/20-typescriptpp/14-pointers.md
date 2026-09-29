---
title: Pointers
description: Pointers
---

# Pointers

- sometimes we need stuff that cannot be statically proven.. raw pointers, *T
- arrghh yes seriously pointers in TypeScript let's go

Safe code may convert a borrow into a raw pointer, while converting back requires an unsafe context:

```tspp
class User {}

let user = new User();

// a checked borrow
let borrow: &User = &user;

// an inert, unchecked pointer value
let pointer: *User = &user;

// pointers only reborrow inside @unsafe
let again: &User = pointer;
//                 ^^^^^^^ error[not-assignable]: type '*User' is not assignable to type '&'static User'
```

- `*T` and `Raw<T>` are the same unchecked, world-relative offset; it does not retain its target and follows the world mapping across CoW forks
- `Pointer<T>` stores an unchecked native machine address; conversion between the two is explicit
