# Static If Structs

Static if gating on struct members.

## gating

### static if gates struct members

> Struct members gated by static if are removed before resolution.

```ds
struct Point {
    @if(import.meta.emit == "js" && import.meta.emit == "native")
    missing: MissingType;

    @if(import.meta.emit == "js" && import.meta.emit == "native")
    missingMethod(): MissingType {
        return missingSymbol;
    }

    x: int32;
    y: int32;
}
```

### static if keeps struct members when true

> Struct members gated by true static if conditions remain available.

```ds
struct Point {
    @if(true)
    x: int32;

    y: int32;
}

const point = Point { x: 1, y: 2 };
point.x satisfies int32;
```

### static if gated struct members are not visible

> Struct members removed by static if are not available on constructed values.

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

### static if gated struct fields are not required in literals

> Struct fields removed by static if are not required in struct literals.

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

### static if true struct fields remain required in literals

> Struct fields gated with true remain required for struct literal construction.

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
