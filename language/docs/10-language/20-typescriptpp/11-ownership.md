---
title: Ownership
description: Managed, owned, borrowed, or raw.
---

# Ownership

| Capability | Meaning |
|------------|---------|
| `Copy` | Value can be duplicated implicitly without changing ownership responsibilities. |
| `Clone` | Code can explicitly create another value, possibly by running code or allocating. |

```tspp
import { Copy, Drop } from "tspp:memory";

struct Plain { value: int32; }
struct Guard implements Drop {
    drop(&exclusive this): void {}
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
-  access (readonly, mutable, immutable, or exclusive)
-  region: space (local or shared) + lifetime

- Value Types, wooo
- various ways to model value types, most complete and natural is "class" vs "struct" (conceptually)
- with move semantics
- bare `T` just means whatever the default form is. preserve TS behavior
- reference types are reference types, value types are value types
- `^T`, `T`, `&T`, `*T`, ...
- `Owned<T>` spells `^T`

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
| `&T` | borrowed access | mutable | no |
| `&readonly T` | borrowed readonly access | readonly | no |
| `&immutable T` | borrowed readonly access that excludes writers | readonly | yes, from owned storage |
| `&exclusive T` | borrowed access that excludes every other access | mutable | yes, from owned storage |
| `*T` | inert unchecked pointer | unchecked | unchecked |

- the owner keeps the value alive and destroys it when the owner's own lifetime ends
- ownership, access, and region compose freely
- plain `T` always preserves the TypeScript-shaped default: structs and other value types are direct values, while classes and other reference types are (local) managed aliases

```tspp src/ownership.tspp
class User {}

const managed: User = new User();
const owned: ^User = new User();
const borrowed: &User = &managed;
const raw: *User = borrowed;
```

- ownership forms qualify the object and reference forms are values: `^T` and the bare class default are the object, while `&T` and `*T` are values of their own
- `&User` and `&^User` therefore borrow the object, and `&&T` stays a borrow of a borrow
- a string or bigint literal is an object in the constant image, which every space reads; a module binding is a local static slot
- an elided constructor receiver is managed-only: `this` borrows the fresh heap block and may escape as the handle, so `new` of that class in the owned form is rejected (`receiver-not-assignable`)
- a dual-use class writes a borrow receiver like `constructor(&exclusive this)`, and escaping that `this` is a borrow error
- a class without a written constructor constructs in either form
- a borrow of a class object has the object as its referent on every rung, so `&exclusive T` and `&exclusive ^T` are one borrow
- forms reduce to one layer: `^^T` collapses to `^T`
- the ladder for a value type is the value, `^T`, and a borrow; boxing a value on the heap is a library class (`Box<T>`)

- a handle grants a borrow by reinterpretation, and the object behind it is aliased
- it grants `&readonly` and `&` always, `&immutable` while no variant reachable through inline values and unique references holds a non-Copy payload an alias could retag, and `&exclusive` while every inline component copies so nothing moves out under an alias (`borrow-access-strengthening` otherwise)
- a fresh allocation grants every rung
- a callee trusts the rung it takes, which is sound because a handle grants only what no alias can exploit
- `^T` of a class is the object inline in its owner, and escaping it to the managed form moves it into a fresh heap block (`new.complete`)
- an elided `this` method is managed-only and is unavailable on an owned object (`receiver-not-assignable`), so a dual-use class writes `&this`, `&readonly this`, or `&exclusive this` receivers

- storage reached without crossing a managed reference is owned storage: frame locals, globals, unique pointees, and the referent of every borrowed parameter
- storage below a managed reference is managed storage

- owned storage frees when its owner ends at scope exit, overwrite, or move, and the borrow checker proves that no borrow outlives that
- managed storage frees when the collector finds nothing reaching it, and a reader through an alias may see a stale value that is always valid for its type
- for value types, and for an object in owned storage like `^User`, the storage is the value, so the promise covers it; raw pointers point at storage the same way
- managed handles, aliasable borrows, and immutable borrows are `Copy`; every read of any other borrow reborrows it
- `Copy` extends `Clone`; `clone(&immutable this): ^this` clones the stored value, `toOwned(&immutable this): this.Owned` creates an owner for a borrowed referent

- a class declares its space: `class` is local, `shared class` is shared, and every handle to it lives there
- a handle carries no place of its own, so `T` is the handle in every position and there is no written managed form
- `SharedSafe` decides what shared storage holds: scalars, owned values whose graph reaches no local handle, handles to shared classes, and synchronization types
- a `shared class` field and a `shared` binding must be `SharedSafe`
- a value enters shared storage as an owned move (`^string` into a shared field) or as a handle to a shared class; a local handle never enters shared storage
- a value crosses workers when it is `SharedSafe` or moves as a unique owner
- a value type lives inline and takes its container's space; `implements !SharedSafe` marks a value type that never enters shared storage, and `shared const point: Point` places the binding
- the constant image is read from every space and the collector skips it by address; no type names it
- transferring a non-Copy owner into managed storage consumes it; unions keep every alternative's form without boxing and are `Copy` when every alternative is
