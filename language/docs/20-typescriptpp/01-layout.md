---
title: Layout
description: by default follows rust-y layout, including niche optimisation
---

# Layout

TS++ should behave as much as TS as we can physically manage while keeping sane and predictable performance _and_ behavior.
By default, we follow a somewhat Rust-y / Go-y layout, including niche optimisation from Rust for union-like types and no-object header by default in classes (unless a vtable requires it with an explicit `virtual` method).

In practice, most things work as expected from non-scripting languages.
For example, both `undefined` and `null` occupy niches in references when used as a variant type, so e.g., `User | null | undefined` is still `usize`-d (assuming `class User`).
However, unlike in most JavaScript engines, because we have real primitives and many are 64-bit values, we cannot use NaN-boxing and e.g. `number | null` needs a real variant type.

## Representation

The layout of a type is just the concrete shape the bits take on in memory, and decorators can be used to constrain the layout as needed, roughly following Rust's conventions:

| Decorator | Meaning |
| --- | --- |
| `@repr("destack")` | Use the native TS++ representation. |
| `@repr("C")` | Use the active target's C ABI layout. |
| `@repr("transparent")` | Give a single-field declaration the same ABI representation as its field. |
| `@repr("uint8")` | Use the named integer representation as an enum backing. |
| `@repr({ align: N })` | Raise the minimum aggregate alignment to `N`. |
| `@repr({ packed: true })` / `@repr({ packed: N })` | Lower the maximum field alignment, with `true` equivalent to `1`. |

```ds
@repr({ align: 64 })
struct CacheLine {
    value: uint64;
}

@repr("C", { packed: true })
struct WireHeader {
    tag: uint8;
    size: uint32;
}
```

The layout of a type can also be queried during compilation, and is filled in post monomorphisation in the IR.

| Layout Query | Result |
| --- | --- |
| `sizeOf<T>()` | The byte size of `T` as `usize`. |
| `alignOf<T>()` | The required alignment of `T` as `usize`. |
| `strideOf<T>()` | The spacing between adjacent array elements of `T` as `usize`. |

## Unions

Variant types - that is, unions - occupy niches of their payloads to reduce the size of instances whenever possible, but this does not always work.
When no niche is available or fits, variant types add the smallest possible `tag` to discriminate the cases.

Unions are laid out in declared order, and - importantly - `A | B` and `B | A` are therefore distinct types with a (free) retag conversion between them.
But differently ordered unions do differ only in invariant positions, which is stricter than TypeScript.
