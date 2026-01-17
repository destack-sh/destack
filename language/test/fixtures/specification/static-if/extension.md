# Static If Extensions

Tests for static if gating on extension members.

## Gating

### static if gates extension members

> Extension members gated by static if are removed before resolution.

```ds
struct Box {
    value: number;
}

extension for Box {
    @if(import.meta.output == "js" && import.meta.output == "native")
    missing(): MissingType {
        return missingSymbol;
    }

    get(): number {
        return this.value;
    }
}

declare function makeBox(): Box;

const box = makeBox();
box.get() satisfies number;
```

### static if keeps extension members when true

> Extension members gated by true static if conditions remain available.

```ds
struct Box {
    value: number;
}

extension for Box {
    @if(true)
    get(): number {
        return this.value;
    }
}

declare function makeBox(): Box;

const box = makeBox();
box.get() satisfies number;
```
