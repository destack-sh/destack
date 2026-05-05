# Static If Structs

Struct members can be gated with `@if`.

## gating

### when false, `@if` removes struct members

When false, `@if` removes struct members.

```ds
struct Point {
    @if(false)
    missing: MissingType;

    @if(false)
    missingMethod(): MissingType {
        return missingSymbol;
    }

    x: int32;
    y: int32;
}
```

### when true, `@if` includes struct members

When true, `@if` includes struct members.

```ds
struct Point {
    @if(true)
    x: int32;

    y: int32;
}

const point = Point { x: 1, y: 2 };
point.x satisfies int32;
```

### when false, `@if` hides struct members

When false, struct members behind `@if` are not available on constructed values.

```ds
struct Point {
    @if(false)
    missing: int32;

    x: int32;
}

const point = Point { x: 1 };
point.missing satisfies int32;
```

- contains: does not exist

### when false, `@if` removes struct literal requirements

When false, struct fields behind `@if` are not required in struct literals.

```ds
struct Point {
    @if(false)
    hidden: int32;

    x: int32;
    y: int32;
}

const point = Point { x: 1, y: 2 };
point.x satisfies int32;
```

### when true, `@if` keeps struct literal requirements

When true, struct fields behind `@if` remain required for struct literal construction.

```ds
struct Point {
    @if(true)
    x: int32;

    y: int32;
}

const point = Point { y: 2 };
point.y satisfies int32;
```

- contains: not assignable
