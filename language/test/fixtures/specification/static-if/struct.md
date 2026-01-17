# Static If Structs

Tests for static if gating on struct members.

## Gating

### static if gates struct members

> Struct members gated by static if are removed before resolution.

```ds
struct Point {
    @if(import.meta.output == "js" && import.meta.output == "native")
    missing: MissingType;

    @if(import.meta.output == "js" && import.meta.output == "native")
    missingMethod(): MissingType {
        return missingSymbol;
    }

    x: int32;
    y: int32;
}
```

### static if keeps struct members when true

> Struct members gated by true static if conditions remain available.

```ds
struct Point {
    @if(true)
    x: int32;

    y: int32;
}

const point = Point { x: 1, y: 2 };
point.x satisfies int32;
```
