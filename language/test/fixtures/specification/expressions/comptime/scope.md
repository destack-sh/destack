# Comptime Scope

Comptime blocks can use their own locals but cannot mutate outer state.

## locals

### comptime locals can be mutated

Mutation inside the comptime block is allowed.

```ds
const width = comptime {
    let value = 4;
    value *= 2;
    value
};

width satisfies 8;
```

### comptime blocks can read static parameters

Generic value parameters are available inside member comptime blocks.

```ds
struct Buffer<comptime N: uint> {
    comptime {
        assert(N > 0);
    }

    data: [uint8; N];
}

declare const buffer: Buffer<4>;
buffer.data satisfies [uint8; 4];
```

## members

### struct comptime blocks are allowed

Comptime blocks are accepted struct members.

```ds
struct Buffer {
    value: int32

    comptime {
        let size = 4;
        let _ = size + 1;
    }
}

const buffer = Buffer { value: 0 };
buffer satisfies Buffer;
```

### class comptime blocks are allowed

Comptime blocks are accepted class members.

```ds
class Counter {
    value: int32 = 0

    comptime {
        let seed = 1;
        let _ = seed;
    }
}

const counter = new Counter();
counter satisfies Counter;
```

## outer state

### comptime blocks cannot mutate outer bindings

Comptime evaluation cannot change bindings outside the block.

```ds
let counter = 0;

comptime {
    counter += 1;
}
```

- contains: static expression

### comptime blocks cannot read instance fields

Member comptime blocks cannot read runtime instance fields.

```ds
class Counter {
    value: int32 = 0;

    comptime {
        const snapshot = this.value;
        snapshot;
    }
}
```

- contains: static expression

### comptime blocks cannot call runtime functions

Comptime evaluation cannot call runtime-only code.

```ds
function read_runtime(): int32 {
    4
}

const value = comptime {
    read_runtime()
};
```

- contains: static expression
