# Static If Interfaces

Interface members can be gated with `@if`.

## gating

### when false, `@if` removes interface members

When false, `@if` removes interface members.

```ds
interface Box {
    @if(false)
    missing: MissingType;
    @if(false)
    missingMethod(): MissingType;

    value: number;
}

declare const box: Box;
box.value satisfies number;
```

### when true, `@if` includes interface members

When true, `@if` includes interface members.

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

### when false, `@if` hides interface members

When false, interface members behind `@if` are not available.

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

### when false, `@if` removes implementation requirements

When false, interface members behind `@if` are not required in implementations.

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

### when true, `@if` keeps implementation requirements

When true, interface members behind `@if` must still be implemented.

```ds
interface Box {
    @if(true)
    value(): number;
}

class Concrete implements Box {}
```

- contains: not assignable
