# Match Newtypes

## scalar newtype patterns

### scalar newtype patterns bind values

Scalar newtype patterns bind the wrapped value.

```ds
newtype UserId = int64;

function nextId(id: UserId): int64 {
    match (id) {
        UserId(value) => {
            value satisfies int64;
            value
        }
    }
}
```

### scalar newtype patterns reject bare literals

Untagged literals do not match nominal newtypes.

```ds
newtype UserId = int64;

function nextId(id: UserId): int64 {
    return match (id) {
        1 => 1
        UserId(value) => value
    };
}
```

- contains: not assignable

## tuple newtype patterns

### tuple newtype patterns bind fields

Tuple newtype patterns bind positional fields.

```ds
newtype Point = (float32, float32);

function sum(point: Point): float32 {
    match (point) {
        Point(x, y) => {
            x satisfies float32;
            y satisfies float32;
            x
        }
    }
}
```

## object newtype patterns

### object newtype patterns unwrap object payloads

Object newtypes use the newtype wrapper pattern around an object pattern.

```ds
newtype Config = { debug: boolean };

function read(config: Config): boolean {
    match (config) {
        Config({ debug }) => {
            debug satisfies boolean;
            debug
        }
    }
}
```

### object newtype patterns reject nominal object syntax

Object-backed newtypes unwrap through the wrapper pattern.

```ds
newtype Config = { debug: boolean };

function read(config: Config): boolean {
    match (config) {
        Config { debug } => debug
    }
}
```

- contains: not assignable

### object newtype patterns reject bare objects

Untagged object patterns do not match object newtypes.

```ds
newtype Config = { debug: boolean };

function read(config: Config): boolean {
    match (config) {
        { debug } => debug
        _ => false
    }
}
```

- contains: not assignable
