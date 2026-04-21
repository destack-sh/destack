# Static If Extensions

Tests for static if gating on extension members.

## Gating

### static if gates extension members

> Extension members gated by static if are removed before resolution.

```ds
struct Box {
    value: number;
}

extension of Box {
    @if(import.meta.emit == "js" && import.meta.emit == "native")
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

extension of Box {
    @if(true)
    get(): number {
        return this.value;
    }
}

declare function makeBox(): Box;

const box = makeBox();
box.get() satisfies number;
```

### static if gated extension members are not visible

> Extension members removed by static if are not available during member resolution.

```ds
struct Box {
    value: number;
}

extension of Box {
    @if(false)
    missing(): number {
        return this.value;
    }

    get(): number {
        return this.value;
    }
}

declare function makeBox(): Box;

const box = makeBox();
box.missing() satisfies number;
```

- contains: does not exist

### static if gated extension members do not interfere with remaining members

> Disabled extension members do not change resolution of enabled members.

```ds
struct Box {
    value: number;
}

extension of Box {
    @if(false)
    hidden(): number {
        this.value
    }

    get(): number {
        this.value
    }

    increment(): number {
        this.value + 1
    }
}

declare function makeBox(): Box;

const box = makeBox();
box.get() satisfies number;
box.increment() satisfies number;
```

### static if true extension members remain resolvable

> Extension members gated with true remain available in member resolution.

```ds
struct Box {
    value: number;
}

extension of Box {
    @if(true)
    double(): number {
        this.value * 2
    }
}

declare function makeBox(): Box;

const box = makeBox();
box.double() satisfies number;
```
