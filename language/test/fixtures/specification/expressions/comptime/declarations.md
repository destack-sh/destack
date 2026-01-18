# Comptime Declarations

## comptime blocks in declarations

### comptime block allowed in struct

> Comptime blocks are valid struct members.

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

> Comptime blocks are valid class members.

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
