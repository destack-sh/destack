---
title: Interfaces
description: Structural interfaces are a key part of TypeScript.
---

# Interfaces

- structural interfaces are a key part of typescript
- `type` vs `interface`
- how dynamic do we want to go
- shape mutation
- excess properties
- declaration exprsesions
- dynamic prototypes
- all sorts of JS hacks that everyone hates anyway

- funky signatures:
- call signatures (just a `Function` fat pointer)
- construct signatures (also a `Function` fat pointer)
- index signatures
    - can I read through index signatures? can I call through them?
    - (index signatures use `dynamic.find` at runtime, which is a linear scan over string equality!)

- in general interfaces should "just work" by default, like everything in TS++
- when we want / need to be explicit, `Dynamic<T>`
- like explicit `dyn T` (but fixed size fat pointer)
- `unknown` is just `Dynamic<unknown>`

```ds:src/interfaces.ds
interface Named {
    name: string;
}

function label(value: Named): string {
    value.name
}
```

## Representation

- closed: aliases, newtypes, and object shapes have one concrete representation
- open: bare structural interfaces and indexed shapes are open and store through `Dynamic<T>`
- indexed structural fields are readonly and return `T | undefined`; represented collections such as `Map` implement `IndexSet` for writes
- keyed lookups borrow their keys: `Map<K, V>` and `SortedMap<K, V>` implement `Index<&immutable K>`, so `map[key]` borrows `key` and a `Copy` key passes by value
