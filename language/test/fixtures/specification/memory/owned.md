# Owned

## values

### owned expression yields owned type

> Owned expressions yield `^T` types.

```ds
struct Point {
    x: int32;
}

let owned: ^Point = ^Point { x: 1 };
```

### owned values are not assignable to plain values

> Owned values are not assignable to plain `T`.

```ds
struct Point {
    x: int32;
}

let value: Point = ^Point { x: 1 };
```

- contains: not assignable

### owned annotations require ownership

> Owned types require explicit ownership conversion.

```ds
struct Point {
    x: int32;
}

let value: ^Point = Point { x: 1 };
```

- contains: not assignable

### owned parameters require explicit ownership

> Owned parameters require explicit ownership conversion.

```ds
struct Data {
    value: int32;
}

function consume(value: ^Data): void {
    value.value;
}

let data = Data { value: 1 };
consume(data);
```

- contains: not assignable

### owned parameters accept explicit ownership conversion

> Owned parameters accept explicit ownership conversion.

```ds
struct Data {
    value: int32;
}

function consume(value: ^Data): void {
    value.value;
}

let data = Data { value: 1 };
consume(^data);
```

### owned conversion rejects owned values

> `^expr` only applies to unowned values.

```ds
struct Data {
    value: int32;
}

let data = ^Data { value: 1 };
let again = ^data;
```

- contains: ownership operator requires an unowned value

### owned fields are allowed in structs

> Structs can store owned fields directly.

```ds
struct Data {
    value: int32;
}

struct Container {
    data: ^Data;
}

const container = Container { data: ^Data { value: 1 } };
container.data satisfies ^Data;
```

## mutability

### owned values are mutable by default

> Owned values are mutable unless wrapped in `readonly`.

```ds
struct Point {
    x: int32;
}

let owned: ^Point = ^Point { x: 1 };
owned.x = 2;
owned satisfies ^Point;
```

### readonly owned values forbid mutation

> `^readonly T` allows ownership transfer but forbids mutation through the handle.

```ds
struct Point {
    x: int32;
}

let owned: ^readonly Point = ^readonly Point { x: 1 };
owned.x = 2;
```

- cannot assign
