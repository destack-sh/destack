# Match Exhaustiveness

## enums

### match reports non-exhaustive enums

> Match expressions must cover every enum field or use a fallback arm.

```ds
enum State {
    Ready
    Loading
    Error
}

function label(state: State): string {
    match (state) {
        Ready => "go"
        Loading => "wait"
    }
}
```

- contains: non-exhaustive match

### match accepts full enum coverage

> Full enum coverage makes the match exhaustive.

```ds
enum State {
    Ready
    Loading
    Error
}

function label(state: State): string {
    match (state) {
        Ready => "go"
        Loading => "wait"
        Error => "error"
    }
}
```

### match accepts fallback arm

> A fallback arm makes the match exhaustive.

```ds
enum State {
    Ready
    Loading
    Error
}

function label(state: State): string {
    match (state) {
        Ready => "go"
        Loading => "wait"
        _ => "error"
    }
}
```

## literal unions

### match reports non-exhaustive literal unions

> Literal unions must cover every literal.

```ds
type Status = "ready" | "loading" | "error";

function label(status: Status): string {
    match (status) {
        "ready" => "go"
        "loading" => "wait"
    }
}
```

- contains: non-exhaustive match

### match accepts fallback arm for literal unions

> A fallback arm makes literal unions exhaustive.

```ds
type Status = "ready" | "loading" | "error";

function label(status: Status): string {
    match (status) {
        "ready" => "go"
        "loading" => "wait"
        _ => "error"
    }
}
```

## discriminated unions

### match reports non-exhaustive discriminated unions

> Discriminated unions must cover every discriminant value.

```ds
type Shape =
    | { kind: "circle", radius: int32 }
    | { kind: "square", size: int32 };

function area(shape: Shape): int32 {
    match (shape) {
        { kind: "circle" } => 0
    }
}
```

- contains: non-exhaustive match

### match accepts fallback arm for discriminated unions

> A fallback arm makes discriminated unions exhaustive.

```ds
type Shape =
    | { kind: "circle", radius: int32 }
    | { kind: "square", size: int32 };

function area(shape: Shape): int32 {
    match (shape) {
        { kind: "circle" } => 0
        _ => 1
    }
}
```

## irrefutable patterns

### match allows irrefutable bindings without fallback

> Binding patterns are irrefutable and satisfy exhaustiveness.

```ds
function identity(value: number): number {
    return match (value) {
        v => v
    };
}
```

## fallback requirement

### match requires fallback when guards are present

> Guarded arms do not prove exhaustiveness.

```ds
function pick(value: int32): int32 {
    match (value) {
        x if x > 0 => x
    }
}
```

- contains: non-exhaustive match

### match requires fallback for open array patterns

> Array patterns over open arrays require a fallback.

```ds
function firstTwo(values: number[]): number {
    return match (values) {
        [first, second] => first + second
    };
}
```

- contains: non-exhaustive match
