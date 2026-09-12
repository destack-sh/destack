---
title: Unsafe
description: Raw pointers are inert.
---

# Unsafe

- `*T` and `Raw<T>` are the same unchecked (world-relative) pointer form
- borrowing and raw-pointer conversion preserve the MemoryMap offset
- native addresses are materialized for machine access and explicit native interop

Safe Destack code can create and carry raw pointers, because there is nothing directly unsafe about just looking at pointers.
Raw pointers are inert: they do not keep storage alive, do not participate in borrow checking, and do not prove exclusivity.

Converting a borrow to a raw pointer is still safe because it does not touch the pointed-to memory:

```ds
class User {}

let user = new User();

let borrow: &User = &user;
let pointer: *User = borrow; // OK: this only creates a raw pointer value
```

Unsafe begins when code relies on a memory invariant the compiler cannot prove:

| Operation | Example | Safe? | Why |
|-----------|---------|-------|-----|
| create or carry raw pointer values | `let pointer: *User = &user`, `pointer == other` | yes | does not touch memory |
| reinterpret raw pointer values | `pointer as *uint8`, `0x1000 as *uint8` | yes | makes no validity claim |
| wrapping address arithmetic | `raw.wrappingOffset(pointer, 4)` | yes | makes no allocation claim |
| allocation-relative pointer math | `raw.offset(pointer, 4)`, `raw.offsetFrom(pointer, origin)` | no | claims same live allocation |
| access memory through a pointer | `raw.asReference(pointer)`, `raw.read(pointer)`, `raw.write(pointer, value)` | no | bypasses borrow checking |
| build a raw slice descriptor from raw parts | `sliceFromRaw<T>(pointer, length)` | no | claims a valid region of `T` |
| raw bytes and layout tricks | `raw.copyBytes(dst, src, n)`, `raw.readVolatile(pointer)`, `raw.transmute<T, U>(value)` | no | touches or reinterprets unchecked memory |

The compiler rejects unsafe operations, like raw pointer dereferencing, outside explicit [`@unsafe` / `@safe`](/docs/language/typescript/decorators/) contexts.
