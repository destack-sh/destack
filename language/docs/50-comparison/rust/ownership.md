---
title: Ownership
description: Ownership and borrowing, compared to Rust.
---

# Ownership

If you know Rust, TS++ ownership will look familiar: owned values move, `Copy` and `Clone` mean what they mean in Rust, `Drop` runs when an owner goes away, and the borrow checker makes sure no borrow outlives what it points at.
The difference is the default.
In Rust, ownership is the starting point.
In TS++, the starting point is a TypeScript-shaped managed handle, and ownership is something you opt into where it pays off.

| Rust | TS++ | Notes |
| --- | --- | --- |
| `T` (owned value) | `^T` | uniquely owned, moves |
| `Rc<RefCell<T>>` | `T` for a class | a handle to a collected object, shared freely |
| `&T` | `&immutable T` | nobody writes while it lives |
| `&mut T` | `&exclusive T` | no other access while it lives |
| — | `&readonly T` | read access, others may still write |
| — | `&T` | mutable access, shared with others |
| `*const T` / `*mut T` | `*T` | unchecked, `unsafe` to use |
| `'a` | `'a` | lifetimes elide much like in Rust |

## Handles Instead of `Rc`

A class value is a handle to a collected object, so sharing is free and nothing is reference counted.
In Rust, you'd reach for `Rc<RefCell<T>>` to get the same thing.

```rust
let counter = Rc::new(RefCell::new(Counter::default()));
let alias = counter.clone();
alias.borrow_mut().increment();
```

```tspp
class Counter {
    value = 0;

    increment(): void {
        this.value += 1;
    }
}

const counter = new Counter();
const alias = counter;
alias.increment();
```

## Four Kinds of Borrow

Rust has two borrows, and TS++ has four, because a TS-shaped program shares a lot.
`&immutable` and `&exclusive` are Rust's `&` and `&mut`.
`&readonly` and `&` are the shared ones: they allow others to keep writing, so they never block an alias, and the checker makes sure what they point at stays valid.

```tspp
struct Point {
    x: float64;
    y: float64;
}

function length(point: &immutable Point): float64 {
    point.x * point.x + point.y * point.y
}

function reset(point: &exclusive Point): void {
    point.x = 0.0;
    point.y = 0.0;
}

let point = Point { x: 3.0, y: 4.0 };
length(&immutable point);
reset(&exclusive point);
```

## Owned Values

`^T` is Rust's owned `T`: one owner, moved on assignment, dropped at the end of its owner's scope.
Classes can be owned too, which puts the object inline in its owner instead of on the collected heap.

```tspp
class Buffer {
    size = 0;

    constructor(&exclusive this) {}
}

function demo(): void {
    const managed = new Buffer();
    const owned: ^Buffer = new Buffer();
    const moved = owned;
    const again = owned;
    //            ^^^^^ error[use-after-move]: use of moved value
}
```
