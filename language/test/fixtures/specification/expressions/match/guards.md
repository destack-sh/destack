# Match Guards

Match guards participate in control flow typing.
Guard expressions narrow the matched value inside the guarded arm.

## guard narrowing

### match guard narrows with is

> Guard expressions using `is` narrow the scrutinee inside the guarded arm.

```ds
struct Foo {
    x: int32
}

struct Bar {
    y: int32
}

function pick(value: Foo | Bar): int32 {
    return match (value) {
        _ if (value is Foo) => {
            value.x satisfies int32;
            value.x
        }
        _ => 0
    };
}
```

### match guard narrows with in

> Guard expressions using `in` narrow object unions inside the guarded arm.

```ds
type WithX = { x: int32 };
type WithY = { y: int32 };

function pick(value: WithX | WithY): int32 {
    return match (value) {
        _ if ("x" in value) => {
            value.x satisfies int32;
            value.x
        }
        _ => 0
    };
}
```

### match guard sees pattern bindings

> Guard expressions can reference pattern bindings.

```ds
struct Point {
    x: int32
    y: int32
}

struct Other {
    y: int32
}

function clamp(point: Point | Other): int32 {
    return match (point) {
        Point { x } if (x > 0) => x
        _ => 0
    };
}
```

### match guard preserves union in later arms

> Guard narrowing does not leak into later arms that do not match the guard.

```ds
struct Foo {
    x: int32
}

struct Bar {
    y: int32
}

function pick(value: Foo | Bar): int32 {
    return match (value) {
        _ if (value is Foo) => value.x
        _ => {
            value satisfies Foo | Bar;
            0
        }
    };
}
```

### match guard narrows on typeof checks

> Typeof guards narrow the match value in the guarded arm.

```ds
function pick(value: string | int32): int32 {
    return match (value) {
        _ if (typeof value == "string") => {
            value satisfies string;
            0
        }
        _ => 0
    };
}
```

### match guard narrows with boolean conjunction

> Boolean guards narrow in the guarded arm when the left side narrows the value.

```ds
struct Foo {
    x: int32
}

struct Bar {
    y: int32
}

function pick(value: Foo | Bar): int32 {
    return match (value) {
        _ if (value is Foo && value.x > 0) => value.x
        _ => 0
    };
}
```
