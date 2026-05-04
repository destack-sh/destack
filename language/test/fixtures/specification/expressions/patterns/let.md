# Let

## literals

### let else accepts literal patterns

> Literal patterns can be used when the failure branch exits.

```ds
function parse(value: "ok" | "bad"): int32 {
    let "ok" = value else {
        return 0;
    };

    1
}
```

### let else narrows after the binding

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

### let else destructures scalar newtypes

> Newtype patterns can bind their inner value.

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

### let else destructures tagged structs

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

### let else requires an else branch for refutable patterns

> Refutable patterns need an else continuation.

```ds
function parse(value: "ok" | "bad"): int32 {
    let "ok" = value;
    1
}
```

- contains: else
