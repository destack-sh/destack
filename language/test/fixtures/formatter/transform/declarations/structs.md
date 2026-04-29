# Struct Declarations

Tests for struct declaration formatting.

## Basic Structs

### simple struct

Structs format like nominal value types with semicolon-terminated members.

```ds
struct Point { x: number; y: number }
```

```ds expected
struct Point {
    x: number;
    y: number;
}
```

### empty struct

Empty structs stay on one line.

```ds
struct Empty { }
```

```ds expected
struct Empty {}
```

### struct with field defaults

Struct field defaults keep spacing around `=`.

```ds
struct Settings { retries: int = 3; verbose: boolean = false }
```

```ds expected
struct Settings {
    retries: int = 3;
    verbose: boolean = false;
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
        return 0;
    }
}
```

### struct with decorated field

Body level struct member annotations stay on their own line above the member.

```ds
struct User { @validate(minLength(1)) name: string }
```

```ds expected
struct User {
    @validate(minLength(1))
    name: string;
}
```

### struct with static field

Static fields keep the static keyword and trailing semicolons.

```ds
struct Versioned { static version: string = "1" }
```

```ds expected
struct Versioned {
    static version: string = "1";
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
        const size = N;
    }
}
```

### struct comptime block with nested control flow

Comptime member blocks keep nested if branch tails semicolonless.

```ds
struct Buffer<comptime N: number> { comptime { if (N > 0) { assert(N) } else { fail() } } }
```

```ds expected
struct Buffer<comptime N: number> {
    comptime {
        if (N > 0) {
            assert(N)
        } else {
            fail()
        }
    }
}
```

### struct value method tail

Value-returning struct methods keep terminal expressions semicolonless.

```ds
struct Vec2 { length(): float { const squared = x * x + y * y; squared.sqrt() } }
```

```ds expected
struct Vec2 {
    length(): float {
        const squared = x * x + y * y;
        squared.sqrt()
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
    value: T;
}
```

### struct with where clause

Where clauses attach to the struct header.

```ds
struct Pair<T, U> where (T: Copy, U: Clone) { left: T; right: U }
```

```ds expected
struct Pair<T, U> where (T: Copy, U: Clone) {
    left: T;
    right: U;
}
```
