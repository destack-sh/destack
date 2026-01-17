# Owned Values

## owned values

### value expression yields owned type

> Value expressions yield `^T` types.

```ds
struct Point {
    x: int32,
}

let owned: ^Point = ^Point { x: 1 };
```

### owned values are not assignable to owned-less types

> Owned values are not assignable to plain `T`.

```ds
struct Point {
    x: int32,
}

let value: Point = ^Point { x: 1 };
```

- contains: not assignable
