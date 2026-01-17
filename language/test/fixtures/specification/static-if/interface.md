# Static If Interfaces

Tests for static if gating on interface members.

## Gating

### static if gates interface members

> Interface members gated by static if are removed before resolution.

```ds
interface Box {
    @if(import.meta.output == "js" && import.meta.output == "native")
    missing: MissingType;

    @if(import.meta.output == "js" && import.meta.output == "native")
    missingMethod(): MissingType;

    value: number;
}

declare const box: Box;
box.value satisfies number;
```

### static if keeps interface members when true

> Interface members gated by true static if conditions remain available.

```ds
interface Box {
    @if(true)
    value: number;

    @if(true)
    increment(): number;
}

declare const box: Box;
box.value satisfies number;
box.increment() satisfies number;
```
