# Interval Nominality

Interval types can be used as newtype backing types.

## newtypes

### newtypes can use interval backing types

```ds
newtype Port = 1..=65535;

let port = Port(443);
port satisfies Port;
```

### validated newtypes preserve runtime invariants through mutation

```ds
newtype Port = int;

extension of Port {
    static try(value: int): Result<Port, Error> {
        if (value < 1 || value > 65535) {
            return Result.err(Error.message("port out of range"));
        }

        return Result.ok(Port(value));
    }
}
```
