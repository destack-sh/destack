# Reference Type Annotations

## reference annotations

### reference annotations are accepted

> Reference annotations can appear in signatures.

```ds
struct Point {
    x: int32;
}

function read(value: &Point): int32 {
    return value.x;
}
```

### value annotations are accepted

> Value annotations can appear in signatures.

```ds
struct Point {
    x: int32;
}

function copy(value: ^Point): ^Point {
    return value;
}
```
