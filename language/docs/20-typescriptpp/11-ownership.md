---
title: Ownership
description: Managed, owned, borrowed, or raw.
---

# Ownership

| Capability | Meaning |
|------------|---------|
| `Copy` | Value can be duplicated implicitly without changing ownership responsibilities. |
| `Clone` | Code can explicitly create another value, possibly by running code or allocating. |

```ds
import { Copy, Drop } from "destack:memory";

struct Plain { value: int32; }
struct Guard implements Drop {
    drop(&this): void {}
}

declare function duplicate<T: Copy>(value: T): void;
declare const plain: Plain;

duplicate(plain);
```

- TypeScript, following JS, has no real way to control memory shapes or allocations directly
- of course, most serious web programmers think about hidden class caches and all the brilliantly engineered details of the popular JS engines to keep their software reasonably fast
- (asm.js, yes, but, no.)
- we can trivially restrict to closed shapes, which buys us predictable layouts

- fixed static shapes (post type algebra solving) .. that's already fine for 80% of use cases, basically what C# / JVM / Go-ish are
- but sometimes we want even more: proper value types, borrowing, pointers (gasp)

- there are basically four axes to model for memory, and TS++ supports them explicitly:
- (with the defaults being TS shaped as always)
-  owneship (managed, owned, borrowed, or raw)
-  access (readonly or mutable)
-  region: space/place (local, shared, inline, or another defined space) + lifetime

- Value Types, wooo
- various ways to model value types, most complete and natural is "class" vs "struct" (conceptually)
- with move semantics
- bare `T` just means whatever the default form is. preserve TS behavior
- reference types are reference types, value types are value types
- `^T`, `T`, `&T`, `*T`, ...
- Managed<T>, Owned<T>, ...

- managed vs owned bridge
- can always do owned into managed (same heap)
- classes are managed by default
- even when the rvalue is owwned
- this is to preserve the key feeling of e.g. Arrays and such, while enabling full owned no-managed where desired

- references are safe and sound, statically guaranteed

| Form | Meaning | Access | Exclusive |
| --- | --- | --- | --- |
| `T` | direct value for value types, managed reference for reference types | mutable | depends on representation |
| `^T` | uniquely owned value | mutable | yes |
| `&T` | borrowed access | mutable | on owned storage, by the borrow check |
| `&readonly T` | borrowed readonly access | readonly | no |
| `*T` | inert unchecked pointer | unchecked | unchecked |

- the owner keeps the value alive and destroys it when the owner's own lifetime ends
- ownership, access, placement, and lifetime compose independently and normalize through `Managed`, `Owned`, `Borrowed`, `Raw`, and `Placed`
- plain `T` always preserves the TypeScript-shaped default: structs and other value types are direct  values, while classes and other reference types are (local) managed aliases

```ds:src/ownership.ds
class User {}

const managed: User = new User();
const owned: ^User = new User();
const borrowed: &User = &managed;
const raw: *User = *borrowed;
```
