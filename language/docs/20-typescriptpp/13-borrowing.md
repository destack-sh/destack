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

- an `&exclusive T` borrow of owned storage is truly exclusive, as in Rust; `&T` / `&readonly T` borrows may alias, and borrows of managed storage always may, and writes through them stay safe because managed memory is only reclaimed at parks, so a value replaced through an alias stays alive while a borrow reaches into it

- borrows into managed storage are traced by address on the non-moving heap, so the objects they reach into stay alive across parks until the borrow's last use

- owned and borrowed sources may remain live across `await` and `yield`; frames live in world memory, so suspension adds no rule of its own
- borrow into shared storage are always readonly

```ds:src/borrowing.ds
function write(storage: &exclusive uint8[], index: isize, value: uint8): void {
    storage[index] = value;
}
```

## Conversions

Reference conversions follow directly from the four memory dimensions, and borrowing from a live place works whenever the requested [loan rules](/docs/language/typescriptpp/borrowing/) hold:

| Source | Borrow | Notes |
| --- | --- | --- |
| local managed `T` | `&T` / `&readonly T` | managed storage grants at most `&`; it can never prove immutability or exclusion |
| owned `^T` | `&T` / `&readonly T` / `&immutable T` / `&exclusive T` | one owner makes exclusion and immutability provable; exclusive needs no overlapping loan |
| shared owned `shared ^T` | `shared &readonly T` / `shared &immutable T` | owned storage stays unique even in shared space because it still has one owner; mutation needs a synchronized guard |
| shared managed `shared T` | `shared &readonly T` | ordinary shared access is readonly; synchronization remains separate |
| shared managed `shared T` | `shared &T` / `shared &exclusive T` | never without a guard: independent Workers prevent exclusive execution |

When a managed or owned value appears where a borrow is required, the compiler inserts a borrow coercion whose lifetime and placement are inferred from the source and use:

```ds
class User {}

declare function inspect<const S: Space>(
    value: Placed<&readonly User, S>,
): void;
declare function modify<const S: Space>(
    value: Placed<&User, S>,
): void;
declare function replace<const S: Space>(
    value: Placed<&exclusive User, S>,
): void;

declare const localUser: local User;
declare const sharedUser: shared User;

inspect(localUser);  // implicit `local User` to `local &readonly User`
inspect(sharedUser); // implicit `shared User` to `shared &readonly User`
modify(localUser);   // implicit `local User` to `local &User`
modify(sharedUser);  // implicit `shared User` to `shared &User`
replace(localUser);  // ERROR: managed storage grants at most `&`; an owned `^User` can
replace(sharedUser); // ERROR: shared managed storage cannot grant exclusive access
```

The explicit `S` is what lets these declarations accept both local and shared borrows.
A declaration written only as `inspect(value: &readonly User)` accepts a local borrow, like any other bare free-function parameter.
Borrows can "weaken" (downgrade into a more restrictive form) but cannot be upgraded:

| From | To | Notes |
| --- | --- | --- |
| `&exclusive T` | `&T` / `&immutable T` / `&readonly T` | temporary reborrow that suspends the exclusive loan |
| `&immutable T` | `&readonly T` | readonly reborrow; `&T` and `&immutable T` are incomparable |
| `&T` | `&readonly T` | readonly reborrow |
| managed reference `T` | `^T` | never: managed ownership cannot become unique ownership |
| `&T` | `T` / `^T` | never: borrowed access does not own the value |

## Access

- four rungs, basically 2x2 of access and aliasing: `&readonly` < `&` / `&immutable` < `&exclusive`; `&T` is mutable and aliasable
- `&immutable T` excludes overlapping writes, `&exclusive T` excludes overlapping reads and writes; both are provable only from owned storage
- on a class value those two rungs borrow the slot holding the handle, the only place that can keep the promise; `&readonly T` and `&T` borrow the object
- a signature states the guarantee its body assumes and the caller proves it, so an `&exclusive this` method is unavailable through a managed root
- reborrows weaken: exclusive to mutable or immutable, both to readonly; parent restrictions last through the reborrow

## Unions

- a whole-union borrow is always fine; borrowing one case's inline payload needs `&immutable` or `&exclusive` on the union, so it exists only from owned roots
- through a managed root a case payload must be `Copy` and is read by value; non-Copy inline payloads in managed storage support whole-value borrow and replacement only
- a narrowing of a managed place ends at any call, `await`, `yield`, or store through a reference; a stale use is rejected statically, stricter than tsc
- narrow a local copy to keep a narrowing across calls; locals whose address never escapes stay narrowed
