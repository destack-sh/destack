# Match Exhaustiveness

## enums

### enum matches need full coverage

Enum matches must cover every field or use a fallback arm.

```ds
enum State {
    Ready,
    Loading,
    Error,
}

function label(state: State): string {
    match (state) {
        Ready => "go"
        Loading => "wait"
    }
}
```

- contains: non-exhaustive match

### enum matches cover every field

Covering every enum field is exhaustive.

```ds
enum State {
    Ready,
    Loading,
    Error,
}

function label(state: State): string {
    match (state) {
        Ready => "go"
        Loading => "wait"
        Error => "error"
    }
}
```

### fallback arms cover enums

A fallback arm covers the remaining enum fields.

```ds
enum State {
    Ready,
    Loading,
    Error,
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

### literal unions need full coverage

Literal unions must cover every literal or use a fallback arm.

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

### boolean matches cover true and false

`true` and `false` cover `boolean`.

```ds
function label(value: boolean): int32 {
    match (value) {
        true => 1
        false => 0
    }
}
```

### boolean unions keep other literals

Boolean literals and other literals must all be covered.

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

### string literal unions cover every member

String literal unions are exhaustive when every literal is covered.

```ds
type Mode = "open" | "closed";

function label(mode: Mode): string {
    match (mode) {
        "open" => "open"
        "closed" => "closed"
    }
}
```

### number literal unions cover every member

Number literal unions are exhaustive when every literal is covered.

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

### bigint literal unions cover every member

Bigint literal unions are exhaustive when every literal is covered.

```ds
type Bits = 1n | 2n;

function label(bits: Bits): int32 {
    match (bits) {
        1n => 1
        2n => 2
    }
}
```

### nullish unions cover null and undefined

`null` and `undefined` both need coverage.

```ds
type Maybe = null | undefined;

function label(value: Maybe): string {
    match (value) {
        null => "null"
        undefined => "undefined"
    }
}
```

### fallback arms cover literal unions

A fallback arm covers the remaining literal members.

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

### union patterns cover multiple literals

Union patterns cover each listed literal.

```ds
type Status = 1 | 2 | 3;

function label(status: Status): int32 {
    match (status) {
        1 | 2 => 0
        3 => 1
    }
}
```

### tuple discriminants cover tuple unions

Literal tuple positions cover tuple union members.

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

### tuple union patterns cover multiple discriminants

Union patterns cover multiple tuple discriminants.

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

### open unions need fallback arms

Unions with open members need a fallback arm.

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

### missing tuple discriminants are non-exhaustive

Tuple unions are non-exhaustive when a discriminant literal is missing.

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

### guarded tuple arms need fallback arms

Guarded arms do not prove tuple coverage.

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

### discriminated unions need full coverage

Discriminated unions must cover every tag or use a fallback arm.

```ds
type Shape = { kind: "circle"; radius: int32 } | { kind: "square"; size: int32 };

function area(shape: Shape): int32 {
    match (shape) {
        { kind: "circle" } => 0
    }
}
```

- contains: non-exhaustive match

### discriminated unions cover every tag

Discriminated unions are exhaustive when every discriminant is covered.

```ds
type Shape = { kind: "circle"; radius: int32 } | { kind: "square"; size: int32 };

function area(shape: Shape): int32 {
    match (shape) {
        { kind: "circle" } => 0
        { kind: "square" } => 1
    }
}
```

### union tag patterns cover multiple variants

Union patterns on tag fields cover each listed variant.

```ds
type Shape =
    | { kind: "circle"; radius: int32 }
    | { kind: "square"; size: int32 }
    | { kind: "triangle"; side: int32 };

function area(shape: Shape): int32 {
    match (shape) {
        { kind: "circle" | "square" } => 0
        { kind: "triangle" } => 1
    }
}
```

### fallback arms cover discriminated unions

A fallback arm covers the remaining variants.

```ds
type Shape = { kind: "circle"; radius: int32 } | { kind: "square"; size: int32 };

function area(shape: Shape): int32 {
    match (shape) {
        { kind: "circle" } => 0
        _ => 1
    }
}
```

### nested discriminants still need full coverage

Nested object discriminant matches must still cover every union variant.

```ds
type Envelope =
    | { kind: "text"; data: { tag: 1; value: string } }
    | { kind: "code"; data: { tag: 2; value: int32 } };

function label(envelope: Envelope): int32 {
    match (envelope) {
        {
            kind: "text",
            data: { tag: 1, value: _ },
        } => 1
    }
}
```

- contains: non-exhaustive match

### wildcard tags cover remaining variants

Wildcard tag fields cover the remaining variants.

```ds
type Envelope = { kind: "text"; payload: string } | { kind: "code"; payload: int32 };

function label(envelope: Envelope): int32 {
    match (envelope) {
        { kind: "text", payload: _ } => 1
        { kind: _ } => 2
    }
}
```

## contextual refutability

### irrefutable bindings cover the value

Binding patterns are irrefutable and satisfy exhaustiveness.

```ds
function identity(value: number): number {
    return match (value) {
        v => v
    };
}
```

### nominal object patterns cover their own type

Nominal object patterns are irrefutable when the value already has the nominal type.

```ds
class User {
    name: string = "";
}

function read(user: User): string {
    return match (user) {
        User { name } => name
    };
}
```

### nominal object patterns need fallback arms for nullish unions

Nominal object patterns do not cover nullish union members.

```ds
class User {
    name: string = "";
}

function read(user: User | null): string {
    return match (user) {
        User { name } => name
    };
}
```

- contains: non-exhaustive match

### newtype patterns cover their own type

Newtype patterns are irrefutable when the value already has the newtype.

```ds
newtype UserId = int64;

function read(id: UserId): int64 {
    return match (id) {
        UserId(value) => value
    };
}
```

### newtype patterns need fallback arms for nullish unions

Newtype patterns do not cover nullish union members.

```ds
newtype UserId = int64;

function read(id: UserId | null): int64 {
    return match (id) {
        UserId(value) => value
    };
}
```

- contains: non-exhaustive match

### structural object patterns cover known object types

Object patterns are irrefutable when all required fields are present.

```ds
type Config = { retries: int32 };

function read(config: Config): int32 {
    return match (config) {
        { retries } => retries
    };
}
```

### object defaults do not cover nullish unions

Defaults only cover missing fields inside matched objects.

```ds
type Config = { retries?: int32 };

function read(config: Config | null): int32 {
    return match (config) {
        { retries = 0 } => retries
    };
}
```

- contains: non-exhaustive match

## fallback arms

### guarded arms need fallback arms

Guarded arms do not prove exhaustiveness.

```ds
function pick(value: int32): int32 {
    match (value) {
        x if (x > 0) => x
    }
}
```

- contains: non-exhaustive match

### open array patterns need fallback arms

Array patterns over open arrays require a fallback.

```ds
function firstTwo(values: number[]): number {
    return match (values) {
        [first, second] => first + second
    };
}
```

- contains: non-exhaustive match
