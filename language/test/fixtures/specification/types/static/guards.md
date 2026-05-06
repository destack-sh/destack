# Static Guards

Members can be gated with `@if`.

## members

### fields can be gated by static parameters

Fields can be `@if` gated with static parameters.

```ds
struct InlineIndex {}
struct ExternalIndex {}

struct Buffer<T, comptime Mode: "inline" | "external"> {
    @if(Mode == "inline")
    index: InlineIndex;

    @if(Mode == "external")
    index: ExternalIndex;

    value: T;
}

declare const buffer: Buffer<string, "external">;
buffer.index satisfies ExternalIndex;
```

### fields can be gated by type relations

Fields can be `@if` gated with type relations.

```ds
struct TextMeta {}

struct Packet<T> {
    @if(T extends string)
    meta: TextMeta;

    value: T;
}

declare const packet: Packet<string>;
packet.meta satisfies TextMeta;
```

### when false, @if removes fields

A field behind `@if(false)` is absent.

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

- contains: does not exist

### fields can be gated by associated constants

Fields can be `@if` gated with associated constants.

```ds
struct WideMeta {}

class Segment<Row> {
    comptime const Width: uint = Row extends string ? 8 : 4;

    @if(this.Width == 8)
    meta: WideMeta;

    value: Row;
}

declare const segment: Segment<string>;
segment.meta satisfies WideMeta;
```

### fields can be gated by contextual placement

Fields can be `@if` gated with the containing value's placement.

```ds
struct SharedLock {}

struct Buffer<T> {
    @if(PlaceOf<this> == "shared")
    lock: SharedLock;

    value: T;
}

declare const buffer: shared Buffer<string>;
buffer.lock satisfies SharedLock;
```

### ambient placement omits shared fields

A field gated on shared placement is absent from the ambient form.

```ds
struct SharedLock {}

struct Buffer<T> {
    @if(PlaceOf<this> == "shared")
    lock: SharedLock;

    value: T;
}

declare const buffer: Buffer<string>;
buffer.lock;
```

- contains: does not exist
