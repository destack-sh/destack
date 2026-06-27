# Static Choices

Generic-dependent static terms can select types and constants, but not source nodes.

## members

### fields can use static parameters in conditional types

Fields can depend on static parameters by making the field type conditional.

```ds
struct InlineIndex {}
struct ExternalIndex {}

struct Buffer<T, comptime Mode: "inline" | "external"> {
    index: Mode == "inline" ? InlineIndex : ExternalIndex;
    value: T;
}

declare const buffer: Buffer<string, "external">;
buffer.index satisfies ExternalIndex;
```

### fields can use type relations in conditional types

Fields can depend on type relations by making the field type conditional.

```ds
struct TextMeta {}

struct Packet<T> {
    meta: T extends string ? TextMeta : ();
    value: T;
}

declare const packet: Packet<string>;
packet.meta satisfies TextMeta;
```

### generic-dependent @if does not remove fields

Generic-dependent member removal is rejected.

```ds
struct ExternalIndex {}

struct Buffer<T, comptime Mode: "inline" | "external"> {
    @if(Mode == "external")
    index: ExternalIndex;

    value: T;
}

declare const buffer: Buffer<string, "inline">;
buffer.index;
```

- contains: static @if condition must be statically decidable

### fields can use associated constants in conditional types

Fields can depend on associated constants by making the field type conditional.

```ds
struct WideMeta {}

class Segment<Row> {
    comptime const Width: uint = Row extends string ? 8 : 4;
    meta: this.Width == 8 ? WideMeta : ();
    value: Row;
}

declare const segment: Segment<string>;
segment.meta satisfies WideMeta;
```

### fields can use contextual placement in conditional types

Fields can depend on the containing value's placement by making the field type conditional.

```ds
struct SharedLock {}

struct Buffer<T> {
    lock: PlaceOf<this> == "shared" ? SharedLock : ();
    value: T;
}

declare const buffer: shared Buffer<string>;
buffer.lock satisfies SharedLock;
```

### ambient placement selects unit fields

The ambient form keeps the same field and selects the unit type.

```ds
struct SharedLock {}

struct Buffer<T> {
    lock: PlaceOf<this> == "shared" ? SharedLock : ();
    value: T;
}

declare const buffer: Buffer<string>;
buffer.lock satisfies ();
```
