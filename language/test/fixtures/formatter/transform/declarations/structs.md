# Struct Declarations

Tests for struct declaration formatting.

## Basic Structs

### simple struct

Structs format like nominal value types with comma-separated members.

```ds
struct Point { x: number; y: number }
```

```ds expected
struct Point {
    x: number,
    y: number,
}
```

### empty struct

Empty structs stay on one line.

```ds
struct Empty { }
```

```ds expected
struct Empty { }
```

### struct with field defaults

Struct field defaults keep spacing around `=`.

```ds
struct Settings { retries: int = 3; verbose: boolean = false }
```

```ds expected
struct Settings {
    retries: int = 3,
    verbose: boolean = false,
}
```

## Struct Members

### struct with method

Struct methods format like class methods.

```ds
struct Vec2 { length(): float { return 0 } }
```

```ds expected
struct Vec2 {
    length(): float {
        return 0
    }
}
```

### struct with static field

Static fields keep the static keyword and commas.

```ds
struct Versioned { static version: string = "1" }
```

```ds expected
struct Versioned {
    static version: string = '1',
}
```

### struct with comptime block

Comptime blocks format like other blocks inside structs.

```ds
struct Buffer<comptime N: number> {
    comptime { const size = N }
}
```

```ds expected
struct Buffer<comptime N: number> {
    comptime {
        const size = N
    }
}
```

## Generics and Heritage

### struct with generics and implements

Structs can be generic and implement interfaces.

```ds
struct Box<T> implements Iterable<T> { value: T }
```

```ds expected
struct Box<T> implements Iterable<T> {
    value: T,
}
```

### struct with where clause

Where clauses attach to the struct header.

```ds
struct Pair<T, U> where (T: Copy, U: Clone) { left: T; right: U }
```

```ds expected
struct Pair<T, U> where (T: Copy, U: Clone) {
    left: T,
    right: U,
}
```
