# Reference Type Annotations

## reference annotations

### reference annotations are accepted

> Reference annotations can appear in signatures.

```ds
struct Point {
    x: int32;
}

function read(value: &Point): int32 {
    return value.x;
}
```

### value annotations are accepted

> Value annotations can appear in signatures.

```ds
struct Point {
    x: int32;
}

function copy(value: ^Point): ^Point {
    return value;
}
```

### generic reference annotations are accepted

> Reference annotations should work with generic owners.

```ds
class Box<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

function read<T>(value: &Box<T>): T {
    return value.value;
}
```

### references can use fixed arrays driven by comptime parameters

> Reference annotations should support fixed arrays parameterized by comptime values.

```ds
function readLane<comptime N: number>(value: &uint8[N as comptime]): uint8 {
    return value[0];
}
```

### references can use associated type projections

> Reference annotations should accept associated type projections in type position.

```ds
interface BufferLike<T> {
    type Item = T;
}

class TextBuffer implements BufferLike<string> {}

function read(value: &TextBuffer.Item): void {
}
```

### references can use associated comptime projections in fixed arrays

> Associated comptime projections should be usable in fixed-array reference annotations.

```ds
class Segment<Row> {
    comptime const Width: number = Row extends string ? 8 : 4;
}

function read(value: &uint8[Segment<string>.Width as comptime]): uint8 {
    return value[0];
}
```
