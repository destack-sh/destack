# Reference Conversion

`As` projects reference views with chosen access.

## access

Reference conversion uses TS++ access forms.

### As defaults to readonly access

`As<T>` projects a readonly view by default.

```ds
class Buffer {
    bytes: uint8[];
}

extension of Buffer implements As<[uint8]> {
    as(): &readonly [uint8] {
        return &readonly this.bytes;
    }
}

declare function read(bytes: &readonly [uint8]): void;

const buffer = new Buffer();
read(buffer.as());
```

### As selects access explicitly

The access parameter picks the projected form.

```ds
class Buffer {
    bytes: uint8[];
}

extension of Buffer implements As<[uint8], "exclusive"> {
    as(): &exclusive [uint8] {
        return &exclusive this.bytes;
    }
}

declare function write(bytes: &exclusive [uint8]): void;

const buffer = new Buffer();
write(buffer.as());
```
