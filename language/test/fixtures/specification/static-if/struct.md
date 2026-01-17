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
