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
