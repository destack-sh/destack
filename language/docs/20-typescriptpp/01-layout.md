---
title: Layout
description: by default follows rust-y layout, including niche optimisation
---

# Layout

- so, TS++ should behave as much as TS as we can physically manage while keeping sane and predictable performance _and_ behavior
- (and something we can actually build into a good toolchain)
- by default follows rust-y layout, including niche optimisation
- null / undefined / nullish is stored in the same address (currently bit patterns just 0x0 and 0x1)

## Representation

The layout of a type is just the concrete shape the bits take on in memory.
Decorators constrain the layout as needed, following Rust's conventions:

| Decorator | Meaning |
| --- | --- |
| `@repr("destack")` | Use the native Destack representation. |
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

Layout can also be queried during compilation - available as a static term during inference - for conditional branching and storage:

| Layout Query | Result |
| --- | --- |
| `sizeOf<T>()` | The byte size of `T` as `usize`. |
| `alignOf<T>()` | The required alignment of `T` as `usize`. |
| `strideOf<T>()` | The spacing between adjacent array elements of `T` as `usize`. |
