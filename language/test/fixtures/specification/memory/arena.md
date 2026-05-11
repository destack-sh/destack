# Arena

## arena

### arena allocations share a lifetime

Arena allocation returns borrowed access tied to the arena lifetime.

```ds
import { Arena, Bump } from "destack:memory/arena";

struct Node {
    value: int32;
}

declare function bump(): Bump;

function allocate<L: Lifetime>(arena: &exclusive Arena<Node, L>): ReadonlyBorrowed<Node, L> {
    return arena.alloc(Node { value: 1 });
}
```

### arena borrows cannot outlive the arena

The arena lifetime prevents allocated borrows from escaping their owner.

```ds
import { Arena, Bump } from "destack:memory/arena";

struct Node {
    value: int32;
}

declare function bump(): Bump;

function escaped<L: Lifetime>(): ReadonlyBorrowed<Node, L> {
    let arena = Arena.new<Node, L>(bump());
    return arena.alloc(Node { value: 1 });
}
```

- contains: lifetime

## bump

### bump is a raw allocator

`Bump` implements the raw `Allocator` protocol and does not create owned values by itself.

```ds
import { AllocationLayout } from "destack:memory";
import { Bump } from "destack:memory/arena";

declare function bump(): Bump;

let allocator = bump();
let layout = AllocationLayout.new(64, 8);
let allocation = allocator.allocate(layout)?;

allocation satisfies Allocation<uint8, "local">;
```
