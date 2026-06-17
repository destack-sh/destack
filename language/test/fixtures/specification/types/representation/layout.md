# Layout Intrinsics

Layout intrinsics observe the concrete representation selected for a type under the active target.

## queries

### layout queries return static values

Concrete types can be measured and reflected at compile time.

```ds
struct Header {
    tag: uint8;
    size: uint32;
}

const size = comptime sizeOf<Header>();
const alignment = comptime alignOf<Header>();
const stride = comptime strideOf<Header>();

size satisfies usize;
alignment satisfies usize;
stride satisfies usize;
```

### sizeOf participates in static inference

Layout queries can solve static generic arguments in type positions.

```ds
struct Header {
    tag: uint8;
    size: uint32;
}

declare function length<T, comptime N: usize>(values: [T; N]): N;

declare let bytes: [uint8; sizeOf<Header>()];
const bytesLength = length(bytes);

bytesLength satisfies sizeOf<Header>();
```

### strideOf participates in static defaults

Layout queries can be evaluated while solving default static arguments.

```ds
struct Header {
    tag: uint8;
    size: uint32;
}

type Slots<T: Concrete, comptime N: usize = strideOf<T>()> = [uint8; N];

declare let slots: Slots<Header>;

slots satisfies [uint8; strideOf<Header>()];
```

### layoutOf returns reflected shape

`layoutOf<T>()` exposes the concrete layout shape.

```ds
struct Header {
    tag: uint8;
    size: uint32;
}

const layout = comptime layoutOf<Header>();

layout satisfies Layout;
layout.shape satisfies { kind: "aggregate"; fields: readonly LayoutField[] };
```

### layout queries require concrete types

Transparent aliases and constraints have no single layout before a representation is selected.

```ds
type Writer = {
    write(bytes: [uint8]): uint;
}

const size = comptime sizeOf<Writer>();
```

- contains: no concrete representation

## representation attributes

### explicit alignment sets aggregate alignment

Alignment attributes raise the aggregate alignment and round the total size accordingly.

```ds
@repr({ align: 16 })
struct Block {
    value: uint8;
}

const alignment = comptime alignOf<Block>();
alignment satisfies 16;
```

### enum backing representation selects storage

Enum representation attributes select the integer backing layout.

```ds
@repr("uint8")
enum Mode {
    read,
    write,
}

const size = comptime sizeOf<Mode>();
size satisfies 1;
```
