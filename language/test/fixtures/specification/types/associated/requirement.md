# Associated Requirements

Interfaces can require associated types and constants from implementors.

## types

### implementors inherit type defaults

> A default type member is used when the implementor does not override it.

```ds
interface Stream<T> {
    type Item = T;
    next(): this.Item;
}

class Counter implements Stream<int32> {
    value: int32;

    next(): this.Item {
        this.value
    }
}

declare const item: Counter.Item;
item satisfies int32;
```

### implementors can override type defaults

> An explicit type member wins over the interface default.

```ds
class DecodeError {}
class JsonError {}
class User {}

declare function parseJson(raw: string): Result<User, JsonError>;

interface Decoder {
    type Error = DecodeError;
    decode(raw: string): Result<User, this.Error>;
}

class JsonDecoder implements Decoder {
    type Error = JsonError;

    decode(raw: string): Result<User, this.Error> {
        parseJson(raw)
    }
}

declare const error: JsonDecoder.Error;
error satisfies JsonError;
```

### required type members need implementations

> A required type member has no default.

```ds
interface Iterator {
    type Item;
    next(): Option<this.Item>;
}

class Empty implements Iterator {}
```

- contains: Item
- contains: next

## constants

### implementors inherit constant defaults

> A default constant member is used when the implementor does not override it.

```ds
interface RegisterBlock {
    comptime const Width: uint = 4;
    type Bytes = [uint8; this.Width];
}

struct Status implements RegisterBlock {}

declare const bytes: Status.Bytes;
bytes satisfies [uint8; 4];
```

### implementors can override constant defaults

> An explicit constant member wins over the interface default.

```ds
interface RegisterBlock {
    comptime const Width: uint = 4;
    type Bytes = [uint8; this.Width];
}

struct WideRegister implements RegisterBlock {
    comptime const Width: uint = 16;
}

declare const bytes: WideRegister.Bytes;
bytes satisfies [uint8; 16];
```

### extensions can provide requirements

> An extension can provide associated members for an interface implementation.

```ds
interface Window<T> {
    comptime const Rows: uint;
    type Slice = [T; this.Rows];
}

struct Data<T> {}

extension<T> of Data<T> implements Window<T> {
    comptime const Rows: uint = 16;
}

declare const slice: Data<uint8>.Slice;
slice satisfies [uint8; 16];
```
