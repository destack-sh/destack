# Ownership

> NOTE #Incomplete: implement/mdtest ownership semantics

Explicit control over references, values, and ownership transfer.

TypeScript doesn't distinguish references from values. Destack adds opt-in
explicit control, enabling a spectrum from TypeScript simplicity to Rust-level control.

## Subdirectories

| Directory | Description |
|-----------|-------------|
| `references/` | Reference types (`&T`, `&mut T`) |
| `values/` | Value types (`^T`, `^var T`) |
| `mutability/` | Mutability modifiers (`const`, `var`) |

## Overview

### Ownership Modifiers

```ds
T            // automatic (TypeScript behavior) - GC-managed
&T           // borrow (read-only reference)
&mut T       // borrow (mutable reference)
^T           // ownership transfer (caller gives up ownership)
^var T       // ownership transfer (explicitly mutable)
```

### Semantics

| Modifier | After `foo(x)` | Who cleans up? |
|----------|----------------|----------------|
| `T` | `x` still valid | GC |
| `&T` | `x` still valid | Original owner |
| `&mut T` | `x` still valid | Original owner |
| `^T` | `x` **invalid** | New owner |

### Use-After-Move

```ds
const node = AstNode { ... }
consume(^node)    // ownership transferred
print(node.value) // ERROR: use after ownership transfer
```

See [DESIGN.md](../../../../../DESIGN.md#ownership) for full documentation.
