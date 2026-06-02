# Representation Attributes

Representation attributes constrain concrete layout without changing type identity.

## repr

### repr C selects ABI layout

`@repr("C")` asks for the active target's C aggregate layout.

```ds
@repr("C")
struct Header {
    tag: uint8;
    size: uint32;
}

const layout = comptime layoutOf<Header>();

layout satisfies Layout;
```

### repr transparent preserves backing ABI

`@repr("transparent")` gives a single-field wrapper the same ABI representation as its field.

```ds
@repr("transparent")
newtype FileDescriptor = int32;

const descriptorSize = comptime sizeOf<FileDescriptor>();
const rawSize = comptime sizeOf<int32>();
const hasSameSize = comptime descriptorSize == rawSize;
const descriptorAlign = comptime alignOf<FileDescriptor>();
const rawAlign = comptime alignOf<int32>();
const hasSameAlign = comptime descriptorAlign == rawAlign;

hasSameSize satisfies true;
hasSameAlign satisfies true;
```

### repr scalar selects enum backing

Enum representation can be fixed to a primitive scalar type.

```ds
@repr(uint8)
enum Color {
    Red = 1,
    Green = 2,
}

const size = comptime sizeOf<Color>();
const alignment = comptime alignOf<Color>();

size satisfies 1;
alignment satisfies 1;
```

## representation options

### repr align raises aggregate alignment

`@repr({ align: N })` raises the minimum alignment of an aggregate declaration.

```ds
@repr({ align: 64 })
struct CacheLine {
    value: uint64;
}

const alignment = comptime alignOf<CacheLine>();
const isAligned = comptime alignment >= 64;

isAligned satisfies true;
```

### repr packed lowers field alignment

`@repr({ packed: true })` is equivalent to `@repr({ packed: 1 })`.

```ds
@repr("C", { packed: true })
struct WireHeader {
    tag: uint8;
    size: uint32;
}

const layout = comptime layoutOf<WireHeader>();
const alignment = comptime alignOf<WireHeader>();

layout satisfies Layout;
alignment satisfies 1;
```
