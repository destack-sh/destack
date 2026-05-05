# Associated Types

Associated types are type members on nominal types and interfaces.

## members

### type members can use enclosing type generics

> A type member can refer to generic parameters from its enclosing type.

```ds
struct Box<T> {
    type Item = T;
    value: T;
}

declare const value: Box<string>.Item;
value satisfies string;
```

### type members can use other type members

> `this` names other static members on the same type.

```ds
class Pair<T> {
    type Item = T;
    type Both = (this.Item, this.Item);
}

declare const value: Pair<string>.Both;
value satisfies (string, string);
```

### type members are not runtime properties

> Associated types can only be used in type positions.

```ds
class Packet {
    type Size = uint32;
}

const size = Packet.Size;
```

- contains: does not exist

## generics

### type members can have generic parameters

> Associated type members use the same generic parameter forms as ordinary type declarations.

```ds
class StorageHandle<T> {}
class User {}

interface Storage {
    type Handle<T>;
}

struct SharedStorage implements Storage {
    type Handle<T> = shared StorageHandle<T>;
}

declare const handle: SharedStorage.Handle<User>;
handle satisfies shared StorageHandle<User>;
```

### type members can have static value parameters

> Associated type parameters can include `comptime` values.

```ds
interface PacketLayout {
    type Bytes<comptime N: uint>;
}

struct InlinePacket implements PacketLayout {
    type Bytes<comptime N: uint> = [uint8; N];
}

declare const bytes: InlinePacket.Bytes<16>;
bytes satisfies [uint8; 16];
```

### generic type members require arguments

> A generic type member must be applied before it names a concrete type.

```ds
class StorageHandle<T> {}

interface Storage {
    type Handle<T>;
}

struct SharedStorage implements Storage {
    type Handle<T> = shared StorageHandle<T>;
}

declare const handle: SharedStorage.Handle;
```

- contains: argument
