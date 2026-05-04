# Associated Constants

Associated constants are `comptime const` members on nominal types and interfaces.

## values

### constants can use owner generics

> A constant member can branch on generic parameters from its owner.

```ds
class Segment<Row> {
    comptime const Width: uint = Row extends string ? 4 : 2;
}

const width = Segment<string>.Width;
width satisfies 4;
```

### constants can size fixed arrays

> Associated constants can be used where a compile-time value is required.

```ds
class Segment<Row> {
    comptime const Width: uint = Row extends string ? 8 : 4;
    type Lane = [uint8; this.Width];
}

declare const lane: Segment<string>.Lane;
lane satisfies [uint8; 8];
```

### constants can use other constants

> `this` names other static members on the same owner.

```ds
class Layout<Row> {
    comptime const Width: uint = Row extends string ? 4 : 2;
    comptime const DoubleWidth: uint = this.Width * 2;
    type Lane = [Row; this.DoubleWidth];
}

declare const lane: Layout<string>.Lane;
lane satisfies [string; 8];
```

## value positions

### value reads need concrete types

> Reading an associated constant as a value requires a concrete type.

```ds
class Segment<Row> {
    comptime const Width: uint = Row extends string ? 4 : 2;
}

function unresolved<Row>() {
    const width = Segment<Row>.Width;
    width
}
```

- contains: resolvable

### constants are not instance fields

> Associated constants are read from the type, not from values of the type.

```ds
class Segment<Row> {
    comptime const Width: uint = Row extends string ? 4 : 2;
}

const segment = new Segment<string>();
const width = segment.Width;
```

- contains: does not exist
