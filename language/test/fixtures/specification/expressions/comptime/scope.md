# Comptime Scope

Comptime expressions have isolated evaluation scope.

## locals

### comptime locals can be mutated

> Mutation inside the comptime expression is allowed.

```ds
const width = comptime {
    let value = 4;
    value *= 2;
    value
};

width satisfies 8;
```

### comptime values return lowerable data

> Lowerable data can leave a comptime expression.

```ds
struct Config {
    width: uint;
    tags: string[];
}

const config = comptime Config {
    width: 4,
    tags: ["fast", "safe"],
};

config.width satisfies 4;
config.tags[0] satisfies "fast";
```

## members

### struct comptime blocks can read static parameters

> Member comptime blocks run in the instantiated static environment.

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

### class comptime blocks can read static parameters

> Class member comptime blocks use the member static environment.

```ds
class Buffer<comptime N: uint> {
    comptime {
        assert(N > 0);
    }

    data: [uint8; N] = [0; N];
}

declare const buffer: Buffer<4>;
buffer.data satisfies [uint8; 4];
```

## outer state

### comptime expressions cannot mutate outer bindings

> Comptime evaluation cannot change bindings outside the expression.

```ds
let counter = 0;

comptime {
    counter += 1;
}
```

- contains: static expression

### comptime member blocks cannot read instance fields

> Member comptime blocks cannot read runtime instance state.

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

### comptime expressions reject non-lowerable values

> Runtime resources cannot escape comptime evaluation.

```ds
declare function currentWorker(): Worker;

const worker = comptime currentWorker();
```

- contains: lowerable
