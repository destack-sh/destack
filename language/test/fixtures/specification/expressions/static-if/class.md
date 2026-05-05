# Static If Classes

Class members can be gated with `@if`.

## gating

### when false, `@if` removes class members

When false, `@if` removes class members.

```ds
class Box {
    @if(false)
    missing: MissingType;

    @if(false)
    missingMethod(): MissingType {
        return missingSymbol;
    }

    @if(false)
    get missingAccessor(): MissingType {
        return missingSymbol;
    }

    @if(false)
    set missingSetter(value: MissingType) {
        missingSymbol;
    }

    @if(false)
    static missingStatic(): MissingType {
        return missingSymbol;
    }

    value: number = 0;
}
```

### when true, `@if` includes class members

When true, `@if` includes class members.

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

### when false, `@if` hides class members

When false, members behind `@if` are not available during member access.

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

### when false, `@if` hides static class members

When false, static members behind `@if` are not available on the constructor.

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
