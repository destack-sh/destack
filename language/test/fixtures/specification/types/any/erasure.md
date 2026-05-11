# Any

`Any<T>` is explicit runtime erasure for values that must keep one representation while hiding their concrete type.

## static interfaces

### interface parameters specialize by default

A plain interface parameter is checked as a hidden generic constraint.

```ds
newtype interface Writer {
    write(bytes: [uint8]): uint;
}

struct Buffer implements Writer {
    write(bytes: [uint8]): uint {
        bytes.length
    }
}

declare const bytes: [uint8];

function write(writer: Writer): uint {
    writer.write(bytes)
}

write(Buffer {}) satisfies uint;
```

### explicit generic parameters use the same model

The named generic form makes the specialization parameter explicit.

```ds
newtype interface Writer {
    write(bytes: [uint8]): uint;
}

struct Buffer implements Writer {
    write(bytes: [uint8]): uint {
        bytes.length
    }
}

declare const bytes: [uint8];

function write<W: Writer>(writer: W): uint {
    writer.write(bytes)
}

write(Buffer {}) satisfies uint;
```

## erased values

### any stores an erased interface value

`Any<T>` keeps one field layout for all concrete implementations.

```ds
newtype interface Writer {
    write(bytes: [uint8]): uint;
}

struct Logger {
    writer: Any<Writer>;
}

declare const bytes: [uint8];
declare const logger: Logger;

logger.writer.write(bytes) satisfies uint;
```

### generic aggregates keep concrete field layout

Generic aggregates carry the concrete implementation instead of erasing it.

```ds
newtype interface Writer {
    write(bytes: [uint8]): uint;
}

struct Buffer implements Writer {
    write(bytes: [uint8]): uint {
        bytes.length
    }
}

struct Logger<W: Writer> {
    writer: W;
}

const logger = Logger<Buffer> { writer: Buffer {} };
logger.writer satisfies Buffer;
```

### empty any uses an explicit anonymous type argument

Anonymous object type arguments use `type` when the syntax would otherwise look like a value expression.

```ds
declare const value: Any<type {}>;
value satisfies Any<type {}>;
```

## unknown

### unknown is not erased any

`unknown` carries no usable interface until it is narrowed.

```ds
newtype interface Writer {
    write(bytes: [uint8]): uint;
}

declare const bytes: [uint8];
declare const writer: Any<Writer>;
declare const opaque: unknown;

writer.write(bytes) satisfies uint;
opaque.write(bytes);
```

- contains: unknown
