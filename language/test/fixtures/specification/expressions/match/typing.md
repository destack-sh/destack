# Match Typing

## Result typing

### match unions case types

> Match results form a union when case types differ.

```ds
function match_union(value: int32): string {
    const result: string = match (value) {
        0 => {
            1 as int32
        }
        _ => "ok"
    };
    result
}
```

- contains: not assignable

## Pattern typing

### match patterns must be compatible with the matched value

> Pattern expressions must be assignable to the match value type.

```ds
function invalid_match_pattern(value: int32): int32 {
    match (value) {
        "hi" => 1
        _ => 2
    }
}
```

- contains: not assignable
