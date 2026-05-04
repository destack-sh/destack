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

### match accepts exhaustive boolean coverage

> Boolean matches are exhaustive when they cover `true` and `false`.

```ds
function label(value: boolean): int32 {
    match (value) {
        true => 1
        false => 0
    }
}
```

### match accepts boolean unions with other literals

> Boolean unions remain exhaustive when `true` and `false` are covered.

```ds
type Status = boolean | "pending";

function label(status: Status): number {
    match (status) {
        true => 1
        false => 0
        "pending" => 2
    }
}
```

### match accepts exhaustive string literal unions

> String literal unions are exhaustive when every literal is covered.

```ds
type Mode = "open" | "closed";

function label(mode: Mode): string {
    match (mode) {
        "open" => "open"
        "closed" => "closed"
    }
}
```

### match accepts exhaustive number literal unions

> Number literal unions are exhaustive when every literal is covered.

```ds
type Level = 1 | 2 | 3;

function label(level: Level): string {
    match (level) {
        1 => "low"
        2 => "mid"
        3 => "high"
    }
}
```

### match accepts exhaustive bigint literal unions

> Bigint literal unions are exhaustive when every literal is covered.

```ds
type Bits = 1n | 2n;

function label(bits: Bits): int32 {
    match (bits) {
        1n => 1
        2n => 2
    }
}
```

### match accepts exhaustive nullish literal unions

> Nullish literal unions are exhaustive when `null` and `undefined` are covered.

```ds
type Maybe = null | undefined;

function label(value: Maybe): string {
    match (value) {
        null => "null"
        undefined => "undefined"
    }
}
```

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

### match accepts exhaustive union literal patterns

> Union patterns count toward literal union exhaustiveness.

```ds
type Status = 1 | 2 | 3;

function label(status: Status): int32 {
    match (status) {
        1 | 2 => 0
        3 => 1
    }
}
```

### match accepts exhaustive tuple unions with literal discriminants

> Tuple unions are exhaustive when a literal discriminant position is fully covered.

```ds
type Pair = (1, string) | (2, string);

function normalize(pair: Pair): int32 {
    match (pair) {
        (1, value) => {
            value satisfies string;
            1
        }
        (2, value) => {
            value satisfies string;
            2
        }
    }
}
```

### match accepts tuple discriminant union patterns

> Tuple union patterns count toward tuple discriminant exhaustiveness.

```ds
type Pair = (1, string) | (2, string) | (3, string);

function normalize(pair: Pair): int32 {
    match (pair) {
        (1 | 2, _) => {
            1
        }
        (3, _) => 2
    }
}
```

### match requires fallback for non-literal unions

> Unions with non-literal members require a fallback arm.

```ds
type Mixed = string | number;

function label(value: Mixed): string {
    match (value) {
        "ready" => "ok"
        1 => "one"
    }
}
```

- contains: non-exhaustive match

### match reports non-exhaustive tuple unions with literal discriminants

> Tuple unions are non-exhaustive when a discriminant literal is missing.

```ds
type Pair = (1, string) | (2, string);

function normalize(pair: Pair): int32 {
    match (pair) {
        (1, value) => {
            value satisfies string;
            1
        }
    }
}
```

- contains: non-exhaustive match

### match requires fallback for guarded tuple discriminants

> Guarded tuple discriminant arms require a fallback because coverage is not provable.

```ds
type Pair = (1, string) | (2, string);

function normalize(pair: Pair): int32 {
    match (pair) {
        (1, _) if (true) => {
            1
        }
        (2, _) => 2
    }
}
```

- contains: non-exhaustive match

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

### match accepts exhaustive discriminated unions

> Discriminated unions are exhaustive when every discriminant is covered.

```ds
type Shape =
    | { kind: "circle", radius: int32 }
    | { kind: "square", size: int32 };

function area(shape: Shape): int32 {
    match (shape) {
        { kind: "circle" } => 0
        { kind: "square" } => 1
    }
}
```

### match accepts exhaustive discriminated unions with union tag patterns

> Union patterns on discriminant keys can cover multiple variants.

```ds
type Shape =
    | { kind: "circle", radius: int32 }
    | { kind: "square", size: int32 }
    | { kind: "triangle", side: int32 };

function area(shape: Shape): int32 {
    match (shape) {
        { kind: "circle" | "square" } => 0
        { kind: "triangle" } => 1
    }
}
```

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

### match reports non-exhaustive nested discriminant object patterns

> Nested object discriminant matches must still cover every union variant.

```ds
type Envelope =
    | { kind: "text", data: { tag: 1, value: string } }
    | { kind: "code", data: { tag: 2, value: int32 } };

function label(envelope: Envelope): int32 {
    match (envelope) {
        { kind: "text", data: { tag: 1, value: _ } } => 1
    }
}
```

- contains: non-exhaustive match

### match accepts wildcard object filters as fallback coverage

> Object wildcard filters can serve as explicit fallback coverage.

```ds
type Envelope =
    | { kind: "text", payload: string }
    | { kind: "code", payload: int32 };

function label(envelope: Envelope): int32 {
    match (envelope) {
        { kind: "text", payload: _ } => 1
        { kind: _ } => 2
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
        x if (x > 0) => x
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
