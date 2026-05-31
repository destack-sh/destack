# Match Guards

Match guards refine the selected arm.
They can use both the matched value and bindings introduced by the pattern.

## guard narrowing

### is guards narrow the arm

`is` guards narrow the matched value.

```ds
struct Foo {
    x: int32;
}

struct Bar {
    y: int32;
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

### in guards narrow the arm

`in` guards narrow object unions.

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

### guards can use pattern bindings

Pattern bindings are in scope for the guard.

```ds
struct Point {
    x: int32;
    y: int32;
}

struct Other {
    y: int32;
}

function clamp(point: Point | Other): int32 {
    return match (point) {
        Point { x } if (x > 0) => x
        _ => 0
    };
}
```

### guard narrowing stays in its arm

Later arms see the original matched type.

```ds
struct Foo {
    x: int32;
}

struct Bar {
    y: int32;
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

### boolean guards keep narrowing facts

`&&` keeps narrowing from its left side.

```ds
struct Foo {
    x: int32;
}

struct Bar {
    y: int32;
}

function pick(value: Foo | Bar): int32 {
    return match (value) {
        _ if (value is Foo && value.x > 0) => value.x
        _ => 0
    };
}
```
