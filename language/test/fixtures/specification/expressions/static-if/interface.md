# Static If Interfaces

Static if gating on interface members.

## Gating

### static if gates interface members

> Interface members gated by static if are removed before resolution.

```ds
interface Box {
    @if(import.meta.emit == "js" && import.meta.emit == "native")
    missing: MissingType;

    @if(import.meta.emit == "js" && import.meta.emit == "native")
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

### static if gated interface members are not visible

> Interface members removed by static if are not available.

```ds
interface Box {
    @if(false)
    missing: number;

    value: number;
}

declare const box: Box;
box.missing satisfies number;
```

- contains: does not exist

### static if gated interface members are not required by implementors

> Interface members removed by static if are not required in implementations.

```ds
interface Box {
    @if(false)
    missing(): number;

    value(): number;
}

class Concrete implements Box {
    value(): number {
        1
    }
}
```

### static if true interface members remain required by implementors

> Interface members gated with true must still be implemented.

```ds
interface Box {
    @if(true)
    value(): number;
}

class Concrete implements Box {}
```

- type unknown is not assignable to type (): number
