# Let

## literals

### let else matches literal patterns

> Literal patterns are valid when the else branch exits.

```ds
function parse(value: "ok" | "bad"): int32 {
    let "ok" = value else {
        return 0;
    };

    1
}
```

### let else narrows after success

> After a literal pattern succeeds, the source value is narrowed.

```ds
function parse(value: "ok" | "bad"): "ok" {
    let "ok" = value else {
        return "ok";
    };

    value
}
```

## newtypes

### let else binds newtypes

> Newtype patterns bind the wrapped value.

```ds
newtype UserId = int64;

function read(id: UserId): int64 {
    let UserId(value) = id else {
        return 0;
    };

    value
}
```

## structs

### let else binds structs

> Struct patterns use the nominal tag.

```ds
struct Point {
    x: int32
    y: int32
}

function read(point: Point): int32 {
    let Point { x, y: _ } = point else {
        return 0;
    };

    x
}
```

### refutable let patterns need else

> Refutable patterns need an else continuation.

```ds
function parse(value: "ok" | "bad"): int32 {
    let "ok" = value;
    1
}
```

- contains: else

## tuples

### let else binds tuples

Tuple fields bind after the pattern succeeds.

```ds
function read(value: (int32, string) | null): int32 {
    let (count, label) = value else {
        return 0;
    };

    label satisfies string;
    count
}
```

## arrays

### let else binds fixed arrays

Fixed array elements bind after the pattern succeeds.

```ds
function read(values: [int32; 2] | null): int32 {
    let [first, second] = values else {
        return 0;
    };

    first + second
}
```

### let else binds fixed array tails

Rest patterns bind the fixed array tail.

```ds
function read(values: [int32; 3] | null): int32 {
    let [first, ...rest] = values else {
        return 0;
    };

    rest satisfies [int32; 2];
    first
}
```

## objects

### let else binds objects

Object fields bind after the pattern succeeds.

```ds
function read(value: { id: int32, name: string } | null): int32 {
    let { id, name } = value else {
        return 0;
    };

    name satisfies string;
    id
}
```
