# Match Typing

## Result typing

### match unions case types

> Match results form a union when case types differ.

```ds
function matchUnion(value: int32): string {
    const result: string = match (value) {
        0 => {
            1 as int32
        }
        _ => "ok"
    };
    result
}
```

- contains: not assignable

## Pattern typing

### match patterns must be compatible with the matched value

> Pattern expressions must be assignable to the match value type.

```ds
function invalidMatchPattern(value: int32): int32 {
    match (value) {
        "hi" => 1
        _ => 2
    }
}
```

- contains: not assignable

### struct patterns destructure tagged structs

> Struct patterns require the type name and destructure fields.

```ds
struct Point {
    x: int32
    y: int32
}

function sum(value: Point): int32 {
    match (value) {
        Point { x, y } => x + y
    }
}
```

### struct patterns reject bare object patterns

> Bare object patterns do not match nominal structs.

```ds
struct Point {
    x: int32
    y: int32
}

function sum(value: Point): int32 {
    match (value) {
        { x, y } => x + y
        _ => 0
    }
}
```

- contains: not assignable

### match arm result types must satisfy contextual annotations

> Contextually typed match expressions require each arm to satisfy the annotation.

```ds
function choose(value: int32): string {
    const result: string = match (value) {
        0 => "zero"
        _ => 1
    };
    result
}
```

- contains: not assignable

### match literal patterns can narrow union primitives

> Literal patterns narrow union primitives within each arm.

```ds
function describe(value: int32 | string): string {
    match (value) {
        0 => "zero"
        "ok" => "ok"
        _ => "other"
    }
}
```
