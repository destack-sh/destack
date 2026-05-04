# Static If Classes

Static if gating on class members.

## Gating

### static if gates class members

> Class members gated by static if are removed before resolution.

```ds
class Box {
    @if(import.meta.emit == "js" && import.meta.emit == "native")
    missing: MissingType;

    @if(import.meta.emit == "js" && import.meta.emit == "native")
    missingMethod(): MissingType {
        return missingSymbol;
    }

    @if(import.meta.emit == "js" && import.meta.emit == "native")
    get missingAccessor(): MissingType {
        return missingSymbol;
    }

    @if(import.meta.emit == "js" && import.meta.emit == "native")
    set missingSetter(value: MissingType) {
        missingSymbol;
    }

    @if(import.meta.emit == "js" && import.meta.emit == "native")
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

### static if gated class members are not visible

> Members removed by static if are not available during member access.

```ds
class Box {
    @if(false)
    missing: number = 1;

    value: number = 0;
}

const box = new Box();
box.missing satisfies number;
```

- contains: does not exist

### static if also gates static class members

> Static class members removed by static if are not available on the constructor.

```ds
class Box {
    @if(false)
    static missing(): number {
        1
    }
}

Box.missing() satisfies number;
```

- contains: does not exist

