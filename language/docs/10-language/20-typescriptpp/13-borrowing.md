---
title: Borrowing
description: As soon as we pass and store references, we need to make sure those are safe too.
---

# Borrowing

- if we want value types and we want to pass them around, we need some way to reference them safely
- we *could* do this asthe C# way and have in / inout / out style params, which is half the solution
- but we want to be unviversal, and we want ot be safe, so if all types can contain references, just like in Rust, ... we need proper borrowing rules

- many blog posts have been penned descrribing some of the less intuitive nuances of a Rust-like borrow checker
- and indeed, even though coming out the other end does give one a new understanding, it is not a _necessary_ understanding for most jobs
- tried a bunch of things to make ownership tracking more TS-native, but ultimately,
- the Rust model really is the most widespread and commonly known
-  (inference only locally within functions, no induced generics beyond that)
- fortunately, we barely write code by hand anymore unless we want to, most use cases for this will be in libraries most users will never see, so whatever. it works, we know it works, it's safe.

- an `&exclusive T` borrow of owned storage is truly exclusive
- `&T` and `&readonly T` borrows may alias, and borrows of managed storage always may
- writes through an alias stay safe because a heap block lives while any traced reference reaches it, so a value replaced through an alias stays valid while a borrow reaches into it

- a write releases the owned storage below the written place, so it invalidates every loan reaching into that storage whatever its rung
- other loans survive a write, except `&immutable` and `&exclusive` loans overlapping the written place
- a move out invalidates every overlapping loan
- `push` while a view into a frame-owned `^Array` lives is an error, and the same code on a handle proceeds with the view reading the stale valid buffer
- every reference that enters heap storage is retained, by the store barrier or at allocation with a payload
- the retained bit is sticky, so a moved value never re-retains, and a retained block returns only through the collector
- a borrow retains nothing: a borrow of owned storage is proven by the borrow checker, a borrow below a handle reaches storage the heap already retains
- a fresh allocation aliases nothing at its constructor call, so its constructor may hold `&exclusive` while the caller still holds arguments
- a borrow takes the region of the place it borrows
- a borrow takes the space of its referent's storage, so a borrow of a local object never names shared space, the same way `'frame` never converts into `'static`
- a borrow reached through a readonly or immutable borrow lends readonly access, whatever the handle it reaches grants
- a borrow of a class object recovers the object's handle by reinterpreting the borrow
- a call argument that holds no borrow of its own borrows the places the borrows it derives from reach, so a view returned by a call conflicts only with what it was derived from
- two reference parameters alias only when both admit aliasing: an exclusive, immutable, or unique parameter's referent is disjoint from every other parameter's

- owned and borrowed sources may remain live across `await` and `yield`; frames live in world memory, so suspension adds no rule of its own
- borrows into shared storage are readonly

```tspp src/borrowing.tspp
function write(storage: &exclusive uint8[], index: isize, value: uint8): void {
    storage[index] = value;
}
```

## Conversions

Reference conversions follow directly from the four memory dimensions, and borrowing from a live place works whenever the requested [loan rules](/docs/language/typescriptpp/borrowing/) hold:

| Source | Borrow | Notes |
| --- | --- | --- |
| local class handle `T` | `&T` / `&readonly T`, `&immutable T` / `&exclusive T` by the object's type | a handle grants the aliasable rungs always and the two strong rungs only where no alias could exploit them (see access) |
| owned `^T` | `&T` / `&readonly T` / `&immutable T` / `&exclusive T` | one owner makes exclusion and immutability provable; exclusive needs no overlapping loan |
| owned `^T` in shared storage | `&readonly T` / `&immutable T` | owned storage stays unique in shared space because it still has one owner; mutation needs a synchronized guard |
| shared class handle `T` | `&readonly T` | ordinary shared access is readonly; synchronization remains separate |
| shared class handle `T` | `&T` / `&exclusive T` | never without a guard: independent Workers prevent exclusive execution |

When a managed or owned value appears where a borrow is required, the compiler inserts a borrow coercion whose region is inferred from the source and the use:

```tspp
class User {
    history: ^string[];
}
shared class Registry {
    users: ^User[];
}

declare function inspect(value: &readonly User): void;
declare function modify(value: &User): void;
declare function replace(value: &exclusive User): void;
declare function scan(value: &readonly Registry): void;
declare function edit(value: &Registry): void;

declare const user: User;
declare const registry: Registry;

inspect(user);    // implicit `User` to `&readonly User`
modify(user);     // implicit `User` to `&User`
replace(user);    // ERROR: `history` is an owned component an alias could move out; an owned `^User` can
scan(registry);   // implicit `Registry` to `&readonly Registry`, the region's space is shared
edit(registry);   // ERROR: shared storage grants `&readonly` alone
```

An elided borrow in a signature induces one hidden region, as a Rust lifetime does; its space follows the referent at each call and one body serves every space.
Borrows can "weaken" (downgrade into a more restrictive form) but cannot be upgraded:

| From | To | Notes |
| --- | --- | --- |
| `&exclusive T` | `&T` / `&immutable T` / `&readonly T` | temporary reborrow that suspends the exclusive loan |
| `&immutable T` | `&readonly T` | readonly reborrow; `&T` and `&immutable T` are incomparable |
| `&T` | `&readonly T` | readonly reborrow |
| managed reference `T` | `^T` | never: managed ownership cannot become unique ownership |
| `&T` | `T` / `^T` | never: borrowed access does not own the value |
| `&T` / `&exclusive T` of a class object | `T` | the handle is recovered from a borrow granting at least mutable access with the `'managed` extent, and an open extent is fixed to `'managed` by the recovery |

## Access

- four rungs, basically 2x2 of access and aliasing: `&readonly` < `&` / `&immutable` < `&exclusive`; `&T` is mutable and aliasable
- a rung means the same in every body:
    - `&readonly` reads
    - `&` reads and writes
    - `&immutable` also borrows non-Copy payloads in place and assumes no retag or move under it
    - `&exclusive` also moves out and replaces cases and assumes no other access
- `&immutable T` excludes overlapping writes and `&exclusive T` excludes overlapping reads and writes
- `&immutable` and `&exclusive` exclude along their root chain, and an owned root is the only chain to its storage
- a handle is a root chain that is not the only one, so through a handle the two strong rungs are granted by reinterpretation exactly where the referent's type makes an invalid read impossible
- `&immutable` through a handle needs every inline variant payload to be `Copy`, and `&exclusive` through a handle needs every inline component to be `Copy` (`borrow-access-strengthening` otherwise)
- a TS-shaped class therefore clones through its handle, and an object holding `^T | undefined` inline does not
- a borrow rooted in a handle is emitted without `noalias`; a borrow rooted in owned storage carries it
- a body that must observe no writes takes an owned root
- a borrow begins at a root: `address` of an owned place, or a reinterpretation of a handle
- a borrow never copies its referent into the frame; `Materialize` exists only to widen a constant
- on a class object every rung borrows the object
- a signature states the guarantee its body assumes and the caller proves it, so an `&exclusive this` method is unavailable through a handle of an object holding a non-Copy field
- a primitive object such as `string` decides through its library class, and an open receiver rung through a handle stays `&`
- a generic `&readonly T` never strengthens, so `T: Compare` operands are compared through `&immutable T`
- borrowing a handle of interface type yields a borrowed dynamic reference that keeps the object's witness beside its address
- reborrows weaken: exclusive to mutable or immutable, both to readonly; parent restrictions last through the reborrow
- every read of a non-`Copy` borrow reborrows its referent

## Representation

The MIR forms of `class User` and `struct Point` show what each source form costs.

- `User` is `ref<User, managed, mutable, local>` and `readonly User` is `ref<User, managed, readonly, local>`
- `^User` is `User`, and `Point`, `^Point`, and `readonly Point` are all `Point`
- every `&… User` and `&… Point` is `ref<T, borrowed, 'a, rung>` under a `<'a>` signature; lowering erases the region's space
- a borrow from a handle is a bit cast of the handle at `'managed`
- a borrow of an owned object is `address` at `'frame`
- an `&immutable` or `&exclusive` parameter keys the body on its root, owned or aliasable, so the owned root body carries `noalias`; every other parameter shares one body

## Unions

- a whole-union borrow is always fine
- no reference into an inline union payload exists below a handle, decided at the borrow site from the root chain of the place
- a `Copy` payload is always read out by value and never borrowed in place
- a non-Copy inline payload is borrowed in place only under an owned chain, and is whole-value below a handle: store, `replace`, `take`
- `replace` and `take` move a non-Copy value out of aliasable storage through `&`, the old value out and the new value in as one step with no window an alias could observe
- a field of a narrowed non-Copy payload reads as a direct load while the narrowing holds, so `holder.slot.value` works and `&readonly holder.slot` is rejected (`borrow-of-aliasable-variant`)
- overwriting a union below a handle runs the old payload's glue eagerly, which is sound because no borrow reaches into it
- `^Array<^T | undefined>` in a frame therefore borrows, moves, and drops per case, and the same array through a handle is whole-value per element and cannot clone
- a narrowing of a managed place ends at any call, `await`, `yield`, or store through a reference
- a stale use is rejected statically, stricter than TypeScript
- narrow a local copy to keep a narrowing across calls; locals whose address never escapes stay narrowed
