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
    get missingAccessor(): MissingType {
        return missingSymbol;
    }

    @if(import.meta.output == "js" && import.meta.output == "native")
    set missingSetter(value: MissingType) {
        missingSymbol;
    }

    @if(import.meta.output == "js" && import.meta.output == "native")
    static missingStatic(): MissingType {
        return missingSymbol;
    }

    value: number = 0;
}
```

### static if keeps class members when true

> Class members gated by true static if conditions remain available.

```ds
class Box {
    @if(true)
    value: number = 1;

    @if(true)
    increment(): number {
        return this.value + 1;
    }
}

const box = new Box();
const total = box.increment();
total satisfies number;
```
