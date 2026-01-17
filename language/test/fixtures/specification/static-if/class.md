# Static If Classes

Tests for static if gating on class members.

## Gating

### static if gates class members

> Class members gated by static if are removed before resolution.

```ds
class Box {
    @if(import.meta.output == "js" && import.meta.output == "native")
    missing: MissingType;

    @if(import.meta.output == "js" && import.meta.output == "native")
    missingMethod(): MissingType {
        return missingSymbol;
    }

    @if(import.meta.output == "js" && import.meta.output == "native")
    static missingStatic(): MissingType {
        return missingSymbol;
    }

    value: number = 0;
}
```
