# base

Low-level utilities shared across the Destack toolchain.

## Arena

Simple chunked allocator providing stable references.
Used by `NodeTree` (AST), `NodeGraph` (DIR), and other IR structures to store nodes efficiently.

```ds
const arena: Arena<MyNode> = Arena.new()
const id: uint32 = arena.allocate(node)
const node: MyNode = arena.get(id)
```

Arenas grow in fixed-size chunks, so existing references remain valid as new elements are added.

## StringPool / StringId

Global string interning.
All identifiers, string literals, and other repeated strings are interned to `StringId` for cheap equality checks and reduced memory usage.

```ds
const id: StringId = pool.intern("foo")
const s: string = pool.get(id)
```