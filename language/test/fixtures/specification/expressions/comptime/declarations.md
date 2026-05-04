# Comptime Declarations

## comptime blocks in declarations

### comptime block allowed in struct

> Comptime blocks are accepted struct members.

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

### comptime block allowed in class

> Comptime blocks are accepted class members.

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

## calls

### comptime calls evaluate functions

> Functions can be evaluated in comptime contexts.

```ds
function factorial(n: int): int {
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}

const value = comptime factorial(4);
value satisfies int;
```

### comptime calls reject runtime inputs

> Comptime calls reject non-static runtime inputs.

```ds
function factorial(n: int): int {
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}

function compute_runtime(value: int): int {
    const result = comptime factorial(value);
    result
}
```

- contains: static expression

### class comptime blocks reject runtime instance access

> Class comptime blocks cannot depend on runtime instance state.

```ds
class Counter {
    value: int32 = 0

    comptime {
        let snapshot = this.value;
        let _ = snapshot;
    }
}
```

- contains: static expression
