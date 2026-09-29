---
title: Lifetimes and Regions
description: Generalised lifetimes into regions.
---

# Lifetimes and Regions

```tspp src/lifetimes.tspp
function first<'a, T>(values: &'a readonly T[]): &'a readonly T {
    &readonly values[0]
}
```

- as soon as we pass and store references, we need to make sure those are safe too
- well wouldn't you know, lifetimes
- generalised lifetimes into regions (combine lifetime + space/place)
- `'a` names one region: an extent and the space of the storage it borrows

- stored borrows write lifetime parameters explicitly

- a borrow used to initialize a binding extends its temporary to the binding lifetime; other
temporaries live to the end of the enclosing statement

- a region is an extent and a space; access and exclusivity are independent, and joins keep sets of extent and space pairs
- a closure / coroutine carries the lifetime of every borrow it captures on its type, through callable and interface conversions alike

- elision follows the position of the elided borrow, as Rust elides lifetimes
- a signature induces one hidden region per elided borrow; nothing else is elided
- a body elides to an inference hole
- a member elides on its own signature like a free function
- a module binding borrows local storage with a static extent
- a type declaration writes its lifetimes; an elided lifetime in a declared field is an error (`elided-declaration-lifetime`)
- an elided borrow inside a type-level function type (a type alias or an interface member type) is late bound in its own binder
- a constructor is a signature term `this: &'a exclusive C` whose region is the construction's; construction matching ignores that term and the body sees `this` as the uninitialized object
- a closure elides to bound regions in its own signature binder, like a function type; only named declarations induce instance parameters
- a where clause or generic bound elides like a signature: each elided borrow induces one hidden region on the declaring template, instantiated at every selection
- a region's space follows the storage it borrows: local for frame storage and local objects, shared for shared objects
- lowering erases a region's space and keys an `&immutable` or `&exclusive` parameter on its root, owned or aliasable
