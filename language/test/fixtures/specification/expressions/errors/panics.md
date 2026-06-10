# Panics

Panics are for bugs: they take a string message, unwind the Worker, and cannot be caught in userland.

## surface

### panic takes a message

`panic` never returns.

```ds
function explode(): never {
    panic("invariant broken")
}
```

### panic messages are strings

There are no exception payloads, only messages.

```ds
panic(42);
```

- contains: string

### todo panics for unfinished code

`todo` marks unfinished code paths.

```ds
function later(): int32 {
    todo("implement later")
}
```

### unreachable panics for impossible paths

`unreachable` marks paths the author believes cannot happen.

```ds
function pick(value: 1 | 2): string {
    match (value) {
        1 => "one"
        2 => "two"
        _ => unreachable("covered above")
    }
}
```

## typing

### panic types as never

A panicking branch contributes nothing to the joined type.

```ds
function half(value: int32): int32 {
    if (value % 2 == 0) {
        value / 2
    } else {
        panic("odd value")
    }
}
```
