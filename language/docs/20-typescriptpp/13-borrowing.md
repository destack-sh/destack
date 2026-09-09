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

- a `&T` borrow of owned storage is exclusive by structural proof, as in Rust; borrows of managed storage may alias, and writes through them stay safe because managed memory is only reclaimed at parks, so a value replaced through an alias stays alive while a borrow reaches into it

- borrows into managed storage are traced by address on the non-moving heap, so the objects they reach into stay alive across parks until the borrow's last use

- owned and borrowed sources may remain live across `await` and `yield` (a borrow rooted in local managed storage must end at the next suspension point)
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
| local managed `T` | `&T` / `&readonly T` / `&exclusive T` | exclusive needs no overlapping loan; none may cross [suspension](/docs/language/typescriptpp/borrowing/) |
| owned `^T` | `&T` / `&readonly T` / `&exclusive T` | owned sources may also cross suspension |
| shared owned `shared ^T` | `shared &T` / `shared &readonly T` / `shared &exclusive T` | owned storage stays unique even in shared space because it still has one owner |
| shared managed `shared T` | `shared &T` / `shared &readonly T` | non-exclusive writes remain limited to overwrite-stable places; synchronization remains separate |
| shared managed `shared T` | `shared &exclusive T` | never: independent Workers prevent exclusive execution |

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
replace(localUser);  // implicit `local User` to `local &exclusive User`
replace(sharedUser); // ERROR: shared managed storage cannot grant exclusive access
```

The explicit `S` is what lets these declarations accept both local and shared borrows.
A declaration written only as `inspect(value: &readonly User)` accepts a local borrow, like any other bare free-function parameter.
Borrows can "weaken" (downgrade into a more restrictive form) but cannot be upgraded:

| From | To | Notes |
| --- | --- | --- |
| `&exclusive T` | `&T` / `&readonly T` | temporary reborrow that suspends the exclusive loan |
| `&T` | `&readonly T` | readonly reborrow |
| managed reference `T` | `^T` | never: managed ownership cannot become unique ownership |
| `&T` | `T` / `^T` | never: borrowed access does not own the value |
