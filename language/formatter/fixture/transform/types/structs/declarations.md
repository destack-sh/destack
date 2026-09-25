# Struct Declarations

Struct fixtures cover declaration heads, fields, methods, and const member blocks.

## Struct Forms

### struct declaration

Structs format like nominal value types with semicolon-terminated members.

```tspp
struct Point { x: number; y: number }
```

```tspp expected
struct Point {
    x: number;
    y: number;
}
```

### empty struct

Empty structs stay on one line.

```tspp
struct Empty { }
```

```tspp expected
struct Empty {}
```

### struct with field defaults

Struct field defaults keep spacing around `=`.

```tspp
struct Settings { retries: int = 3; verbose: boolean = false }
```

```tspp expected
struct Settings {
    retries: int = 3;
    verbose: boolean = false;
}
```

## Struct Members

### struct with method

Struct methods format like class methods.

```tspp
struct Vec2 { length(): float { return 0 } }
```

```tspp expected
struct Vec2 {
    length(): float {
        return 0;
    }
}
```

### struct with decorated field

Body level struct member annotations stay on their own line above the member.

```tspp
struct User { @validate(minLength(1)) name: string }
```

```tspp expected
struct User {
    @validate(minLength(1))
    name: string;
}
```

### struct with static field

Static fields keep the static keyword and trailing semicolons.

```tspp
struct Versioned { static version: string = "1" }
```

```tspp expected
struct Versioned {
    static version: string = "1";
}
```

### struct with const block

Const blocks format like other blocks inside structs.

```tspp
struct Buffer<const N: number> {
    const { const size = N }
}
```

```tspp expected
struct Buffer<const N: number> {
    const {
        const size = N;
    }
}
```

### struct const block with nested control flow

Const member blocks keep nested if branch tails semicolonless.

```tspp
struct Buffer<const N: number> { const { if (N > 0) { assert(N) } else { fail() } } }
```

```tspp expected
struct Buffer<const N: number> {
    const {
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

```tspp
struct Vec2 { length(): float { const squared = x * x + y * y; squared.sqrt() } }
```

```tspp expected
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

```tspp
struct Box<T> implements Iterable<T> { value: T }
```

```tspp expected
struct Box<T> implements Iterable<T> {
    value: T;
}
```

### struct with where clause

Where clauses attach to the struct header.

```tspp
struct Pair<T, U> where (T: Copy, U: Clone) { left: T; right: U }
```

```tspp expected
struct Pair<T, U> where T: Copy, U: Clone {
    left: T;
    right: U;
}
```
