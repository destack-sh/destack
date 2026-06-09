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

function allocate<comptime L: Lifetime>(arena: &exclusive Arena<Node, L>): ReadonlyBorrowed<Node, L> {
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

function escaped<comptime L: Lifetime>(): ReadonlyBorrowed<Node, L> {
    let arena = Arena.new<Node, L>(bump());
    return arena.alloc(Node { value: 1 });
}
```

- contains: lifetime

## bump

### bump allocates typed values

`Bump` owns uninitialized byte storage and returns typed arena borrows.

```ds
import { Result } from "destack:error";
import { AllocationError } from "destack:memory";
import { Bump } from "destack:memory/arena";

struct Node {
    value: int32;
}

function allocate(bump: &exclusive Bump): Result<&exclusive Node, AllocationError> {
    return bump.tryAlloc(^Node { value: 1 });
}
```
