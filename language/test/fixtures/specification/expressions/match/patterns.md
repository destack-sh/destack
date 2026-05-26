# Match Patterns

Match arms use the same pattern families as bindings and if (let ...).

## tuple patterns

### match tuple patterns bind tuple elements

Tuple patterns bind tuple elements by position.

```ds
declare const pair: (int32, int32);

match (pair) {
    (left, right) => {
        left satisfies int32;
        right satisfies int32;
    }
}
```

### match nested tuple patterns bind nested tuple elements

Nested tuple patterns destructure recursively by position.

```ds
declare const pair: ((int32, int32), int32);

match (pair) {
    ((left, right), tail) => {
        left satisfies int32;
        right satisfies int32;
        tail satisfies int32;
    }
}
```

## fixed arrays

### match fixed array patterns bind elements

Fixed array patterns bind element types.

```ds
declare const pair: [int32; 2];

match (pair) {
    [left, right] => {
        left satisfies int32;
        right satisfies int32;
    }
}
```

## slices

### match slice patterns bind heads

Slice patterns bind known positions and keep rest tails as slices.

```ds
declare const values: [int32];

match (values) {
    [first, ...rest] => {
        first satisfies int32;
        rest satisfies [int32];
    }
    _ => {}
}
```

## object patterns

### match object patterns bind named fields

Object patterns bind named fields from structural objects.

```ds
type Config = { enabled: boolean; retries: int32 };

declare const config: Config;

match (config) {
    { enabled, retries } => {
        enabled satisfies boolean;
        retries satisfies int32;
    }
}
```

### match nested object patterns bind nested fields

Nested object patterns bind nested object fields.

```ds
type Config = { runtime: { retries: int32 }; enabled: boolean };

declare const config: Config;

match (config) {
    {
        runtime: { retries },
        enabled,
    } => {
        retries satisfies int32;
        enabled satisfies boolean;
    }
}
```

### match wildcards ignore object fields

Wildcards ignore selected fields while other fields bind.

```ds
type Config =
    | { kind: "a"; retries: int32; enabled: boolean }
    | { kind: "b"; retries: int32; enabled: boolean };

declare const config: Config;

match (config) {
    { kind: _, retries } => {
        retries satisfies int32;
    }
}
```

## nominal object patterns

### match nominal object patterns bind struct fields

Nominal object patterns use the nominal tag.

```ds
struct Point {
    x: int32;
    y: int32;
}

function sum(value: Point): int32 {
    match (value) {
        Point { x, y } => x + y
    }
}
```

### match nominal object patterns bind class fields

Class patterns use the class tag and bind stored fields.

```ds
class User {
    name: string = "";
}

function read(user: User): string {
    match (user) {
        User { name } => name
    }
}
```

### match nominal object patterns reject getters

Object patterns bind stored fields, not getters.

```ds
class User {
    name: string = "";

    get displayName(): string {
        return this.name;
    }
}

function read(user: User): string {
    match (user) {
        User { displayName } => displayName
    }
}
```

- contains: stored field

### match nominal object patterns reject bare object patterns

Bare object patterns do not match nominal values.

```ds
struct Point {
    x: int32;
    y: int32;
}

function sum(value: Point): int32 {
    match (value) {
        { x, y } => x + y
        _ => 0
    }
}
```

- contains: not assignable

## rest patterns

### match tuple rest binds tails

Tuple rest patterns bind the tail as a tuple.

```ds
declare const values: (int32, int32, int32);

match (values) {
    (first, ...rest) => {
        first satisfies int32;
        rest satisfies (int32, int32);
    }
}
```

### match fixed array rest binds tails

Fixed array rest patterns bind the tail as a fixed array.

```ds
declare const values: [int32; 3];

match (values) {
    [first, ...rest] => {
        first satisfies int32;
        rest satisfies [int32; 2];
    }
}
```

### match object rest binds tails

Object rest patterns bind the remaining fields.

```ds
type Config = { enabled: boolean; retries: int32 };

declare const config: Config;

match (config) {
    { enabled, ...rest } => {
        enabled satisfies boolean;
        rest satisfies { retries: int32 };
    }
}
```

## must patterns

### match must patterns bind non-nullish values

Must patterns bind the non-nullish value.

```ds
declare const value: int32 | null;

match (value) {
    x! => {
        x satisfies int32;
    }
    _ => {}
}
```

## ownership patterns

### match value patterns bind owned values

Owned patterns bind an owned view.

```ds
declare const value: ^int32;

match (value) {
    ^x => {
        x satisfies ^int32;
    }
}
```

### match reference patterns bind references

Borrow patterns bind a borrowed view.

```ds
declare const value: &int32;

match (value) {
    &x => {
        x satisfies &int32;
    }
}
```

## union patterns

### match union patterns cover multiple literals

Union patterns match any of their literal values.

```ds
declare const value: 1 | 2 | 3;

match (value) {
    1 | 2 => "low"
    3 => "high"
}
```

## invalid patterns

### literal patterns must fit the matched type

Literal patterns must be assignable to the matched type.

```ds
function invalidMatchPattern(value: int32): int32 {
    match (value) {
        "hi" => 1
        _ => 2
    }
}
```

- contains: not assignable
