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

### static if true class fields remain required in constructors

> Class fields gated with true still participate in class construction requirements.

```json:destack.json
{ "compiler": { "strictPropertyInitialization": false } }
```

```ds
class Box {
    @if(true)
    value: number;
}

const box = Box {};
box.value satisfies number;
```

- contains: not assignable
