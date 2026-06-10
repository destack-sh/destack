# Dynamic

`Dynamic<T>` is the explicit erased runtime value satisfying any dynamic-safe `T`.

## erasure

### dynamic fields have fixed layout

Erasing the constraint keeps the aggregate concrete instead of inducing a generic.

```ds
interface Writer {
    write(bytes: readonly uint8[]): uint;
}

struct Logger {
    writer: Dynamic<Writer>;
}

const size = comptime sizeOf<Logger>();
size satisfies usize;
```

### implementors erase at storage boundaries

Storing an implementor into a `Dynamic` place erases it.

```ds
interface Writer {
    write(bytes: readonly uint8[]): uint;
}

struct Console {
    written: uint;
}

extension of Console {
    write(bytes: readonly uint8[]): uint {
        bytes.length
    }
}

let writer: Dynamic<Writer> = Console { written: 0 };

writer.write([1, 2]) satisfies uint;
```

### dynamic collections hold mixed implementors

Erased elements share one representation, so the collection is heterogeneous.

```ds
interface Writer {
    write(bytes: readonly uint8[]): uint;
}

struct Console {
    written: uint;
}

struct Null {}

extension of Console {
    write(bytes: readonly uint8[]): uint {
        bytes.length
    }
}

extension of Null {
    write(bytes: readonly uint8[]): uint {
        0
    }
}

let writers: Dynamic<Writer>[] = [Console { written: 0 }, Null {}];

writers[0].write([1]) satisfies uint;
```

## safety

### generic members are not dynamic safe

A member that introduces new generic parameters has no erasable witness.

```ds
interface Mapper {
    map<U>(value: U): U;
}

declare const erased: Dynamic<Mapper>;
```

- contains: dynamic

### index signatures are not dynamic safe

Index signatures have no fixed member surface to erase.

```ds
type Bag = {
    [key: string]: int32;
};

declare const erased: Dynamic<Bag>;
```

- contains: dynamic
