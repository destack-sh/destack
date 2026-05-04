# Match Newtypes

## scalar newtype patterns

### match scalar newtype pattern binds inner value

> Newtype patterns bind the underlying scalar value.

```ds
newtype UserId = int64;

function next_id(id: UserId): int64 {
    match (id) {
        UserId(value) => {
            value satisfies int64;
            value
        }
    }
}
```

### match scalar newtype pattern rejects untagged literals

> Untagged literals do not match nominal newtypes.

```ds
newtype UserId = int64;

function next_id(id: UserId): int64 {
    return match (id) {
        1 => 1
        UserId(value) => value
    };
}
```

- type object is not assignable to type Config

## tuple newtype patterns

### match tuple newtype pattern binds positional values

> Tuple newtype patterns bind the underlying tuple fields.

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

### match object newtype pattern requires tag

> Object newtypes require tagged object patterns.

```ds
newtype Config = { debug: boolean };

function read(config: Config): boolean {
    match (config) {
        Config { debug } => {
            debug satisfies boolean;
            debug
        }
    }
}
```

### match object newtype pattern rejects untagged objects

> Untagged object patterns do not match object newtypes.

```ds
newtype Config = { debug: boolean };

function read(config: Config): boolean {
    match (config) {
        { debug } => debug
        _ => false
    }
}
```

- type object is not assignable to type Config
